# P4-02 origin safety: work in progress

The owner authorized autonomous project development decisions, including P4-02
through P4-04 while P3-06 remains open. This work is not deployment or final
acceptance.

The optional signed `originUpstream` points to a same-generation upstream
resource, independently of DNS Profile selection. A missing reference grants no
resolver or fallback, and compilation rejects missing pinned resources. The
resolver takes its domain and port from an existing gateway admission, uses only
that signed upstream, and keeps the resulting numeric addresses borrowed from
the original authorization. It rechecks the admission during network operations
and before returning. A returned result is not permission to connect; the
direct connector rechecks admission and fresh self inventory immediately before
numeric-only dialing.

The resolver bounds concurrent requests, total and per-member time, packet
counts, TCP I/O operations and frame sizes. It uses connected UDP sockets and
random transaction IDs supplied by the pinned DNS codec. A fully validated
truncated UDP reply triggers a bounded TCP retry to the same numeric upstream;
TCP-only members also use framed TCP. A fallback member is tried only when the
signed upstream policy permits it. CNAME followups share the bounded alias
history. Plain DNS does not authenticate answers against an on-path attacker.

The interpreter checks complete frames, response type, ID, exact question,
class, bounded records, canonical names, conflicting CNAMEs, loops, additional
records and EDNS framing. It never uses additional glue as an origin answer.
Some otherwise valid responses can be refused, including zero-length
NULL/unknown RDATA affected by the existing DNS codec compatibility limit.
There is no negative cache. All returned addresses are screened together against
unsafe ranges, configured aliases and listener addresses; mixed unsafe answers
are denied. Fresh bounded OS self-address probes check upstreams and returned
candidates. The host is responsible for supplying all public/NAT aliases not
visible to the OS. Probe worker slots remain held until workers exit.

Local network tests cover UDP CNAME resolution, private rebinding rejection,
revocation during a pending query, result revocation, and UDP truncation followed
by real framed TCP. Component tests cover DNS framing, EDNS, CNAME limits,
self-address inventory and malformed or unsafe answers. Earlier mutations of
ID checks, EDNS validation, upstream pin validation and self matching caused
corresponding regression tests to fail. The loopback DNS fixture is test-only;
production upstream and origin self checks are not bypassed.

An initial direct connector accepts only an address index from the original
resolution and only an explicitly selected `GatewayDirect` intent. Every attempt
rechecks live admission, public/self address screening and fresh OS inventory,
and connects the numeric socket address under the original resolution deadline.
An attempt consumes the non-cloneable result; retries require fresh DNS resolution.
The returned socket is opaque and keeps its admission and process-wide connection
slot until dropped; it has no data-transfer API. Protected proxy intents fail
closed rather than switching to direct. Tests cover refused self/invalid targets,
expired DNS results, expired or retired admission, and protected-proxy refusal.
Disabling the I/O-poll admission check or the original resolution deadline made
the corresponding regression fail. `verify_current` now checks the original
resolution deadline in addition to admission; restoring its earlier behavior made
the stale-result regression fail.

A controlled local integration test runs real DNS decoding, admission, address
screening and OS inventory before exercising the direct dial gate. A private
test-only transport maps the selected public numeric address to a loopback server,
asserting the exact address passed to the dial operation; it does not prove that
the operating system connected to that public address. The production entry point
uses `TcpStream::connect` on the checked numeric target, with no public transport
injection. A mutation replacing that target makes the test fail. A second local
test revokes the generation while the dial holds an established socket and checks
that the socket is closed without being returned. Direct connectivity to a real
external origin and platform-specific behavior remain unverified. Cancellation of an individual inventory probe signals its worker while the
resolver-owned registry retains it. The inventory shutdown operation rejects new
probes, signals all tracked workers and joins them; a cancelled shutdown can be
retried. The future gateway runtime must retain the resolver and explicitly call
this operation. It still needs to stop DNS I/O and close established sockets as
part of a complete gateway shutdown.

P4-02 and P3-06 remain in progress. Production TLS parsing,
data transfer, complete gateway shutdown and end-to-end
SmartDNS-to-TLS acceptance are not implemented. Windows and external review of
the isolated TLS parser candidate remain pending. These local tests do not prove
production traffic, cross-platform behavior, or resistance to an authenticated
DNS upstream being compromised.

On 2026-11-04 the first no-default gateway test run failed because a fixture
encountered `CheckpointRequired` during admission and did not refresh the clock.
The fixture now performs the existing bounded checkpoint maintenance, retaining
the same reservation on a consumption retry. A regression waits for a real
checkpoint window. Its first two attempts failed because its four-second limit
was shorter than the possible five-second checkpoint window; after correcting
the test deadline, both gateway feature-profile runs passed 22 tests and the
compile-fail ownership doctest. Gateway Clippy, formatting and planning checks
also passed locally. These results do not establish unattended gateway clock
maintenance or completion of runtime integration.

The inventory worker registry is capped at eight tracked tasks per resolver, in
addition to the existing process-wide worker semaphore. Local tests cover dropped
requesters, repeated shutdown after cancellation, refusal of new work after
shutdown, actual OS inventory shutdown and worker limits. Deliberately weakening
the limit or returning before joining made the corresponding regression fail.
An initial limit mutation exposed a test fixture that did not release its workers
on assertion failure; that fixture was repaired before repeating the mutation.
This registry manages inventory workers only; it is not gateway readiness.

After the inventory registry integration, gateway tests passed in both default
and no-default profiles (24 tests plus the ownership compile-fail doctest).
Gateway Clippy, formatting and planning self-test passed locally.

A fail-closed listener lifecycle has been added: it checks bounded explicit
sockets against the current signed listener configuration, refuses wildcard
binds, immediately closes accepted streams and stops on shutdown or a detected
listener mismatch. It joins its listener tasks and requests bounded inventory
shutdown. It does not claim readiness, parse TLS or forward traffic. Local tests
cover startup mismatch, connection refusal, shutdown/task joining, unchanged
shutdown notifications, and changed listener bindings. A deliberate removal of
the startup configuration check makes the mismatch test fail. Its first local
test build failed because a fixture import was private; after fixing test-only
visibility, the checks passed. This is not gateway acceptance.

One full gateway run failed when a listener test immediately rebound a released
ephemeral port and got `AddrInUse` amid concurrent tests. The test no longer
assumes exclusive ownership of a system-wide port after closing its socket; it
asserts listener task completion instead. Direct post-shutdown port reuse remains
unverified. After this correction, both local gateway profiles passed 27 tests
and the ownership doctest; gateway Clippy passed. The TLS parser qualifier also
passed its 16 Linux tests, but Windows and independent security audit remain
unverified, so production parser adoption and passthrough remain closed.

After the fail-closed listener change, complete offline workspace tests passed
on Linux in both default and no-default feature profiles outside the restricted
filesystem sandbox, including the existing network and protected-path fixtures.
These results do not establish TLS forwarding, native Windows behavior or
current-head external review.

PR #126 CI run 41890017654 failed its Windows workspace job on head
`8e6cc2926f92669e0c01e4194489925000a15185`: the inventory quota test attempted
to reserve all process-wide permits while parallel resolver tests held permits,
and the TCP-first dual-protocol fixture hit Windows socket error 10013 when UDP
could not bind the same port. The quota fixture now confirms the production
semaphore is shared before exercising exhaustion with a separate shared test
semaphore; production limits are unchanged. The DNS fixture now reserves UDP
first and tries up to 64 complete UDP/TCP pairs within one five-second deadline,
refusing setup if none are available. The failed run remains failed; the fixes
require fresh CI evidence on their own head.

The next Windows workspace run 41891146484 failed on head `051bbe9`: the
successful-dial fixture crossed the five-second authorization checkpoint during
resolution, correctly returning `Admission`. The fixture now waits for the start
of a fresh clock window and refreshes the existing in-memory deployment before
admission; it does not extend the production resolution deadline, alter leases,
or disable current-generation checks. Local gateway tests pass in both feature
profiles. The first local Clippy attempt found a test-helper style lint, which was
fixed; the corrected Clippy run passed. Fresh Windows CI remains necessary.

On 2026-11-05 the owner supplied a screenshot reporting a separate clean
Windows worktree at `836d655`: all 27 gateway tests and one doctest passed,
with no failures or skipped tests. This is owner-reported evidence; the Windows
version, invoked commands, feature profile and raw logs were not supplied. It
does not establish real origin connectivity, TLS passthrough or SmartDNS output.
