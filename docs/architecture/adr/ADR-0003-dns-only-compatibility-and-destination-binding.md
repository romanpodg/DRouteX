# ADR-0003: DNS-only compatibility and destination binding

Status: **Proposed**. Date: 2026-10-15. Specification: §§3,4,6,9,10,14,19.

## Context

SNI is visible only in some TCP/TLS traffic and is not authorization; NAT erases per-device source evidence.

## Decision

Authorize ingress/service/profile/route independently; allowlist target and use visible SNI only as a candidate. Unknown/ECH-ambiguous destination fails closed. Use the accepted network/group scope; the concrete registration and binding mechanism remains D08.

The user requires public changing-IP access without VPN and DNS-setting-only
clients, and explicitly accepted access/profile shared by the network's external IP.
Accepted CR-0002 adds personal secure DNS capabilities selecting Endpoint Profiles,
including distinct DNS policy behind NAT. Legacy DNS and transparent gateway grants
stay network scoped. Manual registration remains bootstrap/fallback; opt-in secure
observations may produce bounded grants only through authenticated publication.
No last-query gateway Profile switch, permission union or automatic /64 expansion.
Same-NAT peers share the grant; an old reassigned IP remains a risk until expiry/revocation.
The [P0-01 review](../../planning/P0_01_FOUNDATION_REVIEW.md) defines the concrete
contract awaiting owner review before this ADR is accepted or implemented.

## Alternatives

SNI-only arbitrary proxy; public IP→individual profile; hidden MITM; blanket removal of ECH records.

## Reasons

Prevents open proxy/SSRF and preserves actual origin certificates and honest limitations.

## Consequences

DNS-only compatibility subset and QUIC/port/IPv6 limits must be visible. A tunnel is a future alternative requiring its own verified ingress; it is unavailable in MVP. A configured proxy outbound does not imply a public explicit-proxy inbound.

## Risks

Dynamic clients/public DNS-only use may require extra endpoint identity/IP resources; TLS outer names must not select unintended origin.

## Validation approach

P4-01/P4-02 adversarial target tests; P3-05/SVCB, P10-05 NAT/ECH/QUIC/IPv6 cases.
