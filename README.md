# Routira

Self-hosted SmartDNS, selective traffic routing and multi-protocol network management.
The accepted MVP scope is **DNS routing and authorized TCP proxying on one Linux VPS**.
Tunnels and managed WG/AWG/Xray engines follow after that release; see the
[accepted scope amendment](docs/architecture/adr/ADR-0016-dns-proxy-mvp-before-tunnels.md).
The owner accepted **P0-01 / CR-0002** on 2026-10-15. Foundation work through P0-05,
domain types and bounded configuration imports (P1-01/P1-02) are complete and merged.
P0-03 fixtures passed in WSL and native Debian 13. P1-03 SQLite storage is being
validated in PR #6; no Routira DNS/gateway service exists yet.

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
- [SQLite storage contract](crates/routira-storage/README.md)
- [P1-03 validation evidence](docs/planning/P1_03_STORAGE_REVIEW.md)
- [English source specification](Routira_Project_Specification_EN.md)

Windows PowerShell development environment: `. ./tools/activate.ps1`.
This uses local Rust tooling and existing Visual Studio without persistent PATH edits.
Read AGENTS.md before work. The user authorized starting MVP roadmap work on 2026-10-15;
P0-01 is accepted; the current execution boundary is P1-03 with its recorded gates.
See [Rust workspace posture](docs/agents/rust.md) for build commands and scope.

Original Routira code is licensed under [Apache-2.0](LICENSE), selected by the owner
on 2026-10-15. Vendored third-party skills/resources retain their own licenses and
notices; see [their provenance](docs/environment/SKILLS_AND_TOOLS.md) and the
[SQLx patch provenance](vendor/sqlx-sqlite/README.routira.md).
