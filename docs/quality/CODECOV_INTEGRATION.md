# Codecov integration

Status on 2026-10-16: **Main reports and meaningful P1-01 application comparison verified.**
The user authorized configuring both quality services for
[romanpodg/Routira](https://github.com/romanpodg/Routira). GitHub CLI has verified
repository admin access. The user signed into Codecov; Routira is activated.
The pre-existing Codecov GitHub App covers this repository; its permissions were
not changed. Root YAML passed the official `https://codecov.io/validate` endpoint
with `Valid!`. No permanent upload token is stored in this repository.
The owner made the repository public and approved activating the existing Codecov
project without changing OIDC, tokens, access or plan. The common/config/policy
workspace exists; no frontend workspace exists. Planning coverage is separate from application
coverage; see [execution status](../planning/EXECUTION_STATUS.md).

On [PR #4](https://app.codecov.io/gh/romanpodg/Routira/pull/4), exact implementation
head `4d79b7a95f094851bd2b73c4c924a85c7231de44` is processed against main base
`a77ed5adce184ffe3214452af0b1e6d83db2cd92`. All seven jobs of
[run 38911280732](https://github.com/romanpodg/Routira/actions/runs/38911280732)
passed. New Rust application lines are 71/71 covered; the backend patch check
passed at 100% and backend totals are 76/79 (96.20%). The aggregate patch 76/76
also counts five auxiliary Python lines, and aggregate 509/515 is not application
coverage. Both flags processed. Linux/service instrumentation counts one identity
line fewer than the local Windows 72/72 report; neither count is substituted for
the other. Coverage does not establish actual capability verification or network
admission. This resolves the P0-04 Codecov comparison deferral; the actual fork
test remains owner-deferred.

## Historical bootstrap evidence

[Actions run 38709252934](https://github.com/romanpodg/Routira/actions/runs/38709252934)
passed both OS jobs and the OIDC upload for commit `2aef0d5` on PR #1.
The [Codecov commit report](https://app.codecov.io/gh/romanpodg/Routira/commit/2aef0d59729ecd6ea6a8b57e0beb801e30e5ca96)
shows **95.95%**, 308 covered lines of 321, six partial and seven missed. This is
the three planning tools only. Codecov and coverage.py count partial branches
differently; use the service report for Codecov's percentage. Main has no baseline
report yet; view the review branch explicitly until the owner merges PR #1.
The subsequent [run 38709858143](https://github.com/romanpodg/Routira/actions/runs/38709858143)
also passed both OS checks and upload on commit `32a458d`.

## Active reports

P0-04 extends the same workflow with pinned cargo-llvm-cov 0.9.1 and LLVM tools
for Rust 1.99.0. Ubuntu generates `coverage/backend.lcov`, retained for seven days,
and the isolated upload job submits it with the `backend` flag. Its path scope is
the three crate source directories. Project/patch statuses are informational with
carryforward disabled; the existing critical-code test requirements still apply.
The initial local baseline was 62.50% of eight instrumented source lines. See the
[P0-04 review](../planning/P0_04_CI_REVIEW.md) for actual service/run evidence;
local percentage and a successful upload are distinct observations.

[Run 38860444206](https://github.com/romanpodg/Routira/actions/runs/38860444206)
passed all six jobs on `e4f4143`, including both explicit-file OIDC uploads. The
downloaded Rust LCOV has the expected crate source path and five of eight lines
covered. The missing-report upload probe failed as intended in
[run 38860049899](https://github.com/romanpodg/Routira/actions/runs/38860049899).
GitHub's backend/planning patch checks succeeded with the explicit message
"No report found to compare against"; main then lacked a baseline. The signed-in
commit page returned 404 and repository search remained empty after Resync, so
the new service-rendered percentages/lines were unverified at that snapshot.
The later owner-approved activation and processed reports are recorded above.
No application grant or billing setting was changed.

The [planning workflow](../../.github/workflows/planning.yml) runs the existing
renderer, contract validator and quality-configuration validator under coverage.py
7.16.2 on Ubuntu and Windows. The [.coveragerc](../../.coveragerc) explicitly scopes
the report to those three planning tools, including their negative self-checks.
Environment/setup/graph tools are outside this report's scope. No product code or
dummy workspace is added to obtain a percentage.

Ubuntu publishes `coverage/planning.lcov` as a seven-day artifact. A separate upload
job uses OIDC and separate `planning`/`backend` flags; it runs no repository scripts.
Fork PRs are configured to keep coverage artifacts and skip upload; the actual
fork test is deferred by the owner. There is no `pull_request_target`,
repository upload secret or branch-protection change. Configured upload errors fail
the upload job. The root [codecov.yml](../../codecov.yml) makes project and patch
coverage informational and disables carryforward so old reports cannot fill gaps.

Local checks reject widened test-job permissions, an unconditional fork upload,
repository scripts in the upload job and unpinned actions. Repository configuration
alone does not verify Codecov account access, OIDC acceptance, upload or PR statuses.
The initial PR #1 baseline branch had no application code; a missing base report on PR #1
must not be reported as a successful coverage comparison.

The inspected current release is [codecov-action v7.1.1](https://github.com/codecov/codecov-action/releases/tag/v7.1.1).
The proposal pins its immutable commit 303a32d7a59b442fa8d48b6a1cc6825c09c847a5.
Its [action inputs](https://github.com/codecov/codecov-action/blob/v7.1.1/action.yml) include explicit
files, disable_search, flags, use_oidc and fail_ci_if_error; older README examples are not used
to select the current major.

## Reports and authentication

Rust: verified cargo-llvm-cov 0.9.1 help supports
`cargo llvm-cov --workspace --locked --lcov --output-path lcov.info`.
Exact feature selection comes from P0-02/P0-04. Namespace/root-requiring tests stay separate
from ordinary coverage unless run in the trusted isolated lab.

Frontend: choose [Vitest V8 coverage](https://vitest.dev/guide/coverage.html); lock Vitest and
@vitest/coverage-v8 at the same supported version, emit LCOV, and include branch-sensitive
behavior checks. Backend/frontend are uploaded separately with distinct flags and explicit paths.

OIDC is active in a separate trusted upload job; tests and fork jobs have no id-token
permissions. Only same-repository PRs, protected pushes and approved manual runs may upload.
Fork PRs are configured to test and retain LCOV artifacts without secrets; the upload guard skips them.
Do not use pull_request_target to run fork code. A future fork upload design requires separate
account policy and trusted report handling; no success is implied by a skipped upload.

If OIDC cannot be enabled for the selected Codecov repository, a GitHub repository Secret
CODECOV_TOKEN is a documented alternative only in the same trusted upload job, never a file.
Keep upload failure fatal where upload is configured; report both report-generation and
service-upload state separately.

## Reviewable files and activation steps

- [coverage workflow proposal](proposals/coverage.yml.example): checkout, exact Rust toolchain,
  tests/LCOV, frontend coverage, separate artifacts and restricted OIDC uploads.
- [codecov.yml proposal](proposals/codecov.yml.example): separate backend/frontend project
  reports, global/patch informational baseline and proposed 90% new critical logic patch gate.

These broader examples remain reference files. P0-04 activates only the existing
Rust workspace through the foundation workflow; frontend coverage is activated
when an actual frontend package/scripts/lockfile exists at its owning gate. Pin
action refs and tools; verify feature and package-manager compatibility at activation.
[Status configuration](https://docs.codecov.com/docs/commit-status) supports filtered project
and patch reporting. No global 90% gate is imposed before meaningful baseline.
Do not exclude tests/handwritten hard cases just to inflate coverage; generated-code exclusions
need an explicit justified inventory.

P0-04 verifies backend path mapping and actual upload/failure behavior. The owner
deferred a real fork PR test; no successful fork run is claimed. D17 and the CI
review track current evidence. Frontend coverage remains pending actual frontend work.

Validation boundary: validate active and proposed Actions with actionlint and run
`tools/verify_quality_proposals.py`, including its negative authorization cases.
The local validator never sends files. Service-side validation and first actual
upload must be recorded separately before claiming this integration works.
The local run generated LCOV successfully with 97% combined line/branch coverage
for the scoped planning tools (coverage.py rounded display); this is not a Codecov
result or application coverage.

## Verified public-repository activation, 2026-10-16

The owner made the repository public; Codecov still reported `activated: false`.
With explicit owner approval the existing project was activated, preserving the
OIDC workflow, credentials, access and plan. Upload attempt 2 of
[Actions run 38865306388](https://github.com/romanpodg/Routira/actions/runs/38865306388)
passed. The [actual main report](https://app.codecov.io/github/romanpodg/routira/commit/c43c2be0d40248f81bb01be9964ba56925b368f5)
is complete with two uploads labeled backend and planning. The exact merge commit
has 5/8 covered Rust config lines (62.50%). The combined 437/443 figure includes
auxiliary Python tooling and must not be presented as application coverage.
The merge shows no covered-file diff; useful new-code PR comparison remains to be
verified at the next relevant change. The owner-deferred real fork test is unchanged.
