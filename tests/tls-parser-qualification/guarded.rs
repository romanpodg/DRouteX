//! Test-only composition, not a production gateway or an accepted profile.
use std::collections::BTreeSet;
use tls_parser::{
    TlsExtension, TlsRecordType, parse_tls_extension_sni_hostname, parse_tls_extension_unknown,
    parse_tls_handshake_client_hello, parse_tls_raw_record, parse_tls_record_header,
};

const MAX_INPUT: usize = 16 * 1024;
const MAX_RECORDS: usize = 16;
const MAX_EXTENSIONS: usize = 128;
#[derive(Debug, Eq, PartialEq)]
enum Outcome {
    Pending,
    Rejected,
    Name(Vec<u8>),
}

// Input remains owned by the caller, including any bytes beyond the ClientHello.
// This synchronous fixture has no socket, deadline, concurrency or admission.
fn extract(input: &[u8]) -> Outcome {
    if input.len() > MAX_INPUT {
        return Outcome::Rejected;
    }
    let mut wire = input;
    let mut handshake = Vec::new();
    for _ in 0..MAX_RECORDS {
        if wire.len() < 5 {
            return Outcome::Pending;
        }
        let Ok((_, header)) = parse_tls_record_header(wire) else {
            return Outcome::Rejected;
        };
        if header.record_type != TlsRecordType::Handshake
            || !(0x0301..=0x0303).contains(&header.version.0)
            || header.len == 0
            || usize::from(header.len) > 16_384
        {
            return Outcome::Rejected;
        }
        if wire.len() < 5 + usize::from(header.len) {
            return Outcome::Pending;
        }
        let Ok((tail, record)) = parse_tls_raw_record(wire) else {
            return Outcome::Rejected;
        };
        if handshake.len() + record.data.len() > MAX_INPUT {
            return Outcome::Rejected;
        }
        handshake.extend_from_slice(record.data);
        if handshake.len() >= 4 {
            if handshake[0] != 1 {
                return Outcome::Rejected;
            }
            let len = (usize::from(handshake[1]) << 16)
                | (usize::from(handshake[2]) << 8)
                | usize::from(handshake[3]);
            if len + 4 > MAX_INPUT {
                return Outcome::Rejected;
            }
            if handshake.len() >= len + 4 {
                return parse_body(&handshake[4..len + 4]);
            }
        }
        wire = tail;
    }
    Outcome::Rejected
}
fn parse_body(body: &[u8]) -> Outcome {
    let Ok((tail, hello)) = parse_tls_handshake_client_hello(body) else {
        return Outcome::Rejected;
    };
    if !tail.is_empty()
        || hello.ciphers.is_empty()
        || !hello.comp.iter().any(|method| method.0 == 0)
    {
        return Outcome::Rejected;
    }
    let Some(mut extensions) = hello.ext else {
        return Outcome::Rejected;
    };
    let mut seen = BTreeSet::new();
    let mut name = None;
    while !extensions.is_empty() {
        if seen.len() >= MAX_EXTENSIONS {
            return Outcome::Rejected;
        }
        let Ok((tail, TlsExtension::Unknown(kind, data))) = parse_tls_extension_unknown(extensions)
        else {
            return Outcome::Rejected;
        };
        if !seen.insert(kind.0) || kind.0 == 0xfe0d || kind.0 == 0xffce {
            return Outcome::Rejected;
        }
        if kind.0 == 0 {
            if data.len() < 2
                || usize::from(u16::from_be_bytes([data[0], data[1]])) != data.len() - 2
            {
                return Outcome::Rejected;
            }
            let Ok((tail, (kind, host))) = parse_tls_extension_sni_hostname(&data[2..]) else {
                return Outcome::Rejected;
            };
            if !tail.is_empty() || kind.0 != 0 || host.is_empty() || !host.is_ascii() {
                return Outcome::Rejected;
            }
            name = Some(host.to_vec());
        }
        extensions = tail;
    }
    name.map_or(Outcome::Rejected, Outcome::Name)
}

fn synthetic(extensions: &[u8]) -> Vec<u8> {
    let mut body = vec![3, 3];
    body.extend_from_slice(&[7; 32]);
    body.extend_from_slice(&[0, 0, 2, 0x13, 1, 1, 0]);
    body.extend_from_slice(&u16::try_from(extensions.len()).unwrap().to_be_bytes());
    body.extend_from_slice(extensions);
    let len = body.len();
    let mut handshake = vec![1, (len >> 16) as u8, (len >> 8) as u8, len as u8];
    handshake.extend_from_slice(&body);
    super::record(&handshake)
}
const SNI: &[u8] = &[0, 0, 0, 6, 0, 4, 0, 0, 1, b'a'];

#[test]
fn real_hello_partial_reads_and_extra_stream_bytes() {
    let data = super::client_hello();
    for index in 0..data.len() {
        assert_eq!(extract(&data[..index]), Outcome::Pending, "prefix {index}");
    }
    assert_eq!(
        extract(&data),
        Outcome::Name(b"origin.example.test".to_vec())
    );
    let mut extra = data.clone();
    extra.extend_from_slice(b"later encrypted bytes");
    assert_eq!(
        extract(&extra),
        Outcome::Name(b"origin.example.test".to_vec())
    );
    assert_eq!(&extra[..data.len()], &data);
}

#[test]
fn every_split_between_tls_records_keeps_the_clienthello() {
    let input = super::client_hello();
    let (_, raw) = parse_tls_raw_record(&input).unwrap();
    for split in 1..raw.data.len() {
        let mut fragmented = super::record(&raw.data[..split]);
        fragmented.extend_from_slice(&super::record(&raw.data[split..]));
        assert_eq!(
            extract(&fragmented),
            Outcome::Name(b"origin.example.test".to_vec()),
            "split {split}"
        );
    }
}

#[test]
fn rejects_ech_duplicate_truncation_and_trailing_hello_bytes() {
    let mut ech = SNI.to_vec();
    ech.extend_from_slice(&[0xfe, 0x0d, 0, 1, 7]);
    let mut esni = SNI.to_vec();
    esni.extend_from_slice(&[0xff, 0xce, 0, 1, 7]);
    let mut duplicate = SNI.to_vec();
    duplicate.extend_from_slice(SNI);
    let malformed_sni = [0, 0, 0, 7, 0, 5, 0, 0, 1, b'a', 0xff];
    let malformed_extension = [0, 0, 0, 9, 0, 4, 0, 0, 1, b'a'];
    for ext in [
        &ech[..],
        &esni[..],
        &duplicate[..],
        &malformed_sni[..],
        &malformed_extension[..],
        &[],
    ] {
        assert_eq!(extract(&synthetic(ext)), Outcome::Rejected);
    }
    let mut bad = synthetic(SNI);
    // Extend the declared record and handshake body with unparsed garbage.
    let len = u16::from_be_bytes([bad[3], bad[4]]) + 1;
    bad[3..5].copy_from_slice(&len.to_be_bytes());
    bad[8] += 1;
    bad.push(0xff);
    assert_eq!(extract(&bad), Outcome::Rejected);
}

#[test]
fn bounds_and_non_handshake_records_refuse() {
    assert_eq!(extract(&vec![0; MAX_INPUT + 1]), Outcome::Rejected);
    assert_eq!(extract(&[0x17, 3, 3, 0, 1, 0]), Outcome::Rejected);
    assert_eq!(extract(&[0x16, 3, 3, 0x40, 1]), Outcome::Rejected);
    let mut too_many = Vec::new();
    for _ in 0..MAX_RECORDS {
        too_many.extend_from_slice(&super::record(&[1]));
    }
    assert_eq!(extract(&too_many), Outcome::Rejected);
}

#[test]
fn rejects_empty_cipher_list_and_missing_null_compression() {
    fn hello(cipher_bytes: &[u8], compression: &[u8]) -> Vec<u8> {
        let mut body = vec![3, 3];
        body.extend_from_slice(&[7; 32]);
        body.push(0);
        body.extend_from_slice(&(cipher_bytes.len() as u16).to_be_bytes());
        body.extend_from_slice(cipher_bytes);
        body.push(compression.len() as u8);
        body.extend_from_slice(compression);
        body.extend_from_slice(&(SNI.len() as u16).to_be_bytes());
        body.extend_from_slice(SNI);
        let len = body.len();
        let mut handshake = vec![1, (len >> 16) as u8, (len >> 8) as u8, len as u8];
        handshake.extend_from_slice(&body);
        super::record(&handshake)
    }
    for wire in [
        hello(&[], &[0]),
        hello(&[0x13, 1], &[]),
        hello(&[0x13, 1], &[1]),
    ] {
        assert!(
            tls_parser::parse_tls_plaintext(&wire).is_ok(),
            "raw parser is permissive"
        );
        assert_eq!(extract(&wire), Outcome::Rejected);
    }
}

fn read_clienthello(
    stream: &mut std::net::TcpStream,
    timeout: std::time::Duration,
) -> std::io::Result<(Outcome, Vec<u8>)> {
    use std::io::Read;
    use std::time::Instant;
    let deadline = Instant::now() + timeout;
    let mut bytes = Vec::new();
    for _ in 0..256 {
        let Some(remaining) = deadline.checked_duration_since(Instant::now()) else {
            return Ok((Outcome::Rejected, bytes));
        };
        if remaining.is_zero() {
            return Ok((Outcome::Rejected, bytes));
        }
        stream.set_read_timeout(Some(remaining))?;
        let mut chunk = [0; 1024];
        match stream.read(&mut chunk) {
            Ok(0) => return Ok((Outcome::Rejected, bytes)),
            Ok(size) => {
                if bytes.len().saturating_add(size) > MAX_INPUT {
                    return Ok((Outcome::Rejected, bytes));
                }
                bytes.extend_from_slice(&chunk[..size]);
            }
            Err(error)
                if matches!(
                    error.kind(),
                    std::io::ErrorKind::TimedOut | std::io::ErrorKind::WouldBlock
                ) =>
            {
                return Ok((Outcome::Rejected, bytes));
            }
            Err(error) => return Err(error),
        }
        match extract(&bytes) {
            Outcome::Pending => (),
            outcome => return Ok((outcome, bytes)),
        }
    }
    Ok((Outcome::Rejected, bytes))
}

#[test]
fn loopback_reads_preserve_fragmented_handshake() {
    use std::io::Write;
    use std::net::{TcpListener, TcpStream};
    use std::time::Duration;
    let listener = TcpListener::bind("127.0.0.1:0").unwrap();
    let address = listener.local_addr().unwrap();
    let server = std::thread::spawn(move || {
        let (mut socket, _) = listener.accept().unwrap();
        read_clienthello(&mut socket, Duration::from_secs(2)).unwrap()
    });
    let mut client = TcpStream::connect(address).unwrap();
    let hello = super::client_hello();
    let (_, raw) = parse_tls_raw_record(&hello).unwrap();
    let split = raw.data.len() / 2;
    let mut fragmented = super::record(&raw.data[..split]);
    fragmented.extend_from_slice(&super::record(&raw.data[split..]));
    for part in fragmented.chunks(19) {
        client.write_all(part).unwrap();
    }
    let (result, consumed) = server.join().unwrap();
    assert_eq!(result, Outcome::Name(b"origin.example.test".to_vec()));
    assert_eq!(consumed, fragmented);
}

#[test]
fn incomplete_clienthello_reaches_the_absolute_deadline() {
    use std::io::Write;
    use std::net::{TcpListener, TcpStream};
    use std::time::Duration;
    let listener = TcpListener::bind("127.0.0.1:0").unwrap();
    let address = listener.local_addr().unwrap();
    let server = std::thread::spawn(move || {
        let (mut socket, _) = listener.accept().unwrap();
        read_clienthello(&mut socket, Duration::from_millis(100))
            .unwrap()
            .0
    });
    let mut client = TcpStream::connect(address).unwrap();
    client.write_all(&[0x16]).unwrap();
    assert_eq!(server.join().unwrap(), Outcome::Rejected);
}
