# Development roadmap

Status: **P0-01/P0-02/P0-03 complete; P0-04 CI in review, real fork test deferred.**
Authorization to start is recorded in the [foundation review](P0_01_FOUNDATION_REVIEW.md).
Concrete technical ADRs are not automatically accepted by that authorization.
The user's bounded execution goal and repository publication/PR-monitoring steps are
recorded in [EXECUTION_STATUS](EXECUTION_STATUS.md). The owner authorized P0-04 on
2026-10-16; P0-05 and merging are not authorized.
Primary source: [base specification](../../Routira_Project_Specification_EN.md), all 27 sections,
with [accepted scope amendment ADR-0016](../architecture/adr/ADR-0016-dns-proxy-mvp-before-tunnels.md).
Base EN/RU specifications remain unchanged. A minimal common/config/policy workspace
passed Windows/Linux checks; runtime DNS/proxy services do not exist yet.
The user authorized [Control D research](../research/CONTROLD_ANALYSIS.md) and plan
adaptation. Accepted [CR-0002](../architecture/adr/ADR-0017-profile-endpoint-dns-product-model.md)
introduces reusable Profiles, DNS Endpoints, personal DoH/DoT, filter/service catalogs,
private records and separately scoped gateway grants. The owner accepted its foundation contracts; implementation evidence remains pending.

## Product outcome and accepted scope

Routira first delivers one Linux VPS with DNS policies and real authorized TCP proxy routing.
No VPN client, tunnel engine, TUN or custom privileged network executor is needed to reach MVP.
The owner can later add administrators and assign roles. Private management, separate ports,
MFA/RBAC, fail-closed proxy policies, secret protection, recovery and honest diagnostics remain.

DNS chooses answers/upstreams; it does not route traffic by itself. Compatible SmartDNS rewrites
can direct a TCP/TLS client to the gateway; independent ingress and destination authorization
are mandatory. Native gateway direct egress is already proxying client traffic through the VPS.
One verified protected proxy outbound can select a further exit without a VPN tunnel.

## Delivery gates and specification phase mapping

P0–P11 are workstream identifiers, not chronological release stages.
CR-0001 explicitly moves original first-release protocol requirements to a later release.

| Gate | Deliverable | Exit evidence |
|---|---|---|
| G0 Foundation | Approved DNS/proxy contracts, minimal workspace, storage/policy/snapshots, early auth, isolated DNS/TCP test harness | P0/P1/P2, early P6-01/P10-02; no tunnel/helper dependencies |
| G1 First DNS/proxy slice | Real legacy and personal DoH/DoT endpoint policy → separately authorized TCP gateway → actual TLS origin; minimal UI/API | P3-07 before P4-08; same-NAT DNS Profiles, unchanged certificate, egress, grant/mismatch/failure/rollback negatives |
| G2 DNS/proxy MVP | Profiles/Endpoints/Filters/Services/private records/Proxy Outbounds management, scoped opt-in IP learning, MFA/RBAC, encrypted restore/systemd | All 61 MVP tasks, especially P7-07/P11-06; CR-0001 scope and accepted CR-0002 model |
| G3 Protocol/tunnel expansion | Independently verified WG/AWG, managed Xray if included, restricted helper, compatible exports/UI | P5/P10-01 after G2; P5-10 preserves original deferred §24.1 cases |
| G4 Multi-node | Remote enrollment/control/snapshots, groups, verified cross-node routes and explicit partial deployment | P8/P9 real two-node acceptance after G3 |
| G5 Advanced networking | Justified extra engines/UDP/TUN/router policies | F-01/F-02 separate approved work |
| G6 Mature platform | Optional client/subscriptions/analytics/PostgreSQL/HA | F-03–F-05 separate requirement and evidence |

**G2 now means DNS/proxy MVP. WG/AWG/Xray are not its release blockers.**
Their tasks and acceptance evidence remain in the backlog; they have not been deleted or passed.

## Workstreams and task counts

| Workstream | Tasks | Delivery |
|---|---:|---|
| P0 Foundation | 5 | MVP: conventions, CI, DNS/proxy lab and dependency/license inventory |
| P1 Domain/configuration | 7 | MVP: current domain types, SQLite/secrets/revisions/snapshots |
| P2 Policy Engine | 7 | MVP: versioned local catalogs, layered deterministic DNS/proxy decisions; reject unsupported future actions |
| P3 DNS | 7 | MVP: restricted forwarding/cache/overrides/compatible redirects/diagnostics |
| P4 Proxy Gateway | 8 | MVP: binding/origin/passthrough/outbound/failure/diagnostics and P4-08 slice |
| P5 Protocol adapters/tunnels | 10 | Post-MVP: engines, peers/exports, UI extension and actual acceptance |
| P6 Control API | 8 | MVP: auth, domain/rules/publication/diagnostics/backup/access |
| P7 Web interface | 7 | MVP: DNS/proxy administration and acceptance without tunnel onboarding |
| P8 Remote Node Agent | 6 | Post-MVP multi-node |
| P9 Multi-node networking | 5 | Post-MVP multi-node |
| P10 Security/resilience | 7 | Six MVP tasks; P10-01 privileged network executor deferred to G3 |
| P11 Release | 6 | MVP: DNS/proxy install/restore/docs/artifacts/release |
| F Future releases | 5 | Separate justified future work |

Total: **88 tasks: 61 MVP, 11 post-MVP protocols/helper, 11 multi-node, 5 future.**
Stable existing IDs remain; P4-08/P5-09/P5-10 separate the new MVP slice from deferred
protocol UI/acceptance. Complexity S/M/L is scope, not calendar duration.

## Practical execution sequence

1. P0-01 approves revised architecture/binding/access and test-target assumptions.
2. P0-02 creates only required crates; P0-03 establishes real DNS/TCP/proxy fixtures.
   P0-05 inventories MVP dependencies/licenses and selects the first proxy transport.
3. P1 storage/revisions/secrets/snapshots and P2 policy; P6-01 starts authentication early.
   P1 separates Profile/Endpoint/resolver capability/grants; P2-07 imports pinned local
   filter/service revisions before P2-05 compiles layered policy.
4. P4-01 binding precedes P3-06 DNS redirection. Build forwarding/cache and bounded TCP origin
   validation/passthrough; P4-05 uses a narrow direct/proxy connector, not engine supervision.
5. P3-07 implements personal DoH/DoT and bounded observations after P3-06/P6-01.
   P10-02 privilege/access checks plus P3/P4 produce P4-08 actual DNS/proxy slice;
   do not infer a gateway's per-device identity from secure DNS endpoint selection.
6. Finish P6/P7 API/UI, diagnostics, roles, encrypted restore and native packaging/security gates.
7. P7-07/P11-06 accept DNS/proxy MVP on a host without WG/AWG/Xray/TUN.
8. Only after G2 implement P10-01 and P5, then P8/P9. Future protocol capabilities remain unavailable.

No MVP task may directly or transitively depend on a later-delivery task. The planning validator
enforces this constraint and rejects an injected tunnel dependency.

## Approval and update protocol

Review [MVP_SCOPE](MVP_SCOPE.md), [OPEN_DECISIONS](OPEN_DECISIONS.md) and proposed technical ADRs.
[backlog.json](backlog.json) is the status/dependency/delivery source;
[IMPLEMENTATION_BACKLOG](IMPLEMENTATION_BACKLOG.md) is its reviewable rendering.
[DEPENDENCY_MAP](DEPENDENCY_MAP.md) and [REQUIREMENTS_TRACEABILITY](REQUIREMENTS_TRACEABILITY.md)
show exact dependencies and explicit amended dispositions. No original requirement is silently lost.

After changes run `python3.12 tools/render_planning.py` and
`python3.12 tools/verify_planning.py --self-test`; refresh the local graph.
The scope amendment alone did not authorize implementation. The later user instruction
to begin MVP work authorizes P0-01, now accepted with a
[threat model](../security/THREAT_MODEL.md), three flow walkthroughs and five failure cases.
Public changing-IP, VPN-free DNS-only access with legacy/gateway network scope is confirmed.
CR-0002 adds personal DNS Profiles and opt-in observed network grants to the proposed
model; it does not turn group-authorized TCP into individual device authorization.
P0-01 foundation contracts and Debian 13 x86_64 assumptions are accepted. Continue
only through P0-03, authorized by the subsequent next-step instruction and explicit
WSL lab approval. The owner approved the separate Debian VPS lab on 2026-10-16;
native fixture and cleanup evidence passed and closes P0-03. Both environments are
recorded in P0_03_NETWORK_HARNESS.md. The owner subsequently authorized P0-04;
see [CI review](P0_04_CI_REVIEW.md). Do not start P0-05 yet.
Detailed implementation decisions and runtime proof remain at their owning gates.

The owner selected selective reuse from PR #2 on 2026-10-15 (option A).
PR #1 remains the foundation; the [donor assessment](EXECUTION_STATUS.md#accepted-selective-reuse-from-pr-2)
maps candidates to existing tasks. This does not start those tasks or waive the
task prerequisites, dependency/license review or CR-0002 contracts.
