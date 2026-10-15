# Bounded execution goal and repository handoff

Date: 2026-10-15. Current boundary: **P0-01 review package; P0-02 is not authorized yet**.
Repository: [romanpodg/Routira](https://github.com/romanpodg/Routira), public.
Review: [PR #1](https://github.com/romanpodg/Routira/pull/1), open against main.
The user explicitly authorized the first commit/push, pull requests and PR activity
monitoring. This does not accept technical ADRs or authorize merging a PR.

## Steps and stopping condition

1. Verify the [P0-01 contracts](P0_01_FOUNDATION_REVIEW.md),
   [product model](../architecture/DNS_PRODUCT_MODEL.md), threat model and planning DAG.
2. Establish main with repository exclusions and byte-preserving source attributes.
   Initial commit: c55a6b2, pushed on 2026-10-15.
3. Publish the complete current review package on docs/review-dns-proxy-foundation
   through a pull request. Preserve all local tool/setup/graph exclusions.
4. Run planning CI on Linux and Windows. Inspect current-head checks, review comments,
   and any CodeScene/Codecov status supplied after the user connects those services.
5. Present the review result and stop before P0-02. Start Rust implementation only
   after the owner explicitly accepts P0-01's concrete contracts and assumptions.

The persistent goal covers this bounded preparation/publication/monitoring setup.
PR monitoring continues separately on a schedule in the same chat. A green planning
check, external bot approval or PR creation cannot substitute for owner acceptance.
The active monitor is monitor-routira-foundation-pr, scheduled every 15 minutes.
It reports meaningful changes and stays quiet on unchanged state; it cannot merge
or advance implementation beyond this gate.

## Evidence and external checks

Local checks: python3.12 tools/render_planning.py;
python3.12 tools/verify_planning.py --self-test; reviewed staged diff and credential
patterns; actionlint for the active planning workflow. Exact validation results
remain in [planning-verification.json](../environment/planning-verification.json).

The active [planning workflow](../../.github/workflows/planning.yml) uses Python 3.12,
read-only permissions and immutable action revisions. It verifies planning artifacts
and preserved EN/RU bytes on Ubuntu and Windows. It does not activate the proposed
Rust/frontend workflows or complete P0-04. Source specifications are not normalized
by Git, because their exact bytes are checked by the planning validator.

CodeScene account/repository integration is user-managed and unverified until an
actual result exists. Codecov likewise needs a real coverage-producing workspace,
report and configured upload; installing its app alone cannot create a report.
No coverage/Code Health score or required branch-protection status is invented.

First remote evidence: [Actions run 38706557088](https://github.com/romanpodg/Routira/actions/runs/38706557088)
passed on Ubuntu and Windows for commit 46b7e51 on 2026-10-15. The automatic
Copilot review reported a reviewer quota limit, not a successful code review.
At that observation no CodeScene/Codecov check or coverage report was present.
Later commits must be checked against their own head SHA by the monitor.

Monitor notifications are limited to meaningful check/review/head/state changes,
completion, failures or required user action. Absent unchanged integrations do not
cause repeated reminders. Inspect current PR head to avoid reporting obsolete runs.
No automatic merge, force-push, production deployment, unsolicited reviewer messages
or work beyond P0-01 is authorized by this goal.
