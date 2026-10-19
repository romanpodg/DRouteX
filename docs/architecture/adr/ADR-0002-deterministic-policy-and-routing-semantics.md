# ADR-0002: Deterministic policy and routing semantics

Status: **Proposed**. Date: 2026-10-15. Specification: §§2,3,6,8.

## Context

DNS actions, traffic actions and decision-point DIRECT have different effects; rule order cannot depend on maps.

## Decision

Priority then documented specificity then stable identifier; reject contradictory unresolved rules. Normalize IDNA names; wildcard excludes apex unless an exact rule is present. Explain identity trust, rule and revision.

Accepted [CR-0002](ADR-0017-profile-endpoint-dns-product-model.md) refines priority
into explicit custom > service > filter > default ranks, with user priority within
a rank. Exact/DomainAndSubdomains/SubdomainsOnly are distinct match modes.
Explicit BYPASS skips lower rule layers, not safety checks; default BYPASS is only
a fallback. Pin filter/service revisions and include their provenance in Explain.

The owner separately accepted the bounded D37 name profile and D38 comparator/
scoped prepared-selection contract. See [D38](../../planning/P2_02_EVALUATION_CONTRACT.md)
for exact within-layer directions, ambiguity refusal, TCP scope and bounds. This
does not accept all later action/capability, publication or runtime details in this
proposed ADR, which retain their own gates.

## Alternatives

First match by user order; hash-map order; universal domain-from-IP mapping.

## Reasons

Reproducible decisions, transparent conflicts and explicit capabilities meet spec §§6/8.

## Consequences

Compiler and simulator share semantics; strict route default failure is BLOCK; fail-open is separate explicit authorization.

## Risks

Stable-ID tie-break can conceal a contradiction if validation is too permissive; domain-derived IP routing is ambiguous.

## Validation approach

P2 permutation/differential tests; P4-08/P10-05 actual DNS/proxy egress evidence. P5-04 adds tunnel egress at G3.
