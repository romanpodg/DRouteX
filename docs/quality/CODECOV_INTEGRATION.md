# Codecov integration

Status on 2026-10-16: **Planning/backend uploads verified; P0-04 in review.**
The user authorized configuring both quality services for
[romanpodg/Routira](https://github.com/romanpodg/Routira). GitHub CLI has verified
repository admin access. The user signed into Codecov; Routira is activated.
The pre-existing Codecov GitHub App covers this repository; its permissions were
not changed. Root YAML passed the official `https://codecov.io/validate` endpoint
with `Valid!`. No permanent upload token is stored in this repository.
GitHub reports this repository as private; its visibility was not changed.
The minimal P0-02 Cargo workspace exists; no frontend workspace exists. Planning coverage is separate from application
coverage; see [execution status](../planning/EXECUTION_STATUS.md).

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
Local coverage is 62.50% of eight instrumented source lines. See the
[P0-04 review](../planning/P0_04_CI_REVIEW.md) for actual service/run evidence;
local percentage and a successful upload are distinct observations.

[Run 38860444206](https://github.com/romanpodg/Routira/actions/runs/38860444206)
passed all six jobs on `e4f4143`, including both explicit-file OIDC uploads. The
downloaded Rust LCOV has the expected crate source path and five of eight lines
covered. The missing-report upload probe failed as intended in
[run 38860049899](https://github.com/romanpodg/Routira/actions/runs/38860049899).
GitHub's backend/planning patch checks succeeded with the explicit message
"No report found to compare against"; main still lacks a baseline. The signed-in
commit page returned 404 and repository search remained empty after Resync, so
the new service-rendered percentages/lines remain unverified. No repository
visibility, application grant or billing setting was changed.

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
The current baseline branch has no application code; a missing base report on PR #1
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
