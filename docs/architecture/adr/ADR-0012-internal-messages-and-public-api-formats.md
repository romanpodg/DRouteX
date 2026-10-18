# ADR-0012: Internal messages and public API formats

Status: **Proposed**. Date: 2026-10-15. Specification: §§12,17,18,19.

Scope: MVP has browser/API and local snapshot contracts; privileged-helper messages are G3 and remote-node messages G4.

## Context

UI/API, local helper and future remote agent expose different trust boundaries.

## Decision

REST /api/v1 with OpenAPI; bounded serde JSON versioned snapshots/events; narrow typed local IPC requests. Stable reason codes/correlation/revision metadata. Exact auth model gets CSRF/Origin controls.

## Alternatives

Unversioned arbitrary JSON; shared database between services; generic command bus; full event platform now.

## Reasons

Small explicit contracts suffice without premature framework; serialization validates external data.

## Consequences

Generate/verify browser schemas; differentiate policy prediction from actual diagnostics; no secrets in event envelopes.

## Risks

Canonical serialization for signing, unknown fields and schema drift; unsafe general-purpose helper commands.

## Validation approach

P1-02/P6-02 negative/schema/RBAC fixtures; P8-03 version negotiation.

## Implemented bounded subset, 2026-10-18

Owner-accepted D11/D35 is implemented in P1-05: bounded authenticated immutable
candidates, canonical typed JSON, strict public-key trust and explicit in-memory
sequence checks. See [candidate v1 contract](../../../api/schemas/snapshot/README.md)
and [acceptance evidence](../../planning/P1_05_SNAPSHOT_REVIEW.md). This does not
accept this ADR wholesale: its status stays Proposed. Durable publication,
activation/recovery, API and later remote/helper contracts retain their owning gates.
