# Codecov integration

Status on 2026-10-15: **Planning coverage upload verified.**
The user authorized configuring both quality services for public
[romanpodg/Routira](https://github.com/romanpodg/Routira). GitHub CLI has verified
repository admin access. The user signed into Codecov; Routira is activated.
The pre-existing Codecov GitHub App covers this repository; its permissions were
not changed. Root YAML passed the official `https://codecov.io/validate` endpoint
with `Valid!`. No permanent upload token is stored in this repository.
No Cargo/frontend workspace exists. Planning coverage is separate from product
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

## Active planning coverage

The [planning workflow](../../.github/workflows/planning.yml) runs the existing
renderer, contract validator and quality-configuration validator under coverage.py
7.16.2 on Ubuntu and Windows. The [.coveragerc](../../.coveragerc) explicitly scopes
the report to those three planning tools, including their negative self-checks.
Environment/setup/graph tools are outside this report's scope. No product code or
dummy workspace is added to obtain a percentage.

Ubuntu publishes `coverage/planning.lcov` as a seven-day artifact. A separate upload
job uses OIDC and the `planning` flag; it runs no repository scripts. Fork PRs keep
the coverage artifact and visibly skip upload. There is no `pull_request_target`,
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

OIDC is proposed for a separate trusted upload job; tests and fork jobs have no id-token
permissions. Only same-repository PRs, protected pushes and approved manual runs may upload.
Fork PRs still test and retain LCOV artifacts without secrets; upload is visibly skipped.
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

These product-coverage files remain .example files in docs, separate from active planning coverage.
P0-04 promotes them only after workspace,
frontend scripts/lockfile, account policy and first run are present. Pin action refs and tool
versions; verify feature sets and Node/package-manager compatibility at activation.
[Status configuration](https://docs.codecov.com/docs/commit-status) supports filtered project
and patch reporting. No global 90% gate is imposed before meaningful baseline.
Do not exclude tests/handwritten hard cases just to inflate coverage; generated-code exclusions
need an explicit justified inventory.

Remaining product verification in P0-04: generate backend/frontend reports, inspect
their path mapping/changed lines, intentionally demonstrate upload failure, then
review required statuses with the owner. D17 tracks the absent product workspaces.

Validation boundary: validate active and proposed Actions with actionlint and run
`tools/verify_quality_proposals.py`, including its negative authorization cases.
The local validator never sends files. Service-side validation and first actual
upload must be recorded separately before claiming this integration works.
The local run generated LCOV successfully with 97% combined line/branch coverage
for the scoped planning tools (coverage.py rounded display); this is not a Codecov
result or application coverage.
