# P1-06 activation implementation evidence

Status: candidate implementation; not complete or merge-ready. The owner accepted
[D36](P1_06_ACTIVATION_CONTRACT.md) after P1-05's reviewed merge and verified main.
No new dependency, protocol engine, privileged execution or listener is introduced.

## Implementation

`droutex-core::deployment` owns a bounded actual-artifact set, authenticated immutable
generation and serialized deployment writer. Decisions capture one generation for
both controlled consumer views. Previous pins provide explicit Busy backpressure.
Trust replacement stops new pins if the active signer is revoked; existing pins
can finish. Replay admission always rechecks the current durable floor.

Linux filesystem storage uses private regular files, process locking, three fixed
slots and a bounded strict manifest. Staged bytes are flushed, reloaded and verified
before committing active identity and floor together. Commit uncertainty freezes
further activation pending reopen. First provisioning and reopen are separate;
recovery never silently falls back to an older/corrupt/revoked generation or floor0.
The config crate adds a narrow, separately typed exact retained-verification path.

## Actual local checks so far

Windows: fmt, workspace/all-target checks, no-default check, all-feature Clippy,
default and no-default workspace tests passed. The later semantic-consumer fixture
and retained-identity test passed targeted core/config reruns. All17 planning tests,
planning self-test (9 negative cases), quality configuration and offline cargo-deny
advisories/bans/licenses/sources passed. The lockfile adds only the local core crate;
all registry package versions/checksums remain unchanged. Graphify AST update ran
(3269 nodes,6907 edges); local graph outputs remain untracked.

The new tests cover coherent captured versions, previous-reader backpressure,
concurrency barriers, replay/gaps/rotation/max-sequence/fresh rollback, cancellation,
scope/pin/signature/budget refusal, revocation and unsupported filesystem platforms.
Linux-only tests additionally exercise actual process exit/reopen at13 durable
protocol points, CP paths removed, missing/corrupt active data, path/link/mode
refusals, exclusive writer, injected write/flush/uncertain outcomes, and a real
unprivileged `/dev/full` ENOSPC propagated through fault injection. These are not
physical power-loss or volume-exhaustion tests.

## Outstanding gates

Actual Linux build/test execution and exact-head Windows/Linux CI have not yet
completed. Independent standards/spec passes are running; required linked-account
Codex implementation/final-evidence reviews remain pending. CodeScene and Codecov
results for this candidate are unavailable until actual remote processing.
No outstanding gate is recorded as passed. P1-06 remains in progress; P2 stays planned.
