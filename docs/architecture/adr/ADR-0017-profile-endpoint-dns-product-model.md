# ADR-0017: Profile and endpoint DNS product model

Status: **Accepted**. Date: 2026-10-15. Design revision: **CR-0002**.
Authority: the owner explicitly accepted P0-01/CR-0002 in this chat on 2026-10-15
and authorized continuation through P0-02. This accepts the product/foundation
contracts, not later technical choices or successful runtime behavior. CR-0001 remains accepted.
Specification: sections 8/9/10/12/14/17/19, with preserved EN/RU source texts.

## Context

The first P0-01 proposal used manually updated external-IP grants and optional
encrypted DNS. That covers legacy group access but is too narrow for personalized
DNS on roaming devices. [Official Control D research](../../research/CONTROLD_ANALYSIS.md)
shows distinct Profiles, Endpoints, secure resolver capabilities, category/service
rules and separate source-network authorization for transparent proxy access.

## Decision

Adopt the [DNS product model](../DNS_PRODUCT_MODEL.md): reusable Profiles; named
DNS Endpoints; rotatable secure resolver capabilities; versioned filter/service
catalogs; explicit private records and rule layers; separate DNS/gateway grants;
VPS direct egress and one protected proxy outbound. Require DoH/DoT in revised G2;
include secure endpoint proof in G1. Keep one Profile per Endpoint initially.

Support manual network bootstrap and opt-in, bounded secure-query observations
published as scoped grants. DNS endpoint policy may differ behind NAT; ordinary
transparent TCP remains network/group scoped. Do not switch gateway Profile based
on the last DNS query, expand IPv6 prefixes automatically, or learn arbitrary IPs.

## Alternatives

Retain legacy/manual-IP-only UX; merge physical devices with DNS resolvers; assume
the DNS credential authenticates TCP; clone a distributed SaaS backend; implement
profile stacks, schedules, DDNS and a router client before the core works.

## Reasons

Separating policy, resolver access, datasets and proxy authority matches the requested
product experience and prevents identity confusion. Pinned catalogs and explicit
matching preserve deterministic publication without copying proprietary datasets.

## Consequences

Update existing task scopes and add P2-07 catalog import, with explicit traceability.
The accepted model has 88 tasks, 61 in MVP. Keep source specs and CR-0001 intact.
P3-07 becomes a P4-08 prerequisite; both secure transports remain independently tested.
No new engine/tunnel/helper/multi-node dependency enters MVP. Other technical ADRs retain their Proposed status for details outside the accepted
foundation. The model itself supplies no runtime acceptance evidence.

## Risks

Resolver-secret disclosure, DoT hostname visibility, ambiguous legacy/NAT mapping,
overbroad learned grants, DNS/proxy address mismatch, stale/reassigned IPs, cross-profile
private-answer leakage, unsafe list updates and accidental data-plane CP dependency.
Source documents do not reveal or independently validate the hosted backend internals.

## Validation approach

Planning DAG/source-hash/link/ADR/traceability checks, including negative removal of
the secure-DNS prerequisite. Later real two-endpoint same-NAT/roaming/revocation,
layer precedence, private-answer isolation, catalog rollback, source mismatch,
grant expiry during CP loss and actual TLS/egress tests. No product pass is claimed.
