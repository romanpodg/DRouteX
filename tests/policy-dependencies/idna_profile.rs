//! D37 qualification only: no production policy dependency or normalization adoption.
use idna::uts46::{AsciiDenyList, DnsLength, Hyphens, Uts46};

fn raw_ascii(input: &str) -> Result<String, idna::Errors> {
    Uts46::new()
        .to_ascii(
            input.as_bytes(),
            AsciiDenyList::URL,
            Hyphens::Allow,
            DnsLength::VerifyAllowRootDot,
        )
        .map(|value| value.into_owned())
}
fn ascii(input: &str) -> Result<String, idna::Errors> {
    let canonical = raw_ascii(input)?;
    let (unicode, status) =
        Uts46::new().to_unicode(canonical.as_bytes(), AsciiDenyList::URL, Hyphens::Allow);
    status?;
    if raw_ascii(&unicode)? != canonical {
        return Err(idna::Errors::default());
    }
    Ok(canonical)
}
fn policy_name(input: &str) -> Result<String, ()> {
    if input.len() > 1024 {
        return Err(());
    }
    let ascii = ascii(input).map_err(|_| ())?;
    let result = ascii.strip_suffix('.').unwrap_or(&ascii);
    if result.is_empty()
        || result.len() > 253
        || result.split('.').any(|label| {
            label.is_empty()
                || label.len() > 63
                || !label
                    .bytes()
                    .all(|b| b.is_ascii_lowercase() || b.is_ascii_digit() || b == b'-' || b == b'_')
        })
    {
        return Err(());
    }
    Ok(result.to_owned())
}

#[test]
fn unicode_ace_nontransitional_and_canonical_forms() {
    for (input, expected) in [
        ("BÜCHER.Example.", "xn--bcher-kva.example"),
        ("bücher.example", "xn--bcher-kva.example"),
        ("XN--BCHER-KVA.EXAMPLE", "xn--bcher-kva.example"),
        ("faß.de", "xn--fa-hia.de"),
        ("fass.de", "fass.de"),
        ("ς.gr", "xn--3xa.gr"),
        ("σ.gr", "xn--4xa.gr"),
        ("e\u{301}.example", "xn--9ca.example"),
        ("é.example", "xn--9ca.example"),
        ("example\u{3002}com\u{ff0e}", "example.com"),
        ("LOCALHOST", "localhost"),
    ] {
        assert_eq!(policy_name(input).unwrap(), expected, "{input:?}");
        assert_eq!(policy_name(expected).unwrap(), expected);
    }
}

#[test]
fn compatibility_is_explicit_not_a_strict_fallback() {
    for name in [
        "r3---sn.example",
        "-user.example",
        "user-.example",
        "_dmarc.example",
        "_sip._tcp.example",
    ] {
        assert!(
            idna::domain_to_ascii_strict(name).is_err(),
            "strict fixture {name}"
        );
        assert_eq!(policy_name(name).unwrap(), name);
    }
}

#[test]
fn malformed_nested_ace_requires_library_roundtrip_without_custom_idna() {
    let name = "xn--xn--a--gua.pt";
    assert!(raw_ascii(name).is_ok()); // Captured fail-open of the bare selected primitive.
    assert!(ascii(name).is_err());
    assert!(policy_name(name).is_err());
    for name in ["xn--bcher-kva.example", "r3---sn.example", "_dmarc.example"] {
        assert_eq!(ascii(name).unwrap(), name);
    }
}

#[test]
fn invalid_unicode_ace_bidi_joiners_punctuation_and_patterns_refuse() {
    for name in [
        "",
        ".",
        "a..b",
        "example.com..",
        "a b.com",
        " a.com",
        "a.com ",
        "a\n.com",
        "xn--.example",
        "xn---.example",
        "xn--a.example",
        "a\u{200d}b.example",
        "a\u{200c}b.example",
        "אa.example",
        "\u{fffd}.example",
        "\u{1}.example",
        "*.example",
        "server-*.example",
        "[a-z].example",
        "example.com/path",
        "https://example.com",
        "a:443",
        "a@b",
        "a%20b",
        "a!b",
        "a$b",
        "a&b",
        "a+b",
        "a=b",
        "a,b",
        "a;b",
        "a(b)",
    ] {
        assert!(policy_name(name).is_err(), "accepted {name:?}");
    }
    assert!(
        Uts46::new()
            .to_ascii(
                &[0xff, 0xfe],
                AsciiDenyList::URL,
                Hyphens::Allow,
                DnsLength::VerifyAllowRootDot
            )
            .is_err()
    );
}

#[test]
fn length_limits_apply_before_and_after_library_mapping() {
    let maximum = format!(
        "{}.{}.{}.{}",
        "a".repeat(63),
        "b".repeat(63),
        "c".repeat(63),
        "d".repeat(61)
    );
    assert_eq!(maximum.len(), 253);
    assert_eq!(policy_name(&maximum).unwrap(), maximum);
    assert_eq!(policy_name(&format!("{maximum}.")).unwrap(), maximum);
    assert!(policy_name(&format!("{maximum}e")).is_err());
    assert!(policy_name(&format!("{}.example", "a".repeat(64))).is_err());
    // UTS46 ignores soft hyphens; the independent input cap must still apply.
    assert!(ascii(&format!("{}a.example", "\u{00ad}".repeat(512))).is_ok());
    assert!(policy_name(&format!("{}a.example", "\u{00ad}".repeat(512))).is_err());
    assert!(policy_name(&format!("{}.example", "é".repeat(64))).is_err());
    let labels = vec!["a"; 127].join(".");
    assert_eq!(policy_name(&labels).unwrap(), labels);
    assert!(policy_name(&vec!["a"; 128].join(".")).is_err());
}

#[test]
fn generated_canonicalization_is_idempotent_and_label_preserving() {
    for first in ["bücher", "faß", "é", "a-b", "_service"] {
        for depth in 0..8 {
            let name = format!("{}{}.Example.", "node.".repeat(depth), first);
            let canonical = policy_name(&name).unwrap();
            assert_eq!(policy_name(&canonical).unwrap(), canonical);
            assert_eq!(canonical.split('.').count(), depth + 2);
            assert!(canonical.ends_with(".example"));
        }
    }
}

#[test]
fn official_non_scalar_cases_refuse_at_the_utf8_boundary() {
    // Unicode17 fixture lines548-549 contain a lone U+D900 between a/z and A/Z.
    // These WTF-8 byte fixtures are not valid Rust str or legal UTF-8 input.
    for input in [
        &[b'a', 0xed, 0xa4, 0x80, b'z'],
        &[b'A', 0xed, 0xa4, 0x80, b'Z'],
    ] {
        assert!(std::str::from_utf8(input).is_err());
        assert!(
            Uts46::new()
                .to_ascii(
                    input,
                    AsciiDenyList::URL,
                    Hyphens::Allow,
                    DnsLength::VerifyAllowRootDot
                )
                .is_err()
        );
    }
}

mod vectors;

#[test]
fn official_unicode17_vectors_for_shared_success_and_required_error_conditions() {
    vectors::check(policy_name);
}
