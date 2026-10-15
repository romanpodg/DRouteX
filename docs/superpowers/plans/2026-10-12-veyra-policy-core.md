# Veyra Policy Core Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** A Rust workspace whose `veyra validate` and `veyra explain` commands load a YAML policy, report every configuration problem, and show deterministically which rule decides a domain for a profile or device.

**Architecture:** One library crate, `veyra-policy`, holds everything that never touches the network: name normalization, domain patterns, the YAML model, a validating compiler and the decision function. A thin binary crate, `veyra-cli`, wraps it. The later DNS and gateway plans call `Policy::decide` with the same types.

**Tech Stack:** Rust 2024 (stable 1.98 on the dev machine), serde + serde-saphyr (YAML), idna, thiserror, clap 4, serde_json.

**Spec:** `Veyra_Project_Specification_RU.md` (repository root). This plan implements §8.1–8.4 (patterns, precedence, YAML format, explain), the route/DNS split of §6.4, the engineering rules of §16.1 and requirements REQ-POL-001…003 of §24.3.

## Where this plan sits

Stage 1a (spec §23.2) is four plans; each ends with working, tested software:

1. **Policy core + CLI** — this plan.
2. DNS engine: `veyra-dataplane` serving DNS over UDP/TCP 53, upstream forwarding, cache, answer synthesis per §9.5, CIDR admission; `veyra explain` gains `dns_answer`.
3. Gateway: TCP 443 by SNI and 80 by `Host`, RST/reject for other ports, SSRF guard, origin resolver (§10).
4. Link-IP admission, snapshot file, Unix control socket, ordered apply with drain, systemd unit (§8.5, §9.6).

Plans 2–4 are written when the previous plan has landed.

## Global Constraints

- Rust edition 2024, `rust-version = "1.85"`; dependencies declared once in `[workspace.dependencies]`; `Cargo.lock` is committed (spec §15.1, §16.1).
- Dependency floors: serde 1.0.229, serde-saphyr 1.3.0, serde_json 1.0.154, idna 1.1.0, thiserror 2.0.21, clap 4.6.7. Do not add `serde_yaml` (archived upstream).
- No `unwrap()` outside tests: clippy `unwrap_used = "warn"` plus `allow-unwrap-in-tests = true` (spec §16.1). `unsafe_code = "forbid"`.
- Every commit passes `cargo fmt --all --check`, `cargo clippy --workspace --all-targets -- -D warnings` and `cargo test --workspace` (spec §16.1).
- Decisions are deterministic: no `HashMap` iteration on the decision path; ties end at the rule id (spec §8.2).
- Patterns (spec §8.1): `x` matches only `x`; `*.x` matches subdomains of `x`, not `x`; `suffix:x` matches `x` and its subdomains. No regex or keyword patterns.
- Precedence (spec §8.2.1): pattern specificity → scope (`device` > `profile` > `default`) → action (`block` > `proxy_node` > `direct`) → smaller rule id. There is no numeric priority.
- Config format: `apiVersion: veyra.io/v1alpha1`, `kind: PolicyBundle`; unknown fields are errors.
- Names: lowercased, IDN converted to punycode, one trailing dot removed, labels 1–63 characters of `a-z 0-9 - _` not starting or ending with `-`, whole name ≤ 253 characters.
- Single-node MVP route actions: `direct`, `block`, `proxy_node`; `onFailure` defaults to `block`; `onFailure: direct` is allowed but produces a `FailOpen` warning (spec §13.3).
- The dev machine is Windows; nothing in this plan is OS-specific. Run every command from the repository root `E:\LeetCode\VeyraDNS`.

## Review Focus

Inputs the spec implies but does not spell out, most likely to bite first. Each is pinned by a test in the task named.

1. **Queries in any case, with a trailing dot or in Unicode** (`WWW.Пример.РФ.`) must match rules written as `suffix:пример.рф` — `decide::tests::idn_rules_match_idn_queries` (Task 5), `explain_prints_the_deciding_rule_as_json` (Task 6).
2. **Garbage domain input** (spaces, `a..b`, 64-character labels, 254-character names) must give a clean error and exit code 1, never a panic — `name::tests::rejects_malformed_names` (Task 1), `explain_rejects_bad_requests_cleanly` (Task 6).
3. **`--device` with a contradicting `--profile`** must be an error, not a silent pick — `decide::tests::subject_errors` (Task 5), `explain_rejects_bad_requests_cleanly` (Task 6).
4. **YAML typos**, including the removed `priority:` field and bad indentation, must fail with a line number — `model::tests::rejects_unknown_fields_with_a_line_number` (Task 3), `malformed_yaml_names_the_line` (Task 6).
5. **Reordering rules in the file** must not change any decision (REQ-POL-001) — `decisions_do_not_depend_on_rule_order` (Task 5).

## File map

| File | Responsibility |
|---|---|
| `Cargo.toml`, `clippy.toml`, `.gitignore` | Workspace, shared dependency versions, lints |
| `crates/veyra-policy/src/name.rs` | `Name`: normalized domain names |
| `crates/veyra-policy/src/pattern.rs` | `Pattern`: exact / `*.` / `suffix:` matching and specificity |
| `crates/veyra-policy/src/model.rs` | Serde types mirroring the YAML file; `Bundle::from_yaml` |
| `crates/veyra-policy/src/compile.rs` | `Policy::compile`: references, patterns, conflicts, shadowing → `Issue`s |
| `crates/veyra-policy/src/decide.rs` | `Policy::subject`, `Policy::decide` → `Decision` with `decided_by` |
| `crates/veyra-policy/tests/determinism.rs` | REQ-POL-001 over all 120 orderings of five rules |
| `configs/examples/home.yaml` | The spec §8.3 example, complete with `nodes` and `devices` |
| `apps/veyra-cli/src/main.rs` | `veyra validate`, `veyra explain` |
| `apps/veyra-cli/tests/cli.rs`, `tests/fixtures/*.yaml` | End-to-end checks of the binary |

---

### Task 1: Workspace and normalized names

**Files:**
- Create: `Cargo.toml`, `clippy.toml`, `.gitignore`
- Create: `crates/veyra-policy/Cargo.toml`, `crates/veyra-policy/src/lib.rs`, `crates/veyra-policy/src/name.rs`

**Interfaces:**
- Consumes: nothing.
- Produces: `Name::parse(&str) -> Result<Name, NameError>`, `Name::as_str(&self) -> &str`, `Name::label_count(&self) -> usize`, `Name::is_strict_subdomain_of(&self, parent: &Name) -> bool`, `impl Display for Name`; `NameError::{Empty, Idna, EmptyLabel, LabelTooLong, NameTooLong, InvalidChar(char), Hyphen}`. `Name` derives `Clone, Debug, PartialEq, Eq, PartialOrd, Ord, Hash`.

- [ ] **Step 1: Initialize the repository and workspace**

Run: `git init`

Create `Cargo.toml` (only the policy crate is a member until Task 6):

```toml
[workspace]
resolver = "3"
members = ["crates/veyra-policy"]

[workspace.package]
version = "0.1.0"
edition = "2024"
rust-version = "1.85"
publish = false

[workspace.dependencies]
veyra-policy = { path = "crates/veyra-policy" }
serde = { version = "1.0.229", features = ["derive"] }
serde-saphyr = "1.3.0"
serde_json = "1.0.154"
idna = "1.1.0"
thiserror = "2.0.21"
clap = { version = "4.6.7", features = ["derive"] }

[workspace.lints.rust]
unsafe_code = "forbid"

[workspace.lints.clippy]
unwrap_used = "warn"
```

Create `clippy.toml`:

```toml
allow-unwrap-in-tests = true
```

Create `.gitignore`:

```gitignore
/target
/.omc/
```

Create `crates/veyra-policy/Cargo.toml`:

```toml
[package]
name = "veyra-policy"
version.workspace = true
edition.workspace = true
rust-version.workspace = true
publish.workspace = true

[dependencies]
serde.workspace = true
serde-saphyr.workspace = true
idna.workspace = true
thiserror.workspace = true

[lints]
workspace = true
```

- [ ] **Step 2: Write the failing tests**

Create `crates/veyra-policy/src/lib.rs`:

```rust
//! Veyra policy core: domain names, patterns, rule precedence and explain (spec §8).

mod name;

pub use name::{Name, NameError};
```

Create `crates/veyra-policy/src/name.rs` containing only the test module for now:

```rust
#[cfg(test)]
mod tests {
    use super::*;

    fn n(s: &str) -> Name {
        Name::parse(s).unwrap()
    }

    #[test]
    fn lowercases_and_strips_one_trailing_dot() {
        assert_eq!(n("MEDIA.Example.").as_str(), "media.example");
    }

    #[test]
    fn converts_idn_to_punycode() {
        assert_eq!(n("пример.рф").as_str(), "xn--e1afmkfd.xn--p1ai");
        assert_eq!(n("xn--e1afmkfd.xn--p1ai"), n("ПРИМЕР.РФ"));
    }

    #[test]
    fn allows_underscore_labels() {
        assert_eq!(n("_dmarc.example.com").as_str(), "_dmarc.example.com");
    }

    #[test]
    fn rejects_malformed_names() {
        assert_eq!(Name::parse(""), Err(NameError::Empty));
        assert_eq!(Name::parse("."), Err(NameError::Empty));
        assert_eq!(Name::parse("a..b"), Err(NameError::EmptyLabel));
        assert_eq!(Name::parse("example.com.."), Err(NameError::EmptyLabel));
        assert_eq!(Name::parse("a b.com"), Err(NameError::InvalidChar(' ')));
        assert_eq!(Name::parse("-bad.com"), Err(NameError::Hyphen));
        let long_label = format!("{}.com", "a".repeat(64));
        assert_eq!(Name::parse(&long_label), Err(NameError::LabelTooLong));
        let long_name = vec!["a".repeat(63); 4].join(".");
        assert_eq!(Name::parse(&long_name), Err(NameError::NameTooLong));
    }

    #[test]
    fn strict_subdomain() {
        assert!(n("a.example.com").is_strict_subdomain_of(&n("example.com")));
        assert!(n("a.b.example.com").is_strict_subdomain_of(&n("example.com")));
        assert!(!n("example.com").is_strict_subdomain_of(&n("example.com")));
        assert!(!n("badexample.com").is_strict_subdomain_of(&n("example.com")));
    }

    #[test]
    fn counts_labels() {
        assert_eq!(n("login.media.example").label_count(), 3);
    }
}
```

- [ ] **Step 3: Run the tests to see them fail**

Run: `cargo test -p veyra-policy`
Expected: compilation fails with ``cannot find type `Name` in this scope``.

- [ ] **Step 4: Write the implementation**

Insert above the `#[cfg(test)]` line of `crates/veyra-policy/src/name.rs`:

```rust
//! Normalized domain names (spec §8.1): lowercase, IDNA/punycode, no trailing dot.

use std::fmt;

/// A normalized domain name: ASCII lowercase, punycode for IDN labels, no trailing dot.
#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct Name(String);

#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
pub enum NameError {
    #[error("empty name")]
    Empty,
    #[error("not a valid internationalized domain name")]
    Idna,
    #[error("empty label")]
    EmptyLabel,
    #[error("label longer than 63 characters")]
    LabelTooLong,
    #[error("name longer than 253 characters")]
    NameTooLong,
    #[error("invalid character {0:?}")]
    InvalidChar(char),
    #[error("label starts or ends with '-'")]
    Hyphen,
}

impl Name {
    pub fn parse(input: &str) -> Result<Name, NameError> {
        let trimmed = input.strip_suffix('.').unwrap_or(input);
        if trimmed.is_empty() {
            return Err(NameError::Empty);
        }
        // idna lowercases and punycodes but accepts empty labels, spaces and
        // over-long labels, so the DNS rules are checked below.
        let ascii = idna::domain_to_ascii(trimmed).map_err(|_| NameError::Idna)?;
        if ascii.len() > 253 {
            return Err(NameError::NameTooLong);
        }
        for label in ascii.split('.') {
            if label.is_empty() {
                return Err(NameError::EmptyLabel);
            }
            if label.len() > 63 {
                return Err(NameError::LabelTooLong);
            }
            let bad = label.chars().find(|c| {
                !(c.is_ascii_lowercase() || c.is_ascii_digit() || *c == '-' || *c == '_')
            });
            if let Some(c) = bad {
                return Err(NameError::InvalidChar(c));
            }
            if label.starts_with('-') || label.ends_with('-') {
                return Err(NameError::Hyphen);
            }
        }
        Ok(Name(ascii))
    }

    pub fn as_str(&self) -> &str {
        &self.0
    }

    pub fn label_count(&self) -> usize {
        self.0.split('.').count()
    }

    /// `a.example.com` is under `example.com`; a name is not under itself.
    pub fn is_strict_subdomain_of(&self, parent: &Name) -> bool {
        let (s, p) = (self.0.as_bytes(), parent.0.as_bytes());
        s.len() > p.len() && s.ends_with(p) && s[s.len() - p.len() - 1] == b'.'
    }
}

impl fmt::Display for Name {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&self.0)
    }
}
```

- [ ] **Step 5: Run the tests to see them pass**

Run: `cargo test -p veyra-policy`
Expected: `test result: ok. 6 passed; 0 failed`.

- [ ] **Step 6: Format and lint**

Run: `cargo fmt --all && cargo clippy --workspace --all-targets -- -D warnings`
Expected: clippy ends with `Finished` and prints no warnings.

- [ ] **Step 7: Commit**

```bash
git add Cargo.toml Cargo.lock clippy.toml .gitignore crates Veyra_Project_Specification_RU.md docs
git commit -m "feat(policy): workspace and normalized domain names"
```

---

### Task 2: Domain patterns

**Files:**
- Create: `crates/veyra-policy/src/pattern.rs`
- Modify: `crates/veyra-policy/src/lib.rs`

**Interfaces:**
- Consumes: `Name`, `NameError` (Task 1).
- Produces: `enum Pattern { Exact(Name), Wildcard(Name), Suffix(Name) }` with `Pattern::parse(&str) -> Result<Pattern, PatternError>`, `Pattern::matches(&self, &Name) -> bool`, `Pattern::specificity(&self) -> Specificity`, `impl Display` (prints `x`, `*.x`, `suffix:x`); `struct Specificity { pub exact: bool, pub labels: usize }` ordered so greater is more specific; `PatternError::{BadWildcard, Name(NameError)}`.

- [ ] **Step 1: Write the failing tests**

Replace `crates/veyra-policy/src/lib.rs` with:

```rust
//! Veyra policy core: domain names, patterns, rule precedence and explain (spec §8).

mod name;
mod pattern;

pub use name::{Name, NameError};
pub use pattern::{Pattern, PatternError, Specificity};
```

Create `crates/veyra-policy/src/pattern.rs` containing only the test module:

```rust
#[cfg(test)]
mod tests {
    use super::*;

    fn p(s: &str) -> Pattern {
        Pattern::parse(s).unwrap()
    }

    fn n(s: &str) -> Name {
        Name::parse(s).unwrap()
    }

    #[test]
    fn parses_three_forms_and_displays_them_back() {
        for s in ["media.example", "*.media.example", "suffix:media.example"] {
            assert_eq!(p(s).to_string(), s);
        }
    }

    #[test]
    fn normalizes_the_name_part() {
        assert_eq!(
            p("suffix:Media.Example.").to_string(),
            "suffix:media.example"
        );
        assert_eq!(p("*.пример.рф").to_string(), "*.xn--e1afmkfd.xn--p1ai");
    }

    #[test]
    fn rejects_misplaced_wildcards() {
        for s in [
            "*",
            "a*.example",
            "*.*.example",
            "suffix:*.example",
            "media.*",
        ] {
            assert_eq!(Pattern::parse(s), Err(PatternError::BadWildcard), "{s}");
        }
    }

    #[test]
    fn reports_bad_names() {
        assert_eq!(
            Pattern::parse("suffix:"),
            Err(PatternError::Name(NameError::Empty))
        );
    }

    #[test]
    fn exact_matches_only_itself() {
        assert!(p("media.example").matches(&n("media.example")));
        assert!(!p("media.example").matches(&n("a.media.example")));
    }

    #[test]
    fn wildcard_excludes_apex() {
        let w = p("*.media.example");
        assert!(w.matches(&n("a.media.example")));
        assert!(w.matches(&n("a.b.media.example")));
        assert!(!w.matches(&n("media.example")));
        assert!(!w.matches(&n("xmedia.example")));
    }

    #[test]
    fn suffix_includes_apex() {
        let s = p("suffix:media.example");
        assert!(s.matches(&n("media.example")));
        assert!(s.matches(&n("a.b.media.example")));
        assert!(!s.matches(&n("xmedia.example")));
    }

    #[test]
    fn specificity_order() {
        let s = |x: &str| p(x).specificity();
        assert!(s("login.media.example") > s("suffix:login.media.example"));
        assert!(s("suffix:login.media.example") > s("suffix:media.example"));
        assert_eq!(s("*.media.example"), s("suffix:media.example"));
    }
}
```

- [ ] **Step 2: Run the tests to see them fail**

Run: `cargo test -p veyra-policy`
Expected: compilation fails; `Pattern`, `PatternError` and `Specificity` are not found.

- [ ] **Step 3: Write the implementation**

Insert above the `#[cfg(test)]` line of `crates/veyra-policy/src/pattern.rs`:

```rust
//! Domain patterns (spec §8.1) and their specificity (spec §8.2.1, step 1).

use crate::name::{Name, NameError};
use std::fmt;

#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum Pattern {
    /// `example.com`: only that name.
    Exact(Name),
    /// `*.example.com`: any subdomain, not the apex.
    Wildcard(Name),
    /// `suffix:example.com`: the apex and any subdomain.
    Suffix(Name),
}

/// Greater is more specific: an exact name beats any wildcard or suffix,
/// then a pattern with more labels beats one with fewer.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub struct Specificity {
    pub exact: bool,
    pub labels: usize,
}

#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
pub enum PatternError {
    #[error("'*' is only allowed as a leading '*.' label")]
    BadWildcard,
    #[error(transparent)]
    Name(#[from] NameError),
}

impl Pattern {
    pub fn parse(s: &str) -> Result<Pattern, PatternError> {
        let (make, rest): (fn(Name) -> Pattern, &str) =
            if let Some(rest) = s.strip_prefix("suffix:") {
                (Pattern::Suffix, rest)
            } else if let Some(rest) = s.strip_prefix("*.") {
                (Pattern::Wildcard, rest)
            } else {
                (Pattern::Exact, s)
            };
        if rest.contains('*') {
            return Err(PatternError::BadWildcard);
        }
        Ok(make(Name::parse(rest)?))
    }

    pub fn matches(&self, name: &Name) -> bool {
        match self {
            Pattern::Exact(n) => name == n,
            Pattern::Wildcard(base) => name.is_strict_subdomain_of(base),
            Pattern::Suffix(base) => name == base || name.is_strict_subdomain_of(base),
        }
    }

    pub fn specificity(&self) -> Specificity {
        match self {
            Pattern::Exact(n) => Specificity {
                exact: true,
                labels: n.label_count(),
            },
            Pattern::Wildcard(base) | Pattern::Suffix(base) => Specificity {
                exact: false,
                labels: base.label_count(),
            },
        }
    }
}

impl fmt::Display for Pattern {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Pattern::Exact(n) => write!(f, "{n}"),
            Pattern::Wildcard(n) => write!(f, "*.{n}"),
            Pattern::Suffix(n) => write!(f, "suffix:{n}"),
        }
    }
}
```

- [ ] **Step 4: Run the tests to see them pass**

Run: `cargo test -p veyra-policy`
Expected: `test result: ok. 14 passed; 0 failed`.

- [ ] **Step 5: Format and lint**

Run: `cargo fmt --all && cargo clippy --workspace --all-targets -- -D warnings`
Expected: no warnings.

- [ ] **Step 6: Commit**

```bash
git add crates/veyra-policy
git commit -m "feat(policy): exact, wildcard and suffix domain patterns"
```

---

### Task 3: YAML model and the example config

**Files:**
- Create: `crates/veyra-policy/src/model.rs`, `configs/examples/home.yaml`
- Modify: `crates/veyra-policy/src/lib.rs`

**Interfaces:**
- Consumes: nothing from earlier tasks (plain serde types).
- Produces: `API_VERSION`, `KIND`; `Bundle { api_version, kind, metadata: Metadata { name }, spec: BundleSpec { nodes, profiles, devices, rules } }`; `NodeSpec { id }`; `ProfileSpec { id, default_action: DefaultAction }`; `enum DefaultAction { Direct, Block }`; `DeviceSpec { id, profile }`; `RuleSpec { id, enabled, scope: Scope, matcher: MatchSpec { domains: Vec<String> }, dns: Option<DnsSpec { upstream_group }>, action: Action, modes_allowed: Vec<Mode> }`; `enum Scope { Default, Profile(String), Device(String) }`; `enum Action { Direct, Block, ProxyNode { node: String, on_failure: OnFailure } }` (serialized with `"type"` tag, field `onFailure`); `enum OnFailure { Block, Direct }`; `enum Mode { SmartdnsTcp, ExplicitProxy, Tunnel }`; `Bundle::from_yaml(&str) -> Result<Bundle, String>` (the error text contains the line number).

- [ ] **Step 1: Create the example config**

Create `configs/examples/home.yaml`:

```yaml
# Example Veyra policy (spec §8.3). Validate: veyra validate configs/examples/home.yaml
apiVersion: veyra.io/v1alpha1
kind: PolicyBundle
metadata:
  name: home-default
spec:
  nodes:
    - id: de-01
  profiles:
    - id: home
      defaultAction: direct
  devices:
    - id: tv
      profile: home
    - id: work-laptop
      profile: home
  rules:
    - id: video-through-germany
      enabled: true
      scope: { profile: home }
      match:
        domains: ["suffix:media.example"]
      action:
        type: proxy_node
        node: de-01
        onFailure: block
      modesAllowed: ["smartdns_tcp", "explicit_proxy", "tunnel"]
    - id: media-login-direct            # exact beats suffix (§8.2.1)
      enabled: true
      scope: { profile: home }
      match:
        domains: ["login.media.example"]
      action:
        type: direct
    - id: work-resolver
      enabled: true
      scope: { device: work-laptop }
      match:
        domains: ["suffix:corp.example"]
      dns:
        upstreamGroup: corp             # resolver choice, not egress (§6.4)
      action:
        type: direct
    - id: block-ads
      enabled: true
      scope: default
      match:
        domains: ["suffix:ads.example"]
      action:
        type: block
```

- [ ] **Step 2: Write the failing tests**

Replace `crates/veyra-policy/src/lib.rs` with:

```rust
//! Veyra policy core: domain names, patterns, rule precedence and explain (spec §8).

mod model;
mod name;
mod pattern;

pub use model::*;
pub use name::{Name, NameError};
pub use pattern::{Pattern, PatternError, Specificity};
```

Create `crates/veyra-policy/src/model.rs` containing only the test module:

```rust
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
```

- [ ] **Step 3: Run the tests to see them fail**

Run: `cargo test -p veyra-policy`
Expected: compilation fails; `Bundle`, `Scope`, `Action` and the other model types are not found.

- [ ] **Step 4: Write the implementation**

Insert above the `#[cfg(test)]` line of `crates/veyra-policy/src/model.rs`:

```rust
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
```

- [ ] **Step 5: Run the tests to see them pass**

Run: `cargo test -p veyra-policy`
Expected: `test result: ok. 18 passed; 0 failed`.

- [ ] **Step 6: Format and lint**

Run: `cargo fmt --all && cargo clippy --workspace --all-targets -- -D warnings`
Expected: no warnings.

- [ ] **Step 7: Commit**

```bash
git add crates/veyra-policy configs
git commit -m "feat(policy): YAML policy model and example config"
```

---

### Task 4: Compiler and validator

**Files:**
- Create: `crates/veyra-policy/src/compile.rs`
- Modify: `crates/veyra-policy/src/lib.rs`

**Interfaces:**
- Consumes: `Pattern` (Task 2); `Bundle`, `Scope`, `Action`, `OnFailure`, `DnsSpec`, `Mode`, `DefaultAction`, `API_VERSION`, `KIND` (Task 3).
- Produces: `Policy::compile(&Bundle) -> Result<(Policy, Vec<Issue>), Vec<Issue>>` (`Ok` carries warnings; `Err` carries all issues when any is an error); `Policy { pub name: String, pub(crate) rules: Vec<Rule> }` (enabled rules only, in file order) with `Policy::rules(&self) -> &[Rule]` (Task 5 adds the `profiles` and `devices` maps that `decide` needs); `Rule { id, scope, patterns: Vec<Pattern>, action, dns: Option<DnsSpec>, modes: Vec<Mode> }` with `pub(crate) fn late_rank(&self) -> (u8, Reverse<&str>)`; `Issue { severity: Severity, code: IssueCode, rules: Vec<String>, message: String }` with `Display` like `error[PatternConflict] a, b: …`; `Severity::{Error, Warning}`; `IssueCode::{UnsupportedFormat, DuplicateId, UnknownReference, InvalidPattern, EmptyMatch, PatternConflict, TieBrokenLate, Shadowed, FailOpen}`.

Validator rules (spec §8.2.1): identical pattern + same scope + different action or target → `PatternConflict` error; `*.x` and `suffix:x` in the same scope with different actions → `TieBrokenLate` warning; a rule whose every pattern is matched at equal specificity by a stronger rule in the same scope → `Shadowed` warning (an identical pattern with another action is already an error and is not counted twice).

- [ ] **Step 1: Write the failing tests**

Replace `crates/veyra-policy/src/lib.rs` with:

```rust
//! Veyra policy core: domain names, patterns, rule precedence and explain (spec §8).

mod compile;
mod model;
mod name;
mod pattern;

pub use compile::{Issue, IssueCode, Policy, Rule, Severity};
pub use model::*;
pub use name::{Name, NameError};
pub use pattern::{Pattern, PatternError, Specificity};
```

Create `crates/veyra-policy/src/compile.rs` containing only the test module:

```rust
#[cfg(test)]
mod tests {
    use super::*;
    use IssueCode::*;

    const EXAMPLE: &str = include_str!("../../../configs/examples/home.yaml");

    fn bundle(rules: &str) -> Bundle {
        let text = format!(
            "apiVersion: veyra.io/v1alpha1\nkind: PolicyBundle\nmetadata: {{ name: t }}\nspec:\n  \
             nodes: [{{ id: de-01 }}, {{ id: us-01 }}]\n  profiles: [{{ id: home }}]\n  \
             devices: [{{ id: tv, profile: home }}]\n  rules:\n{rules}"
        );
        Bundle::from_yaml(&text).unwrap()
    }

    fn codes(issues: &[Issue]) -> Vec<IssueCode> {
        issues.iter().map(|i| i.code).collect()
    }

    fn errors(rules: &str) -> Vec<IssueCode> {
        codes(&Policy::compile(&bundle(rules)).unwrap_err())
    }

    fn warnings(rules: &str) -> Vec<IssueCode> {
        codes(&Policy::compile(&bundle(rules)).unwrap().1)
    }

    #[test]
    fn example_config_compiles_cleanly() {
        let (policy, issues) = Policy::compile(&Bundle::from_yaml(EXAMPLE).unwrap()).unwrap();
        assert!(issues.is_empty(), "{issues:?}");
        assert_eq!(policy.rules.len(), 4);
    }

    #[test]
    fn rejects_other_formats() {
        let mut b = bundle("    []\n");
        b.api_version = "veyra.io/v2".into();
        assert_eq!(
            codes(&Policy::compile(&b).unwrap_err()),
            vec![UnsupportedFormat]
        );
    }

    #[test]
    fn rejects_duplicate_rule_ids() {
        let rules = "    - { id: a, scope: default, match: { domains: [x.example] }, action: { type: block } }\n    \
                     - { id: a, scope: default, match: { domains: [y.example] }, action: { type: block } }\n";
        assert_eq!(errors(rules), vec![DuplicateId]);
    }

    #[test]
    fn rejects_unknown_references() {
        let rules = "    - { id: a, scope: { profile: nobody }, match: { domains: [x.example] }, action: { type: block } }\n    \
                     - { id: b, scope: { device: phone }, match: { domains: [x.example] }, action: { type: block } }\n    \
                     - { id: c, scope: default, match: { domains: [x.example] }, action: { type: proxy_node, node: jp-01 } }\n";
        assert_eq!(
            errors(rules),
            vec![UnknownReference, UnknownReference, UnknownReference]
        );
    }

    #[test]
    fn rejects_bad_and_empty_patterns() {
        let rules = "    - { id: a, scope: default, match: { domains: [\"a..b\"] }, action: { type: block } }\n    \
                     - { id: b, scope: default, match: { domains: [] }, action: { type: block } }\n";
        assert_eq!(errors(rules), vec![InvalidPattern, EmptyMatch]);
    }

    #[test]
    fn same_pattern_and_scope_with_different_action_is_an_error() {
        let rules = "    - { id: a, scope: default, match: { domains: [x.example] }, action: { type: block } }\n    \
                     - { id: b, scope: default, match: { domains: [x.example] }, action: { type: direct } }\n";
        assert_eq!(errors(rules), vec![PatternConflict]);
    }

    #[test]
    fn same_pattern_with_different_targets_is_an_error() {
        let rules = "    - { id: a, scope: default, match: { domains: [x.example] }, action: { type: proxy_node, node: de-01 } }\n    \
                     - { id: b, scope: default, match: { domains: [x.example] }, action: { type: proxy_node, node: us-01 } }\n";
        assert_eq!(errors(rules), vec![PatternConflict]);
    }

    #[test]
    fn same_pattern_in_different_scopes_is_fine() {
        let rules = "    - { id: a, scope: default, match: { domains: [x.example] }, action: { type: block } }\n    \
                     - { id: b, scope: { profile: home }, match: { domains: [x.example] }, action: { type: direct } }\n";
        assert_eq!(warnings(rules), vec![]);
    }

    #[test]
    fn exact_exception_under_a_suffix_is_fine() {
        let rules = "    - { id: a, scope: default, match: { domains: [\"suffix:x.example\"] }, action: { type: block } }\n    \
                     - { id: b, scope: default, match: { domains: [login.x.example] }, action: { type: direct } }\n";
        assert_eq!(warnings(rules), vec![]);
    }

    #[test]
    fn wildcard_and_suffix_of_the_same_base_warn() {
        let rules = "    - { id: a, scope: default, match: { domains: [\"*.x.example\"] }, action: { type: block } }\n    \
                     - { id: b, scope: default, match: { domains: [\"suffix:x.example\"] }, action: { type: direct } }\n";
        assert_eq!(warnings(rules), vec![TieBrokenLate]);
    }

    #[test]
    fn duplicate_rule_is_reported_as_shadowed() {
        let rules = "    - { id: a, scope: default, match: { domains: [x.example] }, action: { type: block } }\n    \
                     - { id: b, scope: default, match: { domains: [x.example] }, action: { type: block } }\n";
        let (_, issues) = Policy::compile(&bundle(rules)).unwrap();
        assert_eq!(codes(&issues), vec![Shadowed]);
        assert_eq!(issues[0].rules, vec!["b".to_string()]);
    }

    #[test]
    fn fail_open_is_a_warning() {
        let rules = "    - { id: a, scope: default, match: { domains: [x.example] }, action: { type: proxy_node, node: de-01, onFailure: direct } }\n";
        assert_eq!(warnings(rules), vec![FailOpen]);
    }

    #[test]
    fn disabled_rules_do_not_conflict() {
        let rules = "    - { id: a, scope: default, match: { domains: [x.example] }, action: { type: block } }\n    \
                     - { id: b, enabled: false, scope: default, match: { domains: [x.example] }, action: { type: direct } }\n";
        let (policy, issues) = Policy::compile(&bundle(rules)).unwrap();
        assert!(issues.is_empty());
        assert_eq!(policy.rules.len(), 1);
    }

    #[test]
    fn issue_display_is_one_readable_line() {
        let rules = "    - { id: a, scope: default, match: { domains: [x.example] }, action: { type: block } }\n    \
                     - { id: b, scope: default, match: { domains: [x.example] }, action: { type: direct } }\n";
        let issues = Policy::compile(&bundle(rules)).unwrap_err();
        assert_eq!(
            issues[0].to_string(),
            "error[PatternConflict] a, b: both match x.example in the same scope with different actions"
        );
    }
}
```

- [ ] **Step 2: Run the tests to see them fail**

Run: `cargo test -p veyra-policy`
Expected: compilation fails; `Policy`, `Issue`, `IssueCode`, `Rule` and `Severity` are not found.

- [ ] **Step 3: Write the implementation**

Insert above the `#[cfg(test)]` line of `crates/veyra-policy/src/compile.rs`:

```rust
//! Turns a parsed `Bundle` into a checked `Policy`, collecting every problem
//! instead of stopping at the first one (spec §8.2.1 validator rules, §17.2).

use crate::model::{API_VERSION, Action, Bundle, DnsSpec, KIND, Mode, OnFailure, Scope};
use crate::pattern::Pattern;
use std::cmp::Reverse;
use std::collections::BTreeSet;
use std::fmt;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Severity {
    Error,
    Warning,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum IssueCode {
    UnsupportedFormat,
    DuplicateId,
    UnknownReference,
    InvalidPattern,
    EmptyMatch,
    PatternConflict,
    TieBrokenLate,
    Shadowed,
    FailOpen,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Issue {
    pub severity: Severity,
    pub code: IssueCode,
    pub rules: Vec<String>,
    pub message: String,
}

impl fmt::Display for Issue {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let severity = match self.severity {
            Severity::Error => "error",
            Severity::Warning => "warning",
        };
        write!(f, "{severity}[{:?}]", self.code)?;
        if !self.rules.is_empty() {
            write!(f, " {}", self.rules.join(", "))?;
        }
        write!(f, ": {}", self.message)
    }
}

/// An enabled rule with parsed patterns.
#[derive(Debug, Clone)]
pub struct Rule {
    pub id: String,
    pub scope: Scope,
    pub patterns: Vec<Pattern>,
    pub action: Action,
    pub dns: Option<DnsSpec>,
    pub modes: Vec<Mode>,
}

impl Rule {
    /// Spec §8.2.1 steps 3 and 4: BLOCK > PROXY > DIRECT, then the smaller rule id.
    pub(crate) fn late_rank(&self) -> (u8, Reverse<&str>) {
        let action = match self.action {
            Action::Block => 2,
            Action::ProxyNode { .. } => 1,
            Action::Direct => 0,
        };
        (action, Reverse(self.id.as_str()))
    }
}

#[derive(Debug, Clone)]
pub struct Policy {
    pub name: String,
    pub(crate) rules: Vec<Rule>,
}

fn issue(severity: Severity, code: IssueCode, rules: &[&str], message: String) -> Issue {
    Issue {
        severity,
        code,
        rules: rules.iter().map(|r| r.to_string()).collect(),
        message,
    }
}

impl Policy {
    /// `Ok` carries warnings; `Err` carries every issue when at least one is an error.
    pub fn compile(bundle: &Bundle) -> Result<(Policy, Vec<Issue>), Vec<Issue>> {
        use IssueCode::*;
        use Severity::*;
        let mut issues = Vec::new();
        if bundle.api_version != API_VERSION || bundle.kind != KIND {
            issues.push(issue(
                Error,
                UnsupportedFormat,
                &[],
                format!(
                    "expected apiVersion {API_VERSION} and kind {KIND}, got {} and {}",
                    bundle.api_version, bundle.kind
                ),
            ));
        }
        let spec = &bundle.spec;
        let nodes = unique_ids(
            "node",
            spec.nodes.iter().map(|n| n.id.as_str()),
            &mut issues,
        );
        let profiles = unique_ids(
            "profile",
            spec.profiles.iter().map(|p| p.id.as_str()),
            &mut issues,
        );
        let devices = unique_ids(
            "device",
            spec.devices.iter().map(|d| d.id.as_str()),
            &mut issues,
        );
        unique_ids(
            "rule",
            spec.rules.iter().map(|r| r.id.as_str()),
            &mut issues,
        );

        for d in &spec.devices {
            if !profiles.contains(d.profile.as_str()) {
                let msg = format!("device {} uses unknown profile {}", d.id, d.profile);
                issues.push(issue(Error, UnknownReference, &[], msg));
            }
        }

        let mut rules = Vec::new();
        for r in &spec.rules {
            let id = [r.id.as_str()];
            match &r.scope {
                Scope::Profile(p) if !profiles.contains(p.as_str()) => {
                    issues.push(issue(
                        Error,
                        UnknownReference,
                        &id,
                        format!("unknown profile {p}"),
                    ));
                }
                Scope::Device(d) if !devices.contains(d.as_str()) => {
                    issues.push(issue(
                        Error,
                        UnknownReference,
                        &id,
                        format!("unknown device {d}"),
                    ));
                }
                _ => {}
            }
            if let Action::ProxyNode { node, on_failure } = &r.action {
                if !nodes.contains(node.as_str()) {
                    issues.push(issue(
                        Error,
                        UnknownReference,
                        &id,
                        format!("unknown node {node}"),
                    ));
                }
                if *on_failure == OnFailure::Direct {
                    let msg = format!(
                        "onFailure: direct sends traffic around the proxy when {node} is down"
                    );
                    issues.push(issue(Warning, FailOpen, &id, msg));
                }
            }
            if r.matcher.domains.is_empty() {
                issues.push(issue(
                    Error,
                    EmptyMatch,
                    &id,
                    "match.domains is empty".into(),
                ));
            }
            let mut patterns = Vec::new();
            for d in &r.matcher.domains {
                match Pattern::parse(d) {
                    Ok(p) => patterns.push(p),
                    Err(e) => issues.push(issue(Error, InvalidPattern, &id, format!("{d:?}: {e}"))),
                }
            }
            if r.enabled {
                rules.push(Rule {
                    id: r.id.clone(),
                    scope: r.scope.clone(),
                    patterns,
                    action: r.action.clone(),
                    dns: r.dns.clone(),
                    modes: r.modes_allowed.clone(),
                });
            }
        }
        check_overlaps(&rules, &mut issues);

        if issues.iter().any(|i| i.severity == Error) {
            return Err(issues);
        }
        let policy = Policy {
            name: bundle.metadata.name.clone(),
            rules,
        };
        Ok((policy, issues))
    }

    /// Enabled rules in file order.
    pub fn rules(&self) -> &[Rule] {
        &self.rules
    }
}

fn unique_ids<'a>(
    kind: &str,
    ids: impl Iterator<Item = &'a str>,
    issues: &mut Vec<Issue>,
) -> BTreeSet<&'a str> {
    let mut seen = BTreeSet::new();
    for id in ids {
        if !seen.insert(id) {
            let msg = format!("{kind} id {id} is used more than once");
            issues.push(issue(Severity::Error, IssueCode::DuplicateId, &[id], msg));
        }
    }
    seen
}

/// Pairwise checks inside one scope. Rules in different scopes are always
/// separated by step 2 of §8.2.1, so they never conflict.
// ponytail: O(rules² × patterns²); index by pattern base name if bundles reach thousands of rules.
fn check_overlaps(rules: &[Rule], issues: &mut Vec<Issue>) {
    for (i, a) in rules.iter().enumerate() {
        for b in rules
            .iter()
            .skip(i + 1)
            .filter(|b| b.scope == a.scope && b.action != a.action)
        {
            for p in &a.patterns {
                for q in &b.patterns {
                    let ids = [a.id.as_str(), b.id.as_str()];
                    if p == q {
                        let msg =
                            format!("both match {p} in the same scope with different actions");
                        issues.push(issue(
                            Severity::Error,
                            IssueCode::PatternConflict,
                            &ids,
                            msg,
                        ));
                    } else if same_tier(p, q) {
                        let msg = format!(
                            "{p} and {q} overlap at equal specificity; decided by action or rule id"
                        );
                        issues.push(issue(
                            Severity::Warning,
                            IssueCode::TieBrokenLate,
                            &ids,
                            msg,
                        ));
                    }
                }
            }
        }
    }
    for r in rules {
        // An identical pattern with another action is already a PatternConflict error.
        let hidden = |p: &Pattern| {
            rules.iter().any(|o| {
                o.id != r.id
                    && o.scope == r.scope
                    && o.late_rank() > r.late_rank()
                    && o.patterns.iter().any(|q| {
                        covers_at_same_specificity(q, p) && (q != p || o.action == r.action)
                    })
            })
        };
        if !r.patterns.is_empty() && r.patterns.iter().all(hidden) {
            let msg = "never decides: a stronger rule in the same scope matches every name at equal specificity";
            issues.push(issue(
                Severity::Warning,
                IssueCode::Shadowed,
                &[&r.id],
                msg.into(),
            ));
        }
    }
}

/// `q` matches every name `p` matches, at the same specificity. Only then can
/// `q`'s rule hide `p`'s rule completely: a broader `q` always loses step 1.
fn covers_at_same_specificity(q: &Pattern, p: &Pattern) -> bool {
    match (p, q) {
        (Pattern::Exact(a), Pattern::Exact(b)) | (Pattern::Suffix(a), Pattern::Suffix(b)) => a == b,
        (Pattern::Wildcard(a), Pattern::Wildcard(b) | Pattern::Suffix(b)) => a == b,
        _ => false,
    }
}

/// Two different patterns that overlap at equal specificity: only `*.x` and `suffix:x`.
fn same_tier(p: &Pattern, q: &Pattern) -> bool {
    matches!(
        (p, q),
        (Pattern::Wildcard(a), Pattern::Suffix(b)) | (Pattern::Suffix(a), Pattern::Wildcard(b)) if a == b
    )
}
```

- [ ] **Step 4: Run the tests to see them pass**

Run: `cargo test -p veyra-policy`
Expected: `test result: ok. 32 passed; 0 failed`.

- [ ] **Step 5: Format and lint**

Run: `cargo fmt --all && cargo clippy --workspace --all-targets -- -D warnings`
Expected: no warnings.

- [ ] **Step 6: Commit**

```bash
git add crates/veyra-policy
git commit -m "feat(policy): compile bundles and report conflicts and shadowed rules"
```

---

### Task 5: Decisions and explain data

**Files:**
- Create: `crates/veyra-policy/src/decide.rs`, `crates/veyra-policy/tests/determinism.rs`
- Modify: `crates/veyra-policy/src/lib.rs`, `crates/veyra-policy/src/compile.rs`

**Interfaces:**
- Consumes: `Policy`, `Rule::late_rank` (Task 4); `Pattern`, `Specificity` (Task 2); `Name` (Task 1); `Action`, `DefaultAction`, `Scope` (Task 3).
- Modifies: `Policy` gains `pub(crate) profiles: BTreeMap<String, DefaultAction>` (profile id → default action) and `pub(crate) devices: BTreeMap<String, String>` (device id → profile id).
- Produces: `Policy::subject(&self, profile: Option<&str>, device: Option<&str>) -> Result<Subject, SubjectError>` (a device implies its profile); `Policy::decide(&self, &Subject, &Name) -> Decision`; `Subject { profile: String, device: Option<String> }`; `SubjectError::{Missing, UnknownProfile(String), UnknownDevice(String), ProfileMismatch { device, actual, requested }}`; `Decision { policy, domain, profile, device, route: Action, upstream_group: Option<String>, matched_rule_id: Option<String>, matched_pattern: Option<String>, decided_by: DecidedBy }` (serializable; this is the `veyra explain` JSON); `DecidedBy::{Default, OnlyMatch, Specificity, Scope, Action, RuleId}` serialized in snake_case.

`decided_by` names the first precedence step at which the winning rule beat the runner-up; `default` means no rule matched and the profile's `defaultAction` applied. Route and upstream group both come from the winning rule.

- [ ] **Step 1: Write the failing tests**

Replace `crates/veyra-policy/src/lib.rs` with:

```rust
//! Veyra policy core: domain names, patterns, rule precedence and explain (spec §8).

mod compile;
mod decide;
mod model;
mod name;
mod pattern;

pub use compile::{Issue, IssueCode, Policy, Rule, Severity};
pub use decide::{DecidedBy, Decision, Subject, SubjectError};
pub use model::*;
pub use name::{Name, NameError};
pub use pattern::{Pattern, PatternError, Specificity};
```

Create `crates/veyra-policy/src/decide.rs` containing only the test module:

```rust
#[cfg(test)]
mod tests {
    use super::*;
    use crate::model::{Bundle, OnFailure};

    const EXAMPLE: &str = include_str!("../../../configs/examples/home.yaml");

    fn policy(rules: &str) -> Policy {
        let text = format!(
            "apiVersion: veyra.io/v1alpha1\nkind: PolicyBundle\nmetadata: {{ name: t }}\nspec:\n  \
             nodes: [{{ id: de-01 }}, {{ id: us-01 }}]\n  \
             profiles: [{{ id: home }}, {{ id: strict, defaultAction: block }}]\n  \
             devices: [{{ id: tv, profile: home }}, {{ id: phone, profile: home }}]\n  rules:\n{rules}"
        );
        Policy::compile(&Bundle::from_yaml(&text).unwrap())
            .unwrap()
            .0
    }

    fn example() -> Policy {
        Policy::compile(&Bundle::from_yaml(EXAMPLE).unwrap())
            .unwrap()
            .0
    }

    fn decide(p: &Policy, profile: Option<&str>, device: Option<&str>, domain: &str) -> Decision {
        p.decide(
            &p.subject(profile, device).unwrap(),
            &Name::parse(domain).unwrap(),
        )
    }

    fn proxy(node: &str) -> Action {
        Action::ProxyNode {
            node: node.into(),
            on_failure: OnFailure::Block,
        }
    }

    #[test]
    fn exact_exception_beats_suffix() {
        let d = decide(&example(), Some("home"), None, "login.media.example");
        assert_eq!(d.route, Action::Direct);
        assert_eq!(d.matched_rule_id.as_deref(), Some("media-login-direct"));
        assert_eq!(d.decided_by, DecidedBy::Specificity);
    }

    #[test]
    fn suffix_matches_the_apex() {
        let d = decide(&example(), Some("home"), None, "media.example");
        assert_eq!(d.route, proxy("de-01"));
        assert_eq!(d.matched_pattern.as_deref(), Some("suffix:media.example"));
        assert_eq!(d.decided_by, DecidedBy::OnlyMatch);
    }

    #[test]
    fn wildcard_does_not_match_the_apex() {
        let p = policy(
            "    - { id: a, scope: default, match: { domains: [\"*.x.example\"] }, action: { type: block } }\n",
        );
        assert_eq!(
            decide(&p, Some("home"), None, "x.example").decided_by,
            DecidedBy::Default
        );
        assert_eq!(
            decide(&p, Some("home"), None, "a.x.example").route,
            Action::Block
        );
    }

    #[test]
    fn device_scope_beats_profile_scope() {
        let p = policy(
            "    - { id: a, scope: { profile: home }, match: { domains: [x.example] }, action: { type: proxy_node, node: de-01 } }\n    \
             - { id: b, scope: { device: tv }, match: { domains: [x.example] }, action: { type: proxy_node, node: us-01 } }\n",
        );
        let tv = decide(&p, None, Some("tv"), "x.example");
        assert_eq!(tv.route, proxy("us-01"));
        assert_eq!(tv.decided_by, DecidedBy::Scope);
        assert_eq!(
            decide(&p, None, Some("phone"), "x.example").route,
            proxy("de-01")
        );
    }

    #[test]
    fn block_beats_proxy_at_equal_specificity_and_scope() {
        let p = policy(
            "    - { id: a, scope: default, match: { domains: [\"suffix:x.example\"] }, action: { type: proxy_node, node: de-01 } }\n    \
             - { id: b, scope: default, match: { domains: [\"*.x.example\"] }, action: { type: block } }\n",
        );
        let d = decide(&p, Some("home"), None, "a.x.example");
        assert_eq!(d.route, Action::Block);
        assert_eq!(d.decided_by, DecidedBy::Action);
    }

    #[test]
    fn smaller_rule_id_wins_the_last_tie() {
        let p = policy(
            "    - { id: b, scope: default, match: { domains: [\"suffix:x.example\"] }, action: { type: proxy_node, node: us-01 } }\n    \
             - { id: a, scope: default, match: { domains: [\"*.x.example\"] }, action: { type: proxy_node, node: de-01 } }\n",
        );
        let d = decide(&p, Some("home"), None, "a.x.example");
        assert_eq!(d.matched_rule_id.as_deref(), Some("a"));
        assert_eq!(d.decided_by, DecidedBy::RuleId);
    }

    #[test]
    fn no_match_uses_the_profile_default() {
        let p = policy("    []\n");
        let home = decide(&p, Some("home"), None, "x.example");
        assert_eq!(
            (home.route, home.decided_by),
            (Action::Direct, DecidedBy::Default)
        );
        assert_eq!(
            decide(&p, Some("strict"), None, "x.example").route,
            Action::Block
        );
    }

    #[test]
    fn device_rules_carry_the_upstream_group() {
        let d = decide(&example(), None, Some("work-laptop"), "git.corp.example");
        assert_eq!(d.profile, "home");
        assert_eq!(d.upstream_group.as_deref(), Some("corp"));
        assert_eq!(
            decide(&example(), None, Some("tv"), "git.corp.example").upstream_group,
            None
        );
    }

    #[test]
    fn idn_rules_match_idn_queries() {
        let p = policy(
            "    - { id: a, scope: default, match: { domains: [\"suffix:пример.рф\"] }, action: { type: block } }\n",
        );
        let d = decide(&p, Some("home"), None, "WWW.Пример.РФ.");
        assert_eq!(d.route, Action::Block);
        assert_eq!(d.domain, "www.xn--e1afmkfd.xn--p1ai");
    }

    #[test]
    fn subject_errors() {
        let p = example();
        assert_eq!(p.subject(None, None), Err(SubjectError::Missing));
        assert_eq!(
            p.subject(Some("nobody"), None),
            Err(SubjectError::UnknownProfile("nobody".into()))
        );
        assert_eq!(
            p.subject(None, Some("toaster")),
            Err(SubjectError::UnknownDevice("toaster".into()))
        );
        assert!(matches!(
            p.subject(Some("other"), Some("tv")),
            Err(SubjectError::ProfileMismatch { .. })
        ));
        assert_eq!(
            p.subject(Some("home"), Some("tv"))
                .unwrap()
                .device
                .as_deref(),
            Some("tv")
        );
    }
}
```

Create `crates/veyra-policy/tests/determinism.rs`:

```rust
//! REQ-POL-001: the decision must not depend on the order of rules in the file.

use std::collections::BTreeSet;
use veyra_policy::{Bundle, Name, Policy};

const RULES: &str = r#"
apiVersion: veyra.io/v1alpha1
kind: PolicyBundle
metadata: { name: perm }
spec:
  nodes: [{ id: de-01 }, { id: us-01 }]
  profiles: [{ id: home }]
  devices: [{ id: tv, profile: home }]
  rules:
    - { id: a, scope: { profile: home }, match: { domains: ["suffix:media.example"] }, action: { type: proxy_node, node: de-01 } }
    - { id: b, scope: { profile: home }, match: { domains: ["*.media.example"] }, action: { type: proxy_node, node: us-01 } }
    - { id: c, scope: { profile: home }, match: { domains: [login.media.example] }, action: { type: direct } }
    - { id: d, scope: { device: tv }, match: { domains: ["suffix:media.example"] }, action: { type: direct } }
    - { id: e, scope: default, match: { domains: ["suffix:example"] }, action: { type: block } }
"#;

/// Every ordering of `items`, by Heap's algorithm.
fn permutations<T: Clone>(items: &mut [T], k: usize, out: &mut Vec<Vec<T>>) {
    if k <= 1 {
        out.push(items.to_vec());
        return;
    }
    permutations(items, k - 1, out);
    for i in 0..k - 1 {
        let j = if k % 2 == 0 { i } else { 0 };
        items.swap(j, k - 1);
        permutations(items, k - 1, out);
    }
}

#[test]
fn decisions_do_not_depend_on_rule_order() {
    let base = Bundle::from_yaml(RULES).unwrap();
    let mut orders = Vec::new();
    let mut rules = base.spec.rules.clone();
    let len = rules.len();
    permutations(&mut rules, len, &mut orders);
    let distinct: BTreeSet<Vec<&str>> = orders
        .iter()
        .map(|o| o.iter().map(|r| r.id.as_str()).collect())
        .collect();
    assert_eq!(distinct.len(), 120);

    let names = [
        "media.example",
        "x.media.example",
        "login.media.example",
        "other.example",
        "unrelated.test",
    ];
    let decide_all = |bundle: &Bundle| {
        let (policy, _) = Policy::compile(bundle).unwrap();
        let mut out = Vec::new();
        for (profile, device) in [(Some("home"), None), (None, Some("tv"))] {
            let subject = policy.subject(profile, device).unwrap();
            for n in names {
                out.push(policy.decide(&subject, &Name::parse(n).unwrap()));
            }
        }
        out
    };

    let expected = decide_all(&base);
    for order in orders {
        let mut bundle = base.clone();
        bundle.spec.rules = order;
        assert_eq!(decide_all(&bundle), expected);
    }
}
```

- [ ] **Step 2: Run the tests to see them fail**

Run: `cargo test -p veyra-policy`
Expected: compilation fails; `Subject`, `Decision`, `DecidedBy` and `SubjectError` are not found.

- [ ] **Step 3: Give `Policy` the profile and device maps**

In `crates/veyra-policy/src/compile.rs`, replace the two import lines

```rust
use crate::model::{API_VERSION, Action, Bundle, DnsSpec, KIND, Mode, OnFailure, Scope};
```

```rust
use std::collections::BTreeSet;
```

with

```rust
use crate::model::{
    API_VERSION, Action, Bundle, DefaultAction, DnsSpec, KIND, Mode, OnFailure, Scope,
};
```

```rust
use std::collections::{BTreeMap, BTreeSet};
```

In `pub struct Policy`, after `pub name: String,` add:

```rust
    pub(crate) profiles: BTreeMap<String, DefaultAction>,
    pub(crate) devices: BTreeMap<String, String>,
```

In `Policy::compile`, in the `let policy = Policy {` literal, after `name: bundle.metadata.name.clone(),` add:

```rust
            profiles: spec
                .profiles
                .iter()
                .map(|p| (p.id.clone(), p.default_action))
                .collect(),
            devices: spec
                .devices
                .iter()
                .map(|d| (d.id.clone(), d.profile.clone()))
                .collect(),
```

- [ ] **Step 4: Write the implementation**

Insert above the `#[cfg(test)]` line of `crates/veyra-policy/src/decide.rs`:

```rust
//! Rule selection (spec §8.2.1) and the explanation `veyra explain` prints (spec §8.4).

use crate::compile::{Policy, Rule};
use crate::model::{Action, DefaultAction, Scope};
use crate::name::Name;
use crate::pattern::{Pattern, Specificity};
use serde::Serialize;
use std::cmp::Reverse;

/// The profile, and optionally the device, a decision is made for.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Subject {
    pub profile: String,
    pub device: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
pub enum SubjectError {
    #[error("give a profile or a device")]
    Missing,
    #[error("unknown profile {0:?}")]
    UnknownProfile(String),
    #[error("unknown device {0:?}")]
    UnknownDevice(String),
    #[error("device {device:?} belongs to profile {actual:?}, not {requested:?}")]
    ProfileMismatch {
        device: String,
        actual: String,
        requested: String,
    },
}

/// The precedence step that separated the winning rule from the runner-up.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum DecidedBy {
    Default,
    OnlyMatch,
    Specificity,
    Scope,
    Action,
    RuleId,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct Decision {
    pub policy: String,
    pub domain: String,
    pub profile: String,
    pub device: Option<String>,
    pub route: Action,
    pub upstream_group: Option<String>,
    pub matched_rule_id: Option<String>,
    pub matched_pattern: Option<String>,
    pub decided_by: DecidedBy,
}

/// Sort key for spec §8.2.1: specificity, then scope, then action and rule id.
type Key<'a> = (Specificity, u8, (u8, Reverse<&'a str>));

impl Policy {
    pub fn subject(
        &self,
        profile: Option<&str>,
        device: Option<&str>,
    ) -> Result<Subject, SubjectError> {
        match (profile, device) {
            (requested, Some(d)) => {
                let actual = self
                    .devices
                    .get(d)
                    .ok_or_else(|| SubjectError::UnknownDevice(d.into()))?;
                if let Some(p) = requested.filter(|p| p != actual) {
                    return Err(SubjectError::ProfileMismatch {
                        device: d.into(),
                        actual: actual.clone(),
                        requested: p.into(),
                    });
                }
                Ok(Subject {
                    profile: actual.clone(),
                    device: Some(d.into()),
                })
            }
            (Some(p), None) if self.profiles.contains_key(p) => Ok(Subject {
                profile: p.into(),
                device: None,
            }),
            (Some(p), None) => Err(SubjectError::UnknownProfile(p.into())),
            (None, None) => Err(SubjectError::Missing),
        }
    }

    pub fn decide(&self, subject: &Subject, name: &Name) -> Decision {
        let mut candidates: Vec<(Key<'_>, &Rule, &Pattern)> = self
            .rules
            .iter()
            .filter(|r| applies(&r.scope, subject))
            .filter_map(|r| {
                let best = r
                    .patterns
                    .iter()
                    .filter(|p| p.matches(name))
                    .max_by_key(|p| p.specificity())?;
                Some((
                    (best.specificity(), scope_rank(&r.scope), r.late_rank()),
                    r,
                    best,
                ))
            })
            .collect();
        candidates.sort_by(|a, b| b.0.cmp(&a.0));

        let mut decision = Decision {
            policy: self.name.clone(),
            domain: name.to_string(),
            profile: subject.profile.clone(),
            device: subject.device.clone(),
            route: match self.profiles.get(&subject.profile) {
                Some(DefaultAction::Block) => Action::Block,
                _ => Action::Direct,
            },
            upstream_group: None,
            matched_rule_id: None,
            matched_pattern: None,
            decided_by: DecidedBy::Default,
        };
        if let Some((best, rule, pattern)) = candidates.first() {
            decision.route = rule.action.clone();
            decision.upstream_group = rule.dns.as_ref().map(|d| d.upstream_group.clone());
            decision.matched_rule_id = Some(rule.id.clone());
            decision.matched_pattern = Some(pattern.to_string());
            decision.decided_by = match candidates.get(1) {
                None => DecidedBy::OnlyMatch,
                Some((second, ..)) if best.0 != second.0 => DecidedBy::Specificity,
                Some((second, ..)) if best.1 != second.1 => DecidedBy::Scope,
                Some((second, ..)) if best.2.0 != second.2.0 => DecidedBy::Action,
                Some(_) => DecidedBy::RuleId,
            };
        }
        decision
    }
}

fn applies(scope: &Scope, subject: &Subject) -> bool {
    match scope {
        Scope::Default => true,
        Scope::Profile(p) => *p == subject.profile,
        Scope::Device(d) => subject.device.as_deref() == Some(d.as_str()),
    }
}

fn scope_rank(scope: &Scope) -> u8 {
    match scope {
        Scope::Default => 0,
        Scope::Profile(_) => 1,
        Scope::Device(_) => 2,
    }
}
```

- [ ] **Step 5: Run the tests to see them pass**

Run: `cargo test -p veyra-policy`
Expected: the unit tests report `test result: ok. 42 passed; 0 failed`, and `tests/determinism.rs` reports `test result: ok. 1 passed`.

- [ ] **Step 6: Format and lint**

Run: `cargo fmt --all && cargo clippy --workspace --all-targets -- -D warnings`
Expected: no warnings.

- [ ] **Step 7: Commit**

```bash
git add crates/veyra-policy
git commit -m "feat(policy): deterministic rule decisions with explain data"
```

---

### Task 6: `veyra` CLI

**Files:**
- Modify: `Cargo.toml` (workspace members), `Veyra_Project_Specification_RU.md` (§8.3, one sentence)
- Create: `apps/veyra-cli/Cargo.toml`, `apps/veyra-cli/src/main.rs`, `apps/veyra-cli/tests/cli.rs`, `apps/veyra-cli/tests/fixtures/conflict.yaml`, `apps/veyra-cli/tests/fixtures/broken.yaml`, `README.md`

**Interfaces:**
- Consumes: `Bundle::from_yaml`, `Policy::compile`, `Policy::subject`, `Policy::decide`, `Name::parse`, `Issue`, `Severity` (Tasks 1–5).
- Produces: binary `veyra` with `veyra validate <config>` (prints warnings and a `valid, N warning(s)` line, exit 0; or prints every issue and exits 1) and `veyra explain --config <file> (--profile <id> | --device <id>) <domain>` (prints the `Decision` as pretty JSON on stdout, exit 0; any problem goes to stderr with exit 1). Clap usage errors exit 2.

- [ ] **Step 1: Add the crate with a stub binary**

In `Cargo.toml`, change the members line to:

```toml
members = ["crates/veyra-policy", "apps/veyra-cli"]
```

Create `apps/veyra-cli/Cargo.toml`:

```toml
[package]
name = "veyra-cli"
version.workspace = true
edition.workspace = true
rust-version.workspace = true
publish.workspace = true

[[bin]]
name = "veyra"
path = "src/main.rs"

[dependencies]
veyra-policy.workspace = true
clap.workspace = true
serde_json.workspace = true

[lints]
workspace = true
```

Create `apps/veyra-cli/src/main.rs` with a stub so the failing tests can build:

```rust
fn main() {}
```

- [ ] **Step 2: Write the failing tests**

Create `apps/veyra-cli/tests/fixtures/conflict.yaml`:

```yaml
apiVersion: veyra.io/v1alpha1
kind: PolicyBundle
metadata: { name: conflict }
spec:
  profiles: [{ id: home }]
  rules:
    - { id: a, scope: default, match: { domains: [media.example] }, action: { type: block } }
    - { id: b, scope: default, match: { domains: [media.example] }, action: { type: direct } }
```

Create `apps/veyra-cli/tests/fixtures/broken.yaml` (the `action` line is indented one space too little on purpose):

```yaml
apiVersion: veyra.io/v1alpha1
kind: PolicyBundle
metadata:
  name: broken
spec:
  rules:
    - id: a
      scope: default
     action: { type: block }
```

Create `apps/veyra-cli/tests/cli.rs`:

```rust
//! End-to-end checks of the `veyra` binary.

use std::process::{Command, Output};

const EXAMPLE: &str = concat!(
    env!("CARGO_MANIFEST_DIR"),
    "/../../configs/examples/home.yaml"
);
const CONFLICT: &str = concat!(env!("CARGO_MANIFEST_DIR"), "/tests/fixtures/conflict.yaml");
const BROKEN: &str = concat!(env!("CARGO_MANIFEST_DIR"), "/tests/fixtures/broken.yaml");

fn veyra(args: &[&str]) -> Output {
    Command::new(env!("CARGO_BIN_EXE_veyra"))
        .args(args)
        .output()
        .expect("run veyra")
}

fn text(bytes: &[u8]) -> String {
    String::from_utf8_lossy(bytes).into_owned()
}

#[test]
fn validate_accepts_the_example() {
    let out = veyra(&["validate", EXAMPLE]);
    assert!(out.status.success(), "{}", text(&out.stderr));
    assert!(text(&out.stdout).contains("valid, 0 warning(s)"));
}

#[test]
fn validate_reports_conflicts_and_fails() {
    let out = veyra(&["validate", CONFLICT]);
    assert_eq!(out.status.code(), Some(1));
    assert!(
        text(&out.stdout).contains("error[PatternConflict] a, b"),
        "{}",
        text(&out.stdout)
    );
    assert!(text(&out.stderr).contains("1 error(s)"));
}

#[test]
fn malformed_yaml_names_the_line() {
    let out = veyra(&["validate", BROKEN]);
    assert_eq!(out.status.code(), Some(1));
    assert!(text(&out.stderr).contains("line"), "{}", text(&out.stderr));
}

#[test]
fn explain_prints_the_deciding_rule_as_json() {
    let out = veyra(&[
        "explain",
        "--config",
        EXAMPLE,
        "--profile",
        "home",
        "WWW.Media.Example.",
    ]);
    assert!(out.status.success(), "{}", text(&out.stderr));
    let v: serde_json::Value = serde_json::from_slice(&out.stdout).expect("json on stdout");
    assert_eq!(v["domain"], "www.media.example");
    assert_eq!(v["matched_rule_id"], "video-through-germany");
    assert_eq!(v["route"]["type"], "proxy_node");
    assert_eq!(v["route"]["node"], "de-01");
    assert_eq!(v["route"]["onFailure"], "block");
    assert_eq!(v["decided_by"], "only_match");
}

#[test]
fn explain_by_device_applies_device_rules() {
    let out = veyra(&[
        "explain",
        "--config",
        EXAMPLE,
        "--device",
        "work-laptop",
        "git.corp.example",
    ]);
    assert!(out.status.success(), "{}", text(&out.stderr));
    let v: serde_json::Value = serde_json::from_slice(&out.stdout).expect("json on stdout");
    assert_eq!(v["profile"], "home");
    assert_eq!(v["matched_rule_id"], "work-resolver");
    assert_eq!(v["upstream_group"], "corp");
}

#[test]
fn explain_rejects_bad_requests_cleanly() {
    let cases: [&[&str]; 4] = [
        &["--profile", "nobody", "x.example"],
        &["--device", "tv", "--profile", "other", "x.example"],
        &["--profile", "home", "a..b"],
        &["--profile", "home", "a b.example"],
    ];
    for args in cases {
        let mut full = vec!["explain", "--config", EXAMPLE];
        full.extend_from_slice(args);
        let out = veyra(&full);
        assert_eq!(out.status.code(), Some(1), "{args:?}");
        let err = text(&out.stderr);
        assert!(
            !err.is_empty() && !err.contains("panicked"),
            "{args:?}: {err}"
        );
    }
}

#[test]
fn explain_refuses_a_config_with_errors() {
    let out = veyra(&[
        "explain",
        "--config",
        CONFLICT,
        "--profile",
        "home",
        "media.example",
    ]);
    assert_eq!(out.status.code(), Some(1));
    assert!(text(&out.stderr).contains("veyra validate"));
}
```

- [ ] **Step 3: Run the tests to see them fail**

Run: `cargo test -p veyra-cli`
Expected: `test result: FAILED. 0 passed; 7 failed` (the stub prints nothing and exits 0).

- [ ] **Step 4: Write the implementation**

Replace `apps/veyra-cli/src/main.rs` with:

```rust
//! `veyra` command line, stage 1a (spec §23.2): validate and explain a policy file.

use clap::{Parser, Subcommand};
use std::path::{Path, PathBuf};
use std::process::ExitCode;
use veyra_policy::{Bundle, Issue, Name, Policy};

#[derive(Parser)]
#[command(name = "veyra", version, about = "Veyra policy tools")]
struct Cli {
    #[command(subcommand)]
    command: Command,
}

#[derive(Subcommand)]
enum Command {
    /// Check a policy file and print every error and warning.
    Validate { config: PathBuf },
    /// Show which rule decides a domain for a profile or device, as JSON.
    Explain {
        #[arg(long)]
        config: PathBuf,
        #[arg(long, required_unless_present = "device")]
        profile: Option<String>,
        #[arg(long)]
        device: Option<String>,
        domain: String,
    },
}

fn main() -> ExitCode {
    match run(Cli::parse()) {
        Ok(()) => ExitCode::SUCCESS,
        Err(message) => {
            eprintln!("{message}");
            ExitCode::FAILURE
        }
    }
}

fn run(cli: Cli) -> Result<(), String> {
    match cli.command {
        Command::Validate { config } => match load(&config)? {
            Ok((_, warnings)) => {
                warnings.iter().for_each(|w| println!("{w}"));
                println!("{}: valid, {} warning(s)", config.display(), warnings.len());
                Ok(())
            }
            Err(issues) => {
                issues.iter().for_each(|i| println!("{i}"));
                let errors = issues
                    .iter()
                    .filter(|i| i.severity == veyra_policy::Severity::Error)
                    .count();
                Err(format!("{}: {errors} error(s)", config.display()))
            }
        },
        Command::Explain {
            config,
            profile,
            device,
            domain,
        } => {
            let policy = match load(&config)? {
                Ok((policy, _)) => policy,
                Err(issues) => {
                    issues.iter().for_each(|i| eprintln!("{i}"));
                    return Err(format!(
                        "{}: has errors, run `veyra validate`",
                        config.display()
                    ));
                }
            };
            let subject = policy
                .subject(profile.as_deref(), device.as_deref())
                .map_err(|e| e.to_string())?;
            let name = Name::parse(&domain).map_err(|e| format!("{domain:?}: {e}"))?;
            let decision = policy.decide(&subject, &name);
            let json = serde_json::to_string_pretty(&decision).map_err(|e| e.to_string())?;
            println!("{json}");
            Ok(())
        }
    }
}

type Compiled = Result<(Policy, Vec<Issue>), Vec<Issue>>;

fn load(path: &Path) -> Result<Compiled, String> {
    let text = std::fs::read_to_string(path).map_err(|e| format!("{}: {e}", path.display()))?;
    let bundle = Bundle::from_yaml(&text).map_err(|e| format!("{}: {e}", path.display()))?;
    Ok(Policy::compile(&bundle))
}
```

- [ ] **Step 5: Run the whole suite**

Run: `cargo test --workspace`
Expected: `veyra-cli` integration tests `ok. 7 passed`, `veyra-policy` unit tests `ok. 42 passed`, determinism `ok. 1 passed`.

Then try it by hand:

Run: `cargo run -q -p veyra-cli -- validate configs/examples/home.yaml`
Expected: `configs/examples/home.yaml: valid, 0 warning(s)`

Run: `cargo run -q -p veyra-cli -- explain --config configs/examples/home.yaml --profile home login.media.example`
Expected (JSON):

```json
{
  "policy": "home-default",
  "domain": "login.media.example",
  "profile": "home",
  "device": null,
  "route": {
    "type": "direct"
  },
  "upstream_group": null,
  "matched_rule_id": "media-login-direct",
  "matched_pattern": "login.media.example",
  "decided_by": "specificity"
}
```

- [ ] **Step 6: Document**

Create `README.md`:

````markdown
# Veyra

Self-hosted SmartDNS and traffic routing platform. Design and requirements: [Veyra_Project_Specification_RU.md](Veyra_Project_Specification_RU.md).

Current state: policy core and CLI (stage 1a, plan 1 of 4 — see `docs/superpowers/plans/`).

## Build and test

```bash
cargo test --workspace
cargo clippy --workspace --all-targets -- -D warnings
```

## Try it

```bash
cargo run -p veyra-cli -- validate configs/examples/home.yaml
cargo run -p veyra-cli -- explain --config configs/examples/home.yaml --profile home login.media.example
cargo run -p veyra-cli -- explain --config configs/examples/home.yaml --device work-laptop git.corp.example
```
````

In `Veyra_Project_Specification_RU.md` §8.3, replace:

```text
Правило принадлежит ровно одному scope: `default`, профилю или устройству. Профиль задаёт `defaultAction`; отдельного «пользовательского» scope нет — `User` в модели означает администратора (§17.1).
```

with:

```text
Правило принадлежит ровно одному scope: `default`, профилю или устройству. Профиль задаёт `defaultAction`; отдельного «пользовательского» scope нет — `User` в модели означает администратора (§17.1).

Рабочий пример с разделами `nodes` и `devices`, на которые ссылаются правила, — `configs/examples/home.yaml`; проверка — `veyra validate`.
```

- [ ] **Step 7: Format, lint and commit**

Run: `cargo fmt --all --check && cargo clippy --workspace --all-targets -- -D warnings`
Expected: no output from fmt; clippy prints no warnings.

```bash
git add Cargo.toml Cargo.lock apps README.md Veyra_Project_Specification_RU.md
git commit -m "feat(cli): veyra validate and veyra explain"
```

---

## Not in this plan

- Service groups, CIDR rules, `node_group` and `outbound` actions: stages 2–3 of the spec; the model rejects them as unknown until then.
- `dns_answer`, `policy_version` and `warnings` in the explain JSON (spec §18.3): they need the DNS engine and the snapshot, so plans 2 and 4 add them.
- Checking that `dns.upstreamGroup` names an existing upstream group: upstream groups arrive with the DNS engine in plan 2.
- `modesAllowed` is parsed and kept on `Rule::modes` but not enforced, and `explain` does not take `protocol`, `port` or `mode` yet (spec §8.4): the connection mode only exists once the DNS engine and gateway do (plans 2 and 3).
- Capability and route-availability checks (spec §8.2, step 6): node health arrives with the gateway and agent work.
