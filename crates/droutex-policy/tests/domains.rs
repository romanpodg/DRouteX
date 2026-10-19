use droutex_policy::{CanonicalDomain, DomainError, DomainMatchMode as Mode, DomainMatcher};
use std::error::Error;

#[path = "../../../tests/policy-dependencies/vectors.rs"]
mod vectors;

fn name(value: &str) -> CanonicalDomain {
    value.parse().unwrap()
}

#[test]
fn production_boundary_passes_official_unicode17_profile_vectors() {
    vectors::check(|input| {
        input
            .parse::<CanonicalDomain>()
            .map(|name| name.as_str().to_owned())
            .map_err(|_| ())
    });
}

#[test]
fn canonical_equivalence_and_nontransitional_distinctions() {
    for (input, expected) in [
        ("BÜCHER.Example.", "xn--bcher-kva.example"),
        ("XN--BCHER-KVA.EXAMPLE", "xn--bcher-kva.example"),
        ("e\u{301}.example", "xn--9ca.example"),
        ("é.example", "xn--9ca.example"),
        ("example\u{3002}com\u{ff0e}", "example.com"),
        ("LOCALHOST", "localhost"),
        ("_sip._tcp.example", "_sip._tcp.example"),
        ("r3---sn.example", "r3---sn.example"),
        ("-user.example", "-user.example"),
        ("user-.example", "user-.example"),
        ("127.0.0.1", "127.0.0.1"), // A textual DNS name; no address coercion.
    ] {
        let canonical = name(input);
        assert_eq!(canonical.as_str(), expected, "{input:?}");
        assert_eq!(canonical, name(expected));
    }
    assert_ne!(name("faß.de"), name("fass.de"));
    assert_ne!(name("ς.gr"), name("σ.gr"));
}

#[test]
fn invalid_names_never_become_matchers_or_leak_into_errors() {
    for input in [
        "",
        ".",
        "a..b",
        "example.com..",
        "xn--xn--a--gua.pt",
        "xn--",
        "xn--a.example",
        "a\u{200d}b.example",
        "אa.example",
        " a.com",
        "a.com ",
        "a\n.com",
        "*.example",
        "server-*.example",
        "[a-z].example",
        "https://example.com",
        "example.com/path",
        "example.com:443",
        "a@b",
        "a%20b",
        "a!b",
        "a\\b",
        "::1",
    ] {
        let error = input.parse::<CanonicalDomain>().unwrap_err();
        assert_eq!(error, DomainError::InvalidName, "{input:?}");
        assert_eq!(error.to_string(), "invalid DNS policy name");
        assert!(error.source().is_none());
    }
}

#[test]
fn byte_label_and_name_limits_apply_before_and_after_mapping() {
    let maximum = format!(
        "{}.{}.{}.{}",
        "a".repeat(63),
        "b".repeat(63),
        "c".repeat(63),
        "d".repeat(61)
    );
    assert_eq!(maximum.len(), 253);
    assert_eq!(name(&maximum), name(&format!("{maximum}.")));
    for input in [
        format!("{maximum}e"),
        format!("{}.example", "a".repeat(64)),
        format!("{}.example", "é".repeat(64)),
        vec!["a"; 128].join("."),
    ] {
        assert_eq!(
            input.parse::<CanonicalDomain>(),
            Err(DomainError::InvalidName)
        );
    }
    assert!(vec!["a"; 127].join(".").parse::<CanonicalDomain>().is_ok());
    let at_limit = format!("{}a.", "\u{00ad}".repeat(511));
    assert_eq!(at_limit.len(), 1024);
    assert_eq!(name(&at_limit).as_str(), "a");
    assert_eq!(
        format!("{at_limit}\u{00ad}").parse::<CanonicalDomain>(),
        Err(DomainError::InputTooLong)
    );
    assert_eq!(
        "a".repeat(1025).parse::<CanonicalDomain>(),
        Err(DomainError::InputTooLong)
    );
}

#[test]
fn every_mode_has_explicit_apex_and_label_boundary_semantics() {
    for base in [
        "example.com",
        "bücher.example",
        "_service.local",
        "localhost",
    ] {
        let canonical = name(base);
        let canonical = canonical.as_str();
        for (query, exact, suffix, wildcard) in [
            (base.to_owned(), true, true, false),
            (format!("{}.", base.to_uppercase()), true, true, false),
            (format!("child.{canonical}"), false, true, true),
            (format!("grand.child.{canonical}"), false, true, true),
            (format!("bad{canonical}"), false, false, false),
            (format!("{canonical}.attacker"), false, false, false),
            ("unrelated.invalid".to_owned(), false, false, false),
        ] {
            for (mode, expected) in [
                (Mode::Exact, exact),
                (Mode::DomainAndSubdomains, suffix),
                (Mode::SubdomainsOnly, wildcard),
            ] {
                let matcher = DomainMatcher::new(name(base), mode);
                assert_eq!(
                    matcher.matches(&name(&query)),
                    expected,
                    "{base:?} {mode:?} {query:?}"
                );
            }
        }
    }
}

#[test]
fn generated_equivalence_and_descendant_depth_are_deterministic() {
    for base in [
        "bücher.example",
        "faß.de",
        "é.example",
        "r3---sn.example",
        "_service.local",
    ] {
        let matcher = DomainMatcher::new(name(base), Mode::SubdomainsOnly);
        for depth in 1..16 {
            let input = format!("{}{base}.", "NODE.".repeat(depth));
            let canonical = name(&input);
            assert_eq!(canonical, name(canonical.as_str()));
            assert!(matcher.matches(&canonical));
            assert!(!matcher.matches(&name(&format!("{}-attacker", canonical.as_str()))));
        }
    }
}
