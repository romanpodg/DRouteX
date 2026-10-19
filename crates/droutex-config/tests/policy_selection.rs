use droutex_common::{
    AdministrativeState, DnsEndpointAssignment, DnsProfileBinding, GatewayNetworkIdentity,
};
use droutex_config::{
    DnsAction, InputFormat, ReferenceCatalog, RuleAction, TrafficAction, parse_configuration,
};
use droutex_policy::{
    DomainMatchMode, DomainMatcher, DomainRule, GatewayProfileAssignment, PolicySet,
    PreparedProfile, RuleMetadata, RuleSource, TcpPortRange, TcpPortScope, TcpRule,
};
use serde_json::json;
use std::{num::NonZeroU16, str::FromStr};

fn id<T: FromStr>(n: usize) -> T
where
    T::Err: std::fmt::Debug,
{
    format!("10000000-0000-4000-8000-{n:012x}").parse().unwrap()
}

#[test]
fn existing_dns_and_traffic_effects_remain_separate_and_bypass_is_explicit() {
    let input = json!({"schemaVersion":1,"profiles":[{
        "id":id::<droutex_common::ProfileId>(1).to_string(),"name":"p",
        "defaultDnsAction":{"kind":"resolve","upstream":id::<droutex_common::DnsUpstreamId>(90).to_string()},
        "defaultTrafficAction":{"kind":"deny"},
        "customRules":[
            {"id":id::<droutex_common::RuleId>(80).to_string(),"enabled":true,"priority":0,
             "matcher":{"mode":"exact","value":"Bypass.Example."},
             "action":{"plane":"dns","action":{"kind":"resolve","upstream":id::<droutex_common::DnsUpstreamId>(90).to_string()}}},
            {"id":id::<droutex_common::RuleId>(81).to_string(),"enabled":true,"priority":0,
             "matcher":{"mode":"exact","value":"Bypass.Example."},"ports":[{"from":443,"to":443}],
             "action":{"plane":"traffic","action":{"kind":"outbound","outbound":id::<droutex_common::ProxyOutboundId>(70).to_string()}}}
        ]}],
        "outbounds":[{"id":id::<droutex_common::ProxyOutboundId>(70).to_string(),"name":"native","enabled":true,"transport":{"kind":"vps_direct"}}]
    });
    let catalog = ReferenceCatalog {
        upstreams: [id(90)].into(),
        ..ReferenceCatalog::default()
    };
    let checked = parse_configuration(
        &serde_json::to_vec(&input).unwrap(),
        InputFormat::Json,
        &catalog,
    )
    .unwrap();
    let before = serde_json::to_vec(checked.document()).unwrap();
    let profile = &checked.document().profiles[0];
    let custom = &profile.custom_rules[0];
    let RuleAction::Dns(bypass) = &custom.action else {
        panic!("DNS fixture")
    };
    let traffic = &profile.custom_rules[1];
    let RuleAction::Traffic(outbound) = &traffic.action else {
        panic!("traffic fixture")
    };
    let filter = RuleSource::Filter(id(101));
    let filter_action = DnsAction::Block {};
    // Synthetic prepared filter conditions only: no catalog payload/import claim.
    let domain_rule = |value: &str| {
        DomainRule::new(
            RuleMetadata {
                source: filter,
                state: AdministrativeState::Enabled,
                priority: 0,
            },
            DomainMatcher::new(value.parse().unwrap(), DomainMatchMode::Exact),
            &filter_action,
        )
    };
    let policies = PolicySet::new(
        vec![PreparedProfile {
            id: profile.id,
            dns_default: &profile.default_dns_action,
            dns_rules: vec![
                DomainRule::new(
                    RuleMetadata {
                        source: RuleSource::Custom(custom.id),
                        state: AdministrativeState::Enabled,
                        priority: custom.priority,
                    },
                    custom.matcher.normalized().unwrap(),
                    bypass,
                ),
                domain_rule("bypass.example"),
                domain_rule("blocked.example"),
            ],
            tcp_default: &profile.default_traffic_action,
            tcp_rules: vec![TcpRule::new(
                DomainRule::new(
                    RuleMetadata {
                        source: RuleSource::Custom(traffic.id),
                        state: AdministrativeState::Enabled,
                        priority: traffic.priority,
                    },
                    traffic.matcher.normalized().unwrap(),
                    outbound,
                ),
                TcpPortScope::Range(TcpPortRange::new(443, 443).unwrap()),
            )],
        }],
        vec![DnsEndpointAssignment {
            endpoint: id(10),
            profile: profile.id,
            state: AdministrativeState::Enabled,
            device: None,
        }],
        vec![GatewayProfileAssignment {
            group: id(20),
            profile: profile.id,
        }],
    )
    .unwrap();
    let binding = DnsProfileBinding {
        endpoint: id(10),
        profile: profile.id,
    };
    for (name, expected) in [
        ("BYPASS.Example.", bypass),
        ("blocked.example", &filter_action),
        ("unmatched.example", &profile.default_dns_action),
    ] {
        assert_eq!(
            *policies
                .evaluate_dns(&binding, &name.parse().unwrap())
                .unwrap()
                .action(),
            expected
        );
    }
    let identity = GatewayNetworkIdentity {
        grant: id(30),
        group: id(20),
        listener: id(40),
        source: "192.0.2.1".parse().unwrap(),
    };
    let name = "bypass.example".parse().unwrap();
    assert_eq!(
        *policies
            .evaluate_tcp(&identity, &name, NonZeroU16::new(443).unwrap())
            .unwrap()
            .action(),
        outbound
    );
    assert_eq!(
        *policies
            .evaluate_tcp(&identity, &name, NonZeroU16::new(80).unwrap())
            .unwrap()
            .action(),
        &TrafficAction::Deny {}
    );
    assert_eq!(serde_json::to_vec(checked.document()).unwrap(), before);
    assert_eq!(profile.custom_rules[0].matcher.value, "Bypass.Example.");
}
