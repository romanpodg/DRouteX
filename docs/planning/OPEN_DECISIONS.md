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
| D14 | Owner selected Apache-2.0 for original DRouteX code; on 2026-10-16 approved Apache-2.0/MIT/ISC/BSD-3-Clause/Unicode-3.0 for dependencies; on 2026-10-17 separately approved Zlib with preserved texts/copyrights/notices and modified-source marking. P0-05 pins the qualification inventory; actual shipped-target notices must be verified at release; engine redistribution later | P0-05/P11-05 / later engine release | Owner decisions; DEPENDENCIES.md and lockfile |
| D15 | External key/rotation accepted in D30-D32; exact envelope, protected persistence/export limits and retention remain design gates before dependent code | P1-07 | Security/operator owner; P1_07_SECRET_REVIEW.md |
| D16 | CodeScene baseline/daily analysis/PR delivery verified; review 7980486 on e4f4143 fails auxiliary planning/graph/fixture score gates; no score-only cleanup, suppression or gate change; product baseline later, MCP optional | P0-04 review; auxiliary tools are outside the owner's independent metric scope | User/maintainer; see P0_04_CI_REVIEW.md for current evidence |
| D17 | Separate planning/backend reports processed for main and P1-01; meaningful exact-base/head PR comparison and backend patch check verified. Actual fork test remains owner-deferred; frontend has no package yet | Fork revisit below; frontend coverage at its owning gate | P1_01_DOMAIN_REVIEW.md and P0_04_CI_REVIEW.md |
| D18 | Measured DNS/TCP/proxy resource budgets; engine budgets later | P10-04 | Operator |
| D19 | Owner selected HTTP CONNECT over TLS on 2026-10-16; verify proxy certificate/hostname, protect credentials, send checked numeric destination; no managed engine dependency; public explicit inbound remains a separate decision | P0-05/P4-05 | Isolated dependency qualification approved; actual runtime proxy/egress proof stays P4-05/P4-08 |
| D20 | Accepted custom > service > filter > default ranks, explicit domain match modes and one active Profile per Endpoint; stacks/schedules deferred | P0-01/P2-01/P2-02 | CR-0002; golden/permutation/conflict tests |
| D21 | Personal DoH/DoT token/hostname provisioning, TLS certificate lifecycle and optional IP restriction/learning bootstrap; possession is not hardware proof | P0-01/P3-07/P6-03 | CR-0002; redaction/rotation/revocation/roaming tests |
| D22 | Bounded local catalog formats, source/license/hash evidence and isolated private DNS records; remote feeds/DDNS/router helper deferred | P0-05/P2-07/P3-04 | CR-0002; real import/update rollback and scope-isolation tests |
| D23 | Owner selected UUIDv4 public resource IDs and qualification of uuid 1.28.0 on 2026-10-16. Domain newtypes use std parse/format only and expose no uuid dependency types. Public credential-record IDs and resolver secret material remain separate; domain association is not token verification or gateway authority. ID/token generators, wire/entropy/size/clock policy and secret lifecycle remain at their owning gates. | P1-01 / P1-07/P3-07/resource creation APIs | P1_01_DOMAIN_REVIEW.md; Windows qualification/audit, implementation Linux/Windows CI and independent Code Review passed |
| D24 | Owner accepted one UTF-8 JSON/YAML/TOML document, 1 MiB, collection depth 32, aggregate 10,000 resources/rules, unknown/duplicate rejection and strict YAML without expansion/include/substitution. Errors expose only stable code, known path and available numeric location. Large dataset payload budgets stay P2-07. | P1-02 | Direct owner answer on 2026-10-16; P1_02_CONFIG_REVIEW.md and api/schemas/README.md |
| D25 | Owner approved qualification of Serde 1.0.229, serde_json 1.0.154, toml 1.1.8+spec-1.1.0 and serde-saphyr 2.0.0, then serde_path_to_error 0.1.20 qualification/adoption if successful. Windows isolated probes/audit passed before config-only adoption; production audit passed. No raw diagnostics or new license exceptions. | P1-02 | Official archive/notices, CONFIG_INVENTORY.csv; implementation Linux/Windows CI and independent re-review passed after four concrete fixes; final evidence review, all seven final-head/main CI jobs and normal PR #5 merge completed |
| D26 | Owner approved SQLx 0.9.0/Tokio 1.53.3, default features off, runtime-tokio/sqlite-bundled/migrate, conditional on qualification. After native FTS5 CVEs in the original SQLite 3.51.3 were found, owner approved qualifying/adopting if successful the exact one-line sqlx-sqlite constraint change from upstream 94aafe3 to allow lock-pinned libsqlite3-sys 0.38.2/SQLite 3.53.2. | P1-03 | P1_03_STORAGE_REVIEW.md; Windows/Linux seven probes and qualification review passed on 5f02779 with all nine CI jobs; qualified profile adopted; implementation/fix reviews and all nine final-head/main jobs passed; PR #6 merged at65c076d; complete evidence in P1_03_STORAGE_REVIEW.md |
| D27 | Owner accepted one writer, queue 64, SQLite busy 1s, response 5s, explicit overload/timeout without hidden retry. Commit-time timeout does not prove cancellation; reread outcome. P1-03 stores public credential metadata and SecretRefId links only; verifier/token bytes wait for P1-07. | P1-03 / P1-07/P3-07 | Direct owner answers on 2026-10-17; Linux implementation/failure/permission tests passed on 20c7a3f; live secret/admission gates remain |
| D28 | After final review exposed slow-upgrade cancellation, owner accepted initialization in a separate owning task with the existing 5s caller response deadline. Timeout/cancellation does not drop backup/migration work; retain the lock until initialization and abandoned-result connection cleanup complete. Concurrent reopen returns Busy; no hidden retry. Runtime must remain alive for completion. | P1-03 / later service shutdown lifecycle | Direct owner answer in this chat on 2026-10-17; actual Linux regression, independent re-review, all nine final-head/main jobs and PR #6 merge completed; P1_03_STORAGE_REVIEW.md |
| D29 | Owner accepted one global complete-configuration draft chain with expected-parent optimistic edits and Conflict without automatic merge. Persist trusted Control Plane author, UTC time, correlation ID and revision/parent atomically; audit excludes document content. Do not fabricate missing legacy author/time. RBAC remains P6. | P1-04 | Direct owner answer in this chat on 2026-10-17; P1_04_REVISION_REVIEW.md; PR #7 merged644348c after independent implementation/final reviews and all nine head/main jobs; exact-merge Codecov processed; failed maintenance dispositions retained |

| D30 | Owner accepted external256-bit master key via protected file outside DB/backups; secret operations fail without correct key. New writes under the rotated active key; retain old keys for reads/re-encryption and matching historical backup restore. | P1-07 | Direct owner answer in this chat2026-10-17; P1_07_SECRET_REVIEW.md; implementation pending |
| D31 | Owner approved isolated aws-lc-rs1.18.2/zeroize1.9.1 qualification and adoption only on success: AES-256-GCM-SIV RFC8452, random nonce, authenticated record ID/purpose. | P1-07 | Direct owner answer in this chat2026-10-17; head203c76f passed Windows/Linux six probes/24 vectors, locked15-artifact audit, all11 CI jobs39056761899 and independent read-only/never review; production implementation still pending |
| D32 | Owner approved official crate-packaged NASM objects for Windows development without installing NASM; Linux builds native sources. Archive/native provenance recorded; binary-to-source reproducibility unproven. | P1-07 build profile | Direct owner answer in this chat2026-10-18; exact profile in tests/secret-dependencies; no FIPS claim |
| D33 | Owner accepted the complete proposed profile: bounded binary envelope with expected reference/purpose and64KiB plaintext limit; external binary keyring with one active and up to32 retained keys; encrypted records in existing SQLite with expected-envelope re-encryption; Linux0600/0700 protected file provider, Windows in-memory tests; explicit internal decryption and P6 user-export gate. | P1-07 | Direct owner answer2026-10-18: accept recommended profile in .setup/p1-07/implementation-design-proposal.md; qualification passed, implementation proceeds in draft PR8; KEY_LIFECYCLE.md records the contract |

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

## P1-03 review dispositions

| Item | Finding, impact and rationale | Owning task / revisit trigger |
|---|---|---|
| P1-03-CS-MIGRATION | CodeScene 7998950 repeats failed new-code/critical Bumpy Road Ahead gates in `migrations::apply` (9.15). History validation, WAL setup and pre-upgrade backup share explicit error exits. This increases maintenance effort, but no concrete defect remained after source/independent review; Linux fresh/upgrade/WAL backup/failed/checksum/dirty-history tests passed. Retain their ordering rather than split security-sensitive code for a score. The serious automatic-rollback defect was fixed and re-reviewed separately, not deferred. | Before adding the next migration or changing upgrade/recovery ordering; preserve current negative/backup tests and review the changed path independently. |
| P1-03-CS-STORAGE | `database::save` (9.39), `writer` and `ConfigurationWrite::parse` (9.69) have explicit index loops, operation dispatch and context arguments. Changes require checking all indexes/replies; full-revision/reopen/FK rollback, bounded queue/deadline and automatic-rollback regressions cover the current contract. No remaining functional defect was identified; a new dispatch or argument wrapper solely for metrics would add indirection. | P1-04/P1-07 when adding storage consumers or protected verifier persistence, and whenever a new index/operation is added. |
| P1-03-CS-PATH | `paths` (9.10) has repeated platform refusal wrappers and an explicit file-kind/mode/link predicate. The predicate is a fail-closed conjunction of required checks, verified by Linux directory/file/symlink/hardlink tests; other platforms remain unsupported. Maintenance duplication is minor; no score-only permission refactor is justified. | Before adding a platform or changing directory/backup permission handling; preserve real-platform negative tests. |
| P1-03-CS-TEST | Test-method size signals (8.54) are outside the owner's independent metric scope. The complete upgrade/failure/reopen scenarios remain readable together and pass in Linux CI. This is not an application defect or a reason to suppress the failed repository check. | Only when a concrete test defect or relevant behavior change requires editing the scenario. |

## P1-04 review dispositions

| Item | Finding, impact and rationale | Owning task / revisit trigger |
|---|---|---|
| P1-04-CS-CONTEXT | CodeScene 7999375 failed new-code health for `DraftRevision::new` / `from_unix_millis` (9.69): five typed metadata fields are explicit constructor arguments. Misordering same-type IDs could increase caller maintenance effort, but distinct public ID newtypes, self-parent/time validation, ID-match rejection and recorded metadata roundtrip tests cover the present contract; independent review found no defect. Keep the small mandatory context rather than add a builder solely for scoring. | Before adding provenance fields, API/RBAC integration at P6, or a concrete caller usability defect; preserve mandatory metadata and type checks. |
| P1-04-CS-TRANSACTION | The same run failed the critical Bumpy Road Ahead rule in `database::save` (9.39 to 8.54). Complete document/index loops now share expected-parent validation, audit insert and conditional tip update in one transaction. This raises maintenance effort when changing indexes, but splitting the ordering for metrics risks obscuring atomicity. Competing drafts, late tip/audit failure with zero leaked rows, durable upgrade/reopen and ID-match red/green regressions passed; independent review found no remaining defect. This is a minor maintenance deferral, not a passed CodeScene gate. | P1-07 when adding protected persistence, any new index/audit operation, or changed transaction/error ordering: reconsider cohesive helpers with the existing failure regressions and independent review. |

## P1-07 review dispositions

| Item | Finding, impact and rationale | Owning task / revisit trigger |
|---|---|---|
| P1-07-CS-AUX | CodeScene7999538 failed new-code health for duplication in tests/secret-dependencies/qualification.rs (9.39) and hotspot complexity in tools/test_planning_tools.py:ValidatorTests.test_foundation_acceptance_and_execution_limit_are_required (10.00 to9.69). Repeated explicit corruption checks and the additional P1-07 planning boundary increase test-maintenance work; source inspection, six Windows/Linux probes, planning negatives and independent review identified no concrete functional defect. Both files are outside the owner's independent metric scope. This minor maintenance disposition preserves the failed status without suppression or a score-only test rewrite. | P1-07 or a later owning task when changing these probes/boundary assertions for real behavior, or when a concrete test defect is found. |
| P1-07-CS-KEYFILE | CodeScene7999861 failed new-code health for `linux::read` complex method/conditional (8.28). The explicit path, exclusion-root, owner/mode, opened-handle and link checks increase maintenance effort. Actual Linux refusal tests and independent implementation review passed on b4732c6. A later dumpability hypothesis was disproved by a nonroot WSL probe and Linux v6.12 kernel source; its unnecessary parser was removed, not recorded as a functional fix. Keep the path-check ordering explicit rather than introduce permission wrappers solely for a score. No functional defect is currently identified; the remaining signal is minor maintenance, with no suppression. | Before changing file ownership/access/exclusion rules, adding a platform/ACL provider, or integrating hardened service packaging at P10-02/P11-03; retain actual-platform negative checks and independent review. |
| P1-07-CS-ENVELOPE | The same run reported the envelope size/magic/suite guard as a complex conditional (9.69). Keeping the length refusal before header indexing makes the input boundary auditable; splitting it just for metrics could obscure this ordering. Every envelope byte, truncation, context substitution and size bounds passed on Windows/Linux, with independent review and meaningful coverage. Minor maintenance is deferred without suppression. | Before envelope schema/purpose/length changes or a concrete parser defect; preserve all malformed-input tests. |
| P1-07-CS-RESTORE-TEST | CodeScene7999861 also scored large restore/schema-upgrade test methods (file9.09), while repeating the earlier excluded planning/qualification signals. Keeping each complete failure/reopen/restore scenario together helps trace durable outcomes. These tests are outside independent metric scope; actual Linux runs passed. No score-only test split is authorized. | When a concrete test defect or a relevant migration/restore contract change requires editing the owning scenario. |

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
