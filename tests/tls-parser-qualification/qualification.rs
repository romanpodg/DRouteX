use tls_parser::{
    MAX_RECORD_LEN, TlsExtension, TlsExtensionType, TlsMessage, TlsMessageHandshake, TlsRecordType,
    parse_tls_client_hello_extensions, parse_tls_message_handshake, parse_tls_plaintext,
    parse_tls_raw_record,
};

fn client_hello() -> Vec<u8> {
    let input = include_str!("client-hello.hex").trim().as_bytes();
    assert_eq!(input.len() % 2, 0);
    input
        .as_chunks::<2>()
        .0
        .iter()
        .map(|hex| u8::from_str_radix(std::str::from_utf8(hex).unwrap(), 16).unwrap())
        .collect()
}
fn record(payload: &[u8]) -> Vec<u8> {
    let mut bytes = vec![0x16, 0x03, 0x01];
    bytes.extend_from_slice(&u16::try_from(payload.len()).unwrap().to_be_bytes());
    bytes.extend_from_slice(payload);
    bytes
}

#[test]
fn real_openssl_clienthello_preserves_sni_and_bytes() {
    let input = client_hello();
    let (remaining, hello) = parse_tls_plaintext(&input).unwrap();
    assert!(remaining.is_empty());
    let [TlsMessage::Handshake(TlsMessageHandshake::ClientHello(hello))] = hello.msg.as_slice()
    else {
        panic!("expected a single ClientHello");
    };
    let (tail, extensions) = parse_tls_client_hello_extensions(hello.ext.unwrap()).unwrap();
    assert!(tail.is_empty());
    assert!(extensions.iter().any(|ext| matches!(ext, TlsExtension::SNI(names) if names.iter().any(|(_,name)| *name == b"origin.example.test"))));
    let (tail, raw) = parse_tls_raw_record(&input).unwrap();
    assert!(tail.is_empty());
    assert_eq!(raw.hdr.record_type, TlsRecordType::Handshake);
    assert_eq!(raw.data, &input[5..]);
}

#[test]
fn tcp_fragmentation_requires_complete_record_and_preserves_all_bytes() {
    let input = client_hello();
    for split in 0..input.len() {
        assert!(
            parse_tls_raw_record(&input[..split]).is_err(),
            "prefix {split}"
        );
    }
    let (tail, record) = parse_tls_raw_record(&input).unwrap();
    assert!(tail.is_empty());
    assert_eq!(record.data.len(), input.len() - 5);
}

#[test]
fn handshake_split_across_records_requires_reassembly() {
    let input = client_hello();
    let (_, original) = parse_tls_raw_record(&input).unwrap();
    for split in [1, 3, 4, 17, original.data.len() - 1] {
        let first = record(&original.data[..split]);
        let second = record(&original.data[split..]);
        assert!(parse_tls_plaintext(&first).is_err(), "split {split}");
        let (_, one) = parse_tls_raw_record(&first).unwrap();
        let (_, two) = parse_tls_raw_record(&second).unwrap();
        let mut joined = one.data.to_vec();
        joined.extend_from_slice(two.data);
        assert_eq!(joined, original.data);
        let (remaining, message) = parse_tls_message_handshake(&joined).unwrap();
        assert!(remaining.is_empty());
        assert!(matches!(
            message,
            TlsMessage::Handshake(TlsMessageHandshake::ClientHello(_))
        ));
    }
}

#[test]
fn ech_extension_type_is_preserved_as_unknown() {
    let wire = [0xfe, 0x0d, 0, 4, 1, 2, 3, 4];
    let (tail, extensions) = parse_tls_client_hello_extensions(&wire).unwrap();
    assert!(tail.is_empty());
    assert!(
        matches!(extensions.as_slice(), [TlsExtension::Unknown(TlsExtensionType(0xfe0d), data)] if *data == [1,2,3,4])
    );
}

#[test]
fn extension_list_parser_can_leave_malformed_trailing_input() {
    let wire = [0xfe, 0x0d, 0, 1, 1, 0, 0, 0];
    let (tail, extensions) = parse_tls_client_hello_extensions(&wire).unwrap();
    assert_eq!(extensions.len(), 1);
    assert_eq!(tail, &[0, 0, 0]);
}

#[test]
fn sni_parser_silently_discards_an_incomplete_name_inside_a_list() {
    // ServerName list: one legal one-byte hostname followed by a truncated entry.
    let wire = [0, 0, 0, 7, 0, 5, 0, 0, 1, b'a', 0xff];
    let (tail, extensions) = parse_tls_client_hello_extensions(&wire).unwrap();
    assert!(tail.is_empty());
    assert!(
        matches!(extensions.as_slice(), [TlsExtension::SNI(names)] if names.len() == 1 && names[0].1 == b"a")
    );
}

#[test]
fn handshake_parser_discards_unparsed_bytes_inside_declared_clienthello() {
    let input = client_hello();
    let (_, raw) = parse_tls_raw_record(&input).unwrap();
    let mut handshake = raw.data.to_vec();
    let size = ((usize::from(handshake[1])) << 16)
        | ((usize::from(handshake[2])) << 8)
        | usize::from(handshake[3]);
    assert_eq!(size + 4, handshake.len());
    assert!(size < 0xffff);
    let new_size = size + 1;
    handshake[1] = (new_size >> 16) as u8;
    handshake[2] = (new_size >> 8) as u8;
    handshake[3] = new_size as u8;
    handshake.push(0xff);
    let (tail, parsed) = parse_tls_message_handshake(&handshake).unwrap();
    assert!(tail.is_empty());
    assert!(matches!(
        parsed,
        TlsMessage::Handshake(TlsMessageHandshake::ClientHello(_))
    ));
}

#[test]
fn oversized_and_truncated_records_are_not_complete() {
    let too_large = [
        0x16,
        3,
        3,
        ((MAX_RECORD_LEN + 1) >> 8) as u8,
        (MAX_RECORD_LEN + 1) as u8,
    ];
    assert!(parse_tls_raw_record(&too_large).is_err());
    assert!(parse_tls_plaintext(&too_large).is_err());
    let short = [0x16, 3, 3, 0, 8, 1, 2];
    assert!(parse_tls_raw_record(&short).is_err());
}

mod guarded;

#[test]
fn deterministic_mutation_corpus_does_not_panic() {
    let original = client_hello();
    for index in 0..original.len() {
        for byte in [0, 0xff] {
            let mut input = original.clone();
            input[index] = byte;
            let result = std::panic::catch_unwind(|| {
                let _ = parse_tls_raw_record(&input);
                let _ = parse_tls_plaintext(&input);
            });
            assert!(
                result.is_ok(),
                "mutation at offset {index}, byte {byte:02x}"
            );
        }
    }
}
