# ADR-0014: Metrics and structured event ownership

Status: **Proposed**. Date: 2026-10-15. Specification: §§12,18,19,20.

## Context

Aggregate privacy-preserving evidence must distinguish process, transport, route and service health.

## Decision

tracing spans/events with redaction; bounded Prometheus-compatible aggregation and low-cardinality labels. Optional OpenTelemetry only if justified. Unavailable metrics remain null/unavailable.

## Alternatives

Per-domain/user UUID metric labels; permanent full DNS logs; zero for missing counts; ping equals service healthy.

## Reasons

Prevents privacy and resource regressions and fake confidence.

## Consequences

Administrative audit separated from optional activity history; each event records actor/correlation/revision/outcome.

## Risks

Logs expose secrets via Debug/error chains; counter units and sampling can mislead.

## Validation approach

P4-07/P6-06/P6-08 redaction/cardinality/retention tests; P10-04 measured load.
