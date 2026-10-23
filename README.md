# DRouteX

Self-hosted SmartDNS, selective traffic routing and multi-protocol network management.
The accepted MVP scope is **DNS routing and authorized TCP proxying on one Linux VPS**.
Tunnels and managed WG/AWG/Xray engines follow after that release; see the
[accepted scope amendment](docs/architecture/adr/ADR-0016-dns-proxy-mvp-before-tunnels.md).
Development is in progress; no public service is deployed. The
[current execution card](docs/planning/EXECUTION_STATUS.md) records completed stages,
active work and open review/CI gates. [Task status](docs/planning/IMPLEMENTATION_BACKLOG.md)
is generated from the canonical backlog; exact historical results live in task reviews.

- [Development roadmap](docs/planning/DEVELOPMENT_ROADMAP.md)
- [Current execution boundary and repository handoff](docs/planning/EXECUTION_STATUS.md)
- [MVP boundary](docs/planning/MVP_SCOPE.md)
- [P0-01 contracts and ingress decision](docs/planning/P0_01_FOUNDATION_REVIEW.md)
- [P0-03 network fixture evidence](docs/planning/P0_03_NETWORK_HARNESS.md)
- [Security policy and private vulnerability reporting](SECURITY.md)
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
Read [AGENTS.md](AGENTS.md) before work, then the current execution card and affected task contract.
See [Rust workspace posture](docs/agents/rust.md) for build commands and scope.

Original DRouteX code is licensed under [Apache-2.0](LICENSE), selected by the owner
on 2026-10-15. Vendored third-party skills/resources retain their own licenses and
notices; see [their provenance](docs/environment/SKILLS_AND_TOOLS.md) and the
[SQLx patch provenance](vendor/sqlx-sqlite/README.droutex.md).
