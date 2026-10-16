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
