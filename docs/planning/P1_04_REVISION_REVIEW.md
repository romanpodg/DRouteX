# P1-04 draft revision and provenance review

Date: 2026-10-17. Status: **complete; PR #7 merged after final review and checks**.
Base: verified PR #6 merge `65c076d6bb00eb7615cc7e697f3f4caeed9e9d7f`.

The owner accepted D29: one global chain of complete configuration drafts, expected
previous revision on every new write, Conflict for concurrent edits without automatic
merge, and atomic trusted Control Plane author/UTC/correlation/revision provenance.
Audit records do not duplicate document contents. Missing legacy author/time must
remain explicitly unknown. RBAC stays P6; a stored draft head is not an active policy,
authenticated snapshot, publication or data-plane authority (P1-05/P1-06).

Reuse existing UUIDv4 parsing/formatting and qualified SQLx/SQLite; no new dependency
or generator is proposed. Existing runtime, queue/busy/reply budgets, owned startup,
permission and secret boundaries remain. Implementation and failure/concurrency/upgrade
checks passed; actual CI, independent review and service results follow below.

## Implementation boundary

`DraftRevision` validates self-parent and supported nonnegative Unix millisecond
metadata; `UserId`/`CorrelationId` use existing UUIDv4 parsing/formatting. No generator,
authentication, signature or serialization of trusted metadata into the config input
is added. A caller supplies trusted Control Plane metadata; clocks may go backwards,
so causal order follows expected-parent links rather than timestamp sorting.

Migration 3 adds immutable per-revision provenance and one draft-head row. All new
configuration saves require matching metadata, compare the current expected parent,
and atomically append document/indexes/provenance plus a conditional head update.
A rejected edit leaves no source/audit/head side effect. Catalog hash claims remain
outside this configuration draft chain and are not dataset-import audit (P2-07).
Old P1-03 documents have LegacyUnknown provenance and no elected head; the first new
audited edit starts a new chain with parent None. This preserves unknown data rather
than inventing history or activating a legacy revision.

Existing callers/tests use the mandatory provenance parameter. Windows concurrency
checks require exactly one winner, explicit Conflict for the other editor and no
loser document; mismatched revision metadata is rejected. That regression was seen
fail after temporarily removing the ID-match guard, then restored (local red log
`.setup/p1-04/provenance-regression-red.log`). Linux tests additionally cover an
injected late tip/audit transaction failure with zero leaked rows, upgrade from
schema 2 without fabricated metadata and durable head/provenance across reopen.
Linux CI and independent implementation review completed on the published head below.

## Local validation before publication

Windows default/no-default tests passed: 36 workspace tests (eight storage, 19 config
including metadata validation), plus four compile-fail examples. All-target check,
fmt, Clippy with warnings denied, rustdoc with warnings denied and locked/offline
cargo-deny passed. Dependency versions/features/lockfiles are unchanged; the four
advisory/license/ban/source categories passed with existing duplicate/unused-license
warnings. Planning self-tests and 17 unit tests passed, including refusal to advance
without completed storage or accepted draft-model evidence. This is not Linux,
external coverage/health or independent Code Review evidence.

## Published implementation evidence

PR [#7](https://github.com/romanpodg/DRouteX/pull/7), implementation head
`ab349f7cf7dd7d9696d03c2164ebea9d5614b0c0`, base
`65c076d6bb00eb7615cc7e697f3f4caeed9e9d7f`:

- All nine CI jobs in [39054676807](https://github.com/romanpodg/DRouteX/actions/runs/39054676807)
  passed. Actual Linux default/no-default runs each passed 46 workspace tests
  (18 storage), plus four compile-fail examples. The additional config
  metadata test is a separate integration binary. Upgrade/reopen, competing drafts,
  late head/audit failure rollback, redaction and existing startup/SQLITE_FULL tests
  passed. Local log: `.setup/p1-04/ci-linux.log`.
- Independent built-in Code Review against that base completed exit 0 without
  confirmed defects, session `01a14af2-3809-7683-8776-3745a060d27a`, read-only sandbox,
  approval never. Log/exit: `.setup/p1-04/codex-review-implementation.log` / `.exit`.
  Reviewer ran fmt, seven planning tests and nine existing Windows binary tests;
  fresh Cargo/Clippy and Linux execution were unavailable in its read-only sandbox.
  Separate exact-head CI supplies those checks. Post-result rollout-recording errors
  remain in the log; they do not replace the completed review result.
- Both exact-head Codecov reports are processed (`complete`, two sessions).
  The exact-base/head comparison contains all storage sources. Rust backend totals
  are 1,635/1,746 lines; backend patch 143/155 (92.25%), with successful backend and
  planning (100%) patch checks. New revision metadata source is 50/55 (90.90%).
  Combined auxiliary Python totals are not application coverage.
- [CodeScene 7999375](https://codescene.io/projects/86145/delta/results/7999375)
  **failed** new-code and critical-rule gates: revision constructors have explicit
  context arguments (9.69); `database::save` has Bumpy Road Ahead (9.39 to 8.54).
  Source inspection and the independent review identified no remaining functional
  defect. These minor maintenance dispositions, impact, rationale and revisit triggers
  are recorded under P1-04-CS in [OPEN_DECISIONS](OPEN_DECISIONS.md). No suppression,
  threshold change or score-only rewrite was made.

Final evidence changes require review on their published head before normal merge.
No API/RBAC, snapshot authentication/publication, encrypted secret storage, physical
OS ENOSPC/power-loss or native Debian service acceptance is claimed by these tests.

## Final review and merge

Final head `b63fd6e95cb1472b4350b15b4fdea612f4028b3a` passed independent read-only,
approval-never Code Review without confirmed defects, session
`01a14afa-f83d-7f03-a277-7264845e9c0d`, exit0. Local log/exit:
`.setup/p1-04/codex-review-final-evidence.log` / `.exit`. The same reviewer sandbox
execution limits described above apply. All nine final-head jobs39055351917 passed.
Both final-head reports were processed, with unchanged exact-base backend patch
143/155 (92.25%) and successful backend/planning checks. CodeScene7999417 **failed**
the same constructor/transaction maintenance gates; P1-04-CS dispositions remain.

PR #7 merged normally at `644348cfe53a4b76f8a921cf3e4886b2486e0cb8` on
2026-10-17T17:54:02Z. Its tree matches the reviewed head. All nine main jobs39055754490
passed and Codecov processed both exact-merge reports. Full local handoff:
`.setup/p1-04/autopilot-completed.json`. Do not repeat this PR loop.
