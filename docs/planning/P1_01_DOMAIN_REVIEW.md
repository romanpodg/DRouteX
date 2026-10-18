# P1-01 domain identity and state review

Date: 2026-10-16. Status: **validated; PR merge gates apply**. Authority: accepted
[CR-0002](../architecture/DNS_PRODUCT_MODEL.md) and
[P0-01 contracts](P0_01_FOUNDATION_REVIEW.md). This task defines pure domain data,
not a running resolver, credential verifier, gateway or storage schema.

## Owner decision and dependency evidence

After PR #3 merged as `a77ed5adce184ffe3214452af0b1e6d83db2cd92`, the owner selected
UUIDv4 public resource IDs and approved qualification of uuid 1.28.0. Its official
crate archive SHA-256 is
`7cc1186384beb7dd8eedea376413fd654937285ea6c9cfbb928dc3043ea4b606`, matching
crates.io metadata. It is not yanked, declares Rust 1.89.0, and includes both
Apache-2.0 and MIT texts. It was built/tested with the approved Rust 1.99.0.
The exact [UUID inventory](../operations/UUID_INVENTORY.csv) records the 19 locked
registry artifacts; the production [Cargo.lock](../../Cargo.lock) is authoritative.
Only uuid is in the Windows/Linux runtime tree; other locked entries are target/
optional support and do not authorize another platform.

The std parse/format profile passed two independent vector/invalid-input probe
tests and all four cargo-deny categories. Unused license allowances were warnings,
not failures or new license exceptions. No v4 RNG feature, secret generator,
serde, time-ordered UUID, custom crypto or new native build tool was introduced.
Actual ID creation belongs to resource-creation boundaries; it must handle entropy
failure explicitly when added. Public UUIDs carry no access authority.

## Implemented model boundaries

- Distinct UUIDv4 newtypes for Profiles, public Endpoints, credential records,
  Device metadata, list/service revisions, private records, rules, outbounds,
  listeners/groups, independent DNS/gateway grants, snapshots and deployments.
  Private fields reject nil, non-v4 and non-RFC-variant identifiers. Public APIs
  expose standard bytes/text and DRouteX errors, not uuid dependency types.
- ResolverCredentialMetadata contains only public record ID, Endpoint ID and
  lifecycle observation. ResolverCapabilityMaterial is a separate owned value
  with redacted Debug and no Clone/Copy/Display/serialization implementation.
  Its constructor only rejects empty already-owned bytes; it is **not** an external
  parser or proof of entropy. Format, size bounds, secure generation, verifier,
  encryption/export/rotation and erasure remain P1-07/P3-07 prerequisites.
- Each DnsEndpointAssignment has one Profile and optional Device metadata. Its
  credential-association helper rejects wrong Endpoint and disabled/expired/revoked
  observations. It requires independent capability verification and current
  clock/revocation/generation checks at the caller. DnsProfileBinding is policy
  association data, never an admission permit or transparent TCP identity.
- Legacy DNS assignments use exact IpAddr plus ListenerId and a shared group.
  Multiple matches fail with a typed ambiguity error, independent of order;
  unknown peers/listeners have no assignment. There is no prefix expansion,
  last-query Profile switch or permission union. This static association check
  does not implement P4-01's live expiry/source/destination authorization.
- GatewayNetworkIdentity requires its own gateway grant/group/listener/source.
  It has no device or resolver-credential field and no conversion from DNS scope.
- Administrative, credential, outbound, revision and snapshot observations use
  enums. Configuration intent, observed availability, published revision and
  applied snapshot remain different facts. These enums do not implement the
  authenticated publication state machine or accept proposed ADRs wholesale.

## Test evidence and limits

Windows fmt/check/Clippy passed. Seven integration tests cover UUID vectors and
invalid variants/versions, redaction/errors, copied bearer material with the same
credential association, unavailable credential/Endpoint states, separate secure
Endpoint Profiles, and exact/ambiguous legacy mappings. Four compile-fail examples
reject Profile/Endpoint mixing, DNS-to-gateway identity conversion, implicit secret
cloning and secret Display. Existing layer-order tests passed.
All seven integration tests were also observed failing with deliberately wrong
expectations in an isolated ignored source copy, then restored. The compile-fail
diagnostics specifically report type mismatches and missing Clone/Display traits.
Local LCOV covers 72/72 instrumented lines in the new domain files and 77/80 in the
workspace; enums without executable bodies do not contribute covered lines. This
is local test evidence, not a service-side or runtime security guarantee.

The copied-material case demonstrates the **domain association after independent
verification**, not a new bearer verifier or real DoH/DoT authentication. A copied
bearer has no new hardware identity. Real unknown/revoked capability lookup,
timed expiry, source observations and network denial must still pass P3-07/P4-01.
Neither the redaction wrapper nor metadata association authorizes listener exposure.
No storage at rest, API serialization, production capability parsing or systemd/
native Debian runtime acceptance is claimed.

## Independent review and service evidence

[PR #4](https://github.com/romanpodg/DRouteX/pull/4) implementation head
`4d79b7a95f094851bd2b73c4c924a85c7231de44` was independently reviewed against
`a77ed5adce184ffe3214452af0b1e6d83db2cd92` with the built-in Codex review through
the signed-in ChatGPT account, read-only sandbox and approval policy never.
The completed review found no confirmed defect. Its read-only environment could
not execute Rust compilation/tests or Python tests needing PyYAML; this limitation
does not stand in for test success. Separate Windows evidence above and all seven
jobs of [CI run 38911280732](https://github.com/romanpodg/DRouteX/actions/runs/38911280732)
passed, including Linux/Windows workspace tests, Linux dependency qualification,
both locked audits, coverage generation and OIDC uploads. Raw logs stay local.

[CodeScene review 7987296](https://codescene.io/projects/86145/delta/results/7987296)
passed all three quality gates. Its expected-document-change suggestions identify
historical cochanges, not functional defects or additional implementation scope.
[Codecov's processed comparison](https://app.codecov.io/gh/romanpodg/DRouteX/pull/4)
uses the exact base/head above. All 71 service-counted new Rust lines are covered
(identity 37/37, IDs 20/20, secret material 14/14); the backend patch check passed
at 100%. Backend totals are 76/79 (96.20%), including the existing config 5/8.
Linux/service and Windows instrumentation count one identity line differently;
the earlier local 72/72 and 77/80 are not substituted for service numbers.
The aggregate patch 76/76 also includes five auxiliary planning lines and must not
be described as 76 new application lines. Both planning/backend uploads processed.

No review defect or minor implementation deferral was identified. Actual resolver
authentication/network denial remains at the gates above, and the real fork test
remains owner-deferred. Final evidence edits require inspection/review and successful
current-head checks before the authorized ordinary merge.
