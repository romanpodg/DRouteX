# P3-01 bounded legacy DNS transport contract (D47)

Status: accepted by the owner on2026-10-22 via direct chat reply accepting the
complete profile after checked PR107. No dependent implementation before verified
normal PR107 merge/exact-main; qualification/adoption gates below remain required.
D45 selects execution order; D46 authorizes current source admission only.

## Scope and dependency qualification

Add the real droutex-dns library at its roadmap gate, reusing core's Deployment,
the actual current D46 gate, D37 domain rules and D38 policy selection. Implement
UDP and TCP DNS transport, bounded parsing/replies and cancellation/shutdown.
No daemon/installer/public management API, DoH/DoT, network grant learning, origin
resolver, cache, DNSSEC validator or private/BLOCK/SmartDNS answers in this stage.
Those remain P3-02..07. Before those handlers exist, a valid authorized request
gets SERVFAIL (RA=false, AD=false), not fabricated resolution/protection success.
An internal bounded processor seam permits controlled independent wire fixtures
and later real upstream/policy integration using the same admitted generation.

Qualify exact hickory-proto0.26.3 for production DNS codec use, defaults off,
features=[std]. Qualify existing Tokio1.53.3 networking/io-util/macros features in
addition to current rt-multi-thread/sync/time. No hickory-server/resolver, Rustls,
TLS, DNSSEC, DoQ, process or privileged network feature adoption here. Existing
P0-05 qualification is prior evidence, not approval of the production tree.
Inspect exact locked transitive versions/checksums/licenses/notices; all five
existing cargo-deny bundles and Windows/Linux feature profiles must pass before
conditional adoption. No new license exception, advisory suppression or floating
direct dependency. Unexpected dependency/license/security changes return to owner.

The installed exact official crate sources were inspected, including Cargo
features and Message::read/from_vec/emit. The from_vec convenience method does not
itself check complete input consumption; explicitly check decoder exhaustion.
Versioned docs.rs pages were unavailable to the browser, not treated as evidence.

## Runtime identity and listener ownership

Only explicit enabled DNS UDP/TCP ListenerIds from the actual active generation
can run. Validate the actual socket's local address/port/transport against that
generation, including after a publication; an old socket never silently serves a
new binding with the same ID. New/changed bind needs explicit runtime restart or
rebind; listener supervision/packaging remains later work. At most16 active DNS
sockets per node. No implicit wildcard bind, automatic interface selection, IP
learning or bind during config import. Tests use unprivileged loopback ports.
Production port53 packaging/socket activation/capabilities remain P10; this change
does not install a service, alter firewall/routes or bind a live public service.

D49 owner decision accepted2026-10-22 after the linked-account wildcard finding:
this initial UDP runtime requires an explicit local IPv4/IPv6 address. Startup
refuses0.0.0.0 and:: with a configuration error before starting a worker; a later
wildcard publication cannot reuse an existing explicit UDP socket. The schema
remains unchanged for future destination-aware I/O qualification. TCP wildcard
is supported because accepted connections retain the kernel-established local
destination. Tests prebind disposable wildcard sockets only for refusal and
TCP loopback checks; no public service is deployed. No new dependency/unsafe I/O
or automatic interface selection is adopted by this decision.

Use actual socket peer and the bound ListenerId. No forwarded header, query label,
EndpointId, EDNS ECS or last DNS query supplies authority. Exact IPv4/IPv6 remains
separate; mapped IPv6 is refused by D46. Unknown/unavailable/revoked/expired peers
receive no UDP reply; TCP is closed without processing a message. Never recurse
or invoke a handler for them. Recheck admission per TCP query, not once per socket.
Hold one admitted generation through parse/policy/response; never repin its Profile.
Before submitting a reply, recheck current authority; a failed current/lease check
drops it. The final check is the decision point; already submitted bounded kernel
writes cannot be retracted on later publication. Test that distinction explicitly.

Run local clock refresh in one bounded blocking maintenance operation at a time,
nominally every1s, independent of CP/DB; no queue of retries and no auto re-anchor.
Checkpoint/clock failure keeps D46 closed. No filesystem I/O or blocking mutex held
across await in request handling. Maintenance cancellation cannot pretend to undo
an in-progress atomic disk transaction. Shutdown waits for or reports that outcome.

## Initial fixed transport limits

These are DRouteX's proposed conservative limits, not vendor claims or throughput
benchmarks. D46's shared per-group/node DNS quotas still apply across transports.

| Resource | Proposed limit |
|---|---:|
| Node UDP intake, before source lookup/parser | 2000 datagrams/s, burst4000 |
| Node TCP accept intake | 100/s, burst200 |
| Live TCP DNS sockets per node | 64 |
| New TCP connection slots / group | Shared node ceiling plus D46 admission; no untrusted per-IP map |
| Active DNS requests | D46:64/group,512/node; no unbounded waiting queue |
| Total owned transport wire buffers | 16MiB/node (not a total-RSS promise) |
| UDP input/output wire ceiling | 1232 bytes |
| TCP framed message/output ceiling | 65535 bytes, exactly one 2-byte length prefix |
| Single query handling / reply write deadline | 2s total; processor at most1500ms, remaining500ms reserved for reply(D48) |
| First TCP frame / non-idle frame completion | 5s absolute; trickle bytes do not extend it |
| Between TCP queries idle time | 15s |
| TCP connection lifetime / query count | 120s /100 queries |
| Graceful shutdown drain | 5s, then cancel and join owned async tasks |

TCP reserves the first D46 attempt before reading its initial frame, then checks
it at consumption; expired/retired reads cannot become new admissions. Process
pipelined frames sequentially, preserving IDs and bounded buffering; no parallel
per-connection queue. Split prefixes/payloads and coalesced frames must work.
Incomplete/zero/invalid frames, timeout, quota/buffer exhaustion and I/O errors
close the connection, releasing every slot/reservation. An application task is
owned/joined; cancellation of an incomplete read/write never resumes it as a new
frame. Rate buckets use monotonic time; rejected traffic allocates no source map.

## Wire and response behavior

Use Hickory's codec, not a custom DNS compression/parser implementation. Check
bounded header counts before parsing record collections; exactly one ordinary
QUERY question, IN class, no answer/authority records and at most one root OPT.
Reject truncated pointers, loops, unsupported label encodings, trailing bytes,
multiple questions/OPT and malformed RDATA without a handler call. Keep valid
wire labels separate from D37 policy canonicalization; unsupported policy names
fail explicitly (including root until separately specified), not by bypassing
policy. A/AAAA/CNAME/TXT/HTTPS/SVCB and auxiliary ordinary IN query types retain
their wire type for later forwarding. AXFR/IXFR, UPDATE, NOTIFY and other opcodes
are unavailable. Authenticated well-formed unsupported requests get NOTIMP;
malformed requests are dropped/connection closed, never reflected to unknown peers.

EDNS(0) only; nonzero version returns bounded BADVERS to an admitted peer. Interpret
an advertised UDP size below512 as512, cap at1232, default512 without OPT. Ignore
unknown well-formed EDNS options; never learn identity or forward ECS. Bound
option processing by16 options/4096 option bytes (and the transport's wire limit).
Never echo arbitrary options or assert AD/validation from request flags. Full
DNSSEC/CD/DO forwarding/validation and override semantics remain P3-02/P3-04/P3-08.

UDP reply cap is min(client UDP ceiling,1232,3*received request bytes). This
additional amplification cap may force TCP fallback earlier for large answers.
If a trusted later processor's complete response exceeds the cap, construct a
valid TC response within the budget using the library encoder; never cut a raw
packet mid-record. If a valid bounded TC response cannot fit, drop it. TCP provides
the full bounded response. Do not send more than one reply per datagram/frame.
Preserve query ID/question and correct QR/opcode/RCODE; do not invent successful
origin/private answers, authority or effective routing. Timeout/unimplemented
processing yields bounded SERVFAIL; overload/unauthorized traffic is dropped.

D48 owner clarification accepted2026-10-22: the processor cutoff is1500ms from
the original query start, retaining500ms within the same two-second total for
fallback encoding/write. If admission/parsing consumed that allocation, skip
the processor and attempt SERVFAIL only while total time remains. A late reply
is dropped and TCP closes; lost/expired/revoked admission never gets a fallback.
An earlier connection lifetime bound still closes the connection. Deadlines
are checked on every future resume and before final submission; a ready socket
operation cannot outrun an already expired timer. Already submitted kernel
bytes cannot be retracted.

RFC references checked2026-10-22: RFC7766 (TCP framing/reuse/pipelining) and RFC6891
(EDNS payload/version/options). The limits/strict initial subset above are DRouteX
proposals; they are not claimed to be mandated by those documents or by Control D.
Control D remains the primary product reference for Endpoint/Profile/source scope;
D46 already records verified vendor facts and deliberate DRouteX distinctions.

## Adoption evidence

Actual unprivileged Windows/Linux UDP/TCP exchanges with independent fixed DNS
bytes; split/coalesced/pipelined/oversized/trailing/compression/EDNS/TC cases;
same-IP different Profiles and exact IPv6; no handler/reply for denied peers;
revocation/expiry between receive/consume/reply; failed clocks/checkpoints/CP loss;
flood/slow frame/buffer/rate/concurrency limits; cancellation, shutdown and real
socket release/rebind. Timeouts and negative network assertions must be bounded
and avoid claiming an absolute impossibility from one observation. Test actual
codec behavior and refusals, not only own encoder roundtrips. All existing
workspace/feature/deny/qualification checks and both required linked-account Code
Reviews plus normal head/merge/main gates remain mandatory. No real public DNS,
TLS, recursion, learning or production systemd acceptance is claimed here.

Owner choice requested: accept this complete profile after verified PR107, with
qualification and conditional production adoption only after successful checks;
or revise scope/limits/dependency choices before dependent code starts.
