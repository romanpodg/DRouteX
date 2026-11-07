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
BLOCK returns NXDOMAIN and Profile-local records take precedence over upstream
resolution. Gateway actions remain unavailable. DO/CD are forwarded without
client EDNS options or ECS; AD/AA are cleared and DNSSEC is not validated here.
Local answers are unsigned, so a validating client may reject them. Root DNSKEY
uses the admitted Profile's default action; other root queries remain unsupported.
DO/CD and DNSSEC-type queries bypass the non-DNSSEC cache. RA describes configured
forwarding capability; upstream queries use independent IDs, RD and EDNS0/1232.
This library does not provide a secure endpoint, gateway or public deployment.
Ordinary HTTPS/SVCB answers retain advertised endpoints, ALPN, ECH and IP hints.
SmartDNS redirection with those records is unsupported; clients can use IPv6,
alternative endpoints or QUIC outside any future TCP-only gateway path.

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

## Manual public-origin probe

Run from the repository root with the pinned toolchain:

```sh
cargo run -p droutex-dns --example smartdns_public_probe --features experimental-smartdns --locked
```

The probe requires an unused loopback TCP port 443, `curl` on PATH (`curl.exe`
on Windows), direct DNS access to 1.1.1.1:53 and HTTPS access to example.com:443.
It never changes host DNS, firewall or privilege settings. If port 443 cannot be
bound unprivileged, run on another authorized test host; do not stop an existing
service or grant new permissions for this probe.

Only disposable in-memory configuration and an RFC8032 public fixture signing
key are used. Never reuse this key or configuration for an operational service.
The probe composes the ordinary DNS and experimental gateway libraries with
their shared deployment and maintained clock, uses the actual SmartDNS answer
for curl's destination and keeps the real origin resolver/inventory/numeric dial.
Curl ignores user configuration and proxies, verifies the certificate chain and
example.com hostname, and must receive HTTPS 200. Removing the route in a new
signed generation must then produce SERVFAIL and refuse HTTPS through the cached
gateway address. All started service tasks are signalled and joined after the
checks, including a startup or check failure; cleanup timeout aborts and joins
remaining tasks and reports an error. The negative result also requires a
current-generation `Scope` admission refusal; unrelated curl errors such as
timeouts, CA failures and DNS failures cannot pass the revocation check.

This is an explicitly invoked external probe, not an ordinary offline test.
Require exit code 0 and both PASS markers; record commit, host and command.
It proves only a local IPv4
client path with direct public origin egress and one route-removal case. It does
not prove public gateway reachability, native Debian acceptance, IPv6, protected
proxy egress, restoration, all revocation races or independent parser security.
