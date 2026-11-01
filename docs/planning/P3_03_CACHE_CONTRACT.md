# P3-03 scoped TTL and negative cache contract (D54)

Status: **accepted by direct owner reply on 2026-11-01**. The owner accepted
all proposed choices and explicitly requested D54 implementation and Project/Roadmap
updates. Runtime implementation is merged; exact-main acceptance remains open.
No new dependency, vendor patch or deployment is authorized.
The [application evidence](P3_03_CACHE_REVIEW.md) and
[allocation audit](P3_03_CACHE_ALLOCATION.md) track actual implementation gates.

## Baseline and authority

GitHub `main` was read on 2026-11-01 and resolves to
[`8cf65d43181c031c9adbd51d0bca8e648c438afc`](https://github.com/romanpodg/DRouteX/commit/8cf65d43181c031c9adbd51d0bca8e648c438afc),
identical to the clean local checkout before these documentation changes. Git
transport was unavailable through the configured proxy; the GitHub connector
independently supplied the current branch SHA. No remote runtime checks were
rerun or newly passed by this analysis.

Authority: [AGENTS](../../AGENTS.md), [roadmap](DEVELOPMENT_ROADMAP.md),
[execution card](EXECUTION_STATUS.md), [canonical P3-03 task](backlog.json),
specification [DNS engine](../../DRouteX_Project_Specification_EN.md#9-dns-engine)
and [testing](../quality/TESTING_STRATEGY.md). P3-03 depends on completed P3-02;
its acceptance requires Profile isolation, correct TTL/negative expiry and a
memory bound, with virtual time, concurrent scopes and generation transitions.
The [D53 forwarding contract](P3_02_FORWARDING_CONTRACT.md) and
[completed application evidence](P3_02_FORWARDING_REVIEW.md#completed-implementation-and-normal-merge)
remain binding. Their recorded checks establish the prerequisite; they are not
P3-03 acceptance evidence.

Relevant current callers and constraints:

- [runtime.rs](../../crates/droutex-dns/src/runtime.rs) admits each request, retains
  its reservation, imposes the original processor/reply deadlines and checks
  authority again at client I/O. A cache hit must follow this same path.
- [wire.rs](../../crates/droutex-dns/src/wire.rs) rejects unsupported requests,
  selects policy from the admitted binding, sanitizes headers/question/EDNS and
  enforces the client transport ceiling. Lookup belongs after these request and
  policy checks, before upstream work and before client-specific truncation.
- [forward/mod.rs](../../crates/droutex-dns/src/forward/mod.rs) returns a `Message`
  for both upstream data and synthesized SERVFAIL. It currently exposes neither
  the supplying member nor the response receipt time. Cache insertion needs a
  private typed success/provenance result, not inference from arbitrary Messages.
- [admission/io.rs](../../crates/droutex-core/src/deployment/admission/io.rs)
  checks the actual current generation, original owner, lease and clock. Cache
  keys and remembered public IDs never substitute for these checks.
- The [allocation audit](P3_02_FORWARDING_ALLOCATION.md) shows that a 4096-byte
  packet can have a much larger decoded allocation. Keeping decoded `Message`s
  indefinitely would not establish the requested cache-memory bound.

## Accepted decisions

| Choice | Recommended complete profile | Tradeoff / alternative requiring a different contract |
|---|---|---|
| Isolation | Exact Endpoint + Profile + authorized network group + generation; separate mode/validation domains | Less reuse than Profile-only or shared upstream caching, with a simpler disclosure boundary |
| Storage and TTL | Complete sanitized response template, at most 4096 wire bytes; earliest RR expiry, positive maximum 3600s, no positive minimum | An RRset cache could retain unrelated longer-lived data, but needs reconstruction and additional credibility rules |
| Negative answers | SOA-backed NXDOMAIN/NODATA, maximum 60s, exact question key; bypass alias-bearing negatives | Cross-type NXDOMAIN reuse and alias-negative reconstruction are deliberately deferred |
| Capacity | 16MiB persistent cache storage and 2048 entries per runtime; 2MiB and 256 entries per isolation scope | Fixed initial limits; configurable budgets and workload tuning remain later work |
| Concurrency | Independent misses through existing D53 slots; no request coalescing; nonwaiting cache lock and bounded FIFO eviction | Duplicate misses can consume the existing slots; coalescing would add shared authority/cancellation ownership |
| Generation | Logical invalidation on every applied generation, one shared capacity budget, cold restart | No reuse across semantically equivalent publications or rollback |
| Known defect | Never cache decode failures or generated failure responses; retain the temporary empty-RDATA disposition | Repair needs a separately accepted exact vendor fix/profile and qualification |

The owner accepted these choices together. D54 does not resolve D09's
BLOCK/DNSSEC/redirect choices or change the existing library qualification profile.

## Key, scope and data provenance

Use a typed equality key with these mandatory components:

```text
K = (node + snapshot ID + publication sequence,
     canonical DNS question name, QTYPE, QCLASS,
     configured upstream group ID + authenticated resource content hash,
     EndpointId, ProfileId, authorized NetworkGroupId,
     response mode, validation profile)
```

Derive every scope/generation/upstream component from the same held
`AuthorizedRequest`, its `Decision`, selected policy and authenticated resource
bytes. Never accept them from headers, EDNS, a query parameter, another request
or caller-supplied public IDs. Generation identity uses the exact authenticated
snapshot metadata, not just a configuration revision or a pointer address.
Retain no strong Generation, Deployment, reservation or credential in an entry.

Normalize question names by DNS label semantics: ASCII case-insensitive labels
and an unambiguous root terminator, within the admitted name bounds. Preserve
the relationship to the original forwarded question; do not merge distinct wire
names merely because a broader Unicode/IDNA transformation maps them alike.
QTYPE and QCLASS stay distinct; only the current D53 IN-class subset can enter.

The only enabled mode is policy-selected forwarded Resolve, with validation
profile `D53 matched response, no DNSSEC validation`. Matching transaction,
question, frame and peer is not cryptographic DNSSEC validation. Locally generated
overrides, BLOCK/SmartDNS rewrites, independently validated DNSSEC data and
policy-independent gateway-origin resolution belong to distinct typed domains
with no enabled insertion/lookup path here. Later tasks must preserve this
separation before enabling those producers. DNS cache contents grant no gateway
permission and are not a source for origin authorization or secure IP learning.

Client transaction ID, question spelling, RD, transport and EDNS payload size
are not reuse dimensions: reconstruct them from each current request and apply
the existing sanitizer and UDP/TCP size rules. No client ECS/options enter the
template. AD/AA stay cleared; DO/CD and unsupported EDNS versions follow the
existing explicit NOTIMP/BADVERS path before any lookup. No future validation
state can be populated by copying an upstream AD flag.

Cache at the configured group boundary, not a freely selected member boundary.
Retain the actual successful member index/transport as private provenance. A
fully accepted response from an explicitly permitted fallback member may satisfy
subsequent exact-key requests until expiry, even if the primary has recovered.
A miss retains D53's original member order. A hit performs no health probe or
new socket operation; current destination/inventory checks remain mandatory for
every actual new upstream I/O. Group content/fallback changes cannot reuse the
old entry because both resource hash and generation differ.

## Eligibility, TTL and response reconstruction

Store one complete response template, not independently reusable additional
records or a client-truncated packet. Insert only after D53's full candidate
validation, non-TC completion, sanitizer and lossless bounded encoding checks,
and successful owned transport cleanup. Carry the successful candidate's
monotonic receipt time through cleanup, so cleanup cannot restart its TTL.
Do not add a DNS wire parser, resolver, CNAME fetch, referral chase or dependency.

Eligible positive responses are NOERROR with a nonempty answer section and no
SOA in authority. This conservative restriction prevents alias-bearing NODATA
from being admitted through the positive path. A CNAME-only response may be
cached as the exact forwarded response; it does not
prove NODATA and causes no local chasing or new key for its target. Retain all
answer/authority/additional records together. Uncacheable valid data still takes
the ordinary D53 response path; cache admission never makes a forwarding failure
out of an otherwise permitted response.

For real RRs in all three sections, treat any TTL whose high bit is set as TTL 0
per [RFC 2181 section 8](https://www.rfc-editor.org/rfc/rfc2181.html#section-8).
OPT flags/version/payload fields are not TTLs. The positive lifetime is:

```text
Lpositive = min(3600s, every retained real RR TTL)
expires_at = successful_candidate_received_at + L
```

Any zero TTL, elapsed lifetime, checked-arithmetic failure or inconsistent clock
sample prevents insertion/reuse. Do not raise a TTL, set a minimum, refresh age
on a hit, prefetch or serve stale data. Expire the whole template at its earliest
RR expiry, including CNAME links and additional data; never silently remove an
expired section and present a reconstructed success.

On a hit, decode the bounded template using the pinned library, reconstruct the
current question/ID/headers/EDNS, and set every real RR TTL to at most
`floor(expires_at - now)` seconds. This deliberately shortens longer TTLs to the
packet's shortest remaining lifetime. The SOA MINIMUM field remains RDATA and is
not rewritten as a countdown. Recheck freshness before encoding and first client
submission; pending reply I/O is additionally capped by this entry's expiry,
without extending the original request deadline. Expiry before submission can
become an ordinary miss only while original authority and processor time remain;
after reply I/O starts, expiry drops/closes rather than starting another response.
Subsecond remaining life may be returned as TTL 0 before actual expiry.

TTL ceilings govern stored templates and cache hits. An uncached/miss response
retains D53's upstream TTL behavior; this proposal adds no user-facing TTL override.
Cache insertion failure, contention or capacity exhaustion leaves that response
eligible for ordinary delivery. No persisted cache or warm restore is supported.

## Negative caching and failure exclusions

Cache NXDOMAIN or NOERROR/NODATA only with an empty answer section and exactly
one IN SOA in authority whose owner is the question name or a DNS-label ancestor.
Reject cache admission for missing/unrelated/ambiguous SOA, authority NS/referral
shapes and contradictory structures. Alias-bearing NXDOMAIN and alias-bearing
NODATA bypass this initial negative cache; forward their permitted response
unchanged. An empty NOERROR answer alone is not proof of cacheable NODATA.

```text
Lnegative = min(60s, SOA.TTL, SOA.MINIMUM, every retained real RR TTL)
```

Use the SOA lifetime rule from
[RFC 2308 sections 3–5](https://datatracker.ietf.org/doc/html/rfc2308#section-5).
Apply the same receipt timestamp, high-bit TTL handling, zero/expiry checks and
whole-response countdown as for positive entries. The authority SOA is retained
and its emitted TTL cannot exceed the remaining negative lifetime. There is no
default invented TTL for a response without a usable SOA.

Both negative kinds use the full exact key, including QTYPE. This deliberately
forgoes RFC 2308's cross-type NXDOMAIN reuse recommendation; it avoids synthesizing
answers to a different question. No NXDOMAIN cut, descendant inference, aggressive
DNSSEC denial synthesis or negative sharing across Profiles/upstreams occurs.
Positive and negative values occupy the same key space and cannot coexist for
one exact key.

Never cache SERVFAIL, REFUSED, FORMERR, NOTIMP, BADVERS, other error RCODEs,
transport/deadline/budget errors, truncated/malformed/foreign responses,
authorization failure or locally synthesized failures. Failure caching is outside
this bounded forwarding profile; no full recursive-resolver compliance is claimed.
The empty-RDATA defect is covered by
[its existing disposition](OPEN_DECISIONS.md#p3-02-forwarding-review-dispositions):
legal empty NULL/unknown data in any section can still trigger configured fallback
or SERVFAIL. Never reinterpret that failure as NODATA/NXDOMAIN, store a failure
sentinel, strip the record or introduce a vendor patch. A separately validated
successful fallback response may be cached normally; it does not prove preservation
of the failed primary response. Keep the failed preservation evidence failed.

## Persistent storage, eviction and bounded work

One cache belongs to one DNS runtime/Deployment owner, across all listeners and
applied generations. Do not make a fresh budget per generation, scope or transport.
All positive/negative data share these initial hard ceilings:

| Resource | Maximum |
|---|---:|
| Persistent requested allocation bytes, including keys/indexes/metadata/capacity slack | 16MiB/runtime |
| Entries, including expired/retired entries awaiting reclamation | 2048/runtime |
| Persistent entry storage and entries for one Endpoint/Profile/network-group scope, across generations | 2MiB and 256 |
| Complete encoded response template | 4096 bytes |
| Cache scans per lookup or insertion | One pass over at most 2048 entry slots |
| Cache-owned maintenance tasks, miss queues and shared-waiter lists | 0 |

Use compact encoded templates and bounded inline keys/metadata or an equivalently
auditable capped allocation strategy using existing dependencies. Account actual
allocated capacity, fixed indexes and replacement/growth overlap before allocation;
`wire_len`, entry count or a claimed approximate byte weight is insufficient.
Reserve fixed/index costs inside the global ceiling and charge per-entry storage
to its scope. Failure to fit skips insertion. Entry decoding is transient and
never occurs while holding the cache lock; never retain decoded library objects
as long-lived entries or unlimited `Arc` copies of evicted values.

Copy at most one 4096-byte template per active request into already reserved
transport-buffer credits. At most the existing 512 request contexts can retain
such copies. Decode one candidate at a time with q=1 and the existing library;
continue to respect D53's complete reply ceiling and runtime context bounds.
Before adoption, extend the allocation audit for cache encode/decode/reply copies,
index overhead and transition/replacement overlap. The 16MiB limit is persistent
cache storage, not a total RSS or decoded-memory bound. P3-02's much larger
source-derived transient envelope is not repaired by introducing a compact cache.

Use bounded FIFO insertion order, without refresh/promotion on hits. Reclaim
expired/retired entries first. If a scope limit would be exceeded, evict its oldest
entries before considering global FIFO eviction for the node limit. Compute the
victim set within one bounded pass and perform no repeated full scans per victim.
Concurrent insertions of the same key keep an existing fresh entry unchanged;
the later completion does not replace its value or restart its age. On FIFO counter
overflow clear/rebase within the same finite storage bound, without changing TTLs.
These are resource-isolation limits, not a throughput/fairness SLA.

## Concurrency, admission and lifecycle

Use a short synchronous nonwaiting lock for lookup/copy/accounting/commit only.
Contention is a cache bypass, not an async waiter queue. Encoding, decoding,
policy evaluation, authority checks and I/O take place outside the cache lock;
no lock guard spans await or blocks publication. A poisoned/unavailable cache
cannot supply data; use the normal admitted forward path and an allowlisted
internal error indication. Do not bypass an admission refusal as a cache error.

There is no singleflight, background refresh, health worker or new miss semaphore.
Each miss owns its original reservation, generation, deadline, upstream slot and
cleanup. Duplicate concurrent misses are permitted. Existing 8/node and
2/network-group forwarding slots, 64/group and 512/node admission limits and
the 16MiB transport budget remain in force. A hit consumes admission/rate/context
and reply-buffer resources, but requires no upstream slot or socket.

Recheck actual current authority before consuming cached data and again at every
client I/O resume, with the existing bounded D50 checkpoint behavior. A completed
checkpoint is not authority. Recheck original authority before insertion; an
already retired result is not a cache producer. No re-admission, generation re-pin,
quota refund, fallback or SERVFAIL after authority loss. Cache work consumes the
original 1500ms processor budget; the reply reserve stays inside the original
2000ms request budget. Misses retain D53's 500ms/member, at most two members/four
wire queries, finite 3s owned cleanup and 5s runtime shutdown.

Cancellation cannot publish an incomplete template. Commit a fully built entry
atomically under its accounting lock or skip it. Cancelling a cache-hit request
releases its copy/credits/context and does not invalidate other legitimate readers.
Cancelling a miss retains all existing upstream ownership/join requirements.
Shutdown frees cache storage after its request owners finish; no detached cache
task or serialization step extends shutdown.

## Generation changes and last-known-good state

Every successfully applied new snapshot produces different keys, even if Profile,
upstream bytes or rule effects are identical. There is no grace reuse, migration
or stale answer in the new generation. A signed higher-sequence rollback is also
a new generation and starts cold. A failed candidate that leaves the same usable
LKG active does not invalidate that LKG's still-fresh entries. CP/database loss
does not itself flush valid LKG data or prolong TTLs/leases. Runtime restart is cold.

Physical reclamation may happen lazily inside the fixed global budget. An old
in-flight writer cannot overwrite a newer key, roll back cache generation state
or remove newer entries in a generation-wide clear. If an activation races after
a live check, an old-key insertion may be retained only as unreachable retired
data within the same capacity ceiling; it cannot become a hit for new work.
The authoritative generation check at consumption/reply still refuses the old
request. Cache state is never itself the publication authority.

## Required application evidence

1. Virtual monotonic time: exact expiry, fractional ages/TTL 0, high-bit/zero TTL,
   unequal RRset TTLs, CNAME/additional earliest expiry, SOA TTL/MINIMUM, arithmetic
   refusal, no hit refresh or wall-clock resurrection, time spent in cleanup.
2. Isolation matrix: same question across Endpoint/Profile/network group, upstream
   ID/resource hash, mode, validation profile, QTYPE/QCLASS and generation;
   mixed case/root spelling reuse with the current client's ID/question. Future
   local/validated/origin domains cannot be reached through enabled entry points.
3. Negative cases: NXDOMAIN versus NODATA, SOA owner label boundaries/root,
   absent/multiple SOA, referrals, alias-negative bypass and A/AAAA independence.
   Test every excluded failure with repeated queries and actual upstream counts.
4. Capacity/concurrency: exact byte/count boundaries, scope-before-global FIFO,
   expired entries, maximum-size keys/data, allocation/replacement overlap,
   bounded scans, lock contention, duplicate-key races, cancellation and resource
   exhaustion. Counters must not underflow or evade limits during eviction.
5. Publication/admission races: warmed cache followed by policy/upstream change,
   rollback, rejected activation, LKG/CP loss, revocation/lease expiry/clock refusal
   before lookup, during decode/checkpoint, before insertion and at client I/O.
   Hold old writers across new inserts; prove no stale hit, late I/O or quota reset.
6. Real controlled UDP/TCP IPv4/IPv6 integration: cold miss then hit with no extra
   upstream query, expiry then miss, actual fallback member warm reuse, different
   client EDNS/IDs, UDP 1232/3x truncation followed by a full TCP cache hit, original
   deadlines, owned cleanup and shutdown/rebind. Preserve the D53 record matrices.
7. Reproduce legal empty NULL/unknown data in answer/authority/additional sections
   of ordinary positive and NXDOMAIN replies. Show bounded fallback/SERVFAIL and
   repeated upstream work for an uncached failure, plus normal caching of a valid
   fallback response. These remain defect-characterization tests; do not relabel
   them preservation passes or widen the existing minor disposition silently.

Apply [QUALITY_STRATEGY](../quality/QUALITY_STRATEGY.md): proportionate Q-DOC for
this proposal; after acceptance, Q-RUST and actual Q-LINUX component checks on
Windows/Linux default/no-default profiles, dependency bundles, current-head actual
implementation/final linked-account reviews, ordinary merge, exact-main checks
and processed reports. Required integration without evidence keeps P3-03 open.
No native full-product/deployment acceptance is inferred from component fixtures.
Reassess the existing forwarder/cache/transport maintenance dispositions when
touching their code, and retain failed CodeScene/compatibility results accurately.

Expected future implementation touches `src/cache` plus existing DNS processor,
forwarder provenance and request/reply integration, focused fixtures/tests and an
updated allocation/evidence document. No current config/snapshot codec, upstream
resource schema, library pins/features or new dependency changes are proposed.
The planning authorization validator now enforces the accepted D54 boundary;
later tasks remain planned. Acceptance is recorded above before runtime changes.

Proposal-only validation on 2026-11-01: the canonical plan was rendered;
`python3.12 tools/verify_planning.py --self-test` passed all nine negative cases,
88 tasks, the 61-task MVP closure and preservation of both source specifications.
`python3.12 tools/test_planning_tools.py` passed all 20 tests. Changed local paths
and heading anchors, generated-output consistency and whitespace were checked.
These are documentation checks only; no Cargo, network fixture, runtime review
or P3-03 acceptance pass is claimed.

## Control D reference and deliberate differences

Verified 2026-11-01 against official
[ctrld configuration](https://github.com/Control-D-Inc/ctrld/blob/main/docs/config.md)
and [Control D TTL overrides](https://docs.controld.com/docs/ttl-overrides).
ctrld documents upstream-TTL caching, a record-count capacity, an optional TTL
override and optional stale serving. Control D exposes separate Profile TTL
options for bypass/block/redirect and notes that changing them cannot clear
answers already cached by clients. These are vendor facts, not evidence of its
internal cache key or tenant isolation.

The accepted DRouteX profile adds explicit byte/scope bounds, shortens cache-hit
TTLs, excludes stale serving/TTL inflation and starts cold after publication.
It adds no Profile TTL controls. BLOCK, redirect and DNSSEC semantics remain
P3-04/P3-05/P3-06 decisions. Client/OS/application caches and existing connections
can outlive a server publication; flushing this runtime cache cannot revoke them.
