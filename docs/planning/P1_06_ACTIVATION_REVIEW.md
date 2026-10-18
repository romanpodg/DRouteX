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

## Independent pre-CI findings and corrections

Separate read-only standards/spec passes found three defects in be9d600 before
publication: cancellation during manifest preparation was not rechecked before
rename; root placement did not exclude mutable untrusted ancestors; previous
retention was not checked on reopen. These are fixed in the candidate: cancellation
is checked immediately before the irreversible rename; ancestors require root or
service ownership and deny non-sticky group/world writes, with root device/inode
revalidation before storage operations; previous storage is structurally bounded
and private without requiring its revoked signing key. Added Linux regression
tests cover each path, strict manifest inconsistencies and cancellation after commit.
The updated Windows core tests/Clippy passed. Re-review and Linux runtime evidence
remain pending; source fixes alone do not pass those gates.

## First actual Linux run, retained failure

On98237f4 run39152363848, Linux fmt/check/no-default-check/Clippy passed and16/17
core tests passed, including all13 process-exit/reopen points. The uncertain-commit
test observed Busy when reopening immediately after dropping its deployment owner.
The same-head backend coverage test run succeeded; this does not erase the workspace
failure. Source inspection and independent follow-up identified an OS file-lock
lifetime gap: closing one File does not release flock while another duplicated or
inherited descriptor still owns the same open-file description. Concurrent process
spawn is a plausible triggering interleaving, not a proven trace of that CI event.

The candidate now explicitly unlocks the uniquely owned non-Clone store before
closing it. A deterministic Linux regression keeps a duplicated descriptor alive
after owner drop and checks immediate reopen, then confirms dropping the old
duplicate cannot unlock the new owner. The previous failure remains recorded;
new exact-head Linux CI and independent review are required before acceptance.
Lock contention now maps specifically from WouldBlock; other lock I/O errors map
to Filesystem instead of claiming contention. The lock lifetime follows the
[standard File contract](https://doc.rust-lang.org/std/fs/struct.File.html#method.unlock).

## Required Codex Code Review findings

The linked-account read-only/never review completed with two actionable findings:
recovery could clean a slot before making its selected post-uncertainty manifest
durable; a Decision could outlive its Deployment owner and bypass previous-pin
backpressure after reopening. Neither is deferred. Recovery now syncs the chosen
manifest directory before any cleanup. A Decision borrows its owner for its entire
lifetime, with a compile-fail regression preventing close/reopen while still used.
Linux fault/crash regressions cover the new recovery boundary. A successful review
process exit with findings is not a passed review; exact-head re-review remains required.
