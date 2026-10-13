# Veyra Gateway Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** `veyra-dataplane` also serves the mode B gateway. It accepts TCP on 443 (routed by SNI) and 80 (routed by `Host`) from admitted sources, but only for names DNS sends to this node. It resolves the origin without going through the policy, refuses internal and special-purpose addresses, and passes the bytes through untouched.

**Architecture:** `veyra-policy` gains the `gateway:` section and the single decision the gateway needs: `routes_to(node, name)`, the union over every profile and device of what DNS sends to that node. `veyra-dns` exports `lookup`, which resolves straight through the upstream servers. A new `veyra-gateway` crate reads the destination from the first bytes (`sniff`), judges origin addresses (`Egress`), runs one connection (`Gateway::handle`), and owns the TCP listeners. `apps/veyra-dataplane` starts it next to DNS.

**Tech Stack:** Rust 2024; no new third-party crates. tokio (sockets, `copy_bidirectional`), thiserror, tracing, and hickory-proto through `veyra-dns` for origin lookups. ClientHello and HTTP head parsing is hand-written, read-only and bounds-checked.

**Spec:** `Veyra_Project_Specification_RU.md` — §3.2 (SNI and ECH), §4.2 (mode B conditions), §6.2 (the passthrough flow), §9.6 (the ACL the gateway shares with DNS), §10.1–10.3 (gateway roles, destination binding, outbound DNS), §19.2 items 3–5 (no open proxy, forbidden destinations, check after every resolve), §19.3 (privacy), §20.4 (reason codes), §21.5 (port model; SNI demultiplexing steps 3–4), §22.3 (network matrix rows for mode B), §23.2 stage 1a, §24.3 REQ-GW-001…004.

## Where this plan sits

Stage 1a (spec §23.2) is four plans. Plans 1 (policy core and CLI) and 2 (DNS engine) are merged; this is plan 3. Plan 4 adds link-IP admission and the DoH/link-IP HTTPS endpoint behind the gateway (§21.5 step 1). It also adds the snapshot file, the control socket, ordered apply with `drain_window`, and systemd deployment.

## Global Constraints

- Everything from plans 1 and 2 still holds: edition 2024, `rust-version = "1.85"`, dependencies declared once in `[workspace.dependencies]`, `Cargo.lock` committed, no `unwrap()` outside tests, `unsafe_code = "forbid"`, fmt + clippy `-D warnings` + tests green on every commit.
- Integration-test helper functions use `expect`, never `unwrap`: `allow-unwrap-in-tests` covers `#[test]` functions only.
- No new third-party dependencies. The gateway uses tokio, thiserror, tracing, and `veyra-dns` for lookups.
- Decisions live in `veyra-policy` (`routes_to`). `veyra-gateway` owns the sockets, the sniffing and the egress checks; the binary holds no gateway logic.
- Spec §10.1, §21.5: TLS is never terminated or altered, and HTTP requests are not modified. The origin receives exactly the bytes the client sent.
- Spec §10.2: the gateway passes a name when DNS, in mode B, sends some profile or device to this node for it. Everything else is closed (fail-closed).
- Spec §9.6, REQ-GW-004: a source that is not admitted is closed before a byte is read.
- Spec §10.3, §19.2 items 4–5, REQ-GW-003: origins resolve straight through the `default` upstream group, never through the policy. Every resolved address is checked, and special-purpose networks and Veyra's own endpoints are never dialed.
- Spec §10.1, REQ-GW-002: the gateway binds TCP only. UDP on its ports stays closed, so the kernel answers with ICMP port unreachable.
- No domain names or client addresses in logs (spec §19.3): a refusal is logged at debug with its reason only.
- `veyra explain` output is unchanged.
- Run every command from the repository root `E:\LeetCode\VeyraDNS`.

## Review Focus

Inputs a person will meet that the spec implies but does not spell out, most likely first. Each is pinned by the named test.

1. **Chrome's ECH GREASE:** a ClientHello with an `encrypted_client_hello` extension but a real SNI must still route. Real ECH, whose outer SNI is the provider's public name, is refused as not routed. Pinned by `keeps_the_sni_next_to_ech_grease` (Task 3).
2. **ClientHellos split across TCP segments or TLS records**, as larger post-quantum key shares make common: the gateway keeps reading until the hello is complete. Pinned by `asks_for_more_until_the_client_hello_is_complete` and `reads_a_client_hello_split_across_records` (Task 3), and `a_client_hello_in_pieces_still_routes` (Task 5).
3. **DNS rebinding, mixed answers, and disguised private addresses:** a name that resolved to a public address may next resolve to a private one; one answer may mix both; private IPv4 may hide inside IPv4-mapped or NAT64 IPv6. Pinned by `every_connection_checks_what_the_name_resolves_to_now` and `forbidden_addresses_are_skipped_for_allowed_ones` (Task 5), and `refuses_special_purpose_addresses` (Task 4).
4. **A name DNS routes here for a different profile than the connecting source's**, such as a device with its own DNS token behind a home NAT admitted as another profile: the gateway serves the union. Pinned by `a_gateway_passes_what_dns_sends_it_for_any_profile_or_device` (Task 1) and `names_routed_here_for_another_profile_pass` (Task 5).
5. **Clients that connect and never speak, or that come in a flood:** a silent client frees its session after the hello timeout, and connections past the cap are closed at once, not queued. Pinned by `a_silent_client_is_dropped_after_the_hello_timeout` (Task 5) and `connections_beyond_the_cap_are_closed_at_once` (Task 6).

## File map

| File | Responsibility |
|---|---|
| `crates/veyra-policy/src/gateway.rs` | `gateway:` section → `GatewayConfig`; `Policy::routes_to` |
| `crates/veyra-policy/tests/gateway.rs` | Public-API tests of the above |
| `crates/veyra-dns/src/upstream.rs` | Adds `lookup`: origin addresses straight from upstream servers |
| `crates/veyra-dns/tests/lookup.rs` | Its tests |
| `crates/veyra-gateway/src/sniff.rs` | `Service`, `Sniff`, `sniff`: SNI or `Host` from the first bytes |
| `crates/veyra-gateway/src/egress.rs` | `Egress`: addresses the gateway never dials |
| `crates/veyra-gateway/src/gateway.rs` | `Gateway::handle` for one connection; `Refusal` |
| `crates/veyra-gateway/src/server.rs` | `Listeners`: TCP sockets and the session cap |
| `crates/veyra-gateway/tests/support/mod.rs` | ClientHello builder, fake upstream, echo origin, test gateway |
| `crates/veyra-gateway/tests/fixtures/*.hex` | Real ClientHellos (OpenSSL, Schannel) |
| `apps/veyra-dataplane/src/main.rs` | Starts the gateway listeners next to DNS |

---

### Task 1: The `gateway:` section and the names a gateway passes

**Files:**
- Create: `crates/veyra-policy/src/gateway.rs`, `crates/veyra-policy/tests/gateway.rs`
- Modify: `crates/veyra-policy/src/compile.rs`, `crates/veyra-policy/src/dns_config.rs`, `crates/veyra-policy/src/lib.rs`, `crates/veyra-policy/src/model.rs`

**Interfaces:**
- Consumes: `BundleSpec`, `NodeSpec.gateway`, `Policy::compile`, `IssueCode`, `issue()` (plans 1–2); `Policy::decide_in`, `Mode::SmartdnsTcp`, `Subject` (plan 2); `socket_addr` in `dns_config.rs`, which becomes `pub(crate)` here.
- Produces (model): `BundleSpec.gateway: Option<GatewayEngineSpec>`; `GatewayEngineSpec { node: String, tls: Vec<String>, http: Vec<String> }` (a bare IP means port 443 for `tls`, 80 for `http`).
- Produces (compiled): `GatewayConfig { node: String, tls: Vec<SocketAddr>, http: Vec<SocketAddr> }`; `Policy::gateway_config(&self) -> Option<&GatewayConfig>`; `Policy::gateways(&self) -> &BTreeMap<String, GatewaySpec>`; `Policy::routes_to(&self, node: &str, name: &Name) -> bool`, true when `decide_in(subject, name, Some(Mode::SmartdnsTcp))` routes `ProxyNode { node, .. }` for some profile (without a device) or some device.
- Checks: unknown node → `UnknownReference`; a node without gateway addresses → `MissingGateway`; `gateway:` without `dns:` → `InvalidValue` (origins resolve through `dns.upstreams`); neither `tls` nor `http` → `InvalidValue`; a bad address → `InvalidAddress`.

- [ ] **Step 1: Write the failing tests**

Create `crates/veyra-policy/tests/gateway.rs`:

````rust
//! The `gateway:` section and which names a node's gateway passes (spec §10).

use std::net::SocketAddr;
use veyra_policy::{Bundle, Issue, IssueCode, Name, Policy, Severity};

/// Two gateway nodes and one without, two profiles and a device; `extra` is
/// appended under `spec:`.
fn compile(extra: &str) -> Result<(Policy, Vec<Issue>), Vec<Issue>> {
    let text = format!(
        "apiVersion: veyra.io/v1alpha1\nkind: PolicyBundle\nmetadata: {{ name: t }}\nspec:\n  \
         nodes:\n    \
         - {{ id: de-01, gateway: {{ ipv4: 203.0.113.10 }} }}\n    \
         - {{ id: us-01, gateway: {{ ipv4: 198.51.100.20, ipv6: \"2001:db8::20\" }} }}\n    \
         - {{ id: bare }}\n  \
         profiles: [{{ id: home }}, {{ id: strict, defaultAction: block }}]\n  \
         devices: [{{ id: tv, profile: home }}]\n  \
         rules:\n    \
         - {{ id: media, scope: {{ profile: home }}, match: {{ domains: [\"suffix:media.example\"] }}, action: {{ type: proxy_node, node: de-01 }} }}\n    \
         - {{ id: login, scope: {{ profile: home }}, match: {{ domains: [login.media.example] }}, action: {{ type: direct }} }}\n    \
         - {{ id: video, scope: {{ profile: strict }}, match: {{ domains: [\"suffix:video.example\"] }}, action: {{ type: proxy_node, node: de-01 }} }}\n    \
         - {{ id: tv-sport, scope: {{ device: tv }}, match: {{ domains: [sport.example] }}, action: {{ type: proxy_node, node: de-01 }} }}\n    \
         - {{ id: us, scope: default, match: {{ domains: [\"suffix:us.example\"] }}, action: {{ type: proxy_node, node: us-01 }} }}\n    \
         - {{ id: tunnel-only, scope: default, match: {{ domains: [tunnel.example] }}, action: {{ type: proxy_node, node: de-01 }}, modesAllowed: [tunnel] }}\n    \
         - {{ id: ads, scope: default, match: {{ domains: [\"suffix:ads.example\"] }}, action: {{ type: block }} }}\n\
         {extra}"
    );
    Policy::compile(&Bundle::from_yaml(&text).expect("test YAML parses"))
}

const DNS: &str = "  dns:\n    listen: [\"127.0.0.1:5353\"]\n    upstreams: [{ id: default, servers: [\"1.1.1.1\"] }]\n";

/// Error codes only: warnings from other sections do not belong to these tests.
fn errors(extra: &str) -> Vec<IssueCode> {
    compile(extra)
        .expect_err("expected errors")
        .iter()
        .filter(|i| i.severity == Severity::Error)
        .map(|i| i.code)
        .collect()
}

fn addr(s: &str) -> SocketAddr {
    s.parse().expect("test address parses")
}

fn name(s: &str) -> Name {
    Name::parse(s).expect("test name parses")
}

#[test]
fn parses_the_gateway_section_with_default_ports() {
    let gateway = "  gateway:\n    node: de-01\n    tls: [\"0.0.0.0\", \"[::]:8443\"]\n    http: [\"127.0.0.1:8080\"]\n";
    let (policy, _warnings) = compile(&format!("{DNS}{gateway}")).unwrap();
    let config = policy.gateway_config().unwrap();
    assert_eq!(config.node, "de-01");
    assert_eq!(config.tls, vec![addr("0.0.0.0:443"), addr("[::]:8443")]);
    assert_eq!(config.http, vec![addr("127.0.0.1:8080")]);

    let gateways = policy.gateways();
    assert_eq!(
        gateways.keys().collect::<Vec<_>>(),
        ["de-01", "us-01"],
        "nodes with a gateway"
    );
    assert_eq!(
        gateways["us-01"].ipv6,
        Some("2001:db8::20".parse().unwrap())
    );
    assert!(compile(DNS).unwrap().0.gateway_config().is_none());
}

#[test]
fn rejects_bad_gateway_sections() {
    use IssueCode::*;
    let gateway = |body: &str| format!("{DNS}  gateway:\n{body}");
    assert_eq!(
        errors(&gateway("    node: nowhere\n    tls: [\"0.0.0.0\"]\n")),
        vec![UnknownReference]
    );
    assert_eq!(
        errors(&gateway("    node: bare\n    tls: [\"0.0.0.0\"]\n")),
        vec![MissingGateway]
    );
    assert_eq!(
        errors(&gateway("    node: de-01\n")),
        vec![InvalidValue],
        "no listeners"
    );
    assert_eq!(
        errors(&gateway(
            "    node: de-01\n    tls: [\"localhost:443\"]\n    http: [\"0.0.0.0:http\"]\n"
        )),
        vec![InvalidAddress, InvalidAddress]
    );
    assert_eq!(
        errors("  gateway:\n    node: de-01\n    tls: [\"0.0.0.0\"]\n"),
        vec![InvalidValue],
        "origins resolve through the dns section"
    );
}

#[test]
fn a_gateway_passes_what_dns_sends_it_for_any_profile_or_device() {
    let (policy, _warnings) = compile(DNS).unwrap();
    let passes = |node: &str, n: &str| policy.routes_to(node, &name(n));
    assert!(passes("de-01", "media.example"));
    assert!(passes("de-01", "www.media.example"));
    assert!(
        passes("de-01", "video.example"),
        "only strict routes it here; the gateway cannot tell profiles apart"
    );
    assert!(passes("de-01", "sport.example"), "a device rule routes it");
    assert!(passes("us-01", "www.us.example"));
    assert!(
        !passes("de-01", "login.media.example"),
        "home sends it direct, strict blocks it"
    );
    assert!(!passes("de-01", "us.example"), "routed to another node");
    assert!(!passes("de-01", "tunnel.example"), "not allowed in mode B");
    assert!(!passes("de-01", "ads.example"));
    assert!(
        !passes("de-01", "example.org"),
        "no rule: a profile default never proxies"
    );
}
````

- [ ] **Step 2: Run the tests to see them fail**

Run: `cargo test -p veyra-policy --test gateway`
Expected: compilation fails: ``no method named `gateway_config` found for struct `Policy` `` (twice), ``no method named `gateways` ``, ``no method named `routes_to` ``.

- [ ] **Step 3: Write the implementation**

In `crates/veyra-policy/src/compile.rs`, apply this patch (save it to a file and run `git apply <file>`, or edit by hand):

````diff
diff --git a/crates/veyra-policy/src/compile.rs b/crates/veyra-policy/src/compile.rs
index c9959e2..0c108f3 100644
--- a/crates/veyra-policy/src/compile.rs
+++ b/crates/veyra-policy/src/compile.rs
@@ -4,6 +4,7 @@
 use crate::admission::{Cidr, compile_admission};
 use crate::decide::Subject;
 use crate::dns_config::{DnsConfig, compile_dns};
+use crate::gateway::{GatewayConfig, compile_gateway};
 use crate::model::{
     API_VERSION, Action, Bundle, DefaultAction, DnsSpec, GatewaySpec, KIND, Mode, OnFailure, Scope,
 };
@@ -98,6 +99,7 @@ pub struct Policy {
     pub(crate) gateways: BTreeMap<String, GatewaySpec>,
     pub(crate) dns: Option<DnsConfig>,
     pub(crate) bindings: Vec<(Cidr, Subject)>,
+    pub(crate) gateway_config: Option<GatewayConfig>,
 }
 
 pub(crate) fn issue(severity: Severity, code: IssueCode, rules: &[&str], message: String) -> Issue {
@@ -222,6 +224,7 @@ impl Policy {
         check_overlaps(&rules, &mut issues);
         let dns = compile_dns(spec, &mut issues);
         let bindings = compile_admission(spec, &mut issues);
+        let gateway_config = compile_gateway(spec, &mut issues);
 
         if issues.iter().any(|i| i.severity == Error) {
             return Err(issues);
@@ -246,6 +249,7 @@ impl Policy {
                 .collect(),
             dns,
             bindings,
+            gateway_config,
         };
         Ok((policy, issues))
     }
@@ -264,6 +268,11 @@ impl Policy {
     pub fn gateway(&self, node: &str) -> Option<&GatewaySpec> {
         self.gateways.get(node)
     }
+
+    /// Every node's gateway endpoint, by node id.
+    pub fn gateways(&self) -> &BTreeMap<String, GatewaySpec> {
+        &self.gateways
+    }
 }
 
 pub(crate) fn unique_ids<'a>(
````

In `crates/veyra-policy/src/dns_config.rs`, apply this patch (save it to a file and run `git apply <file>`, or edit by hand):

````diff
diff --git a/crates/veyra-policy/src/dns_config.rs b/crates/veyra-policy/src/dns_config.rs
index 1cecd66..3c789db 100644
--- a/crates/veyra-policy/src/dns_config.rs
+++ b/crates/veyra-policy/src/dns_config.rs
@@ -104,7 +104,7 @@ pub(crate) fn compile_dns(spec: &BundleSpec, issues: &mut Vec<Issue>) -> Option<
 }
 
 /// `ip:port` or `[ipv6]:port`; with a `default_port`, a bare IP too.
-fn socket_addr(
+pub(crate) fn socket_addr(
     text: &str,
     default_port: Option<u16>,
     field: &str,
````

Create `crates/veyra-policy/src/gateway.rs`:

````rust
//! The `gateway:` section (spec §10) and which names a node's gateway passes (§10.2).

use crate::compile::{Issue, IssueCode, Policy, Severity, issue};
use crate::decide::Subject;
use crate::dns_config::socket_addr;
use crate::model::{Action, BundleSpec, Mode};
use crate::name::Name;
use std::net::SocketAddr;

/// The checked `gateway:` section: this data plane is the gateway of `node`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct GatewayConfig {
    pub node: String,
    /// Where TLS clients connect; the destination comes from SNI.
    pub tls: Vec<SocketAddr>,
    /// Where HTTP clients connect; the destination comes from `Host`.
    pub http: Vec<SocketAddr>,
}

/// `None` when the bundle has no `gateway:` section.
pub(crate) fn compile_gateway(spec: &BundleSpec, issues: &mut Vec<Issue>) -> Option<GatewayConfig> {
    use IssueCode::*;
    let gateway = spec.gateway.as_ref()?;
    let error = |code, message: String| issue(Severity::Error, code, &[], message);

    match spec.nodes.iter().find(|n| n.id == gateway.node) {
        None => {
            let msg = format!("gateway.node: unknown node {}", gateway.node);
            issues.push(error(UnknownReference, msg));
        }
        Some(node) if node.gateway.is_none() => {
            let msg = format!(
                "gateway.node: node {} has no gateway addresses for DNS to hand out",
                node.id
            );
            issues.push(error(MissingGateway, msg));
        }
        Some(_) => {}
    }
    // Origins resolve through dns.upstreams, never through the policy (spec §10.3).
    if spec.dns.is_none() {
        let msg = "gateway needs a dns section: origins resolve through its default upstream group";
        issues.push(error(InvalidValue, msg.into()));
    }
    if gateway.tls.is_empty() && gateway.http.is_empty() {
        let msg = "gateway listens nowhere: give tls or http addresses";
        issues.push(error(InvalidValue, msg.into()));
    }
    let tls = gateway
        .tls
        .iter()
        .filter_map(|a| socket_addr(a, Some(443), "gateway.tls", issues))
        .collect();
    let http = gateway
        .http
        .iter()
        .filter_map(|a| socket_addr(a, Some(80), "gateway.http", issues))
        .collect();
    Some(GatewayConfig {
        node: gateway.node.clone(),
        tls,
        http,
    })
}

impl Policy {
    /// The checked `gateway:` section, if the bundle has one.
    pub fn gateway_config(&self) -> Option<&GatewayConfig> {
        self.gateway_config.as_ref()
    }

    /// Spec §10.2: the gateway of `node` passes `name` when DNS, in mode B, sends
    /// some profile or device to `node` for it. A connection does not say which
    /// profile it belongs to, so the gateway serves all of them.
    // ponytail: one decision per profile and device on every connection; index
    // names by node if bundles grow to thousands of profiles and devices.
    pub fn routes_to(&self, node: &str, name: &Name) -> bool {
        let profiles = self.profiles.keys().map(|p| Subject {
            profile: p.clone(),
            device: None,
        });
        let devices = self.devices.iter().map(|(d, p)| Subject {
            profile: p.clone(),
            device: Some(d.clone()),
        });
        profiles.chain(devices).any(|subject| {
            matches!(
                self.decide_in(&subject, name, Some(Mode::SmartdnsTcp)).route,
                Action::ProxyNode { node: to, .. } if to == node
            )
        })
    }
}
````

In `crates/veyra-policy/src/lib.rs`, apply this patch (save it to a file and run `git apply <file>`, or edit by hand):

````diff
diff --git a/crates/veyra-policy/src/lib.rs b/crates/veyra-policy/src/lib.rs
index 5c26895..ce16913 100644
--- a/crates/veyra-policy/src/lib.rs
+++ b/crates/veyra-policy/src/lib.rs
@@ -5,6 +5,7 @@ mod answer;
 mod compile;
 mod decide;
 mod dns_config;
+mod gateway;
 mod model;
 mod name;
 mod pattern;
@@ -14,6 +15,7 @@ pub use answer::{DnsPlan, QueryKind};
 pub use compile::{Issue, IssueCode, Policy, Rule, Severity};
 pub use decide::{DecidedBy, Decision, Subject, SubjectError};
 pub use dns_config::{DEFAULT_UPSTREAM_GROUP, DnsConfig};
+pub use gateway::GatewayConfig;
 pub use model::*;
 pub use name::{Name, NameError};
 pub use pattern::{Pattern, PatternError, Specificity};
````

In `crates/veyra-policy/src/model.rs`, apply this patch (save it to a file and run `git apply <file>`, or edit by hand):

````diff
diff --git a/crates/veyra-policy/src/model.rs b/crates/veyra-policy/src/model.rs
index 6ebb772..ffec580 100644
--- a/crates/veyra-policy/src/model.rs
+++ b/crates/veyra-policy/src/model.rs
@@ -40,6 +40,23 @@ pub struct BundleSpec {
     /// Source networks allowed to use DNS (spec §9.6). Empty admits nobody.
     #[serde(default)]
     pub admission: Vec<AdmissionSpec>,
+    /// The gateway (spec §10). Without it this data plane serves DNS only.
+    #[serde(default)]
+    pub gateway: Option<GatewayEngineSpec>,
+}
+
+/// The `gateway:` section: this data plane is the gateway of `node` and
+/// accepts the connections DNS sends there (spec §10.1, §10.2).
+#[derive(Debug, Clone, Deserialize)]
+#[serde(deny_unknown_fields)]
+pub struct GatewayEngineSpec {
+    pub node: String,
+    /// `ip:port` for TLS, routed by SNI; a bare IP means port 443.
+    #[serde(default)]
+    pub tls: Vec<String>,
+    /// `ip:port` for HTTP, routed by `Host`; a bare IP means port 80.
+    #[serde(default)]
+    pub http: Vec<String>,
 }
 
 /// One `admission` entry: a network and exactly one of `profile` or `device`.
````

- [ ] **Step 4: Run the tests to see them pass**

Run: `cargo test --workspace`
Expected: `gateway` `ok. 3 passed`; 115 tests pass in total, none fail.

- [ ] **Step 5: Format and lint**

Run: `cargo fmt --all && cargo clippy --workspace --all-targets -- -D warnings`
Expected: clippy prints no warnings.

- [ ] **Step 6: Commit**

````bash
git add crates/veyra-policy/src/compile.rs crates/veyra-policy/src/dns_config.rs crates/veyra-policy/src/gateway.rs crates/veyra-policy/src/lib.rs crates/veyra-policy/src/model.rs crates/veyra-policy/tests/gateway.rs
git commit -m "feat(policy): gateway section and the names a node's gateway passes"
````

---

### Task 2: Origin lookups that bypass the policy

**Files:**
- Create: `crates/veyra-dns/tests/lookup.rs`
- Modify: `crates/veyra-dns/src/lib.rs`, `crates/veyra-dns/src/upstream.rs`

**Interfaces:**
- Consumes: `upstream::exchange` (plan 2: each server in turn, UDP with TCP fallback, Veyra's own id and OPT record).
- Produces: `veyra_dns::lookup(servers: &[SocketAddr], name: &str, wait: Duration) -> Vec<IpAddr>`: the A answers, then the AAAA answers, of replies with RCODE NOERROR, whatever CNAME chain the upstream returns with them; empty when nothing resolves. No cache, no policy.

- [ ] **Step 1: Write the failing tests**

Create `crates/veyra-dns/tests/lookup.rs`:

````rust
//! Origin lookups for the gateway (spec §10.3): straight to the upstream, no policy.

mod support;

use hickory_proto::op::{Message, OpCode, ResponseCode};
use hickory_proto::rr::rdata::{A, AAAA, CNAME};
use hickory_proto::rr::{Name, RData, Record, RecordType};
use std::net::{IpAddr, Ipv4Addr, Ipv6Addr};
use std::time::Duration;
use support::{dead_upstream, upstream};
use veyra_dns::lookup;

const WAIT: Duration = Duration::from_millis(300);
const V4: Ipv4Addr = Ipv4Addr::new(93, 184, 215, 14);
const V6: Ipv6Addr = Ipv6Addr::new(0x2606, 0x2800, 0x21f, 0, 0, 0, 0, 0x14);

/// A reply to `query` with `code` and `answers`.
fn reply(query: &Message, code: ResponseCode, answers: Vec<Record>) -> Message {
    let mut reply = Message::response(query.metadata.id, OpCode::Query);
    reply.metadata.response_code = code;
    reply.add_queries(query.queries.iter().cloned());
    for record in answers {
        reply.add_answer(record);
    }
    reply
}

/// Any name is a CNAME to `edge.example.`, which has `V4` and `V6`.
fn behind_a_cname(query: &Message) -> Message {
    let edge = Name::from_ascii("edge.example.").expect("name");
    let alias = RData::CNAME(CNAME(edge.clone()));
    let address = match query.queries[0].query_type() {
        RecordType::A => RData::A(A(V4)),
        _ => RData::AAAA(AAAA(V6)),
    };
    let answers = vec![
        Record::from_rdata(query.queries[0].name().clone(), 300, alias),
        Record::from_rdata(edge, 300, address),
    ];
    reply(query, ResponseCode::NoError, answers)
}

#[tokio::test]
async fn returns_ipv4_then_ipv6_through_cnames() {
    let up = upstream(behind_a_cname).await;
    let addrs = lookup(&[up.addr], "www.origin.example", WAIT).await;
    assert_eq!(addrs, vec![IpAddr::V4(V4), IpAddr::V6(V6)]);

    let mut asked: Vec<(String, RecordType)> = up
        .received()
        .iter()
        .map(|q| (q.queries[0].name().to_ascii(), q.queries[0].query_type()))
        .collect();
    asked.sort();
    assert_eq!(
        asked,
        [
            ("www.origin.example.".to_string(), RecordType::A),
            ("www.origin.example.".to_string(), RecordType::AAAA)
        ]
    );
}

#[tokio::test]
async fn errors_and_dead_upstreams_resolve_to_nothing() {
    let nxdomain = upstream(|q| reply(q, ResponseCode::NXDomain, Vec::new())).await;
    assert!(
        lookup(&[nxdomain.addr], "gone.example", WAIT)
            .await
            .is_empty()
    );
    let dead = dead_upstream().await;
    assert!(lookup(&[dead], "www.origin.example", WAIT).await.is_empty());
    assert!(lookup(&[], "www.origin.example", WAIT).await.is_empty());
}
````

- [ ] **Step 2: Run the tests to see them fail**

Run: `cargo test -p veyra-dns --test lookup`
Expected: compilation fails: ``unresolved import `veyra_dns::lookup` ``.

- [ ] **Step 3: Write the implementation**

In `crates/veyra-dns/src/lib.rs`, apply this patch (save it to a file and run `git apply <file>`, or edit by hand):

````diff
diff --git a/crates/veyra-dns/src/lib.rs b/crates/veyra-dns/src/lib.rs
index cb91540..ed1c389 100644
--- a/crates/veyra-dns/src/lib.rs
+++ b/crates/veyra-dns/src/lib.rs
@@ -9,3 +9,4 @@ mod upstream;
 
 pub use engine::{Engine, Transport};
 pub use server::Listeners;
+pub use upstream::lookup;
````

In `crates/veyra-dns/src/upstream.rs`, apply this patch (save it to a file and run `git apply <file>`, or edit by hand):

````diff
diff --git a/crates/veyra-dns/src/upstream.rs b/crates/veyra-dns/src/upstream.rs
index 0985a0c..0676309 100644
--- a/crates/veyra-dns/src/upstream.rs
+++ b/crates/veyra-dns/src/upstream.rs
@@ -1,9 +1,10 @@
 //! Forwarding to upstream resolvers.
 
 use crate::synth::MAX_UDP_PAYLOAD;
-use hickory_proto::op::{Edns, Message, MessageType};
+use hickory_proto::op::{Edns, Message, MessageType, Query, ResponseCode};
+use hickory_proto::rr::{Name, RData, RecordType};
 use std::io;
-use std::net::{Ipv4Addr, Ipv6Addr, SocketAddr};
+use std::net::{IpAddr, Ipv4Addr, Ipv6Addr, SocketAddr};
 use std::time::Duration;
 use tokio::io::{AsyncRead, AsyncReadExt, AsyncWrite, AsyncWriteExt};
 use tokio::net::{TcpStream, UdpSocket};
@@ -32,6 +33,34 @@ pub(crate) async fn exchange(
     None
 }
 
+/// The addresses of `name` straight from upstream servers, IPv4 first: how the
+/// gateway finds origins (spec §10.3). It bypasses the policy, so the gateway
+/// never gets a gateway address back from Veyra. Empty when nothing resolves.
+// ponytail: no cache, two upstream queries per gateway connection; cache by TTL
+// if connection setup latency shows up.
+pub async fn lookup(servers: &[SocketAddr], name: &str, wait: Duration) -> Vec<IpAddr> {
+    let Ok(name) = Name::from_ascii(format!("{}.", name.trim_end_matches('.'))) else {
+        return Vec::new();
+    };
+    let ask = |record_type| {
+        let mut request = Message::query();
+        request.add_query(Query::query(name.clone(), record_type));
+        async move { exchange(servers, &request, wait).await }
+    };
+    let (v4, v6) = tokio::join!(ask(RecordType::A), ask(RecordType::AAAA));
+    [v4, v6]
+        .into_iter()
+        .flatten()
+        .filter(|reply| reply.metadata.response_code == ResponseCode::NoError)
+        .flat_map(|reply| reply.answers)
+        .filter_map(|record| match record.data {
+            RData::A(a) => Some(IpAddr::V4(a.0)),
+            RData::AAAA(aaaa) => Some(IpAddr::V6(aaaa.0)),
+            _ => None,
+        })
+        .collect()
+}
+
 /// The query Veyra sends upstream for `request`: a random id of its own, so a
 /// spoofed reply must guess it as well as the source port (spec §19.1), and an
 /// OPT record of its own carrying only the DO bit, so client options such as a
````

- [ ] **Step 4: Run the tests to see them pass**

Run: `cargo test --workspace`
Expected: `lookup` `ok. 2 passed`; 117 in total.

- [ ] **Step 5: Format and lint**

Run: `cargo fmt --all && cargo clippy --workspace --all-targets -- -D warnings`
Expected: clippy prints no warnings.

- [ ] **Step 6: Commit**

````bash
git add crates/veyra-dns/src/lib.rs crates/veyra-dns/src/upstream.rs crates/veyra-dns/tests/lookup.rs
git commit -m "feat(dns): resolve gateway origins straight through upstream servers"
````

---

### Task 3: `veyra-gateway`: the destination from SNI or `Host`

**Files:**
- Create: `crates/veyra-gateway/Cargo.toml`, `crates/veyra-gateway/src/lib.rs`, `crates/veyra-gateway/src/sniff.rs`, `crates/veyra-gateway/tests/fixtures/clienthello-openssl.hex`, `crates/veyra-gateway/tests/fixtures/clienthello-schannel.hex`, `crates/veyra-gateway/tests/sniff.rs`, `crates/veyra-gateway/tests/support/mod.rs`
- Modify: `Cargo.toml`

**Interfaces:**
- Consumes: `veyra_policy::Name` (plan 1).
- Produces: the crate `veyra-gateway`; `Service::{Tls, Http}` (`Copy`, `Debug`, `PartialEq`); `Sniff::{More, Name(Name), Refuse}`; `MAX_HEAD: usize = 32 * 1024`; `sniff(service: Service, head: &[u8]) -> Sniff`. It returns `More` while the ClientHello (in any number of handshake records) or the HTTP head is incomplete and shorter than `MAX_HEAD`. It returns `Name` for exactly one SNI host_name, or exactly one `Host` header (port stripped), that parses as a `Name` and is not an IP literal (all-digit last label). Anything else is `Refuse`.
- Test helpers in `tests/support/mod.rs`: `client_hello(sni: Option<&str>, extra: &[(u16, Vec<u8>)]) -> Vec<u8>`, `server_name(&str) -> Vec<u8>`, `hex(&str) -> Vec<u8>`. The two fixtures are real ClientHellos, captured from OpenSSL 3.2 (`s_client -servername Media.Example`) and from curl on Windows (Schannel).

- [ ] **Step 1: Write the failing tests**

In `Cargo.toml`, apply this patch (save it to a file and run `git apply <file>`, or edit by hand):

````diff
diff --git a/Cargo.toml b/Cargo.toml
index a7c5c82..594c626 100644
--- a/Cargo.toml
+++ b/Cargo.toml
@@ -1,6 +1,6 @@
 [workspace]
 resolver = "3"
-members = ["crates/veyra-policy", "crates/veyra-dns", "apps/veyra-cli", "apps/veyra-dataplane"]
+members = ["crates/veyra-policy", "crates/veyra-dns", "crates/veyra-gateway", "apps/veyra-cli", "apps/veyra-dataplane"]
 
 [workspace.package]
 version = "0.1.0"
````

Create `crates/veyra-gateway/Cargo.toml`:

````toml
[package]
name = "veyra-gateway"
version.workspace = true
edition.workspace = true
rust-version.workspace = true
publish.workspace = true

[dependencies]
veyra-policy.workspace = true

[lints]
workspace = true
````

Create `crates/veyra-gateway/src/lib.rs`:

````rust
//! Veyra gateway (spec §10).
````

Create `crates/veyra-gateway/tests/fixtures/clienthello-openssl.hex`:

````text
16030101430100013f0303fb6a5c0cc8b2e9cd9f9e9c5f1c4446163e94f8
2196227a5c2b0283b4caae636a2078e368243adabeea5cb61e6230473df9
ee226b190e9a013c6ccc17e6bb78becb003e130213031301c02cc030009f
cca9cca8ccaac02bc02f009ec024c028006bc023c0270067c00ac0140039
c009c0130033009d009c003d003c0035002f00ff010000b8000000120010
00000d4d656469612e4578616d706c65000b000403000102000a00160014
001d0017001e001900180100010101020103010400230000001600000017
0000000d0030002e04030503060308070808081a081b081c0809080a080b
080408050806040105010601030303010302040205020602002b00050403
040303002d00020101003300260024001d0020b371cfa8baa3a851d1dea7
d8f93761160eff86fda86e90ce949984d519f95f68001b0003020001
````

Create `crates/veyra-gateway/tests/fixtures/clienthello-schannel.hex`:

````text
16030101c7010001c30303a977cf9db97eb61ca76c2436b100b4f6b10b9c
7b5c07b06960707be921287b22208d00f89f0ab48689e348e450d5f418f1
76d17d0038ed70d63cab403f0e5d9be8002813021301c02cc02bc030c02f
c024c023c028c027c00ac009c014c013009d009c003d003c0035002f0100
015200000012001000000d6d656469612e6578616d706c65000500050100
000000002b00050403040303000d001a0018080408050806040105010201
04030503020302020601060300230000000a00080006001d00170018000b
000201000010000b000908687474702f312e31003300d000ce001d002048
5d9b80f30c90f27d69e4678ebbc98ae1ef033c55ecdabd472e6963a943d4
0a0017004104c46ee401f1bdc73a005214831859a88ee85bda35afdb1659
25c1846c54c771d6dc1ca02ff0ec7f81e714121e882a0438d8d0650ea92e
fbcfb3ad17523ceaa2430018006104a7dd0c85c60c20a52918f81cda19f0
37f53f811182bbdb2ba6bdf0b859e3a7c9a22223e2293a5052afbee879ce
0f2a0eb4e057a6fa7788b8669d51e36d61d4afd68f8b361be17f2fb91840
55df9065abea9be60d4082c933b7ccc0d748a183010031000000170000ff
01000100002d00020101
````

Create `crates/veyra-gateway/tests/sniff.rs`:

````rust
//! Reading the destination from the first bytes of a connection (spec §10.1).

mod support;

use support::{client_hello, hex, server_name};
use veyra_gateway::{MAX_HEAD, Service, Sniff, sniff};
use veyra_policy::Name;

/// Captured from `openssl s_client -servername Media.Example` (OpenSSL 3.2).
const OPENSSL: &str = include_str!("fixtures/clienthello-openssl.hex");
/// Captured from `curl https://media.example/` on Windows (Schannel).
const SCHANNEL: &str = include_str!("fixtures/clienthello-schannel.hex");

fn named(host: &str) -> Sniff {
    Sniff::Name(Name::parse(host).expect("test name parses"))
}

#[test]
fn reads_the_sni_of_real_client_hellos() {
    assert_eq!(sniff(Service::Tls, &hex(OPENSSL)), named("media.example"));
    assert_eq!(sniff(Service::Tls, &hex(SCHANNEL)), named("media.example"));
}

#[test]
fn asks_for_more_until_the_client_hello_is_complete() {
    for hello in [
        hex(OPENSSL),
        hex(SCHANNEL),
        client_hello(Some("media.example"), &[]),
    ] {
        for len in 0..hello.len() {
            assert_eq!(
                sniff(Service::Tls, &hello[..len]),
                Sniff::More,
                "{len} bytes"
            );
        }
    }
}

#[test]
fn reads_a_client_hello_split_across_records() {
    let hello = hex(OPENSSL);
    let handshake = &hello[5..];
    let mut split = Vec::new();
    for part in [&handshake[..100], &handshake[100..]] {
        split.extend([22, 3, 1]);
        split.extend(u16::try_from(part.len()).unwrap().to_be_bytes());
        split.extend(part);
    }
    assert_eq!(sniff(Service::Tls, &split[..110]), Sniff::More);
    assert_eq!(sniff(Service::Tls, &split), named("media.example"));
}

#[test]
fn keeps_the_sni_next_to_ech_grease() {
    // Chrome sends an encrypted_client_hello extension even when it has no ECH
    // config; the SNI is then the real name and must still route.
    let hello = client_hello(Some("media.example"), &[(0xfe0d, vec![0; 32])]);
    assert_eq!(sniff(Service::Tls, &hello), named("media.example"));
}

#[test]
fn refuses_client_hellos_without_one_usable_name() {
    let two_names = client_hello(Some("media.example"), &[(0, server_name("other.example"))]);
    for (why, hello) in [
        ("no SNI", client_hello(None, &[])),
        ("an IP literal", client_hello(Some("10.0.0.1"), &[])),
        ("not a domain name", client_hello(Some("bad name!"), &[])),
        ("two server_name extensions", two_names),
        ("HTTP on the TLS port", b"GET / HTTP/1.1\r\n\r\n".to_vec()),
        ("a TLS alert", vec![21, 3, 3, 0, 2, 2, 40]),
    ] {
        assert_eq!(sniff(Service::Tls, &hello), Sniff::Refuse, "{why}");
    }
}

#[test]
fn survives_corrupted_client_hellos() {
    let hello = hex(OPENSSL);
    for i in 0..hello.len() {
        for flip in [0x01, 0x80, 0xff] {
            let mut bad = hello.clone();
            bad[i] ^= flip;
            let _ = sniff(Service::Tls, &bad); // any answer, but no panic
        }
    }
}

#[test]
fn reads_the_host_header() {
    let head = b"GET /watch?v=1 HTTP/1.1\r\nUser-Agent: tv\r\nhOST: Media.Example:80\r\n\r\n";
    assert_eq!(sniff(Service::Http, head), named("media.example"));
    for len in 0..head.len() {
        assert_eq!(
            sniff(Service::Http, &head[..len]),
            Sniff::More,
            "{len} bytes"
        );
    }
}

#[test]
fn refuses_requests_without_one_usable_host() {
    for (why, head) in [
        ("HTTP/1.0 without Host", "GET / HTTP/1.0\r\n\r\n"),
        (
            "two Host headers",
            "GET / HTTP/1.1\r\nHost: media.example\r\nHost: other.example\r\n\r\n",
        ),
        (
            "a folded Host",
            "GET / HTTP/1.1\r\nHost: media.example\r\n other.example\r\n\r\n",
        ),
        (
            "an IPv4 literal",
            "GET / HTTP/1.1\r\nHost: 10.0.0.1:80\r\n\r\n",
        ),
        (
            "an IPv6 literal",
            "GET / HTTP/1.1\r\nHost: [::1]:80\r\n\r\n",
        ),
        (
            "a bad port",
            "GET / HTTP/1.1\r\nHost: media.example:http\r\n\r\n",
        ),
        ("an empty Host", "GET / HTTP/1.1\r\nHost:\r\n\r\n"),
    ] {
        assert_eq!(
            sniff(Service::Http, head.as_bytes()),
            Sniff::Refuse,
            "{why}"
        );
    }
    let not_utf8 = b"GET / HTTP/1.1\r\nHost: m\xffdia.example\r\n\r\n";
    assert_eq!(sniff(Service::Http, not_utf8), Sniff::Refuse);
}

#[test]
fn refuses_heads_that_never_name_a_destination() {
    // A ClientHello that claims 16 MiB, sent as 1000-byte records.
    let mut endless = Vec::new();
    let mut payload = vec![1, 0xff, 0xff, 0xff];
    while endless.len() < MAX_HEAD {
        payload.resize(1000, 0);
        endless.extend([22, 3, 1, 0x03, 0xe8]);
        endless.extend(&payload);
        payload.clear();
    }
    endless.truncate(MAX_HEAD);
    assert_eq!(sniff(Service::Tls, &endless[..MAX_HEAD - 1]), Sniff::More);
    assert_eq!(sniff(Service::Tls, &endless), Sniff::Refuse);

    let mut cookie = b"GET / HTTP/1.1\r\nCookie: ".to_vec();
    cookie.resize(MAX_HEAD, b'a');
    assert_eq!(sniff(Service::Http, &cookie[..MAX_HEAD - 1]), Sniff::More);
    assert_eq!(sniff(Service::Http, &cookie), Sniff::Refuse);
}
````

Create `crates/veyra-gateway/tests/support/mod.rs`:

````rust
//! Shared test helpers.
#![allow(dead_code)] // each test file uses a different subset

/// A ClientHello record naming `sni` (none for `None`), then the `extra`
/// extensions as (type, body).
pub fn client_hello(sni: Option<&str>, extra: &[(u16, Vec<u8>)]) -> Vec<u8> {
    let mut extensions = Vec::new();
    let mut push = |kind: u16, body: &[u8]| {
        extensions.extend(kind.to_be_bytes());
        extensions.extend(len16(body.len()));
        extensions.extend(body);
    };
    if let Some(name) = sni {
        push(0, &server_name(name));
    }
    for (kind, body) in extra {
        push(*kind, body);
    }
    let mut body = vec![3, 3]; // legacy_version: TLS 1.2
    body.extend([7; 32]); // random
    body.push(0); // no session id
    body.extend([0, 2, 0x13, 0x01]); // one cipher suite: TLS_AES_128_GCM_SHA256
    body.extend([1, 0]); // compression: null only
    body.extend(len16(extensions.len()));
    body.extend(extensions);
    let mut record = vec![22, 3, 1];
    record.extend(len16(body.len() + 4));
    record.push(1); // ClientHello
    record.extend(&u32::try_from(body.len()).expect("fits").to_be_bytes()[1..]);
    record.extend(body);
    record
}

/// The body of a server_name extension with one host_name entry.
pub fn server_name(name: &str) -> Vec<u8> {
    let mut entry = vec![0]; // host_name
    entry.extend(len16(name.len()));
    entry.extend(name.as_bytes());
    let mut body = len16(entry.len()).to_vec();
    body.extend(entry);
    body
}

fn len16(len: usize) -> [u8; 2] {
    u16::try_from(len)
        .expect("length fits in 16 bits")
        .to_be_bytes()
}

/// Bytes from a hex dump; whitespace is ignored.
pub fn hex(text: &str) -> Vec<u8> {
    let digits: Vec<u8> = text.bytes().filter(|b| !b.is_ascii_whitespace()).collect();
    digits
        .chunks(2)
        .map(|pair| {
            let pair = std::str::from_utf8(pair).expect("ASCII");
            u8::from_str_radix(pair, 16).expect("hex digit pair")
        })
        .collect()
}
````

- [ ] **Step 2: Run the tests to see them fail**

Run: `cargo test -p veyra-gateway`
Expected: compilation fails: ``unresolved imports `veyra_gateway::MAX_HEAD`, `veyra_gateway::Service`, `veyra_gateway::Sniff`, `veyra_gateway::sniff` ``.

- [ ] **Step 3: Write the implementation**

Replace `crates/veyra-gateway/src/lib.rs` with:

````rust
//! Veyra gateway (spec §10): TCP 443 routed by SNI and 80 by `Host`, passed
//! through to the origin without terminating TLS. Decisions come from
//! `veyra-policy`; this crate owns the sockets and the protocol sniffing.

mod sniff;

pub use sniff::{MAX_HEAD, Service, Sniff, sniff};
````

Create `crates/veyra-gateway/src/sniff.rs`:

````rust
//! The destination of a new connection, from its first bytes: the SNI of a TLS
//! ClientHello or the `Host` header of an HTTP request. The bytes are only
//! read, never changed (spec §10.1).

use veyra_policy::Name;

/// Which listener a connection arrived on.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Service {
    /// TLS, routed by SNI; the origin is dialed on port 443.
    Tls,
    /// HTTP/1.x, routed by `Host`; the origin is dialed on port 80.
    Http,
}

/// What the bytes read so far say about the destination.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Sniff {
    /// Not enough bytes yet: read more.
    More,
    /// The destination to route by.
    Name(Name),
    /// No usable destination: not TLS or HTTP, no SNI or `Host`, two of them,
    /// an IP literal, or `MAX_HEAD` bytes without an answer.
    Refuse,
}

/// The most a client may send before naming its destination. ClientHellos are
/// a few KiB at most; HTTP heads with cookies can reach tens of KiB.
pub const MAX_HEAD: usize = 32 * 1024;

const HANDSHAKE: u8 = 22;
const CLIENT_HELLO: usize = 1;
const SERVER_NAME: usize = 0;
const HOST_NAME: usize = 0;

/// Read the destination from the first bytes of a connection.
pub fn sniff(service: Service, head: &[u8]) -> Sniff {
    let found = match service {
        Service::Tls => server_name(head),
        Service::Http => http_host(head),
    };
    match found {
        Found::Host(host) => destination(&host).map_or(Sniff::Refuse, Sniff::Name),
        Found::Partial if head.len() < MAX_HEAD => Sniff::More,
        Found::Partial | Found::Nothing => Sniff::Refuse,
    }
}

/// What a parser found in the bytes so far.
enum Found {
    Partial,
    Host(String),
    Nothing,
}

/// The name to route by. IP literals are refused: the gateway dials names a
/// rule routes to it, never addresses a client picks (spec §10.2).
fn destination(host: &str) -> Option<Name> {
    let name = Name::parse(host).ok()?;
    // No top-level domain is all digits: this refuses `10.0.0.1`, `0x7f.1` and the like.
    let tld = name.as_str().rsplit('.').next()?;
    (!tld.bytes().all(|b| b.is_ascii_digit())).then_some(name)
}

/// TLS: the SNI of the ClientHello, which may span several handshake records
/// (RFC 8446 §5.1, RFC 6066 §3).
fn server_name(head: &[u8]) -> Found {
    let mut handshake = Vec::new();
    let mut rest = head;
    loop {
        let Some((header, after)) = rest.split_at_checked(5) else {
            return Found::Partial;
        };
        if header[0] != HANDSHAKE || header[1] != 3 {
            return Found::Nothing;
        }
        let len = usize::from(u16::from_be_bytes([header[3], header[4]]));
        if len == 0 || len > 1 << 14 {
            return Found::Nothing;
        }
        let Some((payload, after)) = after.split_at_checked(len) else {
            return Found::Partial;
        };
        handshake.extend_from_slice(payload);
        rest = after;
        if let Some(found) = client_hello(&handshake) {
            return found;
        }
    }
}

/// `None` until the whole handshake message is in.
fn client_hello(handshake: &[u8]) -> Option<Found> {
    let mut r = Reader(handshake);
    if r.number(1)? != CLIENT_HELLO {
        return Some(Found::Nothing);
    }
    let body = r.vector(3)?;
    Some(sni(body).map_or(Found::Nothing, Found::Host))
}

/// The host name of the only server_name extension; `None` without one, with
/// two (illegal, RFC 8446 §4.2: refuse rather than guess), or when malformed.
fn sni(body: &[u8]) -> Option<String> {
    let mut r = Reader(body);
    r.take(2 + 32)?; // legacy_version, random
    r.vector(1)?; // legacy_session_id
    r.vector(2)?; // cipher_suites
    r.vector(1)?; // legacy_compression_methods
    let mut extensions = Reader(r.vector(2)?);
    let mut names = Vec::new();
    while !extensions.is_empty() {
        let kind = extensions.number(2)?;
        let data = extensions.vector(2)?;
        if kind == SERVER_NAME {
            names.push(host_name(data)?);
        }
    }
    let [name] = <[String; 1]>::try_from(names).ok()?;
    Some(name)
}

/// A server_name extension body with exactly one entry, of type host_name.
fn host_name(data: &[u8]) -> Option<String> {
    let mut data = Reader(data);
    let mut list = Reader(data.vector(2)?);
    let kind = list.number(1)?;
    let name = list.vector(2)?;
    if kind != HOST_NAME || !list.is_empty() || !data.is_empty() {
        return None;
    }
    String::from_utf8(name.to_vec()).ok()
}

/// Bounds-checked reading of TLS wire structures (RFC 8446 §3).
struct Reader<'a>(&'a [u8]);

impl<'a> Reader<'a> {
    fn take(&mut self, n: usize) -> Option<&'a [u8]> {
        let (head, rest) = self.0.split_at_checked(n)?;
        self.0 = rest;
        Some(head)
    }

    /// A big-endian number of `bytes` bytes.
    fn number(&mut self, bytes: usize) -> Option<usize> {
        Some(
            self.take(bytes)?
                .iter()
                .fold(0, |n, b| n << 8 | usize::from(*b)),
        )
    }

    /// A vector with a length prefix of `bytes` bytes.
    fn vector(&mut self, bytes: usize) -> Option<&'a [u8]> {
        let len = self.number(bytes)?;
        self.take(len)
    }

    fn is_empty(&self) -> bool {
        self.0.is_empty()
    }
}

/// HTTP/1.x: the `Host` header of the request head (RFC 9112 §3.2).
fn http_host(head: &[u8]) -> Found {
    let Some(end) = head.windows(4).position(|w| w == b"\r\n\r\n") else {
        return Found::Partial;
    };
    let Ok(text) = std::str::from_utf8(&head[..end]) else {
        return Found::Nothing;
    };
    let mut hosts = Vec::new();
    for line in text.split("\r\n").skip(1) {
        // A folded line (obs-fold) could carry on a Host value: refuse it.
        if line.starts_with([' ', '\t']) {
            return Found::Nothing;
        }
        if let Some((name, value)) = line.split_once(':') {
            if name.eq_ignore_ascii_case("host") {
                hosts.push(value.trim());
            }
        }
    }
    // HTTP/1.0 without Host, or two Host headers: no single destination.
    match hosts[..] {
        [host] => strip_port(host).map_or(Found::Nothing, |h| Found::Host(h.into())),
        _ => Found::Nothing,
    }
}

/// `example.com:8080` → `example.com`.
fn strip_port(host: &str) -> Option<&str> {
    match host.rsplit_once(':') {
        Some((name, port)) => port.parse::<u16>().is_ok().then_some(name),
        None => Some(host),
    }
}
````

- [ ] **Step 4: Run the tests to see them pass**

Run: `cargo test --workspace`
Expected: `sniff` `ok. 9 passed`; 126 in total.

- [ ] **Step 5: Format and lint**

Run: `cargo fmt --all && cargo clippy --workspace --all-targets -- -D warnings`
Expected: clippy prints no warnings.

- [ ] **Step 6: Commit**

````bash
git add Cargo.lock Cargo.toml crates/veyra-gateway/Cargo.toml crates/veyra-gateway/src/lib.rs crates/veyra-gateway/src/sniff.rs crates/veyra-gateway/tests/fixtures/clienthello-openssl.hex crates/veyra-gateway/tests/fixtures/clienthello-schannel.hex crates/veyra-gateway/tests/sniff.rs crates/veyra-gateway/tests/support/mod.rs
git commit -m "feat(gateway): read the destination from the SNI or the Host header"
````

---

### Task 4: Forbidden origin addresses

**Files:**
- Create: `crates/veyra-gateway/src/egress.rs`, `crates/veyra-gateway/tests/egress.rs`
- Modify: `crates/veyra-gateway/src/lib.rs`

**Interfaces:**
- Consumes: `Cidr` (plan 2; `contains` judges IPv4-mapped IPv6 as IPv4), `Policy::gateways`, `Policy::gateway_config` (Task 1), `Policy::dns` (plan 2).
- Produces: `SPECIAL_PURPOSE: [&str; 27]` (the IANA special-purpose IPv4 and IPv6 networks, plus multicast); `Egress::new(policy: &Policy) -> Egress`, which refuses those networks, every node's gateway IPv4 and IPv6, and every specific DNS and gateway listen address; `Egress::allow(&mut self, network: Cidr)`, a carve-out for tests; `Egress::allows(&self, ip: IpAddr) -> bool`, which judges IPv4-mapped addresses as IPv4 and `64:ff9b::/96` NAT64 addresses by the IPv4 address inside.

- [ ] **Step 1: Write the failing tests**

Create `crates/veyra-gateway/tests/egress.rs`:

````rust
//! Where the gateway may connect (spec §19.2 item 4, REQ-GW-003).

use std::net::IpAddr;
use veyra_gateway::{Egress, SPECIAL_PURPOSE};
use veyra_policy::{Bundle, Cidr, Policy};

/// Two nodes with gateways, DNS on a specific and an unspecified address, and
/// a gateway on a specific TLS address.
fn policy() -> Policy {
    let yaml = r#"
apiVersion: veyra.io/v1alpha1
kind: PolicyBundle
metadata: { name: egress }
spec:
  nodes:
    - { id: de-01, gateway: { ipv4: 5.6.7.8, ipv6: "2a01:4f8::8" } }
    - { id: us-01, gateway: { ipv4: 9.9.8.8 } }
  profiles: [{ id: home }]
  admission: [{ cidr: 127.0.0.1/32, profile: home }]
  dns:
    listen: ["0.0.0.0:53", "5.6.7.9:53"]
    upstreams: [{ id: default, servers: ["1.1.1.1"] }]
  gateway: { node: de-01, tls: ["5.6.7.10"], http: ["[::]:80"] }
"#;
    let bundle = Bundle::from_yaml(yaml).expect("test YAML parses");
    Policy::compile(&bundle).expect("test bundle compiles").0
}

fn ip(s: &str) -> IpAddr {
    s.parse().expect("test address parses")
}

#[test]
fn every_built_in_network_parses() {
    for network in SPECIAL_PURPOSE {
        assert!(Cidr::parse(network).is_ok(), "{network}");
    }
}

#[test]
fn refuses_special_purpose_addresses() {
    let egress = Egress::new(&policy());
    for addr in [
        "0.0.0.0",
        "10.1.2.3",
        "100.64.0.1",
        "127.0.0.1",
        "169.254.169.254",
        "172.31.255.255",
        "192.168.1.1",
        "198.18.0.1",
        "224.0.0.251",
        "255.255.255.255",
        "::",
        "::1",
        "fe80::1",
        "fd00:ec2::254",
        "ff02::1",
        "2001:db8::1",
        "::ffff:127.0.0.1",
        "::ffff:10.0.0.1",
        "64:ff9b::a00:1",
        "64:ff9b::7f00:1",
    ] {
        assert!(!egress.allows(ip(addr)), "{addr}");
    }
}

#[test]
fn allows_public_addresses() {
    let egress = Egress::new(&policy());
    for addr in [
        "1.1.1.1",
        "93.184.215.14",
        "2606:4700:4700::1111",
        "::ffff:1.1.1.1",
        "64:ff9b::101:101",
    ] {
        assert!(egress.allows(ip(addr)), "{addr}");
    }
}

#[test]
fn refuses_veyras_own_endpoints() {
    let egress = Egress::new(&policy());
    for own in ["5.6.7.8", "2a01:4f8::8", "9.9.8.8", "5.6.7.9", "5.6.7.10"] {
        assert!(!egress.allows(ip(own)), "{own}");
    }
    assert!(egress.allows(ip("5.6.7.11")));
}

#[test]
fn an_allowed_network_is_carved_out() {
    let mut egress = Egress::new(&policy());
    egress.allow(Cidr::parse("127.0.0.1/32").unwrap());
    assert!(egress.allows(ip("127.0.0.1")));
    assert!(!egress.allows(ip("127.0.0.2")));
}
````

- [ ] **Step 2: Run the tests to see them fail**

Run: `cargo test -p veyra-gateway --test egress`
Expected: compilation fails: ``unresolved imports `veyra_gateway::Egress`, `veyra_gateway::SPECIAL_PURPOSE` ``.

- [ ] **Step 3: Write the implementation**

Create `crates/veyra-gateway/src/egress.rs`:

````rust
//! Where the gateway may connect (spec §10.1, §19.2 item 4): never to loopback,
//! private, link-local, multicast, metadata or other special-purpose networks,
//! nor to Veyra's own endpoints. Every resolved address is checked, so a name
//! that starts resolving to an internal address is refused too (REQ-GW-003).

use std::net::{IpAddr, Ipv4Addr, Ipv6Addr};
use veyra_policy::{Cidr, Policy};

/// Networks that are not destinations on the public internet: the IANA IPv4
/// and IPv6 special-purpose address registries, plus multicast.
pub const SPECIAL_PURPOSE: [&str; 27] = [
    "0.0.0.0/8",       // this network
    "10.0.0.0/8",      // private
    "100.64.0.0/10",   // carrier-grade NAT
    "127.0.0.0/8",     // loopback
    "169.254.0.0/16",  // link-local, cloud metadata (169.254.169.254)
    "172.16.0.0/12",   // private
    "192.0.0.0/24",    // IETF protocol assignments
    "192.0.2.0/24",    // documentation
    "192.88.99.0/24",  // 6to4 relay anycast (deprecated)
    "192.168.0.0/16",  // private
    "198.18.0.0/15",   // benchmarking
    "198.51.100.0/24", // documentation
    "203.0.113.0/24",  // documentation
    "224.0.0.0/4",     // multicast
    "240.0.0.0/4",     // reserved, broadcast
    "::/96",           // unspecified, loopback, IPv4-compatible (deprecated)
    "64:ff9b:1::/48",  // local-use NAT64
    "100::/64",        // discard-only
    "2001::/23",       // IETF protocol assignments (Teredo, benchmarking, ORCHID)
    "2001:db8::/32",   // documentation
    "2002::/16",       // 6to4 (deprecated)
    "3fff::/20",       // documentation
    "5f00::/16",       // segment routing (SRv6) SIDs
    "fc00::/7",        // unique local, cloud metadata (fd00:ec2::254)
    "fe80::/10",       // link-local
    "fec0::/10",       // site-local (deprecated)
    "ff00::/8",        // multicast
];

/// The well-known NAT64 prefix (RFC 6052): judged by the IPv4 address inside.
const NAT64: Ipv6Addr = Ipv6Addr::new(0x64, 0xff9b, 0, 0, 0, 0, 0, 0);

/// The addresses the gateway refuses to dial.
#[derive(Debug, Clone)]
pub struct Egress {
    deny: Vec<Cidr>,
    own: Vec<IpAddr>,
    allow: Vec<Cidr>,
}

impl Egress {
    /// Special-purpose networks, every node's gateway addresses, and the
    /// addresses this data plane listens on.
    pub fn new(policy: &Policy) -> Egress {
        let gateways = policy
            .gateways()
            .values()
            .flat_map(|g| [Some(IpAddr::V4(g.ipv4)), g.ipv6.map(IpAddr::V6)])
            .flatten();
        let dns = policy
            .dns()
            .map(|d| d.listen.as_slice())
            .unwrap_or_default();
        let (tls, http) = policy
            .gateway_config()
            .map(|g| (g.tls.as_slice(), g.http.as_slice()))
            .unwrap_or_default();
        let listeners = dns
            .iter()
            .chain(tls)
            .chain(http)
            .map(|a| a.ip())
            .filter(|ip| !ip.is_unspecified());
        Egress {
            deny: SPECIAL_PURPOSE
                .iter()
                .filter_map(|n| Cidr::parse(n).ok())
                .collect(),
            own: gateways
                .chain(listeners)
                .map(|ip| ip.to_canonical())
                .collect(),
            allow: Vec::new(),
        }
    }

    /// Let the gateway dial `network` after all, such as a test origin on loopback.
    pub fn allow(&mut self, network: Cidr) {
        self.allow.push(network);
    }

    /// May the gateway connect to `ip`?
    pub fn allows(&self, ip: IpAddr) -> bool {
        let refused = |ip: IpAddr| {
            (self.deny.iter().any(|n| n.contains(ip)) || self.own.contains(&ip))
                && !self.allow.iter().any(|n| n.contains(ip))
        };
        let ip = ip.to_canonical();
        !refused(ip) && nat64_v4(ip).is_none_or(|v4| !refused(v4))
    }
}

/// The IPv4 address inside a well-known-prefix NAT64 address (RFC 6052 §2.1).
fn nat64_v4(ip: IpAddr) -> Option<IpAddr> {
    let IpAddr::V6(v6) = ip else {
        return None;
    };
    let bits = u128::from(v6);
    (bits >> 32 == u128::from(NAT64) >> 32).then(|| IpAddr::V4(Ipv4Addr::from(bits as u32)))
}
````

In `crates/veyra-gateway/src/lib.rs`, apply this patch (save it to a file and run `git apply <file>`, or edit by hand):

````diff
diff --git a/crates/veyra-gateway/src/lib.rs b/crates/veyra-gateway/src/lib.rs
index 01b5d92..9dbf2b4 100644
--- a/crates/veyra-gateway/src/lib.rs
+++ b/crates/veyra-gateway/src/lib.rs
@@ -2,6 +2,8 @@
 //! through to the origin without terminating TLS. Decisions come from
 //! `veyra-policy`; this crate owns the sockets and the protocol sniffing.
 
+mod egress;
 mod sniff;
 
+pub use egress::{Egress, SPECIAL_PURPOSE};
 pub use sniff::{MAX_HEAD, Service, Sniff, sniff};
````

- [ ] **Step 4: Run the tests to see them pass**

Run: `cargo test --workspace`
Expected: `egress` `ok. 5 passed`; 131 in total.

- [ ] **Step 5: Format and lint**

Run: `cargo fmt --all && cargo clippy --workspace --all-targets -- -D warnings`
Expected: clippy prints no warnings.

- [ ] **Step 6: Commit**

````bash
git add crates/veyra-gateway/src/egress.rs crates/veyra-gateway/src/lib.rs crates/veyra-gateway/tests/egress.rs
git commit -m "feat(gateway): refuse special-purpose and Veyra's own addresses as origins"
````

---

### Task 5: One connection: admit, route, resolve, check, pass through

**Files:**
- Create: `crates/veyra-gateway/src/gateway.rs`, `crates/veyra-gateway/tests/gateway.rs`
- Modify: `crates/veyra-gateway/Cargo.toml`, `crates/veyra-gateway/src/lib.rs`, `crates/veyra-gateway/tests/support/mod.rs`

**Interfaces:**
- Consumes: `Policy::admit` (plan 2), `Policy::routes_to` and `Policy::gateway_config` (Task 1), `DEFAULT_UPSTREAM_GROUP` and `DnsConfig.upstreams` (plan 2), `veyra_dns::lookup` (Task 2), `sniff`, `Service`, `Sniff` (Task 3), `Egress` (Task 4).
- Produces: `Refusal::{SourceNotAdmitted, NoName, NotRouted, Unresolved, EgressBlocked, Unreachable, Client(io::Error)}` (thiserror; its Display uses the spec §20.4 codes where one fits); `Gateway::new(policy: Policy) -> Result<Gateway, String>` (needs a `gateway:` section); the builders `with_origin_ports(self, tls: u16, http: u16)`, `with_timeouts(self, hello: Duration, upstream: Duration)` and `allowing(self, network: Cidr)`; `Gateway::policy(&self) -> &Policy`; `Gateway::handle(&self, client: TcpStream, service: Service) -> Result<(), Refusal>`.
- Inside `handle`, in order: admission (before reading anything); read until `sniff` names the destination (10 s limit); `routes_to(node, name)`; `lookup` through the `default` upstream group; drop forbidden addresses; dial the rest in order on 443 or 80 (5 s each); write the bytes read so far; `copy_bidirectional`.
- Test helpers added to `tests/support/mod.rs`: `upstream(&[Ipv4Addr]) -> SocketAddr`, `upstream_with(impl Fn(usize) -> Vec<Ipv4Addr>)` (the n-th A query's answer), `origin() -> Origin` (an echo server with `port` and `connections()`), `gateway(upstream, origin_port) -> Gateway`, `connect(Arc<Gateway>, source, Service) -> (TcpStream, JoinHandle<Result<(), Refusal>>)`, `round_trip(&mut TcpStream, &[u8]) -> Vec<u8>`.

- [ ] **Step 1: Write the failing tests**

In `crates/veyra-gateway/Cargo.toml`, apply this patch (save it to a file and run `git apply <file>`, or edit by hand):

````diff
diff --git a/crates/veyra-gateway/Cargo.toml b/crates/veyra-gateway/Cargo.toml
index d7bfb76..d3e7ea3 100644
--- a/crates/veyra-gateway/Cargo.toml
+++ b/crates/veyra-gateway/Cargo.toml
@@ -7,6 +7,12 @@ publish.workspace = true
 
 [dependencies]
 veyra-policy.workspace = true
+veyra-dns.workspace = true
+thiserror.workspace = true
+tokio.workspace = true
+
+[dev-dependencies]
+hickory-proto.workspace = true
 
 [lints]
 workspace = true
````

Create `crates/veyra-gateway/tests/gateway.rs`:

````rust
//! One connection through the gateway (spec §6.2, §10; REQ-GW-001, 003, 004).

mod support;

use std::io;
use std::net::Ipv4Addr;
use std::sync::Arc;
use std::time::{Duration, Instant};
use support::{client_hello, connect, gateway, origin, round_trip, upstream, upstream_with};
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::TcpListener;
use tokio::time::sleep;
use veyra_gateway::{Gateway, Refusal, Service};

const LOOPBACK: Ipv4Addr = Ipv4Addr::LOCALHOST;
const REQUEST: &[u8] = b"GET / HTTP/1.1\r\nHost: media.example\r\n\r\n";

/// Send `head` from the admitted source, close, and return the verdict.
async fn verdict(gateway: &Arc<Gateway>, service: Service, head: &[u8]) -> Result<(), Refusal> {
    let (mut client, verdict) = connect(gateway.clone(), "127.0.0.1", service).await;
    let _ = client.write_all(head).await;
    let _ = client.shutdown().await;
    verdict.await.expect("gateway task")
}

#[tokio::test]
async fn tls_passes_through_byte_for_byte() {
    let origin = origin().await;
    let gateway = Arc::new(gateway(upstream(&[LOOPBACK]).await, origin.port));
    let (mut client, verdict) = connect(gateway, "127.0.0.1", Service::Tls).await;
    let mut bytes = client_hello(Some("www.media.example"), &[]);
    bytes.extend(b"encrypted application data");
    assert_eq!(round_trip(&mut client, &bytes).await, bytes);
    drop(client);
    verdict.await.unwrap().unwrap();
    assert_eq!(origin.connections(), 1);
}

#[tokio::test]
async fn http_is_routed_by_host_and_passed_unchanged() {
    let origin = origin().await;
    let gateway = Arc::new(gateway(upstream(&[LOOPBACK]).await, origin.port));
    let (mut client, _) = connect(gateway, "127.0.0.1", Service::Http).await;
    assert_eq!(round_trip(&mut client, REQUEST).await, REQUEST);
}

#[tokio::test]
async fn a_client_hello_in_pieces_still_routes() {
    let origin = origin().await;
    let gateway = Arc::new(gateway(upstream(&[LOOPBACK]).await, origin.port));
    let (mut client, _) = connect(gateway, "127.0.0.1", Service::Tls).await;
    client.set_nodelay(true).unwrap();
    let hello = client_hello(Some("media.example"), &[]);
    for piece in [&hello[..3], &hello[3..40], &hello[40..]] {
        client.write_all(piece).await.unwrap();
        sleep(Duration::from_millis(20)).await;
    }
    let mut back = vec![0; hello.len()];
    client.read_exact(&mut back).await.unwrap();
    assert_eq!(back, hello);
}

#[tokio::test]
async fn strangers_are_turned_away_before_anything_is_read() {
    let origin = origin().await;
    let gateway = Arc::new(gateway(upstream(&[LOOPBACK]).await, origin.port));
    let (mut client, verdict) = connect(gateway, "127.0.0.3", Service::Tls).await;
    assert!(matches!(
        verdict.await.unwrap(),
        Err(Refusal::SourceNotAdmitted)
    ));
    let mut byte = [0u8; 1];
    assert!(matches!(client.read(&mut byte).await, Ok(0) | Err(_)));
    assert_eq!(origin.connections(), 0);
}

#[tokio::test]
async fn names_not_routed_here_are_refused() {
    let origin = origin().await;
    let gateway = Arc::new(gateway(upstream(&[LOOPBACK]).await, origin.port));
    for name in [
        "login.media.example", // home sends it direct, strict blocks it
        "us.example",          // routed to another node
        "tunnel.example",      // not allowed in mode B
        "ads.example",         // blocked
        "example.org",         // no rule
    ] {
        let result = verdict(&gateway, Service::Tls, &client_hello(Some(name), &[])).await;
        assert!(
            matches!(result, Err(Refusal::NotRouted)),
            "{name}: {result:?}"
        );
    }
    assert_eq!(origin.connections(), 0);
}

#[tokio::test]
async fn names_routed_here_for_another_profile_pass() {
    // 127.0.0.1 is admitted as home, but only strict routes video.example to
    // de-01. The gateway cannot tell profiles apart, so it serves both (spec §10.2).
    let origin = origin().await;
    let gateway = Arc::new(gateway(upstream(&[LOOPBACK]).await, origin.port));
    let (mut client, _) = connect(gateway, "127.0.0.1", Service::Tls).await;
    let hello = client_hello(Some("video.example"), &[]);
    assert_eq!(round_trip(&mut client, &hello).await, hello);
}

#[tokio::test]
async fn connections_without_a_usable_name_are_refused() {
    let origin = origin().await;
    let gateway = Arc::new(gateway(upstream(&[LOOPBACK]).await, origin.port));
    let hello = client_hello(Some("media.example"), &[]);
    for (why, service, head) in [
        ("no SNI", Service::Tls, client_hello(None, &[])),
        (
            "an IP literal",
            Service::Tls,
            client_hello(Some("10.0.0.1"), &[]),
        ),
        ("not TLS", Service::Tls, b"\x00\x01garbage".to_vec()),
        ("closed mid-hello", Service::Tls, hello[..20].to_vec()),
        ("no Host", Service::Http, b"GET / HTTP/1.0\r\n\r\n".to_vec()),
    ] {
        let result = verdict(&gateway, service, &head).await;
        assert!(matches!(result, Err(Refusal::NoName)), "{why}: {result:?}");
    }
    assert_eq!(origin.connections(), 0);
}

#[tokio::test]
async fn private_and_own_addresses_are_refused_after_resolving() {
    let origin = origin().await;
    for ip in [
        Ipv4Addr::new(10, 0, 0, 1),
        Ipv4Addr::new(169, 254, 169, 254),
        Ipv4Addr::new(203, 0, 113, 10), // de-01's own gateway address
    ] {
        let gateway = Arc::new(gateway(upstream(&[ip]).await, origin.port));
        let result = verdict(&gateway, Service::Http, REQUEST).await;
        assert!(
            matches!(result, Err(Refusal::EgressBlocked)),
            "{ip}: {result:?}"
        );
    }
    assert_eq!(origin.connections(), 0);
}

#[tokio::test]
async fn every_connection_checks_what_the_name_resolves_to_now() {
    // DNS rebinding: first the (allowed) test origin, then a private address.
    let origin = origin().await;
    let rebinding = upstream_with(|n| match n {
        0 => vec![LOOPBACK],
        _ => vec![Ipv4Addr::new(10, 0, 0, 1)],
    })
    .await;
    let gateway = Arc::new(gateway(rebinding, origin.port));
    let (mut first, _) = connect(gateway.clone(), "127.0.0.1", Service::Http).await;
    assert_eq!(round_trip(&mut first, REQUEST).await, REQUEST);
    let result = verdict(&gateway, Service::Http, REQUEST).await;
    assert!(matches!(result, Err(Refusal::EgressBlocked)), "{result:?}");
    assert_eq!(origin.connections(), 1);
}

#[tokio::test]
async fn forbidden_addresses_are_skipped_for_allowed_ones() {
    let origin = origin().await;
    let mixed = upstream(&[Ipv4Addr::new(10, 0, 0, 1), LOOPBACK]).await;
    let gateway = Arc::new(gateway(mixed, origin.port));
    let (mut client, _) = connect(gateway, "127.0.0.1", Service::Http).await;
    assert_eq!(round_trip(&mut client, REQUEST).await, REQUEST);
}

#[tokio::test]
async fn a_silent_client_is_dropped_after_the_hello_timeout() {
    let origin = origin().await;
    let gateway = Arc::new(gateway(upstream(&[LOOPBACK]).await, origin.port));
    let started = Instant::now();
    let (_client, verdict) = connect(gateway, "127.0.0.1", Service::Tls).await;
    let result = verdict.await.unwrap();
    assert!(
        matches!(&result, Err(Refusal::Client(e)) if e.kind() == io::ErrorKind::TimedOut),
        "{result:?}"
    );
    assert!(started.elapsed() < Duration::from_secs(2));
}

#[tokio::test]
async fn unresolvable_and_unreachable_origins_are_refused() {
    let origin = origin().await;
    let nothing = Arc::new(gateway(upstream(&[]).await, origin.port));
    let result = verdict(&nothing, Service::Http, REQUEST).await;
    assert!(matches!(result, Err(Refusal::Unresolved)), "{result:?}");

    let closed = TcpListener::bind("127.0.0.1:0").await.unwrap();
    let closed_port = closed.local_addr().unwrap().port();
    drop(closed);
    let dead = Arc::new(gateway(upstream(&[LOOPBACK]).await, closed_port));
    let result = verdict(&dead, Service::Http, REQUEST).await;
    assert!(matches!(result, Err(Refusal::Unreachable)), "{result:?}");
}
````

In `crates/veyra-gateway/tests/support/mod.rs`, apply this patch (save it to a file and run `git apply <file>`, or edit by hand):

````diff
diff --git a/crates/veyra-gateway/tests/support/mod.rs b/crates/veyra-gateway/tests/support/mod.rs
index 423e88f..273af20 100644
--- a/crates/veyra-gateway/tests/support/mod.rs
+++ b/crates/veyra-gateway/tests/support/mod.rs
@@ -1,6 +1,167 @@
-//! Shared test helpers.
+//! Shared test helpers: ClientHello bytes, a fake upstream DNS server, an echo
+//! origin, and a gateway over a small policy.
 #![allow(dead_code)] // each test file uses a different subset
 
+use hickory_proto::op::{Message, OpCode};
+use hickory_proto::rr::rdata::A;
+use hickory_proto::rr::{RData, Record, RecordType};
+use std::net::{Ipv4Addr, SocketAddr};
+use std::sync::Arc;
+use std::sync::atomic::{AtomicUsize, Ordering};
+use std::time::Duration;
+use tokio::io::{AsyncReadExt, AsyncWriteExt};
+use tokio::net::{TcpListener, TcpSocket, TcpStream, UdpSocket};
+use tokio::task::JoinHandle;
+use tokio::time::timeout;
+use veyra_gateway::{Gateway, Refusal, Service};
+use veyra_policy::{Bundle, Cidr, Policy};
+
+/// A UDP DNS server on 127.0.0.1 that answers every A query with `ips` and
+/// every other query with no records.
+pub async fn upstream(ips: &[Ipv4Addr]) -> SocketAddr {
+    let ips = ips.to_vec();
+    upstream_with(move |_| ips.clone()).await
+}
+
+/// A UDP DNS server on 127.0.0.1 whose answer to the n-th A query (from 0)
+/// is `answer(n)`; other queries get no records.
+pub async fn upstream_with(answer: impl Fn(usize) -> Vec<Ipv4Addr> + Send + 'static) -> SocketAddr {
+    let socket = UdpSocket::bind("127.0.0.1:0")
+        .await
+        .expect("bind fake upstream");
+    let addr = socket.local_addr().expect("fake upstream address");
+    tokio::spawn(async move {
+        let mut buf = vec![0; 4096];
+        let mut asked = 0;
+        loop {
+            let Ok((len, from)) = socket.recv_from(&mut buf).await else {
+                continue;
+            };
+            let Ok(query) = Message::from_vec(&buf[..len]) else {
+                continue;
+            };
+            let Some(question) = query.queries.first().cloned() else {
+                continue;
+            };
+            let mut reply = Message::response(query.metadata.id, OpCode::Query);
+            reply.add_query(question.clone());
+            if question.query_type() == RecordType::A {
+                for ip in answer(asked) {
+                    let name = question.name().clone();
+                    reply.add_answer(Record::from_rdata(name, 60, RData::A(A(ip))));
+                }
+                asked += 1;
+            }
+            let bytes = reply.to_vec().expect("encode fake reply");
+            let _ = socket.send_to(&bytes, from).await;
+        }
+    });
+    addr
+}
+
+/// A TCP server on 127.0.0.1 that echoes every byte back.
+pub struct Origin {
+    pub port: u16,
+    connections: Arc<AtomicUsize>,
+}
+
+impl Origin {
+    /// Connections accepted so far.
+    pub fn connections(&self) -> usize {
+        self.connections.load(Ordering::SeqCst)
+    }
+}
+
+pub async fn origin() -> Origin {
+    let listener = TcpListener::bind("127.0.0.1:0").await.expect("bind origin");
+    let port = listener.local_addr().expect("origin address").port();
+    let connections = Arc::new(AtomicUsize::new(0));
+    let counter = connections.clone();
+    tokio::spawn(async move {
+        loop {
+            let Ok((mut stream, _)) = listener.accept().await else {
+                continue;
+            };
+            counter.fetch_add(1, Ordering::SeqCst);
+            tokio::spawn(async move {
+                let (mut read, mut write) = stream.split();
+                let _ = tokio::io::copy(&mut read, &mut write).await;
+            });
+        }
+    });
+    Origin { port, connections }
+}
+
+/// A gateway for node de-01 (203.0.113.10) whose origins resolve through
+/// `upstream` and listen on `origin_port`. 127.0.0.1 is admitted as `home` and
+/// may be dialed despite the egress rules. Rules:
+/// - home: `suffix:media.example` via de-01, `login.media.example` direct;
+/// - strict (default block): `suffix:video.example` via de-01;
+/// - everyone: `suffix:us.example` via us-01, `tunnel.example` via de-01 in
+///   tunnel mode only, `suffix:ads.example` blocked.
+pub fn gateway(upstream: SocketAddr, origin_port: u16) -> Gateway {
+    let yaml = format!(
+        r#"
+apiVersion: veyra.io/v1alpha1
+kind: PolicyBundle
+metadata: {{ name: gateway }}
+spec:
+  nodes:
+    - {{ id: de-01, gateway: {{ ipv4: 203.0.113.10 }} }}
+    - {{ id: us-01, gateway: {{ ipv4: 198.51.100.20 }} }}
+  profiles: [{{ id: home }}, {{ id: strict, defaultAction: block }}]
+  admission: [{{ cidr: 127.0.0.1/32, profile: home }}]
+  rules:
+    - {{ id: media, scope: {{ profile: home }}, match: {{ domains: ["suffix:media.example"] }}, action: {{ type: proxy_node, node: de-01 }} }}
+    - {{ id: login, scope: {{ profile: home }}, match: {{ domains: [login.media.example] }}, action: {{ type: direct }} }}
+    - {{ id: video, scope: {{ profile: strict }}, match: {{ domains: ["suffix:video.example"] }}, action: {{ type: proxy_node, node: de-01 }} }}
+    - {{ id: us, scope: default, match: {{ domains: ["suffix:us.example"] }}, action: {{ type: proxy_node, node: us-01 }} }}
+    - {{ id: tunnel-only, scope: default, match: {{ domains: [tunnel.example] }}, action: {{ type: proxy_node, node: de-01 }}, modesAllowed: [tunnel] }}
+    - {{ id: ads, scope: default, match: {{ domains: ["suffix:ads.example"] }}, action: {{ type: block }} }}
+  dns:
+    listen: ["127.0.0.1:0"]
+    upstreams: [{{ id: default, servers: ["{upstream}"] }}]
+  gateway: {{ node: de-01, tls: ["127.0.0.1:0"], http: ["127.0.0.1:0"] }}
+"#
+    );
+    let bundle = Bundle::from_yaml(&yaml).expect("test YAML parses");
+    let (policy, _warnings) = Policy::compile(&bundle).expect("test bundle compiles");
+    Gateway::new(policy)
+        .expect("policy has a gateway section")
+        .with_origin_ports(origin_port, origin_port)
+        .with_timeouts(Duration::from_millis(500), Duration::from_millis(300))
+        .allowing(Cidr::parse("127.0.0.1/32").expect("network"))
+}
+
+/// Connect from `source` to a fresh listener and hand the accepted end to the
+/// gateway: the client end, and the gateway's verdict when it is done.
+pub async fn connect(
+    gateway: Arc<Gateway>,
+    source: &str,
+    service: Service,
+) -> (TcpStream, JoinHandle<Result<(), Refusal>>) {
+    let listener = TcpListener::bind("127.0.0.1:0").await.expect("bind");
+    let socket = TcpSocket::new_v4().expect("socket");
+    let source = format!("{source}:0").parse().expect("source address");
+    socket.bind(source).expect("bind client");
+    let addr = listener.local_addr().expect("listener address");
+    let client = socket.connect(addr).await.expect("connect");
+    let (accepted, _) = listener.accept().await.expect("accept");
+    let verdict = tokio::spawn(async move { gateway.handle(accepted, service).await });
+    (client, verdict)
+}
+
+/// Send `bytes` and read as many back: what the echo origin returned.
+pub async fn round_trip(client: &mut TcpStream, bytes: &[u8]) -> Vec<u8> {
+    client.write_all(bytes).await.expect("write");
+    let mut back = vec![0; bytes.len()];
+    timeout(Duration::from_secs(5), client.read_exact(&mut back))
+        .await
+        .expect("the echo arrives in time")
+        .expect("read the echo");
+    back
+}
+
 /// A ClientHello record naming `sni` (none for `None`), then the `extra`
 /// extensions as (type, body).
 pub fn client_hello(sni: Option<&str>, extra: &[(u16, Vec<u8>)]) -> Vec<u8> {
````

- [ ] **Step 2: Run the tests to see them fail**

Run: `cargo test -p veyra-gateway`
Expected: compilation fails: ``unresolved imports `veyra_gateway::Gateway`, `veyra_gateway::Refusal` `` in `tests/support/mod.rs` and `tests/gateway.rs`. Every test file that includes `support` stops compiling, the earlier ones too.

- [ ] **Step 3: Write the implementation**

Create `crates/veyra-gateway/src/gateway.rs`:

````rust
//! One client connection (spec §6.2, §10.1): admit the source, read the
//! destination, check its route, resolve it, check every address, connect,
//! then pass bytes both ways untouched.

use crate::egress::Egress;
use crate::sniff::{Service, Sniff, sniff};
use std::io;
use std::net::{IpAddr, SocketAddr};
use std::time::Duration;
use tokio::io::{AsyncReadExt, AsyncWriteExt, copy_bidirectional};
use tokio::net::TcpStream;
use tokio::time::timeout;
use veyra_dns::lookup;
use veyra_policy::{Cidr, DEFAULT_UPSTREAM_GROUP, Name, Policy};

/// How long a client may take to name its destination.
const HELLO_TIMEOUT: Duration = Duration::from_secs(10);
/// How long each upstream resolver may take to answer.
const UPSTREAM_TIMEOUT: Duration = Duration::from_secs(2);
/// How long each origin address may take to accept.
const CONNECT_TIMEOUT: Duration = Duration::from_secs(5);

/// Why a client was turned away before reaching an origin. The codes of spec
/// §20.4 where one fits.
#[derive(Debug, thiserror::Error)]
pub enum Refusal {
    /// The source is not admitted (spec §9.6, REQ-GW-004).
    #[error("SOURCE_NOT_ADMITTED")]
    SourceNotAdmitted,
    /// No SNI or `Host`, or one that is not a domain name.
    #[error("no destination name")]
    NoName,
    /// DNS never sends clients here for this name in mode B (spec §10.2).
    #[error("not routed to this node")]
    NotRouted,
    #[error("origin does not resolve")]
    Unresolved,
    /// Every address the name resolved to is forbidden (spec §19.2 item 4).
    #[error("EGRESS_BLOCKED")]
    EgressBlocked,
    #[error("origin unreachable")]
    Unreachable,
    /// The client was too slow to name its destination, or the socket failed.
    #[error("client: {0}")]
    Client(#[from] io::Error),
}

/// The gateway of one node: which names it passes and how it reaches origins.
pub struct Gateway {
    policy: Policy,
    node: String,
    resolvers: Vec<SocketAddr>,
    egress: Egress,
    tls_port: u16,
    http_port: u16,
    hello_timeout: Duration,
    upstream_timeout: Duration,
}

impl Gateway {
    /// Fails when the policy has no `gateway:` section.
    pub fn new(policy: Policy) -> Result<Gateway, String> {
        let node = policy
            .gateway_config()
            .ok_or("the config has no gateway section")?
            .node
            .clone();
        // Origins resolve through the default upstream group, never through
        // Veyra's own DNS, which would answer with a gateway address (spec §10.3).
        let resolvers = policy
            .dns()
            .and_then(|dns| dns.upstreams.get(DEFAULT_UPSTREAM_GROUP))
            .cloned()
            .unwrap_or_default();
        let egress = Egress::new(&policy);
        Ok(Gateway {
            policy,
            node,
            resolvers,
            egress,
            tls_port: 443,
            http_port: 80,
            hello_timeout: HELLO_TIMEOUT,
            upstream_timeout: UPSTREAM_TIMEOUT,
        })
    }

    /// Dial origins on other ports than 443 and 80, such as test origins.
    pub fn with_origin_ports(mut self, tls: u16, http: u16) -> Gateway {
        self.tls_port = tls;
        self.http_port = http;
        self
    }

    /// How long a client may take to name its destination, and how long each
    /// upstream resolver may take to answer.
    pub fn with_timeouts(mut self, hello: Duration, upstream: Duration) -> Gateway {
        self.hello_timeout = hello;
        self.upstream_timeout = upstream;
        self
    }

    /// Let the gateway dial `network` despite the egress rules, such as a test
    /// origin on loopback.
    pub fn allowing(mut self, network: Cidr) -> Gateway {
        self.egress.allow(network);
        self
    }

    pub fn policy(&self) -> &Policy {
        &self.policy
    }

    /// Serve one connection to the end. `Ok` once bytes flowed to an origin
    /// and both sides closed; `Err` says why the client was turned away.
    pub async fn handle(&self, mut client: TcpStream, service: Service) -> Result<(), Refusal> {
        // Not admitted: close before reading a byte (spec §9.6).
        if self.policy.admit(client.peer_addr()?.ip()).is_none() {
            return Err(Refusal::SourceNotAdmitted);
        }
        let (head, name) = timeout(self.hello_timeout, read_head(&mut client, service))
            .await
            .map_err(|_| io::Error::from(io::ErrorKind::TimedOut))??;
        if !self.policy.routes_to(&self.node, &name) {
            return Err(Refusal::NotRouted);
        }
        let addrs = lookup(&self.resolvers, name.as_str(), self.upstream_timeout).await;
        if addrs.is_empty() {
            return Err(Refusal::Unresolved);
        }
        let allowed: Vec<IpAddr> = addrs
            .into_iter()
            .filter(|ip| self.egress.allows(*ip))
            .collect();
        if allowed.is_empty() {
            return Err(Refusal::EgressBlocked);
        }
        let port = match service {
            Service::Tls => self.tls_port,
            Service::Http => self.http_port,
        };
        let mut origin = connect(&allowed, port).await.ok_or(Refusal::Unreachable)?;
        origin
            .write_all(&head)
            .await
            .map_err(|_| Refusal::Unreachable)?;
        let _ = client.set_nodelay(true);
        let _ = origin.set_nodelay(true);
        // A close or reset on either side ends the passthrough; it is not a refusal.
        // Later requests on a kept-alive HTTP connection are not read: they go to
        // the origin address the first `Host` chose.
        // ponytail: no idle timeout once bytes flow; add one if stale sessions
        // fill the session cap.
        let _ = copy_bidirectional(&mut client, &mut origin).await;
        Ok(())
    }
}

/// Read until the ClientHello or request head names the destination. Returns
/// every byte read, to be sent to the origin unchanged.
async fn read_head(client: &mut TcpStream, service: Service) -> Result<(Vec<u8>, Name), Refusal> {
    let mut head = Vec::new();
    let mut buf = [0u8; 4096];
    loop {
        let len = client.read(&mut buf).await?;
        if len == 0 {
            return Err(Refusal::NoName);
        }
        head.extend_from_slice(&buf[..len]);
        match sniff(service, &head) {
            Sniff::More => {}
            Sniff::Name(name) => return Ok((head, name)),
            Sniff::Refuse => return Err(Refusal::NoName),
        }
    }
}

/// The first address that accepts, in resolver order (IPv4 first).
// ponytail: one address after another; Happy Eyeballs (RFC 8305) if
// dual-stack origins with a dead family stall connections.
async fn connect(addrs: &[IpAddr], port: u16) -> Option<TcpStream> {
    for ip in addrs {
        if let Ok(Ok(stream)) = timeout(CONNECT_TIMEOUT, TcpStream::connect((*ip, port))).await {
            return Some(stream);
        }
    }
    None
}
````

In `crates/veyra-gateway/src/lib.rs`, apply this patch (save it to a file and run `git apply <file>`, or edit by hand):

````diff
diff --git a/crates/veyra-gateway/src/lib.rs b/crates/veyra-gateway/src/lib.rs
index 9dbf2b4..27b1112 100644
--- a/crates/veyra-gateway/src/lib.rs
+++ b/crates/veyra-gateway/src/lib.rs
@@ -3,7 +3,9 @@
 //! `veyra-policy`; this crate owns the sockets and the protocol sniffing.
 
 mod egress;
+mod gateway;
 mod sniff;
 
 pub use egress::{Egress, SPECIAL_PURPOSE};
+pub use gateway::{Gateway, Refusal};
 pub use sniff::{MAX_HEAD, Service, Sniff, sniff};
````

- [ ] **Step 4: Run the tests to see them pass**

Run: `cargo test --workspace`
Expected: `gateway` (in veyra-gateway) `ok. 12 passed` in about 2 s; one test dials a closed port, and Windows takes 2 s to refuse it. 143 in total.

- [ ] **Step 5: Format and lint**

Run: `cargo fmt --all && cargo clippy --workspace --all-targets -- -D warnings`
Expected: clippy prints no warnings.

- [ ] **Step 6: Commit**

````bash
git add Cargo.lock crates/veyra-gateway/Cargo.toml crates/veyra-gateway/src/gateway.rs crates/veyra-gateway/src/lib.rs crates/veyra-gateway/tests/gateway.rs crates/veyra-gateway/tests/support/mod.rs
git commit -m "feat(gateway): pass admitted, routed connections through to the origin"
````

---

### Task 6: Listeners and the gateway in `veyra-dataplane`

**Files:**
- Create: `crates/veyra-gateway/src/server.rs`, `crates/veyra-gateway/tests/server.rs`
- Modify: `Cargo.toml`, `README.md`, `apps/veyra-dataplane/Cargo.toml`, `apps/veyra-dataplane/src/main.rs`, `apps/veyra-dataplane/tests/dataplane.rs`, `configs/examples/home.yaml`, `crates/veyra-gateway/Cargo.toml`, `crates/veyra-gateway/src/lib.rs`

**Interfaces:**
- Consumes: `Gateway`, `Gateway::handle`, `Service` (Task 5), `GatewayConfig` (Task 1).
- Produces: `veyra_gateway::Listeners::bind(config: &GatewayConfig) -> io::Result<Listeners>` (TCP only); `with_max_sessions(self, n: usize)` (default 1024; past the cap, a new connection is closed at once); `local_addrs(&self) -> io::Result<Vec<(SocketAddr, Service)>>` (TLS first, then HTTP); `serve(self, gateway: Arc<Gateway>)`. Refusals are logged at debug with the reason only.
- `veyra-dataplane` starts the gateway when the config has `gateway:` and prints `listening on <addr> (tls)` and `(http)` lines after the DNS ones. The end-to-end helper `start(name, config, lines)` gives each test its own config file and reads stdout on a thread, allowing 10 s per line.
- The example config gains a `gateway:` section on 127.0.0.1:10443 and :10080; the README gains "Run the gateway".

- [ ] **Step 1: Write the failing tests**

In `apps/veyra-dataplane/tests/dataplane.rs`, apply this patch (save it to a file and run `git apply <file>`, or edit by hand):

````diff
diff --git a/apps/veyra-dataplane/tests/dataplane.rs b/apps/veyra-dataplane/tests/dataplane.rs
index fe8636f..e1aeb4e 100644
--- a/apps/veyra-dataplane/tests/dataplane.rs
+++ b/apps/veyra-dataplane/tests/dataplane.rs
@@ -2,10 +2,12 @@
 
 use hickory_proto::op::{Message, MessageType, OpCode, Query, ResponseCode};
 use hickory_proto::rr::{Name, RData, RecordType};
-use std::io::{BufRead, BufReader, Read, Write};
+use std::io::{BufRead, BufReader, ErrorKind, Read, Write};
 use std::net::{Ipv4Addr, SocketAddr, TcpStream, UdpSocket};
 use std::path::PathBuf;
 use std::process::{Child, Command, Stdio};
+use std::sync::mpsc;
+use std::thread;
 use std::time::Duration;
 
 const CONFIG: &str = r#"
@@ -40,9 +42,10 @@ fn write_config(name: &str, text: &str) -> PathBuf {
     path
 }
 
-/// Start the binary and wait for its "listening on <addr>" line.
-fn start(config: &str) -> (Server, SocketAddr) {
-    let path = write_config("serve", config);
+/// Start the binary on a config file of its own (tests run in parallel) and
+/// read its first `lines` "listening on <addr> (...)" lines.
+fn start(name: &str, config: &str, lines: usize) -> (Server, Vec<SocketAddr>) {
+    let path = write_config(name, config);
     let mut child = Command::new(env!("CARGO_BIN_EXE_veyra-dataplane"))
         .args(["--config".as_ref(), path.as_os_str()])
         .stdout(Stdio::piped())
@@ -51,16 +54,28 @@ fn start(config: &str) -> (Server, SocketAddr) {
         .expect("start veyra-dataplane");
     let stdout = child.stdout.take().expect("stdout");
     let server = Server(child);
-    let mut line = String::new();
-    BufReader::new(stdout)
-        .read_line(&mut line)
-        .expect("read startup line");
-    let addr = line
-        .split_whitespace()
-        .nth(2)
-        .and_then(|a| a.parse().ok())
-        .unwrap_or_else(|| panic!("unexpected startup line {line:?}"));
-    (server, addr)
+    // Read on a thread: a binary that prints fewer lines fails the test instead
+    // of hanging it, and later output never meets a closed pipe.
+    let (tx, rx) = mpsc::channel();
+    thread::spawn(move || {
+        for line in BufReader::new(stdout).lines().map_while(Result::ok) {
+            if tx.send(line).is_err() {
+                break;
+            }
+        }
+    });
+    let addrs = (0..lines)
+        .map(|_| {
+            let line = rx
+                .recv_timeout(Duration::from_secs(10))
+                .expect("a listening line within 10 s");
+            line.split_whitespace()
+                .nth(2)
+                .and_then(|a| a.parse().ok())
+                .unwrap_or_else(|| panic!("unexpected startup line {line:?}"))
+        })
+        .collect();
+    (server, addrs)
 }
 
 fn query(name: &str, record_type: RecordType) -> Vec<u8> {
@@ -75,7 +90,8 @@ fn query(name: &str, record_type: RecordType) -> Vec<u8> {
 
 #[test]
 fn serves_synthesized_answers_over_udp_and_tcp() {
-    let (_server, addr) = start(CONFIG);
+    let (_server, addrs) = start("dns", CONFIG, 1);
+    let addr = addrs[0];
 
     let udp = UdpSocket::bind("127.0.0.1:0").expect("bind client");
     udp.set_read_timeout(Some(Duration::from_secs(5)))
@@ -118,3 +134,49 @@ fn refuses_a_config_without_a_dns_section() {
     let err = String::from_utf8_lossy(&out.stderr);
     assert!(err.contains("no dns section"), "{err}");
 }
+
+/// True when the peer closed the connection, false when the read timed out.
+fn closed_by_peer(stream: &mut TcpStream) -> bool {
+    match stream.read(&mut [0u8; 1]) {
+        Ok(0) => true,
+        Ok(_) => false,
+        Err(e) => !matches!(e.kind(), ErrorKind::WouldBlock | ErrorKind::TimedOut),
+    }
+}
+
+#[test]
+fn the_gateway_refuses_unrouted_names_and_rejects_udp() {
+    let gateway = "  gateway: { node: de-01, tls: [\"127.0.0.1:0\"], http: [\"127.0.0.1:0\"] }\n";
+    let (_server, addrs) = start("gateway", &format!("{CONFIG}{gateway}"), 3);
+    let (tls, http) = (addrs[1], addrs[2]);
+
+    for (addr, head) in [
+        (tls, &b"not a ClientHello"[..]),
+        (http, &b"GET / HTTP/1.1\r\nHost: example.org\r\n\r\n"[..]),
+    ] {
+        let mut client = TcpStream::connect(addr).expect("connect");
+        client
+            .set_read_timeout(Some(Duration::from_secs(5)))
+            .expect("timeout");
+        client.write_all(head).expect("write");
+        assert!(closed_by_peer(&mut client), "{addr}");
+    }
+
+    // UDP to the TLS port is rejected (ICMP port unreachable), not dropped, so
+    // HTTP/3 clients fall back to TCP at once (spec §10.1, REQ-GW-002).
+    let udp = UdpSocket::bind("127.0.0.1:0").expect("bind client");
+    udp.connect(tls).expect("connect");
+    udp.set_read_timeout(Some(Duration::from_secs(1)))
+        .expect("timeout");
+    udp.send(b"QUIC initial").expect("send");
+    let err = udp
+        .recv(&mut [0u8; 64])
+        .expect_err("nothing serves UDP on the gateway port");
+    assert!(
+        matches!(
+            err.kind(),
+            ErrorKind::ConnectionRefused | ErrorKind::ConnectionReset
+        ),
+        "{err:?}"
+    );
+}
````

Create `crates/veyra-gateway/tests/server.rs`:

````rust
//! The gateway's listeners (spec §10.1).

mod support;

use std::net::Ipv4Addr;
use std::sync::Arc;
use std::time::Duration;
use support::{client_hello, gateway, origin, round_trip, upstream};
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::TcpStream;
use tokio::time::{sleep, timeout};
use veyra_gateway::{Listeners, Service};

const REQUEST: &[u8] = b"GET / HTTP/1.1\r\nHost: media.example\r\n\r\n";

#[tokio::test]
async fn serves_tls_and_http_listeners() {
    let origin = origin().await;
    let gateway = gateway(upstream(&[Ipv4Addr::LOCALHOST]).await, origin.port);
    let config = gateway.policy().gateway_config().expect("gateway section");
    let listeners = Listeners::bind(config).await.expect("bind");
    let addrs = listeners.local_addrs().expect("bound addresses");
    tokio::spawn(listeners.serve(Arc::new(gateway)));
    let services: Vec<Service> = addrs.iter().map(|(_, s)| *s).collect();
    assert_eq!(services, [Service::Tls, Service::Http]);

    let hello = client_hello(Some("media.example"), &[]);
    let mut tls = TcpStream::connect(addrs[0].0).await.unwrap();
    assert_eq!(round_trip(&mut tls, &hello).await, hello);
    let mut http = TcpStream::connect(addrs[1].0).await.unwrap();
    assert_eq!(round_trip(&mut http, REQUEST).await, REQUEST);
}

#[tokio::test]
async fn connections_beyond_the_cap_are_closed_at_once() {
    let origin = origin().await;
    let gateway = gateway(upstream(&[Ipv4Addr::LOCALHOST]).await, origin.port)
        .with_timeouts(Duration::from_secs(5), Duration::from_millis(300));
    let config = gateway.policy().gateway_config().expect("gateway section");
    let listeners = Listeners::bind(config)
        .await
        .expect("bind")
        .with_max_sessions(1);
    let tls = listeners.local_addrs().expect("bound addresses")[0].0;
    tokio::spawn(listeners.serve(Arc::new(gateway)));

    // Holds the only session while the gateway waits for its ClientHello.
    let _silent = TcpStream::connect(tls).await.unwrap();
    sleep(Duration::from_millis(100)).await;
    let mut second = TcpStream::connect(tls).await.unwrap();
    let _ = second
        .write_all(&client_hello(Some("media.example"), &[]))
        .await;
    let mut byte = [0u8; 1];
    let read = timeout(Duration::from_secs(1), second.read(&mut byte))
        .await
        .expect("closed at once, not after the hello timeout");
    assert!(matches!(read, Ok(0) | Err(_)), "{read:?}");
    assert_eq!(origin.connections(), 0);
}
````

- [ ] **Step 2: Run the tests to see them fail**

Run: `cargo test -p veyra-gateway --test server`
Expected: compilation fails: ``unresolved import `veyra_gateway::Listeners` ``.

Run: `cargo test -p veyra-dataplane`
Expected: after 10 s, `the_gateway_refuses_unrouted_names_and_rejects_udp ... FAILED` with `a listening line within 10 s: Timeout` (the binary still serves DNS only); `test result: FAILED. 2 passed; 1 failed`.

- [ ] **Step 3: Write the implementation**

In `Cargo.toml`, apply this patch (save it to a file and run `git apply <file>`, or edit by hand):

````diff
diff --git a/Cargo.toml b/Cargo.toml
index 594c626..948744c 100644
--- a/Cargo.toml
+++ b/Cargo.toml
@@ -11,6 +11,7 @@ publish = false
 [workspace.dependencies]
 veyra-policy = { path = "crates/veyra-policy" }
 veyra-dns = { path = "crates/veyra-dns" }
+veyra-gateway = { path = "crates/veyra-gateway" }
 serde = { version = "1.0.229", features = ["derive"] }
 serde-saphyr = "1.3.0"
 serde_json = "1.0.154"
````

In `README.md`, apply this patch (save it to a file and run `git apply <file>`, or edit by hand):

````diff
diff --git a/README.md b/README.md
index af9bacd..bc98f2d 100644
--- a/README.md
+++ b/README.md
@@ -2,7 +2,7 @@
 
 Self-hosted SmartDNS and traffic routing platform. Design and requirements: [Veyra_Project_Specification_RU.md](Veyra_Project_Specification_RU.md).
 
-Current state: policy core, CLI and DNS engine (stage 1a, plan 2 of 4 — see `docs/superpowers/plans/`).
+Current state: policy core, CLI, DNS engine and gateway (stage 1a, plan 3 of 4 — see `docs/superpowers/plans/`).
 
 ## Build and test
 
@@ -30,3 +30,15 @@ dig @127.0.0.1 -p 10053 example.org     # forwarded to 1.1.1.1
 ```
 
 Only sources listed under `admission:` get answers; everyone else is ignored. Windows `nslookup` ignores `-port`: to use it, set `listen: ["127.0.0.1:53"]` in a copy of the config and run `nslookup media.example 127.0.0.1`.
+
+## Run the gateway
+
+The example's `gateway:` section makes the same process de-01's gateway: TLS on 10443 routed by SNI and HTTP on 10080 routed by `Host`, for the names DNS sends to de-01. The gateway resolves origins through the `default` upstream group, dials them on 443 and 80, and never dials private, loopback or other special-purpose addresses. TLS is passed through, not terminated: the client sees the origin's own certificate.
+
+The example's names are made up. To see a real passthrough, route a real domain to de-01 in a copy of the config (`domains: ["suffix:example.com"]`), start `veyra-dataplane` with it, and run:
+
+```bash
+curl --resolve example.com:10443:127.0.0.1 https://example.com:10443/
+```
+
+Connections for other names, and from sources not under `admission:`, are closed at once. On a server, the firewall must reject, not drop, traffic to ports the gateway does not serve: the kernel's TCP reset and ICMP port unreachable are what make clients fail fast (spec §10.1).
````

In `apps/veyra-dataplane/Cargo.toml`, apply this patch (save it to a file and run `git apply <file>`, or edit by hand):

````diff
diff --git a/apps/veyra-dataplane/Cargo.toml b/apps/veyra-dataplane/Cargo.toml
index 516bcff..e636beb 100644
--- a/apps/veyra-dataplane/Cargo.toml
+++ b/apps/veyra-dataplane/Cargo.toml
@@ -8,6 +8,7 @@ publish.workspace = true
 [dependencies]
 veyra-policy.workspace = true
 veyra-dns.workspace = true
+veyra-gateway.workspace = true
 clap.workspace = true
 tokio.workspace = true
 tracing.workspace = true
````

In `apps/veyra-dataplane/src/main.rs`, apply this patch (save it to a file and run `git apply <file>`, or edit by hand):

````diff
diff --git a/apps/veyra-dataplane/src/main.rs b/apps/veyra-dataplane/src/main.rs
index 2550b5f..57adddd 100644
--- a/apps/veyra-dataplane/src/main.rs
+++ b/apps/veyra-dataplane/src/main.rs
@@ -1,5 +1,5 @@
-//! `veyra-dataplane`: serves DNS for a policy file (spec §5.2, §9). The gateway
-//! joins this process in plan 3.
+//! `veyra-dataplane`: serves DNS and, with a `gateway:` section, the gateway
+//! for a policy file (spec §5.2, §9, §10).
 
 use clap::Parser;
 use std::path::PathBuf;
@@ -7,12 +7,18 @@ use std::process::ExitCode;
 use std::sync::Arc;
 use tracing_subscriber::EnvFilter;
 use veyra_dns::{Engine, Listeners};
+use veyra_gateway::{Gateway, Service};
 use veyra_policy::{Bundle, Policy};
 
 #[derive(Parser)]
-#[command(name = "veyra-dataplane", version, about = "Veyra data plane: DNS")]
+#[command(
+    name = "veyra-dataplane",
+    version,
+    about = "Veyra data plane: DNS and gateway"
+)]
 struct Args {
-    /// The policy file; it must have `dns:` and `admission:` sections.
+    /// The policy file; it must have `dns:` and `admission:` sections, and
+    /// `gateway:` to serve the gateway too.
     #[arg(long)]
     config: PathBuf,
 }
@@ -53,6 +59,14 @@ async fn run(args: Args) -> Result<(), String> {
         .dns()
         .map(|dns| dns.listen.clone())
         .unwrap_or_default();
+    // The gateway reads the same policy: DNS hands out its address (spec §10.2).
+    let gateway = match policy.gateway_config().cloned() {
+        Some(config) => {
+            let gateway = Gateway::new(policy.clone()).map_err(|e| format!("{path}: {e}"))?;
+            Some((config, gateway))
+        }
+        None => None,
+    };
     let engine = Arc::new(Engine::new(policy).map_err(|e| format!("{path}: {e}"))?);
     let listeners = Listeners::bind(&listen)
         .await
@@ -60,8 +74,29 @@ async fn run(args: Args) -> Result<(), String> {
     for addr in listeners.local_addrs().map_err(|e| e.to_string())? {
         println!("listening on {addr} (udp, tcp)");
     }
+    let mut gateway_listeners = None;
+    if let Some((config, gateway)) = gateway {
+        let bound = veyra_gateway::Listeners::bind(&config)
+            .await
+            .map_err(|e| format!("cannot listen on {:?}: {e}", [config.tls, config.http]))?;
+        for (addr, service) in bound.local_addrs().map_err(|e| e.to_string())? {
+            let service = match service {
+                Service::Tls => "tls",
+                Service::Http => "http",
+            };
+            println!("listening on {addr} ({service})");
+        }
+        gateway_listeners = Some(bound.serve(Arc::new(gateway)));
+    }
+    let gateway = async {
+        match gateway_listeners {
+            Some(serve) => serve.await,
+            None => std::future::pending().await,
+        }
+    };
     tokio::select! {
         () = listeners.serve(engine) => Ok(()),
+        () = gateway => Ok(()),
         _ = tokio::signal::ctrl_c() => {
             tracing::info!("shutting down");
             Ok(())
````

In `configs/examples/home.yaml`, apply this patch (save it to a file and run `git apply <file>`, or edit by hand):

````diff
diff --git a/configs/examples/home.yaml b/configs/examples/home.yaml
index fe168a1..f1b884d 100644
--- a/configs/examples/home.yaml
+++ b/configs/examples/home.yaml
@@ -62,3 +62,7 @@ spec:
         servers: ["1.1.1.1", "9.9.9.9"]
       - id: corp
         servers: ["10.0.0.53"]
+  gateway:                              # this data plane is de-01's gateway (§10)
+    node: de-01
+    tls: ["127.0.0.1:10443"]            # routed by SNI; production on Linux: ["[::]:443"]
+    http: ["127.0.0.1:10080"]           # routed by Host; production on Linux: ["[::]:80"]
````

In `crates/veyra-gateway/Cargo.toml`, apply this patch (save it to a file and run `git apply <file>`, or edit by hand):

````diff
diff --git a/crates/veyra-gateway/Cargo.toml b/crates/veyra-gateway/Cargo.toml
index d3e7ea3..6b62c9a 100644
--- a/crates/veyra-gateway/Cargo.toml
+++ b/crates/veyra-gateway/Cargo.toml
@@ -10,6 +10,7 @@ veyra-policy.workspace = true
 veyra-dns.workspace = true
 thiserror.workspace = true
 tokio.workspace = true
+tracing.workspace = true
 
 [dev-dependencies]
 hickory-proto.workspace = true
````

In `crates/veyra-gateway/src/lib.rs`, apply this patch (save it to a file and run `git apply <file>`, or edit by hand):

````diff
diff --git a/crates/veyra-gateway/src/lib.rs b/crates/veyra-gateway/src/lib.rs
index 27b1112..ea834ae 100644
--- a/crates/veyra-gateway/src/lib.rs
+++ b/crates/veyra-gateway/src/lib.rs
@@ -4,8 +4,10 @@
 
 mod egress;
 mod gateway;
+mod server;
 mod sniff;
 
 pub use egress::{Egress, SPECIAL_PURPOSE};
 pub use gateway::{Gateway, Refusal};
+pub use server::Listeners;
 pub use sniff::{MAX_HEAD, Service, Sniff, sniff};
````

Create `crates/veyra-gateway/src/server.rs`:

````rust
//! TCP listeners for the gateway (spec §10.1): accept, cap, hand off.

use crate::gateway::Gateway;
use crate::sniff::Service;
use std::io;
use std::net::SocketAddr;
use std::sync::Arc;
use std::time::Duration;
use tokio::net::TcpListener;
use tokio::sync::Semaphore;
use tokio::task::JoinSet;
use tokio::time::sleep;
use veyra_policy::GatewayConfig;

/// Connections served at once; beyond this, new ones are closed at once
/// (spec §10.1 limits). Each holds two sockets: keep it under the file limit.
const MAX_SESSIONS: usize = 1024;

/// Bound sockets. Binding first lets the caller learn the real addresses
/// (for `127.0.0.1:0`) before serving.
pub struct Listeners {
    bound: Vec<(TcpListener, Service)>,
    max_sessions: usize,
}

impl Listeners {
    /// TCP only. UDP on the same ports stays closed, so an HTTP/3 client gets
    /// ICMP port unreachable and falls back to TCP at once (spec §10.1).
    pub async fn bind(config: &GatewayConfig) -> io::Result<Listeners> {
        let tls = config.tls.iter().map(|a| (a, Service::Tls));
        let http = config.http.iter().map(|a| (a, Service::Http));
        let mut bound = Vec::new();
        for (addr, service) in tls.chain(http) {
            bound.push((TcpListener::bind(addr).await?, service));
        }
        Ok(Listeners {
            bound,
            max_sessions: MAX_SESSIONS,
        })
    }

    /// Serve at most `n` connections at once.
    pub fn with_max_sessions(mut self, n: usize) -> Listeners {
        self.max_sessions = n;
        self
    }

    /// The bound addresses: TLS, then HTTP, each in `gateway:` order.
    pub fn local_addrs(&self) -> io::Result<Vec<(SocketAddr, Service)>> {
        self.bound
            .iter()
            .map(|(listener, service)| Ok((listener.local_addr()?, *service)))
            .collect()
    }

    /// Serve until the future is dropped.
    pub async fn serve(self, gateway: Arc<Gateway>) {
        let permits = Arc::new(Semaphore::new(self.max_sessions));
        let mut tasks = JoinSet::new();
        for (listener, service) in self.bound {
            tasks.spawn(serve_listener(
                listener,
                service,
                gateway.clone(),
                permits.clone(),
            ));
        }
        while tasks.join_next().await.is_some() {}
    }
}

async fn serve_listener(
    listener: TcpListener,
    service: Service,
    gateway: Arc<Gateway>,
    permits: Arc<Semaphore>,
) {
    loop {
        let stream = match listener.accept().await {
            Ok((stream, _)) => stream,
            Err(e) => {
                // Out of file descriptors and similar: back off instead of spinning.
                tracing::warn!(error = %e, "accept failed");
                sleep(Duration::from_millis(100)).await;
                continue;
            }
        };
        // At capacity: close at once rather than queue.
        let Ok(permit) = permits.clone().try_acquire_owned() else {
            continue;
        };
        let gateway = gateway.clone();
        tokio::spawn(async move {
            // The reason only: no names or addresses in logs (spec §19.3).
            if let Err(reason) = gateway.handle(stream, service).await {
                tracing::debug!(%reason, ?service, "connection refused");
            }
            drop(permit);
        });
    }
}
````

- [ ] **Step 4: Run the tests to see them pass**

Run: `cargo test --workspace`
Expected: veyra-gateway `server` `ok. 2 passed`, `dataplane` `ok. 3 passed`; 146 in total.

Run: `cargo run -q -p veyra-cli -- validate configs/examples/home.yaml`
Expected: `configs/examples/home.yaml: valid, 0 warning(s)`

Optional, needs internet: route `suffix:example.com` to de-01 in a copy of the example config, start `cargo run -p veyra-dataplane -- --config <copy>`, then run `curl --resolve example.com:10443:127.0.0.1 https://example.com:10443/`.
Expected: HTTP 200, and curl verifies the certificate of example.com itself: the gateway did not terminate TLS.

- [ ] **Step 5: Format and lint**

Run: `cargo fmt --all && cargo clippy --workspace --all-targets -- -D warnings`
Expected: clippy prints no warnings.

- [ ] **Step 6: Commit**

````bash
git add Cargo.lock Cargo.toml README.md apps/veyra-dataplane/Cargo.toml apps/veyra-dataplane/src/main.rs apps/veyra-dataplane/tests/dataplane.rs configs/examples/home.yaml crates/veyra-gateway/Cargo.toml crates/veyra-gateway/src/lib.rs crates/veyra-gateway/src/server.rs crates/veyra-gateway/tests/server.rs
git commit -m "feat(dataplane): serve the gateway next to DNS"
````

---

## Not in this plan

- §21.5 SNI demultiplexing steps 1–2: the DoH and link-IP HTTPS endpoint (`dns.<domain>`, ACME TLS-ALPN-01) arrives in plan 4, and REALITY to Xray over PROXY protocol v2 in stage 1c. Until then, every TLS connection either passes step 3 (mode B) or is closed.
- Link-IP admission, and DoH-token admission of gateway sources (§9.6): plan 4 and stage 1b. Only static CIDR admission here.
- Reloading the snapshot, serving a removed route from the previous version until `drain_window` ends (§8.5, §10.1): plan 4. The gateway reads the YAML file at start.
- Node health and `onFailure` (§9.5, §20.3): stage 2, with the agent.
- Resolving origins through a rule's `dns.upstreamGroup`, and caching origin lookups: every connection resolves through `default` (marked `ponytail:`).
- An idle timeout once bytes flow, per-client quotas, rate limits and metrics (§10.1, §20.1): only the session cap and the hello timeout exist (marked `ponytail:`).
- Happy Eyeballs (RFC 8305): addresses are tried one after another (marked `ponytail:`).
- UDP and QUIC (§10.5): not served. UDP on the gateway ports stays closed so clients fall back to TCP (REQ-GW-002).
- Firewall rules that keep the kernel's TCP reset and ICMP port unreachable for unserved ports (REQ-GW-002 on a real host): plan 4's deployment. The README says so.
- The minor findings deferred from plan 2's final review (EDNS BADVERS, AD on cache hits, IPv4-mapped `admission` networks, SERVFAIL on a re-encode failure, the UDP receive-error spin, a TCP connection cap for DNS) stay deferred.
