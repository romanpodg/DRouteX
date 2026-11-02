# P3-05 DNSSEC and HTTPS/SVCB: work in progress

On 2026-11-02 the owner requested P3-05 and selected bounded transit of DO/CD
and signed data without local DNSSEC validation. Local policy answers remain
unsigned and may be rejected by a validating client. Root DNSKEY uses the
admitted Profile's default action; other root questions remain unavailable.
Ordinary HTTPS/SVCB data retains its endpoint and parameters. SmartDNS redirects
with HTTPS/SVCB are unsupported pending P3-06; no ECH removal or QUIC control is
claimed. These choices do not authorize deployment or a new dependency profile.

The implementation in progress forwards DO/CD, clears upstream AD/AA, and keeps
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

These are interim results, not task acceptance. The IPv6 bypass matrix for a
supported redirect belongs to its later integration; external current-head review,
CI, and security/architecture review remain required. Do not mark P3-05 complete
or start P3-06 on this evidence.
