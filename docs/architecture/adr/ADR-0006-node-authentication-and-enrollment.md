# ADR-0006: Node authentication and enrollment

Status: **Proposed**. Date: 2026-10-15. Specification: §§11,19.

Scope: Remote-node enrollment is G4, after the DNS/proxy MVP and protocol expansion.

## Context

Remote nodes must verify controller identity and not keep permanent admin SSH keys.

## Decision

G4 uses short-lived single-use enrollment, pending owner approval and mTLS or approved equivalent. Bind node-scoped authenticated snapshots; rotate/revoke identity with clock/expiry policy.

## Alternatives

Static shared bearer token; permanent SSH orchestration; unverified TLS; trust every enrollment automatically.

## Reasons

Limits replay/theft and separates installation from ongoing management.

## Consequences

Offline nodes retain last-known-good and may not learn revocation immediately; control status must reflect that.

## Risks

Certificate rotation outage, stolen enrollment before first use and controller compromise.

## Validation approach

P8-01/P8-03/P8-05 actual identity/tamper/replay/expiry/partition tests.
