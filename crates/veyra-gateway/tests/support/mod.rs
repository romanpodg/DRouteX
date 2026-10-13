//! Shared test helpers.
#![allow(dead_code)] // each test file uses a different subset

/// A ClientHello record naming `sni` (none for `None`), then the `extra`
/// extensions as (type, body).
pub fn client_hello(sni: Option<&str>, extra: &[(u16, Vec<u8>)]) -> Vec<u8> {
    let mut extensions = Vec::new();
    let mut push = |kind: u16, body: &[u8]| {
        extensions.extend(kind.to_be_bytes());
        extensions.extend(len16(body.len()));
        extensions.extend(body);
    };
    if let Some(name) = sni {
        push(0, &server_name(name));
    }
    for (kind, body) in extra {
        push(*kind, body);
    }
    let mut body = vec![3, 3]; // legacy_version: TLS 1.2
    body.extend([7; 32]); // random
    body.push(0); // no session id
    body.extend([0, 2, 0x13, 0x01]); // one cipher suite: TLS_AES_128_GCM_SHA256
    body.extend([1, 0]); // compression: null only
    body.extend(len16(extensions.len()));
    body.extend(extensions);
    let mut record = vec![22, 3, 1];
    record.extend(len16(body.len() + 4));
    record.push(1); // ClientHello
    record.extend(&u32::try_from(body.len()).expect("fits").to_be_bytes()[1..]);
    record.extend(body);
    record
}

/// The body of a server_name extension with one host_name entry.
pub fn server_name(name: &str) -> Vec<u8> {
    let mut entry = vec![0]; // host_name
    entry.extend(len16(name.len()));
    entry.extend(name.as_bytes());
    let mut body = len16(entry.len()).to_vec();
    body.extend(entry);
    body
}

fn len16(len: usize) -> [u8; 2] {
    u16::try_from(len)
        .expect("length fits in 16 bits")
        .to_be_bytes()
}

/// Bytes from a hex dump; whitespace is ignored.
pub fn hex(text: &str) -> Vec<u8> {
    let digits: Vec<u8> = text.bytes().filter(|b| !b.is_ascii_whitespace()).collect();
    digits
        .chunks(2)
        .map(|pair| {
            let pair = std::str::from_utf8(pair).expect("ASCII");
            u8::from_str_radix(pair, 16).expect("hex digit pair")
        })
        .collect()
}
