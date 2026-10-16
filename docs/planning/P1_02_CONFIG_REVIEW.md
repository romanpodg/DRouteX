# P1-02 configuration boundary review

Date: 2026-10-17. Status: **validated; PR #5 final-head review/checks and merge gates apply**.
Authority: accepted [CR-0002](../architecture/DNS_PRODUCT_MODEL.md),
[P0-01 contracts](P0_01_FOUNDATION_REVIEW.md), owner answers recorded in D24/D25
of [OPEN_DECISIONS](OPEN_DECISIONS.md). PR #4 merged first at
`cd2a9c011e4950d80458cfaa72baddd1ac4243c2`; P1-02 starts from that verified main.

## Dependency decision and qualification

The owner approved qualification of Serde 1.0.229, serde_json 1.0.154,
toml 1.1.8+spec-1.1.0 and serde-saphyr 2.0.0, then approved qualification and
adoption of serde_path_to_error 0.1.20 if successful. Official registry archives
match their recorded SHA-256 values and contain approved license texts. The
isolated 38-artifact lock passed all four cargo-deny categories, eight Windows
probes and warning-free Clippy before production use. Probes cover independently
written formats, duplicate/unknown rejection, bounded YAML and sanitized paths.
Raw errors from all three parsers echo a synthetic canary, so Routira discards
their text and source chains rather than trying to scrub arbitrary diagnostics.

Only the config crate adopts these five dependencies. TOML enables std/parse/serde;
YAML enables deserialize only, without include, properties or serialization.
The current production lock has 51 registry artifacts; the
[config inventory](../operations/CONFIG_INVENTORY.csv) verifies archive hashes
and top-level notice hashes for all 51. The locked production cargo-deny check
passes advisories, bans, licenses and sources. The unused ISC allowance warning
is not a failure or exception. Preserve complete notices at distribution time
under P11-05; the CSV is evidence, not a release notice bundle. The P0-05 network
stack remains isolated from production crates.

## Implementation and security scope

The [wire contract](../../api/schemas/README.md) and typed DTOs define the accepted
single-document UTF-8 profile: 1 MiB, depth 32, aggregate 10,000 resources/rules,
strict YAML and unknown/duplicate rejection. A bounded intermediate value makes
all three parsers feed the same model, while errors retain only known schema
paths, indices and available numeric positions. No secret material is accepted;
the optional outbound secret is a typed reference to caller-known storage.
Common adds distinct `DnsUpstreamId` and `SecretRefId` without importing Serde.

Validation rejects cross-Endpoint credentials, wrong listener purposes, ambiguous
legacy/gateway group selection, missing revisions, duplicate IDs, invalid ports,
DNS/traffic action confusion and future engine modes. Listener collision detection
uses ordered maps/sets rather than an all-pairs scan. Reusable Profiles and private
records stay scoped; DNS gateway intent cannot mint a TCP authorization grant.
Trusted reference existence and imported lifecycle states are not authentication.

An empty draft or disabled resource can be staged without claiming availability.
The private `CheckedConfiguration` wrapper exposes read-only imported data and is
not compiled policy, snapshot or admission proof. Domain canonicalization/conflict
checks, dataset contents/hashes, secret lifetime, DNS resolution and live network
authorization remain at their explicit P2/P3/P4 gates. No listener is opened here.

## Actual validation

Windows Rust 1.99.0: fmt, workspace/all-target check, no-default tests, full tests,
warning-free all-feature Clippy and locked offline cargo-deny passed. Eighteen new
configuration integration tests cover independent JSON/YAML/TOML payloads and a
complete synthetic JSON fixture, valid/invalid size and depth boundaries, aggregate
resource limits, unknown/duplicate input, error-canary redaction, separate scoped
grants, references, lifecycle observations, port ranges and listener conflicts.
Seven domain tests, two layer tests and four compile-fail examples also pass.
A valid empty draft was first observed failing before implementation. A later
diagnostic-path assertion exposed Serde's tagged-enum path limitation; the test
now explicitly expects the safe enclosing container, with no raw-message parsing.

Local LCOV and raw logs remain ignored under `.setup/p1-02/`. After review fixes,
the new schema has 761/815 covered executable lines and the workspace 837/894.
These local numbers are distinct from service-side coverage.

[PR #5](https://github.com/romanpodg/Routira/pull/5) initial head
`edbcb8fa1cdf1badbf7c6263646fc46215e3474d` passed all seven jobs of
[run 38954296579](https://github.com/romanpodg/Routira/actions/runs/38954296579),
including Linux/Windows workspace, fresh locked dependency checks and coverage.
The completed independent read-only built-in Code Review against verified base
`cd2a9c011e4950d80458cfaa72baddd1ac4243c2` identified three defects:

- Serde's internally tagged unit variants ignored unknown fields. Empty struct
  variants now enforce rejection without changing the wire representation.
- The intermediate Value deserializer accepted positional arrays as structs.
  The recursive boundary now requires objects for structs and reuses Serde's
  map/sequence adapters, without a second schema-field whitelist or new dependency.
- Semantic rule errors omitted the inner action payload segment. They now report
  the actual `action.action.upstream` or `action.action.outbound` field.

Each of the three new regression tests failed before its fix; all config tests
and full workspace/default/no-default/Clippy/rustdoc checks passed afterward.
Initial review log: `.setup/p1-02/codex-review.log` (exit 0 with findings, not approval).
The reviewer ran fmt and an existing test binary; read-only restrictions prevented
rebuilding/Clippy. The separate local/CI runs above provide execution evidence.
The first corrected head `3a57a1a7c551521a60a70b0732406fa362dcc974` passed
all seven jobs of [run 38955541778](https://github.com/romanpodg/Routira/actions/runs/38955541778).
Codecov processed both flags and the exact base/head comparison: backend 817/874,
Rust patch 741/795 with a successful backend patch check (93.20% displayed).
Aggregate patch 746/800 includes five auxiliary Python lines and is not product
coverage. CodeScene 7991752 repeated the maintenance signals below.

Its independent follow-up review found the analogous tagged-enum sequence path:
Serde uses `deserialize_any`, bypassing struct-only shape checks. Map-only visitors
now guard every tagged action/transport/record field, each DNS access item and the
nested rule action payload. The fourth regression failed before this correction;
all 18 configuration tests and full local checks pass afterward. Follow-up review
log: `.setup/p1-02/codex-review-fixes.log` (exit 0 with a finding).
Implementation head `a708e34b27a82fe5b4682b1761b34569144c94c7` subsequently passed
full independent built-in Code Review against the same verified base with no
confirmed defect. Log: `.setup/p1-02/codex-review-tagged.log`, exit 0. The reviewer
ran fmt/diff checks and 27 tests from existing binaries; read-only build-lock
restrictions prevented fresh Cargo compilation/Clippy in that review. Separate
local checks and all seven jobs of
[run 38956580440](https://github.com/romanpodg/Routira/actions/runs/38956580440)
passed, including default/no-default Linux/Windows and fresh dependency auditing.
No review finding remains unresolved or is deferred.

Codecov processed backend/planning reports for that exact head and comparison
with base `cd2a9c011e4950d80458cfaa72baddd1ac4243c2`: backend 837/894 (93.62%),
new Rust 761/815, successful backend patch check (93.37% displayed) and successful
100% planning patch check. Aggregate patch 766/820 includes five auxiliary Python
lines; aggregate total 1271/1331 likewise is not application coverage.
The [PR report](https://app.codecov.io/gh/romanpodg/Routira/pull/5) is real service
evidence, distinct from local LCOV. The actual fork test remains owner-deferred.

[CodeScene 7991651](https://codescene.io/projects/86145/delta/results/7991651)
failed its new-code gate and passed two other gates. Complex collision predicates
in binding.rs scored 8.41; explicit helper argument counts in access.rs/rules.rs
and the test helper scored 9.69. Source/edge-case tests were inspected. No concrete
functional defect follows from these scores; no suppression or score-only rewrite
was made. The separate findings above were fixed. Maintenance dispositions are in
the existing register. Final merge evidence remains required. No native socket/runtime, systemd,
credential-verification or product interoperability success is claimed.

[CodeScene 7991872](https://codescene.io/projects/86145/delta/results/7991872)
on `a708e34` repeated the same failed new-code gate and two passed gates, without
new functional findings. The check remains failed, not waived by an invented pass.
The independent functional review and explicit maintenance dispositions support
retaining these conditions/parameters. Normal merge still requires the final
documentation head's applicable checks and review; no protection is bypassed.

The diagnostic precision disposition is recorded in
[OPEN_DECISIONS](OPEN_DECISIONS.md#p1-02-review-dispositions). P1-03 has not started;
PR #2 remains a reference-only donor with no imported code.
