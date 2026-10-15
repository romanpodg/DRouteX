# ADR-0007: SQLite and future PostgreSQL migration

Status: **Proposed**. Date: 2026-10-15. Specification: §§17,21,23.

## Context

Single VPS needs predictable local storage, while later concurrency/HA may change requirements.

## Decision

SQLite WAL with SQLx repositories/migrations and bounded write concurrency; semantic service contracts avoid SQLite leaking into packet processing. PostgreSQL only after measured requirement and approved migration.

## Alternatives

Mandatory PostgreSQL now; storage directly queried in every DNS/flow request; generic ORM framework first.

## Reasons

Preserves simple self-hosting and CP-independent Data Plane.

## Consequences

Backup uses consistent SQLite API, not copying live DB alone; test transactions and migration parity.

## Risks

Writer contention/disk-full; offline encryption keys; schema evolution and future PostgreSQL differences.

## Validation approach

P1-03/P6-07/P11-03 real migration/backup/restore; F-05 parity if needed.
