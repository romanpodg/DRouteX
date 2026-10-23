# P3-01 transport evidence

Status: **qualification PR108 completed; runtime PR109 merged normally, but its
first exact-main Windows run failed a TCP fixture case. A bounded fixture-start
correction is under validation; P3-01 remains in progress**. The owner accepted
complete D47 conditionally on checked PR107. Before qualification code, PR107
merged normally at02b6695928a64c37de5e073a4bdb5162f1928c54; all11 exact-main39747433154
jobs and both Codecov sessions processed(Rust6689/7013). See
[P4-01 completion](P4_01_BINDING_REVIEW.md) and [D47](P3_01_LISTENER_CONTRACT.md).

## Qualification boundary

The standalone tests/dns-transport-dependencies package qualifies exact
Hickory0.26.3 defaults-off/std and Tokio1.53.3 defaults-off networking/io/macros
plus existing runtime/sync/time. At that isolated stage root production manifests/lockfile
and all six production crates remained unchanged. Existing test-only TLS qualification remains
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

## Qualification completion before runtime adoption

Final62f2f2712229074d4897514463b7b3b6af763ce6 passed all11
[CI39752553613 jobs](https://github.com/romanpodg/DRouteX/actions/runs/39752553613).
Linked-account read-only/never final review01a162da-85ac-73d0-85fd-7d35bee10e7a
completed exit0 without a confirmed defect. Fmt/9 planning checks passed;
read-only Rust locks prevented a rebuild. The post-verdict rollout warning is
retained, not substituted for a review result. Both Codecov sessions processed
with unchanged Rust6689/7013. Failed CodeScene8075138 has identical finding text
and numeric tables to8074936; the existing auxiliary disposition applies.

[PR108](https://github.com/romanpodg/DRouteX/pull/108) merged normally at
d36d7b2c5233822113d1085ac85dc9dd422fa401,2026-10-22T09:10:55Z, exact-head match,
without admin/force/check bypass. All11
[exact-main39753424789 jobs](https://github.com/romanpodg/DRouteX/actions/runs/39753424789)
passed and both Codecov sessions processed(Rust6689/7013). Only after those gates
did actual runtime work begin. Earlier open qualification statements above are
historical candidate evidence; full runtime acceptance remains separate.

## Runtime implementation candidate

The real seventh crate consumes a Deployment owner and explicit prebound sockets.
Node-wide budgets span listeners/generations; no public alternate processor or
admission path exists. Each UDP datagram/TCP query uses actual source/listener D46
and D38 selection from one generation, with final current/lease checks. Binding
replacement cannot reuse an old socket merely by retaining its ID. Default
processing returns SERVFAIL, RA=false/AD=false. No upstream/network learning,
private/filter/SmartDNS answer, public deployment or gateway connector is claimed.

All28 newly added root registry name/version/checksum tuples exactly match the
qualified graph; no old tuple was removed. Fresh current-advisory cargo-deny
passed advisories/bans/licenses/sources for all six bundles. Root coverage paths
include the actual DNS source, while upload permissions/isolation remain intact.
Planning authorization includes directly accepted D48:1500ms processor plus
remaining500ms inside the original2s total. The owner accepted this clarification
after independent spec review identified that a full2s processor wait leaves no
time for the promised timeout SERVFAIL. Total expiry/lost admission drops/closes.

Concrete regressions found and corrected before adoption:

- An over-budget TXT record can make the codec return an error instead of TC.
  The runtime retries a complete library-truncated response and validates the
  encoded question/ID/length before sending; no raw packet cutting.
- The pinned OPT parser silently clears incomplete options. An independent
  partial-option fixture failed refusal before the fix, then passed after exact
  original-RDATA accounting. Options are discarded before the processor, including ECS.
- Tokio Timeout polls its inner future first. Independent probes and an actual
  expired buffered frame reproduced successful work after deadline. Every resume
  now checks monotonic time before polling I/O, with timer-first cancellation and
  explicit stage/final-submit checks. A late-ready operation regression is retained.
- A compressed root OPT owner takes two bytes. Independent maximum-question/
  4096-byte option data accepts4379 bytes; the derived query bound includes it.
- A departed Windows UDP client can surface a connection-reset receive error.
  Transient peer errors are contained per listener instead of terminating the node.

Current local fmt/check/all-target all-feature warning-free Clippy and default/
no-default workspace runs excluding the single known-host IPv6 case passed.
The earlier complete native run had17 DNS successes/one IPv6 failure, so it is
not a complete platform pass. Mandatory IPv6 remains enabled in clean CI; no
system/firewall setting or test skip was added. Runtime CI, current production
coverage/CodeScene, linked-account implementation/final reviews and normal
merge/exact-main remain unverified at this candidate stage.

Tests use independent wire bytes and actual unprivileged sockets: type/ID/rcode,
bad compression/counts/trailing/EDNS, TC versus full TCP, split/coalesced pipeline,
unknown sources/no handler, exact IPv6, same-IP separate Profiles, revocation and
expiry during processing, changed bind, absolute trickle deadline, shared group
quota across UDP/TCP, node64 TCP refusal and real release/rebind. The shutdown
scheduling regression waits the real5s drain, aborts/joins an owned socket task,
proves permit/rebind release and reports a still-running bounded blocking task.
It verifies task/outcome scheduling, not a new atomic-store crash qualification.
Linux adds actual durable-store/CP-directory loss and a real automatic checkpoint
filesystem failure. Those Linux tests still require their actual platform run.

## Actual implementation review and platform evidence

Candidate5117e5f4a60b070a8fdc9f0d5307b9c88c03f802 in
[draft PR109](https://github.com/romanpodg/DRouteX/pull/109) passed all11
[CI39761170349 jobs](https://github.com/romanpodg/DRouteX/actions/runs/39761170349).
Actual workspace counts were225 Windows/257 Linux for each default/no-default
profile, with zero ignored/filtered cases. DNS26 Windows/27 Linux includes exact
IPv6 UDP/TCP, pending-processor SERVFAIL and pending-write admission checks. Linux
also passed actual durable CP-directory loss and automatic checkpoint filesystem
failure. Both Codecov sessions processed: Rust7462/7857; actual new Rust patch
770/841(91.56%). No coverage value proves route/security correctness.

Repeated supplementary standards/spec reviews found no confirmed source defect.
Required linked-account read-only/never review01a16317-f124-7290-aafa-7ca351381e18
completed exit0 with a confirmed P2 finding: wildcard UDP receive loses the local
destination, so send_to may reply from a different local address. Independent
Python stdlib reproduced request127.0.0.2/reply127.0.0.1 on this host. A connected
client may discard that response. Fmt passed; read-only Cargo locks prevented its
Rust/Clippy rebuild. Rollout-record warnings are retained alongside the actual
completed verdict. This is a completed review with a defect, never a clean review.

Green CI did not cover wildcard source preservation and does not close the
finding. The owner has been asked to choose explicit-only UDP addresses for this
initial runtime, or separate destination-aware I/O qualification before wildcard
support. No restriction/new I/O dependency is accepted merely by that question.
The independent positive public-API regression confirms explicit127.0.0.2 replies
correctly to a connected client on127.0.0.1. Source correction/re-review and final
current-head/normal merge/exact-main gates remain open.

CodeScene8076431 failed all three gates, including its critical Bumpy Road label
for runtime lifecycle complexity. Source/independent review supports explicit
minor maintenance dispositions, not a critical functional defect conclusion;
see [the register](OPEN_DECISIONS.md#p3-01-review-dispositions). No suppression,
threshold change, check bypass or score-only refactoring was performed. The
confirmed wildcard bug remains a separate merge blocker.

## Owner-accepted wildcard correction (D49)

The owner selected explicit UDP addresses now by direct reply2026-10-22.
`Binding::check` refuses actual UDP wildcard0.0.0.0/:: before workers start;
unchanged actual-bind equality also refuses an old explicit socket after a
wildcard republication. Configuration import/schema and TCP wildcard remain
unchanged. No destination-aware I/O dependency, unsafe wrapper or interface
enumeration is adopted. The original review finding remains preserved above;
its correction still needs actual independent current-source/platform evidence.

New bounded startup regressions check both IPv4/IPv6 wildcard refusal and socket
release/rebind, a publication-to-wildcard refusal without a handler call, and TCP
wildcard correct reply to a connected nondefault127.0.0.2 destination. Removing
the production refusal made its startup test fail at the bounded timeout;
restoring the exact source made it pass. The public explicit-UDP127.0.0.2
regression remains a separate successful supported-path test.

The full local native DNS run outside sandbox had29 successes/one known IPv6
receive failure, with zero ignored/filtered cases. All three new D49 cases passed,
including TCP wildcard; its ordinary sandbox connect had Windows10013, while
the outside-sandbox Python stdlib probe and actual Tokio test passed. No network/
firewall/system setting changed. IPv6 remains mandatory in clean Windows/Linux
CI. Local incremental-cache hardlink fallback warnings were avoided in the
subsequent process-local CARGO_INCREMENTAL=0 run; no project lint was weakened.

## Corrective implementation evidence

Corrective head d00c054788948c9636199a76385e1e9c60b9fb67 received completed
linked-account read-only/never review01a1640b-eb5c-77c2-b4d3-21b773595406,
exit0 with no confirmed defect. Fmt and diff checks passed; read-only Cargo build
locks prevented its Clippy/test rebuild. Both supplementary standards/spec passes
also found no confirmed defect. The original wildcard finding above is retained;
D49 closes it in source, subject to actual platform/final-head/merge evidence.

Codecov processed both exact-head sessions: Rust7489/7860 and actual new Rust
797/844(94.43%). CodeScene8081708 failed all three gates. Runtime health7.61
adds Binding.check complexity to the earlier findings: the new explicit UDP
refusal is one transport/address predicate before signed binding validation,
not a second admission path. Its bounded refusal mutation and explicit/TCP
positive regressions support retaining the guard without score-only rewriting.
Wire remains7.91; auxiliary hotspot scores are9.15/8.75. Updated concrete minor
dispositions remain in [the register](OPEN_DECISIONS.md#p3-01-review-dispositions),
with no suppression or check bypass. Actual CI39797242225 and final evidence
review/current-head/normal merge/exact-main gates remain open.

Actual corrective Workspace logs passed229 Windows/261 Linux tests in each
default/no-default profile, zero failed/ignored/filtered. DNS30/31 includes both
exact IPv6 transports, all D49 refusal/supported-path cases and Linux durable
CP-loss/checkpoint failures. All six dependency bundles passed. GitHub reports
the run completed/success and ten individual jobs completed/success, but the
Linux Workspace check still reports in_progress despite its completed successful
steps/log. Its status is explicitly unconfirmed; no owner CI-status exception
was requested or used. Final evidence review and the final commit's actual full
head/main checks remain mandatory before P3-01 completion.

## Continuation and exact-main failure, 2026-10-23

Fresh GitHub reads confirmed all11 final-head39800100143 jobs completed/success
on1f94fe4d5bdf454f269eaeb8bd9d15f5be7f0469. Both Codecov sessions processed
(Rust7478/7860, new Rust786/844). Completed linked-account final review
01a1641d-1bd3-70a2-985c-0c0abe711453 exited0 without confirmed defects; fmt/diff
passed, read-only Cargo locks prevented a fresh rebuild and six existing-binary
tests were supplementary only. Its post-verdict rollout warning is retained.
CodeScene8082054 failed with the same finding text/numeric tables as8081708;
the existing concrete minor dispositions apply. The service remains failed.
GitHub reported no protected main branch or active ruleset. No protection,
threshold, service finding or workflow check was changed or bypassed.

[PR109](https://github.com/romanpodg/DRouteX/pull/109) merged normally at
4890609f20839516fbb346505b70a2c208986912 on2026-10-23T03:17:04Z, matching the
reviewed head. The first [exact-main39894180552 attempt](https://github.com/romanpodg/DRouteX/actions/runs/39894180552/attempts/1)
passed ten jobs but failed Windows Workspace: tcp_split_prefix_payload_and_coalesced_pipeline_keep_ids
received UnexpectedEof while reading a reply prefix. DNS29/30 passed, including
actual IPv6. Both exact-merge Codecov reports processed(Rust7489/7860); that
does not make the failed platform run pass. Only the failed Windows job was
re-run to gather further evidence; the original failure remains part of this record.

The TCP case passed100 isolated local repetitions. Five full diagnostic DNS
runs each passed29 cases and failed only the existing local-host IPv6 case.
Temporary admission diagnostics were removed and exact production source bytes
restored. These results do not establish the cause of the original CI refusal.

Source inspection found a separate reproducible fixture startup race: Server::start
slept40ms from spawning the server, although node intake rate buckets are created
inside serve's first poll. A delayed first poll could therefore return a fixture
before its rate warmup even started. The fixture now awaits a bounded one-shot
first-poll acknowledgement before the same40ms rate warmup. Production admission,
quota values, deadlines and transport behavior are unchanged. A controlled delayed
start regression fails with the specific premature-start assertion when the
barrier is removed and passes after exact restoration. It then exchanges a real
TCP query and verifies reply identity and socket release/rebind. This proves the
fixture correction, not that it was the sole cause of the earlier CI failure.

Local corrected fmt and warning-free workspace all-target/all-feature Clippy
passed. Full default/no-default workspace attempts each reached DNS30 successes
and the one existing host IPv6 receive failure, zero ignored/filtered DNS cases;
neither is recorded as a complete passing workspace profile. The delayed-start
regression and unchanged TCP split/pipeline scenario passed. Clean Windows/Linux
CI must run the complete profiles, including IPv6, before acceptance.

Corrective current-head checks, independent review, normal merge and exact-main
gates remain required before P3-01 completion or the accepted next qualification.

## Fixture correction review and platform evidence

The second attempt of original exact-main39894180552 completed all11 jobs
successfully after re-running only Windows Workspace. Attempt1 remains failed;
the retry does not prove the cause of its UnexpectedEof. The independently
reproduced startup correction is delivered in [PR110](https://github.com/romanpodg/DRouteX/pull/110).
Its separate documentation commit publishes the previously owner-approved
guidance/security maintenance; no new runtime or dependency behavior is bundled.

Head81820e0599a9b60dbd8139deed6a6493471da8c6 passed all11
[CI39895933186 jobs](https://github.com/romanpodg/DRouteX/actions/runs/39895933186).
Actual workspace logs passed230 Windows/262 Linux tests in each default/no-default
profile, zero failed/ignored/filtered. DNS31/32 includes the delayed-start regression,
unchanged split/coalesced pipeline, mandatory actual IPv6, and Linux CP-loss and
checkpoint-failure cases. All six dependency bundles passed. Both Codecov sessions
processed(Rust7478/7860); this test/documentation patch has no new production Rust
coverage claim. CodeScene8089824 passed all three gates for this delta. That does
not change the historical failed runtime analyses or their recorded dispositions.

Required linked-account read-only/never review01a166d4-4e4b-74a1-b024-ddeabc36d036
completed exit0 without confirmed defects. Fmt, diff and read-only planning/source
checks passed; Cargo's read-only build lock prevented a fresh Clippy/test rebuild.
Two targeted cases passed from the existing corrected binary as supplementary
evidence. The post-verdict rollout warning is retained. Separate standards/spec
passes also found no confirmed defect; they do not substitute for that account review.

Local AST-only graph refresh completed without an API/LLM backend:4956 nodes,
10488 edges. Its warnings retain the empty Cargo.toml extraction and four SQL
files without an installed SQL parser. The separate historical document semantic
baseline issue remains deferred; graph outputs stay local and sources remain
authoritative. No parser installation, paid backend or forced shrink was used.

Final evidence head review/checks, normal exact-head PR110 merge and exact-main
checks/report processing remain mandatory. No P3-02 qualification implementation
or production forwarding has begun.
