//! D53 upstream resource data. Parsing supplies no live admission or dial authority.

use serde::{Deserialize, Deserializer, de};
use std::{
    fmt,
    net::{IpAddr, SocketAddr},
};

pub const MAX_RESOURCE_BYTES: usize = 4096;
pub const MAX_GROUPS: usize = 64;
pub const MAX_MEMBERS: usize = 128;

#[derive(Clone, Copy, Debug, Eq, PartialEq, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Transport {
    UdpTcp,
    Tcp,
}
#[derive(Clone, Copy, Debug, Eq, PartialEq, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Fallback {
    None,
    OnFailure,
}
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum Error {
    Size,
    Format,
    Members,
    Address,
}
impl fmt::Display for Error {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "DNS upstream resource {:?}", self)
    }
}
impl std::error::Error for Error {}

#[derive(Clone, Copy, Debug, Eq, PartialEq, Deserialize)]
#[serde(try_from = "Object<RawMember>")]
pub struct Member {
    transport: Transport,
    address: SocketAddr,
}
impl Member {
    pub fn transport(self) -> Transport {
        self.transport
    }
    pub fn address(self) -> SocketAddr {
        self.address
    }
}
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct RawMember {
    #[serde(deserialize_with = "string_enum")]
    transport: Transport,
    address: String,
}
impl TryFrom<Object<RawMember>> for Member {
    type Error = Error;
    fn try_from(Object(raw): Object<RawMember>) -> Result<Self, Error> {
        let address: SocketAddr = raw.address.parse().map_err(|_| Error::Address)?;
        if address.to_string() != raw.address
            || address.port() != 53
            || matches!(address, SocketAddr::V6(ip) if ip.scope_id() != 0 || ip.flowinfo() != 0)
            || !public_address(address.ip())
        {
            return Err(Error::Address);
        }
        Ok(Self {
            transport: raw.transport,
            address,
        })
    }
}
#[derive(Debug, Eq, PartialEq)]
pub struct Group {
    fallback: Fallback,
    members: Vec<Member>,
}
impl Group {
    pub fn fallback(&self) -> Fallback {
        self.fallback
    }
    pub fn members(&self) -> &[Member] {
        &self.members
    }
}
#[derive(Deserialize)]
enum Format {
    #[serde(rename = "droutex-dns-upstream-v1")]
    V1,
}
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct RawGroup {
    #[serde(deserialize_with = "string_enum")]
    format: Format,
    #[serde(deserialize_with = "string_enum")]
    fallback: Fallback,
    #[serde(deserialize_with = "members")]
    members: Vec<Member>,
}

// Serde struct derives also accept sequences; D53 requires JSON objects.
struct Object<T>(T);
impl<'de, T: Deserialize<'de>> Deserialize<'de> for Object<T> {
    fn deserialize<D: Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        struct Visitor<T>(std::marker::PhantomData<T>);
        impl<'de, T: Deserialize<'de>> de::Visitor<'de> for Visitor<T> {
            type Value = Object<T>;
            fn expecting(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
                f.write_str("an upstream object")
            }
            fn visit_map<A: de::MapAccess<'de>>(self, map: A) -> Result<Self::Value, A::Error> {
                T::deserialize(de::value::MapAccessDeserializer::new(map)).map(Object)
            }
        }
        deserializer.deserialize_map(Visitor(std::marker::PhantomData))
    }
}

fn string_enum<'de, D: Deserializer<'de>, T: Deserialize<'de>>(
    deserializer: D,
) -> Result<T, D::Error> {
    // Unit enum derives also accept {"variant": null}; require a string first.
    let value = String::deserialize(deserializer)?;
    T::deserialize(de::value::StringDeserializer::new(value))
}
fn members<'de, D: Deserializer<'de>>(deserializer: D) -> Result<Vec<Member>, D::Error> {
    struct Visitor;
    impl<'de> de::Visitor<'de> for Visitor {
        type Value = Vec<Member>;
        fn expecting(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
            f.write_str("one or two upstream members")
        }
        fn visit_seq<A: de::SeqAccess<'de>>(
            self,
            mut sequence: A,
        ) -> Result<Self::Value, A::Error> {
            let first = sequence
                .next_element()?
                .ok_or_else(|| de::Error::custom("empty upstream group"))?;
            let second = sequence.next_element()?;
            if sequence.next_element::<de::IgnoredAny>()?.is_some() {
                return Err(de::Error::custom("too many upstream members"));
            }
            // No collection allocation until the actual member count is bounded.
            Ok(match second {
                Some(second) => vec![first, second],
                None => vec![first],
            })
        }
    }
    deserializer.deserialize_seq(Visitor)
}

/// Strict bounded v1 resource codec. CP/DP must still authenticate and pin its bytes.
pub fn parse(bytes: &[u8]) -> Result<Group, Error> {
    if bytes.len() > MAX_RESOURCE_BYTES {
        return Err(Error::Size);
    }
    // Serde diagnostics may echo input. Keep them inside this boundary.
    let Object(raw): Object<RawGroup> = serde_json::from_slice(bytes).map_err(|_| Error::Format)?;
    let Format::V1 = raw.format;
    if raw.members.len() == 2
        && (raw.fallback == Fallback::None || raw.members[0].address == raw.members[1].address)
    {
        return Err(Error::Members);
    }
    Ok(Group {
        fallback: raw.fallback,
        members: raw.members,
    })
}

fn public_address(address: IpAddr) -> bool {
    // D53 deliberately excludes protocol-special ranges, even globally reachable
    // exceptions. Registry identity checked2026-10-28, not a routability promise:
    // https://www.iana.org/assignments/iana-ipv4-special-registry
    // https://www.iana.org/assignments/iana-ipv6-special-registry
    match address {
        IpAddr::V4(ip) => {
            let prefixes = [
                ([0, 0, 0, 0], 8),
                ([10, 0, 0, 0], 8),
                ([100, 64, 0, 0], 10),
                ([127, 0, 0, 0], 8),
                ([169, 254, 0, 0], 16),
                ([172, 16, 0, 0], 12),
                ([192, 0, 0, 0], 24),
                ([192, 0, 2, 0], 24),
                ([192, 31, 196, 0], 24),
                ([192, 52, 193, 0], 24),
                ([192, 88, 99, 0], 24),
                ([192, 168, 0, 0], 16),
                ([192, 175, 48, 0], 24),
                ([198, 18, 0, 0], 15),
                ([198, 51, 100, 0], 24),
                ([203, 0, 113, 0], 24),
                ([224, 0, 0, 0], 4),
                ([240, 0, 0, 0], 4),
                // Azure's special host/platform address is outside private ranges.
                // https://learn.microsoft.com/en-us/azure/virtual-network/what-is-ip-address-168-63-129-16
                ([168, 63, 129, 16], 32),
            ];
            let value = u32::from(ip);
            !prefixes.into_iter().any(|(prefix, bits)| {
                value >> (32 - bits) == u32::from_be_bytes(prefix) >> (32 - bits)
            })
        }
        IpAddr::V6(ip) => {
            let s = ip.segments();
            s[0] & 0xe000 == 0x2000
                && !(s[0] == 0x2001 && s[1] & 0xfe00 == 0)
                && !(s[0] == 0x2001 && s[1] == 0x0db8)
                && s[0] != 0x2002
                && s[0] != 0x3ffe
                && !(s[0] == 0x3fff && s[1] & 0xf000 == 0)
                && !(s[0] == 0x2620 && s[1] == 0x004f && s[2] == 0x8000)
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    const VALID: &[u8] = br#"{"format":"droutex-dns-upstream-v1","fallback":"none","members":[{"transport":"udp_tcp","address":"1.1.1.1:53"}]}"#;

    #[test]
    fn resource_shapes_require_objects_and_string_enums() {
        for input in [
            br#"["droutex-dns-upstream-v1","none",[{"transport":"udp_tcp","address":"1.1.1.1:53"}]]"#.as_slice(),
            br#"{"format":"droutex-dns-upstream-v1","fallback":"none","members":[["udp_tcp","1.1.1.1:53"]]}"#,
            br#"{"format":{"droutex-dns-upstream-v1":null},"fallback":"none","members":[{"transport":"udp_tcp","address":"1.1.1.1:53"}]}"#,
            br#"{"format":"droutex-dns-upstream-v1","fallback":{"none":null},"members":[{"transport":"udp_tcp","address":"1.1.1.1:53"}]}"#,
            br#"{"format":"droutex-dns-upstream-v1","fallback":"none","members":[{"transport":{"udp_tcp":null},"address":"1.1.1.1:53"}]}"#,
        ] {
            assert_eq!(parse(input), Err(Error::Format), "{input:?}");
        }
    }

    #[test]
    fn independent_resources_keep_priority_and_refuse_implicit_fallback_or_endpoint_syntax() {
        assert_eq!(
            parse(VALID).unwrap().members()[0].address().to_string(),
            "1.1.1.1:53"
        );
        let pair = br#"{"format":"droutex-dns-upstream-v1","fallback":"on_failure","members":[{"transport":"tcp","address":"[2606:4700:4700::1111]:53"},{"transport":"udp_tcp","address":"1.1.1.1:53"}]}"#;
        let group = parse(pair).unwrap();
        assert_eq!(group.members()[0].transport(), Transport::Tcp);
        assert_eq!(group.members()[1].address().to_string(), "1.1.1.1:53");
        for (from, to) in [
            ("droutex-dns-upstream-v1", "droutex-dns-upstream-v2"),
            ("udp_tcp", "tls"),
            ("1.1.1.1:53", "dns.example:53"),
            ("1.1.1.1:53", "1.1.1.1:5353"),
            ("1.1.1.1:53", "1.1.1.1:053"),
        ] {
            let input = String::from_utf8(VALID.to_vec()).unwrap().replace(from, to);
            assert!(parse(input.as_bytes()).is_err(), "{from} replaced by {to}");
        }
        assert_eq!(parse(pair).unwrap().fallback(), Fallback::OnFailure);
        let mut at_limit = VALID.to_vec();
        at_limit.resize(MAX_RESOURCE_BYTES, b' ');
        assert!(parse(&at_limit).is_ok());
        at_limit.push(b' ');
        assert_eq!(parse(&at_limit), Err(Error::Size));
        for input in [
            String::from_utf8(pair.to_vec())
                .unwrap()
                .replace("on_failure", "none"),
            String::from_utf8(pair.to_vec())
                .unwrap()
                .replace("[2606:4700:4700::1111]:53", "1.1.1.1:53"),
            String::from_utf8(VALID.to_vec())
                .unwrap()
                .replace("\"fallback\":", "\"extra\":0,\"fallback\":"),
            String::from_utf8(VALID.to_vec())
                .unwrap()
                .replace("\"fallback\":", "\"fallback\":\"none\",\"fallback\":"),
            String::from_utf8(VALID.to_vec()).unwrap().replace(
                "[{\"transport\":\"udp_tcp\",\"address\":\"1.1.1.1:53\"}]",
                "[]",
            ),
        ] {
            assert!(parse(input.as_bytes()).is_err());
        }
        for input in [
            br#"{"format":"droutex-dns-upstream-v1","fallback":"on_failure","members":[{"transport":"tcp","address":"1.1.1.1:53"},{"transport":"tcp","address":"8.8.8.8:53"},{"transport":"tcp","address":"9.9.9.9:53"}]}"#.as_slice(),
            br#"{"format":"droutex-dns-upstream-v1","fallback":"none","members":[{"transport":"tcp","address":"1.1.1.1:53","extra":true}]}"#,
            br#"{"format":"droutex-dns-upstream-v1","fallback":"none","members":[{"transport":"tcp","address":"1.1.1.1:53","address":"8.8.8.8:53"}]}"#,
            br#"{"format":"droutex-dns-upstream-v1","fallback":"none","members":[null]}"#,
            br#"{"format":"droutex-dns-upstream-v1","fallback":"none","members":[{"transport":"tcp","address":"[2606:4700:4700::1111%1]:53"}]}"#,
        ] {
            assert!(parse(input).is_err(), "{input:?}");
        }
        let error = parse(br#"{"format":"droutex-dns-upstream-v1","fallback":"none","members":[{"transport":"tcp","address":"private-value.invalid:53"}]}"#).unwrap_err();
        assert!(!format!("{error} {error:?}").contains("private-value"));
    }

    #[test]
    fn destination_policy_refuses_special_ranges_and_keeps_public_neighbours() {
        for address in [
            "0.1.2.3",
            "10.255.255.255",
            "100.64.0.0",
            "100.127.255.255",
            "127.9.8.7",
            "169.254.169.254",
            "172.16.0.0",
            "172.31.255.255",
            "192.0.0.9",
            "192.0.2.1",
            "192.31.196.0",
            "192.52.193.255",
            "192.88.99.2",
            "192.168.1.1",
            "192.175.48.1",
            "198.18.0.0",
            "198.19.255.255",
            "198.51.100.1",
            "203.0.113.1",
            "224.0.0.1",
            "255.255.255.255",
            "168.63.129.16",
            "::",
            "::1",
            "::ffff:1.1.1.1",
            "64:ff9b::a9fe:a9fe",
            "fc00::1",
            "fe80::1",
            "ff02::1",
            "2001:1::3",
            "2001:db8::1",
            "2002:7f00:1::",
            "3ffe::1",
            "3fff:fff:ffff::1",
            "2620:4f:8000::1",
        ] {
            assert!(!public_address(address.parse().unwrap()), "{address}");
        }
        for address in [
            "1.1.1.1",
            "76.76.2.2",
            "100.63.255.255",
            "100.128.0.0",
            "172.15.255.255",
            "172.32.0.0",
            "192.0.1.255",
            "192.0.3.0",
            "198.17.255.255",
            "198.20.0.0",
            "168.63.129.15",
            "168.63.129.17",
            "2001:4860:4860::8888",
            "2606:4700:4700::1111",
            "2620:fe::fe",
            "3fff:1000::1",
        ] {
            assert!(public_address(address.parse().unwrap()), "{address}");
        }
    }
}
