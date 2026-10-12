//! The YAML policy file (spec §8.3). These types mirror the file one to one;
//! `Policy::compile` turns them into a checked policy.

use serde::{Deserialize, Serialize};

pub const API_VERSION: &str = "veyra.io/v1alpha1";
pub const KIND: &str = "PolicyBundle";

#[derive(Debug, Clone, Deserialize)]
#[serde(deny_unknown_fields, rename_all = "camelCase")]
pub struct Bundle {
    pub api_version: String,
    pub kind: String,
    pub metadata: Metadata,
    pub spec: BundleSpec,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Metadata {
    pub name: String,
}

#[derive(Debug, Clone, Default, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct BundleSpec {
    #[serde(default)]
    pub nodes: Vec<NodeSpec>,
    #[serde(default)]
    pub profiles: Vec<ProfileSpec>,
    #[serde(default)]
    pub devices: Vec<DeviceSpec>,
    #[serde(default)]
    pub rules: Vec<RuleSpec>,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct NodeSpec {
    pub id: String,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(deny_unknown_fields, rename_all = "camelCase")]
pub struct ProfileSpec {
    pub id: String,
    #[serde(default)]
    pub default_action: DefaultAction,
}

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Deserialize, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum DefaultAction {
    #[default]
    Direct,
    Block,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct DeviceSpec {
    pub id: String,
    pub profile: String,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(deny_unknown_fields, rename_all = "camelCase")]
pub struct RuleSpec {
    pub id: String,
    #[serde(default = "enabled_by_default")]
    pub enabled: bool,
    pub scope: Scope,
    #[serde(rename = "match")]
    pub matcher: MatchSpec,
    #[serde(default)]
    pub dns: Option<DnsSpec>,
    pub action: Action,
    #[serde(default = "all_modes")]
    pub modes_allowed: Vec<Mode>,
}

fn enabled_by_default() -> bool {
    true
}

fn all_modes() -> Vec<Mode> {
    vec![Mode::SmartdnsTcp, Mode::ExplicitProxy, Mode::Tunnel]
}

#[derive(Debug, Clone, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct MatchSpec {
    pub domains: Vec<String>,
}

/// DNS action (spec §6.4): which resolver group answers. Never selects egress.
#[derive(Debug, Clone, PartialEq, Eq, Deserialize, Serialize)]
#[serde(deny_unknown_fields, rename_all = "camelCase")]
pub struct DnsSpec {
    pub upstream_group: String,
}

/// Who a rule applies to (spec §8.2.1, step 2: device > profile > default).
#[derive(Debug, Clone, PartialEq, Eq, Hash, Deserialize, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum Scope {
    Default,
    Profile(String),
    Device(String),
}

/// Route action (spec §6.4). The single-node MVP has direct, block and proxy_node.
#[derive(Debug, Clone, PartialEq, Eq, Deserialize, Serialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum Action {
    Direct,
    Block,
    ProxyNode {
        node: String,
        #[serde(default, rename = "onFailure")]
        on_failure: OnFailure,
    },
}

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Deserialize, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum OnFailure {
    #[default]
    Block,
    Direct,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Deserialize, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum Mode {
    SmartdnsTcp,
    ExplicitProxy,
    Tunnel,
}

impl Bundle {
    pub fn from_yaml(text: &str) -> Result<Bundle, String> {
        serde_saphyr::from_str(text).map_err(|e| e.to_string())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const EXAMPLE: &str = include_str!("../../../configs/examples/home.yaml");

    #[test]
    fn parses_the_example_config() {
        let b = Bundle::from_yaml(EXAMPLE).unwrap();
        assert_eq!(b.api_version, API_VERSION);
        assert_eq!(b.metadata.name, "home-default");
        let rules = &b.spec.rules;
        assert_eq!(rules.len(), 4);
        assert_eq!(rules[0].scope, Scope::Profile("home".into()));
        assert_eq!(
            rules[0].action,
            Action::ProxyNode {
                node: "de-01".into(),
                on_failure: OnFailure::Block
            }
        );
        assert_eq!(rules[2].scope, Scope::Device("work-laptop".into()));
        assert_eq!(
            rules[2].dns,
            Some(DnsSpec {
                upstream_group: "corp".into()
            })
        );
        assert_eq!(rules[3].scope, Scope::Default);
        assert_eq!(rules[3].modes_allowed, all_modes());
        assert!(rules.iter().all(|r| r.enabled));
    }

    #[test]
    fn on_failure_defaults_to_block() {
        let yaml = "type: proxy_node\nnode: de-01\n";
        let a: Action = serde_saphyr::from_str(yaml).unwrap();
        assert_eq!(
            a,
            Action::ProxyNode {
                node: "de-01".into(),
                on_failure: OnFailure::Block
            }
        );
    }

    #[test]
    fn rejects_unknown_fields_with_a_line_number() {
        let yaml = EXAMPLE.replace(
            "      enabled: true\n",
            "      enabled: true\n      priority: 100\n",
        );
        let err = Bundle::from_yaml(&yaml).unwrap_err();
        assert!(err.contains("unknown field `priority`"), "{err}");
        assert!(err.contains("line"), "{err}");
    }

    #[test]
    fn rejects_unknown_action_types() {
        let yaml = EXAMPLE.replace("type: block", "type: teleport");
        let err = Bundle::from_yaml(&yaml).unwrap_err();
        assert!(err.contains("teleport"), "{err}");
    }
}
