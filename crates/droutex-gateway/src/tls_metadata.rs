//! Experimental bounded ClientHello inspection. The result is untrusted input,
//! not an access grant, route, or permission to connect. The non-default
//! preview listener uses it; independent review and full integration remain open.
use droutex_policy::CanonicalDomain;
use std::{collections::BTreeSet, error::Error, fmt};
use tls_parser::{
    TlsExtension, TlsRecordType, parse_tls_extension_sni_hostname, parse_tls_extension_unknown,
    parse_tls_handshake_client_hello, parse_tls_raw_record, parse_tls_record_header,
};

pub const MAX_INPUT: usize = 16 * 1024;
const MAX_RECORDS: usize = 16;
const MAX_EXTENSIONS: usize = 128;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum HelloError {
    Invalid,
    Limit,
    EncryptedName,
}
impl fmt::Display for HelloError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "client hello {self:?}")
    }
}
impl Error for HelloError {}

/// An observed name is not a gateway identity or proof of authorization.
#[derive(Debug, Eq, PartialEq)]
pub enum Inspection {
    Pending,
    Complete(CanonicalDomain),
}

/// Inspect caller-owned bytes without consuming or altering them. The caller
/// must bound reads and elapsed time and retain all received bytes for replay.
pub fn inspect(input: &[u8]) -> Result<Inspection, HelloError> {
    if input.len() > MAX_INPUT {
        return Err(HelloError::Limit);
    }
    let mut wire = input;
    let mut handshake = Vec::new();
    for _ in 0..MAX_RECORDS {
        let Some(HandshakeRecord { tail, data }) = handshake_record(wire)? else {
            return Ok(Inspection::Pending);
        };
        if handshake.len() + data.len() > MAX_INPUT {
            return Err(HelloError::Limit);
        }
        handshake.extend_from_slice(data);
        if let Some(body) = complete_hello(&handshake)? {
            return parse_body(body).map(Inspection::Complete);
        }
        wire = tail;
    }
    Err(HelloError::Limit)
}
/// Accept only a complete bounded TLS handshake record; incomplete data is not
/// rejected until the fixed input/read deadline has been reached.
struct HandshakeRecord<'a> {
    tail: &'a [u8],
    data: &'a [u8],
}
fn handshake_record(wire: &[u8]) -> Result<Option<HandshakeRecord<'_>>, HelloError> {
    if wire.len() < 5 {
        return Ok(None);
    }
    let (_, header) = parse_tls_record_header(wire).map_err(|_| HelloError::Invalid)?;
    if header.record_type != TlsRecordType::Handshake
        || !(0x0301..=0x0303).contains(&header.version.0)
        || header.len == 0
        || usize::from(header.len) > 16_384
    {
        return Err(HelloError::Invalid);
    }
    if wire.len() < 5 + usize::from(header.len) {
        return Ok(None);
    }
    let (tail, record) = parse_tls_raw_record(wire).map_err(|_| HelloError::Invalid)?;
    Ok(Some(HandshakeRecord {
        tail,
        data: record.data,
    }))
}
fn complete_hello(handshake: &[u8]) -> Result<Option<&[u8]>, HelloError> {
    if handshake.len() < 4 {
        return Ok(None);
    }
    if handshake[0] != 1 {
        return Err(HelloError::Invalid);
    }
    let len = (usize::from(handshake[1]) << 16)
        | (usize::from(handshake[2]) << 8)
        | usize::from(handshake[3]);
    if len + 4 > MAX_INPUT {
        return Err(HelloError::Limit);
    }
    Ok((handshake.len() >= len + 4).then(|| &handshake[4..len + 4]))
}
fn parse_body(body: &[u8]) -> Result<CanonicalDomain, HelloError> {
    let (tail, hello) = parse_tls_handshake_client_hello(body).map_err(|_| HelloError::Invalid)?;
    if !tail.is_empty()
        || hello.ciphers.is_empty()
        || !hello.comp.iter().any(|method| method.0 == 0)
    {
        return Err(HelloError::Invalid);
    }
    parse_extensions(hello.ext.ok_or(HelloError::Invalid)?)
}

fn parse_extensions(mut extensions: &[u8]) -> Result<CanonicalDomain, HelloError> {
    let mut seen = BTreeSet::new();
    let mut name = None;
    while !extensions.is_empty() {
        if seen.len() >= MAX_EXTENSIONS {
            return Err(HelloError::Limit);
        }
        let (tail, TlsExtension::Unknown(kind, data)) =
            parse_tls_extension_unknown(extensions).map_err(|_| HelloError::Invalid)?
        else {
            return Err(HelloError::Invalid);
        };
        if !seen.insert(kind.0) {
            return Err(HelloError::Invalid);
        }
        if kind.0 == 0xfe0d || kind.0 == 0xffce {
            return Err(HelloError::EncryptedName);
        }
        if kind.0 == 0 {
            name = Some(parse_server_name(data)?);
        }
        extensions = tail;
    }
    name.ok_or(HelloError::Invalid)
}
fn parse_server_name(data: &[u8]) -> Result<CanonicalDomain, HelloError> {
    if data.len() < 2 || usize::from(u16::from_be_bytes([data[0], data[1]])) != data.len() - 2 {
        return Err(HelloError::Invalid);
    }
    let (tail, (kind, host)) =
        parse_tls_extension_sni_hostname(&data[2..]).map_err(|_| HelloError::Invalid)?;
    if !tail.is_empty() || kind.0 != 0 || host.is_empty() || !host.is_ascii() {
        return Err(HelloError::Invalid);
    }
    let host = std::str::from_utf8(host).map_err(|_| HelloError::Invalid)?;
    if host.ends_with('.') || host.parse::<std::net::IpAddr>().is_ok() {
        return Err(HelloError::Invalid);
    }
    host.parse::<CanonicalDomain>()
        .map_err(|_| HelloError::Invalid)
}

const MAX_READS: usize = 256;

/// Original bytes and observed name only; neither grants access or a route.
pub struct ObservedHello<R> {
    name: CanonicalDomain,
    bytes: Vec<u8>,
    stream: R,
}
impl<R> fmt::Debug for ObservedHello<R> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("ObservedHello")
            .field("bytes", &self.bytes.len())
            .finish_non_exhaustive()
    }
}
impl<R> ObservedHello<R> {
    pub fn name(&self) -> &CanonicalDomain {
        &self.name
    }
    pub fn bytes(&self) -> &[u8] {
        &self.bytes
    }
    pub(crate) fn into_parts(self) -> (R, CanonicalDomain, Vec<u8>) {
        (self.stream, self.name, self.bytes)
    }
}

/// Read one ClientHello under a fixed deadline and bounded number of reads.
/// This does not authenticate the peer or grant permission to forward bytes.
pub async fn read_hello<R: tokio::io::AsyncRead + Unpin>(
    mut reader: R,
    deadline: tokio::time::Instant,
) -> Result<ObservedHello<R>, HelloError> {
    use tokio::io::AsyncReadExt;
    let mut bytes = Vec::new();
    for _ in 0..MAX_READS {
        if tokio::time::Instant::now() >= deadline {
            return Err(HelloError::Invalid);
        }
        let mut chunk = [0u8; 1024];
        let room = MAX_INPUT - bytes.len();
        if room == 0 {
            return Err(HelloError::Limit);
        }
        let size = tokio::time::timeout_at(deadline, reader.read(&mut chunk[..room.min(1024)]))
            .await
            .map_err(|_| HelloError::Invalid)?
            .map_err(|_| HelloError::Invalid)?;
        if tokio::time::Instant::now() >= deadline || size == 0 {
            return Err(HelloError::Invalid);
        }
        bytes.extend_from_slice(&chunk[..size]);
        match inspect(&bytes)? {
            Inspection::Pending => (),
            Inspection::Complete(name) => {
                return Ok(ObservedHello {
                    name,
                    bytes,
                    stream: reader,
                });
            }
        }
    }
    Err(HelloError::Limit)
}

#[cfg(test)]
pub(crate) mod tests {
    use super::*;
    use tokio::{
        io::AsyncWriteExt,
        time::{Duration, Instant},
    };

    fn fixture() -> Vec<u8> {
        include_str!("../../../tests/tls-parser-qualification/client-hello.hex")
            .trim()
            .as_bytes()
            .as_chunks::<2>()
            .0
            .iter()
            .map(|pair| u8::from_str_radix(std::str::from_utf8(pair).unwrap(), 16).unwrap())
            .collect()
    }
    fn record(data: &[u8]) -> Vec<u8> {
        let mut bytes = vec![0x16, 0x03, 0x01];
        bytes.extend_from_slice(&(data.len() as u16).to_be_bytes());
        bytes.extend_from_slice(data);
        bytes
    }
    fn extension(kind: u16, value: &[u8]) -> Vec<u8> {
        let mut result = kind.to_be_bytes().to_vec();
        result.extend_from_slice(&(value.len() as u16).to_be_bytes());
        result.extend_from_slice(value);
        result
    }
    pub(crate) fn synthetic(extensions: &[u8]) -> Vec<u8> {
        let mut body = vec![3, 3];
        body.extend_from_slice(&[7; 32]);
        body.extend_from_slice(&[0, 0, 2, 0x13, 1, 1, 0]);
        body.extend_from_slice(&(extensions.len() as u16).to_be_bytes());
        body.extend_from_slice(extensions);
        let len = body.len();
        let mut hello = vec![1, (len >> 16) as u8, (len >> 8) as u8, len as u8];
        hello.extend(body);
        record(&hello)
    }
    pub(crate) fn sni(host: &[u8]) -> Vec<u8> {
        let mut value = ((host.len() + 3) as u16).to_be_bytes().to_vec();
        value.push(0);
        value.extend_from_slice(&(host.len() as u16).to_be_bytes());
        value.extend_from_slice(host);
        extension(0, &value)
    }
    #[test]
    fn real_hello_handles_each_prefix_and_record_fragment() {
        let original = fixture();
        for size in 0..original.len() {
            assert_eq!(
                inspect(&original[..size]),
                Ok(Inspection::Pending),
                "prefix {size}"
            );
        }
        assert_eq!(
            inspect(&original),
            Ok(Inspection::Complete("origin.example.test".parse().unwrap()))
        );
        let (_, raw) = parse_tls_raw_record(&original).unwrap();
        for split in 1..raw.data.len() {
            let mut fragmented = record(&raw.data[..split]);
            fragmented.extend(record(&raw.data[split..]));
            assert_eq!(
                inspect(&fragmented),
                Ok(Inspection::Complete("origin.example.test".parse().unwrap())),
                "split {split}"
            );
        }
    }
    #[test]
    fn refuses_encrypted_names_duplicate_extensions_and_invalid_sni() {
        let good = sni(b"service.test");
        let mut ech = good.clone();
        ech.extend(extension(0xfe0d, &[0]));
        assert_eq!(inspect(&synthetic(&ech)), Err(HelloError::EncryptedName));
        let mut esni = good.clone();
        esni.extend(extension(0xffce, &[0]));
        assert_eq!(inspect(&synthetic(&esni)), Err(HelloError::EncryptedName));
        let mut duplicate = good.clone();
        duplicate.extend(&good);
        assert_eq!(inspect(&synthetic(&duplicate)), Err(HelloError::Invalid));
        assert_eq!(
            inspect(&synthetic(&sni(b"bad..test"))),
            Err(HelloError::Invalid)
        );
        assert_eq!(
            inspect(&synthetic(&sni(b"127.0.0.1"))),
            Err(HelloError::Invalid)
        );
        assert_eq!(
            inspect(&synthetic(&sni(b"service.test."))),
            Err(HelloError::Invalid)
        );
        let mut malformed = good;
        malformed.push(0);
        assert_eq!(inspect(&synthetic(&malformed)), Err(HelloError::Invalid));
        assert_eq!(inspect(&vec![0; MAX_INPUT + 1]), Err(HelloError::Limit));
    }
    #[tokio::test]
    async fn reader_preserves_input_and_refuses_slow_or_incomplete_hello() {
        let bytes = fixture();
        let (mut client, server) = tokio::io::duplex(MAX_INPUT);
        client.write_all(&bytes).await.unwrap();
        let hello = read_hello(server, Instant::now() + Duration::from_secs(1))
            .await
            .unwrap();
        assert_eq!(hello.name().as_str(), "origin.example.test");
        assert_eq!(hello.bytes(), bytes);
        let (mut client, server) = tokio::io::duplex(1024);
        client.write_all(&[0x16]).await.unwrap();
        assert!(matches!(
            read_hello(server, Instant::now() + Duration::from_millis(30)).await,
            Err(HelloError::Invalid)
        ));
    }
}
