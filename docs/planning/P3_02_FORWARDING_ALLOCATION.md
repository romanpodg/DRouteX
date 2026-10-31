# D53 forwarding data allocation audit

Status: conservative source-derived envelope for the exact pinned x86_64 profile.
This is not measured peak memory, a RAM requirement, allocator overhead or total
RSS. It does not equate128KiB wire credits with the library's complete heap use.
The owning [application review](P3_02_FORWARDING_REVIEW.md) records platform and
review acceptance. Changing pins, type layouts, queue/slot/context limits or
decoder behavior requires recalculation before adoption.

## Pinned sources and measured layouts

Rust1.99.0 commitb940084d7eb6a299eb4bfeb8e34901bc051e7ac4 supplies the reviewed
[RawVec growth](https://github.com/rust-lang/rust/blob/b940084d7eb6a299eb4bfeb8e34901bc051e7ac4/library/alloc/src/raw_vec/mod.rs)
and [BTree node layout](https://github.com/rust-lang/rust/blob/b940084d7eb6a299eb4bfeb8e34901bc051e7ac4/library/alloc/src/collections/btree/node.rs).
The former's doubling permits charging old/new growth overlap conservatively.
The latter has11 key slots and12 child slots. No rust-src component or dependency
was installed for this audit. The reviewed public source blobs are respectively
5d4ad3ac4bf98b49c3bdaedefa9c791095799d50 and84dd4b7e49def6126f665e62df59f7902b62413c.

Exact hickory-proto/net0.26.3 and futures-channel0.3.34 registry sources were
inspected: `op/message.rs`, `op/dns_response.rs`, `rr/domain/name.rs`, `rr/record.rs`,
`rr/record_data.rs`, reachable `rr/rdata/` decoders, `serialize/binary/encoder.rs`,
`xfer/dns_multiplexer.rs`, `xfer/dns_exchange.rs`, TCP framing and channel queues.
Their qualified archives and notice hashes remain in the
[client inventory](../operations/DNS_CLIENT_DEPENDENCY_INVENTORY.csv).

The application regression measures these size/alignment pairs on pinned Windows
x86_64 and asserts them in both mandatory CI platforms:

| Type | Size bytes | Alignment bytes |
|---|---:|---:|
| Query |88|8|
| Record |272|8|
| Message |152|8|
| DnsResponse / Result<DnsResponse, NetError> |176|8|
| RecordType |4|2|
| EDNS (code, option) / SVCB (key, value) tuple |40|8|
| NetError |88|8|

The measured declared-question reservation alone is
`Q = 65535 * 88 = 5,767,080 bytes`. Multiplying only Q by8 gives46,136,640 bytes;
that number is not a complete forwarding bound.

## Decoder and encoder terms

For a frame with B wire bytes and declared q questions, use the intentionally
loose requested-data envelope `D(B,q) = 88q + 4096B + 65,536`:

- Message eagerly reserves the u16 question count. Successfully decoded questions
  consume at least5 wire bytes each. A malformed count may reserve Q transiently
  before the decoder rejects it.
- Each Name has at most255 encoded bytes. Charge1024 heap bytes including both
  TinyVec spills and old/new vector growth overlap. A record consumes at least11
  wire bytes; decoded question/record/name counts therefore have finite byte
  bounds even when names use compression.
- Record sections initially reserve at most8 entries and then grow. Charge
  `6528 + 816 * record_count` across the three sections, including overlap.
- RDLENGTH is checked against remaining frame bytes before allocating RDATA.
  Reachable TXT/ALPN/OPT/SVCB vectors, strings, boxed slices, raw unknown bytes
  and EDNS conversion copies fit a conservative256 bytes per consumed wire byte.
- CSYNC additionally stores original encoding and a RecordType BTreeSet. Charge
  at most160 bytes per node and at most`2 * inserted_types + 2` nodes, including
  split overlap. At most8 type insert attempts occur per bitmap byte. No extra
  recursive answer resolution or DNSSEC crypto allocation is enabled.

These terms fit below4096B; the additive64KiB covers small eager vectors,
partially decoded names and container slack. This is a source-derived upper
bound, not a claim that independent maxima are simultaneously attainable.
In particular, names cost at most`1024*(q_actual+3*r_actual+3)`, and CSYNC costs
at most`2561B+320*r_actual`. With `q_actual<=B/5` and `r_actual<=B/11`, all variable
terms total less than3410B. This leaves more than686B margin under4096B. Eager
record vectors6528 plus partial names3072 and bounded inline/fresh-query terms
fit the additive64KiB. The checked source anchors are Message lines425/677,
Record338–343, RecordTypeSet200/223 and Encoder273/466 in the pinned archives.

The encoder has at most64 compression candidates with at most255 suffix bytes,
bounded candidate-vector growth,127 label offsets and at most65535 output bytes.
Charge `E = 131,072 bytes` per encoder, separately from a decoder copy. Decoded
CSYNC retains its original encoding, avoiding reconstruction of its bitmap tree
during emission. Canonical accepted responses have q=1 and B<=4096; use
`R = D(4096,1) = 16,842,840 bytes` for their retained/round-trip copies.

## Concurrent ownership

Only8 forwarding workers can hold upstream transports. TCP's channel buffer8
plus its single sender permits9 queued responses, with one worker candidate and
one current background decoder:11 frame/decoder lifetimes. The sum of successfully
received frame bytes is bounded by `T = 65,568` per transport. Charge successful
question vectors by `ceil(T/5)` and one additional transient Q reservation for
the current malformed decoder. A partial current TCP frame can already reserve
65535 bytes before those bytes arrive.

During a UDP-TC to TCP assignment, the old validated UDP TC response may remain
alive until the TCP future returns. Also charge a canonical validation/encoding
decoder copy. The conservative per-worker data bound is therefore:

```text
A = Q + 88*ceil(T/5) + 4096*T + 11*65,536 + 65,535 + 2*R + 2*E
  = 310,221,895 bytes
8*A = 2,481,775,160 bytes
```

Reaping a worker releases its forwarding slot before a client finishes writing.
Existing request contexts can therefore retain completed clean responses outside
those8 slots. Account separately for up to512 contexts, each with a response,
an encoder round-trip copy and encoder storage:

```text
C = 512*(2*R + E) = 17,314,177,024 bytes
8*A + C = 19,795,952,184 requested data bytes
```

The deliberately large, non-tight result demonstrates finite data allocation
under the accepted profile. It is not an attained peak or a host sizing estimate.
Existing client input/wire storage remains separately bounded by P3-01's16MiB
credits. Fixed task/channel/socket/registry metadata and OS/allocator costs are
separate from this data formula; bounded ownership counts are reviewed with the
application lifecycle evidence. No claim of a complete process-memory bound is made.

The control metadata counts are independently finite:8 registry workers/owners,
at most8 TCP background handles and one active library request per TCP worker.
Each has three bounded library channels. Only one outbound item can be published
per exchange/write channel; the response channel has at most9 queued items.
Including stub/transient nodes, charge at most12 response nodes of at most192
bytes each, and4 nodes per outbound channel with its concrete element size plus
pointer/tag fields. Sender counts are at most2 for requests and1 each for writes
and responses. Cloning does not create an unbounded sender history.

There are at most8 inventory workers, scoped readers and children, one Probe per
attempt, two role Wakers and at most one checkpoint/cancellation future per Live,
512 result oneshots/client contexts and one shutdown cleanup coordinator. Their
fixed control-object byte total is outside the numeric DATA formula, as are OS
thread stacks, socket buffers and allocator internals. The separate standards
pass found no omitted enabled binary RDATA family. Linux's current-head default/
no-default CI asserts the same layouts and passes both full workspace profiles.
