# P1-04 draft revision and provenance review

Date: 2026-10-17. Status: **owner model accepted; implementation in progress**.
Base: verified PR #6 merge `65c076d6bb00eb7615cc7e697f3f4caeed9e9d7f`.

The owner accepted D29: one global chain of complete configuration drafts, expected
previous revision on every new write, Conflict for concurrent edits without automatic
merge, and atomic trusted Control Plane author/UTC/correlation/revision provenance.
Audit records do not duplicate document contents. Missing legacy author/time must
remain explicitly unknown. RBAC stays P6; a stored draft head is not an active policy,
authenticated snapshot, publication or data-plane authority (P1-05/P1-06).

Reuse existing UUIDv4 parsing/formatting and qualified SQLx/SQLite; no new dependency
or generator is proposed. Existing runtime, queue/busy/reply budgets, owned startup,
permission and secret boundaries remain. Implementation, failure/concurrency/upgrade/
redaction checks, actual Linux CI and independent review are pending.

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
Actual Linux CI, exact-head independent review and external metrics remain pending.

## Local validation before publication

Windows default/no-default tests passed: 36 workspace tests (eight storage, 19 config
including metadata validation), plus four compile-fail examples. All-target check,
fmt, Clippy with warnings denied, rustdoc with warnings denied and locked/offline
cargo-deny passed. Dependency versions/features/lockfiles are unchanged; the four
advisory/license/ban/source categories passed with existing duplicate/unused-license
warnings. Planning self-tests and 17 unit tests passed, including refusal to advance
without completed storage or accepted draft-model evidence. This is not Linux,
external coverage/health or independent Code Review evidence.
