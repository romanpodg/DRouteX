# Bounded DNS transport, forwarding and scoped cache library

P3-01 implements the accepted [D47/D48/D49/D50 contract](../../docs/planning/P3_01_LISTENER_CONTRACT.md).
The caller owns a Tokio runtime, an initialized `Deployment`, explicit prebound
UDP/TCP sockets and a shutdown watch channel. `DnsNode::new` consumes the deployment;
`deployment()` allows the host to share it with publication/gateway consumers.
`serve` consumes the DNS owner. Importing configuration binds no socket.

Every query uses actual peer/listener/current D46 admission and one immutable
policy generation, with another current/lease check before submission. Socket
addresses/transport must still match that generation. New bindings need explicit
restart/rebind. Unknown, expired, revoked or unavailable peers get no reply.
Under accepted D49, UDP requires an explicit local IPv4/IPv6 address:0.0.0.0/::
are refused before workers start. This avoids a wildcard reply with the wrong
source address until destination-aware UDP I/O is separately qualified. TCP
wildcard retains its kernel-established destination and remains supported.

Under the accepted [D53 contract](../../docs/planning/P3_02_FORWARDING_CONTRACT.md),
Resolve uses the same generation's strictly decoded public numeric upstream group.
Other actions remain unavailable and produce SERVFAIL without upstream traffic.
DO/CD requests receive NOTIMP. This subset provides no private override,
filtering answer, DNSSEC validation, secure endpoint, gateway or public deployment.
RA describes configured forwarding capability; AD/AA are cleared. Fresh upstream
queries use independent IDs, RD and EDNS0/1232 without client options or ECS.

Groups have one or two UDP/TCP members on port53, with explicit fallback only.
Each member has500ms inside the original processor cutoff. Only a fully validated
UDP TC response permits TCP to the same pinned address. There is no retransmit,
reconnect, OS resolver, connection to returned addresses or traffic DIRECT fallback.
Plaintext upstream DNS provides no peer authentication or confidentiality.

Fresh unprivileged OS address inventory and all configured listener addresses
prevent local loops before new connect/send operations. `serve_with_aliases` also
accepts at most256 trusted operator-declared public/NAT/service addresses; hosts
must supply aliases not visible in their OS inventory. Enumeration failure or a
self destination refuses the whole group. No host networking changes occur.

Under [D54](../../docs/planning/P3_03_CACHE_CONTRACT.md), Resolve first checks a
private whole-response cache after current admission and policy selection. Keys
include exact Endpoint/Profile/network-group/generation, DNS question, configured
upstream resource and the non-DNSSEC forwarding domain. Positive entries use the
shortest retained RR TTL capped at3600s; SOA-backed exact-question NXDOMAIN/NODATA
use at most60s. Failures, zero TTLs, alias-bearing negatives and expired data are
not cached. There is no coalescing, prefetch, stale serving or gateway-origin use.

The fixed arena, keys and metadata fit within16MiB/2048 slots; one scope uses at
most2MiB/256 slots. FIFO eviction and nonwaiting lock contention preserve bounded
work; misses keep D53's normal limits. Each hit rechecks current authority and
carries expiry to client writes; conservative TTL aging includes the remaining
write window. New applied generations/restarts start cold, while a rejected
candidate preserves usable LKG cache entries. See the
[D54 allocation audit](../../docs/planning/P3_03_CACHE_ALLOCATION.md) for requested
persistent storage versus decoded transient data and RSS.

Limits are node-wide:16 listeners,64 TCP connections,512 owned contexts,16MiB
transport wire credits and fixed monotonic intake buckets. Forwarding additionally
allows8 slots/node and2/network group, without a waiting queue. Each holds128KiB
wire credits and its original admission through all attempts and joined cleanup.
D46 group/node quotas
remain shared with every DNS transport. UDP is1232 bytes and at most3x the query;
TCP uses one u16 big-endian frame prefix. Large responses use a valid encoder TC
fallback. Strict parsing accounts for all original EDNS bytes and discards client
options before processing. No ECS/header/query field supplies identity.
Upstream acceptance is4096 bytes with mandatory complete question/frame checks.
Per transport, prefilter limits are16 UDP packets,16 delivered TCP candidates,
65568 read bytes,4098 write bytes and256 successful operations per direction.
Pending polls are free. The [allocation audit](../../docs/planning/P3_02_FORWARDING_ALLOCATION.md)
records library allocations separately from wire credits and total RSS.

Known temporary compatibility defect: pinned Proto rejects zero-length NULL or
unknown RDATA, even in an additional section of an otherwise ordinary answer.
Such a response can cause configured fallback or SERVFAIL. Successful preservation
of these valid records is not claimed; the failed probe and minor disposition
remain in [the decision register](../../docs/planning/OPEN_DECISIONS.md#p3-02-forwarding-review-dispositions).
Fixing the dependency profile retains its separate owner/qualification gate.

A missing next checkpoint may wait for one identified maintenance outcome inside
the original deadline. Completion never authorizes output: the same held admission
is checked again. Failed or superseded outcomes refuse, and generic unavailability
never requests another operation. Partial I/O keeps its original future.

A query/reply has2s total: at most1500ms for processing, remaining500ms for bounded
SERVFAIL/write. Absolute deadlines are checked at every future resume. TCP frame,
idle,lifetime/query-count and five-second drain bounds remain as specified in
the contract. Counters count events, not authenticated identities or route proof.

Signal shutdown and await `serve`. Its listener/request JoinSet drains then aborts
and joins async tasks. A separate bounded root registry owns upstream workers,
TCP background handles and inventory helpers until explicit cleanup joins them.
Cleanup cannot extend response acceptance; a timeout retains ownership and reports
`ShutdownIncomplete`. Clock maintenance uses one awaited blocking operation;
`ShutdownIncomplete` explicitly reports a still-running atomic operation. Dropping
the service future cannot claim graceful completion or roll back disk writes.
Public service deployment/port53 packaging is not supplied by this library.
