# Veyra DNS Engine Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** A `veyra-dataplane` binary that serves DNS over UDP and TCP for the sources a policy file admits, answers blocked and proxied names itself exactly as spec §9.5 prescribes, and forwards and caches everything else.

**Architecture:** `veyra-policy` keeps every decision and stays free of sockets: it gains the `dns:` and `admission:` config sections, mode-aware decisions, and a pure "what to answer" function (`dns_plan`) that `veyra explain` shares with the engine. A new `veyra-dns` crate owns the wire format (hickory-proto) and the network: admission, synthesis, forwarding, cache and the UDP/TCP listeners. `apps/veyra-dataplane` only loads the config and starts the listeners.

**Tech Stack:** Rust 2024; adds hickory-proto 0.26 (messages only), tokio 1.53, tracing; plan 1's serde-saphyr, idna, clap, serde_json.

**Spec:** `Veyra_Project_Specification_RU.md` — §9.1 (DNS tasks), §9.3 (cache), §9.4 (safe overrides, loops), §9.5 (answer table), §9.6 (admission; the static CIDR part), §10.2 (gateway per exit node), §17.2 (invariants), §19.1–19.3 (abuse, spoofing, privacy), §20.3 (SERVFAIL policy), §23.2 stage 1a, §24.3 REQ-DNS-001…004.

## Where this plan sits

Stage 1a (spec §23.2) is four plans. Plan 1 (policy core + CLI) is merged. This is plan 2. Plan 3 adds the gateway (TCP 443 by SNI, 80 by `Host`) to `veyra-dataplane`; plan 4 adds link-IP admission, the snapshot file, the control socket and ordered apply.

## Global Constraints

- Everything from plan 1 still holds: edition 2024, `rust-version = "1.85"`, dependencies declared once in `[workspace.dependencies]`, `Cargo.lock` committed, no `unwrap()` outside tests, `unsafe_code = "forbid"`, fmt + clippy `-D warnings` + tests green on every commit.
- New dependency floors: hickory-proto 0.26.3 with `default-features = false, features = ["std"]`; tokio 1.53.2 (`macros`, `rt-multi-thread`, `net`, `time`, `io-util`, `sync`, `signal`); tracing 0.1.44; tracing-subscriber 0.3.23 (`env-filter`).
- Integration-test helper functions use `expect`, never `unwrap`: `allow-unwrap-in-tests` covers `#[test]` functions only, not helpers.
- Decisions live in `veyra-policy`; `veyra-dns` owns sockets and the wire format; the binary holds no DNS logic.
- Spec §9.5 exactly: BLOCK answers `0.0.0.0`/`::` (NODATA for other types); PROXY answers the node's gateway IPv4, its IPv6 or NODATA for AAAA, NODATA for HTTPS/SVCB, and forwards other types; DIRECT forwards everything; synthesized TTL = `synthTtl` (default 60, 30–300); synthesized replies never set AD.
- Spec §9.6: a source not admitted gets no UDP reply at all and a TCP connection closed before anything is read.
- No domain names in logs (spec §19.3): logs carry startup, warnings and upstream failures (server and error only).
- DNS-only config checks (upstream groups, gateways, loops) run only when the bundle has a `dns:` section; plan 1's rules-only bundles stay valid.
- `veyra explain` JSON only ever gains fields during stage 1a (`mode`, `source`, `source_binding`, `dns_answer`); none is renamed.
- Run every command from the repository root `E:\LeetCode\VeyraDNS`.

## Review Focus

Inputs a person will meet that the spec implies but does not spell out, most likely first. Each is pinned by the named test.

1. **Clients on a dual-stack listener arrive as IPv4-mapped IPv6** (`::ffff:192.168.1.7`) and must match IPv4 `admission` networks — `admission::cidrs_parse_normalize_and_match` (Task 4).
2. **Query names in any case or that do not normalize** (`WWW.Ads.Example.`, `*.example.`): the policy sees the normalized name, the reply echoes the client's spelling, and unparseable names get the profile default instead of an error — `blocked_names_get_null_addresses`, `names_that_do_not_normalize_use_the_profile_default` (Task 6).
3. **Answers too big for UDP** must come back with TC set so the client retries over TCP, never silently cut — `oversized_udp_answers_are_truncated_but_tcp_gets_everything` (Task 6), `a_truncated_upstream_reply_is_retried_over_tcp` (Task 7).
4. **A dead or unreachable upstream** must give a quick SERVFAIL after trying the next server, and failures must never be cached — `dead_upstreams_give_servfail`, `a_dead_upstream_falls_through_to_the_next` (Task 6), `servfail_is_not_cached` (Task 7).
5. **Strangers and garbage** — silence over UDP, a closed TCP connection, no panic — `drops_sources_that_are_not_admitted`, `drops_garbage_and_responses` (Task 6), `strangers_get_silence_over_udp`, `strangers_get_a_closed_tcp_connection` (Task 8).

## File map

| File | Responsibility |
|---|---|
| `crates/veyra-policy/src/dns_config.rs` | `dns:` section → `DnsConfig`; upstream-group, gateway and loop checks |
| `crates/veyra-policy/src/admission.rs` | `Cidr`, `admission:` bindings, `Policy::admit` |
| `crates/veyra-policy/src/answer.rs` | `QueryKind`, `DnsPlan`, `Policy::dns_plan` (the §9.5 table) |
| `crates/veyra-policy/tests/{dns_config,admission,dns_plan}.rs` | Public-API tests of the three above |
| `crates/veyra-dns/src/engine.rs` | `Engine::handle`: admission, decision, synthesis or forward, truncation |
| `crates/veyra-dns/src/synth.rs` | Replies Veyra writes itself: addresses, NODATA + SOA, error codes |
| `crates/veyra-dns/src/upstream.rs` | Forwarding over UDP with TCP fallback; TCP framing |
| `crates/veyra-dns/src/cache.rs` | TTL cache of upstream answers |
| `crates/veyra-dns/src/server.rs` | `Listeners`: UDP and TCP sockets, limits |
| `crates/veyra-dns/tests/support/mod.rs` | Fake upstreams, query builders, a test engine |
| `apps/veyra-dataplane/src/main.rs` | Load config, bind, serve until Ctrl-C |

---

### Task 1: Plan 1 review follow-ups

**Files:**
- Create: `apps/veyra-cli/tests/fixtures/warn.yaml`
- Modify: `apps/veyra-cli/src/main.rs`, `apps/veyra-cli/tests/cli.rs`, `apps/veyra-cli/tests/fixtures/broken.yaml`, `crates/veyra-policy/src/compile.rs`, `crates/veyra-policy/src/decide.rs`

**Interfaces:**
- Consumes: plan 1's `veyra-policy` and `veyra-cli`.
- Produces: `IssueCode::InvalidId` (ids must start with a letter or digit and use only letters, digits, `.`, `_`, `-`); on equal specificity `decide` reports the smallest pattern by `Pattern`'s `Ord`, so `matched_pattern` no longer depends on list order; `veyra explain` prints compile warnings to stderr.

- [ ] **Step 1: Write the failing tests**

In `apps/veyra-cli/tests/cli.rs`, apply this patch (save it to a file and run `git apply <file>`, or edit by hand):

````diff
diff --git a/apps/veyra-cli/tests/cli.rs b/apps/veyra-cli/tests/cli.rs
index bd2caad..6263a18 100644
--- a/apps/veyra-cli/tests/cli.rs
+++ b/apps/veyra-cli/tests/cli.rs
@@ -8,6 +8,7 @@ const EXAMPLE: &str = concat!(
 );
 const CONFLICT: &str = concat!(env!("CARGO_MANIFEST_DIR"), "/tests/fixtures/conflict.yaml");
 const BROKEN: &str = concat!(env!("CARGO_MANIFEST_DIR"), "/tests/fixtures/broken.yaml");
+const WARN: &str = concat!(env!("CARGO_MANIFEST_DIR"), "/tests/fixtures/warn.yaml");
 
 fn veyra(args: &[&str]) -> Output {
     Command::new(env!("CARGO_BIN_EXE_veyra"))
@@ -44,6 +45,11 @@ fn malformed_yaml_names_the_line() {
     let out = veyra(&["validate", BROKEN]);
     assert_eq!(out.status.code(), Some(1));
     assert!(text(&out.stderr).contains("line"), "{}", text(&out.stderr));
+    assert!(
+        !text(&out.stderr).contains("`match`"),
+        "{}",
+        text(&out.stderr)
+    );
 }
 
 #[test]
@@ -117,3 +123,20 @@ fn explain_refuses_a_config_with_errors() {
     assert_eq!(out.status.code(), Some(1));
     assert!(text(&out.stderr).contains("veyra validate"));
 }
+
+#[test]
+fn explain_prints_compile_warnings_to_stderr() {
+    let out = veyra(&[
+        "explain",
+        "--config",
+        WARN,
+        "--profile",
+        "home",
+        "a.x.example",
+    ]);
+    assert!(out.status.success(), "{}", text(&out.stderr));
+    let err = text(&out.stderr);
+    assert!(err.contains("warning[TieBrokenLate] a, b"), "{err}");
+    let v: serde_json::Value = serde_json::from_slice(&out.stdout).expect("json on stdout");
+    assert_eq!(v["decided_by"], "action");
+}
````

In `apps/veyra-cli/tests/fixtures/broken.yaml`, apply this patch (save it to a file and run `git apply <file>`, or edit by hand):

````diff
diff --git a/apps/veyra-cli/tests/fixtures/broken.yaml b/apps/veyra-cli/tests/fixtures/broken.yaml
index d6fc1cc..f32ded2 100644
--- a/apps/veyra-cli/tests/fixtures/broken.yaml
+++ b/apps/veyra-cli/tests/fixtures/broken.yaml
@@ -6,4 +6,6 @@ spec:
   rules:
     - id: a
       scope: default
+      match:
+        domains: [x.example]
      action: { type: block }
````

Create `apps/veyra-cli/tests/fixtures/warn.yaml`:

````yaml
apiVersion: veyra.io/v1alpha1
kind: PolicyBundle
metadata: { name: warn }
spec:
  profiles: [{ id: home }]
  rules:
    - { id: a, scope: default, match: { domains: ["*.x.example"] }, action: { type: block } }
    - { id: b, scope: default, match: { domains: ["suffix:x.example"] }, action: { type: direct } }
````

In `crates/veyra-policy/src/compile.rs`, apply this patch (save it to a file and run `git apply <file>`, or edit by hand):

````diff
diff --git a/crates/veyra-policy/src/compile.rs b/crates/veyra-policy/src/compile.rs
index ece6b1a..242c83e 100644
--- a/crates/veyra-policy/src/compile.rs
+++ b/crates/veyra-policy/src/compile.rs
@@ -471,6 +471,13 @@ mod tests {
         assert_eq!(policy.rules.len(), 1);
     }
 
+    #[test]
+    fn rejects_invalid_ids() {
+        let rules = "    - { id: \"\", scope: default, match: { domains: [x.example] }, action: { type: block } }\n    \
+                     - { id: \"b, c\", scope: default, match: { domains: [y.example] }, action: { type: block } }\n";
+        assert_eq!(errors(rules), vec![InvalidId, InvalidId]);
+    }
+
     #[test]
     fn issue_display_is_one_readable_line() {
         let rules = "    - { id: a, scope: default, match: { domains: [x.example] }, action: { type: block } }\n    \
````

In `crates/veyra-policy/src/decide.rs`, apply this patch (save it to a file and run `git apply <file>`, or edit by hand):

````diff
diff --git a/crates/veyra-policy/src/decide.rs b/crates/veyra-policy/src/decide.rs
index 90fe4de..6d82d64 100644
--- a/crates/veyra-policy/src/decide.rs
+++ b/crates/veyra-policy/src/decide.rs
@@ -300,6 +300,24 @@ mod tests {
         assert_eq!(d.domain, "www.xn--e1afmkfd.xn--p1ai");
     }
 
+    #[test]
+    fn matched_pattern_does_not_depend_on_pattern_order() {
+        for domains in [
+            "[\"*.y.example\", \"suffix:y.example\"]",
+            "[\"suffix:y.example\", \"*.y.example\"]",
+        ] {
+            let p = policy(&format!(
+                "    - {{ id: a, scope: default, match: {{ domains: {domains} }}, action: {{ type: block }} }}\n"
+            ));
+            let d = decide(&p, Some("home"), None, "a.y.example");
+            assert_eq!(
+                d.matched_pattern.as_deref(),
+                Some("*.y.example"),
+                "{domains}"
+            );
+        }
+    }
+
     #[test]
     fn subject_errors() {
         let p = example();
````

- [ ] **Step 2: Run the tests to see them fail**

Run: `cargo test -p veyra-policy`
Expected: compilation fails with ``cannot find value `InvalidId` in this scope``.

Run: `cargo test -p veyra-cli`
Expected: `explain_prints_compile_warnings_to_stderr ... FAILED`; `test result: FAILED. 7 passed; 1 failed`. (`malformed_yaml_names_the_line` already passes: its fixture fix is the change.)

- [ ] **Step 3: Write the implementation**

In `apps/veyra-cli/src/main.rs`, apply this patch (save it to a file and run `git apply <file>`, or edit by hand):

````diff
diff --git a/apps/veyra-cli/src/main.rs b/apps/veyra-cli/src/main.rs
index a969f7c..6014f66 100644
--- a/apps/veyra-cli/src/main.rs
+++ b/apps/veyra-cli/src/main.rs
@@ -62,7 +62,10 @@ fn run(cli: Cli) -> Result<(), String> {
             domain,
         } => {
             let policy = match load(&config)? {
-                Ok((policy, _)) => policy,
+                Ok((policy, warnings)) => {
+                    warnings.iter().for_each(|w| eprintln!("{w}"));
+                    policy
+                }
                 Err(issues) => {
                     issues.iter().for_each(|i| eprintln!("{i}"));
                     return Err(format!(
````

In `crates/veyra-policy/src/compile.rs`, apply this patch (save it to a file and run `git apply <file>`, or edit by hand):

````diff
diff --git a/crates/veyra-policy/src/compile.rs b/crates/veyra-policy/src/compile.rs
index 242c83e..cb43809 100644
--- a/crates/veyra-policy/src/compile.rs
+++ b/crates/veyra-policy/src/compile.rs
@@ -26,6 +26,7 @@ pub enum IssueCode {
     TieBrokenLate,
     Shadowed,
     FailOpen,
+    InvalidId,
 }
 
 #[derive(Debug, Clone, PartialEq, Eq)]
@@ -240,6 +241,12 @@ fn unique_ids<'a>(
 ) -> BTreeSet<&'a str> {
     let mut seen = BTreeSet::new();
     for id in ids {
+        if !valid_id(id) {
+            let msg = format!(
+                "{kind} id {id:?} must start with a letter or digit and use only letters, digits, '.', '_' or '-'"
+            );
+            issues.push(issue(Severity::Error, IssueCode::InvalidId, &[id], msg));
+        }
         if !seen.insert(id) {
             let msg = format!("{kind} id {id} is used more than once");
             issues.push(issue(Severity::Error, IssueCode::DuplicateId, &[id], msg));
@@ -248,6 +255,15 @@ fn unique_ids<'a>(
     seen
 }
 
+/// Ids name rules in issue lines and break the last precedence tie (spec §8.2.1),
+/// so they stay simple: a letter or digit, then letters, digits, `.`, `_` or `-`.
+fn valid_id(id: &str) -> bool {
+    id.starts_with(|c: char| c.is_ascii_alphanumeric())
+        && id
+            .chars()
+            .all(|c| c.is_ascii_alphanumeric() || matches!(c, '.' | '_' | '-'))
+}
+
 /// Pairwise checks inside one scope. Rules in different scopes are always
 /// separated by step 2 of §8.2.1, so they never conflict.
 // ponytail: O(rules² × patterns²); index by pattern base name if bundles reach thousands of rules.
````

In `crates/veyra-policy/src/decide.rs`, apply this patch (save it to a file and run `git apply <file>`, or edit by hand):

````diff
diff --git a/crates/veyra-policy/src/decide.rs b/crates/veyra-policy/src/decide.rs
index 6d82d64..354feb2 100644
--- a/crates/veyra-policy/src/decide.rs
+++ b/crates/veyra-policy/src/decide.rs
@@ -101,7 +101,9 @@ impl Policy {
                     .patterns
                     .iter()
                     .filter(|p| p.matches(name))
-                    .max_by_key(|p| p.specificity())?;
+                    // On equal specificity the smallest pattern wins, so the reported
+                    // pattern does not depend on the order of the domains list.
+                    .max_by_key(|p| (p.specificity(), Reverse(*p)))?;
                 Some((
                     (best.specificity(), scope_rank(&r.scope), r.late_rank()),
                     r,
````

- [ ] **Step 4: Run the tests to see them pass**

Run: `cargo test --workspace`
Expected: `veyra-cli` tests `ok. 8 passed`, `veyra-policy` unit tests `ok. 47 passed`, determinism `ok. 1 passed`.

- [ ] **Step 5: Format and lint**

Run: `cargo fmt --all && cargo clippy --workspace --all-targets -- -D warnings`
Expected: clippy prints no warnings.

- [ ] **Step 6: Commit**

````bash
git add apps/veyra-cli/src/main.rs apps/veyra-cli/tests/cli.rs apps/veyra-cli/tests/fixtures/broken.yaml apps/veyra-cli/tests/fixtures/warn.yaml crates/veyra-policy/src/compile.rs crates/veyra-policy/src/decide.rs
git commit -m "fix(policy,cli): plan-1 review follow-ups"
````

---

### Task 2: Mode-aware decisions and `explain --mode`

**Files:**
- Create: `apps/veyra-cli/tests/fixtures/modes.yaml`
- Modify: `apps/veyra-cli/Cargo.toml`, `apps/veyra-cli/src/main.rs`, `apps/veyra-cli/tests/cli.rs`, `crates/veyra-policy/src/decide.rs`, `crates/veyra-policy/src/model.rs`

**Interfaces:**
- Consumes: `Policy`, `Rule.modes`, `Decision`, `Mode` (plan 1).
- Produces: `Policy::decide_in(&self, subject: &Subject, name: &Name, mode: Option<Mode>) -> Decision` (rules whose `modesAllowed` excludes `mode` do not apply; `None` ignores modes); `Policy::decide` is `decide_in(.., None)`; `Policy::default_decision(&self, subject: &Subject, domain: &str) -> Decision` (the profile's `defaultAction`, `decided_by: default`); `impl FromStr for Mode` (`Err = String`, the YAML names); `veyra explain --mode <smartdns_tcp|explicit_proxy|tunnel>` defaulting to `smartdns_tcp`; the explain JSON gains `"mode"` after the decision fields (a private `Explain` struct with `#[serde(flatten)]`).

- [ ] **Step 1: Write the failing tests**

In `apps/veyra-cli/tests/cli.rs`, apply this patch (save it to a file and run `git apply <file>`, or edit by hand):

````diff
diff --git a/apps/veyra-cli/tests/cli.rs b/apps/veyra-cli/tests/cli.rs
index 6263a18..89259da 100644
--- a/apps/veyra-cli/tests/cli.rs
+++ b/apps/veyra-cli/tests/cli.rs
@@ -9,6 +9,7 @@ const EXAMPLE: &str = concat!(
 const CONFLICT: &str = concat!(env!("CARGO_MANIFEST_DIR"), "/tests/fixtures/conflict.yaml");
 const BROKEN: &str = concat!(env!("CARGO_MANIFEST_DIR"), "/tests/fixtures/broken.yaml");
 const WARN: &str = concat!(env!("CARGO_MANIFEST_DIR"), "/tests/fixtures/warn.yaml");
+const MODES: &str = concat!(env!("CARGO_MANIFEST_DIR"), "/tests/fixtures/modes.yaml");
 
 fn veyra(args: &[&str]) -> Output {
     Command::new(env!("CARGO_BIN_EXE_veyra"))
@@ -140,3 +141,38 @@ fn explain_prints_compile_warnings_to_stderr() {
     let v: serde_json::Value = serde_json::from_slice(&out.stdout).expect("json on stdout");
     assert_eq!(v["decided_by"], "action");
 }
+
+#[test]
+fn explain_reports_and_honours_the_mode() {
+    let run = |extra: &[&str]| {
+        let mut args = vec!["explain", "--config", MODES, "--profile", "home"];
+        args.extend_from_slice(extra);
+        args.push("x.example");
+        let out = veyra(&args);
+        assert!(out.status.success(), "{}", text(&out.stderr));
+        serde_json::from_slice::<serde_json::Value>(&out.stdout).expect("json on stdout")
+    };
+    let default = run(&[]);
+    assert_eq!(default["mode"], "smartdns_tcp");
+    assert_eq!(default["decided_by"], "default");
+    let tunnel = run(&["--mode", "tunnel"]);
+    assert_eq!(tunnel["mode"], "tunnel");
+    assert_eq!(tunnel["route"]["type"], "block");
+
+    let bad = veyra(&[
+        "explain",
+        "--config",
+        MODES,
+        "--profile",
+        "home",
+        "--mode",
+        "udp",
+        "x.example",
+    ]);
+    assert_eq!(bad.status.code(), Some(2));
+    assert!(
+        text(&bad.stderr).contains("unknown mode"),
+        "{}",
+        text(&bad.stderr)
+    );
+}
````

Create `apps/veyra-cli/tests/fixtures/modes.yaml`:

````yaml
apiVersion: veyra.io/v1alpha1
kind: PolicyBundle
metadata: { name: modes }
spec:
  profiles: [{ id: home }]
  rules:
    - { id: tunnel-only, scope: default, match: { domains: [x.example] }, action: { type: block }, modesAllowed: [tunnel] }
````

In `crates/veyra-policy/src/decide.rs`, apply this patch (save it to a file and run `git apply <file>`, or edit by hand):

````diff
diff --git a/crates/veyra-policy/src/decide.rs b/crates/veyra-policy/src/decide.rs
index 354feb2..e47b6d0 100644
--- a/crates/veyra-policy/src/decide.rs
+++ b/crates/veyra-policy/src/decide.rs
@@ -163,7 +163,7 @@ fn scope_rank(scope: &Scope) -> u8 {
 #[cfg(test)]
 mod tests {
     use super::*;
-    use crate::model::{Bundle, OnFailure};
+    use crate::model::{Bundle, Mode, OnFailure};
 
     const EXAMPLE: &str = include_str!("../../../configs/examples/home.yaml");
 
@@ -302,6 +302,36 @@ mod tests {
         assert_eq!(d.domain, "www.xn--e1afmkfd.xn--p1ai");
     }
 
+    #[test]
+    fn rules_outside_the_mode_do_not_apply() {
+        let p = policy(
+            "    - { id: a, scope: default, match: { domains: [x.example] }, action: { type: block }, modesAllowed: [tunnel] }\n",
+        );
+        let subject = p.subject(Some("home"), None).unwrap();
+        let name = Name::parse("x.example").unwrap();
+        assert_eq!(
+            p.decide_in(&subject, &name, Some(Mode::Tunnel)).route,
+            Action::Block {}
+        );
+        let dns = p.decide_in(&subject, &name, Some(Mode::SmartdnsTcp));
+        assert_eq!(
+            (dns.route, dns.decided_by),
+            (Action::Direct {}, DecidedBy::Default)
+        );
+        assert_eq!(p.decide(&subject, &name).route, Action::Block {});
+    }
+
+    #[test]
+    fn default_decision_uses_the_profile_default() {
+        let p = policy("    []\n");
+        let strict = p.subject(Some("strict"), None).unwrap();
+        let d = p.default_decision(&strict, "not a name");
+        assert_eq!(
+            (d.route, d.decided_by, d.domain.as_str()),
+            (Action::Block {}, DecidedBy::Default, "not a name")
+        );
+    }
+
     #[test]
     fn matched_pattern_does_not_depend_on_pattern_order() {
         for domains in [
````

In `crates/veyra-policy/src/model.rs`, apply this patch (save it to a file and run `git apply <file>`, or edit by hand):

````diff
diff --git a/crates/veyra-policy/src/model.rs b/crates/veyra-policy/src/model.rs
index 8c64f20..1188ca9 100644
--- a/crates/veyra-policy/src/model.rs
+++ b/crates/veyra-policy/src/model.rs
@@ -224,4 +224,12 @@ mod tests {
         let err = Bundle::from_yaml(&yaml).unwrap_err();
         assert!(err.contains("teleport"), "{err}");
     }
+
+    #[test]
+    fn modes_parse_from_their_yaml_names() {
+        assert_eq!("smartdns_tcp".parse::<Mode>(), Ok(Mode::SmartdnsTcp));
+        assert_eq!("explicit_proxy".parse::<Mode>(), Ok(Mode::ExplicitProxy));
+        assert_eq!("tunnel".parse::<Mode>(), Ok(Mode::Tunnel));
+        assert!("udp".parse::<Mode>().is_err());
+    }
 }
````

- [ ] **Step 2: Run the tests to see them fail**

Run: `cargo test -p veyra-policy`
Expected: compilation fails: ``no method named `decide_in` ``, ``no method named `default_decision` ``, ``the trait bound `model::Mode: FromStr` is not satisfied``.

Run: `cargo test -p veyra-cli`
Expected: `explain_reports_and_honours_the_mode ... FAILED`; `test result: FAILED. 8 passed; 1 failed`.

- [ ] **Step 3: Write the implementation**

In `apps/veyra-cli/Cargo.toml`, apply this patch (save it to a file and run `git apply <file>`, or edit by hand):

````diff
diff --git a/apps/veyra-cli/Cargo.toml b/apps/veyra-cli/Cargo.toml
index 698b0c9..c45cd15 100644
--- a/apps/veyra-cli/Cargo.toml
+++ b/apps/veyra-cli/Cargo.toml
@@ -12,6 +12,7 @@ path = "src/main.rs"
 [dependencies]
 veyra-policy.workspace = true
 clap.workspace = true
+serde.workspace = true
 serde_json.workspace = true
 
 [lints]
````

In `apps/veyra-cli/src/main.rs`, apply this patch (save it to a file and run `git apply <file>`, or edit by hand):

````diff
diff --git a/apps/veyra-cli/src/main.rs b/apps/veyra-cli/src/main.rs
index 6014f66..c07e5b1 100644
--- a/apps/veyra-cli/src/main.rs
+++ b/apps/veyra-cli/src/main.rs
@@ -1,9 +1,10 @@
 //! `veyra` command line, stage 1a (spec §23.2): validate and explain a policy file.
 
 use clap::{Parser, Subcommand};
+use serde::Serialize;
 use std::path::{Path, PathBuf};
 use std::process::ExitCode;
-use veyra_policy::{Bundle, Issue, Name, Policy};
+use veyra_policy::{Bundle, Decision, Issue, Mode, Name, Policy};
 
 #[derive(Parser)]
 #[command(name = "veyra", version, about = "Veyra policy tools")]
@@ -24,10 +25,22 @@ enum Command {
         profile: Option<String>,
         #[arg(long)]
         device: Option<String>,
+        /// Connection mode: smartdns_tcp, explicit_proxy or tunnel.
+        #[arg(long, default_value = "smartdns_tcp")]
+        mode: Mode,
         domain: String,
     },
 }
 
+/// The `veyra explain` JSON: the decision plus the inputs it was made for.
+/// Fields are only ever added during stage 1a, never renamed.
+#[derive(Serialize)]
+struct Explain<'a> {
+    #[serde(flatten)]
+    decision: &'a Decision,
+    mode: Mode,
+}
+
 fn main() -> ExitCode {
     match run(Cli::parse()) {
         Ok(()) => ExitCode::SUCCESS,
@@ -59,6 +72,7 @@ fn run(cli: Cli) -> Result<(), String> {
             config,
             profile,
             device,
+            mode,
             domain,
         } => {
             let policy = match load(&config)? {
@@ -78,8 +92,12 @@ fn run(cli: Cli) -> Result<(), String> {
                 .subject(profile.as_deref(), device.as_deref())
                 .map_err(|e| e.to_string())?;
             let name = Name::parse(&domain).map_err(|e| format!("{domain:?}: {e}"))?;
-            let decision = policy.decide(&subject, &name);
-            let json = serde_json::to_string_pretty(&decision).map_err(|e| e.to_string())?;
+            let decision = policy.decide_in(&subject, &name, Some(mode));
+            let explain = Explain {
+                decision: &decision,
+                mode,
+            };
+            let json = serde_json::to_string_pretty(&explain).map_err(|e| e.to_string())?;
             println!("{json}");
             Ok(())
         }
````

In `crates/veyra-policy/src/decide.rs`, apply this patch (save it to a file and run `git apply <file>`, or edit by hand):

````diff
diff --git a/crates/veyra-policy/src/decide.rs b/crates/veyra-policy/src/decide.rs
index e47b6d0..fb66a77 100644
--- a/crates/veyra-policy/src/decide.rs
+++ b/crates/veyra-policy/src/decide.rs
@@ -1,7 +1,7 @@
 //! Rule selection (spec §8.2.1) and the explanation `veyra explain` prints (spec §8.4).
 
 use crate::compile::{Policy, Rule};
-use crate::model::{Action, DefaultAction, Scope};
+use crate::model::{Action, DefaultAction, Mode, Scope};
 use crate::name::Name;
 use crate::pattern::{Pattern, Specificity};
 use serde::Serialize;
@@ -91,11 +91,18 @@ impl Policy {
         }
     }
 
+    /// Decide ignoring `modesAllowed`: every enabled rule may apply.
     pub fn decide(&self, subject: &Subject, name: &Name) -> Decision {
+        self.decide_in(subject, name, None)
+    }
+
+    /// Decide for one connection mode: rules whose `modesAllowed` excludes
+    /// `mode` do not apply (spec §8.3, §12.6).
+    pub fn decide_in(&self, subject: &Subject, name: &Name, mode: Option<Mode>) -> Decision {
         let mut candidates: Vec<(Key<'_>, &Rule, &Pattern)> = self
             .rules
             .iter()
-            .filter(|r| applies(&r.scope, subject))
+            .filter(|r| applies(&r.scope, subject) && mode.is_none_or(|m| r.modes.contains(&m)))
             .filter_map(|r| {
                 let best = r
                     .patterns
@@ -113,20 +120,7 @@ impl Policy {
             .collect();
         candidates.sort_by(|a, b| b.0.cmp(&a.0));
 
-        let mut decision = Decision {
-            policy: self.name.clone(),
-            domain: name.to_string(),
-            profile: subject.profile.clone(),
-            device: subject.device.clone(),
-            route: match self.profiles.get(&subject.profile) {
-                Some(DefaultAction::Block) => Action::Block {},
-                _ => Action::Direct {},
-            },
-            upstream_group: None,
-            matched_rule_id: None,
-            matched_pattern: None,
-            decided_by: DecidedBy::Default,
-        };
+        let mut decision = self.default_decision(subject, name.as_str());
         if let Some((best, rule, pattern)) = candidates.first() {
             decision.route = rule.action.clone();
             decision.upstream_group = rule.dns.as_ref().map(|d| d.upstream_group.clone());
@@ -142,6 +136,25 @@ impl Policy {
         }
         decision
     }
+
+    /// The decision when no rule matches: the profile's `defaultAction`. The DNS
+    /// engine also uses it for query names that do not normalize to a `Name`.
+    pub fn default_decision(&self, subject: &Subject, domain: &str) -> Decision {
+        Decision {
+            policy: self.name.clone(),
+            domain: domain.to_string(),
+            profile: subject.profile.clone(),
+            device: subject.device.clone(),
+            route: match self.profiles.get(&subject.profile) {
+                Some(DefaultAction::Block) => Action::Block {},
+                _ => Action::Direct {},
+            },
+            upstream_group: None,
+            matched_rule_id: None,
+            matched_pattern: None,
+            decided_by: DecidedBy::Default,
+        }
+    }
 }
 
 fn applies(scope: &Scope, subject: &Subject) -> bool {
````

In `crates/veyra-policy/src/model.rs`, apply this patch (save it to a file and run `git apply <file>`, or edit by hand):

````diff
diff --git a/crates/veyra-policy/src/model.rs b/crates/veyra-policy/src/model.rs
index 1188ca9..0a8d090 100644
--- a/crates/veyra-policy/src/model.rs
+++ b/crates/veyra-policy/src/model.rs
@@ -140,6 +140,22 @@ pub enum Mode {
     Tunnel,
 }
 
+impl std::str::FromStr for Mode {
+    type Err = String;
+
+    /// The same names as `modesAllowed` in the YAML file.
+    fn from_str(s: &str) -> Result<Mode, String> {
+        match s {
+            "smartdns_tcp" => Ok(Mode::SmartdnsTcp),
+            "explicit_proxy" => Ok(Mode::ExplicitProxy),
+            "tunnel" => Ok(Mode::Tunnel),
+            _ => Err(format!(
+                "unknown mode {s:?}; expected smartdns_tcp, explicit_proxy or tunnel"
+            )),
+        }
+    }
+}
+
 impl Bundle {
     pub fn from_yaml(text: &str) -> Result<Bundle, String> {
         serde_saphyr::from_str(text).map_err(|e| e.to_string())
````

- [ ] **Step 4: Run the tests to see them pass**

Run: `cargo test --workspace`
Expected: `veyra-cli` `ok. 9 passed`, `veyra-policy` unit `ok. 50 passed`, determinism `ok. 1 passed`.

- [ ] **Step 5: Format and lint**

Run: `cargo fmt --all && cargo clippy --workspace --all-targets -- -D warnings`
Expected: clippy prints no warnings.

- [ ] **Step 6: Commit**

````bash
git add Cargo.lock apps/veyra-cli/Cargo.toml apps/veyra-cli/src/main.rs apps/veyra-cli/tests/cli.rs apps/veyra-cli/tests/fixtures/modes.yaml crates/veyra-policy/src/decide.rs crates/veyra-policy/src/model.rs
git commit -m "feat(policy,cli): mode-aware decisions and explain --mode"
````

---

### Task 3: The `dns:` section and node gateways

**Files:**
- Create: `crates/veyra-policy/src/dns_config.rs`, `crates/veyra-policy/tests/dns_config.rs`
- Modify: `configs/examples/home.yaml`, `crates/veyra-policy/src/compile.rs`, `crates/veyra-policy/src/lib.rs`, `crates/veyra-policy/src/model.rs`

**Interfaces:**
- Consumes: `BundleSpec`, `NodeSpec`, `RuleSpec`, `Policy::compile`, `IssueCode` (plan 1).
- Produces (model): `NodeSpec.gateway: Option<GatewaySpec>`; `GatewaySpec { ipv4: Ipv4Addr, ipv6: Option<Ipv6Addr> }`; `BundleSpec.dns: Option<DnsEngineSpec>`; `DnsEngineSpec { listen: Vec<String>, synth_ttl: u32 /* YAML synthTtl, default 60 */, upstreams: Vec<UpstreamGroupSpec> }`; `UpstreamGroupSpec { id: String, servers: Vec<String> }`.
- Produces (compiled): `DnsConfig { listen: Vec<SocketAddr>, synth_ttl: u32, upstreams: BTreeMap<String, Vec<SocketAddr>> }`; `DEFAULT_UPSTREAM_GROUP = "default"`; `Policy::dns(&self) -> Option<&DnsConfig>`; `Policy::gateway(&self, node: &str) -> Option<&GatewaySpec>`; `IssueCode::{InvalidAddress, InvalidValue, MissingGateway, UpstreamLoop}`; `compile.rs`'s `issue()` and `unique_ids()` become `pub(crate)` for the new `dns_config.rs`.
- Rules: the DNS checks run only when the bundle has a `dns:` section, so plan 1's rules-only bundles stay valid. Upstream servers may be a bare IP (port 53).

- [ ] **Step 1: Write the failing tests**

Create `crates/veyra-policy/tests/dns_config.rs`:

````rust
//! The `dns:` section and node gateways (spec §9, §10.2, §17.2).

use std::net::{Ipv4Addr, Ipv6Addr, SocketAddr};
use veyra_policy::{Bundle, GatewaySpec, Issue, IssueCode, Policy, Severity};

/// A bundle with one gateway node, one node without a gateway, and `extra`
/// appended under `spec:` (more rules, a `dns:` section, ...).
fn compile(rules: &str, extra: &str) -> Result<(Policy, Vec<Issue>), Vec<Issue>> {
    let text = format!(
        "apiVersion: veyra.io/v1alpha1\nkind: PolicyBundle\nmetadata: {{ name: t }}\nspec:\n  \
         nodes:\n    \
         - {{ id: de-01, gateway: {{ ipv4: 203.0.113.10, ipv6: \"2001:db8::10\" }} }}\n    \
         - {{ id: us-01 }}\n  \
         profiles: [{{ id: home }}]\n  \
         rules:\n    \
         - {{ id: video, scope: default, match: {{ domains: [\"suffix:media.example\"] }}, action: {{ type: proxy_node, node: de-01 }} }}\n\
         {rules}{extra}"
    );
    Policy::compile(&Bundle::from_yaml(&text).expect("test YAML parses"))
}

const DNS: &str = "  dns:\n    listen: [\"127.0.0.1:5353\"]\n    upstreams:\n      - { id: default, servers: [\"1.1.1.1\"] }\n";

/// Error codes only: warnings from other sections do not belong to these tests.
fn errors(rules: &str, extra: &str) -> Vec<IssueCode> {
    compile(rules, extra)
        .expect_err("expected errors")
        .iter()
        .filter(|i| i.severity == Severity::Error)
        .map(|i| i.code)
        .collect()
}

fn addr(s: &str) -> SocketAddr {
    s.parse().expect("test address parses")
}

#[test]
fn parses_listeners_upstreams_gateways_and_defaults() {
    let dns = "  dns:\n    listen: [\"127.0.0.1:5353\", \"[::1]:5353\"]\n    upstreams:\n      \
               - { id: default, servers: [\"1.1.1.1\", \"[2606:4700:4700::1111]:53\"] }\n      \
               - { id: corp, servers: [\"10.0.0.53:5300\"] }\n";
    let (policy, _warnings) = compile("", dns).unwrap();
    let config = policy.dns().unwrap();
    assert_eq!(
        config.listen,
        vec![addr("127.0.0.1:5353"), addr("[::1]:5353")]
    );
    assert_eq!(config.synth_ttl, 60);
    assert_eq!(
        config.upstreams["default"],
        vec![addr("1.1.1.1:53"), addr("[2606:4700:4700::1111]:53")]
    );
    assert_eq!(config.upstreams["corp"], vec![addr("10.0.0.53:5300")]);
    assert_eq!(
        policy.gateway("de-01"),
        Some(&GatewaySpec {
            ipv4: Ipv4Addr::new(203, 0, 113, 10),
            ipv6: Some(Ipv6Addr::new(0x2001, 0xdb8, 0, 0, 0, 0, 0, 0x10)),
        })
    );
    assert_eq!(policy.gateway("us-01"), None);
}

#[test]
fn without_a_dns_section_the_bundle_is_rules_only() {
    let rules = "    - { id: us, scope: default, match: { domains: [us.example] }, action: { type: proxy_node, node: us-01 } }\n    \
                 - { id: corp, scope: default, match: { domains: [corp.example] }, dns: { upstreamGroup: nope }, action: { type: direct } }\n";
    let (policy, issues) = compile(rules, "").unwrap();
    assert!(issues.is_empty(), "{issues:?}");
    assert!(policy.dns().is_none());
}

#[test]
fn rejects_bad_addresses() {
    let dns = "  dns:\n    listen: [\"localhost:53\"]\n    upstreams:\n      - { id: default, servers: [\"1.1.1\"] }\n";
    assert_eq!(
        errors("", dns),
        vec![IssueCode::InvalidAddress, IssueCode::InvalidAddress]
    );
}

#[test]
fn requires_a_default_upstream_group() {
    let dns = "  dns:\n    listen: [\"127.0.0.1:5353\"]\n    upstreams:\n      - { id: corp, servers: [\"10.0.0.53\"] }\n";
    assert_eq!(errors("", dns), vec![IssueCode::UnknownReference]);
}

#[test]
fn rejects_unknown_upstream_group_references() {
    let rules = "    - { id: corp, scope: default, match: { domains: [corp.example] }, dns: { upstreamGroup: nope }, action: { type: direct } }\n";
    assert_eq!(errors(rules, DNS), vec![IssueCode::UnknownReference]);
}

#[test]
fn rejects_synth_ttl_outside_30_to_300() {
    for ttl in ["5", "301"] {
        let dns = DNS.replace(
            "    upstreams:",
            &format!("    synthTtl: {ttl}\n    upstreams:"),
        );
        assert_eq!(errors("", &dns), vec![IssueCode::InvalidValue], "{ttl}");
    }
    let dns = DNS.replace("    upstreams:", "    synthTtl: 300\n    upstreams:");
    assert_eq!(compile("", &dns).unwrap().0.dns().unwrap().synth_ttl, 300);
}

#[test]
fn rejects_empty_listener_and_server_lists() {
    let dns = "  dns:\n    listen: []\n    upstreams:\n      - { id: default, servers: [] }\n";
    assert_eq!(
        errors("", dns),
        vec![IssueCode::InvalidValue, IssueCode::InvalidValue]
    );
}

#[test]
fn proxy_rules_served_over_dns_need_a_gateway() {
    let rules = "    - { id: us, scope: default, match: { domains: [us.example] }, action: { type: proxy_node, node: us-01 } }\n";
    assert_eq!(errors(rules, DNS), vec![IssueCode::MissingGateway]);
    let tunnel_only = "    - { id: us, scope: default, match: { domains: [us.example] }, action: { type: proxy_node, node: us-01 }, modesAllowed: [tunnel] }\n";
    assert!(compile(tunnel_only, DNS).is_ok());
}

#[test]
fn rejects_forwarding_to_itself() {
    for (listen, server) in [
        ("0.0.0.0:53", "127.0.0.1"),
        ("127.0.0.1:5353", "127.0.0.1:5353"),
    ] {
        let dns = format!(
            "  dns:\n    listen: [\"{listen}\"]\n    upstreams:\n      - {{ id: default, servers: [\"{server}\"] }}\n"
        );
        assert_eq!(
            errors("", &dns),
            vec![IssueCode::UpstreamLoop],
            "{listen} {server}"
        );
    }
    let other_port = "  dns:\n    listen: [\"127.0.0.1:5353\"]\n    upstreams:\n      - { id: default, servers: [\"127.0.0.1\"] }\n";
    assert!(compile("", other_port).is_ok());
}
````

- [ ] **Step 2: Run the tests to see them fail**

Run: `cargo test -p veyra-policy --test dns_config`
Expected: compilation fails: ``unresolved import `veyra_policy::GatewaySpec` ``, ``no method named `dns` ``, ``no method named `gateway` ``, and no `IssueCode` variants `InvalidAddress`, `InvalidValue`, `MissingGateway`, `UpstreamLoop`.

- [ ] **Step 3: Write the implementation**

In `configs/examples/home.yaml`, apply this patch (save it to a file and run `git apply <file>`, or edit by hand):

````diff
diff --git a/configs/examples/home.yaml b/configs/examples/home.yaml
index 2bef481..4a39a4f 100644
--- a/configs/examples/home.yaml
+++ b/configs/examples/home.yaml
@@ -6,6 +6,7 @@ metadata:
 spec:
   nodes:
     - id: de-01
+      gateway: { ipv4: 203.0.113.10 }   # where DNS sends mode-B clients (§10.2)
   profiles:
     - id: home
       defaultAction: direct
@@ -48,3 +49,11 @@ spec:
         domains: ["suffix:ads.example"]
       action:
         type: block
+  dns:
+    listen: ["127.0.0.1:5353"]          # production: ["0.0.0.0:53", "[::]:53"]
+    synthTtl: 60
+    upstreams:
+      - id: default
+        servers: ["1.1.1.1", "9.9.9.9"]
+      - id: corp
+        servers: ["10.0.0.53"]
````

In `crates/veyra-policy/src/compile.rs`, apply this patch (save it to a file and run `git apply <file>`, or edit by hand):

````diff
diff --git a/crates/veyra-policy/src/compile.rs b/crates/veyra-policy/src/compile.rs
index cb43809..3d19093 100644
--- a/crates/veyra-policy/src/compile.rs
+++ b/crates/veyra-policy/src/compile.rs
@@ -1,8 +1,9 @@
 //! Turns a parsed `Bundle` into a checked `Policy`, collecting every problem
 //! instead of stopping at the first one (spec §8.2.1 validator rules, §17.2).
 
+use crate::dns_config::{DnsConfig, compile_dns};
 use crate::model::{
-    API_VERSION, Action, Bundle, DefaultAction, DnsSpec, KIND, Mode, OnFailure, Scope,
+    API_VERSION, Action, Bundle, DefaultAction, DnsSpec, GatewaySpec, KIND, Mode, OnFailure, Scope,
 };
 use crate::pattern::Pattern;
 use std::cmp::Reverse;
@@ -27,6 +28,10 @@ pub enum IssueCode {
     Shadowed,
     FailOpen,
     InvalidId,
+    InvalidAddress,
+    InvalidValue,
+    MissingGateway,
+    UpstreamLoop,
 }
 
 #[derive(Debug, Clone, PartialEq, Eq)]
@@ -85,9 +90,11 @@ pub struct Policy {
     pub(crate) profiles: BTreeMap<String, DefaultAction>,
     pub(crate) devices: BTreeMap<String, String>,
     pub(crate) rules: Vec<Rule>,
+    pub(crate) gateways: BTreeMap<String, GatewaySpec>,
+    pub(crate) dns: Option<DnsConfig>,
 }
 
-fn issue(severity: Severity, code: IssueCode, rules: &[&str], message: String) -> Issue {
+pub(crate) fn issue(severity: Severity, code: IssueCode, rules: &[&str], message: String) -> Issue {
     Issue {
         severity,
         code,
@@ -207,6 +214,7 @@ impl Policy {
             }
         }
         check_overlaps(&rules, &mut issues);
+        let dns = compile_dns(spec, &mut issues);
 
         if issues.iter().any(|i| i.severity == Error) {
             return Err(issues);
@@ -224,6 +232,12 @@ impl Policy {
                 .map(|d| (d.id.clone(), d.profile.clone()))
                 .collect(),
             rules,
+            gateways: spec
+                .nodes
+                .iter()
+                .filter_map(|n| Some((n.id.clone(), n.gateway?)))
+                .collect(),
+            dns,
         };
         Ok((policy, issues))
     }
@@ -232,9 +246,19 @@ impl Policy {
     pub fn rules(&self) -> &[Rule] {
         &self.rules
     }
+
+    /// The checked `dns:` section, if the bundle has one.
+    pub fn dns(&self) -> Option<&DnsConfig> {
+        self.dns.as_ref()
+    }
+
+    /// The gateway endpoint of a node, if it has one.
+    pub fn gateway(&self, node: &str) -> Option<&GatewaySpec> {
+        self.gateways.get(node)
+    }
 }
 
-fn unique_ids<'a>(
+pub(crate) fn unique_ids<'a>(
     kind: &str,
     ids: impl Iterator<Item = &'a str>,
     issues: &mut Vec<Issue>,
````

Create `crates/veyra-policy/src/dns_config.rs`:

````rust
//! The `dns:` section (spec §9): listeners, upstream groups, the TTL of
//! synthesized answers, and the checks rules need once DNS serves them.

use crate::compile::{Issue, IssueCode, Severity, issue, unique_ids};
use crate::model::{Action, BundleSpec, Mode};
use std::collections::BTreeMap;
use std::net::{IpAddr, SocketAddr};

/// Rules without `dns.upstreamGroup` resolve through this group.
pub const DEFAULT_UPSTREAM_GROUP: &str = "default";

/// The checked `dns:` section.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DnsConfig {
    pub listen: Vec<SocketAddr>,
    /// TTL of synthesized answers and of the SOA in synthesized NODATA (spec §9.5).
    pub synth_ttl: u32,
    pub upstreams: BTreeMap<String, Vec<SocketAddr>>,
}

/// `None` when the bundle has no `dns:` section.
pub(crate) fn compile_dns(spec: &BundleSpec, issues: &mut Vec<Issue>) -> Option<DnsConfig> {
    use IssueCode::*;
    let dns = spec.dns.as_ref()?;
    let error =
        |code, rules: &[&str], message: String| issue(Severity::Error, code, rules, message);

    if dns.listen.is_empty() {
        issues.push(error(InvalidValue, &[], "dns.listen is empty".into()));
    }
    let listen: Vec<SocketAddr> = dns
        .listen
        .iter()
        .filter_map(|a| socket_addr(a, None, "dns.listen", issues))
        .collect();
    if !(30..=300).contains(&dns.synth_ttl) {
        let msg = format!("dns.synthTtl is {}, expected 30 to 300", dns.synth_ttl);
        issues.push(error(InvalidValue, &[], msg));
    }

    let groups = unique_ids(
        "upstream group",
        dns.upstreams.iter().map(|g| g.id.as_str()),
        issues,
    );
    if !groups.contains(DEFAULT_UPSTREAM_GROUP) {
        let msg = format!("dns.upstreams needs a group with id {DEFAULT_UPSTREAM_GROUP}");
        issues.push(error(UnknownReference, &[], msg));
    }
    let mut upstreams = BTreeMap::new();
    for g in &dns.upstreams {
        let id = [g.id.as_str()];
        if g.servers.is_empty() {
            issues.push(error(
                InvalidValue,
                &id,
                "upstream group has no servers".into(),
            ));
        }
        let servers: Vec<SocketAddr> = g
            .servers
            .iter()
            .filter_map(|s| socket_addr(s, Some(53), "dns.upstreams.servers", issues))
            .collect();
        for server in &servers {
            if let Some(own) = listen.iter().find(|l| forwards_to_itself(*server, **l)) {
                let msg = format!("{server} is this server's own listener {own}");
                issues.push(error(UpstreamLoop, &id, msg));
            }
        }
        upstreams.insert(g.id.clone(), servers);
    }

    let has_gateway: BTreeMap<&str, bool> = spec
        .nodes
        .iter()
        .map(|n| (n.id.as_str(), n.gateway.is_some()))
        .collect();
    for r in &spec.rules {
        let id = [r.id.as_str()];
        if let Some(d) = r
            .dns
            .as_ref()
            .filter(|d| !groups.contains(d.upstream_group.as_str()))
        {
            let msg = format!("unknown upstream group {}", d.upstream_group);
            issues.push(error(UnknownReference, &id, msg));
        }
        if let Action::ProxyNode { node, .. } = &r.action {
            if r.modes_allowed.contains(&Mode::SmartdnsTcp)
                && has_gateway.get(node.as_str()) == Some(&false)
            {
                let msg = format!("node {node} has no gateway, so DNS cannot send clients to it");
                issues.push(error(MissingGateway, &id, msg));
            }
        }
    }

    Some(DnsConfig {
        listen,
        synth_ttl: dns.synth_ttl,
        upstreams,
    })
}

/// `ip:port` or `[ipv6]:port`; with a `default_port`, a bare IP too.
fn socket_addr(
    text: &str,
    default_port: Option<u16>,
    field: &str,
    issues: &mut Vec<Issue>,
) -> Option<SocketAddr> {
    let parsed = text
        .parse::<SocketAddr>()
        .ok()
        .or_else(|| Some(SocketAddr::new(text.parse::<IpAddr>().ok()?, default_port?)));
    if parsed.is_none() {
        let msg = format!("{field}: {text:?} is not a valid address");
        issues.push(issue(Severity::Error, IssueCode::InvalidAddress, &[], msg));
    }
    parsed
}

/// Forwarding to our own listener would loop forever (spec §9.4, `DNS_UPSTREAM_LOOP`).
// ponytail: catches exact and loopback matches only; another local interface
// address behind an unspecified listener needs an interface scan to detect.
fn forwards_to_itself(server: SocketAddr, listen: SocketAddr) -> bool {
    server.port() == listen.port()
        && (server.ip() == listen.ip()
            || server.ip().is_unspecified()
            || (listen.ip().is_unspecified() && server.ip().is_loopback()))
}
````

In `crates/veyra-policy/src/lib.rs`, apply this patch (save it to a file and run `git apply <file>`, or edit by hand):

````diff
diff --git a/crates/veyra-policy/src/lib.rs b/crates/veyra-policy/src/lib.rs
index 8b3ea90..309eb6b 100644
--- a/crates/veyra-policy/src/lib.rs
+++ b/crates/veyra-policy/src/lib.rs
@@ -2,12 +2,14 @@
 
 mod compile;
 mod decide;
+mod dns_config;
 mod model;
 mod name;
 mod pattern;
 
 pub use compile::{Issue, IssueCode, Policy, Rule, Severity};
 pub use decide::{DecidedBy, Decision, Subject, SubjectError};
+pub use dns_config::{DEFAULT_UPSTREAM_GROUP, DnsConfig};
 pub use model::*;
 pub use name::{Name, NameError};
 pub use pattern::{Pattern, PatternError, Specificity};
````

In `crates/veyra-policy/src/model.rs`, apply this patch (save it to a file and run `git apply <file>`, or edit by hand):

````diff
diff --git a/crates/veyra-policy/src/model.rs b/crates/veyra-policy/src/model.rs
index 0a8d090..560bcb4 100644
--- a/crates/veyra-policy/src/model.rs
+++ b/crates/veyra-policy/src/model.rs
@@ -2,6 +2,7 @@
 //! `Policy::compile` turns them into a checked policy.
 
 use serde::{Deserialize, Serialize};
+use std::net::{Ipv4Addr, Ipv6Addr};
 
 pub const API_VERSION: &str = "veyra.io/v1alpha1";
 pub const KIND: &str = "PolicyBundle";
@@ -32,12 +33,53 @@ pub struct BundleSpec {
     pub devices: Vec<DeviceSpec>,
     #[serde(default)]
     pub rules: Vec<RuleSpec>,
+    /// The DNS engine. Without it the file is a rules-only bundle and the
+    /// DNS-only checks (upstream groups, gateways) do not run.
+    #[serde(default)]
+    pub dns: Option<DnsEngineSpec>,
 }
 
 #[derive(Debug, Clone, Deserialize)]
 #[serde(deny_unknown_fields)]
 pub struct NodeSpec {
     pub id: String,
+    #[serde(default)]
+    pub gateway: Option<GatewaySpec>,
+}
+
+/// The gateway endpoint DNS hands out for mode B (spec §10.2, §17.1 `GatewayEndpoint`).
+#[derive(Debug, Clone, Copy, PartialEq, Eq, Deserialize, Serialize)]
+#[serde(deny_unknown_fields)]
+pub struct GatewaySpec {
+    pub ipv4: Ipv4Addr,
+    /// Without it, AAAA queries for proxied names get NODATA (spec §9.5).
+    #[serde(default)]
+    pub ipv6: Option<Ipv6Addr>,
+}
+
+/// The `dns:` section (spec §9). Addresses stay strings here so `compile` can
+/// report every bad one with a clear message.
+#[derive(Debug, Clone, Deserialize)]
+#[serde(deny_unknown_fields, rename_all = "camelCase")]
+pub struct DnsEngineSpec {
+    /// `ip:port` to serve DNS on, UDP and TCP.
+    pub listen: Vec<String>,
+    /// TTL of synthesized answers, 30 to 300 seconds (spec §9.5).
+    #[serde(default = "default_synth_ttl")]
+    pub synth_ttl: u32,
+    pub upstreams: Vec<UpstreamGroupSpec>,
+}
+
+fn default_synth_ttl() -> u32 {
+    60
+}
+
+#[derive(Debug, Clone, Deserialize)]
+#[serde(deny_unknown_fields)]
+pub struct UpstreamGroupSpec {
+    pub id: String,
+    /// `ip`, `ip:port` or `[ipv6]:port`; a bare IP means port 53.
+    pub servers: Vec<String>,
 }
 
 #[derive(Debug, Clone, Deserialize)]
````

- [ ] **Step 4: Run the tests to see them pass**

Run: `cargo test --workspace`
Expected: `dns_config` `ok. 9 passed`; everything else unchanged (`veyra-cli` 9, `veyra-policy` unit 50, determinism 1).

Run: `cargo run -q -p veyra-cli -- validate configs/examples/home.yaml`
Expected: `configs/examples/home.yaml: valid, 0 warning(s)`

- [ ] **Step 5: Format and lint**

Run: `cargo fmt --all && cargo clippy --workspace --all-targets -- -D warnings`
Expected: clippy prints no warnings.

- [ ] **Step 6: Commit**

````bash
git add configs/examples/home.yaml crates/veyra-policy/src/compile.rs crates/veyra-policy/src/dns_config.rs crates/veyra-policy/src/lib.rs crates/veyra-policy/src/model.rs crates/veyra-policy/tests/dns_config.rs
git commit -m "feat(policy): dns section, upstream groups and node gateways"
````

---

### Task 4: Client admission and `explain --source`

**Files:**
- Create: `crates/veyra-policy/src/admission.rs`, `crates/veyra-policy/tests/admission.rs`
- Modify: `apps/veyra-cli/src/main.rs`, `apps/veyra-cli/tests/cli.rs`, `configs/examples/home.yaml`, `crates/veyra-policy/src/compile.rs`, `crates/veyra-policy/src/lib.rs`, `crates/veyra-policy/src/model.rs`

**Interfaces:**
- Consumes: `Subject` (plan 1), `issue()`, `IssueCode` (Task 3), `BundleSpec.dns` (Task 3).
- Produces: `BundleSpec.admission: Vec<AdmissionSpec>`; `AdmissionSpec { cidr: String, profile: Option<String>, device: Option<String> }` (exactly one of profile/device); `Cidr::parse(&str) -> Result<Cidr, String>` (host bits cleared, bare IP = /32 or /128), `Cidr::contains(&self, IpAddr) -> bool` (IPv4-mapped IPv6 matches IPv4), `Cidr::prefix_len(&self) -> u8`, `impl Display` (`192.168.1.0/24`); `Admitted { subject: Subject, binding: String }`; `Policy::admit(&self, IpAddr) -> Option<Admitted>` (longest prefix wins); `IssueCode::{InvalidBinding, SourceBindingConflict, NoAdmission}` (`NoAdmission` is a warning: `dns:` present, `admission:` empty); `veyra explain --source <ip>` (conflicts with `--profile`/`--device`; not admitted → exit 1); explain JSON gains `"source"` and `"source_binding"` when `--source` is used.

- [ ] **Step 1: Write the failing tests**

In `apps/veyra-cli/tests/cli.rs`, apply this patch (save it to a file and run `git apply <file>`, or edit by hand):

````diff
diff --git a/apps/veyra-cli/tests/cli.rs b/apps/veyra-cli/tests/cli.rs
index 89259da..bb400ce 100644
--- a/apps/veyra-cli/tests/cli.rs
+++ b/apps/veyra-cli/tests/cli.rs
@@ -176,3 +176,48 @@ fn explain_reports_and_honours_the_mode() {
         text(&bad.stderr)
     );
 }
+
+#[test]
+fn explain_by_source_address_uses_admission() {
+    let out = veyra(&[
+        "explain",
+        "--config",
+        EXAMPLE,
+        "--source",
+        "192.168.1.50",
+        "media.example",
+    ]);
+    assert!(out.status.success(), "{}", text(&out.stderr));
+    let v: serde_json::Value = serde_json::from_slice(&out.stdout).expect("json on stdout");
+    assert_eq!(v["profile"], "home");
+    assert_eq!(v["device"], "tv");
+    assert_eq!(v["source"], "192.168.1.50");
+    assert_eq!(v["source_binding"], "192.168.1.50/32");
+
+    let stranger = veyra(&[
+        "explain",
+        "--config",
+        EXAMPLE,
+        "--source",
+        "10.9.9.9",
+        "media.example",
+    ]);
+    assert_eq!(stranger.status.code(), Some(1));
+    assert!(
+        text(&stranger.stderr).contains("not admitted"),
+        "{}",
+        text(&stranger.stderr)
+    );
+
+    let both = veyra(&[
+        "explain",
+        "--config",
+        EXAMPLE,
+        "--source",
+        "127.0.0.1",
+        "--profile",
+        "home",
+        "media.example",
+    ]);
+    assert_eq!(both.status.code(), Some(2));
+}
````

Create `crates/veyra-policy/tests/admission.rs`:

````rust
//! Who may use the DNS engine, and as which profile or device (spec §9.6, §17.2).

use std::net::IpAddr;
use veyra_policy::{Bundle, Cidr, Issue, IssueCode, Policy, Subject};

fn compile(admission: &str, dns: bool) -> Result<(Policy, Vec<Issue>), Vec<Issue>> {
    let dns = if dns {
        "  dns:\n    listen: [\"127.0.0.1:5353\"]\n    upstreams:\n      - { id: default, servers: [\"1.1.1.1\"] }\n"
    } else {
        ""
    };
    let text = format!(
        "apiVersion: veyra.io/v1alpha1\nkind: PolicyBundle\nmetadata: {{ name: t }}\nspec:\n  \
         profiles: [{{ id: home }}, {{ id: kids }}]\n  \
         devices: [{{ id: tv, profile: home }}]\n  \
         admission:\n{admission}{dns}"
    );
    Policy::compile(&Bundle::from_yaml(&text).expect("test YAML parses"))
}

fn errors(admission: &str) -> Vec<IssueCode> {
    compile(admission, false)
        .expect_err("expected errors")
        .iter()
        .map(|i| i.code)
        .collect()
}

fn ip(s: &str) -> IpAddr {
    s.parse().expect("test address parses")
}

#[test]
fn cidrs_parse_normalize_and_match() {
    let lan = Cidr::parse("192.168.1.5/24").unwrap();
    assert_eq!(lan.to_string(), "192.168.1.0/24");
    assert!(lan.contains(ip("192.168.1.77")));
    assert!(!lan.contains(ip("192.168.2.1")));
    assert!(
        lan.contains(ip("::ffff:192.168.1.7")),
        "IPv4-mapped IPv6 source"
    );
    assert_eq!(Cidr::parse("10.0.0.1").unwrap().to_string(), "10.0.0.1/32");
    let v6 = Cidr::parse("2001:db8::/32").unwrap();
    assert!(v6.contains(ip("2001:db8:1::1")));
    assert!(!v6.contains(ip("192.168.1.1")));
    assert_eq!(Cidr::parse("0.0.0.0/0").unwrap().prefix_len(), 0);
    for bad in [
        "192.168.1.0/33",
        "::/129",
        "abc",
        "10.0.0.0/-1",
        "/24",
        "10.0.0.0/",
    ] {
        assert!(Cidr::parse(bad).is_err(), "{bad}");
    }
}

#[test]
fn the_longest_prefix_decides_who_the_source_is() {
    let admission = "    - { cidr: 192.168.1.0/24, profile: home }\n    \
                     - { cidr: 192.168.1.50/32, device: tv }\n";
    let (policy, _) = compile(admission, false).unwrap();
    let tv = policy.admit(ip("192.168.1.50")).unwrap();
    assert_eq!(
        tv.subject,
        Subject {
            profile: "home".into(),
            device: Some("tv".into())
        }
    );
    assert_eq!(tv.binding, "192.168.1.50/32");
    let other = policy.admit(ip("192.168.1.7")).unwrap();
    assert_eq!(
        (other.subject.device, other.binding.as_str()),
        (None, "192.168.1.0/24")
    );
    assert!(policy.admit(ip("10.0.0.1")).is_none());
}

#[test]
fn overlapping_bindings_of_different_profiles_conflict() {
    let admission = "    - { cidr: 192.168.1.0/24, profile: home }\n    \
                     - { cidr: 192.168.1.128/25, profile: kids }\n";
    assert_eq!(errors(admission), vec![IssueCode::SourceBindingConflict]);
}

#[test]
fn the_same_network_twice_conflicts() {
    let admission = "    - { cidr: 192.168.1.0/24, profile: home }\n    \
                     - { cidr: 192.168.1.0/24, profile: home }\n";
    assert_eq!(errors(admission), vec![IssueCode::SourceBindingConflict]);
}

#[test]
fn rejects_bad_bindings() {
    let admission = "    - { cidr: 10.0.0.0/8 }\n    \
                     - { cidr: 10.1.0.0/16, profile: home, device: tv }\n    \
                     - { cidr: 10.2.0.0/16, profile: nobody }\n    \
                     - { cidr: 10.3.0.0/33, profile: home }\n";
    assert_eq!(
        errors(admission),
        vec![
            IssueCode::InvalidBinding,
            IssueCode::InvalidBinding,
            IssueCode::UnknownReference,
            IssueCode::InvalidAddress
        ]
    );
}

#[test]
fn warns_when_dns_admits_nobody() {
    let (_, warnings) = compile("    []\n", true).unwrap();
    let codes: Vec<IssueCode> = warnings.iter().map(|i| i.code).collect();
    assert_eq!(codes, vec![IssueCode::NoAdmission]);
}
````

- [ ] **Step 2: Run the tests to see them fail**

Run: `cargo test -p veyra-policy --test admission`
Expected: compilation fails: ``unresolved import `veyra_policy::Cidr` ``, ``no method named `admit` ``, and no `IssueCode` variants `InvalidBinding`, `SourceBindingConflict`, `NoAdmission`.

Run: `cargo test -p veyra-cli`
Expected: `explain_by_source_address_uses_admission ... FAILED` (``unexpected argument '--source' found``); `test result: FAILED. 9 passed; 1 failed`.

- [ ] **Step 3: Write the implementation**

In `apps/veyra-cli/src/main.rs`, apply this patch (save it to a file and run `git apply <file>`, or edit by hand):

````diff
diff --git a/apps/veyra-cli/src/main.rs b/apps/veyra-cli/src/main.rs
index c07e5b1..2f1e86f 100644
--- a/apps/veyra-cli/src/main.rs
+++ b/apps/veyra-cli/src/main.rs
@@ -2,6 +2,7 @@
 
 use clap::{Parser, Subcommand};
 use serde::Serialize;
+use std::net::IpAddr;
 use std::path::{Path, PathBuf};
 use std::process::ExitCode;
 use veyra_policy::{Bundle, Decision, Issue, Mode, Name, Policy};
@@ -17,14 +18,17 @@ struct Cli {
 enum Command {
     /// Check a policy file and print every error and warning.
     Validate { config: PathBuf },
-    /// Show which rule decides a domain for a profile or device, as JSON.
+    /// Show which rule decides a domain for a profile, device or source address, as JSON.
     Explain {
         #[arg(long)]
         config: PathBuf,
-        #[arg(long, required_unless_present = "device")]
+        #[arg(long, required_unless_present_any = ["device", "source"])]
         profile: Option<String>,
         #[arg(long)]
         device: Option<String>,
+        /// Client address; its profile comes from `admission` like a DNS query's would.
+        #[arg(long, conflicts_with_all = ["profile", "device"])]
+        source: Option<IpAddr>,
         /// Connection mode: smartdns_tcp, explicit_proxy or tunnel.
         #[arg(long, default_value = "smartdns_tcp")]
         mode: Mode,
@@ -39,6 +43,10 @@ struct Explain<'a> {
     #[serde(flatten)]
     decision: &'a Decision,
     mode: Mode,
+    #[serde(skip_serializing_if = "Option::is_none")]
+    source: Option<IpAddr>,
+    #[serde(skip_serializing_if = "Option::is_none")]
+    source_binding: Option<String>,
 }
 
 fn main() -> ExitCode {
@@ -72,6 +80,7 @@ fn run(cli: Cli) -> Result<(), String> {
             config,
             profile,
             device,
+            source,
             mode,
             domain,
         } => {
@@ -88,14 +97,27 @@ fn run(cli: Cli) -> Result<(), String> {
                     ));
                 }
             };
-            let subject = policy
-                .subject(profile.as_deref(), device.as_deref())
-                .map_err(|e| e.to_string())?;
+            let (subject, source_binding) = match source {
+                Some(ip) => {
+                    let admitted = policy
+                        .admit(ip)
+                        .ok_or_else(|| format!("source {ip} is not admitted (spec §9.6)"))?;
+                    (admitted.subject, Some(admitted.binding))
+                }
+                None => {
+                    let subject = policy
+                        .subject(profile.as_deref(), device.as_deref())
+                        .map_err(|e| e.to_string())?;
+                    (subject, None)
+                }
+            };
             let name = Name::parse(&domain).map_err(|e| format!("{domain:?}: {e}"))?;
             let decision = policy.decide_in(&subject, &name, Some(mode));
             let explain = Explain {
                 decision: &decision,
                 mode,
+                source,
+                source_binding,
             };
             let json = serde_json::to_string_pretty(&explain).map_err(|e| e.to_string())?;
             println!("{json}");
````

In `configs/examples/home.yaml`, apply this patch (save it to a file and run `git apply <file>`, or edit by hand):

````diff
diff --git a/configs/examples/home.yaml b/configs/examples/home.yaml
index 4a39a4f..7c2a2ba 100644
--- a/configs/examples/home.yaml
+++ b/configs/examples/home.yaml
@@ -49,6 +49,11 @@ spec:
         domains: ["suffix:ads.example"]
       action:
         type: block
+  admission:                            # who may use this DNS (§9.6); others are dropped
+    - { cidr: 127.0.0.0/8, profile: home }
+    - { cidr: "::1/128", profile: home }
+    - { cidr: 192.168.1.0/24, profile: home }
+    - { cidr: 192.168.1.50/32, device: tv }
   dns:
     listen: ["127.0.0.1:5353"]          # production: ["0.0.0.0:53", "[::]:53"]
     synthTtl: 60
````

Create `crates/veyra-policy/src/admission.rs`:

````rust
//! Client admission (spec §9.6): which source networks may use the DNS engine,
//! and which profile or device each one is. This is the static CIDR part;
//! link-IP, DDNS and DoT/DoH tokens arrive in later plans.

use crate::compile::{Issue, IssueCode, Policy, Severity, issue};
use crate::decide::Subject;
use crate::model::BundleSpec;
use std::fmt;
use std::net::IpAddr;

/// An IP network such as `192.168.1.0/24` or `2001:db8::/32`. A bare address is
/// one host. Host bits are cleared, so `192.168.1.5/24` is `192.168.1.0/24`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Cidr {
    network: IpAddr,
    prefix_len: u8,
}

impl Cidr {
    pub fn parse(text: &str) -> Result<Cidr, String> {
        let bad = || format!("{text:?} is not an IP network like 192.168.1.0/24");
        let (addr, len) = match text.split_once('/') {
            Some((addr, len)) => (addr, Some(len)),
            None => (text, None),
        };
        let addr: IpAddr = addr.parse().map_err(|_| bad())?;
        let max = if addr.is_ipv4() { 32 } else { 128 };
        let prefix_len = match len {
            None => max,
            Some(len) => len
                .parse::<u8>()
                .ok()
                .filter(|len| *len <= max)
                .ok_or_else(bad)?,
        };
        Ok(Cidr {
            network: mask(addr, prefix_len),
            prefix_len,
        })
    }

    pub fn prefix_len(&self) -> u8 {
        self.prefix_len
    }

    /// IPv4-mapped IPv6 sources (`::ffff:a.b.c.d`), as a dual-stack socket
    /// reports IPv4 clients, match IPv4 networks.
    pub fn contains(&self, ip: IpAddr) -> bool {
        let ip = ip.to_canonical();
        ip.is_ipv4() == self.network.is_ipv4() && mask(ip, self.prefix_len) == self.network
    }

    fn overlaps(&self, other: &Cidr) -> bool {
        self.contains(other.network) || other.contains(self.network)
    }
}

impl fmt::Display for Cidr {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}/{}", self.network, self.prefix_len)
    }
}

fn mask(ip: IpAddr, prefix_len: u8) -> IpAddr {
    let len = u32::from(prefix_len);
    match ip {
        IpAddr::V4(v4) => {
            let mask = u32::MAX.checked_shl(32 - len).unwrap_or(0);
            IpAddr::V4((u32::from(v4) & mask).into())
        }
        IpAddr::V6(v6) => {
            let mask = u128::MAX.checked_shl(128 - len).unwrap_or(0);
            IpAddr::V6((u128::from(v6) & mask).into())
        }
    }
}

/// An admitted source: who it is, and the `admission` network that said so.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Admitted {
    pub subject: Subject,
    pub binding: String,
}

impl Policy {
    /// Who a source address is (spec §9.6): the longest matching `admission`
    /// network wins. `None` means not admitted, and DNS drops the query.
    pub fn admit(&self, ip: IpAddr) -> Option<Admitted> {
        self.bindings
            .iter()
            .filter(|(cidr, _)| cidr.contains(ip))
            .max_by_key(|(cidr, _)| cidr.prefix_len())
            .map(|(cidr, subject)| Admitted {
                subject: subject.clone(),
                binding: cidr.to_string(),
            })
    }
}

pub(crate) fn compile_admission(
    spec: &BundleSpec,
    issues: &mut Vec<Issue>,
) -> Vec<(Cidr, Subject)> {
    use IssueCode::*;
    let error = |code, message: String| issue(Severity::Error, code, &[], message);
    let mut bindings = Vec::new();
    for a in &spec.admission {
        let subject = match (&a.profile, &a.device) {
            (Some(p), None) if spec.profiles.iter().any(|known| &known.id == p) => Subject {
                profile: p.clone(),
                device: None,
            },
            (Some(p), None) => {
                let msg = format!("admission {}: unknown profile {p}", a.cidr);
                issues.push(error(UnknownReference, msg));
                continue;
            }
            (None, Some(d)) => match spec.devices.iter().find(|known| &known.id == d) {
                Some(device) => Subject {
                    profile: device.profile.clone(),
                    device: Some(d.clone()),
                },
                None => {
                    let msg = format!("admission {}: unknown device {d}", a.cidr);
                    issues.push(error(UnknownReference, msg));
                    continue;
                }
            },
            _ => {
                let msg = format!(
                    "admission {}: give exactly one of profile or device",
                    a.cidr
                );
                issues.push(error(InvalidBinding, msg));
                continue;
            }
        };
        match Cidr::parse(&a.cidr) {
            Ok(cidr) => bindings.push((cidr, subject)),
            Err(e) => issues.push(error(InvalidAddress, format!("admission: {e}"))),
        }
    }

    // One source address must map to exactly one profile (spec §17.2).
    for (i, (a, first)) in bindings.iter().enumerate() {
        for (b, second) in &bindings[i + 1..] {
            if a == b {
                issues.push(error(
                    SourceBindingConflict,
                    format!("admission lists {a} twice"),
                ));
            } else if a.overlaps(b) && first.profile != second.profile {
                let msg = format!(
                    "{a} ({}) and {b} ({}) overlap; one source must map to one profile",
                    first.profile, second.profile
                );
                issues.push(error(SourceBindingConflict, msg));
            }
        }
    }
    if spec.dns.is_some() && spec.admission.is_empty() {
        let msg = "dns is configured but admission is empty, so every query is dropped";
        issues.push(issue(Severity::Warning, NoAdmission, &[], msg.into()));
    }
    bindings
}
````

In `crates/veyra-policy/src/compile.rs`, apply this patch (save it to a file and run `git apply <file>`, or edit by hand):

````diff
diff --git a/crates/veyra-policy/src/compile.rs b/crates/veyra-policy/src/compile.rs
index 3d19093..c9959e2 100644
--- a/crates/veyra-policy/src/compile.rs
+++ b/crates/veyra-policy/src/compile.rs
@@ -1,6 +1,8 @@
 //! Turns a parsed `Bundle` into a checked `Policy`, collecting every problem
 //! instead of stopping at the first one (spec §8.2.1 validator rules, §17.2).
 
+use crate::admission::{Cidr, compile_admission};
+use crate::decide::Subject;
 use crate::dns_config::{DnsConfig, compile_dns};
 use crate::model::{
     API_VERSION, Action, Bundle, DefaultAction, DnsSpec, GatewaySpec, KIND, Mode, OnFailure, Scope,
@@ -32,6 +34,9 @@ pub enum IssueCode {
     InvalidValue,
     MissingGateway,
     UpstreamLoop,
+    InvalidBinding,
+    SourceBindingConflict,
+    NoAdmission,
 }
 
 #[derive(Debug, Clone, PartialEq, Eq)]
@@ -92,6 +97,7 @@ pub struct Policy {
     pub(crate) rules: Vec<Rule>,
     pub(crate) gateways: BTreeMap<String, GatewaySpec>,
     pub(crate) dns: Option<DnsConfig>,
+    pub(crate) bindings: Vec<(Cidr, Subject)>,
 }
 
 pub(crate) fn issue(severity: Severity, code: IssueCode, rules: &[&str], message: String) -> Issue {
@@ -215,6 +221,7 @@ impl Policy {
         }
         check_overlaps(&rules, &mut issues);
         let dns = compile_dns(spec, &mut issues);
+        let bindings = compile_admission(spec, &mut issues);
 
         if issues.iter().any(|i| i.severity == Error) {
             return Err(issues);
@@ -238,6 +245,7 @@ impl Policy {
                 .filter_map(|n| Some((n.id.clone(), n.gateway?)))
                 .collect(),
             dns,
+            bindings,
         };
         Ok((policy, issues))
     }
````

In `crates/veyra-policy/src/lib.rs`, apply this patch (save it to a file and run `git apply <file>`, or edit by hand):

````diff
diff --git a/crates/veyra-policy/src/lib.rs b/crates/veyra-policy/src/lib.rs
index 309eb6b..1a4ec0c 100644
--- a/crates/veyra-policy/src/lib.rs
+++ b/crates/veyra-policy/src/lib.rs
@@ -1,5 +1,6 @@
 //! Veyra policy core: domain names, patterns, rule precedence and explain (spec §8).
 
+mod admission;
 mod compile;
 mod decide;
 mod dns_config;
@@ -7,6 +8,7 @@ mod model;
 mod name;
 mod pattern;
 
+pub use admission::{Admitted, Cidr};
 pub use compile::{Issue, IssueCode, Policy, Rule, Severity};
 pub use decide::{DecidedBy, Decision, Subject, SubjectError};
 pub use dns_config::{DEFAULT_UPSTREAM_GROUP, DnsConfig};
````

In `crates/veyra-policy/src/model.rs`, apply this patch (save it to a file and run `git apply <file>`, or edit by hand):

````diff
diff --git a/crates/veyra-policy/src/model.rs b/crates/veyra-policy/src/model.rs
index 560bcb4..6ebb772 100644
--- a/crates/veyra-policy/src/model.rs
+++ b/crates/veyra-policy/src/model.rs
@@ -37,6 +37,20 @@ pub struct BundleSpec {
     /// DNS-only checks (upstream groups, gateways) do not run.
     #[serde(default)]
     pub dns: Option<DnsEngineSpec>,
+    /// Source networks allowed to use DNS (spec §9.6). Empty admits nobody.
+    #[serde(default)]
+    pub admission: Vec<AdmissionSpec>,
+}
+
+/// One `admission` entry: a network and exactly one of `profile` or `device`.
+#[derive(Debug, Clone, Deserialize)]
+#[serde(deny_unknown_fields)]
+pub struct AdmissionSpec {
+    pub cidr: String,
+    #[serde(default)]
+    pub profile: Option<String>,
+    #[serde(default)]
+    pub device: Option<String>,
 }
 
 #[derive(Debug, Clone, Deserialize)]
````

- [ ] **Step 4: Run the tests to see them pass**

Run: `cargo test --workspace`
Expected: `admission` `ok. 6 passed`, `veyra-cli` `ok. 10 passed`; `dns_config` 9, `veyra-policy` unit 50, determinism 1.

- [ ] **Step 5: Format and lint**

Run: `cargo fmt --all && cargo clippy --workspace --all-targets -- -D warnings`
Expected: clippy prints no warnings.

- [ ] **Step 6: Commit**

````bash
git add apps/veyra-cli/src/main.rs apps/veyra-cli/tests/cli.rs configs/examples/home.yaml crates/veyra-policy/src/admission.rs crates/veyra-policy/src/compile.rs crates/veyra-policy/src/lib.rs crates/veyra-policy/src/model.rs crates/veyra-policy/tests/admission.rs
git commit -m "feat(policy,cli): CIDR client admission and explain --source"
````

---

### Task 5: The DNS answer table and `explain` `dns_answer`

**Files:**
- Create: `crates/veyra-policy/src/answer.rs`, `crates/veyra-policy/tests/dns_plan.rs`
- Modify: `apps/veyra-cli/src/main.rs`, `apps/veyra-cli/tests/cli.rs`, `crates/veyra-policy/src/lib.rs`

**Interfaces:**
- Consumes: `Decision` (plan 1), `Policy::gateway`, `DEFAULT_UPSTREAM_GROUP`, `DnsConfig.synth_ttl` (Task 3), `decide_in` (Task 2), `admit` (Task 4).
- Produces: `QueryKind::{A, Aaaa, Https, Svcb, Other}`; `DnsPlan::{Forward { group: String }, Addresses(Vec<IpAddr>), NoData}`; `Policy::dns_plan(&self, decision: &Decision, kind: QueryKind) -> DnsPlan` — the spec §9.5 table, for decisions made with `Some(Mode::SmartdnsTcp)`; explain JSON gains `"dns_answer": { "A", "AAAA", "HTTPS", "ttl" }` (values: an address list, `"NODATA"` or `"forward:<group>"`) when the config has `dns:` and the mode is `smartdns_tcp`.

- [ ] **Step 1: Write the failing tests**

In `apps/veyra-cli/tests/cli.rs`, apply this patch (save it to a file and run `git apply <file>`, or edit by hand):

````diff
diff --git a/apps/veyra-cli/tests/cli.rs b/apps/veyra-cli/tests/cli.rs
index bb400ce..a1f94db 100644
--- a/apps/veyra-cli/tests/cli.rs
+++ b/apps/veyra-cli/tests/cli.rs
@@ -221,3 +221,28 @@ fn explain_by_source_address_uses_admission() {
     ]);
     assert_eq!(both.status.code(), Some(2));
 }
+
+#[test]
+fn explain_shows_the_dns_answer_in_dns_mode() {
+    let run = |args: &[&str]| {
+        let mut full = vec!["explain", "--config", EXAMPLE];
+        full.extend_from_slice(args);
+        let out = veyra(&full);
+        assert!(out.status.success(), "{}", text(&out.stderr));
+        serde_json::from_slice::<serde_json::Value>(&out.stdout).expect("json on stdout")
+    };
+    let proxied = run(&["--profile", "home", "media.example"]);
+    assert_eq!(
+        proxied["dns_answer"]["A"],
+        serde_json::json!(["203.0.113.10"])
+    );
+    assert_eq!(proxied["dns_answer"]["AAAA"], "NODATA");
+    assert_eq!(proxied["dns_answer"]["HTTPS"], "NODATA");
+    assert_eq!(proxied["dns_answer"]["ttl"], 60);
+
+    let direct = run(&["--profile", "home", "example.org"]);
+    assert_eq!(direct["dns_answer"]["A"], "forward:default");
+
+    let tunnel = run(&["--profile", "home", "--mode", "tunnel", "media.example"]);
+    assert!(tunnel.get("dns_answer").is_none(), "{tunnel}");
+}
````

Create `crates/veyra-policy/tests/dns_plan.rs`:

````rust
//! REQ-DNS-002: what DNS answers for each route and query kind (spec §9.5).

use std::net::IpAddr;
use veyra_policy::{Bundle, DnsPlan, Mode, Name, Policy, QueryKind};

const BUNDLE: &str = r#"
apiVersion: veyra.io/v1alpha1
kind: PolicyBundle
metadata: { name: plan }
spec:
  nodes:
    - { id: de-01, gateway: { ipv4: 203.0.113.10, ipv6: "2001:db8::10" } }
    - { id: us-01, gateway: { ipv4: 198.51.100.7 } }
  profiles: [{ id: home }]
  admission: [{ cidr: 127.0.0.0/8, profile: home }]
  rules:
    - { id: corp, scope: default, match: { domains: [corp.example] }, dns: { upstreamGroup: corp }, action: { type: direct } }
    - { id: ads, scope: default, match: { domains: [ads.example] }, action: { type: block } }
    - { id: dual, scope: default, match: { domains: [dual.example] }, action: { type: proxy_node, node: de-01 } }
    - { id: v4, scope: default, match: { domains: [v4.example] }, action: { type: proxy_node, node: us-01 } }
  dns:
    listen: ["127.0.0.1:5353"]
    upstreams:
      - { id: default, servers: ["1.1.1.1"] }
      - { id: corp, servers: ["10.0.0.53"] }
"#;

fn plans(domain: &str) -> [DnsPlan; 5] {
    let (policy, _) = Policy::compile(&Bundle::from_yaml(BUNDLE).expect("test YAML parses"))
        .expect("test bundle compiles");
    let subject = policy.subject(Some("home"), None).expect("home exists");
    let decision = policy.decide_in(
        &subject,
        &Name::parse(domain).expect("test domain parses"),
        Some(Mode::SmartdnsTcp),
    );
    [
        QueryKind::A,
        QueryKind::Aaaa,
        QueryKind::Https,
        QueryKind::Svcb,
        QueryKind::Other,
    ]
    .map(|kind| policy.dns_plan(&decision, kind))
}

fn forward(group: &str) -> DnsPlan {
    DnsPlan::Forward {
        group: group.into(),
    }
}

fn addresses(ip: &str) -> DnsPlan {
    DnsPlan::Addresses(vec![ip.parse::<IpAddr>().expect("test address parses")])
}

#[test]
fn direct_forwards_every_kind_to_the_default_group() {
    assert_eq!(plans("other.example"), [(); 5].map(|_| forward("default")));
}

#[test]
fn direct_with_an_upstream_group_forwards_there() {
    assert_eq!(plans("corp.example"), [(); 5].map(|_| forward("corp")));
}

#[test]
fn block_answers_null_addresses_and_nodata() {
    assert_eq!(
        plans("ads.example"),
        [
            addresses("0.0.0.0"),
            addresses("::"),
            DnsPlan::NoData,
            DnsPlan::NoData,
            DnsPlan::NoData,
        ]
    );
}

#[test]
fn proxy_answers_the_gateway_and_hides_https_records() {
    assert_eq!(
        plans("dual.example"),
        [
            addresses("203.0.113.10"),
            addresses("2001:db8::10"),
            DnsPlan::NoData,
            DnsPlan::NoData,
            forward("default"),
        ]
    );
}

#[test]
fn proxy_without_gateway_ipv6_never_leaks_the_origin_aaaa() {
    assert_eq!(
        plans("v4.example"),
        [
            addresses("198.51.100.7"),
            DnsPlan::NoData,
            DnsPlan::NoData,
            DnsPlan::NoData,
            forward("default"),
        ]
    );
}
````

- [ ] **Step 2: Run the tests to see them fail**

Run: `cargo test -p veyra-policy --test dns_plan`
Expected: compilation fails: ``unresolved imports `veyra_policy::DnsPlan`, `veyra_policy::QueryKind` ``, ``no method named `dns_plan` ``.

Run: `cargo test -p veyra-cli`
Expected: `explain_shows_the_dns_answer_in_dns_mode ... FAILED`; `test result: FAILED. 10 passed; 1 failed`.

- [ ] **Step 3: Write the implementation**

In `apps/veyra-cli/src/main.rs`, apply this patch (save it to a file and run `git apply <file>`, or edit by hand):

````diff
diff --git a/apps/veyra-cli/src/main.rs b/apps/veyra-cli/src/main.rs
index 2f1e86f..c8a39b3 100644
--- a/apps/veyra-cli/src/main.rs
+++ b/apps/veyra-cli/src/main.rs
@@ -5,7 +5,7 @@ use serde::Serialize;
 use std::net::IpAddr;
 use std::path::{Path, PathBuf};
 use std::process::ExitCode;
-use veyra_policy::{Bundle, Decision, Issue, Mode, Name, Policy};
+use veyra_policy::{Bundle, Decision, DnsPlan, Issue, Mode, Name, Policy, QueryKind};
 
 #[derive(Parser)]
 #[command(name = "veyra", version, about = "Veyra policy tools")]
@@ -47,6 +47,37 @@ struct Explain<'a> {
     source: Option<IpAddr>,
     #[serde(skip_serializing_if = "Option::is_none")]
     source_binding: Option<String>,
+    /// Present when the config has a `dns:` section and the mode is smartdns_tcp.
+    #[serde(skip_serializing_if = "Option::is_none")]
+    dns_answer: Option<DnsAnswer>,
+}
+
+/// What the DNS engine would answer (spec §9.5): addresses, `"NODATA"`, or
+/// `"forward:<group>"`.
+#[derive(Serialize)]
+struct DnsAnswer {
+    #[serde(rename = "A")]
+    a: serde_json::Value,
+    #[serde(rename = "AAAA")]
+    aaaa: serde_json::Value,
+    #[serde(rename = "HTTPS")]
+    https: serde_json::Value,
+    ttl: u32,
+}
+
+fn dns_answer(policy: &Policy, decision: &Decision) -> Option<DnsAnswer> {
+    let config = policy.dns()?;
+    let show = |kind| match policy.dns_plan(decision, kind) {
+        DnsPlan::Forward { group } => serde_json::json!(format!("forward:{group}")),
+        DnsPlan::Addresses(ips) => serde_json::json!(ips),
+        DnsPlan::NoData => serde_json::json!("NODATA"),
+    };
+    Some(DnsAnswer {
+        a: show(QueryKind::A),
+        aaaa: show(QueryKind::Aaaa),
+        https: show(QueryKind::Https),
+        ttl: config.synth_ttl,
+    })
 }
 
 fn main() -> ExitCode {
@@ -118,6 +149,11 @@ fn run(cli: Cli) -> Result<(), String> {
                 mode,
                 source,
                 source_binding,
+                dns_answer: if mode == Mode::SmartdnsTcp {
+                    dns_answer(&policy, &decision)
+                } else {
+                    None
+                },
             };
             let json = serde_json::to_string_pretty(&explain).map_err(|e| e.to_string())?;
             println!("{json}");
````

Create `crates/veyra-policy/src/answer.rs`:

````rust
//! What DNS answers for a decision: the table of spec §9.5. Pure, so the DNS
//! engine and `veyra explain` share it without either owning the wire format.

use crate::compile::Policy;
use crate::decide::Decision;
use crate::dns_config::DEFAULT_UPSTREAM_GROUP;
use crate::model::{Action, GatewaySpec};
use std::net::{IpAddr, Ipv4Addr, Ipv6Addr};

/// The query types §9.5 treats differently. Everything else, including any
/// class other than IN, is `Other`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum QueryKind {
    A,
    Aaaa,
    Https,
    Svcb,
    Other,
}

/// How to answer one query.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum DnsPlan {
    /// Ask this upstream group and pass its answer through unchanged.
    Forward { group: String },
    /// Answer with these addresses (A or AAAA by family), TTL `synthTtl`.
    Addresses(Vec<IpAddr>),
    /// NOERROR without records, with a synthesized SOA (spec §9.5 NODATA).
    NoData,
}

impl Policy {
    /// The §9.5 answer for `decision`, which must come from
    /// `decide_in(.., Some(Mode::SmartdnsTcp))`: only rules that allow
    /// `smartdns_tcp` may send DNS clients to a gateway.
    pub fn dns_plan(&self, decision: &Decision, kind: QueryKind) -> DnsPlan {
        use QueryKind::*;
        let forward = || DnsPlan::Forward {
            group: decision
                .upstream_group
                .clone()
                .unwrap_or_else(|| DEFAULT_UPSTREAM_GROUP.into()),
        };
        match &decision.route {
            Action::Direct {} => forward(),
            // Null addresses fail fast; NXDOMAIN would be cached and invite
            // clients to try a fallback resolver (spec §9.5).
            Action::Block {} => match kind {
                A => DnsPlan::Addresses(vec![Ipv4Addr::UNSPECIFIED.into()]),
                Aaaa => DnsPlan::Addresses(vec![Ipv6Addr::UNSPECIFIED.into()]),
                _ => DnsPlan::NoData,
            },
            Action::ProxyNode { node, .. } => match (kind, self.gateway(node)) {
                (A, Some(gateway)) => DnsPlan::Addresses(vec![gateway.ipv4.into()]),
                (Aaaa, Some(GatewaySpec { ipv6: Some(v6), .. })) => {
                    DnsPlan::Addresses(vec![(*v6).into()])
                }
                // Other types do not steer the connection, so they resolve normally.
                (Other, _) => forward(),
                // HTTPS/SVCB carry origin address hints and ECH (spec §9.5), an AAAA
                // without a gateway IPv6 would leak the origin: answer nothing.
                _ => DnsPlan::NoData,
            },
        }
    }
}
````

In `crates/veyra-policy/src/lib.rs`, apply this patch (save it to a file and run `git apply <file>`, or edit by hand):

````diff
diff --git a/crates/veyra-policy/src/lib.rs b/crates/veyra-policy/src/lib.rs
index 1a4ec0c..5c26895 100644
--- a/crates/veyra-policy/src/lib.rs
+++ b/crates/veyra-policy/src/lib.rs
@@ -1,6 +1,7 @@
 //! Veyra policy core: domain names, patterns, rule precedence and explain (spec §8).
 
 mod admission;
+mod answer;
 mod compile;
 mod decide;
 mod dns_config;
@@ -9,6 +10,7 @@ mod name;
 mod pattern;
 
 pub use admission::{Admitted, Cidr};
+pub use answer::{DnsPlan, QueryKind};
 pub use compile::{Issue, IssueCode, Policy, Rule, Severity};
 pub use decide::{DecidedBy, Decision, Subject, SubjectError};
 pub use dns_config::{DEFAULT_UPSTREAM_GROUP, DnsConfig};
````

- [ ] **Step 4: Run the tests to see them pass**

Run: `cargo test --workspace`
Expected: `dns_plan` `ok. 5 passed`, `veyra-cli` `ok. 11 passed`; `admission` 6, `dns_config` 9, `veyra-policy` unit 50, determinism 1.

Run: `cargo run -q -p veyra-cli -- explain --config configs/examples/home.yaml --source 192.168.1.50 www.media.example`
Expected: JSON ending with `"source": "192.168.1.50"`, `"source_binding": "192.168.1.50/32"` and `"dns_answer": { "A": ["203.0.113.10"], "AAAA": "NODATA", "HTTPS": "NODATA", "ttl": 60 }`.

- [ ] **Step 5: Format and lint**

Run: `cargo fmt --all && cargo clippy --workspace --all-targets -- -D warnings`
Expected: clippy prints no warnings.

- [ ] **Step 6: Commit**

````bash
git add apps/veyra-cli/src/main.rs apps/veyra-cli/tests/cli.rs crates/veyra-policy/src/answer.rs crates/veyra-policy/src/lib.rs crates/veyra-policy/tests/dns_plan.rs
git commit -m "feat(policy,cli): DNS answer plan (spec §9.5) and explain dns_answer"
````

---

### Task 6: `veyra-dns`: the engine

**Files:**
- Create: `crates/veyra-dns/Cargo.toml`, `crates/veyra-dns/src/engine.rs`, `crates/veyra-dns/src/lib.rs`, `crates/veyra-dns/src/synth.rs`, `crates/veyra-dns/src/upstream.rs`, `crates/veyra-dns/tests/engine.rs`, `crates/veyra-dns/tests/support/mod.rs`
- Modify: `Cargo.toml`

**Interfaces:**
- Consumes: `Policy::{admit, decide_in, default_decision, dns_plan, dns}`, `DnsConfig`, `DnsPlan`, `QueryKind`, `Mode`, `Name`, `Subject` (Tasks 2–5).
- Produces (crate `veyra-dns`): `Engine::new(Policy) -> Result<Engine, String>` (fails without `dns:`); `Engine::with_upstream_timeout(self, Duration) -> Engine` (default 2 s per server); `Engine::policy(&self) -> &Policy`; `async Engine::handle(&self, source: IpAddr, bytes: &[u8], transport: Transport) -> Option<Vec<u8>>` (`None` = send nothing); `Transport::{Udp, Tcp}`. Internal: `synth::{reply, addresses, no_data, MAX_UDP_PAYLOAD = 1232}`, `upstream::{exchange, answers}`.
- Behaviour: not admitted or not a query → `None`; opcode ≠ QUERY → NOTIMP; question count ≠ 1 → FORMERR; synthesized replies have RA=1, AA=0, AD=0, the client's id/question/RD, and an OPT echoing DO when the query had EDNS; NODATA carries a SOA whose TTL and minimum are `synthTtl`; forwarding tries each server of the group over UDP (fresh socket, RD=1, reply must match id and question) and answers SERVFAIL when all fail; a UDP reply larger than the client's limit (512, or its EDNS size capped at 1232) is sent truncated with TC=1.
- Test support (`crates/veyra-dns/tests/support/mod.rs`): `upstream(reply_fn) -> Upstream { addr, queries() }`, `dead_upstream() -> SocketAddr`, `a_records(&Message, &[Ipv4Addr], ttl) -> Message`, `query(name, RecordType) -> Message` (id `0x4242`, RD=1), `engine(&[SocketAddr]) -> Engine` (127.0.0.1 = `home`, 127.0.0.2 = `strict` with default block; `suffix:ads.example` blocked; `suffix:media.example` via de-01 = 203.0.113.10; 300 ms upstream timeout), `ask(&Engine, source, &Message, Transport) -> Option<Message>`.

- [ ] **Step 1: Write the failing tests**

In `Cargo.toml`, apply this patch (save it to a file and run `git apply <file>`, or edit by hand):

````diff
diff --git a/Cargo.toml b/Cargo.toml
index 092ea2a..f57b182 100644
--- a/Cargo.toml
+++ b/Cargo.toml
@@ -1,6 +1,6 @@
 [workspace]
 resolver = "3"
-members = ["crates/veyra-policy", "apps/veyra-cli"]
+members = ["crates/veyra-policy", "crates/veyra-dns", "apps/veyra-cli"]
 
 [workspace.package]
 version = "0.1.0"
@@ -10,12 +10,17 @@ publish = false
 
 [workspace.dependencies]
 veyra-policy = { path = "crates/veyra-policy" }
+veyra-dns = { path = "crates/veyra-dns" }
 serde = { version = "1.0.229", features = ["derive"] }
 serde-saphyr = "1.3.0"
 serde_json = "1.0.154"
 idna = "1.1.0"
 thiserror = "2.0.21"
 clap = { version = "4.6.7", features = ["derive"] }
+hickory-proto = { version = "0.26.3", default-features = false, features = ["std"] }
+tokio = { version = "1.53.2", features = ["macros", "rt-multi-thread", "net", "time", "io-util", "sync", "signal"] }
+tracing = "0.1.44"
+tracing-subscriber = { version = "0.3.23", features = ["env-filter"] }
 
 [workspace.lints.rust]
 unsafe_code = "forbid"
````

Create `crates/veyra-dns/Cargo.toml`:

````toml
[package]
name = "veyra-dns"
version.workspace = true
edition.workspace = true
rust-version.workspace = true
publish.workspace = true

[dependencies]
veyra-policy.workspace = true
hickory-proto.workspace = true
tokio.workspace = true
tracing.workspace = true

[lints]
workspace = true
````

Create `crates/veyra-dns/src/lib.rs`:

````rust
//! Veyra DNS engine (spec §9).
````

Create `crates/veyra-dns/tests/engine.rs`:

````rust
//! The DNS engine: admission, synthesis (spec §9.5) and forwarding.

mod support;

use hickory_proto::op::{Edns, Message, MessageType, OpCode, ResponseCode};
use hickory_proto::rr::{RData, RecordType};
use std::net::{IpAddr, Ipv4Addr};
use support::{a_records, ask, dead_upstream, engine, query, upstream};
use veyra_dns::Transport::{Tcp, Udp};

fn addresses(reply: &Message) -> Vec<IpAddr> {
    reply
        .answers
        .iter()
        .filter_map(|r| match &r.data {
            RData::A(a) => Some(IpAddr::V4(a.0)),
            RData::AAAA(aaaa) => Some(IpAddr::V6(aaaa.0)),
            _ => None,
        })
        .collect()
}

#[tokio::test]
async fn drops_sources_that_are_not_admitted() {
    let engine = engine(&[dead_upstream().await]);
    let reply = ask(
        &engine,
        "10.9.9.9",
        &query("ads.example.", RecordType::A),
        Udp,
    )
    .await;
    assert!(reply.is_none());
}

#[tokio::test]
async fn drops_garbage_and_responses() {
    let engine = engine(&[dead_upstream().await]);
    let source = "127.0.0.1".parse().unwrap();
    assert!(engine.handle(source, &[1, 2, 3], Udp).await.is_none());
    let mut response = query("ads.example.", RecordType::A);
    response.metadata.message_type = MessageType::Response;
    assert!(ask(&engine, "127.0.0.1", &response, Udp).await.is_none());
}

#[tokio::test]
async fn blocked_names_get_null_addresses() {
    let engine = engine(&[dead_upstream().await]);
    let reply = ask(
        &engine,
        "127.0.0.1",
        &query("WWW.Ads.Example.", RecordType::A),
        Udp,
    )
    .await
    .unwrap();
    assert_eq!(reply.metadata.id, 0x4242);
    assert_eq!(reply.metadata.response_code, ResponseCode::NoError);
    assert!(reply.metadata.recursion_available);
    assert!(!reply.metadata.authentic_data);
    assert_eq!(reply.queries[0].name().to_ascii(), "WWW.Ads.Example.");
    assert_eq!(addresses(&reply), vec![IpAddr::V4(Ipv4Addr::UNSPECIFIED)]);
    assert_eq!(reply.answers[0].ttl, 60);
}

#[tokio::test]
async fn proxied_names_get_the_gateway_and_nodata_elsewhere() {
    let engine = engine(&[dead_upstream().await]);
    let a = ask(
        &engine,
        "127.0.0.1",
        &query("media.example.", RecordType::A),
        Udp,
    )
    .await
    .unwrap();
    assert_eq!(
        addresses(&a),
        vec!["203.0.113.10".parse::<IpAddr>().unwrap()]
    );
    for record_type in [RecordType::AAAA, RecordType::HTTPS] {
        let reply = ask(
            &engine,
            "127.0.0.1",
            &query("media.example.", record_type),
            Udp,
        )
        .await
        .unwrap();
        assert_eq!(reply.metadata.response_code, ResponseCode::NoError);
        assert!(reply.answers.is_empty(), "{record_type}");
        let soa = &reply.authorities[0];
        assert_eq!(soa.ttl, 60);
        let RData::SOA(soa) = &soa.data else {
            panic!("expected a SOA, got {:?}", soa.data)
        };
        assert_eq!(soa.minimum, 60);
    }
}

#[tokio::test]
async fn direct_names_are_forwarded() {
    let origin = Ipv4Addr::new(93, 184, 215, 14);
    let up = upstream(move |q| a_records(q, &[origin], 300)).await;
    let engine = engine(&[up.addr]);
    let reply = ask(
        &engine,
        "127.0.0.1",
        &query("example.org.", RecordType::A),
        Udp,
    )
    .await
    .unwrap();
    assert_eq!(addresses(&reply), vec![IpAddr::V4(origin)]);
    assert_eq!(reply.answers[0].ttl, 300);
    assert_eq!(up.queries(), 1);
}

#[tokio::test]
async fn names_that_do_not_normalize_use_the_profile_default() {
    let engine = engine(&[dead_upstream().await]);
    let reply = ask(
        &engine,
        "127.0.0.2",
        &query("*.example.", RecordType::A),
        Udp,
    )
    .await
    .unwrap();
    assert_eq!(addresses(&reply), vec![IpAddr::V4(Ipv4Addr::UNSPECIFIED)]);
}

#[tokio::test]
async fn dead_upstreams_give_servfail() {
    let engine = engine(&[dead_upstream().await]);
    let reply = ask(
        &engine,
        "127.0.0.1",
        &query("example.org.", RecordType::A),
        Udp,
    )
    .await
    .unwrap();
    assert_eq!(reply.metadata.response_code, ResponseCode::ServFail);
}

#[tokio::test]
async fn a_dead_upstream_falls_through_to_the_next() {
    let origin = Ipv4Addr::new(93, 184, 215, 14);
    let up = upstream(move |q| a_records(q, &[origin], 300)).await;
    let engine = engine(&[dead_upstream().await, up.addr]);
    let reply = ask(
        &engine,
        "127.0.0.1",
        &query("example.org.", RecordType::A),
        Udp,
    )
    .await
    .unwrap();
    assert_eq!(addresses(&reply), vec![IpAddr::V4(origin)]);
}

#[tokio::test]
async fn other_opcodes_and_question_counts_are_refused_politely() {
    let engine = engine(&[dead_upstream().await]);
    let mut status = query("example.org.", RecordType::A);
    status.metadata.op_code = OpCode::Status;
    let reply = ask(&engine, "127.0.0.1", &status, Udp).await.unwrap();
    assert_eq!(reply.metadata.response_code, ResponseCode::NotImp);

    let mut two = query("a.example.", RecordType::A);
    two.add_queries(query("b.example.", RecordType::A).queries);
    let reply = ask(&engine, "127.0.0.1", &two, Udp).await.unwrap();
    assert_eq!(reply.metadata.response_code, ResponseCode::FormErr);
}

#[tokio::test]
async fn edns_queries_get_edns_answers_without_the_ad_bit() {
    let engine = engine(&[dead_upstream().await]);
    let mut q = query("ads.example.", RecordType::A);
    let mut edns = Edns::new();
    edns.set_max_payload(1232);
    edns.set_dnssec_ok(true);
    q.set_edns(edns);
    let reply = ask(&engine, "127.0.0.1", &q, Udp).await.unwrap();
    let edns = reply.edns.expect("an OPT record in the reply");
    assert!(edns.flags().dnssec_ok);
    assert!(!reply.metadata.authentic_data);
    assert!(
        reply
            .answers
            .iter()
            .all(|r| r.record_type() != RecordType::RRSIG)
    );
}

#[tokio::test]
async fn oversized_udp_answers_are_truncated_but_tcp_gets_everything() {
    let many: Vec<Ipv4Addr> = (1..=40).map(|i| Ipv4Addr::new(192, 0, 2, i)).collect();
    let up = upstream(move |q| a_records(q, &many, 300)).await;
    let engine = engine(&[up.addr]);
    let q = query("big.example.", RecordType::A);

    let udp = ask(&engine, "127.0.0.1", &q, Udp).await.unwrap();
    assert!(udp.metadata.truncation);
    assert!(udp.answers.is_empty());

    let tcp = ask(&engine, "127.0.0.1", &q, Tcp).await.unwrap();
    assert!(!tcp.metadata.truncation);
    assert_eq!(tcp.answers.len(), 40);
}
````

Create `crates/veyra-dns/tests/support/mod.rs`:

````rust
//! Shared test helpers: a fake upstream DNS server, query builders and an engine
//! over a small policy.
#![allow(dead_code)] // each test file uses a different subset

use hickory_proto::op::{Message, MessageType, OpCode, Query};
use hickory_proto::rr::rdata::A;
use hickory_proto::rr::{Name, RData, Record, RecordType};
use std::net::{Ipv4Addr, SocketAddr};
use std::sync::Arc;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::time::Duration;
use tokio::net::UdpSocket;
use veyra_dns::{Engine, Transport};
use veyra_policy::{Bundle, Policy};

/// A fake upstream and how many queries it has received.
pub struct Upstream {
    pub addr: SocketAddr,
    queries: Arc<AtomicUsize>,
}

impl Upstream {
    pub fn queries(&self) -> usize {
        self.queries.load(Ordering::SeqCst)
    }
}

/// A UDP DNS server on 127.0.0.1 that answers every query with `reply(query)`.
pub async fn upstream(reply: impl Fn(&Message) -> Message + Send + Sync + 'static) -> Upstream {
    let socket = UdpSocket::bind("127.0.0.1:0")
        .await
        .expect("bind fake upstream");
    let addr = socket.local_addr().expect("fake upstream address");
    let queries = Arc::new(AtomicUsize::new(0));
    let counter = queries.clone();
    tokio::spawn(async move {
        let mut buf = vec![0; 4096];
        loop {
            let Ok((len, from)) = socket.recv_from(&mut buf).await else {
                continue;
            };
            let Ok(query) = Message::from_vec(&buf[..len]) else {
                continue;
            };
            counter.fetch_add(1, Ordering::SeqCst);
            let bytes = reply(&query).to_vec().expect("encode fake reply");
            let _ = socket.send_to(&bytes, from).await;
        }
    });
    Upstream { addr, queries }
}

/// An address where nothing listens.
pub async fn dead_upstream() -> SocketAddr {
    let socket = UdpSocket::bind("127.0.0.1:0").await.expect("bind");
    socket.local_addr().expect("address")
}

/// A reply to `query` carrying `ips` as A records.
pub fn a_records(query: &Message, ips: &[Ipv4Addr], ttl: u32) -> Message {
    let mut reply = Message::response(query.metadata.id, OpCode::Query);
    reply.metadata.recursion_desired = query.metadata.recursion_desired;
    reply.metadata.recursion_available = true;
    reply.add_queries(query.queries.iter().cloned());
    for ip in ips {
        let name = query.queries[0].name().clone();
        reply.add_answer(Record::from_rdata(name, ttl, RData::A(A(*ip))));
    }
    reply
}

pub fn query(name: &str, record_type: RecordType) -> Message {
    let mut message = Message::new(0x4242, MessageType::Query, OpCode::Query);
    message.metadata.recursion_desired = true;
    message.add_query(Query::query(
        Name::from_ascii(name).expect("test name"),
        record_type,
    ));
    message
}

/// An engine for a policy with:
/// - 127.0.0.1 admitted as `home` (default direct), 127.0.0.2 as `strict` (default block);
/// - `suffix:ads.example` blocked, `suffix:media.example` proxied via de-01 (203.0.113.10);
/// - the `default` upstream group set to `servers`.
pub fn engine(servers: &[SocketAddr]) -> Engine {
    let servers = servers
        .iter()
        .map(|s| format!("\"{s}\""))
        .collect::<Vec<_>>()
        .join(", ");
    let yaml = format!(
        r#"
apiVersion: veyra.io/v1alpha1
kind: PolicyBundle
metadata: {{ name: engine }}
spec:
  nodes: [{{ id: de-01, gateway: {{ ipv4: 203.0.113.10 }} }}]
  profiles: [{{ id: home }}, {{ id: strict, defaultAction: block }}]
  admission:
    - {{ cidr: 127.0.0.1/32, profile: home }}
    - {{ cidr: 127.0.0.2/32, profile: strict }}
  rules:
    - {{ id: ads, scope: default, match: {{ domains: ["suffix:ads.example"] }}, action: {{ type: block }} }}
    - {{ id: media, scope: default, match: {{ domains: ["suffix:media.example"] }}, action: {{ type: proxy_node, node: de-01 }} }}
  dns:
    listen: ["127.0.0.1:5353"]
    upstreams:
      - {{ id: default, servers: [{servers}] }}
"#
    );
    let bundle = Bundle::from_yaml(&yaml).expect("test YAML parses");
    let (policy, _) = Policy::compile(&bundle).expect("test bundle compiles");
    Engine::new(policy)
        .expect("policy has a dns section")
        .with_upstream_timeout(Duration::from_millis(300))
}

/// Send `query` from `source` and decode the reply, if any.
pub async fn ask(
    engine: &Engine,
    source: &str,
    query: &Message,
    transport: Transport,
) -> Option<Message> {
    let bytes = query.to_vec().expect("encode query");
    let source = source.parse().expect("test source address");
    let reply = engine.handle(source, &bytes, transport).await?;
    Some(Message::from_vec(&reply).expect("engine replies with valid DNS"))
}
````

- [ ] **Step 2: Run the tests to see them fail**

Run: `cargo test -p veyra-dns`
Expected: compilation fails: ``unresolved imports `veyra_dns::Engine`, `veyra_dns::Transport` `` (the `[u8]` size errors that follow come from the same missing `handle`).

- [ ] **Step 3: Write the implementation**

Create `crates/veyra-dns/src/engine.rs`:

````rust
//! One DNS query in, at most one reply out (spec §9).

use crate::synth::{self, MAX_UDP_PAYLOAD};
use crate::upstream;
use hickory_proto::op::{Message, MessageType, OpCode, Query, ResponseCode};
use hickory_proto::rr::{DNSClass, RecordType};
use std::net::IpAddr;
use std::time::Duration;
use veyra_policy::{DnsConfig, DnsPlan, Mode, Name, Policy, QueryKind, Subject};

/// How the query arrived: UDP replies must fit the client's buffer.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Transport {
    Udp,
    Tcp,
}

pub struct Engine {
    policy: Policy,
    config: DnsConfig,
    upstream_timeout: Duration,
}

impl Engine {
    /// Fails when the policy has no `dns:` section.
    pub fn new(policy: Policy) -> Result<Engine, String> {
        let config = policy
            .dns()
            .cloned()
            .ok_or("the config has no dns section")?;
        Ok(Engine {
            policy,
            config,
            upstream_timeout: Duration::from_secs(2),
        })
    }

    /// How long to wait for each upstream server before trying the next.
    pub fn with_upstream_timeout(mut self, wait: Duration) -> Engine {
        self.upstream_timeout = wait;
        self
    }

    pub fn policy(&self) -> &Policy {
        &self.policy
    }

    /// Answer one DNS message from `source`. `None` means send nothing: the
    /// source is not admitted (spec §9.6), or the bytes are not a DNS query.
    pub async fn handle(
        &self,
        source: IpAddr,
        bytes: &[u8],
        transport: Transport,
    ) -> Option<Vec<u8>> {
        let admitted = self.policy.admit(source)?;
        let request = Message::from_vec(bytes).ok()?;
        if request.metadata.message_type != MessageType::Query {
            return None;
        }
        let reply = self.respond(&admitted.subject, &request).await;
        encode(&request, reply, transport)
    }

    async fn respond(&self, subject: &Subject, request: &Message) -> Message {
        if request.metadata.op_code != OpCode::Query {
            return synth::reply(request, ResponseCode::NotImp);
        }
        let [query] = request.queries.as_slice() else {
            return synth::reply(request, ResponseCode::FormErr);
        };
        let asked = query.name().to_ascii();
        let decision = match Name::parse(&asked) {
            Ok(name) => self
                .policy
                .decide_in(subject, &name, Some(Mode::SmartdnsTcp)),
            Err(_) => self.policy.default_decision(subject, &asked),
        };
        match self.policy.dns_plan(&decision, kind(query)) {
            DnsPlan::Addresses(ips) => synth::addresses(request, &ips, self.config.synth_ttl),
            DnsPlan::NoData => synth::no_data(request, self.config.synth_ttl),
            DnsPlan::Forward { group } => self.forward(request, &group).await,
        }
    }

    async fn forward(&self, request: &Message, group: &str) -> Message {
        let servers = self
            .config
            .upstreams
            .get(group)
            .map_or(&[][..], Vec::as_slice);
        match upstream::exchange(servers, request, self.upstream_timeout).await {
            Some(reply) => reply,
            None => synth::reply(request, ResponseCode::ServFail),
        }
    }
}

fn kind(query: &Query) -> QueryKind {
    if query.query_class() != DNSClass::IN {
        return QueryKind::Other;
    }
    match query.query_type() {
        RecordType::A => QueryKind::A,
        RecordType::AAAA => QueryKind::Aaaa,
        RecordType::HTTPS => QueryKind::Https,
        RecordType::SVCB => QueryKind::Svcb,
        _ => QueryKind::Other,
    }
}

/// Over UDP, a reply larger than the client can take becomes a truncated
/// reply (TC set, no records) and the client retries over TCP.
fn encode(request: &Message, reply: Message, transport: Transport) -> Option<Vec<u8>> {
    let bytes = reply.to_vec().ok()?;
    if transport == Transport::Udp && bytes.len() > udp_limit(request) {
        return reply.truncate().to_vec().ok();
    }
    Some(bytes)
}

/// 512 bytes without EDNS, else the client's advertised size, at most 1232.
fn udp_limit(request: &Message) -> usize {
    request.edns.as_ref().map_or(512, |edns| {
        usize::from(edns.max_payload().clamp(512, MAX_UDP_PAYLOAD))
    })
}
````

Replace `crates/veyra-dns/src/lib.rs` with:

````rust
//! Veyra DNS engine (spec §9): admission, answer synthesis and forwarding.
//! Policy decisions come from `veyra-policy`; this crate owns the wire format.

mod engine;
mod synth;
mod upstream;

pub use engine::{Engine, Transport};
````

Create `crates/veyra-dns/src/synth.rs`:

````rust
//! Responses Veyra writes itself (spec §9.5): synthesized addresses, NODATA
//! with a synthesized SOA, and error codes.

use hickory_proto::op::{Edns, Message, ResponseCode};
use hickory_proto::rr::rdata::{A, AAAA, SOA};
use hickory_proto::rr::{Name, RData, Record};
use std::net::IpAddr;

/// The largest UDP reply Veyra sends or advertises (DNS Flag Day 2020).
pub(crate) const MAX_UDP_PAYLOAD: u16 = 1232;

/// A reply skeleton for `request`: same id, opcode, question and RD; RA set;
/// AA and AD clear, since synthesized data is never DNSSEC-validated (spec §9.5);
/// an OPT record when the request had one, echoing its DO bit.
pub(crate) fn reply(request: &Message, code: ResponseCode) -> Message {
    let mut reply = Message::response(request.metadata.id, request.metadata.op_code);
    reply.metadata.recursion_desired = request.metadata.recursion_desired;
    reply.metadata.recursion_available = true;
    reply.metadata.response_code = code;
    reply.add_queries(request.queries.iter().cloned());
    if let Some(asked) = &request.edns {
        let mut edns = Edns::new();
        edns.set_max_payload(MAX_UDP_PAYLOAD);
        edns.set_dnssec_ok(asked.flags().dnssec_ok);
        reply.set_edns(edns);
    }
    reply
}

/// A and AAAA records for the question name.
pub(crate) fn addresses(request: &Message, ips: &[IpAddr], ttl: u32) -> Message {
    let mut reply = reply(request, ResponseCode::NoError);
    if let Some(query) = request.queries.first() {
        for ip in ips {
            let data = match ip {
                IpAddr::V4(v4) => RData::A(A(*v4)),
                IpAddr::V6(v6) => RData::AAAA(AAAA(*v6)),
            };
            reply.add_answer(Record::from_rdata(query.name().clone(), ttl, data));
        }
    }
    reply
}

/// NOERROR without records. The SOA's TTL and minimum are both `ttl`, so
/// negative caching (the smaller of the two, RFC 2308) lasts `ttl` (spec §9.5).
pub(crate) fn no_data(request: &Message, ttl: u32) -> Message {
    let mut reply = reply(request, ResponseCode::NoError);
    if let Some(query) = request.queries.first() {
        let name = |text: &str| Name::from_ascii(text).unwrap_or_else(|_| Name::root());
        let soa = SOA::new(
            name("veyra.invalid."),
            name("hostmaster.veyra.invalid."),
            1,
            3600,
            600,
            86400,
            ttl,
        );
        reply.add_authority(Record::from_rdata(
            query.name().clone(),
            ttl,
            RData::SOA(soa),
        ));
    }
    reply
}
````

Create `crates/veyra-dns/src/upstream.rs`:

````rust
//! Forwarding to upstream resolvers.

use hickory_proto::op::{Message, MessageType};
use std::io;
use std::net::{Ipv4Addr, Ipv6Addr, SocketAddr};
use std::time::Duration;
use tokio::net::UdpSocket;
use tokio::time::timeout;

/// Ask each server in turn; the first valid reply wins. `None` when every
/// server failed, which the caller turns into SERVFAIL (spec §20.3).
pub(crate) async fn exchange(
    servers: &[SocketAddr],
    request: &Message,
    wait: Duration,
) -> Option<Message> {
    let mut query = request.clone();
    query.metadata.recursion_desired = true;
    let bytes = query.to_vec().ok()?;
    for &server in servers {
        match udp(server, &bytes, &query, wait).await {
            Ok(reply) => return Some(reply),
            Err(e) => tracing::debug!(%server, error = %e, "upstream failed"),
        }
    }
    None
}

/// One query over a fresh socket: the OS picks a random source port, which
/// with the client's random id makes off-path spoofing hard (spec §19.1).
// ponytail: a socket per query; pool sockets if upstream QPS ever matters.
async fn udp(
    server: SocketAddr,
    bytes: &[u8],
    query: &Message,
    wait: Duration,
) -> io::Result<Message> {
    let local: SocketAddr = if server.is_ipv4() {
        (Ipv4Addr::UNSPECIFIED, 0).into()
    } else {
        (Ipv6Addr::UNSPECIFIED, 0).into()
    };
    let socket = UdpSocket::bind(local).await?;
    socket.connect(server).await?;
    socket.send(bytes).await?;
    let mut buf = vec![0; 65_535];
    timeout(wait, async {
        loop {
            let len = socket.recv(&mut buf).await?;
            if let Ok(reply) = Message::from_vec(&buf[..len]) {
                if answers(&reply, query) {
                    return Ok(reply);
                }
            }
        }
    })
    .await
    .map_err(|_| io::Error::new(io::ErrorKind::TimedOut, "no reply"))?
}

/// A reply belongs to our query only with the same id and question.
pub(crate) fn answers(reply: &Message, query: &Message) -> bool {
    reply.metadata.message_type == MessageType::Response
        && reply.metadata.id == query.metadata.id
        && reply.queries == query.queries
}
````

- [ ] **Step 4: Run the tests to see them pass**

Run: `cargo test -p veyra-dns`
Expected: `engine` `ok. 11 passed`.

Run: `cargo test --workspace`
Expected: every suite passes.

- [ ] **Step 5: Format and lint**

Run: `cargo fmt --all && cargo clippy --workspace --all-targets -- -D warnings`
Expected: clippy prints no warnings.

- [ ] **Step 6: Commit**

````bash
git add Cargo.lock Cargo.toml crates/veyra-dns/Cargo.toml crates/veyra-dns/src/engine.rs crates/veyra-dns/src/lib.rs crates/veyra-dns/src/synth.rs crates/veyra-dns/src/upstream.rs crates/veyra-dns/tests/engine.rs crates/veyra-dns/tests/support/mod.rs
git commit -m "feat(dns): engine with admission, synthesis and UDP forwarding"
````

---

### Task 7: Cache and TCP fallback to upstreams

**Files:**
- Create: `crates/veyra-dns/src/cache.rs`, `crates/veyra-dns/tests/cache.rs`
- Modify: `crates/veyra-dns/src/engine.rs`, `crates/veyra-dns/src/lib.rs`, `crates/veyra-dns/src/upstream.rs`, `crates/veyra-dns/tests/support/mod.rs`

**Interfaces:**
- Consumes: `Engine`, `synth`, `upstream::{exchange, answers}` (Task 6).
- Produces (internal): `cache::Key::new(request: &Message, group: &str) -> Option<Key>` (lowercased name, type, class, group, EDNS present, DO, CD); `cache::Cache::new(capacity)`, `Cache::get(&mut self, &Key, now: Instant) -> Option<Message>` (TTLs reduced by age; `None` once expired), `Cache::put(&mut self, Key, &Message, now: Instant)` (TTL per RFC 2308: smallest answer TTL, or min(SOA TTL, SOA minimum) for NXDOMAIN/NODATA; capped at a day; SERVFAIL, truncated, TTL 0 and SOA-less negatives are not cached; a full cache only drops expired entries); `upstream::{read_frame, write_frame}` (`pub(crate)`, 2-byte length prefix) for Task 8.
- Behaviour: the engine (10 000 entries) answers repeated questions from the cache, re-addressed to the new request (its id, RD and question spelling); a truncated upstream UDP reply is retried over TCP on the same server.
- Test support: `truncating_upstream(reply_fn) -> Upstream` (UDP replies truncated, TCP on the same port answers in full).

- [ ] **Step 1: Write the failing tests**

Create `crates/veyra-dns/src/cache.rs`:

````rust
#[cfg(test)]
mod tests {
    use super::*;
    use hickory_proto::op::{Edns, MessageType, OpCode, Query};
    use hickory_proto::rr::rdata::{A, SOA};
    use hickory_proto::rr::{Name, Record};
    use std::net::Ipv4Addr;
    use std::time::Duration;

    fn request(name: &str, record_type: RecordType) -> Message {
        let mut message = Message::new(1, MessageType::Query, OpCode::Query);
        message.add_query(Query::query(Name::from_ascii(name).unwrap(), record_type));
        message
    }

    fn answer(ttls: &[u32]) -> Message {
        let request = request("example.org.", RecordType::A);
        let mut reply = Message::response(1, OpCode::Query);
        reply.add_queries(request.queries.clone());
        for ttl in ttls {
            let data = RData::A(A(Ipv4Addr::LOCALHOST));
            reply.add_answer(Record::from_rdata(
                request.queries[0].name().clone(),
                *ttl,
                data,
            ));
        }
        reply
    }

    fn negative(code: ResponseCode, soa_ttl: u32, minimum: u32) -> Message {
        let mut reply = answer(&[]);
        reply.metadata.response_code = code;
        let soa = SOA::new(Name::root(), Name::root(), 1, 1, 1, 1, minimum);
        reply.add_authority(Record::from_rdata(Name::root(), soa_ttl, RData::SOA(soa)));
        reply
    }

    #[test]
    fn positive_answers_live_for_their_smallest_ttl_capped_at_a_day() {
        assert_eq!(cache_ttl(&answer(&[300, 60])), Some(60));
        assert_eq!(cache_ttl(&answer(&[1_000_000])), Some(86_400));
        assert_eq!(cache_ttl(&answer(&[0])), None);
    }

    #[test]
    fn negative_answers_live_for_the_smaller_soa_ttl() {
        assert_eq!(
            cache_ttl(&negative(ResponseCode::NXDomain, 900, 60)),
            Some(60)
        );
        assert_eq!(
            cache_ttl(&negative(ResponseCode::NoError, 30, 600)),
            Some(30)
        );
        assert_eq!(cache_ttl(&answer(&[])), None, "NODATA without a SOA");
    }

    #[test]
    fn failures_and_truncated_replies_are_not_cached() {
        assert_eq!(cache_ttl(&negative(ResponseCode::ServFail, 60, 60)), None);
        let mut truncated = answer(&[300]);
        truncated.metadata.truncation = true;
        assert_eq!(cache_ttl(&truncated), None);
    }

    #[test]
    fn hits_age_their_ttls_and_expire() {
        let mut cache = Cache::new(10);
        let key = Key::new(&request("example.org.", RecordType::A), "default").unwrap();
        let start = Instant::now();
        cache.put(key.clone(), &answer(&[300, 60]), start);
        let hit = cache.get(&key, start + Duration::from_secs(20)).unwrap();
        let ttls: Vec<u32> = hit.answers.iter().map(|r| r.ttl).collect();
        assert_eq!(ttls, vec![280, 40]);
        assert!(cache.get(&key, start + Duration::from_secs(60)).is_none());
    }

    #[test]
    fn keys_ignore_name_case_but_not_type_group_or_dnssec() {
        let base = Key::new(&request("Example.ORG.", RecordType::A), "default");
        assert_eq!(
            base,
            Key::new(&request("example.org.", RecordType::A), "default")
        );
        assert_ne!(
            base,
            Key::new(&request("example.org.", RecordType::AAAA), "default")
        );
        assert_ne!(
            base,
            Key::new(&request("example.org.", RecordType::A), "corp")
        );
        let mut dnssec = request("example.org.", RecordType::A);
        let mut edns = Edns::new();
        edns.set_dnssec_ok(true);
        dnssec.set_edns(edns);
        assert_ne!(base, Key::new(&dnssec, "default"));
    }

    #[test]
    fn a_full_cache_makes_room_only_by_dropping_expired_entries() {
        let mut cache = Cache::new(1);
        let start = Instant::now();
        let first = Key::new(&request("a.example.", RecordType::A), "default").unwrap();
        let second = Key::new(&request("b.example.", RecordType::A), "default").unwrap();
        cache.put(first, &answer(&[60]), start);
        let later = start + Duration::from_secs(1);
        cache.put(second.clone(), &answer(&[60]), later);
        assert!(cache.get(&second, later).is_none(), "full of live entries");
        let expired = start + Duration::from_secs(61);
        cache.put(second.clone(), &answer(&[60]), expired);
        assert!(
            cache.get(&second, expired).is_some(),
            "the expired entry made room"
        );
    }
}
````

In `crates/veyra-dns/src/lib.rs`, apply this patch (save it to a file and run `git apply <file>`, or edit by hand):

````diff
diff --git a/crates/veyra-dns/src/lib.rs b/crates/veyra-dns/src/lib.rs
index 31646a3..d8ece32 100644
--- a/crates/veyra-dns/src/lib.rs
+++ b/crates/veyra-dns/src/lib.rs
@@ -1,6 +1,7 @@
 //! Veyra DNS engine (spec §9): admission, answer synthesis and forwarding.
 //! Policy decisions come from `veyra-policy`; this crate owns the wire format.
 
+mod cache;
 mod engine;
 mod synth;
 mod upstream;
````

Create `crates/veyra-dns/tests/cache.rs`:

````rust
//! Caching upstream answers (spec §9.3) and TCP fallback to upstreams.

mod support;

use hickory_proto::op::{Message, ResponseCode};
use hickory_proto::rr::RecordType;
use std::net::Ipv4Addr;
use support::{a_records, ask, engine, query, truncating_upstream, upstream};
use veyra_dns::Transport::{Tcp, Udp};

const ORIGIN: Ipv4Addr = Ipv4Addr::new(93, 184, 215, 14);

#[tokio::test]
async fn a_repeated_question_is_answered_from_the_cache() {
    let up = upstream(|q| a_records(q, &[ORIGIN], 300)).await;
    let engine = engine(&[up.addr]);
    ask(
        &engine,
        "127.0.0.1",
        &query("example.org.", RecordType::A),
        Udp,
    )
    .await
    .unwrap();

    let mut again = query("EXAMPLE.org.", RecordType::A);
    again.metadata.id = 0x0bad;
    let reply = ask(&engine, "127.0.0.1", &again, Udp).await.unwrap();
    assert_eq!(
        up.queries(),
        1,
        "the second question must not reach the upstream"
    );
    assert_eq!(reply.metadata.id, 0x0bad);
    assert_eq!(reply.queries[0].name().to_ascii(), "EXAMPLE.org.");
    assert_eq!(reply.answers.len(), 1);
}

#[tokio::test]
async fn different_record_types_are_cached_separately() {
    let up = upstream(|q| a_records(q, &[ORIGIN], 300)).await;
    let engine = engine(&[up.addr]);
    for record_type in [RecordType::A, RecordType::TXT, RecordType::A] {
        ask(
            &engine,
            "127.0.0.1",
            &query("example.org.", record_type),
            Udp,
        )
        .await
        .unwrap();
    }
    assert_eq!(up.queries(), 2);
}

#[tokio::test]
async fn servfail_is_not_cached() {
    let up = upstream(|q| {
        let mut reply = a_records(q, &[], 0);
        reply.metadata.response_code = ResponseCode::ServFail;
        reply
    })
    .await;
    let engine = engine(&[up.addr]);
    for _ in 0..2 {
        let reply = ask(
            &engine,
            "127.0.0.1",
            &query("example.org.", RecordType::A),
            Udp,
        )
        .await
        .unwrap();
        assert_eq!(reply.metadata.response_code, ResponseCode::ServFail);
    }
    assert_eq!(up.queries(), 2);
}

#[tokio::test]
async fn a_truncated_upstream_reply_is_retried_over_tcp() {
    let many: Vec<Ipv4Addr> = (1..=40).map(|i| Ipv4Addr::new(192, 0, 2, i)).collect();
    let up = truncating_upstream(move |q: &Message| a_records(q, &many, 300)).await;
    let engine = engine(&[up.addr]);
    let reply = ask(
        &engine,
        "127.0.0.1",
        &query("big.example.", RecordType::A),
        Tcp,
    )
    .await
    .unwrap();
    assert!(!reply.metadata.truncation);
    assert_eq!(reply.answers.len(), 40);
}

#[tokio::test]
async fn cached_answers_never_cross_into_another_profile() {
    // REQ-DNS-004: `home` (default direct) caches example.org; `strict`
    // (default block) asking the same name must still be blocked.
    let up = upstream(|q| a_records(q, &[ORIGIN], 300)).await;
    let engine = engine(&[up.addr]);
    let q = query("example.org.", RecordType::A);
    let home = ask(&engine, "127.0.0.1", &q, Udp).await.unwrap();
    assert_eq!(home.answers.len(), 1);
    let strict = ask(&engine, "127.0.0.2", &q, Udp).await.unwrap();
    let blocked = matches!(&strict.answers[..], [r] if r.data == hickory_proto::rr::RData::A(hickory_proto::rr::rdata::A(Ipv4Addr::UNSPECIFIED)));
    assert!(blocked, "{strict:?}");
    assert_eq!(up.queries(), 1);
}
````

In `crates/veyra-dns/tests/support/mod.rs`, apply this patch (save it to a file and run `git apply <file>`, or edit by hand):

````diff
diff --git a/crates/veyra-dns/tests/support/mod.rs b/crates/veyra-dns/tests/support/mod.rs
index d2e9199..726e572 100644
--- a/crates/veyra-dns/tests/support/mod.rs
+++ b/crates/veyra-dns/tests/support/mod.rs
@@ -9,7 +9,8 @@ use std::net::{Ipv4Addr, SocketAddr};
 use std::sync::Arc;
 use std::sync::atomic::{AtomicUsize, Ordering};
 use std::time::Duration;
-use tokio::net::UdpSocket;
+use tokio::io::{AsyncReadExt, AsyncWriteExt};
+use tokio::net::{TcpListener, UdpSocket};
 use veyra_dns::{Engine, Transport};
 use veyra_policy::{Bundle, Policy};
 
@@ -50,6 +51,40 @@ pub async fn upstream(reply: impl Fn(&Message) -> Message + Send + Sync + 'stati
     Upstream { addr, queries }
 }
 
+/// An upstream that answers UDP with a truncated reply (TC set, no records) and
+/// TCP, on the same port, with `reply(query)` in full.
+pub async fn truncating_upstream(
+    reply: impl Fn(&Message) -> Message + Send + Sync + Clone + 'static,
+) -> Upstream {
+    let full = reply.clone();
+    let udp = upstream(move |q| full(q).truncate()).await;
+    let listener = TcpListener::bind(udp.addr)
+        .await
+        .expect("bind TCP on the same port");
+    tokio::spawn(async move {
+        loop {
+            let Ok((mut stream, _)) = listener.accept().await else {
+                continue;
+            };
+            let reply = reply.clone();
+            tokio::spawn(async move {
+                let mut len = [0u8; 2];
+                stream.read_exact(&mut len).await.expect("length prefix");
+                let mut buf = vec![0; usize::from(u16::from_be_bytes(len))];
+                stream.read_exact(&mut buf).await.expect("query");
+                let query = Message::from_vec(&buf).expect("valid query");
+                let bytes = reply(&query).to_vec().expect("encode fake reply");
+                let len = u16::try_from(bytes.len())
+                    .expect("reply fits")
+                    .to_be_bytes();
+                stream.write_all(&len).await.expect("write length");
+                stream.write_all(&bytes).await.expect("write reply");
+            });
+        }
+    });
+    udp
+}
+
 /// An address where nothing listens.
 pub async fn dead_upstream() -> SocketAddr {
     let socket = UdpSocket::bind("127.0.0.1:0").await.expect("bind");
````

- [ ] **Step 2: Run the tests to see them fail**

Run: `cargo test -p veyra-dns --lib`
Expected: compilation fails: ``cannot find type `Cache` ``, ``cannot find type `Key` ``, ``cannot find function `cache_ttl` `` (plus the imports the module does not have yet).

Run: `cargo test -p veyra-dns --test cache`
Expected: `test result: FAILED. 2 passed; 3 failed` — `a_repeated_question_is_answered_from_the_cache` (`left: 2, right: 1`), `different_record_types_are_cached_separately` (`left: 3, right: 2`) and `a_truncated_upstream_reply_is_retried_over_tcp` fail. `servfail_is_not_cached` and `cached_answers_never_cross_into_another_profile` already pass because nothing is cached yet; they guard that the cache never stores failures and never leaks one profile's answer to another (REQ-DNS-004).

- [ ] **Step 3: Write the implementation**

Insert above the `#[cfg(test)]` line of `crates/veyra-dns/src/cache.rs`:

````rust
//! Upstream answers cached for their TTL (spec §9.3). Synthesized answers are
//! never cached: they come from the policy on every query.

use hickory_proto::op::{Message, ResponseCode};
use hickory_proto::rr::{DNSClass, RData, RecordType};
use std::collections::HashMap;
use std::time::Instant;

/// No answer is kept longer than a day.
const MAX_TTL: u32 = 86_400;

/// Everything that changes what the upstream answers.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub(crate) struct Key {
    name: String,
    record_type: RecordType,
    class: DNSClass,
    group: String,
    edns: bool,
    dnssec_ok: bool,
    checking_disabled: bool,
}

impl Key {
    /// `None` for a request without a question.
    pub(crate) fn new(request: &Message, group: &str) -> Option<Key> {
        let query = request.queries.first()?;
        Some(Key {
            name: query.name().to_lowercase().to_ascii(),
            record_type: query.query_type(),
            class: query.query_class(),
            group: group.into(),
            edns: request.edns.is_some(),
            dnssec_ok: request.edns.as_ref().is_some_and(|e| e.flags().dnssec_ok),
            checking_disabled: request.metadata.checking_disabled,
        })
    }
}

struct Entry {
    reply: Message,
    stored: Instant,
    ttl: u32,
}

pub(crate) struct Cache {
    entries: HashMap<Key, Entry>,
    capacity: usize,
}

impl Cache {
    pub(crate) fn new(capacity: usize) -> Cache {
        Cache {
            entries: HashMap::new(),
            capacity,
        }
    }

    /// The cached reply with every TTL reduced by its age, or `None` once expired.
    pub(crate) fn get(&mut self, key: &Key, now: Instant) -> Option<Message> {
        let entry = self.entries.get(key)?;
        let age = age(entry.stored, now);
        if age >= entry.ttl {
            self.entries.remove(key);
            return None;
        }
        let mut reply = entry.reply.clone();
        let records = reply
            .answers
            .iter_mut()
            .chain(reply.authorities.iter_mut())
            .chain(reply.additionals.iter_mut());
        for record in records {
            record.ttl = record.ttl.saturating_sub(age);
        }
        Some(reply)
    }

    // ponytail: no eviction of live entries; a full cache stops caching new
    // answers until some expire. Add LRU if hit rates ever matter.
    pub(crate) fn put(&mut self, key: Key, reply: &Message, now: Instant) {
        let Some(ttl) = cache_ttl(reply) else {
            return;
        };
        if self.entries.len() >= self.capacity && !self.entries.contains_key(&key) {
            self.entries.retain(|_, e| age(e.stored, now) < e.ttl);
            if self.entries.len() >= self.capacity {
                return;
            }
        }
        let entry = Entry {
            reply: reply.clone(),
            stored: now,
            ttl,
        };
        self.entries.insert(key, entry);
    }
}

fn age(stored: Instant, now: Instant) -> u32 {
    u32::try_from(now.saturating_duration_since(stored).as_secs()).unwrap_or(u32::MAX)
}

/// RFC 2308: positive answers live for their smallest TTL; NXDOMAIN and NODATA
/// for the smaller of the SOA's TTL and minimum. SERVFAIL, truncated replies and
/// anything else are not cached.
fn cache_ttl(reply: &Message) -> Option<u32> {
    if reply.metadata.truncation {
        return None;
    }
    let ttl = match reply.metadata.response_code {
        ResponseCode::NoError if !reply.answers.is_empty() => {
            reply.answers.iter().map(|r| r.ttl).min()?
        }
        ResponseCode::NoError | ResponseCode::NXDomain => {
            reply.authorities.iter().find_map(|r| match &r.data {
                RData::SOA(soa) => Some(r.ttl.min(soa.minimum)),
                _ => None,
            })?
        }
        _ => return None,
    };
    Some(ttl.min(MAX_TTL)).filter(|ttl| *ttl > 0)
}
````

In `crates/veyra-dns/src/engine.rs`, apply this patch (save it to a file and run `git apply <file>`, or edit by hand):

````diff
diff --git a/crates/veyra-dns/src/engine.rs b/crates/veyra-dns/src/engine.rs
index 1a1180e..55a6624 100644
--- a/crates/veyra-dns/src/engine.rs
+++ b/crates/veyra-dns/src/engine.rs
@@ -1,11 +1,13 @@
 //! One DNS query in, at most one reply out (spec §9).
 
+use crate::cache::{Cache, Key};
 use crate::synth::{self, MAX_UDP_PAYLOAD};
 use crate::upstream;
 use hickory_proto::op::{Message, MessageType, OpCode, Query, ResponseCode};
 use hickory_proto::rr::{DNSClass, RecordType};
 use std::net::IpAddr;
-use std::time::Duration;
+use std::sync::{Mutex, MutexGuard, PoisonError};
+use std::time::{Duration, Instant};
 use veyra_policy::{DnsConfig, DnsPlan, Mode, Name, Policy, QueryKind, Subject};
 
 /// How the query arrived: UDP replies must fit the client's buffer.
@@ -15,10 +17,14 @@ pub enum Transport {
     Tcp,
 }
 
+/// Upstream answers kept at once (spec §9.3).
+const CACHE_ENTRIES: usize = 10_000;
+
 pub struct Engine {
     policy: Policy,
     config: DnsConfig,
     upstream_timeout: Duration,
+    cache: Mutex<Cache>,
 }
 
 impl Engine {
@@ -32,6 +38,7 @@ impl Engine {
             policy,
             config,
             upstream_timeout: Duration::from_secs(2),
+            cache: Mutex::new(Cache::new(CACHE_ENTRIES)),
         })
     }
 
@@ -84,16 +91,42 @@ impl Engine {
     }
 
     async fn forward(&self, request: &Message, group: &str) -> Message {
+        let key = Key::new(request, group);
+        let hit = key
+            .as_ref()
+            .and_then(|key| self.cache().get(key, Instant::now()));
+        if let Some(hit) = hit {
+            return for_client(hit, request);
+        }
         let servers = self
             .config
             .upstreams
             .get(group)
             .map_or(&[][..], Vec::as_slice);
         match upstream::exchange(servers, request, self.upstream_timeout).await {
-            Some(reply) => reply,
+            Some(reply) => {
+                if let Some(key) = key {
+                    self.cache().put(key, &reply, Instant::now());
+                }
+                reply
+            }
             None => synth::reply(request, ResponseCode::ServFail),
         }
     }
+
+    /// Never held across an `.await`; a poisoned lock still holds a usable cache.
+    fn cache(&self) -> MutexGuard<'_, Cache> {
+        self.cache.lock().unwrap_or_else(PoisonError::into_inner)
+    }
+}
+
+/// A cached reply readdressed to this request: its id, RD bit, and question
+/// spelled the way this client spelled it.
+fn for_client(mut reply: Message, request: &Message) -> Message {
+    reply.metadata.id = request.metadata.id;
+    reply.metadata.recursion_desired = request.metadata.recursion_desired;
+    reply.queries = request.queries.clone();
+    reply
 }
 
 fn kind(query: &Query) -> QueryKind {
````

In `crates/veyra-dns/src/upstream.rs`, apply this patch (save it to a file and run `git apply <file>`, or edit by hand):

````diff
diff --git a/crates/veyra-dns/src/upstream.rs b/crates/veyra-dns/src/upstream.rs
index cb39b94..7b9f2d4 100644
--- a/crates/veyra-dns/src/upstream.rs
+++ b/crates/veyra-dns/src/upstream.rs
@@ -4,7 +4,8 @@ use hickory_proto::op::{Message, MessageType};
 use std::io;
 use std::net::{Ipv4Addr, Ipv6Addr, SocketAddr};
 use std::time::Duration;
-use tokio::net::UdpSocket;
+use tokio::io::{AsyncRead, AsyncReadExt, AsyncWrite, AsyncWriteExt};
+use tokio::net::{TcpStream, UdpSocket};
 use tokio::time::timeout;
 
 /// Ask each server in turn; the first valid reply wins. `None` when every
@@ -18,7 +19,12 @@ pub(crate) async fn exchange(
     query.metadata.recursion_desired = true;
     let bytes = query.to_vec().ok()?;
     for &server in servers {
-        match udp(server, &bytes, &query, wait).await {
+        let reply = match udp(server, &bytes, &query, wait).await {
+            // A truncated reply means the full one only fits over TCP.
+            Ok(reply) if reply.metadata.truncation => tcp(server, &bytes, &query, wait).await,
+            other => other,
+        };
+        match reply {
             Ok(reply) => return Some(reply),
             Err(e) => tracing::debug!(%server, error = %e, "upstream failed"),
         }
@@ -26,6 +32,56 @@ pub(crate) async fn exchange(
     None
 }
 
+/// One query over TCP with the 2-byte length prefix of RFC 1035 §4.2.2.
+async fn tcp(
+    server: SocketAddr,
+    bytes: &[u8],
+    query: &Message,
+    wait: Duration,
+) -> io::Result<Message> {
+    timeout(wait, async {
+        let mut stream = TcpStream::connect(server).await?;
+        write_frame(&mut stream, bytes).await?;
+        let reply = read_frame(&mut stream).await?;
+        let reply =
+            Message::from_vec(&reply).map_err(|e| io::Error::new(io::ErrorKind::InvalidData, e))?;
+        if answers(&reply, query) {
+            Ok(reply)
+        } else {
+            Err(io::Error::new(
+                io::ErrorKind::InvalidData,
+                "reply does not match the query",
+            ))
+        }
+    })
+    .await
+    .map_err(|_| io::Error::new(io::ErrorKind::TimedOut, "no reply"))?
+}
+
+/// Read one length-prefixed DNS message from a TCP stream.
+pub(crate) async fn read_frame(stream: &mut (impl AsyncRead + Unpin)) -> io::Result<Vec<u8>> {
+    let mut len = [0u8; 2];
+    stream.read_exact(&mut len).await?;
+    let mut message = vec![0; usize::from(u16::from_be_bytes(len))];
+    stream.read_exact(&mut message).await?;
+    Ok(message)
+}
+
+/// Write one length-prefixed DNS message to a TCP stream.
+pub(crate) async fn write_frame(
+    stream: &mut (impl AsyncWrite + Unpin),
+    message: &[u8],
+) -> io::Result<()> {
+    let len = u16::try_from(message.len()).map_err(|_| {
+        io::Error::new(
+            io::ErrorKind::InvalidInput,
+            "message longer than 65535 bytes",
+        )
+    })?;
+    stream.write_all(&len.to_be_bytes()).await?;
+    stream.write_all(message).await
+}
+
 /// One query over a fresh socket: the OS picks a random source port, which
 /// with the client's random id makes off-path spoofing hard (spec §19.1).
 // ponytail: a socket per query; pool sockets if upstream QPS ever matters.
````

- [ ] **Step 4: Run the tests to see them pass**

Run: `cargo test -p veyra-dns`
Expected: unit `ok. 6 passed`, `cache` `ok. 5 passed`, `engine` `ok. 11 passed`.

- [ ] **Step 5: Format and lint**

Run: `cargo fmt --all && cargo clippy --workspace --all-targets -- -D warnings`
Expected: clippy prints no warnings.

- [ ] **Step 6: Commit**

````bash
git add crates/veyra-dns/src/cache.rs crates/veyra-dns/src/engine.rs crates/veyra-dns/src/lib.rs crates/veyra-dns/src/upstream.rs crates/veyra-dns/tests/cache.rs crates/veyra-dns/tests/support/mod.rs
git commit -m "feat(dns): cache upstream answers and retry truncated replies over TCP"
````

---

### Task 8: Listeners and the `veyra-dataplane` binary

**Files:**
- Create: `apps/veyra-dataplane/Cargo.toml`, `apps/veyra-dataplane/src/main.rs`, `apps/veyra-dataplane/tests/dataplane.rs`, `crates/veyra-dns/src/server.rs`, `crates/veyra-dns/tests/server.rs`
- Modify: `Cargo.toml`, `README.md`, `configs/examples/home.yaml`, `crates/veyra-dns/src/lib.rs`

**Interfaces:**
- Consumes: `Engine`, `Transport` (Task 6), `upstream::{read_frame, write_frame}` (Task 7), `Policy::admit` (Task 4), `Policy::dns` (Task 3).
- Produces: `Listeners::bind(&[SocketAddr]) -> io::Result<Listeners>` (async; UDP and TCP on each address, TCP on the port UDP got, so `:0` works); `Listeners::local_addrs(&self) -> io::Result<Vec<SocketAddr>>`; `async Listeners::serve(self, Arc<Engine>)` (runs until dropped; 1024 queries in flight, beyond that UDP queries are dropped; TCP closes non-admitted peers before reading and idle connections after 10 s; UDP receive errors are skipped because Windows reports earlier ICMP errors there); binary `veyra-dataplane --config <file>` printing `listening on <addr> (udp, tcp)` per listener on stdout, logs on stderr (`RUST_LOG`, default `info`), exit 1 with a message for an unreadable/invalid config or one without `dns:`.
- The example listens on `127.0.0.1:10053`: 5353 is mDNS and Windows holds it exclusively.

- [ ] **Step 1: Write the failing tests**

In `Cargo.toml`, apply this patch (save it to a file and run `git apply <file>`, or edit by hand):

````diff
diff --git a/Cargo.toml b/Cargo.toml
index f57b182..a7c5c82 100644
--- a/Cargo.toml
+++ b/Cargo.toml
@@ -1,6 +1,6 @@
 [workspace]
 resolver = "3"
-members = ["crates/veyra-policy", "crates/veyra-dns", "apps/veyra-cli"]
+members = ["crates/veyra-policy", "crates/veyra-dns", "apps/veyra-cli", "apps/veyra-dataplane"]
 
 [workspace.package]
 version = "0.1.0"
````

Create `apps/veyra-dataplane/Cargo.toml`:

````toml
[package]
name = "veyra-dataplane"
version.workspace = true
edition.workspace = true
rust-version.workspace = true
publish.workspace = true

[dependencies]
veyra-policy.workspace = true
veyra-dns.workspace = true
clap.workspace = true
tokio.workspace = true
tracing.workspace = true
tracing-subscriber.workspace = true

[dev-dependencies]
hickory-proto.workspace = true

[lints]
workspace = true
````

Create `apps/veyra-dataplane/src/main.rs`:

````rust
fn main() {}
````

Create `apps/veyra-dataplane/tests/dataplane.rs`:

````rust
//! End-to-end: the `veyra-dataplane` binary serving DNS from a policy file.

use hickory_proto::op::{Message, MessageType, OpCode, Query, ResponseCode};
use hickory_proto::rr::{Name, RData, RecordType};
use std::io::{BufRead, BufReader, Read, Write};
use std::net::{Ipv4Addr, SocketAddr, TcpStream, UdpSocket};
use std::path::PathBuf;
use std::process::{Child, Command, Stdio};
use std::time::Duration;

const CONFIG: &str = r#"
apiVersion: veyra.io/v1alpha1
kind: PolicyBundle
metadata: { name: e2e }
spec:
  nodes: [{ id: de-01, gateway: { ipv4: 203.0.113.10 } }]
  profiles: [{ id: home }]
  admission: [{ cidr: 127.0.0.1/32, profile: home }]
  rules:
    - { id: ads, scope: default, match: { domains: ["suffix:ads.example"] }, action: { type: block } }
    - { id: media, scope: default, match: { domains: ["suffix:media.example"] }, action: { type: proxy_node, node: de-01 } }
  dns:
    listen: ["127.0.0.1:0"]
    upstreams: [{ id: default, servers: ["127.0.0.1:9"] }]
"#;

/// Kills the server when the test ends, pass or fail.
struct Server(Child);

impl Drop for Server {
    fn drop(&mut self) {
        let _ = self.0.kill();
        let _ = self.0.wait();
    }
}

fn write_config(name: &str, text: &str) -> PathBuf {
    let path = std::env::temp_dir().join(format!("veyra-e2e-{}-{name}.yaml", std::process::id()));
    std::fs::write(&path, text).expect("write config");
    path
}

/// Start the binary and wait for its "listening on <addr>" line.
fn start(config: &str) -> (Server, SocketAddr) {
    let path = write_config("serve", config);
    let mut child = Command::new(env!("CARGO_BIN_EXE_veyra-dataplane"))
        .args(["--config".as_ref(), path.as_os_str()])
        .stdout(Stdio::piped())
        .stderr(Stdio::null())
        .spawn()
        .expect("start veyra-dataplane");
    let stdout = child.stdout.take().expect("stdout");
    let server = Server(child);
    let mut line = String::new();
    BufReader::new(stdout)
        .read_line(&mut line)
        .expect("read startup line");
    let addr = line
        .split_whitespace()
        .nth(2)
        .and_then(|a| a.parse().ok())
        .unwrap_or_else(|| panic!("unexpected startup line {line:?}"));
    (server, addr)
}

fn query(name: &str, record_type: RecordType) -> Vec<u8> {
    let mut message = Message::new(7, MessageType::Query, OpCode::Query);
    message.metadata.recursion_desired = true;
    message.add_query(Query::query(
        Name::from_ascii(name).expect("name"),
        record_type,
    ));
    message.to_vec().expect("encode")
}

#[test]
fn serves_synthesized_answers_over_udp_and_tcp() {
    let (_server, addr) = start(CONFIG);

    let udp = UdpSocket::bind("127.0.0.1:0").expect("bind client");
    udp.set_read_timeout(Some(Duration::from_secs(5)))
        .expect("timeout");
    udp.send_to(&query("ads.example.", RecordType::A), addr)
        .expect("send");
    let mut buf = [0; 4096];
    let len = udp.recv(&mut buf).expect("UDP reply");
    let reply = Message::from_vec(&buf[..len]).expect("valid reply");
    assert!(matches!(&reply.answers[0].data, RData::A(a) if a.0 == Ipv4Addr::UNSPECIFIED));

    let mut tcp = TcpStream::connect(addr).expect("connect");
    tcp.set_read_timeout(Some(Duration::from_secs(5)))
        .expect("timeout");
    let bytes = query("media.example.", RecordType::AAAA);
    let len = u16::try_from(bytes.len()).expect("fits").to_be_bytes();
    tcp.write_all(&len).expect("write length");
    tcp.write_all(&bytes).expect("write query");
    let mut len = [0u8; 2];
    tcp.read_exact(&mut len).expect("read length");
    let mut buf = vec![0; usize::from(u16::from_be_bytes(len))];
    tcp.read_exact(&mut buf).expect("read reply");
    let reply = Message::from_vec(&buf).expect("valid reply");
    assert_eq!(reply.metadata.response_code, ResponseCode::NoError);
    assert!(
        reply.answers.is_empty(),
        "AAAA without a gateway IPv6 is NODATA"
    );
}

#[test]
fn refuses_a_config_without_a_dns_section() {
    let rules_only = CONFIG.split("  dns:").next().expect("config text");
    let path = write_config("nodns", rules_only);
    let out = Command::new(env!("CARGO_BIN_EXE_veyra-dataplane"))
        .args(["--config".as_ref(), path.as_os_str()])
        .output()
        .expect("run veyra-dataplane");
    assert_eq!(out.status.code(), Some(1));
    let err = String::from_utf8_lossy(&out.stderr);
    assert!(err.contains("no dns section"), "{err}");
}
````

Create `crates/veyra-dns/tests/server.rs`:

````rust
//! UDP and TCP listeners (spec §9.1, §9.6).

mod support;

use hickory_proto::op::Message;
use hickory_proto::rr::{RData, RecordType};
use std::net::{Ipv4Addr, SocketAddr};
use std::sync::Arc;
use std::time::Duration;
use support::{dead_upstream, engine, query};
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::{TcpSocket, TcpStream, UdpSocket};
use tokio::time::timeout;
use veyra_dns::Listeners;

/// Serve the support engine on 127.0.0.1 with an OS-chosen port.
async fn serve() -> SocketAddr {
    let listeners = Listeners::bind(&["127.0.0.1:0".parse().expect("address")])
        .await
        .expect("bind listeners");
    let addr = listeners.local_addrs().expect("bound addresses")[0];
    let engine = Arc::new(engine(&[dead_upstream().await]));
    tokio::spawn(listeners.serve(engine));
    addr
}

fn blocked(reply: &Message) -> bool {
    matches!(&reply.answers[..], [record] if matches!(record.data, RData::A(a) if a.0 == Ipv4Addr::UNSPECIFIED))
}

async fn udp_ask(server: SocketAddr, from: &str, query: &Message) -> Option<Message> {
    let socket = UdpSocket::bind(from).await.expect("bind client");
    socket
        .send_to(&query.to_vec().expect("encode"), server)
        .await
        .expect("send");
    let mut buf = vec![0; 4096];
    let len = timeout(Duration::from_millis(500), socket.recv(&mut buf))
        .await
        .ok()?
        .expect("receive");
    Some(Message::from_vec(&buf[..len]).expect("valid reply"))
}

async fn tcp_ask(stream: &mut TcpStream, query: &Message) -> Message {
    let bytes = query.to_vec().expect("encode");
    let len = u16::try_from(bytes.len()).expect("fits").to_be_bytes();
    stream.write_all(&len).await.expect("write length");
    stream.write_all(&bytes).await.expect("write query");
    let mut len = [0u8; 2];
    stream.read_exact(&mut len).await.expect("read length");
    let mut buf = vec![0; usize::from(u16::from_be_bytes(len))];
    stream.read_exact(&mut buf).await.expect("read reply");
    Message::from_vec(&buf).expect("valid reply")
}

#[tokio::test]
async fn answers_over_udp() {
    let server = serve().await;
    let reply = udp_ask(server, "127.0.0.1:0", &query("ads.example.", RecordType::A))
        .await
        .expect("a reply");
    assert!(blocked(&reply), "{reply:?}");
}

#[tokio::test]
async fn one_tcp_connection_carries_several_queries() {
    let server = serve().await;
    let mut stream = TcpStream::connect(server).await.expect("connect");
    for name in ["ads.example.", "www.ads.example."] {
        let reply = tcp_ask(&mut stream, &query(name, RecordType::A)).await;
        assert!(blocked(&reply), "{name}: {reply:?}");
    }
}

#[tokio::test]
async fn strangers_get_silence_over_udp() {
    let server = serve().await;
    let reply = udp_ask(server, "127.0.0.3:0", &query("ads.example.", RecordType::A)).await;
    assert!(reply.is_none());
}

#[tokio::test]
async fn strangers_get_a_closed_tcp_connection() {
    let server = serve().await;
    let socket = TcpSocket::new_v4().expect("socket");
    socket
        .bind("127.0.0.3:0".parse().unwrap())
        .expect("bind client");
    let mut stream = socket.connect(server).await.expect("connect");
    let mut byte = [0u8; 1];
    let read = timeout(Duration::from_secs(2), stream.read(&mut byte))
        .await
        .expect("the server closes the connection promptly");
    assert!(matches!(read, Ok(0) | Err(_)), "{read:?}");
}
````

- [ ] **Step 2: Run the tests to see them fail**

Run: `cargo test -p veyra-dns --test server`
Expected: compilation fails: ``unresolved import `veyra_dns::Listeners` ``.

Run: `cargo test -p veyra-dataplane`
Expected: `test result: FAILED. 0 passed; 2 failed` (the stub binary prints nothing and exits 0).

- [ ] **Step 3: Write the implementation**

In `README.md`, apply this patch (save it to a file and run `git apply <file>`, or edit by hand):

````diff
diff --git a/README.md b/README.md
index 6506c5a..af9bacd 100644
--- a/README.md
+++ b/README.md
@@ -2,7 +2,7 @@
 
 Self-hosted SmartDNS and traffic routing platform. Design and requirements: [Veyra_Project_Specification_RU.md](Veyra_Project_Specification_RU.md).
 
-Current state: policy core and CLI (stage 1a, plan 1 of 4 — see `docs/superpowers/plans/`).
+Current state: policy core, CLI and DNS engine (stage 1a, plan 2 of 4 — see `docs/superpowers/plans/`).
 
 ## Build and test
 
@@ -11,10 +11,22 @@ cargo test --workspace
 cargo clippy --workspace --all-targets -- -D warnings
 ```
 
-## Try it
+## Try the policy
 
 ```bash
 cargo run -p veyra-cli -- validate configs/examples/home.yaml
 cargo run -p veyra-cli -- explain --config configs/examples/home.yaml --profile home login.media.example
 cargo run -p veyra-cli -- explain --config configs/examples/home.yaml --device work-laptop git.corp.example
+cargo run -p veyra-cli -- explain --config configs/examples/home.yaml --source 192.168.1.50 media.example
 ```
+
+## Run the DNS engine
+
+```bash
+cargo run -p veyra-dataplane -- --config configs/examples/home.yaml
+dig @127.0.0.1 -p 10053 media.example   # 203.0.113.10: proxied, sent to the de-01 gateway
+dig @127.0.0.1 -p 10053 ads.example     # 0.0.0.0: blocked
+dig @127.0.0.1 -p 10053 example.org     # forwarded to 1.1.1.1
+```
+
+Only sources listed under `admission:` get answers; everyone else is ignored. Windows `nslookup` ignores `-port`: to use it, set `listen: ["127.0.0.1:53"]` in a copy of the config and run `nslookup media.example 127.0.0.1`.
````

Replace `apps/veyra-dataplane/src/main.rs` with:

````rust
//! `veyra-dataplane`: serves DNS for a policy file (spec §5.2, §9). The gateway
//! joins this process in plan 3.

use clap::Parser;
use std::path::PathBuf;
use std::process::ExitCode;
use std::sync::Arc;
use tracing_subscriber::EnvFilter;
use veyra_dns::{Engine, Listeners};
use veyra_policy::{Bundle, Policy};

#[derive(Parser)]
#[command(name = "veyra-dataplane", version, about = "Veyra data plane: DNS")]
struct Args {
    /// The policy file; it must have `dns:` and `admission:` sections.
    #[arg(long)]
    config: PathBuf,
}

#[tokio::main]
async fn main() -> ExitCode {
    // Logs go to stderr (RUST_LOG, default `info`); stdout carries only the
    // "listening on" lines that tests and service managers read.
    let filter = EnvFilter::try_from_default_env().unwrap_or_else(|_| EnvFilter::new("info"));
    tracing_subscriber::fmt()
        .with_env_filter(filter)
        .with_writer(std::io::stderr)
        .init();
    match run(Args::parse()).await {
        Ok(()) => ExitCode::SUCCESS,
        Err(message) => {
            eprintln!("{message}");
            ExitCode::FAILURE
        }
    }
}

async fn run(args: Args) -> Result<(), String> {
    let path = args.config.display();
    let text = std::fs::read_to_string(&args.config).map_err(|e| format!("{path}: {e}"))?;
    let bundle = Bundle::from_yaml(&text).map_err(|e| format!("{path}: {e}"))?;
    let policy = match Policy::compile(&bundle) {
        Ok((policy, warnings)) => {
            warnings.iter().for_each(|w| tracing::warn!("{w}"));
            policy
        }
        Err(issues) => {
            issues.iter().for_each(|i| eprintln!("{i}"));
            return Err(format!("{path}: has errors, run `veyra validate`"));
        }
    };
    let listen = policy
        .dns()
        .map(|dns| dns.listen.clone())
        .unwrap_or_default();
    let engine = Arc::new(Engine::new(policy).map_err(|e| format!("{path}: {e}"))?);
    let listeners = Listeners::bind(&listen)
        .await
        .map_err(|e| format!("cannot listen on {listen:?}: {e}"))?;
    for addr in listeners.local_addrs().map_err(|e| e.to_string())? {
        println!("listening on {addr} (udp, tcp)");
    }
    tokio::select! {
        () = listeners.serve(engine) => Ok(()),
        _ = tokio::signal::ctrl_c() => {
            tracing::info!("shutting down");
            Ok(())
        }
    }
}
````

In `configs/examples/home.yaml`, apply this patch (save it to a file and run `git apply <file>`, or edit by hand):

````diff
diff --git a/configs/examples/home.yaml b/configs/examples/home.yaml
index 7c2a2ba..f825e6b 100644
--- a/configs/examples/home.yaml
+++ b/configs/examples/home.yaml
@@ -55,7 +55,7 @@ spec:
     - { cidr: 192.168.1.0/24, profile: home }
     - { cidr: 192.168.1.50/32, device: tv }
   dns:
-    listen: ["127.0.0.1:5353"]          # production: ["0.0.0.0:53", "[::]:53"]
+    listen: ["127.0.0.1:10053"]         # 5353 is mDNS; production: ["0.0.0.0:53", "[::]:53"]
     synthTtl: 60
     upstreams:
       - id: default
````

In `crates/veyra-dns/src/lib.rs`, apply this patch (save it to a file and run `git apply <file>`, or edit by hand):

````diff
diff --git a/crates/veyra-dns/src/lib.rs b/crates/veyra-dns/src/lib.rs
index d8ece32..cb91540 100644
--- a/crates/veyra-dns/src/lib.rs
+++ b/crates/veyra-dns/src/lib.rs
@@ -3,7 +3,9 @@
 
 mod cache;
 mod engine;
+mod server;
 mod synth;
 mod upstream;
 
 pub use engine::{Engine, Transport};
+pub use server::Listeners;
````

Create `crates/veyra-dns/src/server.rs`:

````rust
//! UDP and TCP listeners for the engine (spec §9.1, §9.6).

use crate::engine::{Engine, Transport};
use crate::upstream::{read_frame, write_frame};
use std::io;
use std::net::SocketAddr;
use std::sync::Arc;
use std::time::Duration;
use tokio::net::{TcpListener, TcpStream, UdpSocket};
use tokio::sync::Semaphore;
use tokio::task::JoinSet;
use tokio::time::{sleep, timeout};

/// Queries in progress at once. Beyond this, UDP queries are dropped and TCP
/// queries wait (spec §19.2: limits).
const MAX_IN_FLIGHT: usize = 1024;
/// An idle TCP connection is closed after this long.
const TCP_IDLE: Duration = Duration::from_secs(10);

/// Bound sockets. Binding first lets the caller learn the real addresses
/// (for `127.0.0.1:0`) before serving.
pub struct Listeners {
    udp: Vec<UdpSocket>,
    tcp: Vec<TcpListener>,
}

impl Listeners {
    /// UDP and TCP on each address; TCP takes the port UDP actually got.
    pub async fn bind(addrs: &[SocketAddr]) -> io::Result<Listeners> {
        let (mut udp, mut tcp) = (Vec::new(), Vec::new());
        for addr in addrs {
            let socket = UdpSocket::bind(addr).await?;
            tcp.push(TcpListener::bind(socket.local_addr()?).await?);
            udp.push(socket);
        }
        Ok(Listeners { udp, tcp })
    }

    /// The bound addresses, one per `listen` entry.
    pub fn local_addrs(&self) -> io::Result<Vec<SocketAddr>> {
        self.udp.iter().map(UdpSocket::local_addr).collect()
    }

    /// Serve until the future is dropped.
    pub async fn serve(self, engine: Arc<Engine>) {
        let permits = Arc::new(Semaphore::new(MAX_IN_FLIGHT));
        let mut tasks = JoinSet::new();
        for socket in self.udp {
            tasks.spawn(serve_udp(Arc::new(socket), engine.clone(), permits.clone()));
        }
        for listener in self.tcp {
            tasks.spawn(serve_tcp(listener, engine.clone(), permits.clone()));
        }
        while tasks.join_next().await.is_some() {}
    }
}

async fn serve_udp(socket: Arc<UdpSocket>, engine: Arc<Engine>, permits: Arc<Semaphore>) {
    let mut buf = vec![0; 65_535];
    loop {
        // Windows reports an ICMP "port unreachable" for an earlier reply as an
        // error on the next receive. It says nothing about this socket: go on.
        let Ok((len, peer)) = socket.recv_from(&mut buf).await else {
            continue;
        };
        let Ok(permit) = permits.clone().try_acquire_owned() else {
            continue;
        };
        let query = buf[..len].to_vec();
        let (socket, engine) = (socket.clone(), engine.clone());
        tokio::spawn(async move {
            if let Some(reply) = engine.handle(peer.ip(), &query, Transport::Udp).await {
                let _ = socket.send_to(&reply, peer).await;
            }
            drop(permit);
        });
    }
}

async fn serve_tcp(listener: TcpListener, engine: Arc<Engine>, permits: Arc<Semaphore>) {
    loop {
        let (stream, peer) = match listener.accept().await {
            Ok(accepted) => accepted,
            Err(e) => {
                // Out of file descriptors and similar: back off instead of spinning.
                tracing::warn!(error = %e, "accept failed");
                sleep(Duration::from_millis(100)).await;
                continue;
            }
        };
        // Not admitted: close at once, before reading a byte (spec §9.6).
        if engine.policy().admit(peer.ip()).is_none() {
            continue;
        }
        tokio::spawn(serve_connection(
            stream,
            peer,
            engine.clone(),
            permits.clone(),
        ));
    }
}

async fn serve_connection(
    mut stream: TcpStream,
    peer: SocketAddr,
    engine: Arc<Engine>,
    permits: Arc<Semaphore>,
) {
    while let Ok(Ok(query)) = timeout(TCP_IDLE, read_frame(&mut stream)).await {
        let Ok(_permit) = permits.acquire().await else {
            return;
        };
        let Some(reply) = engine.handle(peer.ip(), &query, Transport::Tcp).await else {
            return;
        };
        if write_frame(&mut stream, &reply).await.is_err() {
            return;
        }
    }
}
````

- [ ] **Step 4: Run the tests to see them pass**

Run: `cargo test --workspace`
Expected: every suite passes — `veyra-cli` 11, `veyra-dataplane` 2, `veyra-dns` unit 6 / `cache` 5 / `engine` 11 / `server` 4, `veyra-policy` unit 50 / `admission` 6 / determinism 1 / `dns_config` 9 / `dns_plan` 5.

Then try the binary (`dig` on Linux/macOS/WSL; Windows `nslookup` ignores `-port`, see the README):

Run: `cargo run -q -p veyra-dataplane -- --config configs/examples/home.yaml`
Expected on stdout: `listening on 127.0.0.1:10053 (udp, tcp)`

Run, in another terminal: `dig @127.0.0.1 -p 10053 +short media.example`, `... ads.example`, `... example.org`
Expected: `203.0.113.10`; `0.0.0.0`; the real addresses of example.org (forwarded to 1.1.1.1).

- [ ] **Step 5: Format and lint**

Run: `cargo fmt --all && cargo clippy --workspace --all-targets -- -D warnings`
Expected: clippy prints no warnings.

- [ ] **Step 6: Commit**

````bash
git add Cargo.lock Cargo.toml README.md apps/veyra-dataplane/Cargo.toml apps/veyra-dataplane/src/main.rs apps/veyra-dataplane/tests/dataplane.rs configs/examples/home.yaml crates/veyra-dns/src/lib.rs crates/veyra-dns/src/server.rs crates/veyra-dns/tests/server.rs
git commit -m "feat(dataplane): serve DNS over UDP and TCP from a policy file"
````

---

## Not in this plan

- Link-IP, DDNS and DoT/DoH-token admission (spec §9.6): plan 4 and stage 1b. Only static CIDR bindings here.
- DoT (853) and DoH listeners (spec §9.1): stage 1b; this plan serves plain DNS over UDP and TCP.
- Local override records and subscribed block/allow lists (spec §9.1, §17.1 `LocalRecord`, `DomainList`): later; rules cover blocking for now.
- Node health and `onFailure` (spec §9.5 last rule): DNS always answers the configured gateway until the gateway and agent work (plan 3+) report health.
- `BLOCK` as NXDOMAIN per profile, synthesized HTTPS records, matching CNAME targets (spec §9.5): later options; the defaults are implemented.
- EDNS Client Subnet (spec §9.7): off by default, so nothing to send; conditional-forwarding export is stage 1b.
- Metrics (hit/miss, latency, upstream health; spec §9.1, §20.1): stage 1b.
- Per-client rate limits (spec §19.2 item 6): only the global 1024-query in-flight cap exists.
- Snapshot file, control socket, ordered apply (spec §5.3, §8.5): plan 4. Until then `veyra-dataplane` reads the YAML file at start.
