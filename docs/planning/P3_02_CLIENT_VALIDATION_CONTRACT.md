# Accepted client-validation boundary for D51 qualification (D52)

Status: accepted by direct owner reply on2026-10-24 (request call_4629a212e9ed46b4ba18deb5d0bcfd02).
The owner selected qualification with mandatory limiting DRouteX adapters.
This accepts the complete tests-only profile below, not production adoption.

## Observed boundary

On 2026-10-24 the exact approved hickory-net0.26.3 defaults-off/tokio profile
passed independent IPv4 UDP and TCP answer/cleanup probes. The mandatory raw TCP
question-matching probe failed: its low-level DnsMultiplexer delivered a parsed
response with the current transaction ID but question foreign.test instead of
example.test. The controlled connected peer sent those bytes. This establishes
a mismatch with DRouteX's strict client role; it does not establish off-path
spoofing or a generally exploitable library vulnerability.

A separate characterization probe succeeded: keeping the same DnsResponseStream
and requesting its next item yields the subsequent correct answer with the same
transaction ID. It makes one wire query, uses the existing public stream API and
retains an explicitly owned background JoinHandle, abort/join and socket rebind.
All four probes together remain failed (three passed, one failed). This finding
cannot be turned into a qualification pass by relabeling the foreign answer.

## Accepted decision

Authorize isolated qualification of a composed client role: the exact existing
library handles transport/framing/transaction delivery, and mandatory bounded
DRouteX ingress and caller-validation adapters limit work and determine whether
an answer belongs to the original question. Keep version, checksum, features and lock pins.
This allocates a security responsibility explicitly; it is an amendment to D51,
not an assertion that the raw library already supplies every strict check.

The test-only layer must:

- Retain the original question name/type/class and opcode; require response QR,
  the single expected question and complete consumption of the raw DNS frame.
  Compare names using DNS semantics, not presentation-case string equality.
  Keep existing library provenance/transaction checks; never accept a different
  peer, port or ID, and do not infer authority from question data.
- Enforce a 4096-byte accepted DNS message ceiling and at most 16 delivered response
  candidates per attempt. Also bound input before library filtering: at most 16
  UDP datagram receives; for TCP at most 65,568 received bytes and 256 completed
  read/write operations per direction, including input the multiplexer discards.
  Count successful I/O, not Pending polls, so ordinary readiness polling consumes
  no artificial budget. Use one absolute 500ms attempt deadline, including
  connect, send, validation and receive; rejected input never resets it.
  These are qualification constants only. Production budgets remain a later
  complete P3-02 contract, not automatically selected by this experiment. Apply
  these budgets through existing public RuntimeProvider/DnsUdpSocket and Tokio
  stream adapter APIs; delegate actual sockets/framing to Tokio/Hickory. No new
  wire transport or dependency is proposed. An outer response counter alone is
  insufficient because library filters discard some packets internally.
- UDP ingress must detect oversized/truncated datagrams before handing bytes to
  the library's request-sized buffer: receive into a fixed 4097-byte scratch
  buffer, reject lengths over4096 or over the provided destination capacity,
  and never turn an oversized datagram into an apparently valid shorter answer.
  Bound writes to one query (at most4098 framed bytes) and256 successful writes.
- Treat 4096 as an acceptance ceiling, not a preallocation claim: the existing
  TCP framing code may allocate one u16-sized frame (at most 65,535 bytes) before
  caller validation. Keep that explicit bounded behavior under review. A stricter
  preallocation requirement would need another implementation choice, not a
  claim that the caller filter prevents the allocation.
- Discard foreign or structurally invalid candidates and inspect a later item
  only while the original deadline and candidate budget permit. Preserve the
  same request and stream; issue no replacement query, fallback or extra worker.
  Underlying transport failure or exhausted stream/budget/deadline fails closed.
  UDP exposes a one-shot result; a caller-rejected delivered UDP candidate ends
  that attempt. Recovery after internally rejected foreign ID/question packets
  is distinct from recovery in TCP's multi-item stream; do not promise parity.
- Preserve a direct UDP request future and explicit TCP background ownership.
  Cancel/drop the request, abort and join owned background work and prove socket
  release/rebind in success, failure and cancellation cases. No detached helper
  or generic convenience API with hidden background ownership is qualified.
  Cleanup has its own finite 3s abort/join deadline and cannot extend the500ms
  response-acceptance deadline. Missing cleanup proof blocks qualification.
- Retain the raw-library failure as explicit characterization evidence; add
  negative regressions against the composed boundary and valid-after-foreign
  probes. Unproven malformed/frame/lifecycle bounds still block qualification.

No custom wire transport, vendor patch, new dependency/license exception, TLS,
OS resolver, external DNS or privileged networking is authorized. The existing
independent fixture emits raw DNS bytes; the guard can use the already approved
Proto decoder and its consumed-position check. Mandatory IPv4/IPv6 Windows/Linux
proofs, dependency inventory/deny and two account reviews remain unchanged.
Acceptance authorizes investigation/implementation of these adapters only; any
unproven bound or cleanup requirement keeps the composed role unqualified.

## Unselected alternative

Keep D51's strict raw-library role unchanged. Mark this candidate unqualified for
that role and stop its dependent adoption. Selecting another dependency or vendor
patch requires a separate concrete proposal and owner decision. Existing admitted
queries keep the current SERVFAIL behavior in either option until a later full
P3-02 production contract and implementation pass their gates.

## Authority and approval boundary

[AGENTS](../../AGENTS.md) says: "Ask about new consequential choices or unclear requirements:
architecture, dependencies/licenses, security/access, user experience,
infrastructure/costs, scope or development process."
[D51](P3_02_CLIENT_QUALIFICATION_CONTRACT.md) also says a different dependency, custom wire client or relaxed requirement
needs a new decision. The accepted profile keeps strict acceptance requirements
but changes which component owns ingress budgets and question/frame validation, and the owner explicitly accepted it before dependent guard implementation. It does not complete qualification
or authorize P3-02 application behavior, resources or production adoption.
