# ADR-0004: Protocol adapter and capability boundaries

Status: **Proposed**. Date: 2026-10-15. Specification: §§7,10,15.

Scope: This adapter design is deferred to G3 under CR-0001. MVP uses one narrow TCP proxy connector and rejects unavailable engine/tunnel capabilities; do not build this framework first.

## Context

Engines differ in peer updates, TCP/UDP/TUN, transport and restart semantics.

## Decision

Use detect/validate/prepare/apply/health/stop with versioned capabilities; peer/rotate/export/metrics optional contracts. Unknown support is unavailable. Engine adapters consume typed plans, not generic shell commands.

## Alternatives

Native implementation of every protocol; one supports_everything boolean; engine JSON exposed as core domain model.

## Reasons

Protects protocol-independent policy while retaining actual engine behavior.

## Consequences

Each integration has a pinned client/engine gate and independent lifecycle; add sing-box only after requirement proof.

## Risks

Version drift; enum/capability compatibility; oversimplified abstraction may miss partial hot reload.

## Validation approach

P5 contract suite plus actual WG/AWG/Xray engine matrix and failure isolation.
