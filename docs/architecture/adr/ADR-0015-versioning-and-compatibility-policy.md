# ADR-0015: Versioning and compatibility policy

Status: **Proposed**. Date: 2026-10-15. Specification: §§7,15,18,21.

Scope: MVP versions APIs, snapshots and native DNS/proxy deployment. Engine/client compatibility and node negotiation below are post-MVP.

## Context

Library, API, snapshot and external-engine versions do not evolve together.

## Decision

Pin Rust/tool/dependency lockfiles; API major /v1 and explicit snapshot schema/capability versions. Negotiate node support, reject incompatible snapshots, stage engine upgrades with rollback and tested client exports.

## Alternatives

Floating upstream branches; accept unknown fields/capabilities blindly; silently migrate all nodes.

## Reasons

Makes upgrades reproducible and failure modes explicit.

## Consequences

MSRV/dependency feature matrix decided in P0-02; backward fixtures retained and release metadata includes engine/build versions.

## Risks

Mixed node generations, future schema defaults and rollback of database migrations.

## Validation approach

P1-05 fixtures, P5-08 engine versions, P8-03 negotiation and P11-03 upgrade/restore.
