use droutex_common::{
    DeviceId, DnsUpstreamId, EndpointId, FilterListRevisionId, GatewayNetworkGrantId,
    LegacyNetworkGrantId, ListenerId, NetworkGroupId, PrivateDnsRecordId, ProfileId,
    ProxyOutboundId, ResolverCredentialId, RuleId, SecretRefId, ServiceGroupRevisionId,
};
use serde::{Deserialize, Deserializer, Serialize, Serializer, de};
use std::{
    collections::BTreeSet,
    net::{IpAddr, Ipv4Addr, Ipv6Addr, SocketAddr},
    str::FromStr,
};

/// External configuration data. It is not compiled policy or runtime authority.
#[derive(Debug, Deserialize, Serialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct Configuration {
    pub schema_version: u32,
    #[serde(default)]
    pub profiles: Vec<Profile>,
    #[serde(default)]
    pub endpoints: Vec<DnsEndpoint>,
    #[serde(default)]
    pub resolver_credentials: Vec<CredentialMetadata>,
    #[serde(default)]
    pub listeners: Vec<Listener>,
    #[serde(default)]
    pub network_groups: Vec<NetworkGroup>,
    #[serde(default)]
    pub legacy_grants: Vec<LegacyGrant>,
    #[serde(default)]
    pub gateway_grants: Vec<GatewayGrant>,
    #[serde(default)]
    pub outbounds: Vec<ProxyOutbound>,
    #[serde(default)]
    pub private_records: Vec<PrivateRecord>,
}

/// Trusted external resource inventory supplied by the caller, never by the document.
/// Existence is not cryptographic verification or permission to use a live resource.
#[derive(Debug, Default)]
pub struct ReferenceCatalog {
    pub devices: BTreeSet<DeviceId>,
    pub upstreams: BTreeSet<DnsUpstreamId>,
    pub secrets: BTreeSet<SecretRefId>,
    pub filter_revisions: BTreeSet<FilterListRevisionId>,
    pub service_revisions: BTreeSet<ServiceGroupRevisionId>,
}

/// Reusable Profile with fixed custom/service/filter/default layers.
#[derive(Debug, Deserialize, Serialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct Profile {
    #[serde(deserialize_with = "resource_id", serialize_with = "serialize_id")]
    pub id: ProfileId,
    pub name: String,
    #[serde(deserialize_with = "super::value::object")]
    pub default_dns_action: DnsAction,
    #[serde(deserialize_with = "super::value::object")]
    pub default_traffic_action: TrafficAction,
    #[serde(default)]
    pub custom_rules: Vec<CustomRule>,
    #[serde(default)]
    pub service_rules: Vec<ServiceRule>,
    #[serde(default)]
    pub filter_lists: Vec<FilterSelection>,
}

#[derive(Debug, Deserialize, Serialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct DnsEndpoint {
    #[serde(deserialize_with = "resource_id", serialize_with = "serialize_id")]
    pub id: EndpointId,
    pub name: String,
    pub enabled: bool,
    #[serde(deserialize_with = "resource_id", serialize_with = "serialize_id")]
    pub profile: ProfileId,
    #[serde(
        default,
        deserialize_with = "optional_id",
        serialize_with = "serialize_optional_id"
    )]
    pub device: Option<DeviceId>,
    #[serde(deserialize_with = "super::value::objects")]
    pub access: Vec<DnsAccess>,
}

/// Each transport states its own admission source, allowing both on one Endpoint.
#[derive(Debug, Deserialize, Serialize)]
#[serde(
    tag = "transport",
    rename_all = "snake_case",
    rename_all_fields = "camelCase",
    deny_unknown_fields
)]
pub enum DnsAccess {
    Doh {
        #[serde(deserialize_with = "resource_id", serialize_with = "serialize_id")]
        listener: ListenerId,
        #[serde(deserialize_with = "resource_ids", serialize_with = "serialize_ids")]
        credentials: Vec<ResolverCredentialId>,
    },
    Dot {
        #[serde(deserialize_with = "resource_id", serialize_with = "serialize_id")]
        listener: ListenerId,
        #[serde(deserialize_with = "resource_ids", serialize_with = "serialize_ids")]
        credentials: Vec<ResolverCredentialId>,
    },
    Udp {
        #[serde(deserialize_with = "resource_id", serialize_with = "serialize_id")]
        listener: ListenerId,
        #[serde(deserialize_with = "resource_ids", serialize_with = "serialize_ids")]
        grants: Vec<LegacyNetworkGrantId>,
    },
    Tcp {
        #[serde(deserialize_with = "resource_id", serialize_with = "serialize_id")]
        listener: ListenerId,
        #[serde(deserialize_with = "resource_ids", serialize_with = "serialize_ids")]
        grants: Vec<LegacyNetworkGrantId>,
    },
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, Deserialize, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum CredentialState {
    Active,
    Disabled,
    Expired,
    Revoked,
}

/// Public credential record only; importing this cannot authenticate a capability.
#[derive(Debug, Deserialize, Serialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct CredentialMetadata {
    #[serde(deserialize_with = "resource_id", serialize_with = "serialize_id")]
    pub id: ResolverCredentialId,
    #[serde(deserialize_with = "resource_id", serialize_with = "serialize_id")]
    pub endpoint: EndpointId,
    pub state: CredentialState,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, Ord, PartialOrd, Deserialize, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum ListenerTransport {
    DnsUdp,
    DnsTcp,
    Doh,
    Dot,
    GatewayTcp,
}

#[derive(Debug, Deserialize, Serialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct Listener {
    #[serde(deserialize_with = "resource_id", serialize_with = "serialize_id")]
    pub id: ListenerId,
    pub name: String,
    pub enabled: bool,
    pub bind: SocketAddr,
    pub transport: ListenerTransport,
    /// Explicit IPv4 acceptance by an IPv6 wildcard; otherwise IPv6-only intent.
    #[serde(default)]
    pub dual_stack: bool,
}

#[derive(Debug, Deserialize, Serialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct NetworkGroup {
    #[serde(deserialize_with = "resource_id", serialize_with = "serialize_id")]
    pub id: NetworkGroupId,
    pub name: String,
    /// Independent gateway policy selection, never learned from a DNS query.
    #[serde(deserialize_with = "resource_id", serialize_with = "serialize_id")]
    pub traffic_profile: ProfileId,
}

/// Static desired scope; P4-01 must establish current source/expiry/revocation.
#[derive(Debug, Deserialize, Serialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct LegacyGrant {
    #[serde(deserialize_with = "resource_id", serialize_with = "serialize_id")]
    pub id: LegacyNetworkGrantId,
    pub source: IpAddr,
    #[serde(deserialize_with = "resource_id", serialize_with = "serialize_id")]
    pub listener: ListenerId,
    #[serde(deserialize_with = "resource_id", serialize_with = "serialize_id")]
    pub group: NetworkGroupId,
    #[serde(deserialize_with = "resource_id", serialize_with = "serialize_id")]
    pub endpoint: EndpointId,
}

/// Separate gateway scope; this data is not a live destination-binding permit.
#[derive(Debug, Deserialize, Serialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct GatewayGrant {
    #[serde(deserialize_with = "resource_id", serialize_with = "serialize_id")]
    pub id: GatewayNetworkGrantId,
    pub source: IpAddr,
    #[serde(deserialize_with = "resource_id", serialize_with = "serialize_id")]
    pub listener: ListenerId,
    #[serde(deserialize_with = "resource_id", serialize_with = "serialize_id")]
    pub group: NetworkGroupId,
    #[serde(default)]
    pub targets: Vec<DomainMatch>,
    #[serde(
        default,
        deserialize_with = "resource_ids",
        serialize_with = "serialize_ids"
    )]
    pub services: Vec<ServiceGroupRevisionId>,
    pub ports: Vec<PortRange>,
    #[serde(deserialize_with = "resource_ids", serialize_with = "serialize_ids")]
    pub outbounds: Vec<ProxyOutboundId>,
}

#[derive(Debug, Deserialize, Serialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct ProxyOutbound {
    #[serde(deserialize_with = "resource_id", serialize_with = "serialize_id")]
    pub id: ProxyOutboundId,
    pub name: String,
    pub enabled: bool,
    #[serde(deserialize_with = "super::value::object")]
    pub transport: OutboundKind,
}

#[derive(Debug, Deserialize, Serialize)]
#[serde(
    tag = "kind",
    rename_all = "snake_case",
    rename_all_fields = "camelCase",
    deny_unknown_fields
)]
pub enum OutboundKind {
    VpsDirect {},
    HttpConnectTls {
        host: String,
        port: u16,
        tls_server_name: String,
        #[serde(
            default,
            deserialize_with = "optional_id",
            serialize_with = "serialize_optional_id"
        )]
        secret: Option<SecretRefId>,
    },
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, Deserialize, Serialize)]
#[serde(tag = "kind", rename_all = "snake_case", deny_unknown_fields)]
pub enum DnsAction {
    Resolve {
        #[serde(deserialize_with = "resource_id", serialize_with = "serialize_id")]
        upstream: DnsUpstreamId,
    },
    Block {},
    Gateway {
        #[serde(deserialize_with = "resource_id", serialize_with = "serialize_id")]
        outbound: ProxyOutboundId,
    },
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, Deserialize, Serialize)]
#[serde(tag = "kind", rename_all = "snake_case", deny_unknown_fields)]
pub enum TrafficAction {
    Deny {},
    Outbound {
        #[serde(deserialize_with = "resource_id", serialize_with = "serialize_id")]
        outbound: ProxyOutboundId,
    },
}

#[derive(Debug, Deserialize, Serialize)]
#[serde(
    tag = "plane",
    content = "action",
    rename_all = "snake_case",
    deny_unknown_fields
)]
pub enum RuleAction {
    Dns(#[serde(deserialize_with = "super::value::object")] DnsAction),
    Traffic(#[serde(deserialize_with = "super::value::object")] TrafficAction),
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, Deserialize, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum MatchMode {
    Exact,
    Wildcard,
    Suffix,
}

/// Original wire text, retained unchanged in serialization and signed snapshots.
/// Canonical matching derives a separate value; this is not live target authority.
#[derive(Debug, Deserialize, Serialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct DomainMatch {
    pub mode: MatchMode,
    pub value: String,
}

impl DomainMatch {
    /// Validate and derive the explicit CR-0002 matcher without rewriting wire text.
    pub fn normalized(&self) -> Result<droutex_policy::DomainMatcher, droutex_policy::DomainError> {
        use droutex_policy::{DomainMatchMode, DomainMatcher};
        let mode = match self.mode {
            MatchMode::Exact => DomainMatchMode::Exact,
            MatchMode::Suffix => DomainMatchMode::DomainAndSubdomains,
            MatchMode::Wildcard => DomainMatchMode::SubdomainsOnly,
        };
        Ok(DomainMatcher::new(self.value.parse()?, mode))
    }
}

#[derive(Debug, Deserialize, Serialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct PortRange {
    pub from: u16,
    pub to: u16,
}

#[derive(Debug, Deserialize, Serialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct CustomRule {
    #[serde(deserialize_with = "resource_id", serialize_with = "serialize_id")]
    pub id: RuleId,
    pub enabled: bool,
    pub priority: i32,
    pub matcher: DomainMatch,
    #[serde(default)]
    pub ports: Vec<PortRange>,
    pub action: RuleAction,
}

#[derive(Debug, Deserialize, Serialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct ServiceRule {
    #[serde(deserialize_with = "resource_id", serialize_with = "serialize_id")]
    pub id: RuleId,
    pub enabled: bool,
    pub priority: i32,
    #[serde(deserialize_with = "resource_id", serialize_with = "serialize_id")]
    pub revision: ServiceGroupRevisionId,
    pub action: RuleAction,
}

/// Immutable verified revision reference; dataset payload import remains P2-07.
#[derive(Debug, Deserialize, Serialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct FilterSelection {
    #[serde(deserialize_with = "resource_id", serialize_with = "serialize_id")]
    pub revision: FilterListRevisionId,
    pub enabled: bool,
    pub priority: i32,
}

#[derive(Debug, Deserialize, Serialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct PrivateRecord {
    #[serde(deserialize_with = "resource_id", serialize_with = "serialize_id")]
    pub id: PrivateDnsRecordId,
    #[serde(deserialize_with = "resource_id", serialize_with = "serialize_id")]
    pub profile: ProfileId,
    pub name: String,
    pub ttl: u32,
    #[serde(deserialize_with = "super::value::object")]
    pub record: RecordData,
}

#[derive(Debug, Deserialize, Serialize)]
#[serde(tag = "kind", rename_all = "snake_case", deny_unknown_fields)]
pub enum RecordData {
    A { address: Ipv4Addr },
    Aaaa { address: Ipv6Addr },
    Cname { target: String },
}

fn resource_id<'de, D: Deserializer<'de>, T: FromStr>(d: D) -> Result<T, D::Error> {
    String::deserialize(d)?
        .parse()
        .map_err(|_| de::Error::custom("invalid resource ID"))
}

struct Id<T>(T);
impl<'de, T: FromStr> Deserialize<'de> for Id<T> {
    fn deserialize<D: Deserializer<'de>>(d: D) -> Result<Self, D::Error> {
        resource_id(d).map(Self)
    }
}

fn resource_ids<'de, D: Deserializer<'de>, T: FromStr>(d: D) -> Result<Vec<T>, D::Error> {
    Vec::<Id<T>>::deserialize(d).map(|ids| ids.into_iter().map(|id| id.0).collect())
}

fn optional_id<'de, D: Deserializer<'de>, T: FromStr>(d: D) -> Result<Option<T>, D::Error> {
    Option::<Id<T>>::deserialize(d).map(|id| id.map(|id| id.0))
}

fn serialize_id<T: std::fmt::Display, S: Serializer>(
    value: &T,
    serializer: S,
) -> Result<S::Ok, S::Error> {
    serializer.collect_str(value)
}
fn serialize_ids<T: std::fmt::Display, S: Serializer>(
    values: &[T],
    serializer: S,
) -> Result<S::Ok, S::Error> {
    use serde::ser::SerializeSeq;
    let mut sequence = serializer.serialize_seq(Some(values.len()))?;
    for value in values {
        sequence.serialize_element(&value.to_string())?;
    }
    sequence.end()
}
fn serialize_optional_id<T: std::fmt::Display, S: Serializer>(
    value: &Option<T>,
    serializer: S,
) -> Result<S::Ok, S::Error> {
    value
        .as_ref()
        .map(ToString::to_string)
        .serialize(serializer)
}
