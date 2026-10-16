# P0-04 CI review

Date: 2026-10-16. Status: **In review; real fork PR test deferred by owner**.
The owner authorized publishing P0-03 and implementing P0-04 in existing PR #1.
P0-05, product features, privileged CI, merging and branch-protection changes remain
outside this authorization. P0-03/selective-reuse documentation was published in
commit `28eb7d8` before the CI change.

## Scope and approach

Extend the existing foundation workflow rather than add another competing CI system.
Keep Windows/Linux format/check/Clippy/tests, the pinned Rust 1.99.0 MSRV and the
existing unprivileged planning/fixture checks. Current crates have no optional
features: default/no-default/all-features cover the same code; no unsupported
feature combination or rolling-toolchain compatibility is claimed.

One Ubuntu job installs pinned cargo-deny 0.20.2 and cargo-llvm-cov 0.9.1. It fetches
current advisory data, checks the locked dependency graph offline, then generates
Rust LCOV. This reviews the existing dependency-free workspace, not P0-05's future
DNS/TLS/proxy dependency selection. No application dependency or frontend is added.

Rust coverage uses a separate backend artifact and Codecov flag scoped to crate
source. Reports are informational while the first application baseline is measured;
there is no new threshold waiver or test/refactor campaign to raise percentages.
The historical planning flag is retained separately. No frontend report is claimed.
Existing critical-code acceptance and meaningful negative tests remain required.

All test jobs use GitHub-hosted runners with contents:read and no secrets/OIDC.
Only the separate same-repository upload job has id-token:write; it runs pinned
actions and no repository scripts. Fork uploads are skipped; fork checks retain
their artifacts. Upload errors are fatal. No SSH key, VPS/root runner or raw lab
capture is used in Actions.

## Fork policy and validation boundary

GitHub reports Routira as **private**, correcting the earlier planning metadata.
The owner approved enabling private fork workflows with mandatory approval and
without secrets/variables or write tokens. A subsequent API read confirmed all four
settings. Repository visibility and branch protection were not changed.

The owner explicitly deferred a real fork PR test and declined a temporary private
fork in Driics-Team. No fork or test PR was created. Local negative checks verify
the workflow permission/upload boundary; they are not actual fork-run evidence.
Keep that acceptance item deferred rather than claiming it passed.

## Validation evidence and completion

Local Windows validation passed fmt, locked default/no-default checks, all-features
Clippy with warnings denied, both test modes, cargo-deny advisories/bans/licenses/
sources after a fresh fetch, actionlint, 16 planning regression tests and the
planning/quality negative validators. The official Codecov validator accepted the
active YAML. Rust LCOV measured 62.50% of eight instrumented source lines, unchanged
from P0-02; this is a local result, not a Codecov service percentage.

The first implementation, `ee206c96d3bd4573feccddf20c21b1b4ac617f38`, exposed a
clean-runner defect in [run 38859969991](https://github.com/romanpodg/Routira/actions/runs/38859969991):
workspace/planning jobs, cargo-deny and the instrumented tests passed, but LCOV
could not create its absent parent directory. No upload ran. Commit `e4f4143`
adds `mkdir -p coverage` before report generation; actionlint and the authorization
validator passed afterward. The corrected positive run is recorded separately.

For corrected commit `e4f41430515bbd7c6551cdf091a50a9fe57266e5`,
[run 38860444206](https://github.com/romanpodg/Routira/actions/runs/38860444206)
passed all six jobs: Workspace and Planning on Ubuntu/Windows, dependency/coverage,
and both Codecov uploads. The downloaded backend artifact contains only
`crates/routira-config/src/lib.rs`, eight instrumented lines and five covered lines
(62.50%). The other two crates currently have no instrumented executable lines;
no test or planning source appears in the backend LCOV. The artifact's runner-root
source path maps to the intended crate path. Upload and service rendering are
separate evidence; no backend service percentage is inferred from upload alone.

GitHub received successful `codecov/patch/backend` and `codecov/patch/planning`
checks for that commit, both explicitly saying **No report found to compare
against**. Main has no baseline; these informational successes do not establish a
passing coverage comparison. The signed-in Codecov commit page returned 404, and
the repository search stayed empty after one Resync. Backend service percentage/
line rendering is therefore unverified; no access/visibility/billing change was
made to resolve it. Both uploads were accepted and queued in the Actions log.

The disposable same-repository branch `ci/p0-04-failure-probes-20261016` added only
a pinned, GitHub-hosted probe workflow at commit
`b8855f5ba4d80dc6a6c1411c5d18f8c4a82929f1`. Its
[actual run](https://github.com/romanpodg/Routira/actions/runs/38860049899)
failed all three jobs for the intended reasons, without continue-on-error:

| Probe | Actual rejection |
|---|---|
| Clippy, same workspace/all-targets/all-features command | Appended unused function rejected as dead_code, exit 101 |
| Tests, same locked workspace command | Injected assertion `1 == 2` failed, exit 101 |
| Pinned Codecov action, explicit absent report, search disabled | No coverage reports found, exit 1; fail_ci_if_error enforced |

Injected Rust existed only inside separate disposable job checkouts. The probe
commit changed no product source, main or PR #1; no fork/test PR was created. The
temporary ref was deleted after verifying its exact SHA and all three failures;
an API read confirmed no matching ref remained. The negative run remains evidence.

[CodeScene review 7980486](https://codescene.io/projects/86145/delta/results/7980486)
on corrected head `e4f4143` failed gates for score/complexity findings in
`tools/verify_planning.py`, `tools/build_doc_graph.py` and the two network fixture
scripts. These planning/auxiliary/fixture files are outside the owner's independent
metric-improvement scope. No application or workflow finding is listed. The
repository-wide check remains failed; no suppression or metric-driven rewrite
was made, and no product Code Health pass is claimed.

Source/configuration tests do not establish visible fork checks. P0-04 remains
in review while the required actual fork PR evidence is deferred.
