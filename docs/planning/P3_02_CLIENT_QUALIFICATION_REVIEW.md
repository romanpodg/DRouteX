# P3-02 isolated DNS client qualification evidence

Status: authorized tests-only qualification in progress under [D51](P3_02_CLIENT_QUALIFICATION_CONTRACT.md).
No client acceptance, production adoption or P3-02 application completion is claimed.

## Prerequisite and identity

Before qualification code, PR112 completed normal merge8f234c44 and all11
exact-main40097388131 jobs/both processed reports. See [P3-01 completion](P3_01_LISTENER_REVIEW.md#strict-fixture-correction-normal-merge-and-main).
The direct owner2026-10-22 acceptance applies only to the isolated exact client
profile; the earlier proposed application choices remain unaccepted.

Fresh official crates.io0.26.3 metadata on2026-10-24 reports not yanked, MSRV1.88
and MIT OR Apache-2.0. Independent SHA256 of the official cached archive matches
c480823ed7c2c5d0f09c41020cb6b7c28029ce60ec42dc942158dcf22f8e0a4d and registry
metadata. This is package identity evidence, not completed behavior
qualification. The project compiler remains pinned1.99.0.

## Isolated graph and supply-chain evidence

The separate workspace/lock freezes103 registry packages. Comparison with the
reviewed qualification seed retains every shared selected version and checksum.
Nine newly selected names are hickory-net0.26.3, async-trait0.1.92,
futures-channel/core/io/macro/task/util0.3.34 and slab0.4.12. No root production
manifest/lock change or application dependency adoption occurred.

Before executing the new library, independent SHA256 checks verified all103
cached registry archives against the locked checksums and all5022 packaged files
against extracted source bytes. The [inventory](../operations/DNS_CLIENT_DEPENDENCY_INVENTORY.csv)
records exact SPDX metadata, archive checksums, license/notice file hashes and
actual Windows/Linux normal/build profile membership. There are67 active registry
packages; the metadata/tree count68 also includes the local tests-only package.
Selected net features are exactly tokio; Proto remains std. Tokio's additional
default feature marker has an empty feature body and adds no capability.

Fresh cargo-deny fetch and locked/offline advisories/bans/licenses/sources checks
passed for this seventh isolated bundle. This is a local result; CI integration
has not run. Warnings remain visible: unused existing ISC/BSD allow entries and
the selected Syn2.0.119/3.0.9 pair. Both versions were already selected in the seed;
Syn2 is only in the unused Android JNI path, while both current active target
trees use Syn3. Preserve shared pins and revisit before adding the affected
target/features or production adoption; no suppression or pin update.

Three unchanged other-target packages (jni0.22.4, jni-macros0.22.4 and
jni-sys-macros0.4.1) have SPDX metadata without packaged license text. They are
outside the actual Windows/Linux profile and retain the existing P0-05-NOTICES
obligation in [OPEN_DECISIONS](OPEN_DECISIONS.md#p0-05-review-dispositions).
Obtain exact upstream texts before affected-target adoption/redistribution.
This reuses the recorded restriction; it grants no new license exception.

## Raw-client probes and unresolved boundary

Before D52 implementation, both local default and no-default locked/offline profiles ran14 tests:
11 passed,3 failed, zero ignored/filtered. Fmt and all-target/all-feature Clippy
pass. These are partial Windows-host results, not client acceptance or Linux CI.

Passing independent wire probes establish IPv4 UDP/TCP answer handling,11 ordinary
and auxiliary record types (A,AAAA,CNAME,TXT,NS,PTR,MX,SRV,SOA,SVCB,HTTPS),
foreign UDP peer/port/ID/name/type/class followed by a valid answer, bytewise TCP
prefix/body writes and EOF at five incomplete-frame boundaries. Normal explicit
close aborts/joins the owned TCP background task, and peer EOF/socket rebind are
observed. UDP response cancellation releases its actual socket; TCP response-wait
cancellation closes a peer that sent half a prefix. Closing with unread bytes can
produce FIN or ConnectionReset; both prove closure, while other errors, extra
data and an unclosed socket fail. An earlier EOF-only fixture failed on reset;
that failed run remains recorded separately from the corrected final profiles.
The latter cancellation probe is not proof of
the exact internal partial-read cancellation point or cancellation during write
or connect. A dropped UDP response returns typed Timeout without a second query.

The ordinary500ms connect budget returned Timeout for a reserved unlistening
Windows port. A separate finite3s classification probe reports typed
ConnectionRefused; it does not widen the other500ms request probes or establish
refusal delivery within500ms. No host-network/firewall changes were made.

One mandatory test fails on actual behavior: TCP DnsMultiplexer delivers a
response with the current transaction ID and foreign.test question when the
request was example.test. The connected controlled peer sent this answer.
This is a strict DRouteX role mismatch, not evidence of off-path spoofing or a
general library vulnerability. The raw failure remains failed. A separate
characterization receives foreign then valid answers from the same public TCP
response stream, without reissuing the query; it does not make rejection pass.

Both mandatory IPv6 UDP/TCP probes time out on this development host. They remain
failed, not ignored or waived; clean Windows/Linux IPv6 evidence is still required.
The first historical local run of the refusal probe also remains failed.

Independent standards/spec passes confirm another gap: a delivered-response
counter cannot limit packets/frames discarded internally before delivery. UDP
returns one delivered candidate, while TCP exposes a multi-item stream. TCP
framing may allocate up to65535 bytes before caller validation. These facts
constrain any composed profile; post-delivery checks cannot claim smaller
preallocation or UDP multi-candidate recovery.

A supplementary spec pass found that the initial foreign-IP fixture also changed
the source port; it now holds that port constant and asserts the independent IP
and port dimensions. Both final full profiles retain11 passed/3 failed after the
fix. The narrow standards pass found no additional concrete fixture defect;
these supplementary passes are not the required completed implementation/final
owner-linked Code Reviews or full-D51 acceptance.

The owner accepted [D52 validation-boundary contract](P3_02_CLIENT_VALIDATION_CONTRACT.md)
on2026-10-24, selecting mandatory bounded DRouteX ingress/caller adapters around
this exact library before dependent implementation. The raw candidate remains
unqualified for the strict standalone role. The accepted acceptance/I/O/deadline/cleanup constants are tests-only;
P3-02 application behavior and production resources remain a separate gate.

## Required open evidence

Complete the remaining D51/D52 proof and actual clean-platform matrix. Obtain
actual Windows/Linux profiles, implementation/final account reviews, current-head
checks and normal merge/exact-main/report evidence before accepting this role.

Root production manifests/lockfile, seven-crate inventory, runtime behavior,
authentication and application upstream policy remain unchanged by this stage.

## D52 implementation and local results, 2026-10-25

The owner accepted the complete D52 composed role before adapter code. The isolated
workspace keeps the exact reviewed graph and features. `bounded_io.rs` delegates
sockets to Tokio and framing to Hickory. A shared per-attempt budget limits every
received UDP datagram and successful TCP read/write operation, including input
discarded inside the library. Pending polls spend no successful-I/O budget. UDP
receives into4097 bytes and refuses oversized data before copying into the
library's smaller buffer. No truncated prefix can masquerade as a valid packet.

`guarded.rs` retains the original single question/opcode, checks QR, DNS-semantic
name/type/class matching and decoder exhaustion, and rejects foreign record
classes or incomplete EDNS option data. UDP caller rejection exhausts its one
result. TCP can inspect at most16 delivered candidates from the same stream.
Taking its exchange handle makes a request one-shot even when its future is
cancelled. One deadline covers connect, send, receive and final validation;
successful parsing is followed by a final deadline check before acceptance.
The caller keeps the TCP owner outside the cancelled request future, then
explicitly aborts/joins background work within the separate3s cleanup bound.
Emergency owner Drop only aborts and is not qualified as joined cleanup.

Both final local default/no-default runs execute37 tests:33 passed,4 failed,
zero ignored/filtered. The only failures are mandatory raw/composed UDP/TCP IPv6
on this development host. Fmt and all-target/all-feature Clippy pass. Actual
clean Windows/Linux evidence remains mandatory; these local failures are not
waived. Existing Windows/Linux Workspace jobs now contain this exact isolated
matrix; the existing dependency job checks the seventh deny bundle. No runner,
job, permission, secret or privileged network setup was added.

Passing real IPv4 probes cover all11 ordinary/auxiliary record types; DNS name
case handling; actual foreign IP with the same port, foreign port with the same
IP, wrong ID and question; opcode/QR/count/compression/record/EDNS/trailing input;
valid-after-foreign TCP; one-query UDP behavior; bytewise and inconsistent TCP
frames; exact4096 acceptance and UDP513/4097/65507 refusal; limits of16 datagrams,
16 candidates,65568 TCP receive bytes,4098 write bytes and256 completed operations.
The byte/operation adapter limits also have direct real-socket boundary tests,
so discarded frames cannot evade those limits merely by staying invisible to
the delivered-candidate counter.

Controlled cancellation exercises a real connected socket before exposing a
usable client, and exactly one completed read/write byte before cancelling a
request. The connection-stage gate proves ownership during that public future;
it does not claim every kernel SYN/retransmission state was exercised. Partial
read/write counts and peer observations establish actual progress. Explicit
finish, peer closure, zero owned sockets and release/rebind are checked. Dropped
replies, invalid trickle input and a response delayed past the original deadline
fail closed; query time does not receive a new500ms budget.

The guard's4096 acceptance ceiling does not bound all library allocations.
Hickory framing may allocate65535 bytes before caller inspection; Proto's query
vector capacity is driven by the u16 declared question count, while record
vectors initially reserve at most8 entries. These are bounded library internals,
not a4096-byte total-memory claim. Only the inspected exact profile is considered;
production resource sizing remains a separate complete P3-02 decision.

### Regressions, failed attempts and remaining limits

- Empty A data is rejected by Proto with InvalidEmptyRecord before caller
  delivery. The initial counter expectation failed, then was corrected without
  weakening rejection. The TCP foreign/malformed-then-valid matrix passes.
- The deadline fixture originally wrote after the client had closed, producing
  Windows ConnectionAborted10053. It now races observed early closure against
  sending a late valid response. The original failed runs remain failed.
- One full35-test run reported an additional UDP fixture receive timeout beyond
  the four host IPv6 failures. Ten bounded diagnostic repetitions did not
  reproduce it. The result stays failed and unattributed; later passes do not
  prove its original cause. Exact case/client diagnostics are retained.
- Independent source review confirmed a separate availability defect: failed
  bind retained the socket reservation and prevented Hickory's existing bounded
  source-port selection retries. A real occupied-port regression fails before
  correction and passes after cancellation-safe rollback of only uncommitted
  reservations. A successfully opened attempt still refuses another socket.
  This proves the defect/fix, not causation of the earlier unattributed timeout.
- Seven targeted mutations remove question matching, decoder exhaustion, the
  UDP receive cap, TCP candidate cap, completed-I/O cap and each TCP byte cap.
  Every mutation compiles and produces the intended behavioral test failure.
  All original source bytes are restored; both final complete profiles retain
  only the four host IPv6 failures. Mutation logs are local supporting evidence.

Initial supplementary full standards/spec review attempts were usage-limited,
without completed verdicts. Fresh complete supplementary passes subsequently
finished: the bind reservation finding has the red/green correction above, and
the missing TCP wrong-ID proof now has a real same-stream negative test. It sends
a correct question with the wrong ID and forged A203.0.113.66 before the proper
ID/A203.0.113.9 response; both frames are received, only one query is sent, and
cleanup/rebind pass. Follow-up source review closed that finding with no new
defect. Both37-test profiles retain only the four host IPv6 failures.
At this local checkpoint, the required owner-linked account reviews were open.
No self-review, quota error, request or test run is counted as actual completed
implementation/final account Code Review.

## Implementation head review and clean platform evidence

Initial qualification headf4a8d8c0451514938bb2ef847391d75a7e3a5df6 is in
[PR113](https://github.com/romanpodg/DRouteX/pull/113). All11
[CI40190172872 jobs](https://github.com/romanpodg/DRouteX/actions/runs/40190172872)
passed on that head. Full actual logs show244 Windows/276 Linux root-workspace
tests in each default/no-default profile, zero failed/ignored/filtered. The new
isolated client step passes37/37 in each profile on both platforms, including
all four mandatory raw/composed IPv6 probes. All seven deny bundles pass.
Both Codecov uploads have processed state merged; Rust7902/8287 is unchanged
application coverage, not a coverage claim for tests-only adapter code.

Completed linked-ChatGPT-account read-only/never Code Review
01a172e1-ffa9-7b13-88ab-312f92c93875 exited0 without concrete defects. It reviewed
the whole PR from base8f234c44 in a clean managed worktree. Fmt, planning/source/
links and the changed Python authorization test passed. Fresh Rust tests/Clippy
could not run successfully in that read-only toolchain environment; the actual
independent platform results above supply those checks.

CodeScene8117794 on this same head failed all three maintenance gates. It reports
guarded.rs8.55 (Expected.accepts complexity/Bumpy Road Ahead), guarded_tests.rs8.87
(length/cancellation/operation fixture), independent.rs9.84 (udp_probe), and the
auxiliary authorization test9.13->9.02. The concrete findings, risk, rationale and
owners/triggers are recorded in [OPEN_DECISIONS](OPEN_DECISIONS.md#p3-02-qualification-review-dispositions).
The owner excludes tests/fixtures/auxiliary planning as independent metric targets.
This disposition does not hide a failed result or authorize a suppression or
required-check/protection bypass. Functional defects remain blocking.

Final evidence-head whole-PR account review/current-head checks and normal
exact-head merge/exact-main/report gates are still required before completing
qualification. Earlier local IPv6 and unattributed UDP failed runs stay failed;
clean CI establishes the required current platform proof, not their causal history.
