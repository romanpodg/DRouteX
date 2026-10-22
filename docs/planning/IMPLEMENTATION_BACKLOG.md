# Implementation backlog

**P0/P1/P2 complete and merged; PR106 exact-main verified; D45/D46 P4-01 candidate passed implementation review/all11 CI39671360399, final evidence/merge/main remain required.**

Accepted scope: CR-0001, DNS/proxy MVP before tunnels. See [ADR-0016](../architecture/adr/ADR-0016-dns-proxy-mvp-before-tunnels.md).

Accepted CR-0002: [Profile/Endpoint DNS model](../architecture/DNS_PRODUCT_MODEL.md), including personal DoH/DoT and versioned filter/service catalogs. Owner accepted P0-01 on 2026-10-15; detailed implementation gates remain.

Canonical status/delivery source: [backlog.json](backlog.json). Regenerate with python3.12 tools/render_planning.py.

Workstream IDs are stable, not chronological release order. Delivery tags govern release applicability.

S/M/L indicates relative complexity, not elapsed time. Quality bundles and full Definition of Done: [QUALITY_STRATEGY](../quality/QUALITY_STRATEGY.md).

## P0: Foundation (5 tasks)

### P0-01: Approve architectural and security contracts

Status: **completed**. Delivery: **MVP** (CR-0001). Complexity: **M**. Specification: §§5,15,19,23,25.

| Field | Deliverable / evidence |
|---|---|
| Goal and implementation scope | Approve DNS/proxy MVP architecture and security contracts under CR-0001; leave tunnels and managed engines for later |
| Prerequisites | Explicit roadmap approval and registered product decisions |
| Expected files/modules | docs/planning/P0_01_FOUNDATION_REVIEW.md; docs/research/CONTROLD_ANALYSIS.md; docs/architecture/DNS_PRODUCT_MODEL.md; docs/security/THREAT_MODEL.md; docs/architecture/adr |
| Acceptance criteria | Owner reviews proposed CR-0002 Profile/Endpoint model, secure DNS capabilities, separate legacy/gateway group grants and opt-in learning, policy layers/private records, snapshots and Linux assumptions; no tunnel prerequisite |
| Tests | Owner explicitly accepted P0-01/CR-0002 in this chat on 2026-10-15 after W1-W3/F1-F5 review; runtime proof remains in dependent tasks |
| Security considerations | No scope reduction or unsafe default hidden in an ADR |
| Quality checks | Q-RUST for Rust; Q-WEB for frontend; Q-LINUX for OS/network behavior; Q-DOC for docs. External Codecov/CodeScene only after activation; report unavailable evidence. |
| Definition of Done | Task acceptance and negative tests pass; applicable quality bundle passes; security/architecture review recorded; docs and traceability updated; no secrets or unintended changes; unverified required integration keeps task open. |

### P0-02: Create minimal workspace and engineering posture

Status: **completed**. Delivery: **MVP** (CR-0001). Complexity: **M**. Specification: §§15,16,22.

| Field | Deliverable / evidence |
|---|---|
| Goal and implementation scope | Create only common/config/policy crates needed next; record edition 2024, MSRV, lint and unsafe posture |
| Prerequisites | P0-01 |
| Expected files/modules | Cargo.toml; Cargo.lock; crates/droutex-{common,config,policy}; docs/agents/rust.md; deny.toml; LICENSE; .github/workflows/planning.yml |
| Acceptance criteria | Windows and Linux check succeed with pinned compatible dependencies; no placeholder engine crates |
| Tests | Windows/Linux fmt/check/Clippy/default/no-default tests passed in Actions run 38760651619 on 7ac5f14; local cargo-deny all checks and Rust failure-mutation tests passed; see EXECUTION_STATUS.md |
| Security considerations | Supply-chain inventory and no secrets in examples |
| Quality checks | Q-RUST for Rust; Q-WEB for frontend; Q-LINUX for OS/network behavior; Q-DOC for docs. External Codecov/CodeScene only after activation; report unavailable evidence. |
| Definition of Done | Task acceptance and negative tests pass; applicable quality bundle passes; security/architecture review recorded; docs and traceability updated; no secrets or unintended changes; unverified required integration keeps task open. |

### P0-03: Create isolated Linux network test harness

Status: **completed**. Delivery: **MVP** (CR-0001). Complexity: **M**. Specification: §§19,21,22.

| Field | Deliverable / evidence |
|---|---|
| Goal and implementation scope | Disposable dual-stack client/DNS/gateway/origin/proxy fixtures, personal DoH/DoT endpoint names and test certificates, capture and reliable cleanup; no VPN/TUN |
| Prerequisites | P0-01 |
| Expected files/modules | tests/network; docs/planning/P0_03_NETWORK_HARNESS.md; docs/quality/TESTING_STRATEGY.md; .github/workflows/planning.yml |
| Acceptance criteria | Real DNS and TCP/TLS/proxy fixture has separate clients/origins, dual-stack cases and reliable cleanup after failure |
| Tests | Four portable boundary tests and 41 real dual-stack DNS/TLS/proxy observations passed; WSL and native Debian 13.4/KVM/kernel 6.12.85+deb13-amd64 passed normal/setup-failure/SIGTERM/SIGKILL cleanup with unchanged host networking; see P0_03_NETWORK_HARNESS.md |
| Security considerations | Privileged fixture setup remains isolated from production services; no production credentials or untrusted PR root runner |
| Quality checks | Q-RUST for Rust; Q-WEB for frontend; Q-LINUX for OS/network behavior; Q-DOC for docs. External Codecov/CodeScene only after activation; report unavailable evidence. |
| Definition of Done | Task acceptance and negative tests pass; applicable quality bundle passes; security/architecture review recorded; docs and traceability updated; no secrets or unintended changes; unverified required integration keeps task open. |

### P0-04: Activate evidence-based CI

Status: **completed**. Delivery: **MVP** (CR-0001). Complexity: **M**. Specification: §§16,22.

| Field | Deliverable / evidence |
|---|---|
| Goal and implementation scope | Promote reviewed workflow proposals; add format/lint/tests/feature matrix/dependency gates |
| Prerequisites | P0-02 |
| Expected files/modules | .github/workflows; deny.toml; docs/quality |
| Acceptance criteria | Trusted and fork PRs have visible local checks; uploads conditional; first actual run recorded |
| Tests | Positive run and intentional lint/test/upload failure on disposable branch |
| Security considerations | Read-only permissions and no pull_request_target execution of PR code |
| Quality checks | Q-RUST for Rust; Q-WEB for frontend; Q-LINUX for OS/network behavior; Q-DOC for docs. External Codecov/CodeScene only after activation; report unavailable evidence. |
| Definition of Done | Task acceptance and negative tests pass; applicable quality bundle passes; security/architecture review recorded; docs and traceability updated; no secrets or unintended changes; unverified required integration keeps task open unless explicitly accepted as deferred by the owner. P0-04 accepted on 2026-10-16 with real private-fork and Codecov comparison/web-report evidence deferred in OPEN_DECISIONS.md; those checks are not recorded as passed. PR #1 still requires Code Review/current-head checks before authorized merge. |

### P0-05: Inventory DNS/proxy dependencies and licensing

Status: **completed**. Delivery: **MVP** (CR-0001). Complexity: **M**. Specification: §§7,19,25.

| Field | Deliverable / evidence |
|---|---|
| Goal and implementation scope | Inventory pinned DNS/TLS/proxy dependencies, certificate tooling and operator filter/service dataset source/license/hash evidence; defer managed engines to P5 |
| Prerequisites | P0-01 |
| Expected files/modules | docs/architecture/ENGINE_MATRIX.md; docs/operations/DEPENDENCIES.md |
| Acceptance criteria | MVP dependencies and reference proxy transport have version/license/test evidence; unsupported engine/tunnel modes explicitly deferred |
| Tests | Verify actual DNS/TLS/proxy dependency versions and reproduce a protected reference proxy handshake |
| Security considerations | Checksums/signatures and controlled update source |
| Quality checks | Q-RUST for Rust; Q-WEB for frontend; Q-LINUX for OS/network behavior; Q-DOC for docs. External Codecov/CodeScene only after activation; report unavailable evidence. |
| Definition of Done | Task acceptance and negative tests pass; applicable quality bundle passes; security/architecture review recorded; docs and traceability updated; no secrets or unintended changes; unverified required integration keeps task open. P0-05 qualification evidence: docs/operations/DEPENDENCIES.md; 107 pinned registry artifacts, Windows/Linux qualification and independent Code Review completed, fresh reference lab passed; production adoption and runtime gates remain separate. |

## P1: Domain and configuration (7 tasks)

### P1-01: Define stable domain identities and states

Status: **completed**. Delivery: **MVP** (CR-0001). Complexity: **M**. Specification: §§17,14.

| Field | Deliverable / evidence |
|---|---|
| Goal and implementation scope | Typed Profile/DnsEndpoint/ResolverCredential/filter-service revision/private record identities; separate legacy and gateway group grants from Device metadata; proxy/rule/snapshot states |
| Prerequisites | P0-02 |
| Expected files/modules | crates/droutex-common/src/domain |
| Acceptance criteria | Public EndpointId differs from secret capability; DNS endpoint scope never implies per-device transparent TCP identity |
| Tests | Identifier mixup, copied/revoked capability, same-NAT distinct DNS endpoints and ambiguous legacy mapping cases |
| Security considerations | Secrets distinct from serializable public fields |
| Quality checks | Q-RUST for Rust; Q-WEB for frontend; Q-LINUX for OS/network behavior; Q-DOC for docs. External Codecov/CodeScene only after activation; report unavailable evidence. |
| Definition of Done | Task acceptance and negative tests pass; applicable quality bundle passes; security/architecture review recorded; docs and traceability updated; no secrets or unintended changes; unverified required integration keeps task open. |

### P1-02: Validate external configuration boundaries

Status: **completed**. Delivery: **MVP** (CR-0001). Complexity: **M**. Specification: §§8,17,18.

| Field | Deliverable / evidence |
|---|---|
| Goal and implementation scope | Validate endpoint transport/access modes, single active Profile, typed DNS versus traffic actions, match layers, private records, dataset bounds and separate network grant scopes; reject future engine modes |
| Prerequisites | P1-01 |
| Expected files/modules | crates/droutex-config/src/schema; api/schemas |
| Acceptance criteria | Malformed/oversized input rejected with field errors; unknown schema/capability fails predictably |
| Tests | Negative JSON/YAML/TOML fixtures and port scope conflicts |
| Security considerations | Bound parser complexity; no secret echo in errors |
| Quality checks | Q-RUST for Rust; Q-WEB for frontend; Q-LINUX for OS/network behavior; Q-DOC for docs. External Codecov/CodeScene only after activation; report unavailable evidence. |
| Definition of Done | Task acceptance and negative tests pass; applicable quality bundle passes; security/architecture review recorded; docs and traceability updated; no secrets or unintended changes; unverified required integration keeps task open. |

### P1-03: Implement SQLite persistence and migrations

Status: **completed**. Delivery: **MVP** (CR-0001). Complexity: **M**. Specification: §§17,21,22.

| Field | Deliverable / evidence |
|---|---|
| Goal and implementation scope | SQLx/WAL immutable configuration and catalog revision repositories, transactional endpoint/scoped-grant indexes, public credential metadata and SecretRefId links only (verifier material P1-07); bounded writer, no database calls in packet hot path |
| Prerequisites | P1-02 |
| Expected files/modules | crates/droutex-storage; migrations |
| Acceptance criteria | Migration and transaction rollback preserve invariants; database access absent from packet hot path |
| Tests | Fresh/install/upgrade/busy/disk-full transaction tests |
| Security considerations | Parameterized SQL and restricted database permissions |
| Quality checks | Q-RUST for Rust; Q-WEB for frontend; Q-LINUX for OS/network behavior; Q-DOC for docs. External Codecov/CodeScene only after activation; report unavailable evidence. |
| Definition of Done | Task acceptance and negative tests pass; applicable quality bundle passes; security/architecture review recorded; docs and traceability updated; no secrets or unintended changes; unverified required integration keeps task open. |

### P1-04: Track draft revisions and audit provenance

Status: **completed**. Delivery: **MVP** (CR-0001). Complexity: **M**. Specification: §§8,17,18.

| Field | Deliverable / evidence |
|---|---|
| Goal and implementation scope | Immutable source revisions, authors and optimistic edit conflict handling |
| Prerequisites | P1-03 |
| Expected files/modules | crates/droutex-config/src/revisions; crates/droutex-storage |
| Acceptance criteria | Concurrent edits cannot silently overwrite; audit links actor and revision |
| Tests | PR #7 merged644348c after independent implementation/final reviews without confirmed defects; all nine final-head39055351917 and main39055754490 jobs passed, processed exact-merge Codecov. Backend patch143/155=92.25%, CodeScene failed maintenance dispositions recorded. See P1_04_REVISION_REVIEW.md |
| Security considerations | Authorization enforced by API later; no public unaudited writer |
| Quality checks | Q-RUST for Rust; Q-WEB for frontend; Q-LINUX for OS/network behavior; Q-DOC for docs. External Codecov/CodeScene only after activation; report unavailable evidence. |
| Definition of Done | Task acceptance and negative tests pass; applicable quality bundle passes; security/architecture review recorded; docs and traceability updated; no secrets or unintended changes; unverified required integration keeps task open. |

### P1-05: Encode authenticated immutable snapshots

Status: **completed**. Delivery: **MVP** (CR-0001). Complexity: **M**. Specification: §§5,8,19.

| Field | Deliverable / evidence |
|---|---|
| Goal and implementation scope | Canonical authenticated snapshots pin endpoint assignments, resolver capability verification/revocation state, dataset hashes and DNS/gateway grant authority with increasing sequence |
| Prerequisites | P1-04, P1-07 |
| Expected files/modules | crates/droutex-config/src/snapshot; api/schemas/snapshot |
| Acceptance criteria | Tampering, replay and incompatible schema rejected; deterministic bytes/hash for equivalent input |
| Tests | Windows/Linux implementation CI39147507106 all11 jobs and independent read-only/never review passed on cd0e2c5; independent OpenSSL golden, format/order permutations, metadata/payload/signature tampering, replay/rotation/rollback, resource/purpose/size and weak/noncanonical-key refusal cases; durable crash/activation evidence remains P1-06 |
| Security considerations | Use standard signatures/MACs; sign metadata and node scope, never raw embedded secrets |
| Quality checks | Q-RUST for Rust; Q-WEB for frontend; Q-LINUX for OS/network behavior; Q-DOC for docs. External Codecov/CodeScene only after activation; report unavailable evidence. |
| Definition of Done | Task acceptance and negative tests pass; applicable quality bundle passes; security/architecture review recorded; docs and traceability updated; no secrets or unintended changes; unverified required integration keeps task open. |

### P1-06: Implement local staged activation and recovery

Status: **completed**. Delivery: **MVP** (CR-0001). Complexity: **L**. Specification: §§8,11,20,24.

| Field | Deliverable / evidence |
|---|---|
| Goal and implementation scope | Durable DNS/gateway stage/validate/activate/verify journal and rollback; preserve one snapshot generation per decision without configuring OS routes or engines |
| Prerequisites | P1-05 |
| Expected files/modules | crates/droutex-core/src/deployment; crates/droutex-config/src/snapshot; docs/planning/P1_06_ACTIVATION_CONTRACT.md; docs/planning/P1_06_ACTIVATION_REVIEW.md |
| Acceptance criteria | Crash at each DNS/proxy transition recovers complete old/new generation; no partial accepted DNS/gateway policy; CP loss preserves last-known-good |
| Tests | Exact-head3c12bba CI39153071723 all11 jobs passed; Windows77/Linux104 tests per default/no-default configuration;20 Linux core tests,13 publication+3 recovery process exits, ENOSPC/cancellation/uncertain/revoked/corrupt/replay/fresh rollback/reader-lifetime tests; independent read-only/never Codex re-review passed after concrete fixes; PR11 final-head review/checks and merge still required |
| Security considerations | Rollback gets new authorized sequence referring to old content; reject arbitrary old replay |
| Quality checks | Q-RUST for Rust; Q-WEB for frontend; Q-LINUX for OS/network behavior; Q-DOC for docs. External Codecov/CodeScene only after activation; report unavailable evidence. |
| Definition of Done | Task acceptance and negative tests pass; applicable quality bundle passes; security/architecture review recorded; docs and traceability updated; no secrets or unintended changes; unverified required integration keeps task open. |

### P1-07: Implement secret references and key lifecycle

Status: **completed**. Delivery: **MVP** (CR-0001). Complexity: **M**. Specification: §§17,19,21.

| Field | Deliverable / evidence |
|---|---|
| Goal and implementation scope | Encrypted credential storage, external master-key provision, redacted wrappers and export lifecycle |
| Prerequisites | P1-01 |
| Expected files/modules | crates/droutex-security; crates/droutex-storage/src/secrets.rs; migrations/0004_encrypted_secrets.sql; docs/security/KEY_LIFECYCLE.md |
| Acceptance criteria | Master key absent from DB/backups by default; rotations and missing key failures explicit |
| Tests | Redaction in Debug/errors/API fixtures; rotate and restore tests |
| Security considerations | No custom crypto, argv secrets, shared temp files or encryption key beside ciphertext |
| Quality checks | Q-RUST for Rust; Q-WEB for frontend; Q-LINUX for OS/network behavior; Q-DOC for docs. External Codecov/CodeScene only after activation; report unavailable evidence. |
| Definition of Done | Task acceptance and negative tests pass; applicable quality bundle passes; security/architecture review recorded; docs and traceability updated; no secrets or unintended changes; unverified required integration keeps task open. |

## P2: Policy Engine (7 tasks)

### P2-01: Normalize domains and wildcard semantics

Status: **completed**. Delivery: **MVP** (CR-0001). Complexity: **M**. Specification: §§3,8,22.

| Field | Deliverable / evidence |
|---|---|
| Goal and implementation scope | IDNA/lowercase/trailing-dot normalization and explicit Exact/DomainAndSubdomains/SubdomainsOnly modes; no implicit adoption of Control D wildcard grammar |
| Prerequisites | P1-02 |
| Expected files/modules | crates/droutex-policy/src/domain.rs; crates/droutex-config/src/schema/model.rs; crates/droutex-config/src/schema/validation.rs |
| Acceptance criteria | Canonical equivalent names match consistently; invalid labels and wildcard bombs rejected |
| Tests | Unicode/IDNA cases, apex/subdomain matrix and property tests |
| Security considerations | Bound length/label/rule expansion at input boundary |
| Quality checks | Q-RUST for Rust; Q-WEB for frontend; Q-LINUX for OS/network behavior; Q-DOC for docs. External Codecov/CodeScene only after activation; report unavailable evidence. |
| Definition of Done | Task acceptance and negative tests pass; applicable quality bundle passes; security/architecture review recorded; docs and traceability updated; no secrets or unintended changes; unverified required integration keeps task open. |

### P2-02: Evaluate scoped rules deterministically

Status: **completed**. Delivery: **MVP** (CR-0001). Complexity: **M**. Specification: §§8,14,17.

| Field | Deliverable / evidence |
|---|---|
| Goal and implementation scope | Compile custom > service > filter > default ranks; priority within rank, documented specificity, stable ID and trusted endpoint/network scope |
| Prerequisites | P2-01, P1-01 |
| Expected files/modules | crates/droutex-policy/src/evaluate; crates/droutex-config/src/schema/model.rs |
| Acceptance criteria | Explicit BYPASS overrides lower policy layers but not security checks; default BYPASS does not skip filters; reordered input preserves decisions |
| Tests | Permutation, exception/default contrast, conflicting rules and two secure endpoints behind one NAT |
| Security considerations | Untrusted metadata cannot upgrade identity confidence |
| Quality checks | Q-RUST for Rust; Q-WEB for frontend; Q-LINUX for OS/network behavior; Q-DOC for docs. External Codecov/CodeScene only after activation; report unavailable evidence. |
| Definition of Done | Task acceptance and negative tests pass; applicable quality bundle passes; security/architecture review recorded; docs and traceability updated; no secrets or unintended changes; unverified required integration keeps task open. |

### P2-03: Detect policy conflicts and dependencies

Status: **completed**. Delivery: **MVP** (CR-0001). Complexity: **M**. Specification: §§8,17.

| Field | Deliverable / evidence |
|---|---|
| Goal and implementation scope | Reject unresolved layer/match contradictions, missing or unlicensed dataset revisions, invalid private CNAME chains and ambiguous legacy/gateway group assignments |
| Prerequisites | P2-02 |
| Expected files/modules | crates/droutex-policy/src/validate; crates/droutex-config/src/publication.rs |
| Acceptance criteria | Irreconcilable conflict prevents publication with actionable rule IDs |
| Tests | D39 symbolic conflict quotient/differential and resource/cycle/current-reference tests passed. PR14 a50c6a4 merged normally after linked-account implementation/final reviews and all11 head/main jobs; exact-main39476373259 passed, Codecov both reports Rust4071/4274, patch444/462. CodeScene maintenance failures remain recorded; docs/planning/P2_03_PUBLICATION_REVIEW.md. |
| Security considerations | No arbitrary map iteration or warning-only unsafe acceptance |
| Quality checks | Q-RUST for Rust; Q-WEB for frontend; Q-LINUX for OS/network behavior; Q-DOC for docs. External Codecov/CodeScene only after activation; report unavailable evidence. |
| Definition of Done | Task acceptance and negative tests pass; applicable quality bundle passes; security/architecture review recorded; docs and traceability updated; no secrets or unintended changes; unverified required integration keeps task open. |

### P2-04: Resolve actions and failure policies

Status: **completed**. Delivery: **MVP** (CR-0001). Complexity: **M**. Specification: §§6,7,8,20.

| Field | Deliverable / evidence |
|---|---|
| Goal and implementation scope | Keep DNS BYPASS/BLOCK/local overrides and compatible SmartDNS REDIRECT distinct from VPS_DIRECT/protected proxy outbounds; safety checks and strict failures remain mandatory |
| Prerequisites | P2-03 |
| Expected files/modules | crates/droutex-policy/src/decision; crates/droutex-config/src/actions.rs |
| Acceptance criteria | DIRECT decision point explicit; unsupported tunnel/node/UDP capabilities reject; strict proxy failures block |
| Tests | D40 strict action/scope/capability/negative matrix and borrowed lifetime passed. PR15 b4c104c merged normally after linked-account implementation/final reviews and all11 exact-head/main jobs39481364319; processed Codecov Rust4329/4535, patch257/261. Failed CodeScene maintenance findings retained; docs/planning/P2_04_ACTION_REVIEW.md. |
| Security considerations | D40 strict refusal; direct only explicitly selected VPS_DIRECT. Future automatic fail-open requires separately accepted Profile permission and real audit; none implemented. |
| Quality checks | Q-RUST for Rust; Q-WEB for frontend; Q-LINUX for OS/network behavior; Q-DOC for docs. External Codecov/CodeScene only after activation; report unavailable evidence. |
| Definition of Done | Task acceptance and negative tests pass; applicable quality bundle passes; security/architecture review recorded; docs and traceability updated; no secrets or unintended changes; unverified required integration keeps task open. |

### P2-05: Compile policies into snapshots

Status: **completed**. Delivery: **MVP** (CR-0001). Complexity: **M**. Specification: §§5,8,11.

| Field | Deliverable / evidence |
|---|---|
| Goal and implementation scope | Compile layered endpoint Profiles and pinned catalog hashes into bounded deterministic DNS/gateway generation tables |
| Prerequisites | P2-04, P1-05, P2-07 |
| Expected files/modules | crates/droutex-config/src/compile; crates/droutex-config/src/catalog; crates/droutex-core/src/deployment |
| Acceptance criteria | Same revision builds same snapshot; invalid capability never reaches active plan |
| Tests | Differential interpreter/compiler and hash tests |
| Security considerations | Compile validates ingress-target binding and authorized resolver scope |
| Quality checks | Q-RUST for Rust; Q-WEB for frontend; Q-LINUX for OS/network behavior; Q-DOC for docs. External Codecov/CodeScene only after activation; report unavailable evidence. |
| Definition of Done | Task acceptance and negative tests pass; applicable quality bundle passes; security/architecture review recorded; docs and traceability updated; no secrets or unintended changes; unverified required integration keeps task open. |

### P2-06: Provide explain and simulation contracts

Status: **completed**. Delivery: **MVP** (CR-0001). Complexity: **S**. Specification: §§8,12,18,20.

| Field | Deliverable / evidence |
|---|---|
| Goal and implementation scope | Explain endpoint/profile, layer/rule/dataset revision, distinct DNS/gateway identity scope, decision point and generation without invented connectivity or attribution |
| Prerequisites | P2-05 |
| Expected files/modules | crates/droutex-policy/src/explain.rs; crates/droutex-config/src/explain; crates/droutex-core/src/deployment/explain.rs; api/schemas/explain |
| Acceptance criteria | Simulation reports network_test_performed=false and distinguishes predicted/effective routes |
| Tests | Golden explain outputs for precedence, ECH/QUIC/NAT/IPv6 |
| Security considerations | Redact secrets and restrict diagnostic profile visibility |
| Quality checks | Q-RUST for Rust; Q-WEB for frontend; Q-LINUX for OS/network behavior; Q-DOC for docs. External Codecov/CodeScene only after activation; report unavailable evidence. |
| Definition of Done | Task acceptance and negative tests pass; applicable quality bundle passes; security/architecture review recorded; docs and traceability updated; no secrets or unintended changes; unverified required integration keeps task open. |

### P2-07: Import versioned filter and service catalogs

Status: **completed**. Delivery: **MVP** (CR-0001). Complexity: **M**. Specification: §§8,9,12,19,22.

| Field | Deliverable / evidence |
|---|---|
| Goal and implementation scope | Bounded local domain/hosts list and service-bundle imports with source/license/hash metadata and last-known-good preservation |
| Prerequisites | P1-02, P0-05 |
| Expected files/modules | crates/droutex-policy/src/catalog.rs; crates/droutex-config/src/catalog; tests/fixtures/catalogs |
| Acceptance criteria | D41 explicit bounded local formats/modes; actual verified immutable source/resource hash and license/provenance/dependency data can be pinned; malformed/oversized/unsupported/unapproved or empty inputs leave accepted data intact. No automatic publish or network fetch. |
| Tests | Real local format fixtures, Unicode/hosts entries, malformed/oversized lists, revision/hash pinning, failed update rollback and cross-profile isolation |
| Security considerations | No arbitrary URL fetch or proprietary Control D feed import; retain provenance/license and bounded parser limits |
| Quality checks | Q-RUST for Rust; Q-WEB for frontend; Q-LINUX for OS/network behavior; Q-DOC for docs. External Codecov/CodeScene only after activation; report unavailable evidence. |
| Definition of Done | Task acceptance and negative tests pass; applicable quality bundle passes; security/architecture review recorded; docs and traceability updated; no secrets or unintended changes; unverified required integration keeps task open. |

## P3: DNS Engine (7 tasks)

### P3-01: Add bounded DNS UDP/TCP listeners

Status: **planned**. Delivery: **MVP** (CR-0001). Complexity: **M**. Specification: §§9,19,22.

| Field | Deliverable / evidence |
|---|---|
| Goal and implementation scope | Hickory policy handler with restricted bindings, ACL, size/rate/time limits |
| Prerequisites | P0-03, P2-05, P4-01 |
| Expected files/modules | crates/droutex-dns/src/listeners |
| Acceptance criteria | Unauthorized recursion denied; TCP framing and truncation behavior correct |
| Tests | Real UDP/TCP exchange, malformed/truncated frames and flood limits |
| Security considerations | No open resolver; limited concurrency and amplification surface |
| Quality checks | Q-RUST for Rust; Q-WEB for frontend; Q-LINUX for OS/network behavior; Q-DOC for docs. External Codecov/CodeScene only after activation; report unavailable evidence. |
| Definition of Done | Task acceptance and negative tests pass; applicable quality bundle passes; security/architecture review recorded; docs and traceability updated; no secrets or unintended changes; unverified required integration keeps task open. |

### P3-02: Forward via approved upstream groups

Status: **planned**. Delivery: **MVP** (CR-0001). Complexity: **M**. Specification: §§6,9,20.

| Field | Deliverable / evidence |
|---|---|
| Goal and implementation scope | Trusted forwarding, deadlines and policy-selected upstream fallback; no full recursion |
| Prerequisites | P3-01 |
| Expected files/modules | crates/droutex-dns/src/upstream |
| Acceptance criteria | A/AAAA/CNAME/TXT/auxiliary queries resolve; timeout gives permitted fallback or SERVFAIL |
| Tests | Controlled upstream success, timeout, outage and recursion-loop tests |
| Security considerations | DNS resolver changes never imply traffic exit changes |
| Quality checks | Q-RUST for Rust; Q-WEB for frontend; Q-LINUX for OS/network behavior; Q-DOC for docs. External Codecov/CodeScene only after activation; report unavailable evidence. |
| Definition of Done | Task acceptance and negative tests pass; applicable quality bundle passes; security/architecture review recorded; docs and traceability updated; no secrets or unintended changes; unverified required integration keeps task open. |

### P3-03: Implement scoped TTL and negative caching

Status: **planned**. Delivery: **MVP** (CR-0001). Complexity: **M**. Specification: §§9,22.

| Field | Deliverable / evidence |
|---|---|
| Goal and implementation scope | Cache name/type/class/upstream/scope/mode/validation/generation semantics and bounded eviction |
| Prerequisites | P3-02 |
| Expected files/modules | crates/droutex-dns/src/cache |
| Acceptance criteria | Profile rewrites never leak; TTL and negative answers expire correctly; memory bound enforced |
| Tests | Virtual clock, concurrent scopes and policy generation changes |
| Security considerations | Partition validated origin data and local overrides |
| Quality checks | Q-RUST for Rust; Q-WEB for frontend; Q-LINUX for OS/network behavior; Q-DOC for docs. External Codecov/CodeScene only after activation; report unavailable evidence. |
| Definition of Done | Task acceptance and negative tests pass; applicable quality bundle passes; security/architecture review recorded; docs and traceability updated; no secrets or unintended changes; unverified required integration keeps task open. |

### P3-04: Implement DIRECT/BLOCK/local override answers

Status: **planned**. Delivery: **MVP** (CR-0001). Complexity: **M**. Specification: §§4,8,9.

| Field | Deliverable / evidence |
|---|---|
| Goal and implementation scope | DIRECT/BLOCK and profile-scoped private A/AAAA/CNAME answers with dual-stack/conflict/loop checks; local private answers never authorize gateway SSRF |
| Prerequisites | P3-03 |
| Expected files/modules | crates/droutex-dns/src/policy |
| Acceptance criteria | DIRECT returns origin IP; BLOCK predictable; local override does not create proxy route |
| Tests | Private record isolation across Profiles, A/AAAA/CNAME loops, DNSSEC override and no implicit proxy route |
| Security considerations | Signed data modification cannot preserve apparent authenticity |
| Quality checks | Q-RUST for Rust; Q-WEB for frontend; Q-LINUX for OS/network behavior; Q-DOC for docs. External Codecov/CodeScene only after activation; report unavailable evidence. |
| Definition of Done | Task acceptance and negative tests pass; applicable quality bundle passes; security/architecture review recorded; docs and traceability updated; no secrets or unintended changes; unverified required integration keeps task open. |

### P3-05: Handle DNSSEC and HTTPS/SVCB coherently

Status: **planned**. Delivery: **MVP** (CR-0001). Complexity: **M**. Specification: §§3,9,22.

| Field | Deliverable / evidence |
|---|---|
| Goal and implementation scope | Preserve normal verification versus overrides; handle hints/ECH/ALPN/alternative endpoint compatibility |
| Prerequisites | P3-04 |
| Expected files/modules | crates/droutex-dns/src/compatibility |
| Acceptance criteria | Overrides clear invalid validity claims; incompatible redirects rejected or explicitly unsupported |
| Tests | Signed RRsets, validating clients, HTTPS/SVCB hints and IPv6 bypass matrix |
| Security considerations | No blanket ECH stripping, downgrade claims or silent QUIC control |
| Quality checks | Q-RUST for Rust; Q-WEB for frontend; Q-LINUX for OS/network behavior; Q-DOC for docs. External Codecov/CodeScene only after activation; report unavailable evidence. |
| Definition of Done | Task acceptance and negative tests pass; applicable quality bundle passes; security/architecture review recorded; docs and traceability updated; no secrets or unintended changes; unverified required integration keeps task open. |

### P3-06: Publish safe SmartDNS route bindings

Status: **planned**. Delivery: **MVP** (CR-0001). Complexity: **M**. Specification: §§6,9,10.

| Field | Deliverable / evidence |
|---|---|
| Goal and implementation scope | Return gateway addresses only for approved compatible route contexts; reconcile TTL/current generation |
| Prerequisites | P3-05, P4-01 |
| Expected files/modules | crates/droutex-dns/src/routes |
| Acceptance criteria | Unknown services receive declared failure/ordinary-resolution policy; DNS and gateway agree on binding generation |
| Tests | DNS then TLS test, old TTL after publish, expired/removed mapping |
| Security considerations | Never authorize a TCP request solely because prior DNS/IP matches |
| Quality checks | Q-RUST for Rust; Q-WEB for frontend; Q-LINUX for OS/network behavior; Q-DOC for docs. External Codecov/CodeScene only after activation; report unavailable evidence. |
| Definition of Done | Task acceptance and negative tests pass; applicable quality bundle passes; security/architecture review recorded; docs and traceability updated; no secrets or unintended changes; unverified required integration keeps task open. |

### P3-07: Implement personal DoH/DoT endpoints and DNS diagnostics

Status: **planned**. Delivery: **MVP** (CR-0001). Complexity: **L**. Specification: §§9,12,20,22.

| Field | Deliverable / evidence |
|---|---|
| Goal and implementation scope | Mandatory personal DoH and DoT with valid TLS, bounded capability lookup/revocation and privacy; disabled-by-default scoped peer observations for grant publication; real DNS diagnostics |
| Prerequisites | P3-06, P6-01 |
| Expected files/modules | crates/droutex-dns/src/diagnostics; docs/operations/DNS.md |
| Acceptance criteria | Both secure transports select correct Profile across IP changes; unknown/revoked capabilities denied; observations cannot directly mutate authority or bootstrap restricted-IP mode |
| Tests | Real DoH/DoT TLS/hostname/port and roaming tests, same-NAT distinct Profiles, capability leakage/revocation, untrusted headers, bounded observation queue and CP outage |
| Security considerations | Redact resolver path/SNI secrets; do not equate TLS endpoint authentication or client labels with device identity; no implicit learned grants |
| Quality checks | Q-RUST for Rust; Q-WEB for frontend; Q-LINUX for OS/network behavior; Q-DOC for docs. External Codecov/CodeScene only after activation; report unavailable evidence. |
| Definition of Done | Task acceptance and negative tests pass; applicable quality bundle passes; security/architecture review recorded; docs and traceability updated; no secrets or unintended changes; unverified required integration keeps task open. |

## P4: Proxy Gateway (8 tasks)

### P4-01: Specify and enforce authorized destination binding

Status: **in_review**. Delivery: **MVP** (CR-0001). Complexity: **M**. Specification: §§3,6,10,19.

| Field | Deliverable / evidence |
|---|---|
| Goal and implementation scope | Enforce independently scoped legacy DNS and gateway exact-address group grants, service/domain/port/outbound binding, manual bootstrap and opt-in secure-observation publication; D46 manual-first current authorization, secure observation producer remains P3-07 |
| Prerequisites | P2-05, P0-01 |
| Expected files/modules | crates/droutex-core/src/deployment/admission; crates/droutex-config/src/schema; docs/security/GATEWAY_BINDING.md; docs/planning/P4_01_BINDING_CONTRACT.md |
| Acceptance criteria | No last-DNS-query Profile switch or union of conflicting grants; unknown/mismatched source fails closed; expiry and applied revocation persist during CP loss; D46 v1 association-only/v2 bounded leases, conservative durable clock floor and fixed admission quotas; automatic learning unavailable until P3-07 |
| Tests | Same-NAT conflicting endpoints, forged SNI, learning from invalid credentials/legacy/untrusted headers, address replacement, source mismatch and lease expiry |
| Security considerations | SNI identifies a candidate; separate ingress authorization grants access |
| Quality checks | Q-RUST for Rust; Q-WEB for frontend; Q-LINUX for OS/network behavior; Q-DOC for docs. External Codecov/CodeScene only after activation; report unavailable evidence. |
| Definition of Done | Task acceptance and negative tests pass; applicable quality bundle passes; security/architecture review recorded; docs and traceability updated; no secrets or unintended changes; unverified required integration keeps task open. |

### P4-02: Resolve and pin safe origin addresses

Status: **planned**. Delivery: **MVP** (CR-0001). Complexity: **M**. Specification: §§9,10,19.

| Field | Deliverable / evidence |
|---|---|
| Goal and implementation scope | Use policy-independent trusted resolver; canonical IP classification and checked connect address |
| Prerequisites | P4-01 |
| Expected files/modules | crates/droutex-gateway/src/origin |
| Acceptance criteria | Every new resolution denied for private/loopback/link-local/multicast/metadata/self; no TOCTOU re-resolve |
| Tests | Rebinding, CNAME chains, IPv4-mapped IPv6 and self-loop cases |
| Security considerations | Administrative internal override requires narrow explicit authorization |
| Quality checks | Q-RUST for Rust; Q-WEB for frontend; Q-LINUX for OS/network behavior; Q-DOC for docs. External Codecov/CodeScene only after activation; report unavailable evidence. |
| Definition of Done | Task acceptance and negative tests pass; applicable quality bundle passes; security/architecture review recorded; docs and traceability updated; no secrets or unintended changes; unverified required integration keeps task open. |

### P4-03: Parse bounded ClientHello metadata

Status: **planned**. Delivery: **MVP** (CR-0001). Complexity: **M**. Specification: §§3,10,22.

| Field | Deliverable / evidence |
|---|---|
| Goal and implementation scope | Use audited TLS parsing for visible SNI across fragmented records without TLS termination |
| Prerequisites | P4-01 |
| Expected files/modules | crates/droutex-gateway/src/tls_metadata |
| Acceptance criteria | Fragmented valid handshake preserves all bytes; oversized/incomplete/ECH-ambiguous input rejected |
| Tests | Fragmented records, slowloris, malformed length and fuzz seeds |
| Security considerations | No custom cryptography, MITM certificate or unbounded buffer |
| Quality checks | Q-RUST for Rust; Q-WEB for frontend; Q-LINUX for OS/network behavior; Q-DOC for docs. External Codecov/CodeScene only after activation; report unavailable evidence. |
| Definition of Done | Task acceptance and negative tests pass; applicable quality bundle passes; security/architecture review recorded; docs and traceability updated; no secrets or unintended changes; unverified required integration keeps task open. |

### P4-04: Forward TCP with bounded lifecycle

Status: **planned**. Delivery: **MVP** (CR-0001). Complexity: **M**. Specification: §§10,16,22.

| Field | Deliverable / evidence |
|---|---|
| Goal and implementation scope | Bidirectional passthrough, half-close, deadlines/backpressure/session quotas and shutdown |
| Prerequisites | P4-02, P4-03 |
| Expected files/modules | crates/droutex-gateway/src/session |
| Acceptance criteria | Authorized origin certificate remains unchanged; backpressure and shutdown release resources |
| Tests | Real TLS, long stream, half-close, cancelled dial and exhausted quota |
| Security considerations | Per-source/session limits; no locks across unsuitable await |
| Quality checks | Q-RUST for Rust; Q-WEB for frontend; Q-LINUX for OS/network behavior; Q-DOC for docs. External Codecov/CodeScene only after activation; report unavailable evidence. |
| Definition of Done | Task acceptance and negative tests pass; applicable quality bundle passes; security/architecture review recorded; docs and traceability updated; no secrets or unintended changes; unverified required integration keeps task open. |

### P4-05: Dispatch direct and verified TCP proxy outbounds

Status: **planned**. Delivery: **MVP** (CR-0001). Complexity: **M**. Specification: §§7,10.

| Field | Deliverable / evidence |
|---|---|
| Goal and implementation scope | Support direct egress and one selected protected SOCKS5 or HTTP CONNECT outbound using a bounded stream connector; no managed engine or TUN dependency |
| Prerequisites | P4-04, P1-07, P0-05 |
| Expected files/modules | crates/droutex-gateway/src/outbound |
| Acceptance criteria | Real selected TCP proxy transport works with authenticated or equivalently isolated access; unavailable strict proxy never falls back direct |
| Tests | Actual controlled proxy/origin egress, transport failure, secret redaction and unsupported-mode cases |
| Security considerations | Use protected proxy transport; pin checked origin IP through outbound to prevent unsafe remote re-resolution; no plaintext credentials on untrusted WAN |
| Quality checks | Q-RUST for Rust; Q-WEB for frontend; Q-LINUX for OS/network behavior; Q-DOC for docs. External Codecov/CodeScene only after activation; report unavailable evidence. |
| Definition of Done | Task acceptance and negative tests pass; applicable quality bundle passes; security/architecture review recorded; docs and traceability updated; no secrets or unintended changes; unverified required integration keeps task open. |

### P4-06: Keep routing and failover semantics consistent

Status: **planned**. Delivery: **MVP** (CR-0001). Complexity: **M**. Specification: §§2,6,10,20.

| Field | Deliverable / evidence |
|---|---|
| Goal and implementation scope | Pin active DNS/proxy generation for new flows, document draining and reject unavailable strict routes; keep node-group failover deferred |
| Prerequisites | P4-05, P2-04 |
| Expected files/modules | crates/droutex-gateway/src/router |
| Acceptance criteria | Existing sessions follow documented lifecycle; new flows use active revision and chosen fail policy |
| Tests | Publish under load, failed outbound, IPv6 and explicit fallback tests |
| Security considerations | No silent home-IP DIRECT claim or session-migration promise |
| Quality checks | Q-RUST for Rust; Q-WEB for frontend; Q-LINUX for OS/network behavior; Q-DOC for docs. External Codecov/CodeScene only after activation; report unavailable evidence. |
| Definition of Done | Task acceptance and negative tests pass; applicable quality bundle passes; security/architecture review recorded; docs and traceability updated; no secrets or unintended changes; unverified required integration keeps task open. |

### P4-07: Add gateway metrics and real route diagnostics

Status: **planned**. Delivery: **MVP** (CR-0001). Complexity: **M**. Specification: §§10,12,20.

| Field | Deliverable / evidence |
|---|---|
| Goal and implementation scope | Aggregate flow/byte/limit counters and approved-target Route Test with observed egress evidence |
| Prerequisites | P4-06 |
| Expected files/modules | crates/droutex-gateway/src/diagnostics; crates/droutex-telemetry |
| Acceptance criteria | Route Test reports actual target/egress/version and failure source; unavailable metrics remain unavailable |
| Tests | Different decision points, blocked diagnostics and failure attribution |
| Security considerations | Diagnostic API itself cannot become SSRF proxy |
| Quality checks | Q-RUST for Rust; Q-WEB for frontend; Q-LINUX for OS/network behavior; Q-DOC for docs. External Codecov/CodeScene only after activation; report unavailable evidence. |
| Definition of Done | Task acceptance and negative tests pass; applicable quality bundle passes; security/architecture review recorded; docs and traceability updated; no secrets or unintended changes; unverified required integration keeps task open. |

### P4-08: Verify DNS/proxy end-to-end vertical slice

Status: **planned**. Delivery: **MVP** (CR-0001). Complexity: **L**. Specification: §§3,4,6,9,10,22,24.

| Field | Deliverable / evidence |
|---|---|
| Goal and implementation scope | Real legacy and personal DoH/DoT endpoint policy through authorized SmartDNS gateway and checked direct/protected proxy egress; one snapshot, no VPN |
| Prerequisites | P3-06, P4-07, P10-02, P3-07 |
| Expected files/modules | tests/network/dns_proxy; docs/quality/MVP_ACCEPTANCE.md |
| Acceptance criteria | On a host without WG/AWG/Xray/TUN, DIRECT returns origin addresses, BLOCK is enforced at declared boundaries, authorized TCP preserves origin certificate and expected egress is observed |
| Tests | Real UDP/TCP/DoH/DoT and TLS/proxy; distinct DNS Profiles behind one NAT, proxy source mismatch/conflict, private-record SSRF negatives, observed-grant revocation/expiry, CP loss and rollback |
| Security considerations | No unrestricted proxy or TLS MITM; unknown identity/target fails closed; no full-device DNS-only enforcement claim |
| Quality checks | Q-RUST for Rust; Q-WEB for frontend; Q-LINUX for OS/network behavior; Q-DOC for docs. External Codecov/CodeScene only after activation; report unavailable evidence. |
| Definition of Done | Task acceptance and negative tests pass; applicable quality bundle passes; security/architecture review recorded; docs and traceability updated; no secrets or unintended changes; unverified required integration keeps task open. |

## P5: Post-MVP protocol adapters and tunnels (10 tasks)

### P5-01: Define adapter and capability contracts

Status: **planned**. Delivery: **POST_MVP_PROTOCOLS** (CR-0001). Complexity: **M**. Specification: §§7,15,18.

| Field | Deliverable / evidence |
|---|---|
| Goal and implementation scope | After DNS/proxy MVP, select exact WG/AWG/Xray versions and define detect/validate/prepare/apply/health/stop capabilities before adding adapters |
| Prerequisites | P1-02, P0-05, P11-06 |
| Expected files/modules | crates/droutex-protocols |
| Acceptance criteria | Capabilities depend on exact engine version/transport; unknown unavailable; lifecycle errors typed |
| Tests | Contract suite with fake adapters; negative capability tests |
| Security considerations | Protocol abstractions cannot accept arbitrary executable commands |
| Quality checks | Q-RUST for Rust; Q-WEB for frontend; Q-LINUX for OS/network behavior; Q-DOC for docs. External Codecov/CodeScene only after activation; report unavailable evidence. |
| Definition of Done | Task acceptance and negative tests pass; applicable quality bundle passes; security/architecture review recorded; docs and traceability updated; no secrets or unintended changes; unverified required integration keeps task open. |

### P5-02: Supervise pinned external engines safely

Status: **planned**. Delivery: **POST_MVP_PROTOCOLS** (CR-0001). Complexity: **M**. Specification: §§7,19,20.

| Field | Deliverable / evidence |
|---|---|
| Goal and implementation scope | Preapproved binaries, restrictive config files, resource/restart limits and isolated state |
| Prerequisites | P5-01, P1-06, P1-07 |
| Expected files/modules | crates/droutex-agent-core/src/engines |
| Acceptance criteria | Engine config validation precedes replacement; crash isolation and last-good recovery observable |
| Tests | Actual child crash/restart, bad config, cancellation and secret leakage checks |
| Security considerations | No shell interpolation, argv keys, writable binary search paths or public admin ports |
| Quality checks | Q-RUST for Rust; Q-WEB for frontend; Q-LINUX for OS/network behavior; Q-DOC for docs. External Codecov/CodeScene only after activation; report unavailable evidence. |
| Definition of Done | Task acceptance and negative tests pass; applicable quality bundle passes; security/architecture review recorded; docs and traceability updated; no secrets or unintended changes; unverified required integration keeps task open. |

### P5-03: Implement WireGuard provisioning adapter

Status: **planned**. Delivery: **POST_MVP_PROTOCOLS** (CR-0001). Complexity: **M**. Specification: §§7,14,17.

| Field | Deliverable / evidence |
|---|---|
| Goal and implementation scope | Peer/interface/listener and native config export through restricted helper |
| Prerequisites | P5-02, P10-01 |
| Expected files/modules | crates/droutex-protocol-wireguard |
| Acceptance criteria | Pinned Linux engine accepts config; add/revoke peer state is reported applied |
| Tests | Real two-peer Linux handshake/export/revocation tests |
| Security considerations | Keys private; peer access and allowed IP overlaps checked |
| Quality checks | Q-RUST for Rust; Q-WEB for frontend; Q-LINUX for OS/network behavior; Q-DOC for docs. External Codecov/CodeScene only after activation; report unavailable evidence. |
| Definition of Done | Task acceptance and negative tests pass; applicable quality bundle passes; security/architecture review recorded; docs and traceability updated; no secrets or unintended changes; unverified required integration keeps task open. |

### P5-04: Verify WG tunnel routing vertical slice

Status: **planned**. Delivery: **POST_MVP_PROTOCOLS** (CR-0001). Complexity: **L**. Specification: §§2,4,6,22.

| Field | Deliverable / evidence |
|---|---|
| Goal and implementation scope | Integrate real client WG, DNS, policy, egress route and privileged path |
| Prerequisites | P5-03, P3-06, P4-07 |
| Expected files/modules | tests/network/wireguard; docs/operations/CLIENTS.md |
| Acceptance criteria | Observed egress and packet captures match route; revoked peers cannot create new connections |
| Tests | Actual WG engine, IPv4/IPv6 leaks, direct/proxy/block and engine failure |
| Security considerations | No mock acceptance for TUN, route manager or revocation |
| Quality checks | Q-RUST for Rust; Q-WEB for frontend; Q-LINUX for OS/network behavior; Q-DOC for docs. External Codecov/CodeScene only after activation; report unavailable evidence. |
| Definition of Done | Task acceptance and negative tests pass; applicable quality bundle passes; security/architecture review recorded; docs and traceability updated; no secrets or unintended changes; unverified required integration keeps task open. |

### P5-05: Implement and verify AmneziaWG adapter

Status: **planned**. Delivery: **POST_MVP_PROTOCOLS** (CR-0001). Complexity: **L**. Specification: §§7,14,24.

| Field | Deliverable / evidence |
|---|---|
| Goal and implementation scope | Pinned AWG version/parameters and compatible client export; separate endpoint |
| Prerequisites | P5-04, P0-05 |
| Expected files/modules | crates/droutex-protocol-amneziawg; tests/network/amneziawg |
| Acceptance criteria | Real pinned client connects/revokes; incompatible versions fail validation |
| Tests | Kernel/userspace compatibility and client export tests in Linux lab |
| Security considerations | No claim AWG equals WG or sing-box replaces AWG |
| Quality checks | Q-RUST for Rust; Q-WEB for frontend; Q-LINUX for OS/network behavior; Q-DOC for docs. External Codecov/CodeScene only after activation; report unavailable evidence. |
| Definition of Done | Task acceptance and negative tests pass; applicable quality bundle passes; security/architecture review recorded; docs and traceability updated; no secrets or unintended changes; unverified required integration keeps task open. |

### P5-06: Implement Xray VLESS REALITY adapter

Status: **planned**. Delivery: **POST_MVP_PROTOCOLS** (CR-0001). Complexity: **L**. Specification: §§7,14,24.

| Field | Deliverable / evidence |
|---|---|
| Goal and implementation scope | Pinned official Xray lifecycle, listener/outbound credentials and client export |
| Prerequisites | P5-04, P0-05 |
| Expected files/modules | crates/droutex-protocol-xray; tests/network/xray |
| Acceptance criteria | Actual VLESS/REALITY inbound/outbound path tested; crash leaves DNS/WG healthy |
| Tests | Real engine/client config, transport handshake, egress and crash tests |
| Security considerations | Private material redacted; port/TLS combinations validated |
| Quality checks | Q-RUST for Rust; Q-WEB for frontend; Q-LINUX for OS/network behavior; Q-DOC for docs. External Codecov/CodeScene only after activation; report unavailable evidence. |
| Definition of Done | Task acceptance and negative tests pass; applicable quality bundle passes; security/architecture review recorded; docs and traceability updated; no secrets or unintended changes; unverified required integration keeps task open. |

### P5-07: Handle credentials and export lifecycle across engines

Status: **planned**. Delivery: **POST_MVP_PROTOCOLS** (CR-0001). Complexity: **M**. Specification: §§7,14,17.

| Field | Deliverable / evidence |
|---|---|
| Goal and implementation scope | Add/rotate/revoke/export with engine-specific semantics, expiring delivery and QR safeguards |
| Prerequisites | P5-05, P5-06 |
| Expected files/modules | crates/droutex-protocols/src/provisioning; api/schemas/exports |
| Acceptance criteria | Applied revocation rejects new sessions; configuration export lists compatible clients and limitations |
| Tests | Revoke during active session, stale export and rotation tests |
| Security considerations | No publicly accessible permanent download or cross-profile export |
| Quality checks | Q-RUST for Rust; Q-WEB for frontend; Q-LINUX for OS/network behavior; Q-DOC for docs. External Codecov/CodeScene only after activation; report unavailable evidence. |
| Definition of Done | Task acceptance and negative tests pass; applicable quality bundle passes; security/architecture review recorded; docs and traceability updated; no secrets or unintended changes; unverified required integration keeps task open. |

### P5-08: Gate protocol compatibility and failure isolation

Status: **planned**. Delivery: **POST_MVP_PROTOCOLS** (CR-0001). Complexity: **M**. Specification: §§7,21,22,24.

| Field | Deliverable / evidence |
|---|---|
| Goal and implementation scope | Record tested TCP/UDP/inbound/outbound combinations and upgrade rollback matrix |
| Prerequisites | P5-07 |
| Expected files/modules | docs/architecture/ENGINE_MATRIX.md; tests/network/engine_matrix |
| Acceptance criteria | WG/AWG/Xray each has independent passing gate; unknown combinations disabled |
| Tests | Crash each engine separately, port collisions and upgrade rejection |
| Security considerations | Supply-chain verification and adapter resource boundaries |
| Quality checks | Q-RUST for Rust; Q-WEB for frontend; Q-LINUX for OS/network behavior; Q-DOC for docs. External Codecov/CodeScene only after activation; report unavailable evidence. |
| Definition of Done | Task acceptance and negative tests pass; applicable quality bundle passes; security/architecture review recorded; docs and traceability updated; no secrets or unintended changes; unverified required integration keeps task open. |

### P5-09: Add post-MVP tunnel and engine UI

Status: **planned**. Delivery: **POST_MVP_PROTOCOLS** (CR-0001). Complexity: **M**. Specification: §§7,12,14,24.

| Field | Deliverable / evidence |
|---|---|
| Goal and implementation scope | Extend released DNS/proxy UI with verified WG/AWG/Xray provisioning, ports, credentials, compatible exports and engine status |
| Prerequisites | P5-08, P7-02, P7-04 |
| Expected files/modules | apps/droutex-web/src/features/{devices,protocols} |
| Acceptance criteria | Only tested client/engine capabilities exposed; exports/QR and applied revocation are protected; existing DNS/proxy users need no tunnel |
| Tests | Browser/API provisioning, expired export, incompatible version, peer revocation and independent engine failure cases |
| Security considerations | No secret redisplay or cross-profile export; server-side permissions remain authoritative |
| Quality checks | Q-RUST for Rust; Q-WEB for frontend; Q-LINUX for OS/network behavior; Q-DOC for docs. External Codecov/CodeScene only after activation; report unavailable evidence. |
| Definition of Done | Task acceptance and negative tests pass; applicable quality bundle passes; security/architecture review recorded; docs and traceability updated; no secrets or unintended changes; unverified required integration keeps task open. |

### P5-10: Validate post-MVP tunnel and engine expansion

Status: **planned**. Delivery: **POST_MVP_PROTOCOLS** (CR-0001). Complexity: **L**. Specification: §§7,14,19,22,24.

| Field | Deliverable / evidence |
|---|---|
| Goal and implementation scope | Run original WG/AWG/Xray checklist and real Linux helper/routing/isolation/segmentation/upgrade/restore tests as a separate post-MVP gate |
| Prerequisites | P5-08, P5-09, P10-01, P11-06 |
| Expected files/modules | tests/e2e/protocol_expansion; docs/quality/PROTOCOL_ACCEPTANCE.md |
| Acceptance criteria | Original spec 24.1 WG/AWG/Xray and Xray-crash-isolation requirements have actual evidence; DNS/proxy MVP remains operational; no deferred checkbox is marked passed before real testing |
| Tests | Real pinned peers/engines, TUN/routes/helper IPC, revocation, segmentation, IPv6 leaks, crash isolation and engine config upgrade/restore |
| Security considerations | Privileged networking remains disabled until helper review and actual Linux acceptance; no mocks counted as engine compatibility |
| Quality checks | Q-RUST for Rust; Q-WEB for frontend; Q-LINUX for OS/network behavior; Q-DOC for docs. External Codecov/CodeScene only after activation; report unavailable evidence. |
| Definition of Done | Task acceptance and negative tests pass; applicable quality bundle passes; security/architecture review recorded; docs and traceability updated; no secrets or unintended changes; unverified required integration keeps task open. |

## P6: Control Plane API (8 tasks)

### P6-01: Bootstrap authentication and administrator roles

Status: **planned**. Delivery: **MVP** (CR-0001). Complexity: **M**. Specification: §§18,19,21,25.

| Field | Deliverable / evidence |
|---|---|
| Goal and implementation scope | One-time owner setup, login/MFA/sessions, owner/admin/operator permissions; add admins later |
| Prerequisites | P1-03, P1-07 |
| Expected files/modules | crates/droutex-api/src/auth; migrations/auth; api/openapi |
| Acceptance criteria | Bootstrap closes after owner creation; only authorized owner can grant role; last owner protected |
| Tests | Concurrent bootstrap, session fixation, CSRF/origin, MFA and privilege escalation negatives |
| Security considerations | Private TLS admin endpoint, strong password hashing and rate limits |
| Quality checks | Q-RUST for Rust; Q-WEB for frontend; Q-LINUX for OS/network behavior; Q-DOC for docs. External Codecov/CodeScene only after activation; report unavailable evidence. |
| Definition of Done | Task acceptance and negative tests pass; applicable quality bundle passes; security/architecture review recorded; docs and traceability updated; no secrets or unintended changes; unverified required integration keeps task open. |

### P6-02: Publish versioned API/error contracts

Status: **planned**. Delivery: **MVP** (CR-0001). Complexity: **M**. Specification: §§12,18.

| Field | Deliverable / evidence |
|---|---|
| Goal and implementation scope | REST /api/v1, OpenAPI, bounded requests, pagination, typed errors and auth mapping |
| Prerequisites | P6-01, P2-06 |
| Expected files/modules | crates/droutex-api/src/contracts; api/openapi |
| Acceptance criteria | Schema matches handlers; every mutation audited and protected; no secret default fields |
| Tests | Contract fixtures, RBAC table, malformed request and pagination tests |
| Security considerations | Authentication applies to diagnostics, metrics and exports too |
| Quality checks | Q-RUST for Rust; Q-WEB for frontend; Q-LINUX for OS/network behavior; Q-DOC for docs. External Codecov/CodeScene only after activation; report unavailable evidence. |
| Definition of Done | Task acceptance and negative tests pass; applicable quality bundle passes; security/architecture review recorded; docs and traceability updated; no secrets or unintended changes; unverified required integration keeps task open. |

### P6-03: Manage Profiles, DNS Endpoints and scoped access

Status: **planned**. Delivery: **MVP** (CR-0001). Complexity: **M**. Specification: §§14,17,18.

| Field | Deliverable / evidence |
|---|---|
| Goal and implementation scope | RBAC CRUD for reusable Profiles, DNS Endpoints, single active assignment, resolver provisioning/rotation/revocation, manual network bootstrap and scoped opt-in IP-learning settings |
| Prerequisites | P6-02, P1-04 |
| Expected files/modules | crates/droutex-api/src/devices; crates/droutex-api/src/profiles |
| Acceptance criteria | Device metadata is not credential identity; DNS capability and gateway group authority shown separately with observed/pending/applied/expired states |
| Tests | Copied/revoked secret, same-NAT DNS versus gateway policy, ambiguous legacy assignment, restricted-IP bootstrap and grant revocation |
| Security considerations | No MAC-over-internet or shared-IP individual authorization |
| Quality checks | Q-RUST for Rust; Q-WEB for frontend; Q-LINUX for OS/network behavior; Q-DOC for docs. External Codecov/CodeScene only after activation; report unavailable evidence. |
| Definition of Done | Task acceptance and negative tests pass; applicable quality bundle passes; security/architecture review recorded; docs and traceability updated; no secrets or unintended changes; unverified required integration keeps task open. |

### P6-04: Manage layered rules, catalogs and private records

Status: **planned**. Delivery: **MVP** (CR-0001). Complexity: **M**. Specification: §§8,12,18.

| Field | Deliverable / evidence |
|---|---|
| Goal and implementation scope | Draft CRUD/import/diff/simulate/validate for custom/service/filter/default layers and private records; pin approved list/group revisions and reject unsupported inputs |
| Prerequisites | P6-03, P2-06 |
| Expected files/modules | crates/droutex-api/src/rules; crates/droutex-api/src/service_groups |
| Acceptance criteria | Save Draft never implies activation; conflicts and capabilities explained |
| Tests | Layer precedence/default BYPASS, failed/oversized/unlicensed catalog import, private-record scope, overlapping services and draft races |
| Security considerations | Bound subscription/import input; no automatic broad external changes |
| Quality checks | Q-RUST for Rust; Q-WEB for frontend; Q-LINUX for OS/network behavior; Q-DOC for docs. External Codecov/CodeScene only after activation; report unavailable evidence. |
| Definition of Done | Task acceptance and negative tests pass; applicable quality bundle passes; security/architecture review recorded; docs and traceability updated; no secrets or unintended changes; unverified required integration keeps task open. |

### P6-05: Publish deployment state and rollback API

Status: **planned**. Delivery: **MVP** (CR-0001). Complexity: **M**. Specification: §§8,11,12,18.

| Field | Deliverable / evidence |
|---|---|
| Goal and implementation scope | Diff/validate/stage/publish/status/rollback with audit and per-component acknowledgments |
| Prerequisites | P6-04, P1-06 |
| Expected files/modules | crates/droutex-api/src/deployments |
| Acceptance criteria | UI/API show actually applied version and degraded/failed transitions; authorized rollback observable |
| Tests | Unauthorized publish, concurrent deployment and post-health rollback tests |
| Security considerations | No partial ruleset accepted; secrets excluded from diff |
| Quality checks | Q-RUST for Rust; Q-WEB for frontend; Q-LINUX for OS/network behavior; Q-DOC for docs. External Codecov/CodeScene only after activation; report unavailable evidence. |
| Definition of Done | Task acceptance and negative tests pass; applicable quality bundle passes; security/architecture review recorded; docs and traceability updated; no secrets or unintended changes; unverified required integration keeps task open. |

### P6-06: Expose bounded diagnostics and metrics

Status: **planned**. Delivery: **MVP** (CR-0001). Complexity: **M**. Specification: §§12,18,20.

| Field | Deliverable / evidence |
|---|---|
| Goal and implementation scope | Policy Explain, DNS Test, reachability and approved-target Route Test contracts |
| Prerequisites | P6-02, P3-07, P4-07 |
| Expected files/modules | crates/droutex-api/src/diagnostics; crates/droutex-telemetry |
| Acceptance criteria | Four tools distinguish policy prediction/process/transport/route/service evidence |
| Tests | Diagnostic timeout, wrong profile and unavailable metrics tests |
| Security considerations | Limit cardinality and SSRF target scope; redact raw browsing data |
| Quality checks | Q-RUST for Rust; Q-WEB for frontend; Q-LINUX for OS/network behavior; Q-DOC for docs. External Codecov/CodeScene only after activation; report unavailable evidence. |
| Definition of Done | Task acceptance and negative tests pass; applicable quality bundle passes; security/architecture review recorded; docs and traceability updated; no secrets or unintended changes; unverified required integration keeps task open. |

### P6-07: Implement encrypted backup and restore API

Status: **planned**. Delivery: **MVP** (CR-0001). Complexity: **M**. Specification: §§12,19,21,24.

| Field | Deliverable / evidence |
|---|---|
| Goal and implementation scope | Consistent SQLite backup, encrypted archive, separate key recovery and restore validation |
| Prerequisites | P6-05, P1-07 |
| Expected files/modules | crates/droutex-api/src/backups; docs/operations/RECOVERY.md |
| Acceptance criteria | Restore on fresh host produces validated configuration and working service |
| Tests | WAL-consistent backup, wrong key, corrupt archive and upgrade restore |
| Security considerations | Authorized offline recovery; no archive traversal or exported plaintext keys |
| Quality checks | Q-RUST for Rust; Q-WEB for frontend; Q-LINUX for OS/network behavior; Q-DOC for docs. External Codecov/CodeScene only after activation; report unavailable evidence. |
| Definition of Done | Task acceptance and negative tests pass; applicable quality bundle passes; security/architecture review recorded; docs and traceability updated; no secrets or unintended changes; unverified required integration keeps task open. |

### P6-08: Audit access administration and privacy settings

Status: **planned**. Delivery: **MVP** (CR-0001). Complexity: **M**. Specification: §§12,18,19,20.

| Field | Deliverable / evidence |
|---|---|
| Goal and implementation scope | Admin add/remove/role change/session revocation, retention controls and separated audit/activity stores |
| Prerequisites | P6-01, P6-05 |
| Expected files/modules | crates/droutex-api/src/security; crates/droutex-storage/src/audit |
| Acceptance criteria | Role changes take effect predictably; audit retained separately; per-domain logging can be disabled |
| Tests | Last-owner guard, concurrent demotion and retention purge tests |
| Security considerations | Never log credentials; lower-privilege user cannot read unrelated browsing history |
| Quality checks | Q-RUST for Rust; Q-WEB for frontend; Q-LINUX for OS/network behavior; Q-DOC for docs. External Codecov/CodeScene only after activation; report unavailable evidence. |
| Definition of Done | Task acceptance and negative tests pass; applicable quality bundle passes; security/architecture review recorded; docs and traceability updated; no secrets or unintended changes; unverified required integration keeps task open. |

## P7: Web interface (7 tasks)

### P7-01: Create accessible authenticated web shell

Status: **planned**. Delivery: **MVP** (CR-0001). Complexity: **M**. Specification: §§12,13,15.

| Field | Deliverable / evidence |
|---|---|
| Goal and implementation scope | React/TS/Tailwind/shadcn design primitives, light/dark/responsive navigation and login/MFA |
| Prerequisites | P6-02 |
| Expected files/modules | apps/droutex-web/src/{app,auth,ui} |
| Acceptance criteria | Keyboard/mobile/reduced-motion usable; private session lifecycle works |
| Tests | Vitest component tests and browser auth/accessibility checks |
| Security considerations | No secrets in localStorage or bundled environment variables |
| Quality checks | Q-RUST for Rust; Q-WEB for frontend; Q-LINUX for OS/network behavior; Q-DOC for docs. External Codecov/CodeScene only after activation; report unavailable evidence. |
| Definition of Done | Task acceptance and negative tests pass; applicable quality bundle passes; security/architecture review recorded; docs and traceability updated; no secrets or unintended changes; unverified required integration keeps task open. |

### P7-02: Onboard personal and network DNS Endpoints

Status: **planned**. Delivery: **MVP** (CR-0001). Complexity: **M**. Specification: §§12,14,21.

| Field | Deliverable / evidence |
|---|---|
| Goal and implementation scope | Capability-aware personal DoH/DoT setup or legacy network resolver setup, profile assignment, secret rotation and separate proxy source/grant status; no VPN client required |
| Prerequisites | P7-01, P6-03 |
| Expected files/modules | apps/droutex-web/src/features/devices |
| Acceptance criteria | Supported clients roam using personal DNS capability; legacy clients share network scope; no per-device transparent gateway claim |
| Tests | Actual secure and legacy DNS setup, two endpoints on same NAT, unsupported client, secret revocation and mismatched proxy source |
| Security considerations | Mask secrets and gate copy/export permissions |
| Quality checks | Q-RUST for Rust; Q-WEB for frontend; Q-LINUX for OS/network behavior; Q-DOC for docs. External Codecov/CodeScene only after activation; report unavailable evidence. |
| Definition of Done | Task acceptance and negative tests pass; applicable quality bundle passes; security/architecture review recorded; docs and traceability updated; no secrets or unintended changes; unverified required integration keeps task open. |

### P7-03: Build rule editor simulate and publish flow

Status: **planned**. Delivery: **MVP** (CR-0001). Complexity: **M**. Specification: §§8,12,13.

| Field | Deliverable / evidence |
|---|---|
| Goal and implementation scope | Profile editor tabs for Filters, Services, Custom Rules, Private Records and DNS/privacy settings; explicit match modes and layered Explain; diff/draft/validate/publish |
| Prerequisites | P7-01, P6-05 |
| Expected files/modules | apps/droutex-web/src/features/rules |
| Acceptance criteria | Strict failure default; direct fallback separately explicit; saved/applied status distinct |
| Tests | Browser negative edit, conflict, stale version and publish failure tests |
| Security considerations | Warnings about leaks cannot be hidden under Advanced |
| Quality checks | Q-RUST for Rust; Q-WEB for frontend; Q-LINUX for OS/network behavior; Q-DOC for docs. External Codecov/CodeScene only after activation; report unavailable evidence. |
| Definition of Done | Task acceptance and negative tests pass; applicable quality bundle passes; security/architecture review recorded; docs and traceability updated; no secrets or unintended changes; unverified required integration keeps task open. |

### P7-04: Build DNS and proxy administration

Status: **planned**. Delivery: **MVP** (CR-0001). Complexity: **M**. Specification: §§9,12.

| Field | Deliverable / evidence |
|---|---|
| Goal and implementation scope | DNS endpoints/listeners/upstreams/cache/TLS and Proxy Outbounds/grants/status screens; actual transport/port capabilities and missing dataset states; no engines |
| Prerequisites | P7-02, P3-07, P4-08 |
| Expected files/modules | apps/droutex-web/src/features/{profiles,endpoints,filters,services,dns,proxy} |
| Acceptance criteria | Only available metrics shown; port conflicts and compatible transport choices enforced |
| Tests | Component/API contract tests and disabled feature checks |
| Security considerations | No open resolver toggle without explicit restricted access configuration |
| Quality checks | Q-RUST for Rust; Q-WEB for frontend; Q-LINUX for OS/network behavior; Q-DOC for docs. External Codecov/CodeScene only after activation; report unavailable evidence. |
| Definition of Done | Task acceptance and negative tests pass; applicable quality bundle passes; security/architecture review recorded; docs and traceability updated; no secrets or unintended changes; unverified required integration keeps task open. |

### P7-05: Build dashboard activity and diagnostic tools

Status: **planned**. Delivery: **MVP** (CR-0001). Complexity: **M**. Specification: §§12,13,20.

| Field | Deliverable / evidence |
|---|---|
| Goal and implementation scope | Actual aggregate endpoint/profile/action/rule/dataset metrics and DNS/proxy source mismatch diagnostics; no guessed per-device TCP correlation |
| Prerequisites | P7-01, P6-06 |
| Expected files/modules | apps/droutex-web/src/features/{dashboard,diagnostics} |
| Acceptance criteria | Configured/applied/healthy/tested/connected distinct; no fabricated counts or DNS-TCP correlation |
| Tests | Unavailable telemetry, outage, privacy-disabled logs and real e2e tests |
| Security considerations | Bound history; minimum browsing data visible by permission |
| Quality checks | Q-RUST for Rust; Q-WEB for frontend; Q-LINUX for OS/network behavior; Q-DOC for docs. External Codecov/CodeScene only after activation; report unavailable evidence. |
| Definition of Done | Task acceptance and negative tests pass; applicable quality bundle passes; security/architecture review recorded; docs and traceability updated; no secrets or unintended changes; unverified required integration keeps task open. |

### P7-06: Build deployment backup and access screens

Status: **planned**. Delivery: **MVP** (CR-0001). Complexity: **M**. Specification: §§12,13,19,21.

| Field | Deliverable / evidence |
|---|---|
| Goal and implementation scope | Version diffs/status/rollback, backup restore, administrator role assignment and settings |
| Prerequisites | P7-03, P6-07, P6-08 |
| Expected files/modules | apps/droutex-web/src/features/{operations,security} |
| Acceptance criteria | Owner can add later admins; destructive actions confirmed; true apply/restore outcomes displayed |
| Tests | RBAC UI/API parity, last-owner and failed restore browser tests |
| Security considerations | Front-end hiding never substitutes server authorization |
| Quality checks | Q-RUST for Rust; Q-WEB for frontend; Q-LINUX for OS/network behavior; Q-DOC for docs. External Codecov/CodeScene only after activation; report unavailable evidence. |
| Definition of Done | Task acceptance and negative tests pass; applicable quality bundle passes; security/architecture review recorded; docs and traceability updated; no secrets or unintended changes; unverified required integration keeps task open. |

### P7-07: Validate DNS/proxy MVP acceptance

Status: **planned**. Delivery: **MVP** (CR-0001). Complexity: **L**. Specification: §§22,24.

| Field | Deliverable / evidence |
|---|---|
| Goal and implementation scope | Exercise DNS/proxy install, onboarding, policies, diagnostics, rollback and restore through real UI/API without any tunnel or managed engine |
| Prerequisites | P7-04, P7-05, P7-06, P4-08, P10-07, P11-01, P11-03 |
| Expected files/modules | tests/e2e/single_node; docs/quality/MVP_ACCEPTANCE.md |
| Acceptance criteria | CR-0001 effective MVP checklist passes on a host without WG/AWG/Xray/TUN; original tunnel/engine checkboxes remain deferred and uncompleted |
| Tests | Real DNS UDP/TCP, TLS origin and selected proxy transport; NAT/ECH/QUIC/DNSSEC/IPv6/SSRF plus access and recovery negatives |
| Security considerations | No mocked DNS/gateway/proxy, removed safety gate or implied full-device traffic control |
| Quality checks | Q-RUST for Rust; Q-WEB for frontend; Q-LINUX for OS/network behavior; Q-DOC for docs. External Codecov/CodeScene only after activation; report unavailable evidence. |
| Definition of Done | Task acceptance and negative tests pass; applicable quality bundle passes; security/architecture review recorded; docs and traceability updated; no secrets or unintended changes; unverified required integration keeps task open. |

## P8: Remote Node Agent (6 tasks)

### P8-01: Enroll remote node with authenticated approval

Status: **planned**. Delivery: **POST_MVP_MULTI_NODE** (CR-0001). Complexity: **M**. Specification: §§11,19.

| Field | Deliverable / evidence |
|---|---|
| Goal and implementation scope | Single-use expiring token, controller verification, pending operator approval and mTLS identity |
| Prerequisites | P6-01, P1-05, P5-10 |
| Expected files/modules | crates/droutex-agent-core/src/enrollment; crates/droutex-api/src/nodes |
| Acceptance criteria | Replayed/expired/wrong-controller enrollment denied; approved node identity bound |
| Tests | Replay, stolen token after use, MITM and pending-node tests |
| Security considerations | No permanent admin SSH key; identity compromise recovery designed |
| Quality checks | Q-RUST for Rust; Q-WEB for frontend; Q-LINUX for OS/network behavior; Q-DOC for docs. External Codecov/CodeScene only after activation; report unavailable evidence. |
| Definition of Done | Task acceptance and negative tests pass; applicable quality bundle passes; security/architecture review recorded; docs and traceability updated; no secrets or unintended changes; unverified required integration keeps task open. |

### P8-02: Report actual node capabilities and health

Status: **planned**. Delivery: **POST_MVP_MULTI_NODE** (CR-0001). Complexity: **M**. Specification: §§7,11,20.

| Field | Deliverable / evidence |
|---|---|
| Goal and implementation scope | OS/kernel/ports/interfaces/engine compatibility and separately graded health |
| Prerequisites | P8-01, P5-08 |
| Expected files/modules | crates/droutex-agent-core/src/health |
| Acceptance criteria | Unknown metrics unavailable; process/transport/route/service health distinguished |
| Tests | Engine missing, kernel unsupported and handshake-only success cases |
| Security considerations | Telemetry contains no private credentials |
| Quality checks | Q-RUST for Rust; Q-WEB for frontend; Q-LINUX for OS/network behavior; Q-DOC for docs. External Codecov/CodeScene only after activation; report unavailable evidence. |
| Definition of Done | Task acceptance and negative tests pass; applicable quality bundle passes; security/architecture review recorded; docs and traceability updated; no secrets or unintended changes; unverified required integration keeps task open. |

### P8-03: Deliver and acknowledge authenticated snapshots

Status: **planned**. Delivery: **POST_MVP_MULTI_NODE** (CR-0001). Complexity: **M**. Specification: §§8,11,19.

| Field | Deliverable / evidence |
|---|---|
| Goal and implementation scope | Version negotiation, authenticated delivery, local validation/stage and per-node receipt |
| Prerequisites | P8-02, P6-05 |
| Expected files/modules | crates/droutex-agent-core/src/sync |
| Acceptance criteria | Wrong node, old sequence, corrupted hash and unsupported versions fail closed |
| Tests | Duplicate/out-of-order/drop packets and incompatible schema tests |
| Security considerations | Transport auth plus durable anti-replay; secrets delivered only to authorized node |
| Quality checks | Q-RUST for Rust; Q-WEB for frontend; Q-LINUX for OS/network behavior; Q-DOC for docs. External Codecov/CodeScene only after activation; report unavailable evidence. |
| Definition of Done | Task acceptance and negative tests pass; applicable quality bundle passes; security/architecture review recorded; docs and traceability updated; no secrets or unintended changes; unverified required integration keeps task open. |

### P8-04: Recover remote node with last-known-good

Status: **planned**. Delivery: **POST_MVP_MULTI_NODE** (CR-0001). Complexity: **M**. Specification: §§5,8,11,20.

| Field | Deliverable / evidence |
|---|---|
| Goal and implementation scope | Restart journal and disconnected-controller behavior; apply failures restore previous config |
| Prerequisites | P8-03, P1-06 |
| Expected files/modules | crates/droutex-agent-core/src/recovery |
| Acceptance criteria | Offline reboot serves last-good; interrupted apply never exposes partial network state |
| Tests | Controller outage, disk loss/full and power loss at transitions |
| Security considerations | Local state permissions and key-unavailable fail behavior explicit |
| Quality checks | Q-RUST for Rust; Q-WEB for frontend; Q-LINUX for OS/network behavior; Q-DOC for docs. External Codecov/CodeScene only after activation; report unavailable evidence. |
| Definition of Done | Task acceptance and negative tests pass; applicable quality bundle passes; security/architecture review recorded; docs and traceability updated; no secrets or unintended changes; unverified required integration keeps task open. |

### P8-05: Rotate and revoke node trust

Status: **planned**. Delivery: **POST_MVP_MULTI_NODE** (CR-0001). Complexity: **M**. Specification: §§11,19,21.

| Field | Deliverable / evidence |
|---|---|
| Goal and implementation scope | Certificate renewal, controller/node trust rotation and emergency revocation |
| Prerequisites | P8-04, P1-07 |
| Expected files/modules | crates/droutex-security/src/node_trust; docs/operations/NODE_RECOVERY.md |
| Acceptance criteria | Revoked node cannot receive new configs; offline data-plane behavior explicit and bounded by policy |
| Tests | Expired certificate, clock skew, overlap and compromise simulations |
| Security considerations | Revocation cannot magically kill disconnected node; report retained capabilities/secrets |
| Quality checks | Q-RUST for Rust; Q-WEB for frontend; Q-LINUX for OS/network behavior; Q-DOC for docs. External Codecov/CodeScene only after activation; report unavailable evidence. |
| Definition of Done | Task acceptance and negative tests pass; applicable quality bundle passes; security/architecture review recorded; docs and traceability updated; no secrets or unintended changes; unverified required integration keeps task open. |

### P8-06: Validate remote agent lifecycle and isolation

Status: **planned**. Delivery: **POST_MVP_MULTI_NODE** (CR-0001). Complexity: **M**. Specification: §§11,16,22.

| Field | Deliverable / evidence |
|---|---|
| Goal and implementation scope | Package unprivileged agent/helper, resource/shutdown behavior and end-to-end remote apply |
| Prerequisites | P8-05, P10-01 |
| Expected files/modules | tests/network/agent; deploy/systemd |
| Acceptance criteria | Remote installed agent applies/rolls back and survives independent engine/controller failures |
| Tests | Real two-host identity/control/recovery and privilege-negative tests |
| Security considerations | Separate service users and minimal capabilities |
| Quality checks | Q-RUST for Rust; Q-WEB for frontend; Q-LINUX for OS/network behavior; Q-DOC for docs. External Codecov/CodeScene only after activation; report unavailable evidence. |
| Definition of Done | Task acceptance and negative tests pass; applicable quality bundle passes; security/architecture review recorded; docs and traceability updated; no secrets or unintended changes; unverified required integration keeps task open. |

## P9: Multi-node networking (5 tasks)

### P9-01: Manage nodes groups and external proxies

Status: **planned**. Delivery: **POST_MVP_MULTI_NODE** (CR-0001). Complexity: **M**. Specification: §§11,12,17.

| Field | Deliverable / evidence |
|---|---|
| Goal and implementation scope | Registry, regions/groups and external proxy type without fabricated agent telemetry |
| Prerequisites | P8-06, P6-02 |
| Expected files/modules | crates/droutex-api/src/nodes; apps/droutex-web/src/features/nodes |
| Acceptance criteria | Managed node and external proxy lifecycle separate; references authorized |
| Tests | External proxy no-agent fixture, removed target and RBAC tests |
| Security considerations | Proxy credentials encrypted and restricted to assigned routes |
| Quality checks | Q-RUST for Rust; Q-WEB for frontend; Q-LINUX for OS/network behavior; Q-DOC for docs. External Codecov/CodeScene only after activation; report unavailable evidence. |
| Definition of Done | Task acceptance and negative tests pass; applicable quality bundle passes; security/architecture review recorded; docs and traceability updated; no secrets or unintended changes; unverified required integration keeps task open. |

### P9-02: Select and fail over egress by real health

Status: **planned**. Delivery: **POST_MVP_MULTI_NODE** (CR-0001). Complexity: **M**. Specification: §§2,7,11,20.

| Field | Deliverable / evidence |
|---|---|
| Goal and implementation scope | Pinned/first-healthy initial strategy; measured latency/weights later after comparability |
| Prerequisites | P9-01, P4-06 |
| Expected files/modules | crates/droutex-policy/src/node_selection |
| Acceptance criteria | Strict route blocks or switches only to allowed healthy target; cooldown avoids oscillation |
| Tests | Transport up/route down, timeout, flapping and resource failure cases |
| Security considerations | Never treat ping as proof of service health or fallback to direct |
| Quality checks | Q-RUST for Rust; Q-WEB for frontend; Q-LINUX for OS/network behavior; Q-DOC for docs. External Codecov/CodeScene only after activation; report unavailable evidence. |
| Definition of Done | Task acceptance and negative tests pass; applicable quality bundle passes; security/architecture review recorded; docs and traceability updated; no secrets or unintended changes; unverified required integration keeps task open. |

### P9-03: Integrate verified cross-node routing

Status: **planned**. Delivery: **POST_MVP_MULTI_NODE** (CR-0001). Complexity: **L**. Specification: §§2,6,10,22.

| Field | Deliverable / evidence |
|---|---|
| Goal and implementation scope | One pinned WG/AWG termination to compatible proxy outbound using explicit supported bridge |
| Prerequisites | P9-02, P5-06 |
| Expected files/modules | crates/droutex-agent-core/src/routes; tests/network/multi_node |
| Acceptance criteria | Observed egress is selected exit; termination and TCP/UDP limits documented |
| Tests | Actual two-node TUN/route/proxy engines, loops and leak cases |
| Security considerations | No fictional packet protocol conversion; hop credentials isolated |
| Quality checks | Q-RUST for Rust; Q-WEB for frontend; Q-LINUX for OS/network behavior; Q-DOC for docs. External Codecov/CodeScene only after activation; report unavailable evidence. |
| Definition of Done | Task acceptance and negative tests pass; applicable quality bundle passes; security/architecture review recorded; docs and traceability updated; no secrets or unintended changes; unverified required integration keeps task open. |

### P9-04: Coordinate partial deployment and rollback

Status: **planned**. Delivery: **POST_MVP_MULTI_NODE** (CR-0001). Complexity: **M**. Specification: §§8,11,12.

| Field | Deliverable / evidence |
|---|---|
| Goal and implementation scope | Per-node stage/activate/health state and failure-domain policy with divergence display |
| Prerequisites | P9-03, P8-03 |
| Expected files/modules | crates/droutex-core/src/multi_deployment; apps/droutex-web/src/features/deployments |
| Acceptance criteria | Partial failure visibly degraded; compatibility preserved or unauthorized routes blocked |
| Tests | Partition, slow node, mixed revision and coordinator restart tests |
| Security considerations | No distributed global-atomicity claim; rollback uses authorized increasing epoch |
| Quality checks | Q-RUST for Rust; Q-WEB for frontend; Q-LINUX for OS/network behavior; Q-DOC for docs. External Codecov/CodeScene only after activation; report unavailable evidence. |
| Definition of Done | Task acceptance and negative tests pass; applicable quality bundle passes; security/architecture review recorded; docs and traceability updated; no secrets or unintended changes; unverified required integration keeps task open. |

### P9-05: Validate topology diagnostics and acceptance

Status: **planned**. Delivery: **POST_MVP_MULTI_NODE** (CR-0001). Complexity: **L**. Specification: §§11,20,22.

| Field | Deliverable / evidence |
|---|---|
| Goal and implementation scope | Per-hop diagnostics, failure-domain evidence and multi-node operator runbook |
| Prerequisites | P9-04 |
| Expected files/modules | tests/e2e/multi_node; docs/operations/MULTI_NODE.md |
| Acceptance criteria | Route/health/version shown per hop; approved failover behavior proven |
| Tests | Real node outage, partial rollout, restored node and asymmetric IPv6 tests |
| Security considerations | No production runner/root or unapproved diagnostic target |
| Quality checks | Q-RUST for Rust; Q-WEB for frontend; Q-LINUX for OS/network behavior; Q-DOC for docs. External Codecov/CodeScene only after activation; report unavailable evidence. |
| Definition of Done | Task acceptance and negative tests pass; applicable quality bundle passes; security/architecture review recorded; docs and traceability updated; no secrets or unintended changes; unverified required integration keeps task open. |

## P10: Security and resilience (7 tasks)

### P10-01: Implement restricted privileged executor

Status: **planned**. Delivery: **POST_MVP_PROTOCOLS** (CR-0001). Complexity: **M**. Specification: §§15,19,22.

| Field | Deliverable / evidence |
|---|---|
| Goal and implementation scope | Post-MVP, implement authenticated typed interface/route/nftables/approved-engine helper before enabling privileged tunnel features |
| Prerequisites | P0-03, P1-02, P1-07, P11-06 |
| Expected files/modules | apps/droutex-node-executor; crates/droutex-agent-core/src/executor |
| Acceptance criteria | Unprivileged API cannot submit shell/executable/path commands; allowed operations idempotent/reversible |
| Tests | Unauthorized IPC, path injection, interface conflict and cleanup tests |
| Security considerations | Minimal capabilities, peer credentials, deny-by-default and independent review |
| Quality checks | Q-RUST for Rust; Q-WEB for frontend; Q-LINUX for OS/network behavior; Q-DOC for docs. External Codecov/CodeScene only after activation; report unavailable evidence. |
| Definition of Done | Task acceptance and negative tests pass; applicable quality bundle passes; security/architecture review recorded; docs and traceability updated; no secrets or unintended changes; unverified required integration keeps task open. |

### P10-02: Harden API identity and secret boundaries

Status: **planned**. Delivery: **MVP** (CR-0001). Complexity: **M**. Specification: §§17,18,19.

| Field | Deliverable / evidence |
|---|---|
| Goal and implementation scope | Review DNS/proxy bootstrap/RBAC/MFA/session/access/secret boundaries and prove MVP services cannot execute root/network-admin operations |
| Prerequisites | P6-08, P0-03 |
| Expected files/modules | docs/security/THREAT_MODEL.md; tests/security/auth |
| Acceptance criteria | Privilege escalation, last-owner races, credential disclosure and attempted OS/tunnel operations are denied; no custom privileged executor required in MVP |
| Tests | Adversarial API requests, capability/permission checks, brute-force limits and secret scanning |
| Security considerations | Separate browsing history/audit; master-key recovery constraints explicit |
| Quality checks | Q-RUST for Rust; Q-WEB for frontend; Q-LINUX for OS/network behavior; Q-DOC for docs. External Codecov/CodeScene only after activation; report unavailable evidence. |
| Definition of Done | Task acceptance and negative tests pass; applicable quality bundle passes; security/architecture review recorded; docs and traceability updated; no secrets or unintended changes; unverified required integration keeps task open. |

### P10-03: Fuzz DNS config and gateway parsers

Status: **planned**. Delivery: **MVP** (CR-0001). Complexity: **M**. Specification: §§9,10,22.

| Field | Deliverable / evidence |
|---|---|
| Goal and implementation scope | Target first-party policy/config/metadata parsing plus retained regression corpus |
| Prerequisites | P3-05, P4-03, P1-02 |
| Expected files/modules | fuzz; tests/fixtures/malformed |
| Acceptance criteria | Bounded smoke budget clean; every found crash/minimal input becomes regression |
| Tests | Pinned nightly isolated fuzz run; deterministic malformed corpus in ordinary stable tests |
| Security considerations | Nightly/fuzz optional locally, mandatory exposed-parser campaign before release |
| Quality checks | Q-RUST for Rust; Q-WEB for frontend; Q-LINUX for OS/network behavior; Q-DOC for docs. External Codecov/CodeScene only after activation; report unavailable evidence. |
| Definition of Done | Task acceptance and negative tests pass; applicable quality bundle passes; security/architecture review recorded; docs and traceability updated; no secrets or unintended changes; unverified required integration keeps task open. |

### P10-04: Test resource exhaustion and cancellation

Status: **planned**. Delivery: **MVP** (CR-0001). Complexity: **M**. Specification: §§10,16,20,22.

| Field | Deliverable / evidence |
|---|---|
| Goal and implementation scope | Measure DNS/TCP/proxy slow clients, floods, FD pressure, huge rules, bounded queues, cancellation and shutdown without external engines |
| Prerequisites | P3-06, P4-06, P1-06 |
| Expected files/modules | tests/load; docs/quality/PERFORMANCE_BASELINE.md |
| Acceptance criteria | Measured limits and recovery defined; no unbounded growth or stale session resources |
| Tests | Sustained/peak DNS and TCP, cancellation under disk/proxy/upstream failures |
| Security considerations | Circuit breakers and low-cardinality telemetry protect availability |
| Quality checks | Q-RUST for Rust; Q-WEB for frontend; Q-LINUX for OS/network behavior; Q-DOC for docs. External Codecov/CodeScene only after activation; report unavailable evidence. |
| Definition of Done | Task acceptance and negative tests pass; applicable quality bundle passes; security/architecture review recorded; docs and traceability updated; no secrets or unintended changes; unverified required integration keeps task open. |

### P10-05: Verify network security and privacy matrix

Status: **planned**. Delivery: **MVP** (CR-0001). Complexity: **L**. Specification: §§3,19,22,24.

| Field | Deliverable / evidence |
|---|---|
| Goal and implementation scope | Adversarial DNS/proxy NAT/IPv6/DNSSEC/ECH/QUIC/SSRF/rebinding/loop/authorization tests; defer tunnel segmentation to P5-10 |
| Prerequisites | P4-08, P10-02, P10-03 |
| Expected files/modules | tests/network/security; docs/quality/SECURITY_ACCEPTANCE.md |
| Acceptance criteria | No unauthorized gateway egress or profile leakage within the controlled boundary; unsupported paths fail accurately; no endpoint-wide enforcement claim |
| Tests | Real namespaces/packet captures plus parser/property negatives |
| Security considerations | No fake universal DNS enforcement when clients bypass DNS |
| Quality checks | Q-RUST for Rust; Q-WEB for frontend; Q-LINUX for OS/network behavior; Q-DOC for docs. External Codecov/CodeScene only after activation; report unavailable evidence. |
| Definition of Done | Task acceptance and negative tests pass; applicable quality bundle passes; security/architecture review recorded; docs and traceability updated; no secrets or unintended changes; unverified required integration keeps task open. |

### P10-06: Verify deployment crash and recovery behavior

Status: **planned**. Delivery: **MVP** (CR-0001). Complexity: **L**. Specification: §§8,20,21,22.

| Field | Deliverable / evidence |
|---|---|
| Goal and implementation scope | Fault-inject DNS/proxy publication, file/database/backup failures and CP outage; verify fresh-host recovery without kernel route or engine management |
| Prerequisites | P6-07, P1-06, P10-05 |
| Expected files/modules | tests/integration/recovery; docs/operations/RECOVERY.md |
| Acceptance criteria | Complete active config retained; explicit degraded status; restore reproducible |
| Tests | Power-loss DNS/proxy generation transitions, failed migration and authenticated snapshot rollback |
| Security considerations | Recovery respects sequence/auth; rollback does not promise session restoration |
| Quality checks | Q-RUST for Rust; Q-WEB for frontend; Q-LINUX for OS/network behavior; Q-DOC for docs. External Codecov/CodeScene only after activation; report unavailable evidence. |
| Definition of Done | Task acceptance and negative tests pass; applicable quality bundle passes; security/architecture review recorded; docs and traceability updated; no secrets or unintended changes; unverified required integration keeps task open. |

### P10-07: Approve security and resilience release evidence

Status: **planned**. Delivery: **MVP** (CR-0001). Complexity: **M**. Specification: §§19,22,24,25.

| Field | Deliverable / evidence |
|---|---|
| Goal and implementation scope | Independent security review and bounded performance evidence; accept residual risks explicitly |
| Prerequisites | P10-04, P10-06 |
| Expected files/modules | docs/quality/RELEASE_EVIDENCE.md |
| Acceptance criteria | P0 risks closed or accepted; no unverified required security behavior; coverage/health claims real |
| Tests | Replay MVP acceptance matrix and inspect evidence rather than scores alone |
| Security considerations | Licensing and dependency exceptions named with owner/expiry |
| Quality checks | Q-RUST for Rust; Q-WEB for frontend; Q-LINUX for OS/network behavior; Q-DOC for docs. External Codecov/CodeScene only after activation; report unavailable evidence. |
| Definition of Done | Task acceptance and negative tests pass; applicable quality bundle passes; security/architecture review recorded; docs and traceability updated; no secrets or unintended changes; unverified required integration keeps task open. |

## P11: Release preparation (6 tasks)

### P11-01: Package native install and preflight

Status: **planned**. Delivery: **MVP** (CR-0001). Complexity: **M**. Specification: §§21,24.

| Field | Deliverable / evidence |
|---|---|
| Goal and implementation scope | Package native DNS/proxy services with host/port/address/time/firewall preflight and private management; no VPN engines, TUN, kernel modules or NET_ADMIN required |
| Prerequisites | P4-08, P6-07, P10-02 |
| Expected files/modules | deploy/systemd; deploy/packaging; docs/operations/INSTALL.md |
| Acceptance criteria | Clean Linux host without WG/AWG/Xray/TUN installs and runs MVP without Docker; restricted identities and only required bind capability verified |
| Tests | Real fresh VM install, occupied ports, blocked access, dual-stack cases and no-NET_ADMIN execution |
| Security considerations | API unprivileged; DNS port 53 via narrow systemd binding capability/socket activation; no general root helper or automatic host routing changes |
| Quality checks | Q-RUST for Rust; Q-WEB for frontend; Q-LINUX for OS/network behavior; Q-DOC for docs. External Codecov/CodeScene only after activation; report unavailable evidence. |
| Definition of Done | Task acceptance and negative tests pass; applicable quality bundle passes; security/architecture review recorded; docs and traceability updated; no secrets or unintended changes; unverified required integration keeps task open. |

### P11-02: Document constrained Compose deployment

Status: **planned**. Delivery: **MVP** (CR-0001). Complexity: **M**. Specification: §§21.

| Field | Deliverable / evidence |
|---|---|
| Goal and implementation scope | Optional CP/DNS/TCP-proxy Compose deployment with explicit volumes/ports and minimal capabilities; no VPN/TUN/nftables setup |
| Prerequisites | P11-01 |
| Expected files/modules | deploy/compose; docs/operations/COMPOSE.md |
| Acceptance criteria | Native remains primary; DNS/proxy Compose works without privileged containers or NET_ADMIN |
| Tests | Compose smoke on a supported host with negative access/port/capability tests |
| Security considerations | Minimal privileges, secret mount and private admin bind |
| Quality checks | Q-RUST for Rust; Q-WEB for frontend; Q-LINUX for OS/network behavior; Q-DOC for docs. External Codecov/CodeScene only after activation; report unavailable evidence. |
| Definition of Done | Task acceptance and negative tests pass; applicable quality bundle passes; security/architecture review recorded; docs and traceability updated; no secrets or unintended changes; unverified required integration keeps task open. |

### P11-03: Validate upgrade migration and restore

Status: **planned**. Delivery: **MVP** (CR-0001). Complexity: **M**. Specification: §§17,21,24.

| Field | Deliverable / evidence |
|---|---|
| Goal and implementation scope | Backup before DNS/proxy upgrades, snapshot/DB compatibility and fresh-host restore; managed engine upgrade/export tests deferred |
| Prerequisites | P11-01, P10-06 |
| Expected files/modules | tests/e2e/upgrade; docs/operations/UPGRADE.md |
| Acceptance criteria | Corrupt/incompatible upgrade leaves last-good; verified encrypted restore usable |
| Tests | Old-to-new DB/snapshot fixtures, corrupt upgrade and real DNS/proxy host restore |
| Security considerations | Separate master key recovery and verified release artifacts |
| Quality checks | Q-RUST for Rust; Q-WEB for frontend; Q-LINUX for OS/network behavior; Q-DOC for docs. External Codecov/CodeScene only after activation; report unavailable evidence. |
| Definition of Done | Task acceptance and negative tests pass; applicable quality bundle passes; security/architecture review recorded; docs and traceability updated; no secrets or unintended changes; unverified required integration keeps task open. |

### P11-04: Write user/operator/client guides

Status: **planned**. Delivery: **MVP** (CR-0001). Complexity: **M**. Specification: §§3,12,14,21,26.

| Field | Deliverable / evidence |
|---|---|
| Goal and implementation scope | Write DNS/proxy setup, mode/limitations, access/profile/role administration, diagnostics and recovery guides; future tunnel guides separate; document personal DoH/DoT, legacy/manual and opt-in learned group access, private names and secret rotation |
| Prerequisites | P11-01, P7-06 |
| Expected files/modules | docs/user; docs/operations; README.md |
| Acceptance criteria | DNS/proxy guide matches real tests; no VPN client needed; DNS-only limits and decision-point DIRECT explicit |
| Tests | Follow docs on fresh installation and two authorized reference services |
| Security considerations | Secret delivery/retention and private admin recovery instructions |
| Quality checks | Q-RUST for Rust; Q-WEB for frontend; Q-LINUX for OS/network behavior; Q-DOC for docs. External Codecov/CodeScene only after activation; report unavailable evidence. |
| Definition of Done | Task acceptance and negative tests pass; applicable quality bundle passes; security/architecture review recorded; docs and traceability updated; no secrets or unintended changes; unverified required integration keeps task open. |

### P11-05: Build signed release and dependency evidence

Status: **planned**. Delivery: **MVP** (CR-0001). Complexity: **M**. Specification: §§7,19,22,25.

| Field | Deliverable / evidence |
|---|---|
| Goal and implementation scope | Build signed DNS/proxy release metadata, checksums/SBOM/license notices and tested reproducible artifacts; no bundled tunnel engines |
| Prerequisites | P11-03, P0-05, P10-07 |
| Expected files/modules | .github/workflows/release.yml; deploy/packaging; docs/operations/RELEASE.md |
| Acceptance criteria | Approved license inventory and artifact integrity evidence available; no untested engine bundle |
| Tests | Artifact checksum, install/upgrade and dependency advisory checks |
| Security considerations | No untrusted PR release credentials; engine redistribution separately reviewed |
| Quality checks | Q-RUST for Rust; Q-WEB for frontend; Q-LINUX for OS/network behavior; Q-DOC for docs. External Codecov/CodeScene only after activation; report unavailable evidence. |
| Definition of Done | Task acceptance and negative tests pass; applicable quality bundle passes; security/architecture review recorded; docs and traceability updated; no secrets or unintended changes; unverified required integration keeps task open. |

### P11-06: Decide DNS/proxy MVP release readiness

Status: **planned**. Delivery: **MVP** (CR-0001). Complexity: **M**. Specification: §§23,24.

| Field | Deliverable / evidence |
|---|---|
| Goal and implementation scope | Collect CR-0001 DNS/proxy evidence, limitations, unresolved risks and operator signoff before any tunnel work |
| Prerequisites | P0-04, P7-07, P11-02, P11-04, P11-05 |
| Expected files/modules | docs/quality/MVP_ACCEPTANCE.md; docs/planning/DEVELOPMENT_ROADMAP.md |
| Acceptance criteria | All effective DNS/proxy safety/product gates pass without WG/AWG/Xray/TUN; deferred engine/tunnel tests remain explicitly pending |
| Tests | Evidence review plus targeted rerun only for unresolved findings |
| Security considerations | No coverage percentage or Code Health score substitutes acceptance |
| Quality checks | Q-RUST for Rust; Q-WEB for frontend; Q-LINUX for OS/network behavior; Q-DOC for docs. External Codecov/CodeScene only after activation; report unavailable evidence. |
| Definition of Done | Task acceptance and negative tests pass; applicable quality bundle passes; security/architecture review recorded; docs and traceability updated; no secrets or unintended changes; unverified required integration keeps task open. |

## F: Future releases (5 tasks)

### F-01: Evaluate advanced engines and UDP routing

Status: **planned**. Delivery: **FUTURE** (CR-0001). Complexity: **L**. Specification: §§7,10,23.

| Field | Deliverable / evidence |
|---|---|
| Goal and implementation scope | Separate spikes for sing-box/Shadowsocks/Hysteria2/TUIC/Trojan/OpenVPN and UDP flow state |
| Prerequisites | P9-05 |
| Expected files/modules | docs/architecture/adr; future adapter crates |
| Acceptance criteria | Add engine only for confirmed requirement, compatible license and measured routing proof |
| Tests | Actual version-specific engine TCP/UDP/client matrix |
| Security considerations | Amplification, flow identity, timeout and privilege review |
| Quality checks | Q-RUST for Rust; Q-WEB for frontend; Q-LINUX for OS/network behavior; Q-DOC for docs. External Codecov/CodeScene only after activation; report unavailable evidence. |
| Definition of Done | Task acceptance and negative tests pass; applicable quality bundle passes; security/architecture review recorded; docs and traceability updated; no secrets or unintended changes; unverified required integration keeps task open. |

### F-02: Evaluate router profiles and complex matching

Status: **planned**. Delivery: **FUTURE** (CR-0001). Complexity: **L**. Specification: §§8,14,23.

| Field | Deliverable / evidence |
|---|---|
| Goal and implementation scope | CIDR/protocol/service selectors and router/client compatibility beyond accepted MVP subset |
| Prerequisites | P9-05 |
| Expected files/modules | crates/droutex-policy; docs/operations/ROUTERS.md |
| Acceptance criteria | IP cannot be treated as unique domain; complex rules preserve deterministic explain |
| Tests | Shared-CDN IP, split tunnel and router hardware tests |
| Security considerations | No inferred device identity or unrestricted route export |
| Quality checks | Q-RUST for Rust; Q-WEB for frontend; Q-LINUX for OS/network behavior; Q-DOC for docs. External Codecov/CodeScene only after activation; report unavailable evidence. |
| Definition of Done | Task acceptance and negative tests pass; applicable quality bundle passes; security/architecture review recorded; docs and traceability updated; no secrets or unintended changes; unverified required integration keeps task open. |

### F-03: Evaluate DRouteX client and inbound switching

Status: **planned**. Delivery: **FUTURE** (CR-0001). Complexity: **L**. Specification: §§7,14,23.

| Field | Deliverable / evidence |
|---|---|
| Goal and implementation scope | Separate product discovery for endpoint routes/DNS/IPv6 leak diagnostics and consented telemetry |
| Prerequisites | P11-06 |
| Expected files/modules | docs/planning/CLIENT_PROPOSAL.md |
| Acceptance criteria | Client support and secure update model approved before implementation |
| Tests | Actual supported platform/network transitions; sessions may disconnect |
| Security considerations | No server-only automatic inbound switching claim |
| Quality checks | Q-RUST for Rust; Q-WEB for frontend; Q-LINUX for OS/network behavior; Q-DOC for docs. External Codecov/CodeScene only after activation; report unavailable evidence. |
| Definition of Done | Task acceptance and negative tests pass; applicable quality bundle passes; security/architecture review recorded; docs and traceability updated; no secrets or unintended changes; unverified required integration keeps task open. |

### F-04: Evaluate verified subscriptions and notifications

Status: **planned**. Delivery: **FUTURE** (CR-0001). Complexity: **M**. Specification: §§12,20,23.

| Field | Deliverable / evidence |
|---|---|
| Goal and implementation scope | Approved service-list updates and opt-in email/webhook/Telegram/Matrix with anti-spam |
| Prerequisites | P11-06 |
| Expected files/modules | docs/planning/OPERATIONS_EXTENSIONS.md |
| Acceptance criteria | Unverified list never guaranteed; no silent rule publication; owner opts into channels |
| Tests | Malicious subscription, broad wildcard and notification rate tests |
| Security considerations | Signed/approved inputs, URL SSRF checks and minimal retained browsing data |
| Quality checks | Q-RUST for Rust; Q-WEB for frontend; Q-LINUX for OS/network behavior; Q-DOC for docs. External Codecov/CodeScene only after activation; report unavailable evidence. |
| Definition of Done | Task acceptance and negative tests pass; applicable quality bundle passes; security/architecture review recorded; docs and traceability updated; no secrets or unintended changes; unverified required integration keeps task open. |

### F-05: Evaluate PostgreSQL HA and advanced analytics

Status: **planned**. Delivery: **FUTURE** (CR-0001). Complexity: **L**. Specification: §§17,20,23.

| Field | Deliverable / evidence |
|---|---|
| Goal and implementation scope | Use measured concurrency/HA needs to justify storage/controller scaling and retention extension |
| Prerequisites | P9-05 |
| Expected files/modules | docs/architecture/adr; future storage migrations |
| Acceptance criteria | SQLite users retain supported path; HA consistency/failover design approved |
| Tests | Migration parity, coordinator fencing, failure domains and retention tests |
| Security considerations | Key lifecycle and aggregated privacy do not weaken at scale |
| Quality checks | Q-RUST for Rust; Q-WEB for frontend; Q-LINUX for OS/network behavior; Q-DOC for docs. External Codecov/CodeScene only after activation; report unavailable evidence. |
| Definition of Done | Task acceptance and negative tests pass; applicable quality bundle passes; security/architecture review recorded; docs and traceability updated; no secrets or unintended changes; unverified required integration keeps task open. |
