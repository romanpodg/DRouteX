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

// Test-fixture decoding only. Surrogate escapes cannot be represented by the &str
// policy boundary and are counted separately, never claimed as passed vectors.
fn unescape(text: &str) -> Option<String> {
    if text == "\"\"" {
        return Some(String::new());
    }
    let mut result = String::new();
    let mut chars = text.chars();
    while let Some(c) = chars.next() {
        if c != '\\' {
            result.push(c);
            continue;
        }
        let marker = chars.next().unwrap();
        let code = match marker {
            '\\' => {
                result.push('\\');
                continue;
            }
            'u' => chars.by_ref().take(4).collect::<String>(),
            'x' => {
                assert_eq!(chars.next(), Some('{'));
                chars.by_ref().take_while(|c| *c != '}').collect()
            }
            _ => panic!("unknown official fixture escape"),
        };
        result.push(char::from_u32(u32::from_str_radix(&code, 16).unwrap())?);
    }
    Some(result)
}

#[test]
fn official_unicode17_vectors_for_shared_success_and_required_error_conditions() {
    let mut success = 0;
    let mut refusal = 0;
    let mut flag_specific = 0;
    let mut non_scalar = 0;
    let mut mismatches = Vec::new();
    for (index, line) in include_str!("fixtures/IdnaTestV2-Unicode17.txt")
        .lines()
        .enumerate()
    {
        let line = line.split('#').next().unwrap().trim();
        if line.is_empty() {
            continue;
        }
        let fields: Vec<_> = line.split(';').map(str::trim).collect();
        assert!(fields.len() >= 7);
        let Some(source) = unescape(fields[0]) else {
            non_scalar += 1;
            continue;
        };
        let unicode = if fields[1].is_empty() {
            source.clone()
        } else {
            let Some(unicode) = unescape(fields[1]) else {
                non_scalar += 1;
                continue;
            };
            unicode
        };
        let expected = if fields[3].is_empty() {
            unicode
        } else {
            let Some(expected) = unescape(fields[3]) else {
                non_scalar += 1;
                continue;
            };
            expected
        };
        let status = if fields[4].is_empty() {
            fields[2]
        } else {
            fields[4]
        };
        let status = status.trim_matches(['[', ']']);
        if status.is_empty() {
            if ascii(&source).as_deref().ok() != Some(expected.as_str()) {
                mismatches.push(format!("success line {} source {source:?}", index + 1));
            }
            success += 1;
        } else if status
            .split(',')
            .map(str::trim)
            .any(|code| !matches!(code, "V2" | "V3" | "U1" | "A4_1" | "A4_2"))
        {
            if ascii(&source).is_ok() {
                mismatches.push(format!(
                    "required error line {} status {status}: {source:?}",
                    index + 1
                ));
            }
            refusal += 1;
        } else {
            // These vectors use the strict fixture's hyphen/STD3/root-dot flags.
            // Derive the D37 outcome from its independently supplied ASCII result
            // and the explicit DNS-label/input bounds, never from library output.
            let canonical = expected.strip_suffix('.').unwrap_or(&expected);
            let profile_valid = source.len() <= 1024
                && !canonical.is_empty()
                && canonical.len() <= 253
                && canonical.split('.').all(|label| {
                    !label.is_empty()
                        && label.len() <= 63
                        && label.bytes().all(|byte| {
                            byte.is_ascii_lowercase()
                                || byte.is_ascii_digit()
                                || byte == b'-'
                                || byte == b'_'
                        })
                });
            let actual = policy_name(&source);
            if (profile_valid && actual.as_deref() != Ok(canonical))
                || (!profile_valid && actual.is_ok())
            {
                mismatches.push(format!(
                    "profile-specific line {} status {status}: {source:?}",
                    index + 1
                ));
            }
            flag_specific += 1;
        }
    }
    // This is an explicitly scoped profile check, not full Unicode conformance.
    println!(
        "Unicode17 vectors: shared successes={success}, required refusals={refusal}, profile-specific outcomes={flag_specific}, non-scalar &str-inexpressible={non_scalar}"
    );
    assert_eq!(success + refusal + flag_specific + non_scalar, 6391);
    assert!(
        mismatches.is_empty(),
        "{} qualification mismatches (no waiver): {:#?}",
        mismatches.len(),
        mismatches.iter().take(12).collect::<Vec<_>>()
    );
}
