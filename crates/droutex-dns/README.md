# Bounded legacy DNS library

P3-01 implements the accepted [D47/D48 contract](../../docs/planning/P3_01_LISTENER_CONTRACT.md).
The caller owns a Tokio runtime, an initialized `Deployment`, explicit prebound
UDP/TCP sockets and a shutdown watch channel. `DnsNode::new` consumes the deployment;
`deployment()` allows the host to share it with publication/gateway consumers.
`serve` consumes the DNS owner. Importing configuration binds no socket.

Every query uses actual peer/listener/current D46 admission and one immutable
policy generation, with another current/lease check before submission. Socket
addresses/transport must still match that generation. New bindings need explicit
restart/rebind. Unknown, expired, revoked or unavailable peers get no reply.

The current handler returns SERVFAIL with RA/AD clear. It performs no upstream
resolution, private override, filtering answer, DNSSEC validation, learning or
network egress. The private processor seam exists for controlled wire fixtures
and later P3 handlers; it supplies no public alternate authorization path.

Limits are node-wide:16 listeners,64 TCP connections,512 owned contexts,16MiB
transport wire credits and fixed monotonic intake buckets. D46 group/node quotas
remain shared with every DNS transport. UDP is1232 bytes and at most3x the query;
TCP uses one u16 big-endian frame prefix. Large responses use a valid encoder TC
fallback. Strict parsing accounts for all original EDNS bytes and discards client
options before processing. No ECS/header/query field supplies identity.

A query/reply has2s total: at most1500ms for processing, remaining500ms for bounded
SERVFAIL/write. Absolute deadlines are checked at every future resume. TCP frame,
idle,lifetime/query-count and five-second drain bounds remain as specified in
the contract. Counters count events, not authenticated identities or route proof.

Signal shutdown and await `serve`. Its single owned JoinSet drains then aborts
and joins async tasks. Clock maintenance uses one awaited blocking operation;
`ShutdownIncomplete` explicitly reports a still-running atomic operation. Dropping
the service future cannot claim graceful completion or roll back disk writes.
Public service deployment/port53 packaging is not supplied by this library.
