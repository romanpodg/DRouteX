# Veyra Snapshots, Ordered Apply and Deployment Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Config changes reach a running `veyra-dataplane` safely. Every version is a snapshot file. `veyra apply` switches the gateway, then DNS, verifies the listeners and rolls back on failure. Routes a change removes keep working through `drainWindow`. The data plane restarts from its last good snapshot and runs under systemd.

**Architecture:** `veyra-policy` decides what keeps draining (`keeps_draining`), and `veyra-gateway` serves drained routes until their deadlines. The DNS and gateway listeners read the current engine and gateway from `tokio::sync::watch` channels, so a switch is a pointer swap. A new crate `veyra-core` owns the snapshot files (`Store`), the ordered activation with VERIFY and rollback (`Runtime`), and the control socket (JSON lines; a Unix socket file). `veyra-dataplane` gains `--state-dir` and `--control`; the CLI gains `apply`, `rollback` and `status`. `deploy/` and `docs/operations/` hold the systemd unit, an example firewall and the install guide.

**Tech Stack:** Rust 2024; no new third-party crates. std files with temp + `rename`, `tokio::sync::watch`, tokio Unix sockets, serde_json.

**Spec:** `Veyra_Project_Specification_RU.md` — §5.1 (keep running on the last valid config), §5.3 (snapshot file; stage/activate/rollback/status over a local Unix socket), §8.5 (order: gateway then DNS when adding, DNS then gateway after `drain_window` when removing; VERIFY; rollback; `drain_window` 10 min and never below the synthesized TTL), §10.1 (the policy version of the route decision), §19.2 items 9–11, §21.2–21.5 (native systemd, port model), §24.3 REQ-APPLY-001…003 and REQ-GW-002 on a real host.

## Where this plan sits

Stage 1a (spec §23.2) is now five plans. Plans 1–3 (policy core and CLI, DNS engine, gateway) are merged; this is plan 4. Plan 5 adds link-IP admission, link tokens (`veyra link-token create|revoke`), and the HTTPS link endpoint behind the gateway with ACME TLS-ALPN-01 certificates (§9.6, §21.5 step 1). The original plan 4 held both: link-IP comes second because it needs this plan's runtime state, control socket and deployment layout.

## Global Constraints

- Everything from plans 1–3 still holds: edition 2024, `rust-version = "1.85"`, dependencies declared once in `[workspace.dependencies]`, `Cargo.lock` committed, no `unwrap()` outside tests, `unsafe_code = "forbid"`, fmt + clippy `-D warnings` + tests green on every commit; integration-test helpers use `expect`.
- No new third-party dependencies.
- Unix-only code (the control socket) sits behind `#[cfg(unix)]` and stays thin; everything else runs and is tested on any OS. It is type-checked and linted on every commit with `cargo clippy --target aarch64-unknown-linux-gnu --workspace --all-targets -- -D warnings` (the target only needs `rustup target add aarch64-unknown-linux-gnu`; every dependency is pure Rust). Its tests run on Linux.
- Spec §8.5, in this order: switch the gateway (it keeps serving removed routes for `drainWindow`), then DNS, then VERIFY, then commit `active`. On any failure, switch DNS back, then the gateway, and leave `active` unchanged. A removed route is not drained when the new version blocks it.
- `drainWindow` defaults to 600 s and is never shorter than `dns.synthTtl`.
- Spec §5.3: snapshots are files written through a temporary file and `rename`. The data plane starts from the active one with no config file and no database (REQ-APPLY-003).
- Listen addresses change only with a restart; `veyra apply` refuses a config that moves them.
- Decisions stay in `veyra-policy` (`keeps_draining`). `veyra-core` owns snapshots, ordering and the control socket; the binaries hold no apply logic.
- No domain names or client addresses in logs (spec §19.3).
- Run every command from the repository root `E:\LeetCode\VeyraDNS`.

## Review Focus

Inputs a person will meet that the spec implies but does not spell out, most likely first. Each is pinned by the named test.

1. **Changing listen addresses:** an apply that moves them must be refused with a clear message, and a restart must pick them up. Pinned by `staging_refuses_broken_configs_and_new_listeners` (Task 5) and `a_changed_config_file_becomes_the_next_version_at_start` (Task 7).
2. **A config file that is broken or missing at boot**, such as after a bad edit or a failed deploy: the data plane must keep serving its last good snapshot. Pinned by `a_broken_config_file_falls_back_to_the_active_snapshot` and `restarts_from_the_active_snapshot_without_the_config_file` (Task 7).
3. **Power loss or a crash mid-write:** no half-written snapshot or pointer, and a damaged pointer is an error, not a silent fresh start. Pinned by `writes_leave_no_temporary_files` and `a_damaged_pointer_is_an_error_not_a_fresh_start` (Task 4).
4. **Applies in quick succession and routes changed to BLOCK:** earlier drains keep their deadlines, and a blocked route stops at once. Pinned by `quick_applies_keep_earlier_drains` and `a_route_changed_to_block_is_refused_at_once` (Task 2).
5. **Garbage or oversized input on the control socket:** a bad line gets an error and the connection goes on; a huge one is refused without exhausting memory. Pinned by `a_bad_line_gets_an_error_and_the_connection_goes_on` and `an_oversized_request_is_refused_and_the_connection_closed` (Task 6).

## File map

| File | Responsibility |
|---|---|
| `crates/veyra-policy/src/gateway.rs` | `drainWindow`; `Policy::keeps_draining` |
| `crates/veyra-gateway/src/gateway.rs` | `Gateway::draining`, `Gateway::passes` |
| `crates/veyra-dns/src/server.rs`, `crates/veyra-gateway/src/server.rs` | Listeners read the current engine and gateway from watch channels |
| `crates/veyra-core/src/store.rs` | Snapshot files: `Store` |
| `crates/veyra-core/src/protocol.rs` | `Request`, `Response` |
| `crates/veyra-core/src/runtime.rs` | `Runtime`: stage, ordered activate, VERIFY, rollback |
| `crates/veyra-core/src/control.rs` | JSON-lines framing, `Client`, the Unix socket |
| `crates/veyra-core/tests/support/mod.rs` | `TempDir`, `Plane` (a running data plane for tests) |
| `apps/veyra-dataplane/src/main.rs` | `--state-dir`, `--control`; start-up from snapshots |
| `apps/veyra-cli/src/main.rs` | `veyra apply`, `rollback`, `status` |
| `deploy/systemd/veyra-dataplane.service`, `deploy/nftables/veyra.nft` | Deployment |
| `docs/operations/install.md` | Install guide |

---

### Task 1: Drain window and what keeps draining

**Files:**
- Modify: `crates/veyra-policy/src/gateway.rs`, `crates/veyra-policy/src/model.rs`, `crates/veyra-policy/tests/gateway.rs`

**Interfaces:**
- Consumes: `GatewayEngineSpec`, `GatewayConfig`, `compile_gateway`, `Policy::routes_to` (plan 3); `Policy::decide_in`, `Mode::SmartdnsTcp`, `Subject`, `Action` (plans 1–2).
- Produces: `GatewayEngineSpec.drain_window: u64` (YAML `drainWindow`, seconds, default 600; the struct gains `rename_all = "camelCase"`); `GatewayConfig.drain_window: Duration`; `InvalidValue` when `drainWindow` < `dns.synthTtl`; `Policy::keeps_draining(&self, previous: &Policy, node: &str, name: &Name) -> bool`, true when some profile or device that `previous` sent to `node` for `name` is not BLOCKed by `self`. `routes_to` now shares two private helpers with it: `subjects()` and `sends(subject, node, name)`.

- [ ] **Step 1: Write the failing tests**

In `crates/veyra-policy/tests/gateway.rs`, apply this patch (save it to a file and run `git apply <file>`, or edit by hand):

````diff
diff --git a/crates/veyra-policy/tests/gateway.rs b/crates/veyra-policy/tests/gateway.rs
index 6a65ac5..b03323d 100644
--- a/crates/veyra-policy/tests/gateway.rs
+++ b/crates/veyra-policy/tests/gateway.rs
@@ -1,6 +1,7 @@
 //! The `gateway:` section and which names a node's gateway passes (spec §10).
 
 use std::net::SocketAddr;
+use std::time::Duration;
 use veyra_policy::{Bundle, Issue, IssueCode, Name, Policy, Severity};
 
 /// Two gateway nodes and one without, two profiles and a device; `extra` is
@@ -55,6 +56,7 @@ fn parses_the_gateway_section_with_default_ports() {
     assert_eq!(config.node, "de-01");
     assert_eq!(config.tls, vec![addr("0.0.0.0:443"), addr("[::]:8443")]);
     assert_eq!(config.http, vec![addr("127.0.0.1:8080")]);
+    assert_eq!(config.drain_window, Duration::from_secs(600), "ten minutes");
 
     let gateways = policy.gateways();
     assert_eq!(
@@ -97,6 +99,13 @@ fn rejects_bad_gateway_sections() {
         vec![InvalidValue],
         "origins resolve through the dns section"
     );
+    assert_eq!(
+        errors(&gateway(
+            "    node: de-01\n    tls: [\"0.0.0.0\"]\n    drainWindow: 30\n"
+        )),
+        vec![InvalidValue],
+        "shorter than synthTtl (60 s)"
+    );
 }
 
 #[test]
@@ -123,3 +132,41 @@ fn a_gateway_passes_what_dns_sends_it_for_any_profile_or_device() {
         "no rule: a profile default never proxies"
     );
 }
+
+/// One version of a small policy: everyone sends `suffix:media.example`
+/// through `action`; strict blocks everything else.
+fn version(action: &str) -> Policy {
+    let text = format!(
+        "apiVersion: veyra.io/v1alpha1\nkind: PolicyBundle\nmetadata: {{ name: t }}\nspec:\n  \
+         nodes: [{{ id: de-01, gateway: {{ ipv4: 203.0.113.10 }} }}, {{ id: us-01, gateway: {{ ipv4: 198.51.100.20 }} }}]\n  \
+         profiles: [{{ id: home }}, {{ id: strict, defaultAction: block }}]\n  \
+         rules:\n    \
+         - {{ id: media, scope: default, match: {{ domains: [\"suffix:media.example\"] }}, action: {action} }}\n"
+    );
+    let bundle = Bundle::from_yaml(&text).expect("test YAML parses");
+    Policy::compile(&bundle).expect("test bundle compiles").0
+}
+
+#[test]
+fn removed_routes_drain_unless_the_new_version_blocks_them() {
+    let before = version("{ type: proxy_node, node: de-01 }");
+    let media = name("www.media.example");
+    let drains = |after: &str| version(after).keeps_draining(&before, "de-01", &media);
+    assert!(
+        drains("{ type: direct }"),
+        "PROXY → DIRECT keeps the old route"
+    );
+    assert!(
+        drains("{ type: proxy_node, node: us-01 }"),
+        "moved to another node"
+    );
+    assert!(!drains("{ type: block }"), "PROXY → BLOCK refuses at once");
+    assert!(
+        !version("{ type: direct }").keeps_draining(&before, "de-01", &name("example.org")),
+        "never routed here"
+    );
+    assert!(
+        !version("{ type: direct }").keeps_draining(&before, "us-01", &media),
+        "routed to another node before"
+    );
+}
````

- [ ] **Step 2: Run the tests to see them fail**

Run: `cargo test -p veyra-policy --test gateway`
Expected: compilation fails: ``no field `drain_window` on type `&GatewayConfig` ``, ``no method named `keeps_draining` found for struct `Policy` `` (three times).

- [ ] **Step 3: Write the implementation**

In `crates/veyra-policy/src/gateway.rs`, apply this patch (save it to a file and run `git apply <file>`, or edit by hand):

````diff
diff --git a/crates/veyra-policy/src/gateway.rs b/crates/veyra-policy/src/gateway.rs
index 8f7216b..61ae540 100644
--- a/crates/veyra-policy/src/gateway.rs
+++ b/crates/veyra-policy/src/gateway.rs
@@ -6,6 +6,7 @@ use crate::dns_config::socket_addr;
 use crate::model::{Action, BundleSpec, Mode};
 use crate::name::Name;
 use std::net::SocketAddr;
+use std::time::Duration;
 
 /// The checked `gateway:` section: this data plane is the gateway of `node`.
 #[derive(Debug, Clone, PartialEq, Eq)]
@@ -15,6 +16,8 @@ pub struct GatewayConfig {
     pub tls: Vec<SocketAddr>,
     /// Where HTTP clients connect; the destination comes from `Host`.
     pub http: Vec<SocketAddr>,
+    /// How long removed routes keep being served after an apply (spec §8.5).
+    pub drain_window: Duration,
 }
 
 /// `None` when the bundle has no `gateway:` section.
@@ -56,10 +59,24 @@ pub(crate) fn compile_gateway(spec: &BundleSpec, issues: &mut Vec<Issue>) -> Opt
         .iter()
         .filter_map(|a| socket_addr(a, Some(80), "gateway.http", issues))
         .collect();
+    // Clients keep a synthesized answer for synthTtl at least; the gateway must
+    // keep serving the route that long (spec §8.5).
+    if let Some(dns) = spec
+        .dns
+        .as_ref()
+        .filter(|d| gateway.drain_window < u64::from(d.synth_ttl))
+    {
+        let msg = format!(
+            "gateway.drainWindow is {} s, below dns.synthTtl {} s",
+            gateway.drain_window, dns.synth_ttl
+        );
+        issues.push(error(InvalidValue, msg));
+    }
     Some(GatewayConfig {
         node: gateway.node.clone(),
         tls,
         http,
+        drain_window: Duration::from_secs(gateway.drain_window),
     })
 }
 
@@ -75,6 +92,25 @@ impl Policy {
     // ponytail: one decision per profile and device on every connection; index
     // names by node if bundles grow to thousands of profiles and devices.
     pub fn routes_to(&self, node: &str, name: &Name) -> bool {
+        self.subjects().any(|s| self.sends(&s, node, name))
+    }
+
+    /// Spec §8.5: for `drainWindow` after an apply, the gateway of `node` keeps
+    /// passing a name `previous` routed there, since clients may still hold the
+    /// gateway's address from DNS. Not when this version blocks the name for
+    /// every profile and device that was routed there: refusing is the point.
+    pub fn keeps_draining(&self, previous: &Policy, node: &str, name: &Name) -> bool {
+        previous.subjects().any(|s| {
+            previous.sends(&s, node, name)
+                && !matches!(
+                    self.decide_in(&s, name, Some(Mode::SmartdnsTcp)).route,
+                    Action::Block {}
+                )
+        })
+    }
+
+    /// Every profile, and every device with its profile.
+    fn subjects(&self) -> impl Iterator<Item = Subject> + '_ {
         let profiles = self.profiles.keys().map(|p| Subject {
             profile: p.clone(),
             device: None,
@@ -83,11 +119,14 @@ impl Policy {
             profile: p.clone(),
             device: Some(d.clone()),
         });
-        profiles.chain(devices).any(|subject| {
-            matches!(
-                self.decide_in(&subject, name, Some(Mode::SmartdnsTcp)).route,
-                Action::ProxyNode { node: to, .. } if to == node
-            )
-        })
+        profiles.chain(devices)
+    }
+
+    /// DNS sends `subject` to the gateway of `node` for `name` in mode B.
+    fn sends(&self, subject: &Subject, node: &str, name: &Name) -> bool {
+        matches!(
+            self.decide_in(subject, name, Some(Mode::SmartdnsTcp)).route,
+            Action::ProxyNode { node: to, .. } if to == node
+        )
     }
 }
````

In `crates/veyra-policy/src/model.rs`, apply this patch (save it to a file and run `git apply <file>`, or edit by hand):

````diff
diff --git a/crates/veyra-policy/src/model.rs b/crates/veyra-policy/src/model.rs
index ffec580..8febd22 100644
--- a/crates/veyra-policy/src/model.rs
+++ b/crates/veyra-policy/src/model.rs
@@ -48,7 +48,7 @@ pub struct BundleSpec {
 /// The `gateway:` section: this data plane is the gateway of `node` and
 /// accepts the connections DNS sends there (spec §10.1, §10.2).
 #[derive(Debug, Clone, Deserialize)]
-#[serde(deny_unknown_fields)]
+#[serde(deny_unknown_fields, rename_all = "camelCase")]
 pub struct GatewayEngineSpec {
     pub node: String,
     /// `ip:port` for TLS, routed by SNI; a bare IP means port 443.
@@ -57,6 +57,14 @@ pub struct GatewayEngineSpec {
     /// `ip:port` for HTTP, routed by `Host`; a bare IP means port 80.
     #[serde(default)]
     pub http: Vec<String>,
+    /// Seconds the gateway keeps serving a route an apply removed, for clients
+    /// that still hold its address from DNS (spec §8.5). At least `dns.synthTtl`.
+    #[serde(default = "default_drain_window")]
+    pub drain_window: u64,
+}
+
+fn default_drain_window() -> u64 {
+    600
 }
 
 /// One `admission` entry: a network and exactly one of `profile` or `device`.
````

- [ ] **Step 4: Run the tests to see them pass**

Run: `cargo test --workspace`
Expected: `gateway` (veyra-policy) `ok. 4 passed`; 148 tests pass in total, none fail.

- [ ] **Step 5: Format and lint**

Run: `cargo fmt --all && cargo clippy --workspace --all-targets -- -D warnings`
Expected: clippy prints no warnings.

Run: `cargo clippy --target aarch64-unknown-linux-gnu --workspace --all-targets -- -D warnings`
Expected: no warnings: the Unix-only code type-checks and lints too.

- [ ] **Step 6: Commit**

````bash
git add crates/veyra-policy/src/gateway.rs crates/veyra-policy/src/model.rs crates/veyra-policy/tests/gateway.rs
git commit -m "feat(policy): drain window and the routes a gateway keeps draining"
````

---

### Task 2: The gateway serves drained routes until their deadline

**Files:**
- Create: `crates/veyra-gateway/tests/drain.rs`
- Modify: `crates/veyra-gateway/src/gateway.rs`

**Interfaces:**
- Consumes: `GatewayConfig.drain_window`, `Policy::keeps_draining` (Task 1); `Gateway`, `Gateway::handle` (plan 3).
- Produces: `Gateway` holds `Arc<Policy>` and `drained: Vec<(Arc<Policy>, Instant)>`; `Gateway::draining(self, previous: &Gateway, now: Instant) -> Gateway` (drains `previous` until `now + drainWindow`, and keeps `previous`'s own unexpired drains); `Gateway::passes(&self, name: &Name, now: Instant) -> bool` (routed now, or drained and not blocked now). `handle` asks `passes(&name, Instant::now())`. Time is a parameter, so tests control the clock (REQ-APPLY-001).

- [ ] **Step 1: Write the failing tests**

Create `crates/veyra-gateway/tests/drain.rs`:

````rust
//! Routes an apply removed keep working for `drainWindow` (spec §8.5,
//! REQ-APPLY-001). Time is passed in, so the tests control the clock.

use std::time::{Duration, Instant};
use veyra_gateway::Gateway;
use veyra_policy::{Bundle, Name, Policy};

const PROXY: &str = "{ type: proxy_node, node: de-01 }";
const WINDOW: Duration = Duration::from_secs(600);
const SECOND: Duration = Duration::from_secs(1);

/// de-01's gateway for a version in which everyone sends `suffix:media.example`
/// through `action` and strict blocks everything else; drainWindow is 600 s.
fn gateway(action: &str) -> Gateway {
    let yaml = format!(
        r#"
apiVersion: veyra.io/v1alpha1
kind: PolicyBundle
metadata: {{ name: drain }}
spec:
  nodes: [{{ id: de-01, gateway: {{ ipv4: 203.0.113.10 }} }}, {{ id: us-01, gateway: {{ ipv4: 198.51.100.20 }} }}]
  profiles: [{{ id: home }}, {{ id: strict, defaultAction: block }}]
  rules:
    - {{ id: media, scope: default, match: {{ domains: ["suffix:media.example"] }}, action: {action} }}
  dns:
    listen: ["127.0.0.1:0"]
    upstreams: [{{ id: default, servers: ["127.0.0.1:9"] }}]
  gateway: {{ node: de-01, tls: ["127.0.0.1:0"], drainWindow: 600 }}
"#
    );
    let bundle = Bundle::from_yaml(&yaml).expect("test YAML parses");
    let (policy, _warnings) = Policy::compile(&bundle).expect("test bundle compiles");
    Gateway::new(policy).expect("policy has a gateway section")
}

fn name(s: &str) -> Name {
    Name::parse(s).expect("test name parses")
}

#[test]
fn a_route_moved_to_direct_is_served_until_the_window_ends() {
    let t0 = Instant::now();
    let media = name("www.media.example");
    let v2 = gateway("{ type: direct }").draining(&gateway(PROXY), t0);
    assert!(v2.passes(&media, t0));
    assert!(v2.passes(&media, t0 + WINDOW - SECOND));
    assert!(!v2.passes(&media, t0 + WINDOW));
    assert!(
        !gateway("{ type: direct }").passes(&media, t0),
        "nothing to drain without a previous version"
    );
}

#[test]
fn a_route_changed_to_block_is_refused_at_once() {
    let t0 = Instant::now();
    let v2 = gateway("{ type: block }").draining(&gateway(PROXY), t0);
    assert!(!v2.passes(&name("www.media.example"), t0));
}

#[test]
fn new_routes_pass_at_once_and_others_never() {
    let t0 = Instant::now();
    let v2 = gateway(PROXY).draining(&gateway("{ type: direct }"), t0);
    assert!(v2.passes(&name("www.media.example"), t0));
    assert!(!v2.passes(&name("example.org"), t0));
}

#[test]
fn quick_applies_keep_earlier_drains() {
    // v1 routes media here; v2 (at t0) and v3 (at t0 + 100 s) send it direct.
    let t0 = Instant::now();
    let media = name("www.media.example");
    let v2 = gateway("{ type: direct }").draining(&gateway(PROXY), t0);
    let v3 = gateway("{ type: direct }").draining(&v2, t0 + 100 * SECOND);
    assert!(
        v3.passes(&media, t0 + WINDOW - SECOND),
        "v1's drain still runs"
    );
    assert!(!v3.passes(&media, t0 + WINDOW), "v2 never routed it here");
}
````

- [ ] **Step 2: Run the tests to see them fail**

Run: `cargo test -p veyra-gateway --test drain`
Expected: compilation fails: ``no method named `draining` found for struct `Gateway` `` (five times), ``no method named `passes` ``.

- [ ] **Step 3: Write the implementation**

In `crates/veyra-gateway/src/gateway.rs`, apply this patch (save it to a file and run `git apply <file>`, or edit by hand):

````diff
diff --git a/crates/veyra-gateway/src/gateway.rs b/crates/veyra-gateway/src/gateway.rs
index a61c3ce..5d8a6b2 100644
--- a/crates/veyra-gateway/src/gateway.rs
+++ b/crates/veyra-gateway/src/gateway.rs
@@ -6,7 +6,8 @@ use crate::egress::Egress;
 use crate::sniff::{Service, Sniff, sniff};
 use std::io;
 use std::net::{IpAddr, SocketAddr};
-use std::time::Duration;
+use std::sync::Arc;
+use std::time::{Duration, Instant};
 use tokio::io::{AsyncReadExt, AsyncWriteExt, copy_bidirectional};
 use tokio::net::TcpStream;
 use tokio::time::timeout;
@@ -47,8 +48,11 @@ pub enum Refusal {
 
 /// The gateway of one node: which names it passes and how it reaches origins.
 pub struct Gateway {
-    policy: Policy,
+    policy: Arc<Policy>,
     node: String,
+    /// Earlier versions whose routes are still served, each until its deadline.
+    drained: Vec<(Arc<Policy>, Instant)>,
+    drain_window: Duration,
     resolvers: Vec<SocketAddr>,
     egress: Egress,
     tls_port: u16,
@@ -60,11 +64,10 @@ pub struct Gateway {
 impl Gateway {
     /// Fails when the policy has no `gateway:` section.
     pub fn new(policy: Policy) -> Result<Gateway, String> {
-        let node = policy
+        let config = policy
             .gateway_config()
-            .ok_or("the config has no gateway section")?
-            .node
-            .clone();
+            .ok_or("the config has no gateway section")?;
+        let (node, drain_window) = (config.node.clone(), config.drain_window);
         // Origins resolve through the default upstream group, never through
         // Veyra's own DNS, which would answer with a gateway address (spec §10.3).
         let resolvers = policy
@@ -74,8 +77,10 @@ impl Gateway {
             .unwrap_or_default();
         let egress = Egress::new(&policy);
         Ok(Gateway {
-            policy,
+            policy: Arc::new(policy),
             node,
+            drained: Vec::new(),
+            drain_window,
             resolvers,
             egress,
             tls_port: 443,
@@ -107,6 +112,30 @@ impl Gateway {
         self
     }
 
+    /// This gateway, for a version that replaces `previous` at `now`: it keeps
+    /// serving `previous`'s routes for `drainWindow`, and the routes `previous`
+    /// was still draining until their own deadlines (spec §8.5).
+    pub fn draining(mut self, previous: &Gateway, now: Instant) -> Gateway {
+        self.drained = previous
+            .drained
+            .iter()
+            .filter(|(_, until)| *until > now)
+            .cloned()
+            .chain([(previous.policy.clone(), now + self.drain_window)])
+            .collect();
+        self
+    }
+
+    /// Does this gateway pass connections for `name` at `now`? Names routed
+    /// here now, and names an earlier version routed here whose drain has not
+    /// ended and that this version does not block.
+    pub fn passes(&self, name: &Name, now: Instant) -> bool {
+        self.policy.routes_to(&self.node, name)
+            || self.drained.iter().any(|(old, until)| {
+                now < *until && self.policy.keeps_draining(old, &self.node, name)
+            })
+    }
+
     pub fn policy(&self) -> &Policy {
         &self.policy
     }
@@ -121,7 +150,7 @@ impl Gateway {
         let (head, name) = timeout(self.hello_timeout, read_head(&mut client, service))
             .await
             .map_err(|_| io::Error::from(io::ErrorKind::TimedOut))??;
-        if !self.policy.routes_to(&self.node, &name) {
+        if !self.passes(&name, Instant::now()) {
             return Err(Refusal::NotRouted);
         }
         let addrs = lookup(&self.resolvers, name.as_str(), self.upstream_timeout).await;
````

- [ ] **Step 4: Run the tests to see them pass**

Run: `cargo test --workspace`
Expected: `drain` `ok. 4 passed`; 152 in total.

- [ ] **Step 5: Format and lint**

Run: `cargo fmt --all && cargo clippy --workspace --all-targets -- -D warnings`
Expected: clippy prints no warnings.

Run: `cargo clippy --target aarch64-unknown-linux-gnu --workspace --all-targets -- -D warnings`
Expected: no warnings: the Unix-only code type-checks and lints too.

- [ ] **Step 6: Commit**

````bash
git add crates/veyra-gateway/src/gateway.rs crates/veyra-gateway/tests/drain.rs
git commit -m "feat(gateway): keep serving removed routes for the drain window"
````

---

### Task 3: Listeners take whichever engine and gateway are current

**Files:**
- Modify: `apps/veyra-dataplane/src/main.rs`, `crates/veyra-dns/src/server.rs`, `crates/veyra-dns/tests/server.rs`, `crates/veyra-gateway/src/server.rs`, `crates/veyra-gateway/tests/server.rs`

**Interfaces:**
- Consumes: `veyra_dns::Listeners`, `Engine` (plan 2); `veyra_gateway::Listeners`, `Gateway` (plan 3).
- Produces: `veyra_dns::Listeners::serve(self, engine: watch::Receiver<Arc<Engine>>)`: every UDP query, every TCP accept (for admission) and every TCP query reads the engine current at that moment. `veyra_gateway::Listeners::serve(self, gateway: watch::Receiver<Arc<Gateway>>)`: each connection gets the gateway current when it is accepted, and keeps it to the end (spec §10.1). `veyra-dataplane` passes `watch::channel(...).1` for now; Task 7 keeps the senders.

- [ ] **Step 1: Write the failing tests**

In `crates/veyra-dns/tests/server.rs`, apply this patch (save it to a file and run `git apply <file>`, or edit by hand):

````diff
diff --git a/crates/veyra-dns/tests/server.rs b/crates/veyra-dns/tests/server.rs
index 0d49cba..331e2ed 100644
--- a/crates/veyra-dns/tests/server.rs
+++ b/crates/veyra-dns/tests/server.rs
@@ -2,7 +2,7 @@
 
 mod support;
 
-use hickory_proto::op::Message;
+use hickory_proto::op::{Message, ResponseCode};
 use hickory_proto::rr::{RData, RecordType};
 use std::net::{Ipv4Addr, SocketAddr};
 use std::sync::Arc;
@@ -10,18 +10,43 @@ use std::time::Duration;
 use support::{dead_upstream, engine, query};
 use tokio::io::{AsyncReadExt, AsyncWriteExt};
 use tokio::net::{TcpSocket, TcpStream, UdpSocket};
+use tokio::sync::watch;
 use tokio::time::timeout;
-use veyra_dns::Listeners;
+use veyra_dns::{Engine, Listeners};
+use veyra_policy::{Bundle, Policy};
 
-/// Serve the support engine on 127.0.0.1 with an OS-chosen port.
-async fn serve() -> SocketAddr {
+/// Serve `engine` on 127.0.0.1 with an OS-chosen port; the sender swaps it.
+async fn serve_engine(engine: Engine) -> (SocketAddr, watch::Sender<Arc<Engine>>) {
     let listeners = Listeners::bind(&["127.0.0.1:0".parse().expect("address")])
         .await
         .expect("bind listeners");
     let addr = listeners.local_addrs().expect("bound addresses")[0];
-    let engine = Arc::new(engine(&[dead_upstream().await]));
-    tokio::spawn(listeners.serve(engine));
-    addr
+    let (engines, current) = watch::channel(Arc::new(engine));
+    tokio::spawn(listeners.serve(current));
+    (addr, engines)
+}
+
+/// Serve the support engine on 127.0.0.1 with an OS-chosen port.
+async fn serve() -> SocketAddr {
+    serve_engine(engine(&[dead_upstream().await])).await.0
+}
+
+/// An engine whose only profile, for 127.0.0.1, blocks every name.
+fn blocking_engine() -> Engine {
+    let yaml = r#"
+apiVersion: veyra.io/v1alpha1
+kind: PolicyBundle
+metadata: { name: blocking }
+spec:
+  profiles: [{ id: home, defaultAction: block }]
+  admission: [{ cidr: 127.0.0.1/32, profile: home }]
+  dns:
+    listen: ["127.0.0.1:0"]
+    upstreams: [{ id: default, servers: ["127.0.0.1:9"] }]
+"#;
+    let bundle = Bundle::from_yaml(yaml).expect("test YAML parses");
+    let (policy, _warnings) = Policy::compile(&bundle).expect("test bundle compiles");
+    Engine::new(policy).expect("policy has a dns section")
 }
 
 fn blocked(reply: &Message) -> bool {
@@ -94,3 +119,15 @@ async fn strangers_get_a_closed_tcp_connection() {
         .expect("the server closes the connection promptly");
     assert!(matches!(read, Ok(0) | Err(_)), "{read:?}");
 }
+
+#[tokio::test]
+async fn a_new_engine_answers_the_next_query_on_an_open_connection() {
+    let (server, engines) = serve_engine(engine(&[dead_upstream().await])).await;
+    let mut stream = TcpStream::connect(server).await.expect("connect");
+    let before = tcp_ask(&mut stream, &query("example.org.", RecordType::A)).await;
+    assert_eq!(before.metadata.response_code, ResponseCode::ServFail);
+
+    engines.send_replace(Arc::new(blocking_engine()));
+    let after = tcp_ask(&mut stream, &query("example.org.", RecordType::A)).await;
+    assert!(blocked(&after), "{after:?}");
+}
````

In `crates/veyra-gateway/tests/server.rs`, apply this patch (save it to a file and run `git apply <file>`, or edit by hand):

````diff
diff --git a/crates/veyra-gateway/tests/server.rs b/crates/veyra-gateway/tests/server.rs
index 010966d..857e50d 100644
--- a/crates/veyra-gateway/tests/server.rs
+++ b/crates/veyra-gateway/tests/server.rs
@@ -8,8 +8,10 @@ use std::time::Duration;
 use support::{client_hello, gateway, origin, round_trip, upstream};
 use tokio::io::{AsyncReadExt, AsyncWriteExt};
 use tokio::net::TcpStream;
+use tokio::sync::watch;
 use tokio::time::{sleep, timeout};
-use veyra_gateway::{Listeners, Service};
+use veyra_gateway::{Gateway, Listeners, Service};
+use veyra_policy::{Bundle, Policy};
 
 const REQUEST: &[u8] = b"GET / HTTP/1.1\r\nHost: media.example\r\n\r\n";
 
@@ -20,7 +22,7 @@ async fn serves_tls_and_http_listeners() {
     let config = gateway.policy().gateway_config().expect("gateway section");
     let listeners = Listeners::bind(config).await.expect("bind");
     let addrs = listeners.local_addrs().expect("bound addresses");
-    tokio::spawn(listeners.serve(Arc::new(gateway)));
+    tokio::spawn(listeners.serve(watch::channel(Arc::new(gateway)).1));
     let services: Vec<Service> = addrs.iter().map(|(_, s)| *s).collect();
     assert_eq!(services, [Service::Tls, Service::Http]);
 
@@ -42,7 +44,7 @@ async fn connections_beyond_the_cap_are_closed_at_once() {
         .expect("bind")
         .with_max_sessions(1);
     let tls = listeners.local_addrs().expect("bound addresses")[0].0;
-    tokio::spawn(listeners.serve(Arc::new(gateway)));
+    tokio::spawn(listeners.serve(watch::channel(Arc::new(gateway)).1));
 
     // Holds the only session while the gateway waits for its ClientHello.
     let _silent = TcpStream::connect(tls).await.unwrap();
@@ -58,3 +60,52 @@ async fn connections_beyond_the_cap_are_closed_at_once() {
     assert!(matches!(read, Ok(0) | Err(_)), "{read:?}");
     assert_eq!(origin.connections(), 0);
 }
+
+/// de-01's gateway for a policy that routes nothing to it.
+fn unrouted() -> Gateway {
+    let yaml = r#"
+apiVersion: veyra.io/v1alpha1
+kind: PolicyBundle
+metadata: { name: unrouted }
+spec:
+  nodes: [{ id: de-01, gateway: { ipv4: 203.0.113.10 } }]
+  profiles: [{ id: home }]
+  admission: [{ cidr: 127.0.0.1/32, profile: home }]
+  dns:
+    listen: ["127.0.0.1:0"]
+    upstreams: [{ id: default, servers: ["127.0.0.1:9"] }]
+  gateway: { node: de-01, tls: ["127.0.0.1:0"] }
+"#;
+    let bundle = Bundle::from_yaml(yaml).expect("test YAML parses");
+    let (policy, _warnings) = Policy::compile(&bundle).expect("test bundle compiles");
+    Gateway::new(policy).expect("policy has a gateway section")
+}
+
+#[tokio::test]
+async fn a_new_gateway_takes_the_next_connection_and_open_ones_keep_theirs() {
+    let origin = origin().await;
+    let gateway = gateway(upstream(&[Ipv4Addr::LOCALHOST]).await, origin.port);
+    let config = gateway.policy().gateway_config().expect("gateway section");
+    let listeners = Listeners::bind(config).await.expect("bind");
+    let tls = listeners.local_addrs().expect("bound addresses")[0].0;
+    let (gateways, current) = watch::channel(Arc::new(gateway));
+    tokio::spawn(listeners.serve(current));
+
+    let hello = client_hello(Some("media.example"), &[]);
+    let mut open = TcpStream::connect(tls).await.unwrap();
+    assert_eq!(round_trip(&mut open, &hello).await, hello);
+
+    gateways.send_replace(Arc::new(unrouted()));
+    let mut next = TcpStream::connect(tls).await.unwrap();
+    let _ = next.write_all(&hello).await;
+    let mut byte = [0u8; 1];
+    let read = timeout(Duration::from_secs(2), next.read(&mut byte))
+        .await
+        .expect("refused, not left waiting");
+    assert!(matches!(read, Ok(0) | Err(_)), "{read:?}");
+    assert_eq!(
+        round_trip(&mut open, b"more").await,
+        b"more",
+        "the open connection keeps its version"
+    );
+}
````

- [ ] **Step 2: Run the tests to see them fail**

Run: `cargo test -p veyra-dns -p veyra-gateway --test server`
Expected: compilation fails with ``mismatched types``: the tests pass a `watch::Receiver` where `serve` still takes an `Arc`.

- [ ] **Step 3: Write the implementation**

In `apps/veyra-dataplane/src/main.rs`, apply this patch (save it to a file and run `git apply <file>`, or edit by hand):

````diff
diff --git a/apps/veyra-dataplane/src/main.rs b/apps/veyra-dataplane/src/main.rs
index 57adddd..42fc689 100644
--- a/apps/veyra-dataplane/src/main.rs
+++ b/apps/veyra-dataplane/src/main.rs
@@ -5,6 +5,7 @@ use clap::Parser;
 use std::path::PathBuf;
 use std::process::ExitCode;
 use std::sync::Arc;
+use tokio::sync::watch;
 use tracing_subscriber::EnvFilter;
 use veyra_dns::{Engine, Listeners};
 use veyra_gateway::{Gateway, Service};
@@ -86,7 +87,7 @@ async fn run(args: Args) -> Result<(), String> {
             };
             println!("listening on {addr} ({service})");
         }
-        gateway_listeners = Some(bound.serve(Arc::new(gateway)));
+        gateway_listeners = Some(bound.serve(watch::channel(Arc::new(gateway)).1));
     }
     let gateway = async {
         match gateway_listeners {
@@ -95,7 +96,7 @@ async fn run(args: Args) -> Result<(), String> {
         }
     };
     tokio::select! {
-        () = listeners.serve(engine) => Ok(()),
+        () = listeners.serve(watch::channel(engine).1) => Ok(()),
         () = gateway => Ok(()),
         _ = tokio::signal::ctrl_c() => {
             tracing::info!("shutting down");
````

In `crates/veyra-dns/src/server.rs`, apply this patch (save it to a file and run `git apply <file>`, or edit by hand):

````diff
diff --git a/crates/veyra-dns/src/server.rs b/crates/veyra-dns/src/server.rs
index 1840a73..c893005 100644
--- a/crates/veyra-dns/src/server.rs
+++ b/crates/veyra-dns/src/server.rs
@@ -7,7 +7,7 @@ use std::net::SocketAddr;
 use std::sync::Arc;
 use std::time::Duration;
 use tokio::net::{TcpListener, TcpStream, UdpSocket};
-use tokio::sync::Semaphore;
+use tokio::sync::{Semaphore, watch};
 use tokio::task::JoinSet;
 use tokio::time::{sleep, timeout};
 
@@ -41,8 +41,10 @@ impl Listeners {
         self.udp.iter().map(UdpSocket::local_addr).collect()
     }
 
-    /// Serve until the future is dropped.
-    pub async fn serve(self, engine: Arc<Engine>) {
+    /// Serve until the future is dropped. Each query is answered by the engine
+    /// `engine` holds when it arrives, so a new version takes over from the
+    /// next query on (spec §8.5).
+    pub async fn serve(self, engine: watch::Receiver<Arc<Engine>>) {
         let permits = Arc::new(Semaphore::new(MAX_IN_FLIGHT));
         let mut tasks = JoinSet::new();
         for socket in self.udp {
@@ -55,7 +57,11 @@ impl Listeners {
     }
 }
 
-async fn serve_udp(socket: Arc<UdpSocket>, engine: Arc<Engine>, permits: Arc<Semaphore>) {
+async fn serve_udp(
+    socket: Arc<UdpSocket>,
+    engine: watch::Receiver<Arc<Engine>>,
+    permits: Arc<Semaphore>,
+) {
     let mut buf = vec![0; 65_535];
     loop {
         // Windows reports an ICMP "port unreachable" for an earlier reply as an
@@ -67,7 +73,7 @@ async fn serve_udp(socket: Arc<UdpSocket>, engine: Arc<Engine>, permits: Arc<Sem
             continue;
         };
         let query = buf[..len].to_vec();
-        let (socket, engine) = (socket.clone(), engine.clone());
+        let (socket, engine) = (socket.clone(), engine.borrow().clone());
         tokio::spawn(async move {
             if let Some(reply) = engine.handle(peer.ip(), &query, Transport::Udp).await {
                 let _ = socket.send_to(&reply, peer).await;
@@ -77,7 +83,11 @@ async fn serve_udp(socket: Arc<UdpSocket>, engine: Arc<Engine>, permits: Arc<Sem
     }
 }
 
-async fn serve_tcp(listener: TcpListener, engine: Arc<Engine>, permits: Arc<Semaphore>) {
+async fn serve_tcp(
+    listener: TcpListener,
+    engine: watch::Receiver<Arc<Engine>>,
+    permits: Arc<Semaphore>,
+) {
     loop {
         let (stream, peer) = match listener.accept().await {
             Ok(accepted) => accepted,
@@ -89,7 +99,7 @@ async fn serve_tcp(listener: TcpListener, engine: Arc<Engine>, permits: Arc<Sema
             }
         };
         // Not admitted: close at once, before reading a byte (spec §9.6).
-        if engine.policy().admit(peer.ip()).is_none() {
+        if engine.borrow().policy().admit(peer.ip()).is_none() {
             continue;
         }
         tokio::spawn(serve_connection(
@@ -104,13 +114,14 @@ async fn serve_tcp(listener: TcpListener, engine: Arc<Engine>, permits: Arc<Sema
 async fn serve_connection(
     mut stream: TcpStream,
     peer: SocketAddr,
-    engine: Arc<Engine>,
+    engine: watch::Receiver<Arc<Engine>>,
     permits: Arc<Semaphore>,
 ) {
     while let Ok(Ok(query)) = timeout(TCP_IDLE, read_frame(&mut stream)).await {
         let Ok(_permit) = permits.acquire().await else {
             return;
         };
+        let engine = engine.borrow().clone();
         let Some(reply) = engine.handle(peer.ip(), &query, Transport::Tcp).await else {
             return;
         };
````

In `crates/veyra-gateway/src/server.rs`, apply this patch (save it to a file and run `git apply <file>`, or edit by hand):

````diff
diff --git a/crates/veyra-gateway/src/server.rs b/crates/veyra-gateway/src/server.rs
index 9f549e0..3771926 100644
--- a/crates/veyra-gateway/src/server.rs
+++ b/crates/veyra-gateway/src/server.rs
@@ -7,7 +7,7 @@ use std::net::SocketAddr;
 use std::sync::Arc;
 use std::time::Duration;
 use tokio::net::TcpListener;
-use tokio::sync::Semaphore;
+use tokio::sync::{Semaphore, watch};
 use tokio::task::JoinSet;
 use tokio::time::sleep;
 use veyra_policy::GatewayConfig;
@@ -55,8 +55,10 @@ impl Listeners {
             .collect()
     }
 
-    /// Serve until the future is dropped.
-    pub async fn serve(self, gateway: Arc<Gateway>) {
+    /// Serve until the future is dropped. Each connection is handled by the
+    /// gateway `gateway` holds when it is accepted: the policy version of the
+    /// route decision serves it to the end (spec §10.1).
+    pub async fn serve(self, gateway: watch::Receiver<Arc<Gateway>>) {
         let permits = Arc::new(Semaphore::new(self.max_sessions));
         let mut tasks = JoinSet::new();
         for (listener, service) in self.bound {
@@ -74,7 +76,7 @@ impl Listeners {
 async fn serve_listener(
     listener: TcpListener,
     service: Service,
-    gateway: Arc<Gateway>,
+    gateway: watch::Receiver<Arc<Gateway>>,
     permits: Arc<Semaphore>,
 ) {
     loop {
@@ -91,7 +93,7 @@ async fn serve_listener(
         let Ok(permit) = permits.clone().try_acquire_owned() else {
             continue;
         };
-        let gateway = gateway.clone();
+        let gateway = gateway.borrow().clone();
         tokio::spawn(async move {
             // The reason only: no names or addresses in logs (spec §19.3).
             if let Err(reason) = gateway.handle(stream, service).await {
````

- [ ] **Step 4: Run the tests to see them pass**

Run: `cargo test --workspace`
Expected: `server` (veyra-dns) `ok. 5 passed`, `server` (veyra-gateway) `ok. 3 passed`; 154 in total.

- [ ] **Step 5: Format and lint**

Run: `cargo fmt --all && cargo clippy --workspace --all-targets -- -D warnings`
Expected: clippy prints no warnings.

Run: `cargo clippy --target aarch64-unknown-linux-gnu --workspace --all-targets -- -D warnings`
Expected: no warnings: the Unix-only code type-checks and lints too.

- [ ] **Step 6: Commit**

````bash
git add apps/veyra-dataplane/src/main.rs crates/veyra-dns/src/server.rs crates/veyra-dns/tests/server.rs crates/veyra-gateway/src/server.rs crates/veyra-gateway/tests/server.rs
git commit -m "feat(dns,gateway): listeners serve whichever engine and gateway are current"
````

---

### Task 4: `veyra-core`: snapshot files

**Files:**
- Create: `crates/veyra-core/Cargo.toml`, `crates/veyra-core/src/lib.rs`, `crates/veyra-core/src/store.rs`, `crates/veyra-core/tests/store.rs`, `crates/veyra-core/tests/support/mod.rs`
- Modify: `Cargo.toml`

**Interfaces:**
- Consumes: nothing but std.
- Produces: the crate `veyra-core`; `Store::open(dir: impl Into<PathBuf>) -> io::Result<Store>` (creates `dir/snapshots`); `stage(&self, bundle: &str) -> io::Result<u64>` (next number after the highest snapshot file, written as `snapshots/<10-digit version>.yaml`); `read(&self, version: u64) -> io::Result<String>`; `active(&self)` and `previous(&self) -> io::Result<Option<u64>>` (a damaged pointer file is `InvalidData`, not `None`); `activate(&self, version: u64) -> io::Result<()>` (unknown version → `NotFound`; the old active becomes `previous`; activating the active version changes nothing). Every write is a temporary file, `sync_all`, then `rename`. Test helper `tests/support/mod.rs`: `TempDir(pub PathBuf)` with `new(name)`, removed on drop.

- [ ] **Step 1: Write the failing tests**

In `Cargo.toml`, apply this patch (save it to a file and run `git apply <file>`, or edit by hand):

````diff
diff --git a/Cargo.toml b/Cargo.toml
index 948744c..75d29c9 100644
--- a/Cargo.toml
+++ b/Cargo.toml
@@ -1,6 +1,6 @@
 [workspace]
 resolver = "3"
-members = ["crates/veyra-policy", "crates/veyra-dns", "crates/veyra-gateway", "apps/veyra-cli", "apps/veyra-dataplane"]
+members = ["crates/veyra-policy", "crates/veyra-dns", "crates/veyra-gateway", "crates/veyra-core", "apps/veyra-cli", "apps/veyra-dataplane"]
 
 [workspace.package]
 version = "0.1.0"
````

Create `crates/veyra-core/Cargo.toml`:

````toml
[package]
name = "veyra-core"
version.workspace = true
edition.workspace = true
rust-version.workspace = true
publish.workspace = true

[lints]
workspace = true
````

Create `crates/veyra-core/src/lib.rs`:

````rust
//! Veyra runtime orchestration (spec §5.3, §8.5).
````

Create `crates/veyra-core/tests/store.rs`:

````rust
//! Snapshot files (spec §5.3, §8.5): what the data plane starts from.

mod support;

use support::TempDir;
use veyra_core::Store;

#[test]
fn a_fresh_store_has_nothing_active() {
    let dir = TempDir::new("fresh");
    let store = Store::open(&dir.0).unwrap();
    assert_eq!(store.active().unwrap(), None);
    assert_eq!(store.previous().unwrap(), None);
}

#[test]
fn versions_only_grow_and_keep_their_text() {
    let dir = TempDir::new("grow");
    let store = Store::open(&dir.0).unwrap();
    assert_eq!(store.stage("one").unwrap(), 1);
    assert_eq!(store.stage("two").unwrap(), 2);
    let reopened = Store::open(&dir.0).unwrap();
    assert_eq!(reopened.stage("three").unwrap(), 3);
    assert_eq!(reopened.read(2).unwrap(), "two");
}

#[test]
fn activating_moves_the_active_version_to_previous() {
    let dir = TempDir::new("activate");
    let store = Store::open(&dir.0).unwrap();
    let one = store.stage("one").unwrap();
    let two = store.stage("two").unwrap();
    store.activate(one).unwrap();
    assert_eq!(store.active().unwrap(), Some(one));
    assert_eq!(store.previous().unwrap(), None);
    store.activate(two).unwrap();
    assert_eq!(store.active().unwrap(), Some(two));
    assert_eq!(store.previous().unwrap(), Some(one));
    store.activate(two).unwrap();
    assert_eq!(
        store.previous().unwrap(),
        Some(one),
        "activating the active version changes nothing"
    );
    assert!(store.activate(9).is_err(), "there is no snapshot 9");
    assert_eq!(store.active().unwrap(), Some(two));
}

#[test]
fn writes_leave_no_temporary_files() {
    let dir = TempDir::new("tmp");
    let store = Store::open(&dir.0).unwrap();
    store.activate(store.stage("one").unwrap()).unwrap();
    let mut names: Vec<String> = ["", "snapshots"]
        .iter()
        .flat_map(|sub| std::fs::read_dir(dir.0.join(sub)).unwrap())
        .map(|entry| entry.unwrap().file_name().to_string_lossy().into_owned())
        .collect();
    names.sort();
    assert_eq!(names, ["0000000001.yaml", "active", "snapshots"]);
}

#[test]
fn a_damaged_pointer_is_an_error_not_a_fresh_start() {
    let dir = TempDir::new("damaged");
    let store = Store::open(&dir.0).unwrap();
    std::fs::write(dir.0.join("active"), "garbage").unwrap();
    let err = store.active().unwrap_err();
    assert_eq!(err.kind(), std::io::ErrorKind::InvalidData);
}
````

Create `crates/veyra-core/tests/support/mod.rs`:

````rust
//! Shared test helpers.
#![allow(dead_code)] // each test file uses a different subset

use std::path::PathBuf;

/// A state directory of its own for one test, removed when the test ends.
pub struct TempDir(pub PathBuf);

impl TempDir {
    pub fn new(name: &str) -> TempDir {
        let path = std::env::temp_dir().join(format!("veyra-core-{}-{name}", std::process::id()));
        let _ = std::fs::remove_dir_all(&path);
        TempDir(path)
    }
}

impl Drop for TempDir {
    fn drop(&mut self) {
        let _ = std::fs::remove_dir_all(&self.0);
    }
}
````

- [ ] **Step 2: Run the tests to see them fail**

Run: `cargo test -p veyra-core`
Expected: compilation fails: ``unresolved import `veyra_core::Store` ``.

- [ ] **Step 3: Write the implementation**

Replace `crates/veyra-core/src/lib.rs` with:

````rust
//! Veyra runtime orchestration (spec §5.3, §8.5): the snapshot files the data
//! plane starts from, ordered activation of new versions, and the control socket.

mod store;

pub use store::Store;
````

Create `crates/veyra-core/src/store.rs`:

````rust
//! Snapshot files (spec §5.3, §8.5). Every staged config is kept as an
//! immutable numbered file; `active` names the version the data plane runs and
//! `previous` the one before it. Each write goes to a temporary file that is
//! then renamed into place, so a crash never leaves a half-written file.

use std::fs;
use std::io::{self, Write};
use std::path::{Path, PathBuf};

/// A state directory: `snapshots/<version>.yaml`, `active` and `previous`.
#[derive(Debug, Clone)]
pub struct Store {
    dir: PathBuf,
}

impl Store {
    /// Open the state directory, creating it when it does not exist.
    pub fn open(dir: impl Into<PathBuf>) -> io::Result<Store> {
        let dir = dir.into();
        fs::create_dir_all(dir.join("snapshots"))?;
        Ok(Store { dir })
    }

    /// Keep `bundle` as the next version and return its number. Numbers only
    /// grow, so a version always names the same config.
    pub fn stage(&self, bundle: &str) -> io::Result<u64> {
        let version = self.versions()?.into_iter().max().unwrap_or(0) + 1;
        write_atomically(&self.snapshot(version), bundle)?;
        Ok(version)
    }

    /// The config text of `version`.
    pub fn read(&self, version: u64) -> io::Result<String> {
        fs::read_to_string(self.snapshot(version))
    }

    /// The version the data plane runs, if one was ever activated.
    pub fn active(&self) -> io::Result<Option<u64>> {
        self.pointer("active")
    }

    /// The version that was active before the current one: the rollback target.
    pub fn previous(&self) -> io::Result<Option<u64>> {
        self.pointer("previous")
    }

    /// Make `version` the active one; the active one becomes `previous`.
    pub fn activate(&self, version: u64) -> io::Result<()> {
        if !self.snapshot(version).is_file() {
            let msg = format!("there is no snapshot {version}");
            return Err(io::Error::new(io::ErrorKind::NotFound, msg));
        }
        match self.active()? {
            Some(current) if current == version => return Ok(()),
            Some(current) => write_atomically(&self.dir.join("previous"), &format!("{current}\n"))?,
            None => {}
        }
        write_atomically(&self.dir.join("active"), &format!("{version}\n"))
    }

    fn snapshot(&self, version: u64) -> PathBuf {
        self.dir
            .join("snapshots")
            .join(format!("{version:010}.yaml"))
    }

    fn versions(&self) -> io::Result<Vec<u64>> {
        let mut versions = Vec::new();
        for entry in fs::read_dir(self.dir.join("snapshots"))? {
            let name = entry?.file_name();
            let number = name.to_str().and_then(|n| n.strip_suffix(".yaml"));
            versions.extend(number.and_then(|n| n.parse::<u64>().ok()));
        }
        Ok(versions)
    }

    fn pointer(&self, file: &str) -> io::Result<Option<u64>> {
        let path = self.dir.join(file);
        match fs::read_to_string(&path) {
            Ok(text) => text.trim().parse().map(Some).map_err(|_| {
                let msg = format!("{}: not a version number", path.display());
                io::Error::new(io::ErrorKind::InvalidData, msg)
            }),
            Err(e) if e.kind() == io::ErrorKind::NotFound => Ok(None),
            Err(e) => Err(e),
        }
    }
}

/// Readers see the old file or the new one, never a torn one.
fn write_atomically(path: &Path, text: &str) -> io::Result<()> {
    let tmp = path.with_extension("tmp");
    let mut file = fs::File::create(&tmp)?;
    file.write_all(text.as_bytes())?;
    file.sync_all()?;
    drop(file);
    fs::rename(&tmp, path)
}
````

- [ ] **Step 4: Run the tests to see them pass**

Run: `cargo test --workspace`
Expected: `store` `ok. 5 passed`; 159 in total.

- [ ] **Step 5: Format and lint**

Run: `cargo fmt --all && cargo clippy --workspace --all-targets -- -D warnings`
Expected: clippy prints no warnings.

Run: `cargo clippy --target aarch64-unknown-linux-gnu --workspace --all-targets -- -D warnings`
Expected: no warnings: the Unix-only code type-checks and lints too.

- [ ] **Step 6: Commit**

````bash
git add Cargo.lock Cargo.toml crates/veyra-core/Cargo.toml crates/veyra-core/src/lib.rs crates/veyra-core/src/store.rs crates/veyra-core/tests/store.rs crates/veyra-core/tests/support/mod.rs
git commit -m "feat(core): snapshot files with active and previous versions"
````

---

### Task 5: Ordered activation, VERIFY and rollback

**Files:**
- Create: `crates/veyra-core/src/protocol.rs`, `crates/veyra-core/src/runtime.rs`, `crates/veyra-core/tests/runtime.rs`
- Modify: `crates/veyra-core/Cargo.toml`, `crates/veyra-core/src/lib.rs`, `crates/veyra-core/tests/support/mod.rs`

**Interfaces:**
- Consumes: `Store` (Task 4); `Gateway::new`, `Gateway::draining` (Task 2); `Engine::new`, `Engine::policy` (plan 2); the watch channels of Task 3; `Bundle::from_yaml`, `Policy::compile` (plan 1).
- Produces: `Request::{Status, Stage { bundle: String }, Activate { version: u64 }, Rollback}` (serde, `#[serde(tag = "command", rename_all = "snake_case")]`); `Response { ok, active, previous, staged: Option<u64>, steps: Vec<String>, messages: Vec<String> }` with `Response::failed(message)`. `Runtime::new(store, policy: &Policy, engine: watch::Sender<Arc<Engine>>, gateway: Option<watch::Sender<Arc<Gateway>>>, probes: Vec<SocketAddr>)` and `Runtime::handle(&self, Request) -> Response`, which serves one request at a time.
- `Stage` compiles the config (issues in `messages` on failure, warnings on success), refuses other listen addresses than the running ones, then keeps it. `Activate` loads the version, builds the engine and gateway, switches the gateway (draining the old one), then DNS, then VERIFY. VERIFY is a TCP connect to every probe (3 s each; unspecified addresses through loopback). It ends by committing `active`. Any failure after a switch restores DNS, then the gateway. Steps read `gateway: version N`, `dns: version N`, `verify: K listeners accept connections`, `active: version N`, or `rollback dns: version M` and `rollback gateway: version M`. `Rollback` activates `previous` the same way.
- Test helpers move into `tests/support/mod.rs`: `config(action) -> String`, `PROXY`, `DIRECT`, `proxied() -> Action`, and `Plane::start(name, extra_probe) -> Plane { runtime, engine, gateway }` (a data plane on version 1, listeners bound on 127.0.0.1), with `dns_route(name)`, `gateway_passes(name, at)` and `stage(text)`.

- [ ] **Step 1: Write the failing tests**

In `crates/veyra-core/Cargo.toml`, apply this patch (save it to a file and run `git apply <file>`, or edit by hand):

````diff
diff --git a/crates/veyra-core/Cargo.toml b/crates/veyra-core/Cargo.toml
index 03dfa13..de38368 100644
--- a/crates/veyra-core/Cargo.toml
+++ b/crates/veyra-core/Cargo.toml
@@ -5,5 +5,13 @@ edition.workspace = true
 rust-version.workspace = true
 publish.workspace = true
 
+[dependencies]
+veyra-policy.workspace = true
+veyra-dns.workspace = true
+veyra-gateway.workspace = true
+serde.workspace = true
+tokio.workspace = true
+tracing.workspace = true
+
 [lints]
 workspace = true
````

Create `crates/veyra-core/tests/runtime.rs`:

````rust
//! Ordered activation, VERIFY and rollback (spec §8.5; REQ-APPLY-001, 002).

mod support;

use std::sync::Arc;
use std::time::{Duration, Instant};
use support::{DIRECT, Plane, config, proxied};
use tokio::net::TcpListener;
use veyra_core::Request;
use veyra_policy::Action;

#[tokio::test]
async fn an_apply_switches_the_gateway_then_dns_then_commits() {
    let plane = Plane::start("order", None).await;
    let staged = plane.stage(config(DIRECT)).await;
    assert!(staged.ok, "{staged:?}");
    assert_eq!(staged.staged, Some(2));

    let applied = plane.runtime.handle(Request::Activate { version: 2 }).await;
    assert!(applied.ok, "{applied:?}");
    assert_eq!(
        applied.steps,
        [
            "gateway: version 2",
            "dns: version 2",
            "verify: 2 listeners accept connections",
            "active: version 2"
        ]
    );
    assert_eq!((applied.active, applied.previous), (Some(2), Some(1)));
    assert_eq!(plane.dns_route("www.media.example"), Action::Direct {});

    // DNS stopped sending clients to the gateway; the gateway keeps serving them
    // until drainWindow (600 s) ends.
    let now = Instant::now();
    assert!(plane.gateway_passes("www.media.example", now));
    assert!(!plane.gateway_passes("www.media.example", now + Duration::from_secs(601)));
}

#[tokio::test]
async fn a_failed_verify_rolls_back_every_component() {
    let closed = TcpListener::bind("127.0.0.1:0").await.unwrap();
    let dead = closed.local_addr().unwrap();
    drop(closed);
    let plane = Plane::start("verify", Some(dead)).await;
    let before = plane.gateway.borrow().clone();
    assert!(plane.stage(config(DIRECT)).await.ok);

    let applied = plane.runtime.handle(Request::Activate { version: 2 }).await;
    assert!(!applied.ok, "{applied:?}");
    assert!(
        applied.messages[0].contains("does not accept connections"),
        "{applied:?}"
    );
    assert_eq!(
        applied.steps,
        [
            "gateway: version 2",
            "dns: version 2",
            "rollback dns: version 1",
            "rollback gateway: version 1"
        ]
    );
    assert_eq!(applied.active, Some(1));
    assert_eq!(plane.dns_route("www.media.example"), proxied());
    assert!(Arc::ptr_eq(&plane.gateway.borrow(), &before));
    let status = plane.runtime.handle(Request::Status).await;
    assert_eq!((status.active, status.previous), (Some(1), None));
}

#[tokio::test]
async fn rollback_returns_to_the_previous_version() {
    let plane = Plane::start("rollback", None).await;
    assert!(plane.stage(config(DIRECT)).await.ok);
    assert!(
        plane
            .runtime
            .handle(Request::Activate { version: 2 })
            .await
            .ok
    );

    let back = plane.runtime.handle(Request::Rollback).await;
    assert!(back.ok, "{back:?}");
    assert_eq!((back.active, back.previous), (Some(1), Some(2)));
    assert_eq!(plane.dns_route("www.media.example"), proxied());
}

#[tokio::test]
async fn staging_refuses_broken_configs_and_new_listeners() {
    let plane = Plane::start("refuse", None).await;
    let broken = plane.stage("spec: [".into()).await;
    assert!(!broken.ok && !broken.messages.is_empty(), "{broken:?}");
    let unknown_node = plane
        .stage(config("{ type: proxy_node, node: nowhere }"))
        .await;
    assert!(!unknown_node.ok, "{unknown_node:?}");
    assert!(
        unknown_node.messages[0].contains("UnknownReference"),
        "{unknown_node:?}"
    );
    let moved = plane
        .stage(config(DIRECT).replace("listen: [\"127.0.0.1:0\"]", "listen: [\"127.0.0.1:5300\"]"))
        .await;
    assert!(!moved.ok, "{moved:?}");
    assert!(moved.messages[0].contains("restart"), "{moved:?}");

    let staged = plane.stage(config(DIRECT)).await;
    assert_eq!(staged.staged, Some(2), "nothing refused was kept");
}

#[tokio::test]
async fn unknown_versions_and_a_missing_previous_change_nothing() {
    let plane = Plane::start("unknown", None).await;
    let missing = plane.runtime.handle(Request::Activate { version: 9 }).await;
    assert!(!missing.ok && missing.steps.is_empty(), "{missing:?}");
    let no_previous = plane.runtime.handle(Request::Rollback).await;
    assert!(!no_previous.ok, "{no_previous:?}");
    assert_eq!(plane.dns_route("www.media.example"), proxied());
}
````

In `crates/veyra-core/tests/support/mod.rs`, apply this patch (save it to a file and run `git apply <file>`, or edit by hand):

````diff
diff --git a/crates/veyra-core/tests/support/mod.rs b/crates/veyra-core/tests/support/mod.rs
index 68b9184..7cdb593 100644
--- a/crates/veyra-core/tests/support/mod.rs
+++ b/crates/veyra-core/tests/support/mod.rs
@@ -1,7 +1,15 @@
 //! Shared test helpers.
 #![allow(dead_code)] // each test file uses a different subset
 
+use std::net::SocketAddr;
 use std::path::PathBuf;
+use std::sync::Arc;
+use std::time::Instant;
+use tokio::sync::watch;
+use veyra_core::{Request, Response, Runtime, Store};
+use veyra_dns::Engine;
+use veyra_gateway::Gateway;
+use veyra_policy::{Action, Bundle, Name, Policy};
 
 /// A state directory of its own for one test, removed when the test ends.
 pub struct TempDir(pub PathBuf);
@@ -19,3 +27,107 @@ impl Drop for TempDir {
         let _ = std::fs::remove_dir_all(&self.0);
     }
 }
+
+pub const PROXY: &str = "{ type: proxy_node, node: de-01 }";
+pub const DIRECT: &str = "{ type: direct }";
+
+/// A config in which everyone sends `suffix:media.example` through `action`;
+/// DNS and the gateway listen on 127.0.0.1 with OS-chosen ports.
+pub fn config(action: &str) -> String {
+    format!(
+        r#"
+apiVersion: veyra.io/v1alpha1
+kind: PolicyBundle
+metadata: {{ name: runtime }}
+spec:
+  nodes: [{{ id: de-01, gateway: {{ ipv4: 203.0.113.10 }} }}]
+  profiles: [{{ id: home }}]
+  admission: [{{ cidr: 127.0.0.1/32, profile: home }}]
+  rules:
+    - {{ id: media, scope: default, match: {{ domains: ["suffix:media.example"] }}, action: {action} }}
+  dns:
+    listen: ["127.0.0.1:0"]
+    upstreams: [{{ id: default, servers: ["127.0.0.1:9"] }}]
+  gateway: {{ node: de-01, tls: ["127.0.0.1:0"], drainWindow: 600 }}
+"#
+    )
+}
+
+fn compile(text: &str) -> Policy {
+    let bundle = Bundle::from_yaml(text).expect("test YAML parses");
+    Policy::compile(&bundle).expect("test bundle compiles").0
+}
+
+/// A running data plane on version 1 (media proxied), plus what tests watch.
+pub struct Plane {
+    _dir: TempDir,
+    pub runtime: Runtime,
+    pub engine: watch::Receiver<Arc<Engine>>,
+    pub gateway: watch::Receiver<Arc<Gateway>>,
+}
+
+impl Plane {
+    /// `extra_probe` is one more address for VERIFY to check.
+    pub async fn start(name: &str, extra_probe: Option<SocketAddr>) -> Plane {
+        let dir = TempDir::new(name);
+        let store = Store::open(&dir.0).expect("state directory");
+        let text = config(PROXY);
+        store
+            .activate(store.stage(&text).expect("stage"))
+            .expect("activate");
+        let policy = compile(&text);
+        let dns = veyra_dns::Listeners::bind(&policy.dns().expect("dns").listen)
+            .await
+            .expect("bind DNS");
+        let gateway = veyra_gateway::Listeners::bind(policy.gateway_config().expect("gateway"))
+            .await
+            .expect("bind gateway");
+        let mut probes = dns.local_addrs().expect("DNS addresses");
+        probes.extend(
+            gateway
+                .local_addrs()
+                .expect("gateway addresses")
+                .iter()
+                .map(|(a, _)| *a),
+        );
+        probes.extend(extra_probe);
+        let engine = Engine::new(policy.clone()).expect("engine");
+        let (engines, engine_rx) = watch::channel(Arc::new(engine));
+        let first = Gateway::new(policy.clone()).expect("gateway");
+        let (gateways, gateway_rx) = watch::channel(Arc::new(first));
+        tokio::spawn(dns.serve(engine_rx.clone()));
+        tokio::spawn(gateway.serve(gateway_rx.clone()));
+        Plane {
+            _dir: dir,
+            runtime: Runtime::new(store, &policy, engines, Some(gateways), probes),
+            engine: engine_rx,
+            gateway: gateway_rx,
+        }
+    }
+
+    /// What DNS currently decides for `name` (home profile).
+    pub fn dns_route(&self, name: &str) -> Action {
+        let engine = self.engine.borrow().clone();
+        let policy = engine.policy();
+        let subject = policy.subject(Some("home"), None).expect("profile");
+        policy
+            .decide(&subject, &Name::parse(name).expect("name"))
+            .route
+    }
+
+    pub fn gateway_passes(&self, name: &str, at: Instant) -> bool {
+        let gateway = self.gateway.borrow().clone();
+        gateway.passes(&Name::parse(name).expect("name"), at)
+    }
+
+    pub async fn stage(&self, text: String) -> Response {
+        self.runtime.handle(Request::Stage { bundle: text }).await
+    }
+}
+
+pub fn proxied() -> Action {
+    Action::ProxyNode {
+        node: "de-01".into(),
+        on_failure: Default::default(),
+    }
+}
````

- [ ] **Step 2: Run the tests to see them fail**

Run: `cargo test -p veyra-core`
Expected: compilation fails: ``unresolved imports `veyra_core::Request`, `veyra_core::Response`, `veyra_core::Runtime` `` in `tests/support/mod.rs` (`store` stops compiling too, since it includes `support`) and ``unresolved import `veyra_core::Request` `` in `tests/runtime.rs`.

- [ ] **Step 3: Write the implementation**

In `crates/veyra-core/src/lib.rs`, apply this patch (save it to a file and run `git apply <file>`, or edit by hand):

````diff
diff --git a/crates/veyra-core/src/lib.rs b/crates/veyra-core/src/lib.rs
index 6355c9e..4146d8b 100644
--- a/crates/veyra-core/src/lib.rs
+++ b/crates/veyra-core/src/lib.rs
@@ -1,6 +1,10 @@
 //! Veyra runtime orchestration (spec §5.3, §8.5): the snapshot files the data
 //! plane starts from, ordered activation of new versions, and the control socket.
 
+mod protocol;
+mod runtime;
 mod store;
 
+pub use protocol::{Request, Response};
+pub use runtime::Runtime;
 pub use store::Store;
````

Create `crates/veyra-core/src/protocol.rs`:

````rust
//! What the control socket carries (spec §5.3): a request and its response.

use serde::{Deserialize, Serialize};

/// A control command.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "command", rename_all = "snake_case")]
pub enum Request {
    /// The active and previous versions.
    Status,
    /// Check a config and keep it as the next version, without activating it.
    Stage { bundle: String },
    /// Switch the data plane to a staged version in the order of spec §8.5.
    Activate { version: u64 },
    /// Switch back to the previous version the same way.
    Rollback,
}

/// The answer to a request: `steps` says what an activation did, in order;
/// `messages` carries config issues, warnings and errors.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct Response {
    pub ok: bool,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub active: Option<u64>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub previous: Option<u64>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub staged: Option<u64>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub steps: Vec<String>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub messages: Vec<String>,
}

impl Response {
    /// A failure with one message.
    pub fn failed(message: impl Into<String>) -> Response {
        Response {
            messages: vec![message.into()],
            ..Response::default()
        }
    }
}
````

Create `crates/veyra-core/src/runtime.rs`:

````rust
//! The running data plane's versions (spec §5.3, §8.5): stage a config, switch
//! the gateway and then DNS to it, verify, and roll back what was switched when
//! anything fails.

use crate::protocol::{Request, Response};
use crate::store::Store;
use std::net::{IpAddr, Ipv4Addr, Ipv6Addr, SocketAddr};
use std::sync::Arc;
use std::time::{Duration, Instant};
use tokio::net::TcpStream;
use tokio::sync::{Mutex, watch};
use tokio::time::timeout;
use veyra_dns::Engine;
use veyra_gateway::Gateway;
use veyra_policy::{Bundle, Policy};

/// How long VERIFY waits for each listener to accept a connection.
const VERIFY_TIMEOUT: Duration = Duration::from_secs(3);

const LISTEN_CHANGED: &str =
    "the config listens on other addresses than the running data plane: restart it instead";

/// The addresses a version listens on. Sockets are bound once, at start, so a
/// version that listens elsewhere needs a restart, not an apply.
#[derive(Debug, PartialEq, Eq)]
struct Listen {
    dns: Vec<SocketAddr>,
    gateway: Option<(Vec<SocketAddr>, Vec<SocketAddr>)>,
}

impl Listen {
    fn of(policy: &Policy) -> Listen {
        Listen {
            dns: policy.dns().map(|d| d.listen.clone()).unwrap_or_default(),
            gateway: policy
                .gateway_config()
                .map(|g| (g.tls.clone(), g.http.clone())),
        }
    }
}

/// The data plane's switchable parts: the DNS engine and the gateway its
/// listeners use, and the snapshot files behind them.
pub struct Runtime {
    store: Store,
    engine: watch::Sender<Arc<Engine>>,
    gateway: Option<watch::Sender<Arc<Gateway>>>,
    probes: Vec<SocketAddr>,
    listen: Listen,
    /// Requests are handled one at a time.
    busy: Mutex<()>,
}

impl Runtime {
    /// `policy` is the active version's; `probes` are the bound listener
    /// addresses VERIFY connects to.
    pub fn new(
        store: Store,
        policy: &Policy,
        engine: watch::Sender<Arc<Engine>>,
        gateway: Option<watch::Sender<Arc<Gateway>>>,
        probes: Vec<SocketAddr>,
    ) -> Runtime {
        Runtime {
            store,
            engine,
            gateway,
            probes,
            listen: Listen::of(policy),
            busy: Mutex::new(()),
        }
    }

    pub async fn handle(&self, request: Request) -> Response {
        let _one_at_a_time = self.busy.lock().await;
        match request {
            Request::Status => self.status(),
            Request::Stage { bundle } => self.stage(&bundle),
            Request::Activate { version } => self.activate(version).await,
            Request::Rollback => match self.store.previous() {
                Ok(Some(version)) => self.activate(version).await,
                Ok(None) => Response::failed("there is no previous version"),
                Err(e) => Response::failed(format!("state directory: {e}")),
            },
        }
    }

    fn status(&self) -> Response {
        match (self.store.active(), self.store.previous()) {
            (Ok(active), Ok(previous)) => Response {
                ok: true,
                active,
                previous,
                ..Response::default()
            },
            (Err(e), _) | (_, Err(e)) => Response::failed(format!("state directory: {e}")),
        }
    }

    fn stage(&self, bundle: &str) -> Response {
        let (policy, warnings) = match compile(bundle) {
            Ok(compiled) => compiled,
            Err(issues) => {
                return Response {
                    messages: issues,
                    ..Response::default()
                };
            }
        };
        if Listen::of(&policy) != self.listen {
            return Response::failed(LISTEN_CHANGED);
        }
        match self.store.stage(bundle) {
            Ok(version) => Response {
                ok: true,
                staged: Some(version),
                messages: warnings,
                ..Response::default()
            },
            Err(e) => Response::failed(format!("state directory: {e}")),
        }
    }

    async fn activate(&self, version: u64) -> Response {
        let current = self.store.active().ok().flatten();
        let mut steps = Vec::new();
        let policy = match self.store.read(version) {
            Ok(text) => match compile(&text) {
                Ok((policy, _warnings)) => policy,
                Err(issues) => return Response::failed(issues.join("; ")),
            },
            Err(e) => return Response::failed(format!("version {version}: {e}")),
        };
        if Listen::of(&policy) != self.listen {
            return Response::failed(LISTEN_CHANGED);
        }
        let engine = match Engine::new(policy.clone()) {
            Ok(engine) => Arc::new(engine),
            Err(e) => return Response::failed(e),
        };

        // Gateway first: it must accept the new names before DNS hands out its
        // address, and it keeps serving the removed ones for drainWindow.
        let mut old_gateway = None;
        if let Some(gateways) = &self.gateway {
            let old = gateways.borrow().clone();
            match Gateway::new(policy) {
                Ok(new) => gateways.send_replace(Arc::new(new.draining(&old, Instant::now()))),
                Err(e) => return Response::failed(e),
            };
            steps.push(format!("gateway: version {version}"));
            old_gateway = Some(old);
        }
        let old_engine = self.engine.send_replace(engine);
        steps.push(format!("dns: version {version}"));

        let outcome = match self.verify().await {
            Ok(()) => {
                let n = self.probes.len();
                steps.push(format!("verify: {n} listeners accept connections"));
                self.store
                    .activate(version)
                    .map_err(|e| format!("state directory: {e}"))
            }
            Err(e) => Err(e),
        };
        let was = current.map_or("the start-up config".into(), |v| format!("version {v}"));
        if let Err(e) = outcome {
            // Undo in reverse: DNS stops handing out the new answers first.
            self.engine.send_replace(old_engine);
            steps.push(format!("rollback dns: {was}"));
            if let (Some(gateways), Some(old)) = (&self.gateway, old_gateway) {
                gateways.send_replace(old);
                steps.push(format!("rollback gateway: {was}"));
            }
            tracing::warn!(version, error = %e, "activation failed, rolled back");
            return Response {
                active: current,
                steps,
                messages: vec![e],
                ..Response::default()
            };
        }
        steps.push(format!("active: version {version}"));
        tracing::info!(version, "activated");
        Response {
            ok: true,
            active: Some(version),
            previous: current,
            steps,
            ..Response::default()
        }
    }

    /// Spec §8.5 VERIFY: every listener still accepts connections.
    // ponytail: listeners are checked by a TCP connect; a canary DNS answer from
    // the new version needs an admission exception for the data plane's own probes.
    async fn verify(&self) -> Result<(), String> {
        for probe in &self.probes {
            let target = SocketAddr::new(reachable(probe.ip()), probe.port());
            if !matches!(
                timeout(VERIFY_TIMEOUT, TcpStream::connect(target)).await,
                Ok(Ok(_))
            ) {
                return Err(format!("verify: {probe} does not accept connections"));
            }
        }
        Ok(())
    }
}

/// A listener on the unspecified address is reached over loopback.
fn reachable(ip: IpAddr) -> IpAddr {
    match ip {
        IpAddr::V4(v4) if v4.is_unspecified() => Ipv4Addr::LOCALHOST.into(),
        IpAddr::V6(v6) if v6.is_unspecified() => Ipv6Addr::LOCALHOST.into(),
        ip => ip,
    }
}

/// Parse and check a config: the policy and its warnings, or every issue.
fn compile(text: &str) -> Result<(Policy, Vec<String>), Vec<String>> {
    let bundle = Bundle::from_yaml(text).map_err(|e| vec![e])?;
    let lines = |issues: &[veyra_policy::Issue]| issues.iter().map(ToString::to_string).collect();
    match Policy::compile(&bundle) {
        Ok((policy, warnings)) => Ok((policy, lines(&warnings))),
        Err(issues) => Err(lines(&issues)),
    }
}
````

- [ ] **Step 4: Run the tests to see them pass**

Run: `cargo test --workspace`
Expected: `runtime` `ok. 5 passed` in about 2 s (one test dials a closed port, which Windows takes 2 s to refuse); 164 in total.

- [ ] **Step 5: Format and lint**

Run: `cargo fmt --all && cargo clippy --workspace --all-targets -- -D warnings`
Expected: clippy prints no warnings.

Run: `cargo clippy --target aarch64-unknown-linux-gnu --workspace --all-targets -- -D warnings`
Expected: no warnings: the Unix-only code type-checks and lints too.

- [ ] **Step 6: Commit**

````bash
git add Cargo.lock crates/veyra-core/Cargo.toml crates/veyra-core/src/lib.rs crates/veyra-core/src/protocol.rs crates/veyra-core/src/runtime.rs crates/veyra-core/tests/runtime.rs crates/veyra-core/tests/support/mod.rs
git commit -m "feat(core): apply versions in order, verify, and roll back on failure"
````

---

### Task 6: The control socket

**Files:**
- Create: `crates/veyra-core/src/control.rs`, `crates/veyra-core/tests/control.rs`
- Modify: `crates/veyra-core/Cargo.toml`, `crates/veyra-core/src/lib.rs`

**Interfaces:**
- Consumes: `Request`, `Response`, `Runtime::handle` (Task 5).
- Produces: `pub mod control` in `veyra-core`. `MAX_REQUEST: u64 = 4 MiB`. `serve_connection(stream: impl AsyncRead + AsyncWrite, runtime: &Runtime) -> io::Result<()>`: one JSON request per line, one JSON response per line, until EOF. A line that does not parse gets `bad request: ...` and the connection goes on; a longer line than `MAX_REQUEST` gets a refusal and the connection is closed. `Client::new(stream)`, `Client::request(&mut self, &Request) -> io::Result<Response>`, and `Client::apply(&mut self, bundle: String)` (stage, then activate; staging warnings first in `messages`).
- Unix only (`#[cfg(unix)]`): `bind(path) -> io::Result<UnixListener>` (removes a stale file, socket mode 0660), `serve(listener, Arc<Runtime>)`, `connect(path) -> io::Result<Client<UnixStream>>`.

- [ ] **Step 1: Write the failing tests**

In `crates/veyra-core/Cargo.toml`, apply this patch (save it to a file and run `git apply <file>`, or edit by hand):

````diff
diff --git a/crates/veyra-core/Cargo.toml b/crates/veyra-core/Cargo.toml
index de38368..d937c29 100644
--- a/crates/veyra-core/Cargo.toml
+++ b/crates/veyra-core/Cargo.toml
@@ -10,6 +10,7 @@ veyra-policy.workspace = true
 veyra-dns.workspace = true
 veyra-gateway.workspace = true
 serde.workspace = true
+serde_json.workspace = true
 tokio.workspace = true
 tracing.workspace = true
 
````

Create `crates/veyra-core/tests/control.rs`:

````rust
//! The control socket's framing (spec §5.3): JSON lines in, JSON lines out.
//! The framing runs over an in-memory stream here; the Unix socket test runs
//! on Unix only.

mod support;

use support::{DIRECT, Plane, config};
use tokio::io::{AsyncBufReadExt, AsyncWriteExt, BufReader, duplex};
use veyra_core::control::{Client, MAX_REQUEST, serve_connection};
use veyra_core::{Request, Response};

#[tokio::test]
async fn status_and_apply_travel_as_json_lines() {
    let plane = Plane::start("control-apply", None).await;
    let (client_end, server_end) = duplex(64 * 1024);
    let client = async {
        let mut client = Client::new(client_end);
        let status = client.request(&Request::Status).await.expect("status");
        let applied = client.apply(config(DIRECT)).await.expect("apply");
        (status, applied)
    };
    let (served, (status, applied)) =
        tokio::join!(serve_connection(server_end, &plane.runtime), client);
    served.expect("the server ends cleanly when the client hangs up");
    assert_eq!((status.ok, status.active), (true, Some(1)));
    assert!(applied.ok, "{applied:?}");
    assert_eq!((applied.active, applied.previous), (Some(2), Some(1)));
    assert_eq!(
        applied.steps.last().map(String::as_str),
        Some("active: version 2")
    );
}

#[tokio::test]
async fn a_bad_line_gets_an_error_and_the_connection_goes_on() {
    let plane = Plane::start("control-bad", None).await;
    let (client_end, server_end) = duplex(64 * 1024);
    let client = async {
        let (read, mut write) = tokio::io::split(client_end);
        let mut lines = BufReader::new(read).lines();
        write
            .write_all(b"not json\n{\"command\":\"status\"}\n")
            .await
            .unwrap();
        let bad: Response =
            serde_json::from_str(&lines.next_line().await.unwrap().unwrap()).unwrap();
        let good: Response =
            serde_json::from_str(&lines.next_line().await.unwrap().unwrap()).unwrap();
        (bad, good)
    };
    let (_, (bad, good)) = tokio::join!(serve_connection(server_end, &plane.runtime), client);
    assert!(
        !bad.ok && bad.messages[0].starts_with("bad request"),
        "{bad:?}"
    );
    assert!(good.ok, "{good:?}");
}

#[tokio::test]
async fn an_oversized_request_is_refused_and_the_connection_closed() {
    let plane = Plane::start("control-big", None).await;
    let (client_end, server_end) = duplex(64 * 1024);
    let client = async {
        let (read, mut write) = tokio::io::split(client_end);
        let huge = vec![b'a'; usize::try_from(MAX_REQUEST).unwrap() + 1];
        let _ = write.write_all(&huge).await;
        let mut lines = BufReader::new(read).lines();
        let refused = lines.next_line().await.unwrap().unwrap();
        let after = lines.next_line().await.unwrap();
        (refused, after)
    };
    let (served, (refused, after)) =
        tokio::join!(serve_connection(server_end, &plane.runtime), client);
    served.unwrap();
    let refused: Response = serde_json::from_str(&refused).unwrap();
    assert!(
        !refused.ok && refused.messages[0].contains("longer than"),
        "{refused:?}"
    );
    assert_eq!(after, None, "the server closed the connection");
}

#[cfg(unix)]
#[tokio::test]
async fn the_unix_socket_is_private_to_the_data_plane_and_replaces_a_stale_one() {
    use std::os::unix::fs::PermissionsExt;
    use std::sync::Arc;
    use veyra_core::control::{bind, connect, serve};

    let plane = Plane::start("control-unix", None).await;
    let dir = support::TempDir::new("control-unix-socket");
    std::fs::create_dir_all(&dir.0).unwrap();
    let path = dir.0.join("control.sock");
    std::fs::write(&path, "left by a crash").unwrap();

    let listener = bind(&path).expect("bind over the stale file");
    let mode = std::fs::metadata(&path).unwrap().permissions().mode() & 0o777;
    assert_eq!(mode, 0o660);
    let Plane { runtime, .. } = plane;
    tokio::spawn(serve(listener, Arc::new(runtime)));
    let mut client = connect(&path).await.expect("connect");
    let status = client.request(&Request::Status).await.expect("status");
    assert_eq!(status.active, Some(1));
}
````

- [ ] **Step 2: Run the tests to see them fail**

Run: `cargo test -p veyra-core --test control`
Expected: compilation fails: ``unresolved import `veyra_core::control` ``, with follow-on type errors in the same file.

- [ ] **Step 3: Write the implementation**

Create `crates/veyra-core/src/control.rs`:

````rust
//! The control socket (spec §5.3): one JSON request per line, one JSON response
//! per line. The framing works over any stream; on Unix the data plane serves
//! it on a socket file that only its user and group may open.

use crate::protocol::{Request, Response};
use crate::runtime::Runtime;
use std::io;
use tokio::io::{
    AsyncBufReadExt, AsyncRead, AsyncReadExt, AsyncWrite, AsyncWriteExt, BufReader, ReadHalf,
    WriteHalf,
};

/// The longest request line accepted. Configs are far smaller.
pub const MAX_REQUEST: u64 = 4 * 1024 * 1024;

/// Answer requests on one connection until the client closes it.
pub async fn serve_connection<S: AsyncRead + AsyncWrite>(
    stream: S,
    runtime: &Runtime,
) -> io::Result<()> {
    let (read, mut write) = tokio::io::split(stream);
    let mut lines = BufReader::new(read);
    loop {
        let mut line = String::new();
        if (&mut lines)
            .take(MAX_REQUEST + 1)
            .read_line(&mut line)
            .await?
            == 0
        {
            return Ok(());
        }
        if line.len() as u64 > MAX_REQUEST {
            // The rest of the line cannot be told from the next request: stop.
            let refused = Response::failed(format!("request longer than {MAX_REQUEST} bytes"));
            return send(&mut write, &refused).await;
        }
        let response = match serde_json::from_str::<Request>(&line) {
            Ok(request) => runtime.handle(request).await,
            Err(e) => Response::failed(format!("bad request: {e}")),
        };
        send(&mut write, &response).await?;
    }
}

async fn send(
    write: &mut (impl AsyncWrite + Unpin),
    message: &impl serde::Serialize,
) -> io::Result<()> {
    let mut line = serde_json::to_string(message).map_err(io::Error::other)?;
    line.push('\n');
    write.write_all(line.as_bytes()).await?;
    write.flush().await
}

/// The client side: what `veyra apply`, `rollback` and `status` use.
pub struct Client<S> {
    lines: BufReader<ReadHalf<S>>,
    write: WriteHalf<S>,
}

impl<S: AsyncRead + AsyncWrite> Client<S> {
    pub fn new(stream: S) -> Client<S> {
        let (read, write) = tokio::io::split(stream);
        Client {
            lines: BufReader::new(read),
            write,
        }
    }

    /// Send one request and read its response.
    pub async fn request(&mut self, request: &Request) -> io::Result<Response> {
        send(&mut self.write, request).await?;
        let mut line = String::new();
        if self.lines.read_line(&mut line).await? == 0 {
            let msg = "the data plane closed the control connection";
            return Err(io::Error::new(io::ErrorKind::UnexpectedEof, msg));
        }
        serde_json::from_str(&line).map_err(|e| io::Error::new(io::ErrorKind::InvalidData, e))
    }

    /// Stage `bundle`, then activate it: `veyra apply`. The staging warnings
    /// come first in the response's messages.
    pub async fn apply(&mut self, bundle: String) -> io::Result<Response> {
        let staged = self.request(&Request::Stage { bundle }).await?;
        let Some(version) = staged.staged.filter(|_| staged.ok) else {
            return Ok(staged);
        };
        let mut applied = self.request(&Request::Activate { version }).await?;
        applied.messages.splice(0..0, staged.messages);
        Ok(applied)
    }
}

/// Bind the control socket at `path`, replacing a file a crash left there, and
/// let only the data plane's user and group open it.
#[cfg(unix)]
pub fn bind(path: &std::path::Path) -> io::Result<tokio::net::UnixListener> {
    use std::os::unix::fs::PermissionsExt;
    match std::fs::remove_file(path) {
        Err(e) if e.kind() != io::ErrorKind::NotFound => return Err(e),
        _ => {}
    }
    let listener = tokio::net::UnixListener::bind(path)?;
    std::fs::set_permissions(path, std::fs::Permissions::from_mode(0o660))?;
    Ok(listener)
}

/// Serve control connections until the future is dropped.
#[cfg(unix)]
pub async fn serve(listener: tokio::net::UnixListener, runtime: std::sync::Arc<Runtime>) {
    loop {
        match listener.accept().await {
            Ok((stream, _)) => {
                let runtime = runtime.clone();
                tokio::spawn(async move {
                    if let Err(e) = serve_connection(stream, &runtime).await {
                        tracing::debug!(error = %e, "control connection ended");
                    }
                });
            }
            Err(e) => {
                tracing::warn!(error = %e, "control accept failed");
                tokio::time::sleep(std::time::Duration::from_millis(100)).await;
            }
        }
    }
}

/// Connect to the data plane's control socket at `path`.
#[cfg(unix)]
pub async fn connect(path: &std::path::Path) -> io::Result<Client<tokio::net::UnixStream>> {
    Ok(Client::new(tokio::net::UnixStream::connect(path).await?))
}
````

In `crates/veyra-core/src/lib.rs`, apply this patch (save it to a file and run `git apply <file>`, or edit by hand):

````diff
diff --git a/crates/veyra-core/src/lib.rs b/crates/veyra-core/src/lib.rs
index 4146d8b..c22d3a5 100644
--- a/crates/veyra-core/src/lib.rs
+++ b/crates/veyra-core/src/lib.rs
@@ -1,6 +1,7 @@
 //! Veyra runtime orchestration (spec §5.3, §8.5): the snapshot files the data
 //! plane starts from, ordered activation of new versions, and the control socket.
 
+pub mod control;
 mod protocol;
 mod runtime;
 mod store;
````

- [ ] **Step 4: Run the tests to see them pass**

Run: `cargo test --workspace`
Expected: `control` `ok. 3 passed` (the Unix socket test is compiled only on Unix); 167 in total.

- [ ] **Step 5: Format and lint**

Run: `cargo fmt --all && cargo clippy --workspace --all-targets -- -D warnings`
Expected: clippy prints no warnings.

Run: `cargo clippy --target aarch64-unknown-linux-gnu --workspace --all-targets -- -D warnings`
Expected: no warnings: the Unix-only code type-checks and lints too.

- [ ] **Step 6: Commit**

````bash
git add Cargo.lock crates/veyra-core/Cargo.toml crates/veyra-core/src/control.rs crates/veyra-core/src/lib.rs crates/veyra-core/tests/control.rs
git commit -m "feat(core): control socket with JSON lines and a client for apply"
````

---

### Task 7: `veyra-dataplane` from snapshots; `veyra apply`, `rollback`, `status`

**Files:**
- Modify: `Cargo.toml`, `apps/veyra-cli/Cargo.toml`, `apps/veyra-cli/src/main.rs`, `apps/veyra-cli/tests/cli.rs`, `apps/veyra-dataplane/Cargo.toml`, `apps/veyra-dataplane/src/main.rs`, `apps/veyra-dataplane/tests/dataplane.rs`

**Interfaces:**
- Consumes: `Store`, `Runtime` (Tasks 4–5), `control::{bind, serve, connect}` (Task 6), the watch channels (Task 3).
- Produces: `veyra-dataplane --state-dir <dir> [--config <file>] [--control <socket>]`. `--config` alone behaves as before. With `--state-dir`, a valid `--config` that differs from the active snapshot becomes the next version at start (how listen addresses change). A missing or broken one falls back to the active snapshot, and an empty state directory without `--config` is an error. `--control` needs `--state-dir` and Unix; elsewhere it fails with "the control socket needs Unix". After the DNS and gateway lines it prints `listening on <socket> (control)`.
- `veyra apply <config> [--control <socket>]`, `veyra rollback`, `veyra status` (default socket `/run/veyra/control.sock`): steps go to stdout, messages to stderr, and a refusal exits 1. Without Unix they say "the control socket needs Unix"; without a running data plane, "cannot reach the data plane at <path>".
- `veyra-dataplane` and `veyra-cli` depend on `veyra-core` (and the CLI on tokio). The e2e helper `start_with(args, lines)` replaces the body of `start`; `ask(server, name)` and `blocked(reply)` query over UDP; `StateDir` is a temporary state directory.

- [ ] **Step 1: Write the failing tests**

In `Cargo.toml`, apply this patch (save it to a file and run `git apply <file>`, or edit by hand):

````diff
diff --git a/Cargo.toml b/Cargo.toml
index 75d29c9..1c3c4c7 100644
--- a/Cargo.toml
+++ b/Cargo.toml
@@ -12,6 +12,7 @@ publish = false
 veyra-policy = { path = "crates/veyra-policy" }
 veyra-dns = { path = "crates/veyra-dns" }
 veyra-gateway = { path = "crates/veyra-gateway" }
+veyra-core = { path = "crates/veyra-core" }
 serde = { version = "1.0.229", features = ["derive"] }
 serde-saphyr = "1.3.0"
 serde_json = "1.0.154"
````

In `apps/veyra-cli/tests/cli.rs`, apply this patch (save it to a file and run `git apply <file>`, or edit by hand):

````diff
diff --git a/apps/veyra-cli/tests/cli.rs b/apps/veyra-cli/tests/cli.rs
index a1f94db..58be0c4 100644
--- a/apps/veyra-cli/tests/cli.rs
+++ b/apps/veyra-cli/tests/cli.rs
@@ -246,3 +246,35 @@ fn explain_shows_the_dns_answer_in_dns_mode() {
     let tunnel = run(&["--profile", "home", "--mode", "tunnel", "media.example"]);
     assert!(tunnel.get("dns_answer").is_none(), "{tunnel}");
 }
+
+#[cfg(unix)]
+#[test]
+fn control_commands_say_when_no_data_plane_is_running() {
+    let socket = std::env::temp_dir().join(format!("veyra-cli-{}-none.sock", std::process::id()));
+    let socket = socket.to_str().expect("UTF-8 temp path");
+    for args in [
+        vec!["status", "--control", socket],
+        vec!["rollback", "--control", socket],
+        vec!["apply", EXAMPLE, "--control", socket],
+    ] {
+        let out = veyra(&args);
+        assert_eq!(out.status.code(), Some(1), "{args:?}");
+        let err = text(&out.stderr);
+        assert!(
+            err.contains("cannot reach the data plane"),
+            "{args:?}: {err}"
+        );
+    }
+}
+
+#[cfg(not(unix))]
+#[test]
+fn control_commands_need_unix() {
+    let out = veyra(&["status", "--control", "control.sock"]);
+    assert_eq!(out.status.code(), Some(1));
+    assert!(
+        text(&out.stderr).contains("needs Unix"),
+        "{}",
+        text(&out.stderr)
+    );
+}
````

In `apps/veyra-dataplane/Cargo.toml`, apply this patch (save it to a file and run `git apply <file>`, or edit by hand):

````diff
diff --git a/apps/veyra-dataplane/Cargo.toml b/apps/veyra-dataplane/Cargo.toml
index e636beb..fcf6176 100644
--- a/apps/veyra-dataplane/Cargo.toml
+++ b/apps/veyra-dataplane/Cargo.toml
@@ -9,6 +9,7 @@ publish.workspace = true
 veyra-policy.workspace = true
 veyra-dns.workspace = true
 veyra-gateway.workspace = true
+veyra-core.workspace = true
 clap.workspace = true
 tokio.workspace = true
 tracing.workspace = true
````

In `apps/veyra-dataplane/tests/dataplane.rs`, apply this patch (save it to a file and run `git apply <file>`, or edit by hand):

````diff
diff --git a/apps/veyra-dataplane/tests/dataplane.rs b/apps/veyra-dataplane/tests/dataplane.rs
index e1aeb4e..f1c80f8 100644
--- a/apps/veyra-dataplane/tests/dataplane.rs
+++ b/apps/veyra-dataplane/tests/dataplane.rs
@@ -2,6 +2,7 @@
 
 use hickory_proto::op::{Message, MessageType, OpCode, Query, ResponseCode};
 use hickory_proto::rr::{Name, RData, RecordType};
+use std::ffi::OsStr;
 use std::io::{BufRead, BufReader, ErrorKind, Read, Write};
 use std::net::{Ipv4Addr, SocketAddr, TcpStream, UdpSocket};
 use std::path::PathBuf;
@@ -46,8 +47,13 @@ fn write_config(name: &str, text: &str) -> PathBuf {
 /// read its first `lines` "listening on <addr> (...)" lines.
 fn start(name: &str, config: &str, lines: usize) -> (Server, Vec<SocketAddr>) {
     let path = write_config(name, config);
+    start_with(&["--config".as_ref(), path.as_os_str()], lines)
+}
+
+/// Start the binary with `args` and read its first `lines` listening lines.
+fn start_with(args: &[&OsStr], lines: usize) -> (Server, Vec<SocketAddr>) {
     let mut child = Command::new(env!("CARGO_BIN_EXE_veyra-dataplane"))
-        .args(["--config".as_ref(), path.as_os_str()])
+        .args(args)
         .stdout(Stdio::piped())
         .stderr(Stdio::null())
         .spawn()
@@ -88,6 +94,39 @@ fn query(name: &str, record_type: RecordType) -> Vec<u8> {
     message.to_vec().expect("encode")
 }
 
+/// Ask `server` for the A record of `name` over UDP.
+fn ask(server: SocketAddr, name: &str) -> Message {
+    let udp = UdpSocket::bind("127.0.0.1:0").expect("bind client");
+    udp.set_read_timeout(Some(Duration::from_secs(5)))
+        .expect("timeout");
+    udp.send_to(&query(name, RecordType::A), server)
+        .expect("send");
+    let mut buf = [0; 4096];
+    let len = udp.recv(&mut buf).expect("UDP reply");
+    Message::from_vec(&buf[..len]).expect("valid reply")
+}
+
+fn blocked(reply: &Message) -> bool {
+    matches!(&reply.answers[..], [record] if matches!(&record.data, RData::A(a) if a.0 == Ipv4Addr::UNSPECIFIED))
+}
+
+/// A state directory of its own for one test, removed when the test ends.
+struct StateDir(PathBuf);
+
+impl StateDir {
+    fn new(name: &str) -> StateDir {
+        let path = std::env::temp_dir().join(format!("veyra-e2e-{}-{name}", std::process::id()));
+        let _ = std::fs::remove_dir_all(&path);
+        StateDir(path)
+    }
+}
+
+impl Drop for StateDir {
+    fn drop(&mut self) {
+        let _ = std::fs::remove_dir_all(&self.0);
+    }
+}
+
 #[test]
 fn serves_synthesized_answers_over_udp_and_tcp() {
     let (_server, addrs) = start("dns", CONFIG, 1);
@@ -180,3 +219,144 @@ fn the_gateway_refuses_unrouted_names_and_rejects_udp() {
         "{err:?}"
     );
 }
+
+#[test]
+fn restarts_from_the_active_snapshot_without_the_config_file() {
+    // REQ-APPLY-003: DNS and the gateway start from the snapshot file alone.
+    let state = StateDir::new("restart");
+    let config = write_config("seed", CONFIG);
+    let seed = [
+        "--state-dir".as_ref(),
+        state.0.as_os_str(),
+        "--config".as_ref(),
+        config.as_os_str(),
+    ];
+    {
+        let (_server, addrs) = start_with(&seed, 1);
+        assert!(blocked(&ask(addrs[0], "ads.example.")));
+    }
+    std::fs::remove_file(&config).expect("remove the config file");
+    let (_server, addrs) = start_with(&["--state-dir".as_ref(), state.0.as_os_str()], 1);
+    assert!(blocked(&ask(addrs[0], "ads.example.")));
+    let active = std::fs::read_to_string(state.0.join("active")).expect("active pointer");
+    assert_eq!(active.trim(), "1");
+}
+
+#[test]
+fn an_empty_state_directory_needs_a_config() {
+    let state = StateDir::new("empty");
+    let out = Command::new(env!("CARGO_BIN_EXE_veyra-dataplane"))
+        .args(["--state-dir".as_ref(), state.0.as_os_str()])
+        .output()
+        .expect("run veyra-dataplane");
+    assert_eq!(out.status.code(), Some(1));
+    let err = String::from_utf8_lossy(&out.stderr);
+    assert!(err.contains("no active snapshot"), "{err}");
+}
+
+#[cfg(not(unix))]
+#[test]
+fn the_control_socket_needs_unix() {
+    let state = StateDir::new("control");
+    let config = write_config("control", CONFIG);
+    let out = Command::new(env!("CARGO_BIN_EXE_veyra-dataplane"))
+        .args([
+            "--state-dir".as_ref(),
+            state.0.as_os_str(),
+            "--config".as_ref(),
+            config.as_os_str(),
+            "--control".as_ref(),
+            "control.sock".as_ref(),
+        ])
+        .output()
+        .expect("run veyra-dataplane");
+    assert_eq!(out.status.code(), Some(1));
+    let err = String::from_utf8_lossy(&out.stderr);
+    assert!(err.contains("needs Unix"), "{err}");
+}
+
+#[cfg(unix)]
+#[tokio::test]
+async fn apply_and_rollback_switch_the_running_data_plane() {
+    let state = StateDir::new("apply");
+    let config = write_config("apply", CONFIG);
+    let socket = state.0.join("control.sock");
+    let (_server, addrs) = start_with(
+        &[
+            "--state-dir".as_ref(),
+            state.0.as_os_str(),
+            "--config".as_ref(),
+            config.as_os_str(),
+            "--control".as_ref(),
+            socket.as_os_str(),
+        ],
+        1,
+    );
+    assert!(blocked(&ask(addrs[0], "ads.example.")));
+
+    let mut client = None;
+    for _ in 0..50 {
+        if let Ok(connected) = veyra_core::control::connect(&socket).await {
+            client = Some(connected);
+            break;
+        }
+        tokio::time::sleep(Duration::from_millis(100)).await;
+    }
+    let mut client = client.expect("the control socket accepts within 5 s");
+    let applied = client.apply(unblocked()).await.expect("apply");
+    assert!(applied.ok, "{applied:?}");
+    assert!(!blocked(&ask(addrs[0], "ads.example.")));
+
+    let back = client
+        .request(&veyra_core::Request::Rollback)
+        .await
+        .expect("rollback");
+    assert!(back.ok, "{back:?}");
+    assert!(blocked(&ask(addrs[0], "ads.example.")));
+}
+
+/// CONFIG with the ads rule sending ads.example direct instead of blocking it.
+fn unblocked() -> String {
+    let text = CONFIG.replace(
+        "suffix:ads.example\"] }, action: { type: block }",
+        "suffix:ads.example\"] }, action: { type: direct }",
+    );
+    assert_ne!(text, CONFIG, "the ads rule was rewritten");
+    text
+}
+
+#[test]
+fn a_changed_config_file_becomes_the_next_version_at_start() {
+    let state = StateDir::new("changed");
+    let config = write_config("changed", CONFIG);
+    let args = [
+        "--state-dir".as_ref(),
+        state.0.as_os_str(),
+        "--config".as_ref(),
+        config.as_os_str(),
+    ];
+    drop(start_with(&args, 1));
+    std::fs::write(&config, unblocked()).expect("edit the config file");
+    let (_server, addrs) = start_with(&args, 1);
+    assert!(!blocked(&ask(addrs[0], "ads.example.")));
+    let active = std::fs::read_to_string(state.0.join("active")).expect("active pointer");
+    assert_eq!(active.trim(), "2");
+}
+
+#[test]
+fn a_broken_config_file_falls_back_to_the_active_snapshot() {
+    let state = StateDir::new("broken");
+    let config = write_config("broken", CONFIG);
+    let args = [
+        "--state-dir".as_ref(),
+        state.0.as_os_str(),
+        "--config".as_ref(),
+        config.as_os_str(),
+    ];
+    drop(start_with(&args, 1));
+    std::fs::write(&config, "spec: [").expect("break the config file");
+    let (_server, addrs) = start_with(&args, 1);
+    assert!(blocked(&ask(addrs[0], "ads.example.")));
+    let active = std::fs::read_to_string(state.0.join("active")).expect("active pointer");
+    assert_eq!(active.trim(), "1");
+}
````

- [ ] **Step 2: Run the tests to see them fail**

Run: `cargo test -p veyra-dataplane`
Expected: `test result: FAILED. 3 passed; 5 failed`: `restarts_from_the_active_snapshot_without_the_config_file`, `a_changed_config_file_becomes_the_next_version_at_start`, `a_broken_config_file_falls_back_to_the_active_snapshot`, `an_empty_state_directory_needs_a_config` and `the_control_socket_needs_unix` (the binary rejects `--state-dir`).

Run: `cargo test -p veyra-cli`
Expected: `control_commands_need_unix ... FAILED`; `test result: FAILED. 11 passed; 1 failed`. (On Unix it is `control_commands_say_when_no_data_plane_is_running` that fails.)

- [ ] **Step 3: Write the implementation**

In `apps/veyra-cli/Cargo.toml`, apply this patch (save it to a file and run `git apply <file>`, or edit by hand):

````diff
diff --git a/apps/veyra-cli/Cargo.toml b/apps/veyra-cli/Cargo.toml
index c45cd15..05ba737 100644
--- a/apps/veyra-cli/Cargo.toml
+++ b/apps/veyra-cli/Cargo.toml
@@ -11,6 +11,8 @@ path = "src/main.rs"
 
 [dependencies]
 veyra-policy.workspace = true
+veyra-core.workspace = true
+tokio.workspace = true
 clap.workspace = true
 serde.workspace = true
 serde_json.workspace = true
````

In `apps/veyra-cli/src/main.rs`, apply this patch (save it to a file and run `git apply <file>`, or edit by hand):

````diff
diff --git a/apps/veyra-cli/src/main.rs b/apps/veyra-cli/src/main.rs
index c8a39b3..2939a32 100644
--- a/apps/veyra-cli/src/main.rs
+++ b/apps/veyra-cli/src/main.rs
@@ -1,10 +1,12 @@
-//! `veyra` command line, stage 1a (spec §23.2): validate and explain a policy file.
+//! `veyra` command line, stage 1a (spec §23.2): validate and explain a policy
+//! file, and apply it to the running data plane.
 
 use clap::{Parser, Subcommand};
 use serde::Serialize;
 use std::net::IpAddr;
 use std::path::{Path, PathBuf};
 use std::process::ExitCode;
+use veyra_core::{Request, Response};
 use veyra_policy::{Bundle, Decision, DnsPlan, Issue, Mode, Name, Policy, QueryKind};
 
 #[derive(Parser)]
@@ -34,8 +36,28 @@ enum Command {
         mode: Mode,
         domain: String,
     },
+    /// Check a config and switch the running data plane to it (spec §8.5).
+    Apply {
+        config: PathBuf,
+        /// The data plane's control socket.
+        #[arg(long, default_value = CONTROL)]
+        control: PathBuf,
+    },
+    /// Switch the running data plane back to the previous version.
+    Rollback {
+        #[arg(long, default_value = CONTROL)]
+        control: PathBuf,
+    },
+    /// Show the running data plane's active and previous versions.
+    Status {
+        #[arg(long, default_value = CONTROL)]
+        control: PathBuf,
+    },
 }
 
+/// Where the systemd unit puts the control socket (deploy/systemd).
+const CONTROL: &str = "/run/veyra/control.sock";
+
 /// The `veyra explain` JSON: the decision plus the inputs it was made for.
 /// Fields are only ever added during stage 1a, never renamed.
 #[derive(Serialize)]
@@ -159,6 +181,64 @@ fn run(cli: Cli) -> Result<(), String> {
             println!("{json}");
             Ok(())
         }
+        Command::Apply { config, control } => {
+            let bundle = std::fs::read_to_string(&config)
+                .map_err(|e| format!("{}: {e}", config.display()))?;
+            report(&call(&control, Call::Apply(bundle))?)
+        }
+        Command::Rollback { control } => report(&call(&control, Call::One(Request::Rollback))?),
+        Command::Status { control } => {
+            let status = call(&control, Call::One(Request::Status))?;
+            let show = |v: Option<u64>| v.map_or("none".into(), |v| format!("version {v}"));
+            println!("active: {}", show(status.active));
+            println!("previous: {}", show(status.previous));
+            report(&status)
+        }
+    }
+}
+
+/// What to send over the control socket.
+#[cfg_attr(not(unix), allow(dead_code))] // only Unix has the socket
+enum Call {
+    /// Stage a config, then activate it.
+    Apply(String),
+    One(Request),
+}
+
+#[cfg(unix)]
+fn call(path: &Path, call: Call) -> Result<Response, String> {
+    let runtime = tokio::runtime::Builder::new_current_thread()
+        .enable_all()
+        .build()
+        .map_err(|e| e.to_string())?;
+    runtime.block_on(async {
+        let mut client = veyra_core::control::connect(path)
+            .await
+            .map_err(|e| format!("cannot reach the data plane at {}: {e}", path.display()))?;
+        let response = match call {
+            Call::Apply(bundle) => client.apply(bundle).await,
+            Call::One(request) => client.request(&request).await,
+        };
+        response.map_err(|e| format!("{}: {e}", path.display()))
+    })
+}
+
+#[cfg(not(unix))]
+fn call(path: &Path, _call: Call) -> Result<Response, String> {
+    Err(format!(
+        "{}: the control socket needs Unix; run veyra on the Linux host",
+        path.display()
+    ))
+}
+
+/// Print what the data plane did and said; a refusal is an error.
+fn report(response: &Response) -> Result<(), String> {
+    response.steps.iter().for_each(|step| println!("{step}"));
+    response.messages.iter().for_each(|m| eprintln!("{m}"));
+    if response.ok {
+        Ok(())
+    } else {
+        Err("the data plane refused the request".into())
     }
 }
 
````

In `apps/veyra-dataplane/src/main.rs`, apply this patch (save it to a file and run `git apply <file>`, or edit by hand):

````diff
diff --git a/apps/veyra-dataplane/src/main.rs b/apps/veyra-dataplane/src/main.rs
index 42fc689..491a1b8 100644
--- a/apps/veyra-dataplane/src/main.rs
+++ b/apps/veyra-dataplane/src/main.rs
@@ -1,12 +1,15 @@
 //! `veyra-dataplane`: serves DNS and, with a `gateway:` section, the gateway
-//! for a policy file (spec §5.2, §9, §10).
+//! for a policy (spec §5.2, §9, §10). With `--state-dir` it starts from the
+//! active snapshot, and with `--control` it takes new versions from
+//! `veyra apply` (spec §5.3, §8.5).
 
 use clap::Parser;
-use std::path::PathBuf;
+use std::path::{Path, PathBuf};
 use std::process::ExitCode;
 use std::sync::Arc;
 use tokio::sync::watch;
 use tracing_subscriber::EnvFilter;
+use veyra_core::{Runtime, Store};
 use veyra_dns::{Engine, Listeners};
 use veyra_gateway::{Gateway, Service};
 use veyra_policy::{Bundle, Policy};
@@ -18,10 +21,17 @@ use veyra_policy::{Bundle, Policy};
     about = "Veyra data plane: DNS and gateway"
 )]
 struct Args {
-    /// The policy file; it must have `dns:` and `admission:` sections, and
-    /// `gateway:` to serve the gateway too.
+    /// The policy file: it needs `dns:` and `admission:` sections, and
+    /// `gateway:` to serve the gateway too. With `--state-dir`, a valid file
+    /// that differs from the active snapshot becomes the next version at start.
+    #[arg(long, required_unless_present = "state_dir")]
+    config: Option<PathBuf>,
+    /// Where the snapshots are kept; the data plane starts from the active one.
     #[arg(long)]
-    config: PathBuf,
+    state_dir: Option<PathBuf>,
+    /// The control socket for `veyra apply`, `rollback` and `status` (Unix only).
+    #[arg(long, requires = "state_dir")]
+    control: Option<PathBuf>,
 }
 
 #[tokio::main]
@@ -43,19 +53,8 @@ async fn main() -> ExitCode {
 }
 
 async fn run(args: Args) -> Result<(), String> {
-    let path = args.config.display();
-    let text = std::fs::read_to_string(&args.config).map_err(|e| format!("{path}: {e}"))?;
-    let bundle = Bundle::from_yaml(&text).map_err(|e| format!("{path}: {e}"))?;
-    let policy = match Policy::compile(&bundle) {
-        Ok((policy, warnings)) => {
-            warnings.iter().for_each(|w| tracing::warn!("{w}"));
-            policy
-        }
-        Err(issues) => {
-            issues.iter().for_each(|i| eprintln!("{i}"));
-            return Err(format!("{path}: has errors, run `veyra validate`"));
-        }
-    };
+    let (origin, text, store) = load(&args)?;
+    let policy = compile(&text, &origin)?;
     let listen = policy
         .dns()
         .map(|dns| dns.listen.clone())
@@ -63,19 +62,21 @@ async fn run(args: Args) -> Result<(), String> {
     // The gateway reads the same policy: DNS hands out its address (spec §10.2).
     let gateway = match policy.gateway_config().cloned() {
         Some(config) => {
-            let gateway = Gateway::new(policy.clone()).map_err(|e| format!("{path}: {e}"))?;
+            let gateway = Gateway::new(policy.clone()).map_err(|e| format!("{origin}: {e}"))?;
             Some((config, gateway))
         }
         None => None,
     };
-    let engine = Arc::new(Engine::new(policy).map_err(|e| format!("{path}: {e}"))?);
+    let engine = Engine::new(policy.clone()).map_err(|e| format!("{origin}: {e}"))?;
     let listeners = Listeners::bind(&listen)
         .await
         .map_err(|e| format!("cannot listen on {listen:?}: {e}"))?;
-    for addr in listeners.local_addrs().map_err(|e| e.to_string())? {
+    let mut probes = listeners.local_addrs().map_err(|e| e.to_string())?;
+    for addr in &probes {
         println!("listening on {addr} (udp, tcp)");
     }
-    let mut gateway_listeners = None;
+    let (engines, engine) = watch::channel(Arc::new(engine));
+    let (mut gateways, mut gateway_listeners) = (None, None);
     if let Some((config, gateway)) = gateway {
         let bound = veyra_gateway::Listeners::bind(&config)
             .await
@@ -86,21 +87,126 @@ async fn run(args: Args) -> Result<(), String> {
                 Service::Http => "http",
             };
             println!("listening on {addr} ({service})");
+            probes.push(addr);
         }
-        gateway_listeners = Some(bound.serve(watch::channel(Arc::new(gateway)).1));
+        let (sender, current) = watch::channel(Arc::new(gateway));
+        gateways = Some(sender);
+        gateway_listeners = Some(bound.serve(current));
     }
+    let control = match (store, &args.control) {
+        (Some(store), Some(path)) => {
+            let runtime = Runtime::new(store, &policy, engines, gateways, probes);
+            Some(control(path, Arc::new(runtime))?)
+        }
+        _ => None,
+    };
     let gateway = async {
         match gateway_listeners {
             Some(serve) => serve.await,
             None => std::future::pending().await,
         }
     };
+    let control = async {
+        match control {
+            Some(serve) => serve.await,
+            None => std::future::pending().await,
+        }
+    };
     tokio::select! {
-        () = listeners.serve(watch::channel(engine).1) => Ok(()),
+        () = listeners.serve(engine) => Ok(()),
         () = gateway => Ok(()),
+        () = control => Ok(()),
         _ = tokio::signal::ctrl_c() => {
             tracing::info!("shutting down");
             Ok(())
         }
     }
 }
+
+/// Where the config comes from (for messages), its text, and the snapshot
+/// store when there is one.
+fn load(args: &Args) -> Result<(String, String, Option<Store>), String> {
+    let read =
+        |path: &Path| std::fs::read_to_string(path).map_err(|e| format!("{}: {e}", path.display()));
+    let Some(dir) = &args.state_dir else {
+        let path = args
+            .config
+            .as_deref()
+            .ok_or("give --config or --state-dir")?;
+        return Ok((path.display().to_string(), read(path)?, None));
+    };
+    let fail = |e: std::io::Error| format!("{}: {e}", dir.display());
+    let store = Store::open(dir).map_err(fail)?;
+    let active = store.active().map_err(fail)?;
+    // A valid config file that differs from the active snapshot becomes the
+    // next version: a restart is how listen addresses change. A missing or
+    // broken one falls back to the active snapshot, the last version known to
+    // work (spec §5.1).
+    let file = args.config.as_deref().map(|path| {
+        read(path).and_then(|text| compile(&text, &path.display().to_string()).map(|_| text))
+    });
+    let version = match (file, active) {
+        (Some(Ok(text)), active) => {
+            let current = active
+                .map(|v| store.read(v).map(|t| (v, t)))
+                .transpose()
+                .map_err(fail)?;
+            match current {
+                Some((version, same)) if same == text => version,
+                _ => {
+                    let version = store.stage(&text).map_err(fail)?;
+                    store.activate(version).map_err(fail)?;
+                    tracing::info!(version, "the config file becomes the active snapshot");
+                    version
+                }
+            }
+        }
+        (Some(Err(e)), Some(version)) => {
+            tracing::warn!(version, error = %e, "unusable config file, using the active snapshot");
+            version
+        }
+        (Some(Err(e)), None) => return Err(e),
+        (None, Some(version)) => version,
+        (None, None) => {
+            let msg = "has no active snapshot: give --config to seed it";
+            return Err(format!("{}: {msg}", dir.display()));
+        }
+    };
+    tracing::info!(version, "starting from the active snapshot");
+    let text = store.read(version).map_err(fail)?;
+    Ok((format!("snapshot {version}"), text, Some(store)))
+}
+
+fn compile(text: &str, origin: &str) -> Result<Policy, String> {
+    let bundle = Bundle::from_yaml(text).map_err(|e| format!("{origin}: {e}"))?;
+    match Policy::compile(&bundle) {
+        Ok((policy, warnings)) => {
+            warnings.iter().for_each(|w| tracing::warn!("{w}"));
+            Ok(policy)
+        }
+        Err(issues) => {
+            issues.iter().for_each(|i| eprintln!("{i}"));
+            Err(format!("{origin}: has errors, run `veyra validate`"))
+        }
+    }
+}
+
+/// Bind the control socket; the future serves it.
+#[cfg(unix)]
+fn control(
+    path: &Path,
+    runtime: Arc<Runtime>,
+) -> Result<impl std::future::Future<Output = ()>, String> {
+    let listener = veyra_core::control::bind(path)
+        .map_err(|e| format!("cannot open the control socket {}: {e}", path.display()))?;
+    println!("listening on {} (control)", path.display());
+    Ok(veyra_core::control::serve(listener, runtime))
+}
+
+#[cfg(not(unix))]
+fn control(path: &Path, _runtime: Arc<Runtime>) -> Result<std::future::Pending<()>, String> {
+    Err(format!(
+        "{}: the control socket needs Unix; run the data plane on Linux",
+        path.display()
+    ))
+}
````

- [ ] **Step 4: Run the tests to see them pass**

Run: `cargo test --workspace`
Expected: `dataplane` `ok. 8 passed`, `cli` `ok. 12 passed`; 173 in total.

- [ ] **Step 5: Format and lint**

Run: `cargo fmt --all && cargo clippy --workspace --all-targets -- -D warnings`
Expected: clippy prints no warnings.

Run: `cargo clippy --target aarch64-unknown-linux-gnu --workspace --all-targets -- -D warnings`
Expected: no warnings: the Unix-only code type-checks and lints too.

- [ ] **Step 6: Commit**

````bash
git add Cargo.lock Cargo.toml apps/veyra-cli/Cargo.toml apps/veyra-cli/src/main.rs apps/veyra-cli/tests/cli.rs apps/veyra-dataplane/Cargo.toml apps/veyra-dataplane/src/main.rs apps/veyra-dataplane/tests/dataplane.rs
git commit -m "feat(dataplane,cli): start from the active snapshot; veyra apply, rollback, status"
````

---

### Task 8: systemd unit, firewall and install guide

**Files:**
- Create: `deploy/nftables/veyra.nft`, `deploy/systemd/veyra-dataplane.service`, `docs/operations/install.md`
- Modify: `README.md`, `apps/veyra-dataplane/src/main.rs`

**Interfaces:**
- Consumes: the `Args` of `veyra-dataplane` (Task 7) and the CLI's default socket `/run/veyra/control.sock`.
- Produces: `deploy/systemd/veyra-dataplane.service`. It runs as `veyra` with `--state-dir /var/lib/veyra --config /etc/veyra/veyra.yaml --control /run/veyra/control.sock`, with `StateDirectory`, `RuntimeDirectory` (0750, so the `veyra` group can reach the socket), only `CAP_NET_BIND_SERVICE`, `LimitNOFILE=65536` and systemd sandboxing. `deploy/nftables/veyra.nft` is an example firewall: it accepts SSH, DNS and 80/443 and rejects everything else with a TCP reset or ICMP port unreachable (REQ-GW-002). `docs/operations/install.md` is the install guide, and the README gains "Deploy and change the config". Unit tests in `veyra-dataplane`'s `main.rs` parse the unit's `ExecStart` with the real `Args` and check that the firewall never drops.

- [ ] **Step 1: Write the failing tests**

In `apps/veyra-dataplane/src/main.rs`, apply this patch (save it to a file and run `git apply <file>`, or edit by hand):

````diff
diff --git a/apps/veyra-dataplane/src/main.rs b/apps/veyra-dataplane/src/main.rs
index 491a1b8..127a84c 100644
--- a/apps/veyra-dataplane/src/main.rs
+++ b/apps/veyra-dataplane/src/main.rs
@@ -210,3 +210,58 @@ fn control(path: &Path, _runtime: Arc<Runtime>) -> Result<std::future::Pending<(
         path.display()
     ))
 }
+#[cfg(test)]
+mod tests {
+    use super::*;
+
+    const UNIT: &str = include_str!("../../../deploy/systemd/veyra-dataplane.service");
+    const FIREWALL: &str = include_str!("../../../deploy/nftables/veyra.nft");
+
+    #[test]
+    fn the_systemd_unit_starts_the_data_plane_with_flags_it_accepts() {
+        let exec = UNIT
+            .lines()
+            .find_map(|l| l.strip_prefix("ExecStart="))
+            .expect("ExecStart line");
+        let mut words = exec.split_whitespace();
+        assert_eq!(words.next(), Some("/usr/local/bin/veyra-dataplane"));
+        let args = Args::try_parse_from(std::iter::once("veyra-dataplane").chain(words))
+            .expect("the unit's flags parse");
+        assert_eq!(args.state_dir.as_deref(), Some(Path::new("/var/lib/veyra")));
+        assert_eq!(
+            args.control.as_deref(),
+            Some(Path::new("/run/veyra/control.sock")),
+            "where `veyra apply` looks by default"
+        );
+        for setting in [
+            "User=veyra",
+            "StateDirectory=veyra",
+            "RuntimeDirectory=veyra",
+            "AmbientCapabilities=CAP_NET_BIND_SERVICE",
+            "LimitNOFILE=65536",
+        ] {
+            assert!(UNIT.lines().any(|l| l == setting), "{setting}");
+        }
+    }
+
+    #[test]
+    fn the_example_firewall_rejects_instead_of_dropping() {
+        let rules: Vec<&str> = FIREWALL
+            .lines()
+            .map(str::trim)
+            .filter(|l| !l.starts_with('#'))
+            .collect();
+        assert!(
+            rules.iter().all(|l| !l.contains("drop")),
+            "REQ-GW-002: unserved ports must fail fast"
+        );
+        for needed in [
+            "th dport 53 accept",
+            "tcp dport { 80, 443 } accept",
+            "meta l4proto tcp reject with tcp reset",
+            "reject with icmpx type port-unreachable",
+        ] {
+            assert!(rules.iter().any(|l| l.contains(needed)), "{needed}");
+        }
+    }
+}
````

- [ ] **Step 2: Run the tests to see them fail**

Run: `cargo test -p veyra-dataplane --bin veyra-dataplane`
Expected: compilation fails: ``couldn't read `apps\veyra-dataplane\src\../../../deploy/systemd/veyra-dataplane.service` `` and the same for `deploy/nftables/veyra.nft` (the files do not exist yet).

- [ ] **Step 3: Write the implementation**

In `README.md`, apply this patch (save it to a file and run `git apply <file>`, or edit by hand):

````diff
diff --git a/README.md b/README.md
index 414deef..72085e0 100644
--- a/README.md
+++ b/README.md
@@ -2,7 +2,7 @@
 
 Self-hosted SmartDNS and traffic routing platform. Design and requirements: [Veyra_Project_Specification_RU.md](Veyra_Project_Specification_RU.md).
 
-Current state: policy core, CLI, DNS engine and gateway (stage 1a, plan 3 of 4 — see `docs/superpowers/plans/`).
+Current state: policy core, CLI, DNS engine, gateway, and snapshots with ordered apply (stage 1a, plan 4 of 5 — see `docs/superpowers/plans/`).
 
 ## Build and test
 
@@ -42,3 +42,13 @@ curl --resolve example.com:10443:127.0.0.1 https://example.com:10443/
 ```
 
 Connections for other names, and from sources not under `admission:`, are closed at once. On Linux the gateway serves at most one connection per 8 files the process may open (`ulimit -n`, `LimitNOFILE` under systemd), so a stock limit of 1024 allows 128; raise the limit to serve more. On a server, the firewall must reject, not drop, traffic to ports the gateway does not serve: the kernel's TCP reset and ICMP port unreachable are what make clients fail fast (spec §10.1).
+
+## Deploy and change the config
+
+On a Linux server the data plane runs under systemd from snapshot files, and `veyra apply` switches it to a new config without a restart: gateway first, then DNS, verified, rolled back on failure. See [docs/operations/install.md](docs/operations/install.md).
+
+`--state-dir` works anywhere: the config file becomes snapshot 1, and the data plane falls back to the active snapshot when the file is later missing or broken.
+
+```bash
+cargo run -p veyra-dataplane -- --state-dir /tmp/veyra-state --config configs/examples/home.yaml
+```
````

In `apps/veyra-dataplane/src/main.rs`, apply this patch (save it to a file and run `git apply <file>`, or edit by hand):

````diff
diff --git a/apps/veyra-dataplane/src/main.rs b/apps/veyra-dataplane/src/main.rs
index 127a84c..5c8c68c 100644
--- a/apps/veyra-dataplane/src/main.rs
+++ b/apps/veyra-dataplane/src/main.rs
@@ -210,6 +210,7 @@ fn control(path: &Path, _runtime: Arc<Runtime>) -> Result<std::future::Pending<(
         path.display()
     ))
 }
+
 #[cfg(test)]
 mod tests {
     use super::*;
````

Create `deploy/nftables/veyra.nft`:

````text
#!/usr/sbin/nft -f
# Example host firewall for a Veyra node (spec §10.1, §19.2).
#
# Ports the node does not serve are REJECTED, never dropped: a TCP reset or an
# ICMP port unreachable makes a client fail at once instead of waiting for a
# timeout (REQ-GW-002). That includes UDP 443, so HTTP/3 clients fall back to
# TCP straight away.
#
# Check the SSH line before loading this on a remote host.

table inet veyra
delete table inet veyra

table inet veyra {
    chain input {
        type filter hook input priority 0; policy accept;

        ct state established,related accept
        iif lo accept
        meta l4proto { icmp, ipv6-icmp } accept

        tcp dport 22 accept comment "SSH: change to your port"
        meta l4proto { tcp, udp } th dport 53 accept comment "DNS"
        tcp dport { 80, 443 } accept comment "gateway"

        meta l4proto tcp reject with tcp reset
        reject with icmpx type port-unreachable
    }
}
````

Create `deploy/systemd/veyra-dataplane.service`:

````text
# Veyra data plane: DNS and gateway (spec §5.2, §21.2).
# Install: docs/operations/install.md
[Unit]
Description=Veyra data plane (DNS and gateway)
After=network-online.target
Wants=network-online.target

[Service]
Type=simple
User=veyra
Group=veyra
ExecStart=/usr/local/bin/veyra-dataplane --state-dir /var/lib/veyra --config /etc/veyra/veyra.yaml --control /run/veyra/control.sock
Restart=on-failure
RestartSec=2

# Snapshots live in /var/lib/veyra; the control socket in /run/veyra, which
# the veyra group may enter so its members can run `veyra apply`.
StateDirectory=veyra
StateDirectoryMode=0750
RuntimeDirectory=veyra
RuntimeDirectoryMode=0750

# Ports 53, 80 and 443 without root, and nothing else.
AmbientCapabilities=CAP_NET_BIND_SERVICE
CapabilityBoundingSet=CAP_NET_BIND_SERVICE
NoNewPrivileges=yes

# The gateway serves one connection per 8 open files: 65536 allows 8192.
LimitNOFILE=65536

ProtectSystem=strict
ProtectHome=yes
PrivateTmp=yes
PrivateDevices=yes
ProtectKernelTunables=yes
ProtectKernelModules=yes
ProtectControlGroups=yes
RestrictAddressFamilies=AF_INET AF_INET6 AF_UNIX
RestrictNamespaces=yes
LockPersonality=yes
MemoryDenyWriteExecute=yes
SystemCallArchitectures=native

[Install]
WantedBy=multi-user.target
````

Create `docs/operations/install.md`:

````markdown
# Installing the Veyra data plane on a Linux VPS

Stage 1a runs one service, `veyra-dataplane`: DNS on port 53 and the gateway on 443 and 80, configured from one YAML file and changed with `veyra apply` (spec §5.3, §8.5, §21).

## 1. Build and install the binaries

```bash
cargo build --release
sudo install -m 0755 target/release/veyra-dataplane target/release/veyra /usr/local/bin/
```

## 2. Create the service user and the config

```bash
sudo useradd --system --home-dir /var/lib/veyra --shell /usr/sbin/nologin veyra
sudo install -d -m 0755 /etc/veyra
sudo install -m 0644 configs/examples/home.yaml /etc/veyra/veyra.yaml
```

Edit `/etc/veyra/veyra.yaml` for the server:

- `dns.listen: ["[::]:53"]`, `gateway.tls: ["[::]:443"]` and `gateway.http: ["[::]:80"]`. On Linux, `[::]` also takes IPv4.
- The node's `gateway.ipv4` (and `ipv6`): the public addresses DNS hands out for proxied names.
- `admission`: the networks allowed to use DNS and the gateway. Everyone else is ignored.
- `dns.upstreams`: the resolvers Veyra forwards to and resolves origins through.

Check it with `veyra validate /etc/veyra/veyra.yaml`.

If `systemd-resolved` holds port 53 (`sudo ss -lunp 'sport = :53'` shows it), set `DNSStubListener=no` in `/etc/systemd/resolved.conf` and run `sudo systemctl restart systemd-resolved`.

## 3. Firewall

Ports the node does not serve must be rejected, not dropped, so clients fail at once instead of timing out (REQ-GW-002). `deploy/nftables/veyra.nft` is an example that does this. Fix its SSH port first, then:

```bash
sudo nft -f deploy/nftables/veyra.nft
```

If you keep your own firewall, make its last rules `reject`, not `drop`, and leave UDP 443 closed so HTTP/3 clients fall back to TCP.

## 4. Start the service

```bash
sudo install -m 0644 deploy/systemd/veyra-dataplane.service /etc/systemd/system/
sudo systemctl daemon-reload
sudo systemctl enable --now veyra-dataplane
journalctl -u veyra-dataplane -f
```

The unit runs as `veyra` with only `CAP_NET_BIND_SERVICE`, and raises the open-file limit so the gateway can serve 8192 connections at once (one per 8 files).

On the first start, the config file becomes snapshot 1 in `/var/lib/veyra`. On every later start, a changed and valid config file becomes the next snapshot; a missing or broken one is ignored and the data plane runs the active snapshot, the last version that worked.

## 5. Change the config while it runs

The control socket belongs to the `veyra` group. Join it once (then log in again), or run the commands with `sudo -u veyra`:

```bash
sudo usermod -aG veyra "$USER"
```

Then:

```bash
veyra validate /etc/veyra/veyra.yaml
veyra apply /etc/veyra/veyra.yaml   # gateway, then DNS, verify, commit (spec §8.5)
veyra status                        # active and previous versions
veyra rollback                      # back to the previous version, the same way
```

`veyra apply` switches the gateway first, so it accepts new names before DNS hands out its address. It then switches DNS and checks that every listener still accepts connections. If anything fails, both are switched back. A route the new version removes keeps working at the gateway for `gateway.drainWindow` (600 s by default), because clients and TVs cache DNS answers.

Listen addresses cannot change while the data plane runs: `veyra apply` refuses such a config. Edit the file and run `sudo systemctl restart veyra-dataplane` instead.

## Logs

Logs go to the journal without domain names or client addresses (spec §19.3). To see why the gateway refused connections, add `Environment=RUST_LOG=info,veyra_gateway=debug` to the unit with `sudo systemctl edit veyra-dataplane`.
````

- [ ] **Step 4: Run the tests to see them pass**

Run: `cargo test --workspace`
Expected: `veyra-dataplane` unit tests `ok. 2 passed`; 175 in total.

Run: `cargo run -q -p veyra-cli -- validate configs/examples/home.yaml`
Expected: `configs/examples/home.yaml: valid, 0 warning(s)`

- [ ] **Step 5: Format and lint**

Run: `cargo fmt --all && cargo clippy --workspace --all-targets -- -D warnings`
Expected: clippy prints no warnings.

Run: `cargo clippy --target aarch64-unknown-linux-gnu --workspace --all-targets -- -D warnings`
Expected: no warnings: the Unix-only code type-checks and lints too.

- [ ] **Step 6: Commit**

````bash
git add README.md apps/veyra-dataplane/src/main.rs deploy/nftables/veyra.nft deploy/systemd/veyra-dataplane.service docs/operations/install.md
git commit -m "docs(deploy): systemd unit, example firewall, install guide"
````

---

## Not in this plan

- Link-IP admission, link tokens and the HTTPS link endpoint with ACME certificates (§9.6, §21.5 step 1): plan 5.
- Signed snapshots and anti-rollback beyond monotonic version numbers (§19.2 item 11): with the control plane in stage 1b.
- A canary DNS answer in VERIFY (§8.5): VERIFY checks that every listener accepts connections. A canary needs an admission exception for the data plane's own probes (marked `ponytail:`).
- Engine health checks in VERIFY (§20.2 level 2): there are no protocol engines until stage 1b.
- Staged rollout across nodes (§8.5): stage 2.
- Pruning old snapshots: every version is kept; they are a few KB each.
- Keeping the DNS cache across an apply: each new version starts with an empty cache.
- deb/rpm packages and installer pre-checks (§21.2, §21.3): later. The install guide is manual.
- Handling SIGTERM: systemd stops the process; snapshots and pointers are written atomically, so nothing is lost.
- The deferred minors from the plan 2 and plan 3 reviews stay deferred. Two plan-3 ones are covered by the install guide: `CAP_NET_BIND_SERVICE`, and how to see why the gateway refused connections.
