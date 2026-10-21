use droutex_common::*;
use droutex_config::{
    catalog::{CatalogFormat, CatalogInput, CatalogLicenseApproval, CatalogMetadata, CatalogStore},
    snapshot::{
        self, ContentHash, ReplayState, ResourcePin, SnapshotContent, SnapshotMetadata,
        SnapshotResource, SnapshotSequence, TrustedResources,
    },
    *,
};
use droutex_policy::{CatalogRevision, DomainMatchMode, RuleSource};
use droutex_security::{SigningKey, SigningKeyId, SnapshotTrust};
use serde_json::{Value, json};
use std::{num::NonZeroU16, str::FromStr};
#[path = "compilation/admission.rs"]
mod admission;
#[path = "compilation/explain.rs"]
mod explain;

fn id<T: FromStr>(n: usize) -> T
where
    T::Err: std::fmt::Debug,
{
    format!("10000000-0000-4000-8000-{n:012x}").parse().unwrap()
}
fn text(n: usize) -> String {
    id::<ProfileId>(n).to_string()
}
fn metadata() -> CatalogMetadata {
    CatalogMetadata {
        source: "local:compiler-fixture".into(),
        owner: "DRouteX".into(),
        license: "Apache-2.0".into(),
        notice: "Original synthetic compilation fixture.".into(),
    }
}
fn import(store: &mut CatalogStore, revision: CatalogRevision, bytes: &[u8]) {
    let format = match revision {
        CatalogRevision::Filter(_) => CatalogFormat::Domains(DomainMatchMode::DomainAndSubdomains),
        CatalogRevision::Service(_) => CatalogFormat::ServiceJson,
    };
    store
        .import_batch(
            vec![CatalogInput {
                revision,
                format,
                metadata: metadata(),
                bytes,
            }],
            vec![
                CatalogLicenseApproval::new(
                    revision,
                    ContentHash::of(bytes),
                    "Apache-2.0".into(),
                    "owner:synthetic-test".into(),
                )
                .unwrap(),
            ],
        )
        .unwrap();
}
fn catalogs() -> CatalogStore {
    let mut store = CatalogStore::default();
    import(
        &mut store,
        CatalogRevision::Filter(id(200)),
        b"blocked.example.test\ndep-only.example.test\n",
    );
    let hash = ContentHash::of(
        store
            .get(CatalogRevision::Filter(id(200)))
            .unwrap()
            .resource_bytes(),
    )
    .to_string();
    let service = json!({"schemaVersion":1,"kind":"service","entries":[
        {"domain":"service.example.test","mode":"domain_and_subdomains"},
        {"domain":"own.example.test","mode":"subdomains_only"}],
        "dependencies":[{"kind":"filter","revision":text(200),"hash":hash}]});
    import(
        &mut store,
        CatalogRevision::Service(id(201)),
        &serde_json::to_vec(&service).unwrap(),
    );
    store
}
fn references() -> ReferenceCatalog {
    ReferenceCatalog {
        upstreams: [id(60)].into(),
        filter_revisions: [id(200)].into(),
        service_revisions: [id(201)].into(),
        ..Default::default()
    }
}
fn document() -> Value {
    json!({"schemaVersion":1,"profiles":[
        {"id":text(1),"name":"DNS","defaultDnsAction":{"kind":"resolve","upstream":text(60)},"defaultTrafficAction":{"kind":"deny"},
         "customRules":[{"id":text(100),"enabled":true,"priority":-1,"matcher":{"mode":"exact","value":"allow.example.test"},
            "action":{"plane":"dns","action":{"kind":"resolve","upstream":text(60)}}}],
         "serviceRules":[{"id":text(101),"enabled":true,"priority":70,"revision":text(201),"action":{"plane":"dns","action":{"kind":"gateway","outbound":text(51)}}}],
         "filterLists":[{"revision":text(200),"enabled":true,"priority":900}]},
        {"id":text(2),"name":"Gateway","defaultDnsAction":{"kind":"block"},"defaultTrafficAction":{"kind":"outbound","outbound":text(50)},
         "customRules":[{"id":text(102),"enabled":true,"priority":-10,"matcher":{"mode":"exact","value":"blocked.example.test"},
            "ports":[{"from":443,"to":443}],"action":{"plane":"traffic","action":{"kind":"outbound","outbound":text(51)}}}],
         "serviceRules":[{"id":text(103),"enabled":true,"priority":1,"revision":text(201),"action":{"plane":"traffic","action":{"kind":"outbound","outbound":text(50)}}}],
         "filterLists":[{"revision":text(200),"enabled":true,"priority":900}]}],
        "listeners":[{"id":text(20),"name":"DNS UDP","enabled":true,"bind":"127.0.0.1:5353","transport":"dns_udp"},
            {"id":text(21),"name":"Gateway TCP","enabled":true,"bind":"127.0.0.1:10443","transport":"gateway_tcp"}],
        "endpoints":[{"id":text(10),"name":"Personal","enabled":true,"profile":text(1),
            "access":[{"transport":"udp","listener":text(20),"grants":[text(40)]}]}],
        "networkGroups":[{"id":text(30),"name":"Shared NAT","trafficProfile":text(2)}],
        "legacyGrants":[{"id":text(40),"source":"203.0.113.7","listener":text(20),"group":text(30),"endpoint":text(10)}],
        "gatewayGrants":[{"id":text(41),"source":"203.0.113.7","listener":text(21),"group":text(30),
            "targets":[{"mode":"exact","value":"allowed.example.test"}],"services":[text(201)],"ports":[{"from":443,"to":443}],"outbounds":[text(50)]}],
        "outbounds":[{"id":text(50),"name":"Explicit direct","enabled":true,"transport":{"kind":"vps_direct"}},
            {"id":text(51),"name":"Unavailable secure proxy","enabled":false,"transport":{"kind":"http_connect_tls","host":"proxy.test","port":443,"tlsServerName":"proxy.test"}}],
        "privateRecords":[{"id":text(70),"profile":text(1),"name":"PRIVATE.example.test.","ttl":30,"record":{"kind":"a","address":"192.0.2.10"}},
            {"id":text(71),"profile":text(1),"name":"alias.example.test","ttl":30,"record":{"kind":"cname","target":"private.example.test"}}]})
}
fn checked(document: &Value) -> CheckedConfiguration {
    parse_configuration(
        &serde_json::to_vec(document).unwrap(),
        InputFormat::Json,
        &references(),
    )
    .unwrap()
}
fn data(store: &CatalogStore) -> Vec<ResourceData<'_>> {
    let mut result: Vec<_> = store
        .revisions()
        .map(|r| ResourceData {
            resource: match r.revision() {
                CatalogRevision::Filter(id) => SnapshotResource::Filter(id),
                CatalogRevision::Service(id) => SnapshotResource::Service(id),
            },
            bytes: r.resource_bytes(),
        })
        .collect();
    result.push(ResourceData {
        resource: SnapshotResource::Upstream(id(60)),
        bytes: b"verified-upstream-descriptor",
    });
    result
}
fn publication(
    document: &Value,
    store: &CatalogStore,
) -> Result<CompletePublication, CompilationError> {
    CompletePublication::new(
        checked(document),
        vec![],
        &data(store),
        &store.revisions().collect::<Vec<_>>(),
    )
}
fn identity() -> GatewayNetworkIdentity {
    GatewayNetworkIdentity {
        grant: id(41),
        group: id(30),
        listener: id(21),
        source: "203.0.113.7".parse().unwrap(),
    }
}
fn binding() -> DnsProfileBinding {
    DnsProfileBinding {
        endpoint: id(10),
        profile: id(1),
    }
}
fn key() -> (SigningKey, SnapshotTrust) {
    // Published RFC8032 vector1, never an operational private key.
    fn hex(s: &str) -> Vec<u8> {
        s.as_bytes()
            .as_chunks::<2>()
            .0
            .iter()
            .map(|p| u8::from_str_radix(std::str::from_utf8(p).unwrap(), 16).unwrap())
            .collect()
    }
    let public = hex("d75a980182b10ab7d54bfed3c964073a0ee172f3daa62325af021a68f707511a")
        .try_into()
        .unwrap();
    let key=SigningKey::from_pkcs8(SigningKeyId::from_bytes([1;16]).unwrap(),hex("302e020100300506032b6570042204209d61b19deffd5a60ba844af492ec2cc44449c5697b326919703bac031cae7f60"),&public).unwrap();
    let trust = SnapshotTrust::new(vec![(key.id(), public)]).unwrap();
    (key, trust)
}
fn snapshot_metadata() -> SnapshotMetadata {
    SnapshotMetadata {
        node: id(80),
        snapshot: id(81),
        source: id(82),
        sequence: SnapshotSequence::new(1).unwrap(),
    }
}

#[test]
fn complete_builder_and_authenticated_dp_preserve_d11_and_deterministic_decisions() {
    let store = catalogs();
    let original = document();
    let first = publication(&original, &store).unwrap();
    let (key, trust) = key();
    let encoded = first.encode(snapshot_metadata(), &key).unwrap();
    assert_eq!(
        encoded,
        snapshot::encode_snapshot(first.content(), snapshot_metadata(), &key).unwrap()
    );
    let mut reordered = original.clone();
    for field in ["profiles", "listeners", "outbounds", "privateRecords"] {
        reordered[field].as_array_mut().unwrap().reverse();
    }
    let mut inputs = data(&store);
    inputs.reverse();
    let second = CompletePublication::new(
        checked(&reordered),
        vec![],
        &inputs,
        &store.revisions().collect::<Vec<_>>(),
    )
    .unwrap();
    assert_eq!(first.content().hash(), second.content().hash());
    assert_eq!(encoded, second.encode(snapshot_metadata(), &key).unwrap());
    let trusted = TrustedResources::new(
        inputs
            .iter()
            .map(|r| ResourcePin::from_bytes(r.resource, r.bytes).unwrap())
            .collect(),
    )
    .unwrap();
    let verified = snapshot::verify_snapshot(
        &encoded,
        &trust,
        &ReplayState::first_provisioning(id(80)),
        &trusted,
    )
    .unwrap();
    let dp = verified.compile(&inputs).unwrap();
    for (domain, action, source) in [
        (
            "allow.example.test",
            DnsAction::Resolve { upstream: id(60) },
            Some(RuleSource::Custom(id(100))),
        ),
        (
            "service.example.test",
            DnsAction::Gateway { outbound: id(51) },
            Some(RuleSource::Service {
                rule: id(101),
                revision: id(201),
            }),
        ),
        (
            "x.service.example.test",
            DnsAction::Gateway { outbound: id(51) },
            Some(RuleSource::Service {
                rule: id(101),
                revision: id(201),
            }),
        ),
        (
            "blocked.example.test",
            DnsAction::Block {},
            Some(RuleSource::Filter(id(200))),
        ),
        (
            "own.example.test",
            DnsAction::Resolve { upstream: id(60) },
            None,
        ),
        (
            "x.own.example.test",
            DnsAction::Gateway { outbound: id(51) },
            Some(RuleSource::Service {
                rule: id(101),
                revision: id(201),
            }),
        ),
        (
            "unmatched.test",
            DnsAction::Resolve { upstream: id(60) },
            None,
        ),
    ] {
        for compiled in [first.compiled(), second.compiled(), &dp] {
            let name = domain.parse().unwrap();
            let selected = compiled.policy().evaluate_dns(&binding(), &name).unwrap();
            assert_eq!(*selected.action(), action, "{domain}");
            assert_eq!(selected.source(), source);
            assert_eq!(selected.profile(), id(1));
        }
    }
    for (domain, port, action) in [
        (
            "blocked.example.test",
            443,
            TrafficAction::Outbound { outbound: id(51) },
        ),
        ("blocked.example.test", 80, TrafficAction::Deny {}),
        ("dep-only.example.test", 443, TrafficAction::Deny {}),
        (
            "service.example.test",
            443,
            TrafficAction::Outbound { outbound: id(50) },
        ),
        (
            "own.example.test",
            443,
            TrafficAction::Outbound { outbound: id(50) },
        ),
    ] {
        let selected = dp
            .policy()
            .evaluate_tcp(
                &identity(),
                &domain.parse().unwrap(),
                NonZeroU16::new(port).unwrap(),
            )
            .unwrap();
        assert_eq!(*selected.action(), action, "{domain}:{port}");
        assert_eq!(selected.profile(), id(2));
    }
    assert_eq!(
        dp.private_records()[1].owner.as_str(),
        "private.example.test"
    );
    assert_eq!(
        dp.catalog_dependencies(CatalogRevision::Service(id(201)))
            .unwrap()[0],
        dp.catalog_pin(CatalogRevision::Filter(id(200))).unwrap()
    );
    assert_eq!(dp.legacy_scope(id(40)).unwrap().endpoint, id(10));
    assert!(!format!("{dp:?}").contains("example.test"));
}

#[test]
fn service_dependencies_never_expand_gateway_targets_and_all_static_dimensions_match() {
    let store = catalogs();
    let value = publication(&document(), &store).unwrap();
    let scope = value.compiled().gateway_scope(id(41)).unwrap();
    for (name, expected) in [
        ("service.example.test", true),
        ("sub.service.example.test", true),
        ("allowed.example.test", true),
        ("dep-only.example.test", false),
        ("blocked.example.test", false),
        ("own.example.test", false),
        ("x.own.example.test", true),
    ] {
        assert_eq!(
            scope.matches_static_scope(
                &identity(),
                &name.parse().unwrap(),
                NonZeroU16::new(443).unwrap(),
                id(50)
            ),
            expected,
            "{name}"
        );
    }
    let name = "service.example.test".parse().unwrap();
    let port = NonZeroU16::new(443).unwrap();
    for changed in [
        GatewayNetworkIdentity {
            source: "203.0.113.8".parse().unwrap(),
            ..identity()
        },
        GatewayNetworkIdentity {
            listener: id(22),
            ..identity()
        },
        GatewayNetworkIdentity {
            group: id(31),
            ..identity()
        },
        GatewayNetworkIdentity {
            grant: id(42),
            ..identity()
        },
    ] {
        assert!(!scope.matches_static_scope(&changed, &name, port, id(50)));
    }
    assert!(!scope.matches_static_scope(&identity(), &name, NonZeroU16::new(80).unwrap(), id(50)));
    assert!(!scope.matches_static_scope(&identity(), &name, port, id(51)));
}

#[test]
fn real_approvals_complete_pins_conflicts_and_private_validation_gate_cp_publication() {
    let store = catalogs();
    assert!(matches!(
        CompletePublication::new(checked(&document()), vec![], &data(&store), &[]),
        Err(CompilationError::CatalogApproval(_))
    ));
    let mut missing = data(&store);
    missing.retain(|r| r.resource != SnapshotResource::Filter(id(200)));
    assert!(
        CompletePublication::new(
            checked(&document()),
            vec![],
            &missing,
            &store
                .revisions()
                .filter(|r| r.revision() != CatalogRevision::Filter(id(200)))
                .collect::<Vec<_>>()
        )
        .is_err()
    );
    let mut conflict = document();
    let mut rule = conflict["profiles"][0]["customRules"][0].clone();
    rule["id"] = json!(text(104));
    rule["action"]["action"] = json!({"kind":"block"});
    conflict["profiles"][0]["customRules"]
        .as_array_mut()
        .unwrap()
        .push(rule);
    assert!(matches!(
        publication(&conflict, &store),
        Err(CompilationError::Conflict(_))
    ));
    let mut invalid = document();
    invalid["privateRecords"][1]["name"] = json!("private.example.test");
    assert!(matches!(
        publication(&invalid, &store),
        Err(CompilationError::Publication(_))
    ));
}

#[test]
fn correct_signature_does_not_authorize_malformed_or_noncanonical_catalog_resources() {
    let store = catalogs();
    let (key, trust) = key();
    let base = data(&store);
    for mutate in 0..13 {
        let filter = store.get(CatalogRevision::Filter(id(200))).unwrap();
        let original = std::str::from_utf8(filter.resource_bytes()).unwrap();
        // Preserve the real encoder's field order: semantic refusals must not be
        // accidentally proved only by a differently ordered Value serialization.
        let malformed = match mutate {
            0 => original.replace("blocked.example.test", "BAD..private-value.test"),
            1 => original.replace("\"license\":\"Apache-2.0\"", "\"license\":\"\""),
            2 => original.replace(
                &format!("\"revision\":\"{}\"", text(200)),
                &format!("\"revision\":\"{}\"", text(999)),
            ),
            3 => original.replacen(
                "\"mode\":\"domain_and_subdomains\"",
                "\"mode\":\"exact\"",
                1,
            ),
            4 => original.replace(&filter.source_hash().to_string(), &"X".repeat(64)),
            5 => original.replace("owner:synthetic-test", ""),
            6 => format!(
                "{},\"extra\":\"private-value\"}}",
                &original[..original.len() - 1]
            ),
            7 => format!(" {original}"),
            8 => original.replacen(
                "\"schemaVersion\":1",
                "\"schemaVersion\":1,\"schemaVersion\":1",
                1,
            ),
            9 => original.replace("\"kind\":\"filter\"", "\"kind\":\"service\""),
            _ => {
                let control = match mutate {
                    10 => '\n',
                    11 => '\r',
                    _ => '\t',
                };
                original.replace(
                    "\"license\":\"Apache-2.0\"",
                    &format!(
                        "\"license\":{}",
                        serde_json::to_string(&format!("Apache-2.0{control}")).unwrap()
                    ),
                )
            }
        }
        .into_bytes();
        assert_ne!(malformed, filter.resource_bytes());
        // Rebind the service dependency too: a changed filter hash must not mask
        // a decoder defect by causing an unrelated closure-hash refusal later.
        let service = store.get(CatalogRevision::Service(id(201))).unwrap();
        let rebound = std::str::from_utf8(service.resource_bytes())
            .unwrap()
            .replace(
                &ContentHash::of(filter.resource_bytes()).to_string(),
                &ContentHash::of(&malformed).to_string(),
            )
            .into_bytes();
        let mut inputs = base.clone();
        inputs
            .iter_mut()
            .find(|r| r.resource == SnapshotResource::Service(id(201)))
            .unwrap()
            .bytes = &rebound;
        inputs
            .iter_mut()
            .find(|r| r.resource == SnapshotResource::Filter(id(200)))
            .unwrap()
            .bytes = &malformed;
        let trusted = TrustedResources::new(
            inputs
                .iter()
                .map(|r| ResourcePin::from_bytes(r.resource, r.bytes).unwrap())
                .collect(),
        )
        .unwrap();
        // Deliberate use of the low-level codec proves signature validity is insufficient.
        let content = SnapshotContent::new(checked(&document()), vec![], &trusted).unwrap();
        let bytes = snapshot::encode_snapshot(&content, snapshot_metadata(), &key).unwrap();
        let verified = snapshot::verify_snapshot(
            &bytes,
            &trust,
            &ReplayState::first_provisioning(id(80)),
            &trusted,
        )
        .unwrap();
        let error = verified.compile(&inputs).unwrap_err();
        assert!(!format!("{error:?}").contains("private-value"));
    }
    let published = publication(&document(), &store).unwrap();
    let trusted = TrustedResources::new(
        base.iter()
            .map(|r| ResourcePin::from_bytes(r.resource, r.bytes).unwrap())
            .collect(),
    )
    .unwrap();
    let verified = snapshot::verify_snapshot(
        &published.encode(snapshot_metadata(), &key).unwrap(),
        &trust,
        &ReplayState::first_provisioning(id(80)),
        &trusted,
    )
    .unwrap();
    let changed = [ResourceData {
        resource: SnapshotResource::Upstream(id(60)),
        bytes: b"substituted",
    }];
    assert!(matches!(
        verified.compile(&changed),
        Err(CompilationError::Resource)
    ));
}

#[test]
fn complete_expansion_budget_includes_private_tables_before_rule_copies() {
    // 9995 conditions + Profile/Endpoint/group/legacy scope + private record =10000.
    let entries: Vec<_> = (0..9995)
        .map(|n| json!({"domain":format!("n{n}.example.test"),"mode":"exact"}))
        .collect();
    let bytes = serde_json::to_vec(
        &json!({"schemaVersion":1,"kind":"service","entries":entries,"dependencies":[]}),
    )
    .unwrap();
    let mut store = CatalogStore::default();
    import(&mut store, CatalogRevision::Service(id(201)), &bytes);
    let mut input = document();
    input["profiles"].as_array_mut().unwrap().truncate(1);
    input["networkGroups"][0]["trafficProfile"] = json!(text(1));
    input["profiles"][0]["filterLists"] = json!([]);
    input["profiles"][0]["customRules"] = json!([]);
    input["gatewayGrants"] = json!([]);
    input["privateRecords"].as_array_mut().unwrap().truncate(1);
    let resources = [
        ResourceData {
            resource: SnapshotResource::Service(id(201)),
            bytes: store
                .get(CatalogRevision::Service(id(201)))
                .unwrap()
                .resource_bytes(),
        },
        ResourceData {
            resource: SnapshotResource::Upstream(id(60)),
            bytes: b"upstream",
        },
    ];
    assert!(
        CompletePublication::new(
            checked(&input),
            vec![],
            &resources,
            &store.revisions().collect::<Vec<_>>()
        )
        .is_ok()
    );
    let mut extra = input["privateRecords"][0].clone();
    extra["id"] = json!(text(72));
    extra["name"] = json!("other.example.test");
    input["privateRecords"].as_array_mut().unwrap().push(extra);
    assert!(matches!(
        CompletePublication::new(
            checked(&input),
            vec![],
            &resources,
            &store.revisions().collect::<Vec<_>>()
        ),
        Err(CompilationError::Size)
    ));
}

#[test]
fn compiler_matches_explicit_d38_interpreter_on_domains_ports_and_disabled_sources() {
    use droutex_policy::{
        DomainMatcher, DomainRule, GatewayProfileAssignment, PolicySet, PreparedProfile,
        RuleMetadata, TcpPortRange, TcpPortScope, TcpRule,
    };
    fn rule<A>(
        source: RuleSource,
        state: AdministrativeState,
        priority: i32,
        name: &str,
        mode: DomainMatchMode,
        action: A,
    ) -> DomainRule<A> {
        DomainRule::new(
            RuleMetadata {
                source,
                state,
                priority,
            },
            DomainMatcher::new(name.parse().unwrap(), mode),
            action,
        )
    }
    for enabled in [true, false] {
        let state = if enabled {
            AdministrativeState::Enabled
        } else {
            AdministrativeState::Disabled
        };
        let store = catalogs();
        let mut input = document();
        for index in [0, 1] {
            input["profiles"][index]["serviceRules"][0]["enabled"] = json!(enabled);
        }
        let compiled = publication(&input, &store).unwrap();
        // Hand-expanded semantic reference: no reading of compiler tables or
        // iteration over imported conditions to derive this expected plan.
        let mut dns = PreparedProfile {
            id: id(1),
            dns_default: DnsAction::Resolve { upstream: id(60) },
            dns_rules: vec![
                rule(
                    RuleSource::Custom(id(100)),
                    AdministrativeState::Enabled,
                    -1,
                    "allow.example.test",
                    DomainMatchMode::Exact,
                    DnsAction::Resolve { upstream: id(60) },
                ),
                rule(
                    RuleSource::Service {
                        rule: id(101),
                        revision: id(201),
                    },
                    state,
                    70,
                    "service.example.test",
                    DomainMatchMode::DomainAndSubdomains,
                    DnsAction::Gateway { outbound: id(51) },
                ),
                rule(
                    RuleSource::Service {
                        rule: id(101),
                        revision: id(201),
                    },
                    state,
                    70,
                    "own.example.test",
                    DomainMatchMode::SubdomainsOnly,
                    DnsAction::Gateway { outbound: id(51) },
                ),
            ],
            tcp_default: TrafficAction::Deny {},
            tcp_rules: vec![],
        };
        let mut gateway = PreparedProfile {
            id: id(2),
            dns_default: DnsAction::Block {},
            dns_rules: vec![],
            tcp_default: TrafficAction::Outbound { outbound: id(50) },
            tcp_rules: vec![
                TcpRule::new(
                    rule(
                        RuleSource::Custom(id(102)),
                        AdministrativeState::Enabled,
                        -10,
                        "blocked.example.test",
                        DomainMatchMode::Exact,
                        TrafficAction::Outbound { outbound: id(51) },
                    ),
                    TcpPortScope::Range(TcpPortRange::new(443, 443).unwrap()),
                ),
                TcpRule::new(
                    rule(
                        RuleSource::Service {
                            rule: id(103),
                            revision: id(201),
                        },
                        state,
                        1,
                        "service.example.test",
                        DomainMatchMode::DomainAndSubdomains,
                        TrafficAction::Outbound { outbound: id(50) },
                    ),
                    TcpPortScope::Any,
                ),
                TcpRule::new(
                    rule(
                        RuleSource::Service {
                            rule: id(103),
                            revision: id(201),
                        },
                        state,
                        1,
                        "own.example.test",
                        DomainMatchMode::SubdomainsOnly,
                        TrafficAction::Outbound { outbound: id(50) },
                    ),
                    TcpPortScope::Any,
                ),
            ],
        };
        for name in ["blocked.example.test", "dep-only.example.test"] {
            dns.dns_rules.push(rule(
                RuleSource::Filter(id(200)),
                AdministrativeState::Enabled,
                900,
                name,
                DomainMatchMode::DomainAndSubdomains,
                DnsAction::Block {},
            ));
            gateway.tcp_rules.push(TcpRule::new(
                rule(
                    RuleSource::Filter(id(200)),
                    AdministrativeState::Enabled,
                    900,
                    name,
                    DomainMatchMode::DomainAndSubdomains,
                    TrafficAction::Deny {},
                ),
                TcpPortScope::Any,
            ));
        }
        let reference = PolicySet::new(
            vec![dns, gateway],
            vec![DnsEndpointAssignment {
                endpoint: id(10),
                profile: id(1),
                state: AdministrativeState::Enabled,
                device: None,
            }],
            vec![GatewayProfileAssignment {
                group: id(30),
                profile: id(2),
            }],
        )
        .unwrap();
        for name in [
            "allow.example.test",
            "x.allow.example.test",
            "blocked.example.test",
            "x.blocked.example.test",
            "dep-only.example.test",
            "service.example.test",
            "x.service.example.test",
            "own.example.test",
            "x.own.example.test",
            "unmatched.test",
        ] {
            let query = name.parse().unwrap();
            let got = compiled
                .compiled()
                .policy()
                .evaluate_dns(&binding(), &query)
                .unwrap();
            let expected = reference.evaluate_dns(&binding(), &query).unwrap();
            assert_eq!(got.action(), expected.action(), "DNS {name}/{enabled}");
            assert_eq!(got.source(), expected.source());
            for port in [1, 80, 443, 444, 65535] {
                let port = NonZeroU16::new(port).unwrap();
                let identity = identity();
                let got = compiled
                    .compiled()
                    .policy()
                    .evaluate_tcp(&identity, &query, port)
                    .unwrap();
                let expected = reference.evaluate_tcp(&identity, &query, port).unwrap();
                assert_eq!(
                    got.action(),
                    expected.action(),
                    "TCP {name}:{port}/{enabled}"
                );
                assert_eq!(got.source(), expected.source());
            }
        }
    }
}

#[test]
fn complete_resource_and_dependency_sets_refuse_substitution_and_before_oversize_hashing() {
    let store = catalogs();
    let (key, trust) = key();
    let mut inputs = data(&store);
    let service = store.get(CatalogRevision::Service(id(201))).unwrap();
    let bad = std::str::from_utf8(service.resource_bytes())
        .unwrap()
        .replace(
            &ContentHash::of(
                store
                    .get(CatalogRevision::Filter(id(200)))
                    .unwrap()
                    .resource_bytes(),
            )
            .to_string(),
            &"0".repeat(64),
        )
        .into_bytes();
    inputs
        .iter_mut()
        .find(|r| r.resource == SnapshotResource::Service(id(201)))
        .unwrap()
        .bytes = &bad;
    let trusted = TrustedResources::new(
        inputs
            .iter()
            .map(|r| ResourcePin::from_bytes(r.resource, r.bytes).unwrap())
            .collect(),
    )
    .unwrap();
    let content = SnapshotContent::new(checked(&document()), vec![], &trusted).unwrap();
    let encoded = snapshot::encode_snapshot(&content, snapshot_metadata(), &key).unwrap();
    let verified = snapshot::verify_snapshot(
        &encoded,
        &trust,
        &ReplayState::first_provisioning(id(80)),
        &trusted,
    )
    .unwrap();
    assert!(matches!(
        verified.compile(&inputs),
        Err(CompilationError::Publication(_))
    ));
    let bytes = vec![0; 1024 * 1024];
    let oversized = vec![
        ResourceData {
            resource: SnapshotResource::Upstream(id(60)),
            bytes: &bytes
        };
        65
    ];
    assert!(matches!(
        CompletePublication::new(checked(&document()), vec![], &oversized, &[]),
        Err(CompilationError::Size)
    ));
    let too_many = vec![
        ResourceData {
            resource: SnapshotResource::Upstream(id(60)),
            bytes: b"x"
        };
        10001
    ];
    assert!(matches!(
        CompletePublication::new(checked(&document()), vec![], &too_many, &[]),
        Err(CompilationError::Size)
    ));
}
