# Development roadmap

Status: **P0/P1 and P2-01/02/03/04/07 complete and merged; PR16 exact-main verified; P2-05/PR105 complete; P2-06 D44 Explain/simulation in progress; actual fork test deferred.**
Authorization to start is recorded in the [foundation review](P0_01_FOUNDATION_REVIEW.md).
Concrete technical ADRs are not automatically accepted by that authorization.
The user's bounded execution goal and repository publication/PR-monitoring steps are
recorded in [EXECUTION_STATUS](EXECUTION_STATUS.md). The owner accepted P0-04 on
2026-10-16 and permits PR #1 merge after Code Review/current-head checks with no
unresolved serious defects. PR #1 subsequently merged after those checks. The owner
authorized continuing the plan; P0-05 completed and PR #3 merged after reviews and
all seven head/main CI jobs. P1-01 has owner-selected UUIDv4 and qualified uuid
1.28.0; implementation review, all seven CI jobs, CodeScene and meaningful Codecov
patch comparison passed. PR #4 merged at `cd2a9c011e4950d80458cfaa72baddd1ac4243c2`
after final evidence review and all seven final-head/main jobs. P1-02 now implements
the owner-approved bounded JSON/YAML/TOML configuration boundary. PR #5 merged
normally at `57ce7973228d1c2153601b58f82c08c9cd9302fa` after four concrete review
fixes, independent final evidence review and all seven final-head/main CI jobs.
See [P1-02 evidence](P1_02_CONFIG_REVIEW.md). P1-03 starts with separately approved
SQLx/SQLite qualification, its minimal upstream patch and explicit secret/writer
boundaries. See [P1-03 evidence](P1_03_STORAGE_REVIEW.md).
P1-04/PR #7 merged normally at644348c after independent implementation/final reviews
and all nine head/main jobs. See [P1-04 evidence](P1_04_REVISION_REVIEW.md). P1-07
completed in reviewed PR #8 at36cb5e3, with all11 exact-head/main jobs and both
exact-merge Codecov reports processed. Main attempt1 cold-pool fixture timeout and
successful retry remain recorded. P1-05 completed initial Ed25519 qualification
using the pinned crypto profile. The owner accepted D11 codec/replay/rotation;
D35 strict trust and the candidate codec are now reviewed/tested in cd0e2c5.
P1-06 durable activation is authorized under the owner-accepted
[D36 filesystem profile](P1_06_ACTIVATION_CONTRACT.md); the implementation passed
independent review and all11 jobs on3c12bba. See [evidence](P1_06_ACTIVATION_REVIEW.md).
PR11 subsequently passed final-head review/all11 jobs, merged at6b58b89 and passed
all11 exact-main jobs39154498259 plus processed Codecov. P2-01 then merged normally
at6fa4759 after reviews and all11 final-head/main jobs; Codecov processed both reports.
P2-02 is authorized under the owner-accepted [D38 profile](P2_02_EVALUATION_CONTRACT.md)
and merged normally at ecc8a9e after reviews, all11 final-head/main jobs and processed
Codecov. P2-03 accepted [D39 validation](P2_03_PUBLICATION_CONTRACT.md) merged at a50c6a4
after reviews and all11 final-head/main jobs; exact-main Codecov processed both reports.
P2-04 accepted [D40 strict actions](P2_04_ACTION_CONTRACT.md) merged at b4c104c
after reviews and all11 final-head/main jobs; Codecov processed both exact-main reports.
See [evidence](P2_04_ACTION_REVIEW.md). P2-07 accepted
[D41 local catalogs](P2_07_CATALOG_CONTRACT.md) merged normally at e8c5e61; all11
exact-main39546838691 jobs and both processed reports passed before P2-05 code.
See [completed evidence](P2_07_CATALOG_REVIEW.md). P2-05 now implements the accepted
[D43 complete compiler](P2_05_COMPILATION_CONTRACT.md); [evidence](P2_05_COMPILATION_REVIEW.md).
P2-05 merged normally at445f847; all11 exact-main39552210853 jobs and both reports passed.
P2-06 now follows the [accepted D44 contract](P2_06_EXPLAIN_CONTRACT.md);
[current evidence](P2_06_EXPLAIN_REVIEW.md). Later implementation remains planned.
See [P1-05 evidence](P1_05_SNAPSHOT_REVIEW.md).
See [P1-07 evidence](P1_07_SECRET_REVIEW.md).
Further technical choices retain their owning gates.
Primary source: [base specification](../../DRouteX_Project_Specification_EN.md), all 27 sections,
with [accepted scope amendment ADR-0016](../architecture/adr/ADR-0016-dns-proxy-mvp-before-tunnels.md).
Base EN/RU specifications remain unchanged. A minimal common/config/policy workspace
passed Windows/Linux checks; runtime DNS/proxy services do not exist yet.
The user authorized [Control D research](../research/CONTROLD_ANALYSIS.md) and plan
adaptation. Accepted [CR-0002](../architecture/adr/ADR-0017-profile-endpoint-dns-product-model.md)
introduces reusable Profiles, DNS Endpoints, personal DoH/DoT, filter/service catalogs,
private records and separately scoped gateway grants. The owner accepted its foundation contracts; implementation evidence remains pending.

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

## Current continuation order under DRouteX

The owner reaffirmed the following order on 2026-10-18 after the project rename.
This orders existing tasks; it does not change their dependency DAG, accept a new
security solution or waive any task gate. P1-05 subsequently passed its D35 gates;
PR9 merged at521bfa7 and all11 exact-main jobs passed. The owner then accepted D36
for P1-06. P2 remains planned until reviewed P1-06 merge.

| Order | Work and acceptance boundary |
|---|---|
| 1. Finish P1-05 | First obtain separate authorization for a weak-key resolution and qualify it. Preserve the failing weak-key regression. Then verify signature/key refusal, tampered authenticated metadata/payload, wrong node/resource pins, incompatible/noncanonical/oversized input, equivalent-format golden bytes, replay/equal/lower sequences, rotation and rollback with a fresh sequence. Require actual Windows/Linux checks and independent implementation/final-head Code Review before merging PR #9. Initial primitive qualification alone does not pass this task. |
| 2. Complete P1-06 | Implement staged validation/activation, durable journal and replay high-water, complete old/new generation recovery and last-known-good operation without CP/database access. Inject crashes at every transition, disk-full, cancellation and restart; key changes and rollback must never reset the sequence floor. Use controlled DNS/gateway consumers to prove one generation per decision; actual DNS/proxy services are later P3/P4 work, and their end-to-end evidence remains there. Exact persistence/activation choices retain their owning approval gate. |
| 3. Implement P2 | P2-01 normalization and explicit apex/subdomain modes; P2-02 deterministic scoped precedence; P2-03 conflict rejection; P2-04 distinct DNS/gateway actions and strict failures; P2-07 bounded versioned local filter/service imports; P2-05 compilation with pinned catalogs; P2-06 explain/simulation. Catalog imports precede compilation. Require permutation/conflict/negative cases, compiler/interpreter equivalence and honest simulation output without claimed network execution. |

Finish and merge each authorized implementation PR before starting the next.
P2-01/P2-07 have earlier prerequisites in the DAG, but this execution order does
not use them to bypass the current P1-06 acceptance gate. P1-06 is foundational recovery, not
completion of the later P10-06 full-product recovery acceptance.

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
recorded in P0_03_NETWORK_HARNESS.md. The owner subsequently accepted P0-04 with
explicitly deferred fork/report evidence; see [CI review](P0_04_CI_REVIEW.md).
PR #1 passed Code Review after its fixture fix and merged. The owner then authorized
continuation; P0-05 qualified the selected HTTP CONNECT over TLS dependency stack,
including actual Windows/Linux tests, supply-chain checks and independent Code Review.
Other technical choices retain their gates.
Detailed implementation decisions and runtime proof remain at their owning gates.

The owner selected selective reuse from PR #2 on 2026-10-15 (option A).
PR #1 remains the foundation; the [donor assessment](EXECUTION_STATUS.md#accepted-selective-reuse-from-pr-2)
maps candidates to existing tasks. This does not start those tasks or waive the
task prerequisites, dependency/license review or CR-0002 contracts.

P1-05 continuation hold2026-10-18: all11 qualification jobs39063908865 and independent
read-only/never review completed on0c3084d. The owner accepted D11, then chose to
leave the subsequent weak-key security gate open. PR9 remains draft; implementation
prototype is local/uncommitted/unreviewed. Do not qualify/adopt ed25519-dalek, merge,
repeat the answered question or start another implementation PR without a new owner
authorization. See P1-05-WEAK-TRUST and `.setup/roadmap-autopilot-state.json`.

D35 update2026-10-18: the owner subsequently approved ed25519-dalek3.0.0
qualification and conditional adoption after successful Windows/Linux tests,
dependency/license checks and independent review. This supersedes the earlier
no-qualification hold; production acceptance and PR merge remain evidence-gated.

P1-05 acceptance2026-10-18: D35 qualification3bd7db9 and implementationcd0e2c5
passed independent read-only/never reviews and all11 jobs on each exact head.
The weak-trust defect is resolved; actual Rust patch597/621 and failed maintenance
checks are disclosed in P1_05_SNAPSHOT_REVIEW.md. PR9 final current-head review/checks
remain mandatory before normal merge; P1-06 is next and not yet implemented.
