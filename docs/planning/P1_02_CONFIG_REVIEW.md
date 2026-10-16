# P1-02 configuration boundary review

Date: 2026-10-17. Status: **implemented locally; independent review and CI pending**.
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
warning-free all-feature Clippy and locked offline cargo-deny passed. Fourteen new
configuration integration tests cover independent JSON/YAML/TOML payloads and a
complete synthetic JSON fixture, valid/invalid size and depth boundaries, aggregate
resource limits, unknown/duplicate input, error-canary redaction, separate scoped
grants, references, lifecycle observations, port ranges and listener conflicts.
Seven domain tests, two layer tests and four compile-fail examples also pass.
A valid empty draft was first observed failing before implementation. A later
diagnostic-path assertion exposed Serde's tagged-enum path limitation; the test
now explicitly expects the safe enclosing container, with no raw-message parsing.

Local LCOV and raw logs remain ignored under `.setup/p1-02/`. Current coverage is
recorded there per source file; service-side patch coverage and CodeScene have not
run for this uncommitted change. Linux CI, current-head independent Code Review
and final merge evidence remain required. No native socket/runtime, systemd,
credential-verification or product interoperability success is claimed.

The diagnostic precision disposition is recorded in
[OPEN_DECISIONS](OPEN_DECISIONS.md#p1-02-review-dispositions). P1-03 has not started;
PR #2 remains a reference-only donor with no imported code.
