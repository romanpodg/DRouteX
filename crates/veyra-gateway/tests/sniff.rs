//! Reading the destination from the first bytes of a connection (spec §10.1).

mod support;

use support::{client_hello, hex, server_name};
use veyra_gateway::{MAX_HEAD, Service, Sniff, sniff};
use veyra_policy::Name;

/// Captured from `openssl s_client -servername Media.Example` (OpenSSL 3.2).
const OPENSSL: &str = include_str!("fixtures/clienthello-openssl.hex");
/// Captured from `curl https://media.example/` on Windows (Schannel).
const SCHANNEL: &str = include_str!("fixtures/clienthello-schannel.hex");

fn named(host: &str) -> Sniff {
    Sniff::Name(Name::parse(host).expect("test name parses"))
}

#[test]
fn reads_the_sni_of_real_client_hellos() {
    assert_eq!(sniff(Service::Tls, &hex(OPENSSL)), named("media.example"));
    assert_eq!(sniff(Service::Tls, &hex(SCHANNEL)), named("media.example"));
}

#[test]
fn asks_for_more_until_the_client_hello_is_complete() {
    for hello in [
        hex(OPENSSL),
        hex(SCHANNEL),
        client_hello(Some("media.example"), &[]),
    ] {
        for len in 0..hello.len() {
            assert_eq!(
                sniff(Service::Tls, &hello[..len]),
                Sniff::More,
                "{len} bytes"
            );
        }
    }
}

#[test]
fn reads_a_client_hello_split_across_records() {
    let hello = hex(OPENSSL);
    let handshake = &hello[5..];
    let mut split = Vec::new();
    for part in [&handshake[..100], &handshake[100..]] {
        split.extend([22, 3, 1]);
        split.extend(u16::try_from(part.len()).unwrap().to_be_bytes());
        split.extend(part);
    }
    assert_eq!(sniff(Service::Tls, &split[..110]), Sniff::More);
    assert_eq!(sniff(Service::Tls, &split), named("media.example"));
}

#[test]
fn keeps_the_sni_next_to_ech_grease() {
    // Chrome sends an encrypted_client_hello extension even when it has no ECH
    // config; the SNI is then the real name and must still route.
    let hello = client_hello(Some("media.example"), &[(0xfe0d, vec![0; 32])]);
    assert_eq!(sniff(Service::Tls, &hello), named("media.example"));
}

#[test]
fn refuses_client_hellos_without_one_usable_name() {
    let two_names = client_hello(Some("media.example"), &[(0, server_name("other.example"))]);
    for (why, hello) in [
        ("no SNI", client_hello(None, &[])),
        ("an IP literal", client_hello(Some("10.0.0.1"), &[])),
        ("not a domain name", client_hello(Some("bad name!"), &[])),
        ("two server_name extensions", two_names),
        ("HTTP on the TLS port", b"GET / HTTP/1.1\r\n\r\n".to_vec()),
        ("a TLS alert", vec![21, 3, 3, 0, 2, 2, 40]),
    ] {
        assert_eq!(sniff(Service::Tls, &hello), Sniff::Refuse, "{why}");
    }
}

#[test]
fn survives_corrupted_client_hellos() {
    let hello = hex(OPENSSL);
    for i in 0..hello.len() {
        for flip in [0x01, 0x80, 0xff] {
            let mut bad = hello.clone();
            bad[i] ^= flip;
            let _ = sniff(Service::Tls, &bad); // any answer, but no panic
        }
    }
}

#[test]
fn reads_the_host_header() {
    let head = b"GET /watch?v=1 HTTP/1.1\r\nUser-Agent: tv\r\nhOST: Media.Example:80\r\n\r\n";
    assert_eq!(sniff(Service::Http, head), named("media.example"));
    for len in 0..head.len() {
        assert_eq!(
            sniff(Service::Http, &head[..len]),
            Sniff::More,
            "{len} bytes"
        );
    }
}

#[test]
fn refuses_requests_without_one_usable_host() {
    for (why, head) in [
        ("HTTP/1.0 without Host", "GET / HTTP/1.0\r\n\r\n"),
        (
            "two Host headers",
            "GET / HTTP/1.1\r\nHost: media.example\r\nHost: other.example\r\n\r\n",
        ),
        (
            "a folded Host",
            "GET / HTTP/1.1\r\nHost: media.example\r\n other.example\r\n\r\n",
        ),
        (
            "an IPv4 literal",
            "GET / HTTP/1.1\r\nHost: 10.0.0.1:80\r\n\r\n",
        ),
        (
            "an IPv6 literal",
            "GET / HTTP/1.1\r\nHost: [::1]:80\r\n\r\n",
        ),
        (
            "a bad port",
            "GET / HTTP/1.1\r\nHost: media.example:http\r\n\r\n",
        ),
        ("an empty Host", "GET / HTTP/1.1\r\nHost:\r\n\r\n"),
    ] {
        assert_eq!(
            sniff(Service::Http, head.as_bytes()),
            Sniff::Refuse,
            "{why}"
        );
    }
    let not_utf8 = b"GET / HTTP/1.1\r\nHost: m\xffdia.example\r\n\r\n";
    assert_eq!(sniff(Service::Http, not_utf8), Sniff::Refuse);
}

#[test]
fn refuses_heads_that_never_name_a_destination() {
    // A ClientHello that claims 16 MiB, sent as 1000-byte records.
    let mut endless = Vec::new();
    let mut payload = vec![1, 0xff, 0xff, 0xff];
    while endless.len() < MAX_HEAD {
        payload.resize(1000, 0);
        endless.extend([22, 3, 1, 0x03, 0xe8]);
        endless.extend(&payload);
        payload.clear();
    }
    endless.truncate(MAX_HEAD);
    assert_eq!(sniff(Service::Tls, &endless[..MAX_HEAD - 1]), Sniff::More);
    assert_eq!(sniff(Service::Tls, &endless), Sniff::Refuse);

    let mut cookie = b"GET / HTTP/1.1\r\nCookie: ".to_vec();
    cookie.resize(MAX_HEAD, b'a');
    assert_eq!(sniff(Service::Http, &cookie[..MAX_HEAD - 1]), Sniff::More);
    assert_eq!(sniff(Service::Http, &cookie), Sniff::Refuse);
}
