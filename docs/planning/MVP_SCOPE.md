# MVP scope and acceptance

Status: **DNS/proxy-only MVP scope accepted by the user on 2026-10-15 (CR-0001).**
The user authorized starting MVP roadmap work on 2026-10-15.
The owner accepted [P0-01/CR-0002](P0_01_FOUNDATION_REVIEW.md) and authorized
work through P0-02; other detailed technical ADRs retain their own gates.
[Accepted scope amendment](../architecture/adr/ADR-0016-dns-proxy-mvp-before-tunnels.md)
supersedes the original specification's first-release WG/AWG/Xray ordering.
The EN/RU base specifications are preserved unchanged.

Further user clarification on 2026-10-15: public remote access from changing IPs,
without VPN; devices need only DNS settings. The user explicitly accepted access
and profiles shared by the network's external IP after discussing why ordinary
shared TCP ingress cannot individually authenticate those devices. This does not
authorize unrestricted public DNS/proxy access. See D05/D08 and the P0-01 review
for the proposed registration, expiry, revocation and same-NAT limitations.

The user subsequently requested a Control D reference analysis and corresponding
project/plan changes. [CR-0002 / ADR-0017](../architecture/adr/ADR-0017-profile-endpoint-dns-product-model.md)
records the accepted [Profile/Endpoint product model](../architecture/DNS_PRODUCT_MODEL.md).
It replaces the manual-IP-only proposal with personal secure DNS plus separate
legacy/gateway group grants. Foundation acceptance is recorded at P0-01; detailed
implementation choices and runtime tests remain pending. The accepted shared-IP limitation applies to legacy DNS
and ordinary transparent TCP, not to personalized secure DNS endpoint selection.

## MVP product boundary

One Linux VPS runs restricted policy DNS and an authorized TCP proxy gateway, managed
through a private authenticated API/dashboard. Initial owner setup supports later
administrators, role assignment, MFA, audit and last-owner protection.

MVP includes:
- Reusable filtering Profiles and named DNS Endpoints, personal rotatable resolver
  capabilities, versioned filter/service catalogs and profile-scoped private records
  in the revised CR-0002 model; one active Profile per Endpoint initially.
- DNS UDP/TCP forwarding, approved upstream selection, TTL/negative caching, local records,
  DIRECT/BLOCK and conditional SmartDNS redirection for supported TCP/TLS services.
- Deterministic profiles/rules, conflict validation, explain/simulation and real DNS/Route Test.
- Native bounded TCP passthrough preserving the origin certificate and encrypted payload.
- Direct egress from the VPS and one selected protected SOCKS5 or HTTP CONNECT outbound
  through a narrow connector; D19 chooses the first concrete proxy transport.
- Authorized ingress/destination binding, trusted independent origin resolution, resolved-IP
  checks/pinning, SSRF/rebinding/loop prevention, quotas, deadlines and fail-closed behavior.
- Versioned authenticated snapshots, coherent DNS/gateway activation, rollback and last-known-good
  forwarding during Control Plane/database loss.
- DNS/proxy/device/profile administration, actual metrics/status, encrypted backup/fresh-host
  restore, native systemd deployment and applicable security/recovery evidence.

MVP does not require any WG/AWG/Xray/sing-box installation, VPN client, TUN interface,
kernel VPN module, routing-table/nftables manager or custom privileged executor.
systemd may grant only a narrow DNS bind capability/socket activation; the API never runs as root
and no service receives NET_ADMIN. Host access controls remain explicit operator prerequisites.

Accepted CR-0002 makes personal DoH/DoT required, separately tested MVP features.
Legacy UDP/TCP remains available for group-authorized older clients. Manual network
registration remains a bootstrap/fallback; scoped secure-query IP learning is opt-in,
bounded, revocable and never grants individual TCP identity. DDNS/router helpers,
profile stacks/schedules and automatic internet filter feeds are deferred.
A public explicit SOCKS/CONNECT inbound is
not implied by configuring an outbound: adding one needs separate secured-ingress requirements.
DNS-only cannot guarantee ECH/QUIC/HTTP3 support or full-device traffic enforcement.

## Two useful milestones

G1: client → real DRouteX legacy/personal DoH/DoT DNS → authorized SmartDNS TCP gateway → controlled TLS origin.
Demonstrate DIRECT/BLOCK, trusted ingress context, unchanged origin certificate and observed VPS
egress, snapshot/version traces, access denial and failure/rollback behavior, with minimal API/UI.

G2 complete DNS/proxy MVP: add the selected protected proxy outbound, full basic UI/diagnostics,
administrator roles/MFA, backup/restore, packaging and release security evidence.
Both gates run on hosts without tunnel engines or TUN. P4-08 is the new vertical slice;
P7-07/P11-06 accept the revised MVP. No application code is created by this scope revision.

## Concrete real network acceptance

Use controlled origins/proxy and two or three authorized reference services (D07).
These fixtures exercise actual DNS/TCP/TLS; mocks cannot replace their acceptance evidence.

| Scenario | Observable pass condition |
|---|---|
| Ordinary DNS DIRECT | Origin A/AAAA returned; capture shows the client connects to origin |
| BLOCK | Predictable DNS denial; forbidden requests inside the controlled proxy boundary are denied |
| Compatible SmartDNS TCP | Restricted ingress and authorized target; origin certificate preserved; expected VPS egress |
| Selected proxy outbound | Actual supported protected TCP proxy handshake and observed egress; no engine/tunnel required |
| Strict unavailable proxy | New proxy flows denied with accurate reason/version; no silent direct fallback |
| NAT/identity | Shared NAT IP never grants individual device authorization; legacy/gateway group scope explicit |
| Personal DNS identity | Two secure endpoints behind one NAT can use different DNS Profiles; legacy/gateway group authority remains separate |
| Learned access | Invalid capability/legacy/client label cannot learn a grant; conflicting gateway scope and DNS/proxy source mismatch fail closed |
| Catalogs/private records | Pinned validated dataset revisions; failed import preserves working policy; private answers isolated and never authorize private-target proxying |
| ECH/QUIC/SVCB/DNSSEC/IPv6 | No guessed destination/authenticity or universal compatibility; coherent supported gateway paths |
| Bad publication/rollback | Complete old/new DNS/gateway generation only; durable recovery; old sessions may disconnect |
| CP/database outage | Last-known-good DNS/proxy service continues |
| Backup/restore | Encrypted consistent SQLite/secret backup restored on a fresh host with separately supplied key |
| Access administration | Owner adds admins; RBAC/CSRF/MFA/access revocation and last-owner guards enforced |
| Privileges | API/node cannot run arbitrary OS networking actions; no NET_ADMIN or custom root executor needed |

DNS-only clients may bypass DNS or retain answers/connections. Fail-closed protects DRouteX's
authorized proxy boundary, not every connection made by an uncontrolled endpoint.
The system cannot force a DNS-only device's IPv6/QUIC/application traffic into the proxy.
DIRECT from a gateway exits at the VPS, not the client's home connection.

## Platform and test boundaries

Windows: model/policy/config/storage/API/user-space TCP/frontend static and unit checks.
Linux WSL: development fixture if available capabilities suffice; never a replacement for
native Linux service/deployment proof. Disposable namespaces can require lab setup privileges,
which do not authorize production services to modify host routes.
Native Linux VM/VPS: actual DNS/TCP/proxy/certificate/egress, service permissions, IPv4/IPv6,
installation, restart, upgrade and restore acceptance. No VPN modules/TUN are MVP prerequisites.

## Deferred with traceable scope

G3, after P11-06: P5 adapters/engine lifecycle, actual WG/AWG tunnels, optional managed
Xray VLESS+REALITY integration, P10-01 privileged executor, compatible client exports/UI
and P5-10 actual-engine/routing/segmentation acceptance.
Xray remains a proxy engine; its deferral does not classify it as a WireGuard-style tunnel.
The original §24.1 WG/AWG/Xray/isolation checklist remains pending at this later gate.

G4: remote Node Agent, enrollment/mTLS/sync, groups/failover/cross-node paths (P8/P9).
G5/G6: justified extra engines, UDP/TUN/router features, optional client, subscriptions,
analytics and PostgreSQL/HA (F tasks). No speculative full adapter framework or VPN configuration
export is built for the DNS/proxy MVP.
