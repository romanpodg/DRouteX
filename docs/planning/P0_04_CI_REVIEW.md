# P0-04 CI review

Date: 2026-10-16. Status: **In progress; actual runs pending**.
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

## Validation plan and completion

Local Windows validation passed fmt, locked default/no-default checks, all-features
Clippy with warnings denied, both test modes, cargo-deny advisories/bans/licenses/
sources after a fresh fetch, actionlint, 16 planning regression tests and the
planning/quality negative validators. The official Codecov validator accepted the
active YAML. Rust LCOV measured 62.50% of eight instrumented source lines, unchanged
from P0-02; this is a local result, not a Codecov service percentage.

Run YAML/actionlint, planning regression/negative cases, locked dependency checks
and local Rust LCOV. Publish the final workflow to PR #1 and inspect actual job and
Codecov results. On a disposable same-repository branch, exercise actual Clippy,
test and upload failures; delete only the branch created for these probes afterward.
Record exact commits/runs and outcomes here. Source/configuration tests alone do
not establish successful upload or visible fork checks. P0-04 remains in review
if required external evidence is deferred or unavailable.
