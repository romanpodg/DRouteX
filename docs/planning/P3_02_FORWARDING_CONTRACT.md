# P3-02 bounded forwarding contract (D53)

Status: accepted by direct owner reply on2026-10-28 to request
call_b35f42de3b6c42afa31321d7242557b7, selecting the complete proposed profile
at23e291a. Checked PR113 completion, including all11 exact-main40631471007 jobs
and both processed reports, fulfilled its prerequisite before this acceptance.
This authorizes P3-02 implementation and conditional exact-client adoption under
the bounds below. Finish the current PR114 documentation review/merge/main gates
before dependent implementation. P3-02 stays planned until that work starts.

## Accepted outcome and dependency adoption

Implement the existing P3-02 backlog task: an admitted legacy UDP/TCP query whose
same-generation policy selects Resolve reaches its approved upstream group and
returns a bounded answer. All other policy actions remain unavailable to this
handler and send no upstream query. P3-03 cache, P3-04 overrides/block semantics,
P3-06 gateway rewriting, P3-07 secure endpoints, TLS upstreams, full recursion,
learning, public deployment and privileged networking retain their own gates.

Conditionally adopt exactly hickory-net0.26.3 defaults-off/tokio, retaining the
qualified Proto0.26.3/Tokio1.53.3 profiles, lock pins and licenses. Carry the
mandatory D52 ingress/question/frame/deadline/ownership checks into production;
the raw TCP multiplexer is not a qualified replacement. Add no resolver,
recursor, TLS, DNSSEC crypto, OS resolver, vendor patch or new dependency.
Production behavior must independently pass the application tests below before
merge; the qualifier is supporting evidence, not those tests.

Plaintext UDP/TCP upstream traffic provides no on-path confidentiality or peer
authentication. The owner accepted this initial application limit. The unselected
alternative was qualifying encrypted upstreams before application integration.
Later encrypted adoption retains its exact-client/feature qualification gate.

## Immutable upstream resource

Give each existing SnapshotResource::Upstream(DnsUpstreamId) a strictly decoded
UTF-8 JSON payload, at most4096 bytes. Require exactly these fields and reject
unknown/duplicate fields, unsupported versions, empty/duplicate members and
noncanonical numeric socket addresses:

```json
{
  "format": "droutex-dns-upstream-v1",
  "fallback": "none",
  "members": [
    {"transport": "udp_tcp", "address": "76.76.2.2:53"}
  ]
}
```

The address above illustrates encoding; it is not an installed default or a
promise about a provider's future address. A group contains one or two members
in explicit priority order. Transport is udp_tcp or tcp. Fallback is none or
on_failure. A second member requires on_failure. Members have distinct numeric
address/port pairs. No hostname, URL, bootstrap resolver, group reference,
weighted selection, health worker, implicit resolver or custom port is supported.
Only port53 is allowed in the production profile.

CP validates actual payload bytes before signing. DP independently decodes the
authenticated, hash-matching bytes into its immutable compiled Generation.
All referenced groups must exist; malformed or unsupported resources reject the
whole candidate, retaining valid last-known-good state. Retain existing aggregate
resource/table limits and additionally cap groups at64 and members at128 per
generation. Preflight counts and byte limits before collection allocation.
Policy group selection never confers gateway or traffic-egress authority.

Existing pre-forwarding opaque upstream fixture bytes do not become valid v1
resources. Update owning fixtures explicitly. Before any deployment upgrade,
the controller must publish valid v1 resources; do not silently reinterpret old
bytes or discard a failed activation. This contract authorizes no deployment.

## Destination and loop boundary

Permit only explicitly configured public unicast IPv4/IPv6 upstreams. Reject
unspecified, loopback, private/ULA, link-local, shared-address, multicast,
broadcast, documentation, reserved, metadata and IPv4-mapped IPv6 destinations.
Use a documented explicit special-prefix policy with boundary tests for both
families, rather than an incomplete private-address check.

Reject any address owned by the node, any configured listener address and any
explicit node service/public address, irrespective of port. Startup receives a
trusted bounded self-address set: all OS-assigned addresses plus operator-declared
public/NAT aliases, at most256. Enumerating addresses must be unprivileged and
use an existing platform facility; if an additional dependency is necessary,
stop for its approval. A stale/failed inventory disables forwarding. Re-evaluate
the current bounded inventory before each new connect/send; address changes
cannot authorize a previously refused target. Never change host networking.

Validate and pin the exact numeric destination before every new transport
attempt. No re-resolution, OS bootstrap, external name lookup or server-directed
redirect occurs. Responses are DNS data; never connect to answer addresses here.
Detect local loops before I/O. Remote resolver cycles cannot be proven absent;
finite attempts and the original deadline bound their effect. No recursive
chasing of CNAME, referrals or additional-section addresses is implemented.

## Attempts, fallback and response semantics

Use the original held admission/generation and selected immutable group. Recheck
current generation, revocation, lease and clock at every upstream I/O resume and
before each attempt. Reuse D50's single bounded checkpoint wait on the same
reservation; another refusal ends work. No re-pin, new admission or quota reset.
Loss of authority drops/closes without an upstream fallback or client SERVFAIL.

Each member gets at most500ms, including connect, send, receive and validation,
capped by the original1500ms processor cutoff. udp_tcp sends one UDP query; only
a fully validated TC response permits one TCP query to that same numeric target
within that same member deadline. TCP is otherwise one query per connection.
Never retransmit a datagram or reconnect a failed stream. At most two members
and four wire queries per admitted request; no parallel racing or pooling.

With on_failure only, transport error, deadline/budget exhaustion, invalid final
answer, valid SERVFAIL or valid REFUSED may select the next configured member.
NXDOMAIN, NOERROR/NODATA and other valid nontruncated responses are returned;
do not turn a negative answer into implicit fallback. TCP TC is a failure.
An exhausted group returns bounded SERVFAIL only while original authority and
reply time remain. There is no fallback to the OS, another group or traffic DIRECT.

Re-encode the validated original question with an independent library-generated
transaction ID and RD set. Strip all client EDNS options, including ECS; emit a
fresh EDNS0 payload ceiling1232. Preserve question type/class and DNS name
semantics. DO/CD requests return explicit NOTIMP in this initial subset; full
DNSSEC forwarding/validation is unavailable. Responses preserve the client's
ID/question, upstream RCODE and bounded answer/authority/additional data. Clear
AD/AA; RA indicates the configured forwarding capability, not local recursion
or validation. Do not learn identity, routes or authorization from returned data.

Retain D52's4096 accepted-message ceiling,16 UDP datagrams/16 TCP candidates,
65568 TCP input bytes,4098 written bytes and256 successful I/O operations per
direction per transport attempt. Pending polls are free. Delegate framing and
compression to the qualified library and retain complete-frame validation.
Keep P3-01's client UDP min(advertised,1232,3x request) cap and library-encoded TC
fallback; client TCP returns at most4096 bytes from this processor. Oversized
upstream data is an explicit failure, never a silently truncated successful RR.

## Resource ownership and cancellation

Add a forwarding slot ceiling of8/node and2/network group, independent of the
existing64/group and512/node admission ceilings. Reserve without a waiting queue;
exhaustion yields SERVFAIL only with valid admission. Hold one slot through all
attempts and cleanup. Each attempt reserves128KiB of the existing16MiB transport
buffer budget before socket creation, covering its query/scratch/frame/reply
buffers. At most one transport is live for a request at any time.

The4096 acceptance cap does not bound Proto's declared-question vector capacity.
Keep the qualified u16-driven allocation explicit. Before adoption calculate
and record its worst-case allocation with the pinned types and8-slot ceiling,
including library frame/vector overhead; fail implementation review if this
profile cannot demonstrate finite bounded allocation. Do not claim total RSS.

Keep TCP background JoinHandles in a bounded runtime-owned cleanup registry,
never solely inside a cancellable processor future. A failed/cancelled attempt
aborts and joins before a subsequent transport starts. Cleanup may use up to3s
but cannot extend any response-acceptance, processor or reply deadline. On timeout
the runtime retains ownership/slot until cleanup completes; it sends no late
answer and starts no fallback. Registry capacity is8 and follows the same slot;
no extra unbounded queue/worker or emergency abort-only Drop counts as joined
cleanup. Existing5s shutdown cancels and joins this work as well as listeners.

## Required application evidence

Check CP/DP strict resource decoding, actual hash/generation pairing, aggregate
limits, valid-LKG retention and old opaque-byte refusal. Exercise exact public/
special/self-address boundaries, wildcard listeners, public/NAT aliases, inventory
failure/change and pinned numeric connect. Fixtures may inject an exact private
loopback destination only through private test-only construction; production
validation still rejects that payload, with a separate regression.

Run real independent UDP/TCP IPv4/IPv6 forwarding for A/AAAA/CNAME/TXT/HTTPS/SVCB
and the qualified auxiliary types; client ID/question and TC bounds; exact
fallback order, all listed RCODE outcomes, no fallback when disabled, no OS lookup,
loop refusal, oversized/malformed/foreign responses and exact I/O ceilings.
Test generation replacement/revocation/expiry/clock failure during every attempt
and cleanup, shared deadlines, resource exhaustion, cancelled partial connect/
read/write, joined shutdown/rebind and no query/response after authority loss.

Keep all existing tests and meaningful negative/mutation evidence. Require actual
Windows/Linux default/no-default profiles, all dependency bundles, implementation
and final linked-account Code Reviews, normal exact-head merge, exact-main checks
and both processed reports. P3-02 completes only after its own acceptance proof.

## Control D reference and explicit departures

Verified2026-10-28 against Control D's official
[ctrld configuration](https://github.com/Control-D-Inc/ctrld/blob/main/docs/config.md):
ctrld supports endpoint/bootstrap fields, ordered policy upstreams and optional
RCODE failover. Its documented service option can use network DNS after failure.
Those are vendor facts, not adopted DRouteX behavior. This accepted profile uses
numeric public destinations, fixed deadlines, explicit group-only fallback and
no network-DNS escape. Plaintext-only, no private upstreams, the small forwarding
ceiling and unavailable DNSSEC are deliberate owner-accepted initial limits.
It preserves the accepted separate Profile/Endpoint and gateway authorization model.
