# P3-01 transport evidence

Status: **qualification candidate verified; final review/head/merge/main still open;
not adopted into production**. The owner accepted
complete D47 conditionally on checked PR107. Before qualification code, PR107
merged normally at02b6695928a64c37de5e073a4bdb5162f1928c54; all11 exact-main39747433154
jobs and both Codecov sessions processed(Rust6689/7013). See
[P4-01 completion](P4_01_BINDING_REVIEW.md) and [D47](P3_01_LISTENER_CONTRACT.md).

## Qualification boundary

The standalone tests/dns-transport-dependencies package qualifies exact
Hickory0.26.3 defaults-off/std and Tokio1.53.3 defaults-off networking/io/macros
plus existing runtime/sync/time. Root production manifests/lockfile and all six
production crates remain unchanged. Existing test-only TLS qualification remains
separate; no Crypto/TLS/NASM feature is introduced into this isolated package.
Its lockfile starts from reviewed existing qualification versions and is pruned
by Cargo offline. All94 registry package name/version/checksum identities are an
exact subset of the existing qualified TLS/DNS lockfile; none is newly resolved.
Fresh cargo-deny passed advisories/bans/licenses/sources for the isolated profile,
with only unused allowed-license warnings. No new license exception is required.

Independent fixed DNS bytes exercise compressed answers, all truncated prefixes,
malformed pointers/counts, decoder exhaustion (from_vec accepts trailing bytes),
EDNS versions/unknown options and bounded encoder/TC behavior. Actual loopback
Tokio tests cover IPv4/IPv6 UDP, split/coalesced/pipelined TCP and cancellation of
an incomplete prefix with socket/slot release and owned-task abort/join cleanup. These fixture peers are not
DRouteX listeners, source authentication, DNS resolution or runtime acceptance.

## Actual results and open evidence

The first Windows run compiled successfully and passed7/8 cases. UDP IPv4 passed,
but the IPv6 client receive timed out. The targeted retry outside sandbox had the
same result. A separate Python stdlib loopback probe passed IPv4 and timed out on
IPv6 receive, independently reproducing an IPv6 UDP problem on this development
host; it does not establish a Tokio-specific defect. The test remains mandatory,
not skipped or weakened. No firewall/network/system setting was changed. Actual
clean Windows and Linux CI and source review must resolve qualification evidence;
the local failed run is retained and is not recorded as a successful test.
After extending TCP framing to IPv6, the host also refused connect with Windows10013;
Python stdlib independently reproduced the same IPv6 TCP PermissionError. Both
mandatory dual-stack cases remain in CI. The strengthened full run before the
additional abort/join case had6 passing/2 failing tests. The final9-case local run
had7 passing/2 failing tests; this is not a successful qualification profile.
Warning-free isolated all-target Clippy and fmt passed. The execution-boundary
regression initially used P3-01 as unauthorized future work; it was updated to
P3-02 after D47 explicitly authorized P3-01. This was a stale test expectation,
not a runtime defect or permission relaxation for later work.

CI reuses both existing Windows/Linux workspace jobs for the isolated profile,
without new permissions/runners/installers. The existing five dependency bundles
remain mandatory; the new sixth transport bundle also gets cargo-deny. Production
adoption, actual DRouteX sockets, final qualification review/head/main evidence
and normal merge remain unverified. Current implementation evidence follows.

## Current-head platform and review evidence

Candidate083d83d0f6e7e74926bdd1f5378f75265e7fc21d passed all11 actual
[CI39750969047 jobs](https://github.com/romanpodg/DRouteX/actions/runs/39750969047).
The isolated9-case profile passed twice on each Windows/Linux job (default and
no-default), with zero ignored/filtered tests. Actual IPv6 UDP/TCP and abort/join
cases passed on clean runners. The earlier8-case head d062b6e also passed all11
CI39750244190 jobs; it is not substituted for current-source evidence.

All six cargo-deny bundles passed. Independent Python SHA256 checks verified all94
cached official registry archives against the frozen checksums. License choices
use existing permitted alternatives; no LLVM/LGPL/Unlicense exception was adopted.
No root production dependency, TLS/crypto feature or registry version was changed.
Both supplementary standards/spec passes found no confirmed defect. Required
linked-account read-only/never implementation review01a162d2-a03c-7250-8a6c-1e2545d019ca
completed with exit0/no confirmed defect. Its fmt/9 planning checks and lock identity
review passed; read-only build/advisory locks prevented Rust/Clippy/deny execution,
so actual platform/dependency evidence comes from parent runs and independent CI.

Codecov processed both sessions with unchanged Rust6689/7013. There is no changed
production Rust patch to score. CodeScene8074936 failed only the two auxiliary
planning hotspot rules; its other two gates passed. The explicit disposition in
[the register](OPEN_DECISIONS.md#p3-01-review-dispositions) preserves the failure
without suppression or score-only cleanup. Production integration, final account
review, final-head/normal merge/exact-main and full P3-01 runtime acceptance remain
open; a dependency qualifier is not a DRouteX DNS service.
