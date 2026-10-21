# Current legacy DNS and gateway authorization

Status: D46 implementation in progress; actual platform/review/merge gates remain
open. The [accepted profile](../planning/P4_01_BINDING_CONTRACT.md) is authoritative.
P2-06 completed in PR106 before this work. The owner separately accepted
[P4-01 before public DNS transport](../planning/AUTHORIZATION_BEFORE_DNS.md).

## Trusted boundary

The shared gate belongs to the current core Deployment. Static LegacyDnsAssignment,
GatewayNetworkIdentity, compiled scope, D40 intent and D44 simulation remain data,
not live authority. The trusted in-process transport supplies its actual peer IP
and listener; headers, a public ID, latest DNS query and candidate SNI cannot select
an alternate Profile. IPv4-mapped IPv6 is refused until an explicit transport
normalization profile exists. A source grant shares authority among NAT peers.

prepare_legacy_dns derives exactly one current lease/Endpoint/Profile. TCP DNS must
call it for every query. prepare_gateway independently derives its exact group,
checks the canonical target/port/service scope, evaluates the group's real policy
and checks its selected outbound/strict D40 intent. Availability observations are
trusted runtime inputs, not fields from a client. Unknown facts never become DIRECT.

Preparation reserves bounded quota. Gateway rate is charged before target/policy
evaluation, including refused targets; errors release concurrency without returning
rate credits. AdmissionAttempt.consume rechecks the actual current generation,
clock and lease. It linearizes under the current publication read lock. A previously
prepared attempt cannot be consumed after an applied new generation, even if its
old config pin is retained. Non-Clone request evidence owns its quota until Drop;
borrowed scope is still ordinary data and cannot construct another admission.
Gateway evidence retains the exact canonical domain and port, not just a wider
grant. verify_gateway_destination checks that target and current authority again.

The result does not resolve/check an origin address or connect. P4-02 must use the
current gate again at final connect admission and pin only checked safe addresses;
P3-01 must integrate current admission per request. Actual socket races, deadlines,
DNS amplification bounds and TCP stream lifetime remain owning-stage evidence.
Already admitted bounded requests may finish; old evidence never admits a new one.

## Data compatibility and time

Exact v1 canonical configurations/envelopes restore for static history and Explain;
they cannot authorize a runtime request. Strict v2 config requires a manual lease
on each DNS/gateway grant: issuedAt/expiresAt integer UTC seconds,60..86400 seconds
apart, bounded through9999-12-31. No implicit start/renewal or automatic cause exists.
An expired snapshot is historical data; republishing it does not renew expiry.

D11's existing hash/signature/header layout is retained, with authenticated payload
schema2 matching config2. Mixed/unknown versions refuse. v1 vectors remain exact;
older readers reject v2. Key rotation and fresh higher-sequence rollback do not
reset either replay or clock floors. Journal2 adds one clock upper bound, preserving
the existing16KiB atomic Linux transaction. An initialized missing floor cannot be
reprovisioned implicitly. v1 rollback retains an already established clock floor.

The operator must maintain correct host UTC. SystemTime/Instant disagreement over2s
latches new admission unavailable. Each monotonic sample must not regress; the
observed upper time cannot decrease under a tolerated wall-clock correction or
local re-anchor. The whole conservative interval with a2s margin
must lie inside the lease. A durable upper bound covers every admitted5s window.
Restart waits for its conservative lower time to reach that bound; otherwise valid
requests may briefly be denied. CP-independent clock refresh persists the next bound
before use. It performs blocking I/O and requires a bounded blocking context from
future async listeners. The fast path performs no filesystem I/O. Explicit local
re-anchor after repair retains the floor; it is not a public reset API. Corruption
or uncertain commit fails closed and uncertain commit requires reopen. This does
not establish UTC accuracy or defeat a malicious host/whole-volume rollback.

## Quotas and learning

Per-group/node fixed limits are in D46. Group counters are shared across listeners,
addresses and renewals in each plane. Node counters enforce the aggregate. New
groups start empty; publication does not refill existing buckets. Unknown peers
create no per-source map. Current groups plus outstanding retained groups stay
within the existing10000 resource budget; saturation rejects admission/publication.
Between inventory preflight and durable publication, new acquisitions briefly
return Unavailable. Existing reservations may drain. No lock is held during disk
I/O; an RAII guard resumes acquisitions after success or known failure. This
prevents retiring groups gaining outstanding reservations after the inventory check.

automatic_learning returns UnavailableUntilSecureDns. No Boolean, metadata record,
legacy query or fabricated observation is a verification capability. Manual current
authorization is this task's accepted boundary; a real secure-DNS observation
producer, scoped opt-in publication, credential-derived revocation and live TLS
learning tests remain P3-07 after P6-01. Private external management and actor/session
checks remain P6; no public endpoint is added here.
