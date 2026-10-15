# Requirements, proposals, constraints, decisions and risks

Scope: user accepted **DNS/proxy MVP before tunnels**, CR-0001 on 2026-10-15.
[ADR-0016](../architecture/adr/ADR-0016-dns-proxy-mvp-before-tunnels.md) records the explicit
override to original §23.2/§24.1/§24.2 ordering. Technical mechanisms remain proposed.
Private Linux VPS, separate ports, strict proxy fail-closed and owner plus later admins are confirmed.
The user authorized beginning MVP roadmap work on 2026-10-15. P0-01 is in review:
[contracts and design walkthroughs](P0_01_FOUNDATION_REVIEW.md),
[threat model](../security/THREAT_MODEL.md). No technical ADR was automatically accepted.

Subsequent user answers require public access from changing IPs without VPN and
DNS-setting-only clients. After discussing the shared transparent gateway's lack
of individual device evidence, the user explicitly accepted **network-wide access
and one profile shared behind the external IP**. A registered IP is therefore a
coarse group grant, not a device identity. The subsequent Control D research request
produced proposed [CR-0002](../architecture/adr/ADR-0017-profile-endpoint-dns-product-model.md):
personal DoH/DoT Endpoints, reusable Profiles, filters/services/private records and
optional scoped source observation. Manual updates remain bootstrap/fallback;
no installed updater, DDNS polling or router integration is implied.

## Classification and specification tensions

**Explicit current requirements:** deterministic explainable DNS/proxy policy, CP/DP independence,
safe ingress/destination binding, authenticated anti-replay snapshots, coherent activation,
restricted resolver/proxy, real DNS/TCP/TLS/proxy acceptance, roles/MFA/audit, encrypted secrets/
backup/restore, privacy and accurate statuses/limitations.

**Deferred requirements:** WG/AWG/tunnel/TUN/routing management, managed Xray/sing-box,
engine/peer/export UI, privileged executor, remote agents and multi-node. They retain stable tasks
and real future acceptance gates; none is marked completed because it is deferred.

**Proposed architecture:** modest modular Rust workspace; separate CP and DNS/proxy runtime;
SQLite WAL; local snapshot coordinator; narrow TCP outbound connector. Generic engine framework/
network helper are added when post-MVP features require them.

**Constraints:** DNS alone cannot move traffic; ECH hides inner SNI; TCP cannot carry arbitrary
QUIC interception; signed rewrites invalidate DNSSEC; shared NAT identity is ambiguous; gateway
DIRECT exits at the VPS; TTLs and connections persist. DNS-only cannot guarantee endpoint-wide
IPv6/QUIC/application enforcement.

| Issue | Current treatment | Evidence |
|---|---|---|
| Original first-release WG/AWG/Xray vs user's narrower MVP | Accepted CR-0001: DNS/proxy G2, protocols/helper G3 | ADR-0016; dependency validation |
| DNS profile vs trustworthy TCP identity | Trusted network/group or dedicated authenticated context; never NAT-IP individual identity | D08, P4-01 |
| Authorization without a VPN | Restrict gateway ingress/service/target independently; private/trusted network requirements explicit | P4-01/P10-02/P4-08 |
| DNS TTL after publication/revocation | Authorized versioned route grace; revoked/ambiguous target denied | D08, P3-06 |
| IPv6 outside a DNS-only proxy path | Test supported gateway paths; warn uncontrolled clients may bypass; never promise VPN-like enforcement | P10-05 |
| CP/runtime separation and atomic changes | Separate CP; local immutable generation/journal; future kernel/engine transaction issues do not force helper into MVP | P1-06 |
| Proxy transport vs protocol engines | Minimal protected TCP connector; engine supervisor/TUN bridge deferred | D19, P4-05 |
| Privileges | No root API/NET_ADMIN/OS-route actions; narrow DNS bind mechanism only | P10-02/P11-01 |
| Proxy failover vs multi-node | Strict unavailable outbound blocks; general health-selected node groups later | P4-06/P9-02 |

## Decision register

| ID | Decision / current default | Needed before | Owner / validation |
|---|---|---|---|
| D01 | Confirmed owner bootstrap/later admins; define exact role/MFA/recovery/last-owner permission table | P6-01 | Product/security owner; role tests |
| D02 | Confirmed Linux VPS initial product; Windows development host | P0-01 details | Operator |
| D03 | Proposed Debian 13 x86_64 first target; no operator target supplied; pin actual image/kernel in lab; VPN kernel matrix deferred | P0-01 assumptions / P0-03 evidence | Operator; real VM/VPS |
| D04 | Private admin and separate ports confirmed; allocate DNS/gateway/proxy bindings | P4-01/P11-01 | Operator; occupied-port tests |
| D05 | Confirmed public changing-IP VPN-free DNS setup; proposed personal DoH/DoT capabilities select DNS Profiles, while legacy/gateway remain separately group-authorized | P0-01 / P3-07/P4-01 | Group tradeoff accepted; CR-0002 technical review and actual transport/NAT proof pending |
| D06 | Coherent IPv4/IPv6 inside DNS/proxy boundary, with explicit uncontrolled-client limits | P3-04/P4-08 | Network owner; captures |
| D07 | Select controlled origin/proxy and 2–3 authorized DNS/proxy reference services; VPN clients chosen later | P4-08 | User/operator |
| D08 | Exact-address manual bootstrap plus disabled-by-default secure observation learning; separate DNS/gateway scopes, bounded expiry, cause/revocation and no conflicting last-query Profile switch; no automatic /64 | P0-01 / P4-01/P3-06/P3-07 | Owner review; actual source-mismatch/conflict/expiry/derived-revocation tests |
| D09 | BLOCK answer, DNSSEC override and HTTPS/SVCB compatibility behavior | P3-04/P3-05 | Network owner; validating client tests |
| D10 | Wildcard excludes apex; priority/specificity/stable ordering and conflict rejection | P2-01 | Product owner; properties |
| D11 | Snapshot canonical format/auth/epoch and rotation policy | P1-05 | Security owner; replay/crash tests |
| D12 | TUN-to-Xray bridge and privileged routing design deferred | Post-MVP P5/P9 | Real-engine spike before claiming support |
| D13 | WG/AWG/Xray kernel/client/engine versions and distribution inventory deferred | Post-MVP P5-01 | Version-specific lab evidence |
| D14 | Routira license and relevant distribution notices remain open; engine redistribution later | P11-05 / later engine release | Owner review |
| D15 | Separate master key/recovery, access-secret lifecycle and retention | P1-07 | Security/operator owner |
| D16 | CodeScene account/repo/baseline and optional MCP | External P0-04 gate | User/maintainer |
| D17 | Remote GitHub repo/visibility and Codecov OIDC policy | External P0-04 gate | User/maintainer |
| D18 | Measured DNS/TCP/proxy resource budgets; engine budgets later | P10-04 | Operator |
| D19 | Select one first protected SOCKS5 or HTTP CONNECT outbound; no managed engine dependency; public explicit inbound is a separate decision | P0-05/P4-05 | Actual proxy/egress and credential transport test |
| D20 | Proposed custom > service > filter > default ranks, explicit domain match modes and one active Profile per Endpoint; stacks/schedules deferred | P0-01/P2-01/P2-02 | CR-0002; golden/permutation/conflict tests |
| D21 | Personal DoH/DoT token/hostname provisioning, TLS certificate lifecycle and optional IP restriction/learning bootstrap; possession is not hardware proof | P0-01/P3-07/P6-03 | CR-0002; redaction/rotation/revocation/roaming tests |
| D22 | Bounded local catalog formats, source/license/hash evidence and isolated private DNS records; remote feeds/DDNS/router helper deferred | P0-05/P2-07/P3-04 | CR-0002; real import/update rollback and scope-isolation tests |

Scope confirmation did not approve implementation. The later start instruction authorizes
roadmap execution at P0-01; it does not accept all technical ADRs or pass their tests.

## Risk register

| Risk | Impact | Mitigation |
|---|---|---|
| Forged SNI/unrestricted gateway | Critical SSRF/open proxy | Independent ingress authorization/address pinning, P4-01/P4-02/P10-05 |
| Dynamic IP reassignment/shared NAT | All peers at a still-authorized IP may share access; former address may be reassigned | Explicit group scope, bounded expiry, immediate replacement/revocation, quotas; no automatic ownership-detection claim |
| DNS-only expectations/ECH/QUIC/SVCB | Wrong route or false promises | Compatibility validation and prominent honest UI |
| NAT/IPv6/client DNS bypass | False per-device access or traffic guarantee | Trusted scope and supported-path tests, P2-02/P4-08/P10-05 |
| Proxy origin re-resolution/unsafe target | Reserved-range pivot | Check/pin actual dial target through selected connector, P4-05 |
| Inconsistent publication/old TTL | Outage/unsafe path | Generation/journal/recovery and denied revoked routes, P1-06/P3-06 |
| Key loss/admin compromise | Loss of recovery/control | MFA/RBAC/last-owner guard/separate key restore, P1-07/P6-07 |
| Premature engine/helper implementation | Expanded privilege/scope | MVP delivery closure validator; P5/P10-01 gated after P11-06 |
| Later AWG/TUN/Xray/helper risks | Post-MVP integration/security | Retained P5-10 actual-engine/segmentation/helper acceptance |
| External quality service unavailable | Missing external evidence | Local gates continue; explicit activation blockers |
