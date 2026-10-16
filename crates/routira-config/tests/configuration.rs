use routira_config::{
    ConfigError, ConfigErrorCode as Code, InputFormat, MAX_INPUT_BYTES, MAX_RESOURCE_ENTRIES,
    ReferenceCatalog, parse_configuration,
};
use serde_json::{Value, json};
use std::{error::Error, str::FromStr};

fn id(n: usize) -> String {
    format!("10000000-0000-4000-8000-{n:012x}")
}
fn typed<T: FromStr>(n: usize) -> T
where
    T::Err: std::fmt::Debug,
{
    id(n).parse().unwrap()
}
fn catalog() -> ReferenceCatalog {
    ReferenceCatalog {
        devices: [typed(120)].into(),
        upstreams: [typed(90)].into(),
        secrets: [typed(99)].into(),
        filter_revisions: [typed(101)].into(),
        service_revisions: [typed(100)].into(),
    }
}
fn fixture() -> Value {
    serde_json::from_str(include_str!("fixtures/configuration-v1.json")).unwrap()
}
fn parse(value: &Value) -> Result<routira_config::CheckedConfiguration, ConfigError> {
    parse_configuration(
        &serde_json::to_vec(value).unwrap(),
        InputFormat::Json,
        &catalog(),
    )
}
fn reject(value: &Value, code: Code, field: &str) {
    let error = parse(value).unwrap_err();
    assert_eq!(error.code(), code, "{error}");
    assert_eq!(error.field(), field);
    assert!(error.source().is_none());
}

#[test]
fn same_empty_draft_in_all_three_formats() {
    for (format, input) in [
        (InputFormat::Json, "{\"schemaVersion\":1}"),
        (InputFormat::Yaml, "schemaVersion: 1\n"),
        (InputFormat::Toml, "schemaVersion = 1\n"),
    ] {
        let checked =
            parse_configuration(input.as_bytes(), format, &ReferenceCatalog::default()).unwrap();
        assert_eq!(checked.document().schema_version, 1);
        assert!(checked.document().profiles.is_empty());
    }
}

#[test]
fn independent_full_fixture_keeps_dns_profiles_and_gateway_policy_separate() {
    let checked = parse(&fixture()).unwrap();
    let data = checked.document();
    assert_eq!(data.profiles.len(), 2);
    assert_eq!(data.endpoints.len(), 3);
    assert_ne!(data.endpoints[0].profile, data.endpoints[2].profile);
    assert_eq!(data.network_groups[0].traffic_profile, data.profiles[0].id);
    assert_eq!(data.private_records[0].name, data.private_records[1].name);
    assert_ne!(
        data.private_records[0].profile,
        data.private_records[1].profile
    );
    let mut data = fixture();
    data["gatewayGrants"] = json!([]);
    data["profiles"][0]["defaultDnsAction"] = json!({"kind":"gateway", "outbound":id(70)});
    assert!(parse(&data).unwrap().document().gateway_grants.is_empty());
}

#[test]
fn independently_written_profile_documents_have_identical_semantics() {
    let json = format!(
        r#"{{"schemaVersion":1,"profiles":[{{"id":"{}","name":"on","defaultDnsAction":{{"kind":"block"}},"defaultTrafficAction":{{"kind":"deny"}}}}]}}"#,
        id(1)
    );
    let yaml = format!(
        "schemaVersion: 1\nprofiles:\n  - id: {}\n    name: on\n    defaultDnsAction: {{kind: block}}\n    defaultTrafficAction: {{kind: deny}}\n",
        id(1)
    );
    let toml = format!(
        "schemaVersion = 1\n[[profiles]]\nid = '{}'\nname = 'on'\ndefaultDnsAction = {{kind='block'}}\ndefaultTrafficAction = {{kind='deny'}}\n",
        id(1)
    );
    for (format, input) in [
        (InputFormat::Json, json),
        (InputFormat::Yaml, yaml),
        (InputFormat::Toml, toml),
    ] {
        let checked =
            parse_configuration(input.as_bytes(), format, &ReferenceCatalog::default()).unwrap();
        assert_eq!(checked.document().profiles[0].id, typed(1));
        assert_eq!(checked.document().profiles[0].name, "on");
    }
}

#[test]
fn byte_and_utf8_limits_are_enforced_before_parsing() {
    let catalog = ReferenceCatalog::default();
    let mut exact = b"{\"schemaVersion\":1}".to_vec();
    exact.resize(MAX_INPUT_BYTES, b' ');
    assert!(parse_configuration(&exact, InputFormat::Json, &catalog).is_ok());
    exact.push(0xff);
    assert_eq!(
        parse_configuration(&exact, InputFormat::Json, &catalog)
            .unwrap_err()
            .code(),
        Code::InputTooLarge
    );
    assert_eq!(
        parse_configuration(&[0xff], InputFormat::Json, &catalog)
            .unwrap_err()
            .code(),
        Code::InvalidUtf8
    );
}

#[test]
fn maximum_collection_depth_has_positive_and_negative_boundaries() {
    for format in [InputFormat::Json, InputFormat::Yaml, InputFormat::Toml] {
        for (arrays, code) in [(31, Code::InvalidShape), (32, Code::InvalidDocument)] {
            let nested = format!("{}0{}", "[".repeat(arrays), "]".repeat(arrays));
            let input = if format == InputFormat::Toml {
                format!("schemaVersion = 1\nextra = {nested}")
            } else {
                format!("{{\"schemaVersion\":1,\"extra\":{nested}}}")
            };
            let error = parse_configuration(input.as_bytes(), format, &ReferenceCatalog::default())
                .unwrap_err();
            assert_eq!(error.code(), code, "{format:?}, {arrays}: {error}");
        }
    }
}

#[test]
fn duplicates_multi_documents_and_yaml_expansion_features_are_rejected() {
    let cases = [
        (
            InputFormat::Json,
            "{\"schemaVersion\":1,\"schemaVersion\":1}",
        ),
        (InputFormat::Json, "{\"schemaVersion\":1} {}"),
        (InputFormat::Toml, "schemaVersion=1\nschemaVersion=1"),
        (InputFormat::Yaml, "schemaVersion: 1\nschemaVersion: 1"),
        (InputFormat::Yaml, "schemaVersion: 1\n---\nschemaVersion: 1"),
        (InputFormat::Yaml, "schemaVersion: 1\nprofiles: &a []"),
        (InputFormat::Yaml, "schemaVersion: 1\nprofiles: *unknown"),
        (InputFormat::Yaml, "schemaVersion: 1\n<<: {profiles: []}"),
        (
            InputFormat::Yaml,
            "schemaVersion: 1\nprofiles: !include /qualification-only",
        ),
        (InputFormat::Yaml, "schemaVersion: .inf"),
        (InputFormat::Toml, "schemaVersion=inf"),
    ];
    for (format, input) in cases {
        assert_eq!(
            parse_configuration(input.as_bytes(), format, &ReferenceCatalog::default())
                .unwrap_err()
                .code(),
            Code::InvalidDocument,
            "{format:?}: {input}"
        );
    }
    for input in ["schemaVersion: on", "schemaVersion: '${NO_SUBSTITUTION}'"] {
        assert_eq!(
            parse_configuration(
                input.as_bytes(),
                InputFormat::Yaml,
                &ReferenceCatalog::default()
            )
            .unwrap_err()
            .code(),
            Code::InvalidShape
        );
    }
}

#[test]
fn diagnostics_never_echo_values_arbitrary_keys_or_parser_sources() {
    for (format, input) in [
        (
            InputFormat::Json,
            r#"{"schemaVersion":"ERROR_SECRET_CANARY"}"#,
        ),
        (
            InputFormat::Json,
            r#"{"schemaVersion":1,"ERROR_SECRET_CANARY":0}"#,
        ),
        (
            InputFormat::Json,
            r#"{"schemaVersion":1,"profiles":[{"name":"ERROR_SECRET_CANARY","id":"wrong"}]}"#,
        ),
        (InputFormat::Yaml, "schemaVersion: 'ERROR_SECRET_CANARY'"),
        (
            InputFormat::Yaml,
            "schemaVersion: 1\nERROR_SECRET_CANARY: [",
        ),
        (InputFormat::Toml, "schemaVersion='ERROR_SECRET_CANARY'"),
        (
            InputFormat::Toml,
            "schemaVersion=1\nERROR_SECRET_CANARY = [",
        ),
    ] {
        let error = parse_configuration(input.as_bytes(), format, &catalog()).unwrap_err();
        assert!(!format!("{error} {error:?}").contains("ERROR_SECRET_CANARY"));
        assert!(error.source().is_none());
        assert!(error.field().starts_with('$'));
    }
    let mut value = fixture();
    value["outbounds"][1]["transport"]["secret"] = json!("ERROR_SECRET_CANARY");
    let error = parse(&value).unwrap_err();
    // Internally tagged enums buffer their content; report the tracked container
    // rather than parsing or exposing the dependency's original diagnostic.
    assert_eq!(error.field(), "$.outbounds[1].transport");
    assert!(!format!("{error:?}").contains("ERROR_SECRET_CANARY"));
    let syntax = parse_configuration(b"{\n", InputFormat::Json, &catalog()).unwrap_err();
    assert!(syntax.location().is_some());
}

#[test]
fn versions_unknown_fields_and_future_capabilities_fail_closed() {
    reject(
        &json!({"schemaVersion":2}),
        Code::UnsupportedVersion,
        "$.schemaVersion",
    );
    let mut value = fixture();
    value["outbounds"][0]["transport"]["kind"] = json!("wireguard");
    assert_eq!(parse(&value).unwrap_err().code(), Code::InvalidShape);
    let mut value = fixture();
    value["profiles"][0]["customRules"][0]["layer"] = json!("default");
    assert_eq!(parse(&value).unwrap_err().code(), Code::InvalidShape);
    let mut value = fixture();
    value["outbounds"][1]["transport"]["password"] = json!("PLAINTEXT_NOT_SUPPORTED");
    let error = parse(&value).unwrap_err();
    assert_eq!(error.code(), Code::InvalidShape);
    assert!(!format!("{error:?}").contains("PLAINTEXT"));
}

#[test]
fn missing_references_and_duplicate_ids_are_contextual_errors() {
    let mut value = fixture();
    value["profiles"][1]["id"] = json!(id(1));
    reject(&value, Code::DuplicateId, "$.profiles[1].id");
    let mut value = fixture();
    value["profiles"][0]["serviceRules"][0]["id"] = json!(id(80));
    reject(
        &value,
        Code::DuplicateId,
        "$.profiles[0].serviceRules[0].id",
    );
    let mut value = fixture();
    value["endpoints"][0]["profile"] = json!(id(999));
    reject(&value, Code::UnknownReference, "$.endpoints[0].profile");
    let error = parse_configuration(
        &serde_json::to_vec(&fixture()).unwrap(),
        InputFormat::Json,
        &ReferenceCatalog::default(),
    )
    .unwrap_err();
    assert_eq!(error.field(), "$.profiles[0].defaultDnsAction.upstream");
}

#[test]
fn endpoint_credentials_and_grants_cannot_cross_scopes() {
    let mut value = fixture();
    value["endpoints"][0]["access"][0]["credentials"] = json!([id(22)]);
    reject(
        &value,
        Code::InvalidScope,
        "$.endpoints[0].access[0].credentials[0]",
    );
    let mut value = fixture();
    value["resolverCredentials"][0]["state"] = json!("expired");
    reject(
        &value,
        Code::InvalidScope,
        "$.endpoints[0].access[0].credentials",
    );
    value["endpoints"][0]["enabled"] = json!(false);
    assert!(parse(&value).is_ok());
    let mut value = fixture();
    value["endpoints"][1]["access"][0]["listener"] = json!(id(32));
    reject(
        &value,
        Code::InvalidScope,
        "$.endpoints[1].access[0].listener",
    );
    let mut value = fixture();
    value["endpoints"][1]["access"][0]["grants"] = json!([id(51)]);
    reject(
        &value,
        Code::InvalidScope,
        "$.endpoints[1].access[0].grants[0]",
    );
    let mut value = fixture();
    value["legacyGrants"][0]["source"] = json!("203.0.113.0/24");
    assert_eq!(parse(&value).unwrap_err().code(), Code::InvalidShape);
}

#[test]
fn ambiguous_legacy_and_gateway_groups_never_union_or_last_query_switch() {
    let mut value = fixture();
    let mut duplicate = value["legacyGrants"][0].clone();
    duplicate["id"] = json!(id(52));
    value["legacyGrants"]
        .as_array_mut()
        .unwrap()
        .push(duplicate);
    reject(&value, Code::AmbiguousGrant, "$.legacyGrants[2]");
    let mut value = fixture();
    value["networkGroups"]
        .as_array_mut()
        .unwrap()
        .push(json!({"id":id(41),"name":"Other","trafficProfile":id(2)}));
    let mut other = value["gatewayGrants"][0].clone();
    other["id"] = json!(id(61));
    other["group"] = json!(id(41));
    value["gatewayGrants"].as_array_mut().unwrap().push(other);
    reject(&value, Code::AmbiguousGrant, "$.gatewayGrants[1]");
}

#[test]
fn gateway_and_rule_port_scopes_and_dataset_references_are_bounded() {
    let mut value = fixture();
    value["gatewayGrants"][0]["targets"] = json!([]);
    reject(&value, Code::InvalidScope, "$.gatewayGrants[0].targets");
    let mut value = fixture();
    value["gatewayGrants"][0]["ports"] = json!([{"from":444,"to":443}]);
    reject(&value, Code::InvalidValue, "$.gatewayGrants[0].ports[0]");
    let mut value = fixture();
    value["profiles"][0]["customRules"][0]["ports"] = json!([{"from":443,"to":443}]);
    reject(
        &value,
        Code::InvalidScope,
        "$.profiles[0].customRules[0].ports",
    );
    let mut value = fixture();
    value["profiles"][0]["filterLists"][0]["revision"] = json!(id(999));
    reject(
        &value,
        Code::UnknownReference,
        "$.profiles[0].filterLists[0].revision",
    );
    let mut value = fixture();
    value["privateRecords"][0]["profile"] = json!(id(999));
    reject(
        &value,
        Code::UnknownReference,
        "$.privateRecords[0].profile",
    );
}

fn listeners_only(entries: Vec<Value>) -> Value {
    json!({"schemaVersion":1,"listeners":entries})
}
fn listener(n: usize, bind: &str, transport: &str, dual: bool, enabled: bool) -> Value {
    json!({"id":id(n),"name":"test","enabled":enabled,"bind":bind,"transport":transport,"dualStack":dual})
}

#[test]
fn binding_conflicts_cover_wildcards_dual_stack_socket_protocol_and_order() {
    let pairs = [
        ("127.0.0.1:5353", "0.0.0.0:5353", false, true),
        ("[::1]:5353", "[::]:5353", false, true),
        ("127.0.0.1:5353", "[::]:5353", true, true),
        ("127.0.0.1:5353", "[::]:5353", false, false),
        ("127.0.0.1:5353", "127.0.0.2:5353", false, false),
        ("[fe80::1%1]:5353", "[fe80::1%2]:5353", false, false),
        ("[fe80::1]:5353", "[fe80::1%2]:5353", false, true),
    ];
    for (a, b, dual, conflict) in pairs {
        for reverse in [false, true] {
            let mut entries = vec![
                listener(30, a, "dns_tcp", false, true),
                listener(31, b, "doh", dual, true),
            ];
            if reverse {
                entries.reverse();
            }
            let result = parse(&listeners_only(entries));
            assert_eq!(result.is_err(), conflict, "{a} {b} reverse={reverse}");
            if conflict {
                assert_eq!(result.unwrap_err().code(), Code::BindingConflict);
            }
        }
    }
    assert!(
        parse(&listeners_only(vec![
            listener(30, "127.0.0.1:5353", "dns_tcp", false, true),
            listener(31, "127.0.0.1:5353", "dns_udp", false, true)
        ]))
        .is_ok()
    );
    assert!(
        parse(&listeners_only(vec![
            listener(30, "127.0.0.1:5353", "doh", false, true),
            listener(31, "127.0.0.1:5353", "dot", false, false)
        ]))
        .is_ok()
    );
    reject(
        &listeners_only(vec![listener(30, "127.0.0.1:0", "dns_tcp", false, false)]),
        Code::InvalidValue,
        "$.listeners[0].bind",
    );
    reject(
        &listeners_only(vec![listener(30, "127.0.0.1:53", "dns_tcp", true, true)]),
        Code::InvalidScope,
        "$.listeners[0].dualStack",
    );
    reject(
        &listeners_only(vec![listener(
            30,
            "[::ffff:127.0.0.1]:53",
            "dns_tcp",
            false,
            true,
        )]),
        Code::InvalidScope,
        "$.listeners[0].bind",
    );
}

#[test]
fn aggregate_resource_limit_includes_nested_revision_selections() {
    let mut catalog = ReferenceCatalog::default();
    let filters: Vec<_> = (1..MAX_RESOURCE_ENTRIES)
        .map(|n| {
            catalog.filter_revisions.insert(typed(n));
            json!({"revision":id(n),"enabled":true,"priority":0})
        })
        .collect();
    let mut value = json!({"schemaVersion":1,"profiles":[{"id":id(1),"name":"bulk","defaultDnsAction":{"kind":"block"},"defaultTrafficAction":{"kind":"deny"},"filterLists":filters}]});
    let exact = serde_json::to_vec(&value).unwrap();
    assert!(exact.len() <= MAX_INPUT_BYTES);
    assert_eq!(
        parse_configuration(&exact, InputFormat::Json, &catalog)
            .unwrap()
            .document()
            .profiles[0]
            .filter_lists
            .len(),
        9999
    );
    value["profiles"][0]["filterLists"]
        .as_array_mut()
        .unwrap()
        .push(json!({"revision":id(10000),"enabled":true,"priority":0}));
    let error = parse_configuration(
        &serde_json::to_vec(&value).unwrap(),
        InputFormat::Json,
        &catalog,
    )
    .unwrap_err();
    assert_eq!(error.code(), Code::ResourceLimit);
}

#[test]
fn empty_action_variants_reject_unknown_fields_in_every_format() {
    for path in [
        "/outbounds/0/transport",
        "/profiles/1/defaultDnsAction",
        "/profiles/1/defaultTrafficAction",
        "/profiles/0/customRules/0/action/action",
    ] {
        let mut value = fixture();
        value.pointer_mut(path).unwrap()["unexpected"] = json!("ERROR_SECRET_CANARY");
        let input = serde_json::to_vec(&value).unwrap();
        for format in [InputFormat::Json, InputFormat::Yaml] {
            let error = parse_configuration(&input, format, &catalog()).unwrap_err();
            assert_eq!(error.code(), Code::InvalidShape, "{path}, {format:?}");
            assert!(!format!("{error:?}").contains("ERROR_SECRET_CANARY"));
        }
    }
    for extra in [
        "defaultDnsAction = {kind='block', unexpected='ERROR_SECRET_CANARY'}\ndefaultTrafficAction = {kind='deny'}",
        "defaultDnsAction = {kind='block'}\ndefaultTrafficAction = {kind='deny', unexpected='ERROR_SECRET_CANARY'}",
    ] {
        let input = format!(
            "schemaVersion=1\n[[profiles]]\nid='{}'\nname='draft'\n{extra}",
            id(1)
        );
        let error =
            parse_configuration(input.as_bytes(), InputFormat::Toml, &catalog()).unwrap_err();
        assert_eq!(error.code(), Code::InvalidShape);
        assert!(!format!("{error:?}").contains("ERROR_SECRET_CANARY"));
    }
}

#[test]
fn positional_arrays_cannot_replace_root_or_nested_schema_objects() {
    for format in [InputFormat::Json, InputFormat::Yaml] {
        assert_eq!(
            parse_configuration(b"[1]", format, &catalog())
                .unwrap_err()
                .code(),
            Code::InvalidShape
        );
        for (path, fields) in [
            (
                "/profiles/0",
                vec![
                    "id",
                    "name",
                    "defaultDnsAction",
                    "defaultTrafficAction",
                    "customRules",
                    "serviceRules",
                    "filterLists",
                ],
            ),
            ("/profiles/0/customRules/0/matcher", vec!["mode", "value"]),
            ("/gatewayGrants/0/ports/0", vec!["from", "to"]),
            (
                "/privateRecords/0",
                vec!["id", "profile", "name", "ttl", "record"],
            ),
        ] {
            let mut value = fixture();
            let object = value.pointer_mut(path).unwrap();
            *object = fields
                .into_iter()
                .map(|field| object[field].take())
                .collect();
            let error =
                parse_configuration(&serde_json::to_vec(&value).unwrap(), format, &catalog())
                    .unwrap_err();
            assert_eq!(error.code(), Code::InvalidShape, "{path}, {format:?}");
        }
    }
    let input = format!(
        "schemaVersion=1\n[[profiles]]\nid='{}'\nname='draft'\ndefaultDnsAction={{kind='block'}}\ndefaultTrafficAction={{kind='deny'}}\n[[profiles.customRules]]\nid='{}'\nenabled=true\npriority=0\nmatcher={{mode='exact',value='example.com'}}\nports=[[443,443]]\naction={{plane='traffic',action={{kind='deny'}}}}",
        id(1),
        id(2)
    );
    assert_eq!(
        parse_configuration(input.as_bytes(), InputFormat::Toml, &catalog())
            .unwrap_err()
            .code(),
        Code::InvalidShape
    );
}

#[test]
fn semantic_rule_errors_identify_the_nested_action_payload() {
    let mut value = fixture();
    value["profiles"][0]["customRules"][0]["action"]["action"] =
        json!({"kind":"resolve","upstream":id(999)});
    reject(
        &value,
        Code::UnknownReference,
        "$.profiles[0].customRules[0].action.action.upstream",
    );
    let mut value = fixture();
    value["profiles"][0]["serviceRules"][0]["action"]["action"]["outbound"] = json!(id(999));
    reject(
        &value,
        Code::UnknownReference,
        "$.profiles[0].serviceRules[0].action.action.outbound",
    );
}

#[test]
fn tagged_enums_require_objects_even_when_serde_accepts_sequences() {
    for (path, positional) in [
        ("/profiles/0/defaultDnsAction", json!(["resolve", id(90)])),
        ("/profiles/1/defaultDnsAction", json!(["block"])),
        ("/profiles/1/defaultTrafficAction", json!(["deny"])),
        ("/outbounds/0/transport", json!(["vps_direct"])),
        (
            "/outbounds/1/transport",
            json!([
                "http_connect_tls",
                "proxy.example.com",
                443,
                "proxy.example.com",
                id(99)
            ]),
        ),
        ("/endpoints/0/access/0", json!(["doh", id(32), [id(20)]])),
        ("/privateRecords/0/record", json!(["a", "192.168.1.5"])),
        ("/profiles/0/customRules/0/action/action", json!(["block"])),
        (
            "/profiles/0/customRules/1/action/action",
            json!(["outbound", id(71)]),
        ),
    ] {
        let mut value = fixture();
        *value.pointer_mut(path).unwrap() = positional;
        for format in [InputFormat::Json, InputFormat::Yaml] {
            let error =
                parse_configuration(&serde_json::to_vec(&value).unwrap(), format, &catalog())
                    .unwrap_err();
            assert_eq!(error.code(), Code::InvalidShape, "{path}, {format:?}");
        }
    }
    let input = format!(
        "schemaVersion=1\n[[profiles]]\nid='{}'\nname='draft'\ndefaultDnsAction=['block']\ndefaultTrafficAction=['deny']",
        id(1)
    );
    assert_eq!(
        parse_configuration(input.as_bytes(), InputFormat::Toml, &catalog())
            .unwrap_err()
            .code(),
        Code::InvalidShape
    );
}
