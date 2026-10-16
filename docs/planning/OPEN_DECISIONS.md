# Requirements, proposals, constraints, decisions and risks

Scope: user accepted **DNS/proxy MVP before tunnels**, CR-0001 on 2026-10-15.
[ADR-0016](../architecture/adr/ADR-0016-dns-proxy-mvp-before-tunnels.md) records the explicit
override to original §23.2/§24.1/§24.2 ordering. Technical mechanisms remain proposed.
Private Linux VPS, separate ports, strict proxy fail-closed and owner plus later admins are confirmed.
The owner accepted P0-01/CR-0002 on 2026-10-15 and authorized work through P0-02:
[contracts and design walkthroughs](P0_01_FOUNDATION_REVIEW.md),
[threat model](../security/THREAT_MODEL.md). No technical ADR was automatically accepted.

Subsequent user answers require public access from changing IPs without VPN and
DNS-setting-only clients. After discussing the shared transparent gateway's lack
of individual device evidence, the user explicitly accepted **network-wide access
and one profile shared behind the external IP**. A registered IP is therefore a
coarse group grant, not a device identity. The subsequent Control D research request
produced accepted [CR-0002](../architecture/adr/ADR-0017-profile-endpoint-dns-product-model.md):
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
| D03 | Accepted Debian 13 x86_64 first target; no operator target supplied; pin actual image/kernel in lab; VPN kernel matrix deferred | P0-01 assumptions / P0-03 evidence | Operator; real VM/VPS |
| D04 | Private admin and separate ports confirmed; allocate DNS/gateway/proxy bindings | P4-01/P11-01 | Operator; occupied-port tests |
| D05 | Confirmed public changing-IP VPN-free DNS setup; accepted personal DoH/DoT capabilities select DNS Profiles, while legacy/gateway remain separately group-authorized | P0-01 / P3-07/P4-01 | Group tradeoff and CR-0002 foundation accepted; actual transport/NAT proof pending |
| D06 | Coherent IPv4/IPv6 inside DNS/proxy boundary, with explicit uncontrolled-client limits | P3-04/P4-08 | Network owner; captures |
| D07 | Select controlled origin/proxy and 2–3 authorized DNS/proxy reference services; VPN clients chosen later | P4-08 | User/operator |
| D08 | Exact-address manual bootstrap plus disabled-by-default secure observation learning; separate DNS/gateway scopes, bounded expiry, cause/revocation and no conflicting last-query Profile switch; no automatic /64 | P0-01 / P4-01/P3-06/P3-07 | Foundation accepted; numeric/clock limits and actual source-mismatch/conflict/expiry/derived-revocation tests |
| D09 | BLOCK answer, DNSSEC override and HTTPS/SVCB compatibility behavior | P3-04/P3-05 | Network owner; validating client tests |
| D10 | Wildcard excludes apex; priority/specificity/stable ordering and conflict rejection | P2-01 | Product owner; properties |
| D11 | Snapshot canonical format/auth/epoch and rotation policy | P1-05 | Security owner; replay/crash tests |
| D12 | TUN-to-Xray bridge and privileged routing design deferred | Post-MVP P5/P9 | Real-engine spike before claiming support |
| D13 | WG/AWG/Xray kernel/client/engine versions and distribution inventory deferred | Post-MVP P5-01 | Version-specific lab evidence |
| D14 | Owner selected Apache-2.0 for original Routira code; on 2026-10-16 approved Apache-2.0/MIT/ISC/BSD-3-Clause/Unicode-3.0 for dependencies with preserved texts/notices. P0-05 pins the qualification inventory; actual shipped-target notices must be verified at release; engine redistribution later | P0-05/P11-05 / later engine release | Owner decisions; DEPENDENCIES.md and lockfile |
| D15 | Separate master key/recovery, access-secret lifecycle and retention | P1-07 | Security/operator owner |
| D16 | CodeScene baseline/daily analysis/PR delivery verified; review 7980486 on e4f4143 fails auxiliary planning/graph/fixture score gates; no score-only cleanup, suppression or gate change; product baseline later, MCP optional | P0-04 review; auxiliary tools are outside the owner's independent metric scope | User/maintainer; see P0_04_CI_REVIEW.md for current evidence |
| D17 | Separate planning/backend reports processed for main and P1-01; meaningful exact-base/head PR comparison and backend patch check verified. Actual fork test remains owner-deferred; frontend has no package yet | Fork revisit below; frontend coverage at its owning gate | P1_01_DOMAIN_REVIEW.md and P0_04_CI_REVIEW.md |
| D18 | Measured DNS/TCP/proxy resource budgets; engine budgets later | P10-04 | Operator |
| D19 | Owner selected HTTP CONNECT over TLS on 2026-10-16; verify proxy certificate/hostname, protect credentials, send checked numeric destination; no managed engine dependency; public explicit inbound remains a separate decision | P0-05/P4-05 | Isolated dependency qualification approved; actual runtime proxy/egress proof stays P4-05/P4-08 |
| D20 | Accepted custom > service > filter > default ranks, explicit domain match modes and one active Profile per Endpoint; stacks/schedules deferred | P0-01/P2-01/P2-02 | CR-0002; golden/permutation/conflict tests |
| D21 | Personal DoH/DoT token/hostname provisioning, TLS certificate lifecycle and optional IP restriction/learning bootstrap; possession is not hardware proof | P0-01/P3-07/P6-03 | CR-0002; redaction/rotation/revocation/roaming tests |
| D22 | Bounded local catalog formats, source/license/hash evidence and isolated private DNS records; remote feeds/DDNS/router helper deferred | P0-05/P2-07/P3-04 | CR-0002; real import/update rollback and scope-isolation tests |
| D23 | Owner selected UUIDv4 public resource IDs and qualification of uuid 1.28.0 on 2026-10-16. Domain newtypes use std parse/format only and expose no uuid dependency types. Public credential-record IDs and resolver secret material remain separate; domain association is not token verification or gateway authority. ID/token generators, wire/entropy/size/clock policy and secret lifecycle remain at their owning gates. | P1-01 / P1-07/P3-07/resource creation APIs | P1_01_DOMAIN_REVIEW.md; Windows qualification/audit, implementation Linux/Windows CI and independent Code Review passed |
| D24 | Owner accepted one UTF-8 JSON/YAML/TOML document, 1 MiB, collection depth 32, aggregate 10,000 resources/rules, unknown/duplicate rejection and strict YAML without expansion/include/substitution. Errors expose only stable code, known path and available numeric location. Large dataset payload budgets stay P2-07. | P1-02 | Direct owner answer on 2026-10-16; P1_02_CONFIG_REVIEW.md and api/schemas/README.md |
| D25 | Owner approved qualification of Serde 1.0.229, serde_json 1.0.154, toml 1.1.8+spec-1.1.0 and serde-saphyr 2.0.0, then serde_path_to_error 0.1.20 qualification/adoption if successful. Windows isolated probes/audit passed before config-only adoption; production audit passed. No raw diagnostics or new license exceptions. | P1-02 | Official archive/notices, CONFIG_INVENTORY.csv; implementation Linux/Windows CI and independent re-review passed after four concrete fixes; final-head merge gates remain |

The owner explicitly accepted the P0-01 foundation and CR-0002, then authorized
P0-02. Other technical details and later task execution retain their own gates.

## Deferred P0-04 evidence

The owner accepted P0-04 on 2026-10-16 with these disclosed evidence gaps. They are
not successful tests or permission to change access, spend money or run a fork.
Record future minor review findings here (or at their owning backlog task) with
impact, reason for deferral and a task/revisit condition before merging their PR.
Serious defects cannot be deferred to permit merge; uncertain cases require the owner.

| Item | Impact and reason for deferral | Revisit / owner |
|---|---|---|
| P0-04-FORK | Actual private-fork checks are unverified. Same-repository positive/negative runs and local permission checks exist; the owner explicitly declined the temporary fork and accepted P0-04 with this evidence pending. This does not permit privileged/untrusted fork execution. | Owner approval of a real private-fork test environment; required before claiming fork PR support has been exercised |
| P0-04-CODECOV | Resolved on 2026-10-16: owner-approved activation preserved OIDC/access/plan; both main reports and web report verified. PR #4 head 4d79b7a against main a77ed5a has a processed meaningful comparison, 71/71 new Rust lines and a successful 100% backend patch check. Backend is 76/79; aggregate totals include auxiliary Python and are not application coverage. | Evidence: P1_01_DOMAIN_REVIEW.md and the linked Codecov PR comparison. No fork success or runtime security guarantee follows from coverage |

## P0-05 review dispositions

| Item | Finding, impact and rationale | Owning task / revisit trigger |
|---|---|---|
| P0-05-CS | CodeScene 7981306 flags Large Method in the bounded CONNECT qualification test, score 9.60; its other two gates passed. No concrete functional defect was identified by the independent review. Tests are outside the owner's independent metric scope; keep the readable end-to-end assertion together rather than refactor solely for a score. The repository-wide check is failed, not suppressed or reported as passed. | Revisit if a concrete test-maintenance or functional defect justifies a change; no independent metric cleanup task |
| P0-05-NOTICES | Four unused-target packages have SPDX metadata but no top-level license text in their package archive: jni, jni-macros, jni-sys-macros and r-efi. They are not built for the tested Windows/Linux targets, so this does not block the current qualification. Metadata/policy approval alone does not establish redistribution notices. | P11-05 or before adding the affected targets/features: obtain exact upstream texts/attribution, preserve them and verify the actual shipped target inventory |
| P0-05-SYN | All-target lockfile contains syn 2.0.119 for Android JNI macros and 3.0.9 for current macros; no competing syn versions appear in the tested Windows/Linux target trees. Retain required versions rather than force an incompatible dependency change. | Recheck at production adoption, target/feature changes or the next dependency update; no suppression |

## P1-02 review dispositions

| Item | Finding, impact and rationale | Owning task / revisit trigger |
|---|---|---|
| P1-02-PATH | Serde buffering of internally tagged enums can stop a type-error path at the enclosing known object, e.g. `outbounds[1].transport`, instead of its leaf. Rejection and redaction are unchanged; raw parser text would expose input values. Retain the safe container path. Numeric source positions are available for syntax failures, not every DTO/semantic failure. This is diagnostic precision only, not a validation bypass. | P6-02 when API/UI needs leaf-level source mapping; preserve the no-raw-message boundary. See P1_02_CONFIG_REVIEW.md and configuration integration tests |
| P1-02-CS | CodeScene 7991651/7991752/7991872 failed the new-code gate: socket collision predicates 8.41, explicit helper argument counts 9.69, plus a test helper outside independent metric scope. These are maintainability signals; source inspection and wildcard/dual-stack/scope/order tests support the existing explicit checks. No concrete defect was identified from the metrics, no threshold changed and no suppression added. Keep the bounded collision algorithm and explicit context parameters rather than introduce abstractions solely for scoring. Independent review defects are fixed separately and are not deferred here. | Revisit when a concrete maintenance or functional change makes these conditions/parameters unclear or unsafe; recheck current-head CodeScene. P1_02_CONFIG_REVIEW.md records actual results |

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
