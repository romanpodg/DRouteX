# Experimental TLS gateway focused security review

Reviewed on 2026-11-07. Runtime source baseline:
`885de5000680d389e3a8e260d1e8b25c502bff7d` (PR127). The follow-up adds only
production-wrapper regressions and review evidence; runtime behavior is unchanged.

## Result and scope

No confirmed exploitable defect, authorization bypass, unchecked origin dial or
silent DIRECT fallback was found in the inspected paths. This is a focused local
source review with synthetic regressions, not a penetration-test certification,
an independent external audit of tls-parser, or production acceptance. The
experimental features remain disabled by default. P3-06/P4-03 stay open.

The review used separate parser/standards and ingress/contract passes plus an
origin-resolution/resource-ownership pass. Hypotheses were checked against the
accepted contracts rather than treated as vulnerabilities solely from a metric
or an undocumented stronger guarantee. No third-party system was attacked.

Reviewed sources include:

- [TLS metadata wrapper](../../crates/droutex-gateway/src/tls_metadata.rs) and
  the locally resolved tls-parser 0.12.2 record, ClientHello and SNI parsers.
- [Preview listener](../../crates/droutex-gateway/src/listener/preview.rs),
  [core SmartDNS admission](../../crates/droutex-core/src/deployment/admission/smartdns.rs)
  and their current-authority callers.
- [Origin resolution and numeric connection](../../crates/droutex-gateway/src/origin/resolver.rs),
  [DNS response interpretation](../../crates/droutex-gateway/src/origin/response.rs),
  [address screening](../../crates/droutex-gateway/src/origin.rs), inventory and
  its owned worker registry.
- [Opaque relay](../../crates/droutex-gateway/src/origin/resolver/relay.rs).

## Control assessment

| Boundary | Source and exercised evidence | Limit |
|---|---|---|
| Untrusted ClientHello | Exact body/SNI/extension consumption, duplicate extension and ECH/ESNI rejection, canonical hostname handling; prefix, record-fragment and malformed-input regressions pass. The selected dependency parsers enforce session-ID/cipher-vector bounds. | This extracts initial metadata; it does not implement a complete TLS state machine. |
| Parser resource limits | 16 KiB input, 16 records, 128 extensions and 256 reads; listener supplies a five-second hello deadline and 64-session cap. New tests exercise exact wire/record/extension limits and one-over-limit refusal. | Source bounds and a finite corpus are not an exhaustive CPU/memory/fuzzing proof. |
| Trusted ingress | Actual peer and configured bound listener enter a separate reservation; SNI selects a target only within its signed gateway grant. Consumption rechecks generation, clock and leases. DNS Profile/headers are not gateway authority. | Shared source-IP authority remains a group grant, not device identity. |
| DNS readiness | Owner, generation, listener/port, family and inventory availability supplement both live leases. Task exit/shutdown or new generation invalidates readiness. Existing SmartDNS tests cover these transitions. | Readiness is not external NAT/public reachability proof. |
| Origin selection | Independent signed upstream policy; bounded DNS messages, complete question/frame checks and CNAME history; reachable numeric answers are screened against private/special/self addresses and fresh OS inventory. The connector consumes the checked resolution and dials the numeric address. | Plaintext upstream DNS is not authenticated against an on-path attacker; trusted host aliases must include externally mapped self addresses. |
| Relay | Initial observed name, actual peer/listener and current route are checked. Admission/deadline checks surround every I/O poll. Shared idle tracking, lifetime/byte/operation limits and owned connection slots remain in effect. Revocation/dial/relay and half-close regressions pass. | Previously admitted bounded work can complete; this is not an atomic barrier that retracts bytes already submitted before revocation. |
| Cancellation and shutdown | Session JoinSets, bounded listener draining and inventory registry retain task/worker ownership. Cancellation, quotas, forced abort and cleanup tests pass. | Native OS/process shutdown timing is not exhaustively stressed by these tests. |

## New regression evidence

The production wrapper now has direct tests for:

1. Exactly 128 extensions versus 129, 16 TLS records versus 17, and 16 KiB
   input versus one additional byte.
2. A truncated extra SNI entry inside a correctly declared list and unconsumed
   data inside a correctly declared ClientHello body. These cover known
   permissive raw-library behavior at the actual application wrapper.
3. Two deterministic byte replacements at every offset of the public
   ClientHello fixture, without panic. This does not assert that every mutation
   must be rejected: some mutations are valid TLS input.

Removing the ClientHello-body consumption check makes the new rejection test
fail behaviorally; independently removing SNI-list consumption also makes it
fail. Both mutations compiled, failed their expected assertion and were restored.
No check, warning policy or production resource limit was weakened.

## Verification

Windows 10.0.26300.0, pinned Rust 1.99.0. Project activation precedes Cargo.

| Command | Result |
|---|---|
| `cargo fmt --all -- --check` | PASS |
| `cargo clippy -p droutex-gateway -p droutex-dns --all-targets --all-features --locked -- -D warnings` | PASS |
| `cargo test -p droutex-gateway --all-features --locked` | 43 tests and one doctest pass after adding the three regressions; no ignored tests. |
| `cargo test --manifest-path tests/tls-parser-qualification/Cargo.toml --locked` | 16 tests pass; no ignored tests. Raw-parser permissive behavior is deliberately asserted in this qualifier. |
| `cargo deny --locked check` | PASS: advisories, bans, licenses, sources. Existing transitive duplicate-version warnings remain; no dependency/feature/license changes were made for this review. |

RustSec database revision used:
`309392b18f6b17022f82c701f300433126901068` (2026-11-06). This is the database
observed during the 2026-11-07 check, not a guarantee that no advisory is missing.
Duplicate warnings remain a maintenance item of the existing qualified profiles;
revisit during their next approved dependency update, not via an unqualified
security-review upgrade.

Public fixture SHA-256:
`866cd405919d002ac751dacf02b75d3f317f53a2ff2734b872b658665692ec33`.
Current-head hosted checks and account review must be associated separately with
the follow-up commit; older successful runs do not validate new tests/docs.

## Residual observations and follow-up

- `read_hello` checks elapsed time immediately after reading, before synchronous
  inspection. There is no additional post-inspection timestamp check. Completion
  could cross the deadline by one bounded parse interval; no material exploit
  was reproduced. Revisit if strict post-parse completion timing is required.
- DNS readiness checks listener-port/route-port equality. Direct TCP admission
  selects a unique signed route port without separately requiring that equality.
  The target port is still authorized by the source/listener grant; no grant
  expansion was found. Clarify the intended consistency rule before extending
  the experimental host API or claiming port-translation support.
- Later TLS records/ClientHellos, HelloRetryRequest and renegotiation are opaque after the
  initial observation. They do not replace the gateway's pinned numeric origin.
  Shared-IP virtual hosting/domain fronting and origin HTTP authority are outside
  this gateway's encrypted-payload enforcement capability. Exhaustive TLS-stack
  differential testing was not performed.
- Independent external parser security review, broader fuzzing/concurrency/load
  evidence, native Debian and IPv6 acceptance, public ingress and protected
  outbound proof remain separate gates. The owner's native Linux follow-up is
  still deferred. No vulnerability disclosure or production deployment is
  authorized by this report.

Any subsequently confirmed vulnerability follows the private reporting route in
[SECURITY.md](../../SECURITY.md), not a public PR comment with exploit details.
