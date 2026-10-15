# Codecov integration proposal

Status: **Not activated or verified.** The selected public repository is
[romanpodg/Routira](https://github.com/romanpodg/Routira). No Cargo/frontend workspace,
coverage report or verified Codecov account/upload exists. Planning-only CI is
separate from product coverage; see [execution status](../planning/EXECUTION_STATUS.md).
GitHub user authentication is valid outside the restricted sandbox.

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

These are .example files in docs, not active Actions. P0-04 promotes them only after workspace,
frontend scripts/lockfile, account policy and first run are present. Pin action refs and tool
versions; verify feature sets and Node/package-manager compatibility at activation.
[Status configuration](https://docs.codecov.com/docs/commit-status) supports filtered project
and patch reporting. No global 90% gate is imposed before meaningful baseline.
Do not exclude tests/handwritten hard cases just to inflate coverage; generated-code exclusions
need an explicit justified inventory.

User/maintainer actions: select remote repo/visibility; enable Codecov GitHub access to that repo;
approve OIDC trust policy (or store token via Secrets); validate YAML on Codecov; run a real PR,
inspect both reports/path mapping/changed lines, intentionally demonstrate upload failure,
then enable required statuses. D17/P0-04 tracks each blocker.

Validation boundary: actionlint 1.7.12 passed the workflow proposal locally. Sending the
configuration to the public Codecov YAML validator was rejected by automatic approval review
because external file transfer was not explicitly authorized. No request/upload was performed.
Local YAML parsing and policy checks are used instead; service-side validation remains unverified.
