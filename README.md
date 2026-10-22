# DRouteX

Self-hosted SmartDNS, selective traffic routing and multi-protocol network management.
The accepted MVP scope is **DNS routing and authorized TCP proxying on one Linux VPS**.
Tunnels and managed WG/AWG/Xray engines follow after that release; see the
[accepted scope amendment](docs/architecture/adr/ADR-0016-dns-proxy-mvp-before-tunnels.md).
The owner accepted **P0-01 / CR-0002** on 2026-10-15. Foundation work through P0-05,
domain types and bounded configuration imports (P1-01/P1-02) are complete and merged.
P0-03 fixtures passed in WSL and native Debian 13. P1-03/04 storage/revisions,
P1-07 secrets and P1-05 authenticated snapshots are merged. The P1-06 local
activation/recovery completed in PR #11 with exact-main Windows/Linux CI verified.
P2-01 domain normalization/matching merged in PR #12 with exact-main CI verified.
P2-02 deterministic scoped selection merged in PR #13 with exact-main CI verified.
P2-03 publication conflict/resource validation merged in PR #14 with exact-main CI verified.
P2-04 separate DNS/TCP/local-answer intents and strict failures merged in PR #15
with exact-main CI verified. P2-07 local catalog imports and D42 TLS-qualification
security correction merged in PR #16 with exact-main CI verified. P2-05 complete
same-generation compilation merged in PR #105 with exact-main CI verified.
P2-06 generation-bound Explain/simulation merged in PR #106; all eleven exact-main
checks and both coverage reports passed. All seven P2 tasks are complete. The owner
accepted [authorization before DNS](docs/planning/AUTHORIZATION_BEFORE_DNS.md) and
the full [P4-01 profile](docs/planning/P4_01_BINDING_CONTRACT.md), completed in PR107
with all11 exact-main checks and both reports processed. The accepted
[P3-01 transport profile](docs/planning/P3_01_LISTENER_CONTRACT.md) completed
dependency qualification in PR108 with exact-main checks. The bounded DNS
[library](crates/droutex-dns/README.md) is under implementation/review; runtime
acceptance and merge remain open. No public service is deployed.

- [Development roadmap](docs/planning/DEVELOPMENT_ROADMAP.md)
- [Current execution boundary and repository handoff](docs/planning/EXECUTION_STATUS.md)
- [MVP boundary](docs/planning/MVP_SCOPE.md)
- [P0-01 contracts and ingress decision](docs/planning/P0_01_FOUNDATION_REVIEW.md)
- [P0-03 network fixture evidence](docs/planning/P0_03_NETWORK_HARNESS.md)
- [MVP threat model](docs/security/THREAT_MODEL.md)
- [Control D reference analysis](docs/research/CONTROLD_ANALYSIS.md)
- [Accepted Profiles / DNS Endpoints / filters / proxy model](docs/architecture/DNS_PRODUCT_MODEL.md)
- [Actionable backlog](docs/planning/IMPLEMENTATION_BACKLOG.md)
- [Architecture and proposed decisions](docs/architecture/ARCHITECTURE_OVERVIEW.md)
- [Environment and verification](docs/environment/ENVIRONMENT_REPORT.md)
- [Quality strategy](docs/quality/QUALITY_STRATEGY.md)
- [SQLite storage contract](crates/droutex-storage/README.md)
- [P1-03 validation evidence](docs/planning/P1_03_STORAGE_REVIEW.md)
- [P1-06 activation contract](docs/planning/P1_06_ACTIVATION_CONTRACT.md)
- [P1-06 validation evidence](docs/planning/P1_06_ACTIVATION_REVIEW.md)
- [P2-01 domain profile and gates](docs/planning/P2_01_DOMAIN_CONTRACT.md)
- [English source specification](DRouteX_Project_Specification_EN.md)

Windows PowerShell development environment: `. ./tools/activate.ps1`.
This uses local Rust tooling and existing Visual Studio without persistent PATH edits.
Read AGENTS.md before work. The user authorized starting MVP roadmap work on 2026-10-15;
P0-01 is accepted; the current execution boundary is P4-01 with its recorded gates.
See [Rust workspace posture](docs/agents/rust.md) for build commands and scope.

Original DRouteX code is licensed under [Apache-2.0](LICENSE), selected by the owner
on 2026-10-15. Vendored third-party skills/resources retain their own licenses and
notices; see [their provenance](docs/environment/SKILLS_AND_TOOLS.md) and the
[SQLx patch provenance](vendor/sqlx-sqlite/README.droutex.md).
