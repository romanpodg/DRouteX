# P3-05 DNSSEC and HTTPS/SVCB: implementation evidence

On 2026-11-02 the owner requested P3-05 and selected bounded transit of DO/CD
and signed data without local DNSSEC validation. Local policy answers remain
unsigned and may be rejected by a validating client. Root DNSKEY uses the
admitted Profile's default action; other root questions remain unavailable.
Ordinary HTTPS/SVCB data retains its endpoint and parameters. SmartDNS redirects
with HTTPS/SVCB are unsupported pending P3-06; no ECH removal or QUIC control is
claimed. These choices do not authorize deployment or a new dependency profile.

The implementation forwards DO/CD, clears upstream AD/AA, and keeps
DNSSEC-mode queries out of the D53 non-DNSSEC cache. The existing plaintext
upstream is not authenticated. Response and transport limits still apply.
The current codec treats DNSSEC records as opaque data; this is not proof of
complete preservation or of independent client validation. The Ed25519 A RRset
fixture in `tests/dns-fixtures/dnssec-ed25519.json` was verified with OpenSSL;
the independently constructed canonical input verifies and a modified address
fails verification. The forwarding regression checks that the signed A and
RRSIG data survive transit byte-for-byte. An independent dnspython client queried
the actual test node over TCP on IPv4 and IPv6, validated the returned RRSIG
against the fixture's explicitly trusted DNSKEY, rejected a changed address,
and observed an unsigned Profile-local override without AD or RRSIG. The client
also retrieved and verified a separate synthetic root DNSKEY, two signed DS and
DNSKEY delegations, and the signed A answer through the node. Both Rust profiles
passed. This controlled chain uses a synthetic trust anchor, not the public root.

Focused tests exercised upstream DO/CD, a signed RRSIG, cache bypass, root DNSKEY
and Profile BLOCK, unsigned local answers, and HTTPS/SVCB parameters over IPv4
and IPv6 loopback. Workspace tests passed in default and no-default profiles with
actual filesystem permissions. Workspace Clippy, offline cargo-deny, formatting,
planning self-test and planning-tool tests passed. The earlier sandboxed package
run had 108 passes and one `UnsafePath` failure; that failed attempt remains failed.
The current unavailable gateway action returns SERVFAIL for A, AAAA, SVCB and HTTPS
on both IP families without an upstream fallback. This checks the unsupported
boundary; it does not establish an IPv6 bypass matrix for a future redirect.

The owner authorized an isolated test validator on 2026-11-02. The Linux CPython
3.12 x86_64 tool profile is pinned and wheel-hashed in
`tests/dns-fixtures/requirements-linux.txt`: dnspython 2.8.0 (ISC),
cryptography 46.0.5 (Apache-2.0 OR BSD-3-Clause), cffi 2.0.0 (MIT), and
pycparser 3.12 (BSD-3-Clause). Wheels were obtained from PyPI over the configured
verified proxy, installed into a disposable environment with `--require-hashes`,
and `pip check` passed. cffi 2.1.1 was inspected but not installed because its
MIT-0 license is outside the approved list. This is a test tool only; runtime
Cargo dependencies and features are unchanged. The Linux qualification workflow
now runs the client explicitly; ordinary workspace runs without its environment
variable exercise the wire regression but not the independent client.

The IPv6 bypass matrix for an actual supported redirect belongs to its later
integration. P3-05 does not establish gateway, public-root or deployed-service
acceptance. P3-06 remains separately gated.

PR [#124](https://github.com/romanpodg/DRouteX/pull/124) implementation head
`bc687a42fddde04db2fbcdeb536bdbfa6e010f0a` received completed Codex Code Review
with no findings. [CodeScene 8236255](https://codescene.io/projects/86145/delta/results/8236255)
failed on the auxiliary planning validator; its scoped disposition is in
[OPEN_DECISIONS](OPEN_DECISIONS.md#p3-05-review-dispositions). GitHub reported
that check as not required. New heads need their own review and checks.

PR124 merged normally as `3f1fa33f4bceae063e053e3c09d4be92306487e8` after the
owner explicitly authorized the merge. Codex Code Review completed without
findings on final head `7f78a6e02e527a974797626b7b8187920a3f191c`.
PR-head Foundation run [41423277587](https://github.com/romanpodg/DRouteX/actions/runs/41423277587)
passed all 11 jobs and both Codecov patch checks; the Codecov exact-head report
was complete with two sessions. Final-head
[CodeScene 8236343](https://codescene.io/projects/86145/delta/results/8236343)
remained failed with the same recorded non-required planning-tool finding.
Exact-main Foundation run [41428157616](https://github.com/romanpodg/DRouteX/actions/runs/41428157616)
passed all 11 jobs on the exact merge commit. Codecov reported the commit
complete and both planning and backend uploads `merged`. The requested separate
security review has not produced a result. The PR records a security and architecture
assessment; no separate successful security review or deployment is claimed.
