# Development roadmap

This document defines delivery order and gates. The [current execution card](EXECUTION_STATUS.md)
identifies active work and remaining approvals/checks; [backlog.json](backlog.json)
is canonical for task status, dependencies and delivery. Historical PR, SHA and
metric results belong to the linked task reviews and execution history.

Product authority is the preserved [EN specification](../../DRouteX_Project_Specification_EN.md)
with accepted [ADR-0016](../architecture/adr/ADR-0016-dns-proxy-mvp-before-tunnels.md)
and [ADR-0017](../architecture/adr/ADR-0017-profile-endpoint-dns-product-model.md).
The owner authorized roadmap continuation. That authorization does not accept
every proposed technical ADR, change release scope or waive task prerequisites.
Read [AGENTS](../../AGENTS.md) for standing permissions and review/merge policy.

## Product outcome and accepted scope

DRouteX first delivers one Linux VPS with DNS policies and real authorized TCP proxy routing.
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
4. P4-01 current authorization precedes P3-01 public transport and P3-06 DNS redirection. Build forwarding/cache and bounded TCP origin
   validation/passthrough; P4-05 uses a narrow direct/proxy connector, not engine supervision.
5. P3-07 implements personal DoH/DoT and bounded observations after P3-06/P6-01.
   P10-02 privilege/access checks plus P3/P4 produce P4-08 actual DNS/proxy slice;
   do not infer a gateway's per-device identity from secure DNS endpoint selection.
6. Finish P6/P7 API/UI, diagnostics, roles, encrypted restore and native packaging/security gates.
7. P7-07/P11-06 accept DNS/proxy MVP on a host without WG/AWG/Xray/TUN.
8. Only after G2 implement P10-01 and P5, then P8/P9. Future protocol capabilities remain unavailable.

No MVP task may directly or transitively depend on a later-delivery task. The planning validator
enforces this constraint and rejects an injected tunnel dependency.

## Current continuation order under DRouteX

P1-05, P1-06, all seven P2 tasks and P4-01 have completed their normal merge/main
gates; see the execution card and their task evidence. Their historical weak-key
and qualification holds do not block completed work. The owner accepted D45:
P4-01 current authorization precedes P3-01 public transport in the dependency DAG.

P3-01 runtime delivery completed normal merge/exact-main gates through PR109
and corrective PR110 on2026-10-24. A later exact-main run after completion-docs
PR111 failed the durable Linux fixture's strict answer counter; P3-01 is reopened
until its single-datagram fixture correction passes fresh review/head/merge/main
checks. The [D47/D48/D49/D50 contract](P3_01_LISTENER_CONTRACT.md) and
[P3-01 evidence](P3_01_LISTENER_REVIEW.md) preserve both the checked runtime and
later failed run. Isolated hickory-net0.26.3 defaults-off/tokio qualification
remains accepted but paused behind that correction. P3-02 application resources,
fallback and production adoption still need their separate owning contract.

Finish and merge each authorized implementation PR before starting the next.
Subsequent work follows the DAG and owning acceptance contracts. P1-06 library
recovery does not complete P10-06 full-product recovery. Secure learning, actual
origin checking/dialing, TLS forwarding and product deployment retain their gates.

## Approval and update protocol

Use [AGENTS](../../AGENTS.md) for authority resolution and when to ask the owner,
[OPEN_DECISIONS](OPEN_DECISIONS.md) for choices/dispositions, and
[QUALITY_STRATEGY](../quality/QUALITY_STRATEGY.md) for completion rules.
[IMPLEMENTATION_BACKLOG](IMPLEMENTATION_BACKLOG.md), [DEPENDENCY_MAP](DEPENDENCY_MAP.md)
and [REQUIREMENTS_TRACEABILITY](REQUIREMENTS_TRACEABILITY.md) render the plan.
Preserve original requirements and explicit amended dispositions.

When plan data changes, regenerate with `python3.12 tools/render_planning.py`.
For documentation changes run the applicable Q-DOC checks, including
`python3.12 tools/verify_planning.py --self-test`; do not regenerate unrelated
artifacts solely because prose changed. Refresh the local document graph when
changing document relationships if its existing local tooling is available;
do not commit generated graphs or enable a paid/API extraction backend.

Donor PR #2 remains reference-only under
[the accepted reuse assessment](EXECUTION_STATUS.md#accepted-selective-reuse-from-pr-2).
Reuse does not start a future task, waive dependency/license review or authorize a
wholesale donor import. Historical approvals and superseded holds are retained in
execution history and task reports, not repeated as current commands here.
