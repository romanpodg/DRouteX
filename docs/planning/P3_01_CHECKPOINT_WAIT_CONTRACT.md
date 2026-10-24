# P3-01 checkpoint-gap behavior contract (D50)

Status: **Option A accepted by the owner on 2026-10-23 by direct chat reply.**
Implementation is authorized under these bounds and remains review/check gated.
D50 clarifies transport handling under [D47/D48/D49](P3_01_LISTENER_CONTRACT.md).
The [D46 clock/admission checks](P4_01_BINDING_CONTRACT.md) retain their refusal
semantics. PR110 completed normal merge and exact-main gates; see
[the recorded completion](P3_01_LISTENER_REVIEW.md#normal-merge-and-exact-main-completion).

## Evidence and problem

The final PR110 head8027b4f passed completed linked-account final Code Review,
but CI39896580361 failed Windows pending_processor_gets_servfail_within_the_original_query_budget
with UnexpectedEof while reading the TCP reply. Ten other jobs passed; the
split/pipeline case and mandatory IPv6 passed on this head. The earlier
head81820e0 passed all11 jobs. Neither a previous green head nor clean source
review replaces the failed current-head platform check.

D46 rounds the conservative UTC upper bound to five-second checkpoint windows.
A live reservation can encounter CheckpointRequired after a window transition,
before the single maintenance worker persists the next floor. The runtime's
final submission check maps that refusal to Unavailable and never polls the I/O.
This is fail-closed behavior, not authorization from a stale clock.

A bounded local probe reproduced this mechanism using the actual Deployment
and runtime submit gate: reserve a valid manual grant, cross the next checkpoint
window, observe CheckpointRequired and refusal before a ready I/O future is
polled, complete the real bounded blocking refresh, then verify that the same
held reservation is current again. No publication, lease renewal, quota
reacquisition or clock re-anchor occurs. Temporary diagnostic test source was
removed and exact committed bytes restored after the successful probe.

Both observed CI reply failures occurred just after a matching five-second
window boundary. This supports investigating the reproduced mechanism; their
logs do not contain the original AdmissionCode, so exact causation of each CI
failure is still an inference. The fixture startup race is separately proven
and its correction remains useful, but does not close this behavior question.

## Accepted option A: bounded checkpoint wait

Clarify D47 that a missing next clock checkpoint can suspend an otherwise held
attempt; it cannot authorize processing or output while unavailable.

- Retain the original non-Clone reservation, generation, scope and quota charge.
  Add an ownership-preserving consumption operation if needed; leave the
  existing consume API's refusal behavior available to its current callers.
- Only CheckpointRequired, or Unavailable during this node's known single
  maintenance operation, may wait for that operation's outcome. Preserve a
  completion racing with the authority check/refusal even if pending has cleared;
  discard only notifications older than that check. Publish completion before
  clearing the worker-owned pending flag, independently of the async waiter.
  Bind the wait to one operation identity/outcome; an old pending accounting flag
  cannot start another operation after failure. Unknown/superseded outcomes refuse.
  Never retry a generic Unavailable, unknown source, revocation, retired
  generation, expiry, clock fault, restart wait, quota or persistence failure.
- Subscribe before requesting maintenance and use its existing bounded worker.
  At most one checkpoint wait/recheck per gate invocation; another refusal drops
  or closes. Add no worker, retry queue, admission acquisition or re-pinning path.
- All waiting consumes the existing frame/query/connection deadline. Processing
  still stops at1500ms and total work at2s; the500ms reply reserve is not extended.
  Deadline expiry drops/closes without polling ready I/O or inventing a fallback.
- After completion, recheck the original reservation against current generation,
  lease, clock and durable floor before consumption and every I/O resume. A
  successful refresh is never permission by itself. A changed or revoked grant
  still denies; a failed checkpoint still denies.

Required regressions: controlled pending/successful/failed checkpoint, unchanged
generation/quota ownership, revocation/expiry/publication during the wait, timer
and I/O ready together after deadline, partial TCP output, cancellation/shutdown
and actual UDP/TCP behavior. Preserve existing IPv6, CP-loss and persistence
failure cases. Obtain fresh Windows/Linux profiles, dependency checks, both
linked-account reviews, normal current-head merge and exact-main evidence.

## Unselected alternative B: preserve immediate refusal

Retain current production behavior: a required checkpoint absent at consumption
or submission drops UDP/closes TCP immediately. Make positive fixtures explicitly
establish the required current floor and separate the valid-floor SERVFAIL case
from a deterministic missing-checkpoint refusal. Do not count a generic EOF as a
successful positive response, skip a required case, extend deadlines, suppress
the CI failure or retry CI until green. Any extra public test-clock capability
would need separate boundary review; host clock/network changes are not allowed.

## Acceptance and remaining gates

Option A changes when a previously unavailable admission may resume and adds
ownership-preserving retry behavior at a security boundary. The owner explicitly
selected it in this chat, satisfying [AGENTS](../../AGENTS.md); every current
authority check remains required. Option B was not selected. D50 authorizes the
narrow implementation above, not a merge exception, production deployment or
next-task qualification. The already accepted isolated P3-02 dependency
qualification remains gated on checked P3-01 completion.
