# ADR-0016: DNS/proxy MVP before tunnels

Status: **Accepted**. Date: 2026-10-15. Scope change: **CR-0001**.
Authority: explicit user instruction to develop MVP with DNS routing and proxies first,
and add tunnels afterward. This accepts release scope, not the whole implementation roadmap.

## Context

Original specification §§23.2/24.1/24.2 placed WireGuard, AmneziaWG and Xray integrations in
single-node MVP. The first roadmap therefore used WG for G1 and all three engines for G2.
The user has explicitly changed the initial product boundary. DNS-only technical limitations
and required security are unchanged.

## Decision

Deliver restricted policy DNS plus authorized proxy/gateway first.
The proposed implementation uses native TCP passthrough and one protected SOCKS5 or
HTTP CONNECT outbound through a narrow stream connector. Exact transport/binding remain
proposed D08/D19 choices; the accepted decision is release scope and order.
MVP requires neither VPN clients nor WG/AWG/Xray/sing-box engines, TUN, NET_ADMIN or a custom
privileged networking helper. Do not build a generic engine framework ahead of need.

Move WG/AWG, engine supervision/exports/UI, privileged helper and actual protocol acceptance
after the released DNS/proxy MVP. Managed Xray is deferred as an engine integration; it remains
a proxy engine rather than being reclassified as a WireGuard-style tunnel.
Remote agents/multi-node and advanced features stay later.

## Alternatives

Keep the original three-engine MVP; retain a WG-first acceptance path despite the new scope;
remove future protocol requirements entirely; implement a privileged framework speculatively.

## Reasons

The accepted DNS/proxy slice is independently useful and testable and removes unnecessary
first-release engine/kernel/client dependencies. Future capabilities remain traceable.
Security requirements do not depend on shipping tunnels.

## Consequences

P4-08 verifies real DNS/TCP/TLS/proxy routing without VPNs. P7-07/P11-06 accept DNS/proxy MVP.
P5/P10-01 are gated after P11-06; P5-09/P5-10 provide deferred UI and actual-engine acceptance.
Backlog delivery tags prevent MVP dependencies on future work.

Original EN/RU specifications remain unchanged; this accepted amendment supersedes only
release applicability/order. Original WG/AWG/Xray/isolation checkboxes remain pending at G3.
Required MFA/RBAC, safe binding, SSRF checks, limits, authenticated snapshots, rollback,
last-known-good, backup/restore and actual-network evidence remain in MVP.
Services use only justified bind permissions and never expose generic privileged operations.

## Risks

Uncontrolled DNS-only clients can bypass configured DNS or use IPv6/QUIC/ECH paths a TCP
gateway cannot safely intercept. Shared NAT IP is not per-device identity. A gateway must not
become an unrestricted public proxy. Exact ingress and first protected outbound remain D08/D19.

## Validation approach

Run the planning DAG/link/traceability checks and a negative test injecting an MVP→tunnel
prerequisite. Verify every source subsection still has a disposition; explicitly mark original
tunnel/engine checklist cases post-MVP. Future acceptance uses real engines, not mocks.
MVP acceptance runs actual DNS/proxy/TLS on a host without WG/AWG/Xray/TUN.
Preserve base specification hashes. No production code is implemented by this amendment.
