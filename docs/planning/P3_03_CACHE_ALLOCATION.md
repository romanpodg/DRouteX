# D54 cache allocation and lifetime audit

Status: implementation audit, 2026-11-01. This extends, rather than replaces, the
[D53 forwarding audit](P3_02_FORWARDING_ALLOCATION.md). It accounts for requested
persistent cache storage separately from transient decoded objects, allocator/OS
overhead and total RSS. The [accepted D54 contract](P3_03_CACHE_CONTRACT.md) and
[application evidence](P3_03_CACHE_REVIEW.md) own the behavior and actual checks.

## Persistent storage

`cache::Cache` owns exactly one `Mutex<Option<State>>` and an aggregate error
counter. State owns one preallocated vector of 2048 `Option<Entry>` slots. Each
slot contains its key, 4096 inline template bytes, receipt/expiry instants, supplying
member/transport and FIFO order. There is no per-entry Box/Vec/String/Arc, hash
table, expiry heap, scope map, async queue or worker. The arena is shared across
listeners, scopes, both answer kinds and every applied generation.

Construction uses fallible exact reservation. Before initializing slots it checks
`size_of::<Cache>() + actual_vec_capacity * size_of::<Option<Entry>>() <= 16MiB`
with checked arithmetic. An allocation/capacity failure disables this optional
cache; admitted forwarding remains available. Slot insertion needs no heap growth,
and replacement drops the old inline value. Empty, expired and retired slots all
remain inside that same fixed allocation. No reset creates another arena.

The pinned Linux x86_64 run measured:

| Quantity | Requested bytes |
|---|---:|
| Cache control structure | 48 |
| Inline key / template | 424 / 4152 |
| UpstreamAnswer / Outcome / result payload | 184 / 184 / 184 |
| One complete optional entry slot | 4584 |
| 2048-slot arena | 9,388,032 |
| Persistent total including control structure | 9,388,080 |
| 256 occupied slots charged to one scope | 1,173,504 |

The constructor accounts the actual platform sizes/capacity, not these constants.
Tests assert the 16MiB global and 2MiB scope limits with actual `size_of` values;
mandatory Windows CI must independently establish its layouts and results.
Shared control/vector costs are charged globally; each scope is charged for every
occupied full slot, including inline slack. Scope admission checks both 256 entries
and `(scope_count + 1) * slot_size <= 2MiB`. A fixed-size insertion requires at
most one victim: scope FIFO first if the scope is full, otherwise an empty slot
or global FIFO victim. One scan accounts and chooses that victim. FIFO exhaustion
clears/rebases during that same scan; it does not allocate or reset any TTL.

## Transient decoded data

Lookup copies at most one inline 4096-byte template per active request, after
reserving 4096 bytes of the existing transport credits. Decode takes place after
releasing the nonwaiting cache lock. There are at most the existing 512 request
contexts; no escaped shared entry or generation owner survives eviction.

Stored frames are internally produced, q=1 and at most4096 bytes. D53's bound
`R = D(4096,1) = 16,842,840` still bounds a decoded copy; wire length alone is not
its heap bound. Cache-hit decoding supplies the request's retained response in
the existing `512*(2*R+E)` client term. The ordinary final encoder/check roundtrip
supplies its other copy. A cold response remains in its existing request context
while cache insertion serializes it; insertion adds no retained decoded Message.

Conservatively add one extra cache encoder envelope per client context:

```text
extra cache encoder envelope <= 512*E = 67,108,864 requested data bytes
D53 data envelope + extra <= 19,863,061,048 requested data bytes
```

This intentionally loose formula covers encoding/compression work, including the
bounded staging Vec. The copied template bytes are charged to existing16MiB wire
credits, not a new unbounded budget. Cache keys/templates also have bounded inline lifetimes in request futures
and on the stack. Executor-owned future storage is finite under the existing512
context ceiling; this is not a stack-only or zero-allocation claim. Fixed
task/future/channel/control metadata, thread stacks, allocator internals
and kernel buffers remain outside the numeric data formula. No actual peak/RSS or
host-sizing claim follows from it. D54 does not repair D53's large transient bound.

The registry still owns only8 workers. Its result is now a private typed Outcome
with a Message and finite upstream provenance, instead of a bare Message. It
adds no sender, queue, worker, generation pin or reservation. Runtime contexts and
completion oneshots remain capped at512; concrete payload sizes are printed by
the cache allocation regression for platform evidence. Layout/pin/context/queue
changes require recalculation and current-head review before adoption.

## Time and authority ownership

The I/O adapter remembers the first successful receive time for each transport.
For TCP this can include the frame prefix; foreign candidates can make it earlier
than the chosen candidate. This is a conservative receipt floor: library queuing,
validation/checkpoint waits and joined cleanup can shorten cache life but cannot
restart or extend it. Each transport/member has its own floor; a fallback does
not reuse the failed member's time or identity.

A hit carries its absolute expiry through the private reply type. Before encoding,
the runtime caps all real RR TTLs at the remaining lifetime at the latest permitted
write time, and caps every subsequent client submission/resume by that expiry and
the original deadline. Checkpoint/partial-write delays therefore cannot inflate
the advertised lifetime. Near expiry a hit can emit TTL0; it cannot start/resume
reply I/O at or after expiry.

Authority checks occur outside the cache lock before lookup, after a decoded hit,
before insertion and at actual client I/O. Cache entries contain snapshot metadata
and public typed IDs, not strong Generation/Deployment/reservation references.
The actual original authorization check remains authoritative during publication,
revocation, lease/clock refusal and cancellation. Rejected candidates preserve
usable LKG entries; higher-sequence rollback creates distinct cold keys.
