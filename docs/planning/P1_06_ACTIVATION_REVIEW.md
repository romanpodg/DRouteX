# P1-06 activation implementation evidence

Status: P1-06 complete; PR11 merged normally at6b58b892cf2e9c58499cd60d4ccad9c503a1d656
after independent final-evidence review and all11 final-head jobs39153940919.
All11 exact-main jobs39154498259 passed; Codecov processed both reports with Rust
backend3322/3503. Implementation had passed review/all11 jobs on3c12bba.
The owner accepted
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
(3294 nodes,7008 edges after the final code update); local graph outputs remain
untracked. SQL extraction remains unavailable without its optional parser; no
semantic-document refresh or paid backend is claimed.

The new tests cover coherent captured versions, previous-reader backpressure,
concurrency barriers, replay/gaps/rotation/max-sequence/fresh rollback, cancellation,
scope/pin/signature/budget refusal, revocation and unsupported filesystem platforms.
Linux-only tests additionally exercise actual process exit/reopen at13 durable
protocol points, CP paths removed, missing/corrupt active data, path/link/mode
refusals, exclusive writer, injected write/flush/uncertain outcomes, and a real
unprivileged `/dev/full` ENOSPC propagated through fault injection. These are not
physical power-loss or volume-exhaustion tests.

## Outstanding gates

Implementation/final-head/main gates passed. Final evidence review session
01a15041-414f-7d51-a125-c15d56d357c2 exited0 with no findings,228 checked links
and9 negative cases. CodeScene8006668 retained the same failed maintenance findings
after result-link normalization; dispositions below remain. No gate was bypassed.
Later runtime/product acceptance remains at its owning tasks.

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
The updated Windows core tests/Clippy passed. Focused read-only re-reviews reported
no remaining findings; the later actual Linux and linked-account evidence is below.

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
the later exact-head Linux CI and independent review passed as recorded below.
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
process exit with findings is not a passed review; the exact-head re-review below
completed after both corrections.

## Verified implementation head

Head:3c12bba74070cc583bc2360bebd0f1f6a7da277c; base/main:
521bfa76fbc27e14cd4981638af775ba92c6299f. All11 jobs of
[run39153071723](https://github.com/romanpodg/DRouteX/actions/runs/39153071723)
passed. Windows ran77 successful tests per default/no-default configuration;
Linux ran104 each, including20 core tests, the retained-identity and owner-lifetime
compile-fail checks,13 publication and3 uncertain-recovery process-exit points.
Actual fresh dependency/advisory/license checks and both coverage uploads passed.
The earlier4905609 Linux job also passed, but its whole run39152824085 was cancelled
by the next source push; it is not recorded as an all-green run.

Required linked-account Codex Code Review used read-only/never against the exact
base, session01a15036-ec71-7751-a133-482cf55a5b8f, exit0, and reported no actionable
defect. Head3c12bba stayed unchanged during this re-review. Reviewer fmt passed;
its Clippy/test attempts were blocked by the read-only Cargo build lock. Actual
execution evidence comes from the independent successful CI above. Local review
logs and CI/coverage JSON remain under .setup/p1-06 and are not published artifacts.

Codecov processed the exact head with state complete and two reports. The
[backend comparison](https://app.codecov.io/gh/romanpodg/DRouteX/pull/11)
against521bfa7 has738/785 new Rust lines covered (94.01%); total Rust backend is
3322/3503. The combined Python/Rust patch746/793 is not application coverage.
No tests or behavior were rewritten solely for these numbers.

CodeScene8006609 failed its three maintenance gates: filesystem guards7.91,
journal validation9.54 and two5-argument boundary/dispatch methods9.69, plus the
excluded auxiliary validator9.07->9.05. Source-based minor maintenance dispositions
are recorded in [OPEN_DECISIONS](OPEN_DECISIONS.md#p1-06-review-dispositions).
The concrete functional/security findings above were all fixed; none is deferred.
The CodeScene failure remains visible, with no suppression or bypass.

## Limits preserved

D36 proves bounded local state, coherent controlled consumers and unprivileged
Linux crash/restart behavior. It does not claim real DNS/gateway listener admission,
client TTL/session consistency, compiled P2 policy, systemd acceptance, physical
power-loss resilience or detection of malicious rollback of the entire volume.
Those owning gates and the explicit operator recovery boundary remain unchanged.
