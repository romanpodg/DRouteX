use droutex_common::MAX_LOCAL_INPUT_BYTES;
use droutex_policy::{
    BlocklistFormat as Format, CatalogParseCode as Code, DomainMatchMode as Mode,
    MAX_POLICY_ENTRIES, blocklist_entry_count, parse_blocklist,
};
use std::error::Error;

const DOMAINS: &[u8] = include_bytes!("../../../tests/fixtures/catalogs/domains.txt");
const HOSTS: &[u8] = include_bytes!("../../../tests/fixtures/catalogs/blocking.hosts");

#[test]
fn original_domain_and_hosts_fixtures_normalize_without_becoming_local_answers() {
    let list = parse_blocklist(DOMAINS, Format::Domains, Mode::DomainAndSubdomains).unwrap();
    assert_eq!(list.raw_entries(), 3);
    assert_eq!(
        list.entries()
            .iter()
            .map(|e| e.domain().as_str())
            .collect::<Vec<_>>(),
        ["cdn--edge.example", "xn--bcher-kva.example"]
    );
    let hosts = parse_blocklist(HOSTS, Format::BlockingHosts, Mode::Exact).unwrap();
    assert_eq!(hosts.raw_entries(), 5);
    assert_eq!(hosts.entries().len(), 5);
    assert!(hosts.entries().iter().all(|e| e.matches(e.domain())));
    let debug = format!("{list:?} {hosts:?}");
    assert!(!debug.contains("bcher"));
    assert!(!debug.contains("ads.example"));
}

#[test]
fn explicit_modes_keep_apex_and_subdomain_behavior_and_bom_crlf_comments_are_supported() {
    let text = "\u{feff}# own fixture\r\n\tBÜCHER.Example. # comment\r\n";
    for (mode, apex, child) in [
        (Mode::Exact, true, false),
        (Mode::DomainAndSubdomains, true, true),
        (Mode::SubdomainsOnly, false, true),
    ] {
        let list = parse_blocklist(text.as_bytes(), Format::Domains, mode).unwrap();
        let matcher = &list.entries()[0];
        assert_eq!(
            matcher.matches(&"xn--bcher-kva.example".parse().unwrap()),
            apex
        );
        assert_eq!(
            matcher.matches(&"x.xn--bcher-kva.example".parse().unwrap()),
            child
        );
        assert!(!matcher.matches(&"notxn--bcher-kva.example".parse().unwrap()));
    }
}

#[test]
fn unsupported_or_malformed_lines_refuse_the_whole_input_with_sanitized_location() {
    let cases: &[(&[u8], Format, Code)] = &[
        (b"", Format::Domains, Code::Empty),
        (b" # comment\n", Format::Domains, Code::Empty),
        (b"\xff", Format::Domains, Code::Utf8),
        (b"a.example\0", Format::Domains, Code::Format),
        (b"a.example\rb.example", Format::Domains, Code::Format),
        (b"a.example\x0cb.example", Format::Domains, Code::Format),
        (b"a.example b.example", Format::Domains, Code::Format),
        (b"https://a.example/path", Format::Domains, Code::Domain),
        (b"*.a.example", Format::Domains, Code::Domain),
        (b"||a.example^", Format::Domains, Code::Domain),
        (b"a.example:443", Format::Domains, Code::Domain),
        (b"192.0.2.1 a.example", Format::BlockingHosts, Code::Format),
        (b"0.0.0.0", Format::BlockingHosts, Code::Format),
        (
            b"0.0.0.0 a.example\n::1 broken..example",
            Format::BlockingHosts,
            Code::Domain,
        ),
    ];
    for (input, format, code) in cases {
        let error = parse_blocklist(input, *format, Mode::Exact).unwrap_err();
        assert_eq!(error.code, *code, "{input:?}");
        assert!(error.source().is_none());
        assert!(!format!("{error:?} {error}").contains("a.example"));
    }
    assert_eq!(
        parse_blocklist(
            "a.example\n\u{feff}b.example".as_bytes(),
            Format::Domains,
            Mode::Exact
        )
        .unwrap_err()
        .code,
        Code::Format
    );
    assert_eq!(
        parse_blocklist(
            b"valid.example\nbroken..example",
            Format::Domains,
            Mode::Exact
        )
        .unwrap_err()
        .line,
        Some(2)
    );
}

#[test]
fn raw_duplicates_aliases_and_exact_byte_limit_are_charged_before_deduplication() {
    let exact = "a.example\n".repeat(MAX_POLICY_ENTRIES);
    let list = parse_blocklist(exact.as_bytes(), Format::Domains, Mode::Exact).unwrap();
    assert_eq!(
        (list.raw_entries(), list.entries().len()),
        (MAX_POLICY_ENTRIES, 1)
    );
    assert_eq!(
        blocklist_entry_count((exact + "a.example").as_bytes(), Format::Domains)
            .unwrap_err()
            .code,
        Code::EntryLimit
    );
    let aliases = format!("0.0.0.0 {}", "a.example ".repeat(MAX_POLICY_ENTRIES + 1));
    assert_eq!(
        blocklist_entry_count(aliases.as_bytes(), Format::BlockingHosts)
            .unwrap_err()
            .code,
        Code::EntryLimit
    );
    let mut bytes = b"a.example\n#".to_vec();
    bytes.resize(MAX_LOCAL_INPUT_BYTES, b'x');
    assert!(parse_blocklist(&bytes, Format::Domains, Mode::Exact).is_ok());
    bytes.push(b'x');
    assert_eq!(
        blocklist_entry_count(&bytes, Format::Domains)
            .unwrap_err()
            .code,
        Code::Size
    );
}
