# ADR-0009: Licensing and external-engine distribution

Status: **Proposed**. Date: 2026-10-15. Specification: §§7,19,25.

Scope: MVP inventories runtime dependencies; engine licenses, acquisition and distribution are assessed before G3 integrations.

## Context

Crates and separate external engines have distinct license/distribution conditions; the owner selected Apache-2.0 for original Routira code on 2026-10-15.

## Decision

Owner decision on 2026-10-15: original Routira code uses Apache-2.0, with the
official LICENSE text and inherited workspace metadata. Third-party resources
retain their own licenses. The following distribution/inventory mechanisms remain proposed.

Record exact-version dependency/engine SBOM and notices; keep external redistribution packaging subject to owner review. Prefer verified externally acquired engines until distribution plan accepted.

## Alternatives

Assume separate binaries remove all obligations; bundle newest engine silently; choose project license before inventory.

## Reasons

Avoids treating example stack as an approved legal-compatible release bundle.

## Consequences

cargo-deny policy must be based on selected license posture; exceptions have owner/expiry and cannot hide advisories.

## Risks

Upstream terms may change; linking versus distribution differences; supply-chain origin changes.

## Validation approach

P0-05/P11-05 inventory, verified licenses and artifact checks; appropriate owner review.
