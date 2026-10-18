# ADR-0010: Runtime snapshots and durable activation

Status: **Proposed**. Date: 2026-10-15. Specification: §§5,8,11,19,20.

Scope: MVP coordinates DNS/gateway generations. Kernel/engine activation and remote rollout below apply only to the later releases that introduce those components.

## Context

Database commit cannot atomically change several kernel interfaces and external processes; rollback and anti-replay coexist.

## Decision

Immutable canonical content/hash plus schema/source revision/node scope/auth/increasing activation epoch. Stage/validate/prepare/journal/guard ingress/activate/verify; acknowledge complete state. Rollback re-publishes old content with a new authorized epoch.

## Alternatives

Mutable global live rules; activate before validate; distributed global transaction; permit arbitrary sequence rollback.

## Reasons

Makes per-node outcome explicit and prevents partial accepted policy or replay.

## Consequences

Readers pin one generation; old sessions can drain. Multi-node activation tracks divergence rather than claiming simultaneity.

## Risks

Crash ordering, disk flush guarantees, stale DNS route bindings and leaked intermediate OS routes.

## Validation approach

P1-05/P1-06 every-transition fault tests, P10-06 recovery and P9-04 partition acceptance.

## Implemented bounded subset, 2026-10-18

Owner-accepted D11/D35 is implemented in P1-05: bounded authenticated immutable
candidates, canonical typed JSON, strict public-key trust and explicit in-memory
sequence checks. See [candidate v1 contract](../../../api/schemas/snapshot/README.md)
and [acceptance evidence](../../planning/P1_05_SNAPSHOT_REVIEW.md). This does not
accept this ADR wholesale: its status stays Proposed. Durable publication,
activation/recovery, API and later remote/helper contracts retain their owning gates.

The owner subsequently accepted D36's bounded local filesystem profile for P1-06.
See [activation contract](../../planning/P1_06_ACTIVATION_CONTRACT.md). Its
implementation/recovery passed independent review and actual Windows/Linux CI
on3c12bba; PR11 finalization remains gated. This does not accept the complete
ADR or the later kernel/engine/remote activation mechanisms.
