# P3-06 SmartDNS binding profile — proposal

Status: **profile accepted; implementation in progress**. On 2026-11-03 the owner accepted
this profile and selected current-generation checks without old-generation grace.
P3-05 and P4-01 are complete. This proposal does not authorize deployment or a
new dependency. The task remains open until its acceptance evidence is available.

## Existing boundary

A Gateway DNS action currently returns SERVFAIL. Before P3-06, signed configuration
carried independent DNS policy and gateway grants but no explicit DNS-to-gateway
route binding. P4-01 provides current source,
lease, target, port and traffic-policy checks for gateway admission; it does not
implement an origin resolver, dialer or TCP listener. DNS identity and TCP identity
are separate. The DNS peer can differ from the later TCP peer because of resolvers,
NAT or network topology, so a DNS query cannot prove the later peer's authorization.

## Proposed behavior

- Publish explicit operator-configured gateway addresses and compatible domain,
  TCP port, gateway listener and outbound bindings as part of the authenticated
  immutable generation. Validate references, conflicts and address families before
  publication. A bind address, DNS answer, SNI, or DNS endpoint alone is not a grant.
- Publish a gateway address only when the selected DNS action and an unambiguous
  compatible binding agree on domain, outbound and generation. Require a configured
  gateway grant and matching traffic policy for the binding, but retain separate
  current source, lease and target admission for every subsequent TCP connection.
  No DNS operation creates, renews or chooses a gateway grant or traffic Profile.
- Return only explicitly configured A or AAAA addresses of the requested family.
  A missing family, unknown or ambiguous binding, expired/removed authorization or
  unavailable gateway returns SERVFAIL rather than an ordinary-origin fallback.
  HTTPS/SVCB redirects remain unsupported, as selected in P3-05. Other query types
  under a Gateway action also return SERVFAIL. Redirect answers are unsigned and
  do not assert AD. This does not control ECH, QUIC, cached alternatives or all IPv6
  traffic; validating clients may reject unsigned local answers.
- Use a bounded operator-configured TTL, proposed maximum 60 seconds, and recheck
  current generation and DNS admission at output. Cache lifetime is not authority:
  gateway admission independently checks its current generation and lease. After
  removal, expiry or revocation, a new connection using an old DNS answer is denied.
  A changed but still valid mapping is evaluated under the new generation; no old
  admission or revoked access is revived. An old answer can therefore cause an
  outage until client caches expire. No old-generation grace admits a new connection, including for changed but
  non-revoked mappings.
- The complete DNS and gateway route views must derive from one generation. Reject
  mismatched or partially published state; recovery follows the existing durable
  activation and clock/lease floors. Recheck relevant current facts at consumption.

Exact schema, listener/address reachability, lease reconciliation, consistency of
independently selected TCP policy, safe activation and race linearization require a
reviewed implementation design; static configuration presence alone cannot prove
that a listener is operating or that a resolved destination is safe.

## Required evidence

Exercise real DNS transport for A/AAAA and failure paths, family mismatch, query
kind, unsigned answers and policy isolation; reject ambiguous, mismatched and
partially published bindings. Test concurrent publication, an old cached answer
across replacement, removal and expiry, and ensure separate gateway admission
rejects revoked or unknown source/target without prior-DNS authority. Exercise
snapshot recovery and current-generation/lease checks at output and admission.

The backlog also requires a DNS-then-TLS test. P4-02 origin validation, P4-03 ClientHello parsing and P4-04
TCP passthrough are not implemented by P4-01 or this proposal. A unit test of the
admission API or a mock TLS endpoint cannot establish real gateway passthrough,
origin certificate preservation or checked/pinned origin dialing. Until a real
composed path is available and tested, record that acceptance criterion as unverified
and keep P3-06 open; do not describe a DNS-only result as complete.

## Partial implementation and remaining work

The signed v2 configuration now accepts up to 256 exact-name route declarations,
with one address per family and a TTL from 1 to 60 seconds. Empty route tables
are omitted so existing signed configurations retain their canonical bytes.
Publication checks the endpoint, grant, listener, outbound, port, canonical
name uniqueness and both compiled policy planes in the same generation, including
service-derived grant scope. These checks are static, not proof that a lease is
current or a gateway is running. Older readers reject snapshots containing the
new field; they cannot silently interpret it as a working redirect.

The core now checks DNS and gateway leases against one conservative clock sample
under the same current-generation lock. A separate SmartDNS gateway admission
checks its exact current route in addition to the existing independent ingress,
target and outbound checks. These APIs do not implement gateway readiness or
network output. Gateway transports must use the SmartDNS-specific admission gate.

DNS Gateway actions still return SERVFAIL. Lease reconciliation at output,
actual gateway readiness and transport integration, old-answer/revocation races,
and the real DNS-to-TLS path have not been implemented or verified. P4-02 has
partial origin resolution and an opaque direct connector; P4-03 is qualified only
in an isolated test workspace, and P4-04 forwarding remains unavailable. The
fail-closed listener does not establish gateway readiness.

Local partial evidence (2026-11-03): configuration tests passed under default and
no-default features with local networking permitted. The new cross-plane test
failed when the compilation check was intentionally bypassed and passed after
restoration. Workspace Clippy, formatting, offline cargo-deny, planning self-test
and 21 planning-tool tests passed. Two workspace-test attempts failed at 19 durable
core fixtures with `UnsafePath`: in this sandbox `/` is owned by neither root nor
the process user. A private TMPDIR did not change that ancestor check. These
failed runs are not workspace passes. No current-head external review, CI,
Codecov, CodeScene, native acceptance or real gateway/TLS result is claimed.

Additional local evidence: 20 in-memory admission tests pass in both default and
no-default profiles. New tests cover expiry of the gateway lease while DNS stays
valid, removed and changed routes after an old answer, independent source checks,
and retirement of previously prepared attempts. Replacing the gateway lease in
the two-lease check with the DNS lease makes the expiry assertion fail. Workspace
Clippy and the planning checks pass. These are core-level results, not evidence of
a running gateway, actual DNS output or TLS passthrough.

On 2026-11-03, after the sandboxed `UnsafePath` failures above, complete offline
workspace tests passed outside that sandbox in both default and no-default profiles,
including actual IPv4/IPv6 DNS tests and the protected-path fixtures. The earlier
failures remain failures of those sandboxed runs. This does not replace the required
SmartDNS-to-TLS integration, review, or exact-head CI evidence.
