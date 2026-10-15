# Routira

Self-hosted SmartDNS, selective traffic routing and multi-protocol network management.
The accepted MVP scope is **DNS routing and authorized TCP proxying on one Linux VPS**.
Tunnels and managed WG/AWG/Xray engines follow after that release; see the
[accepted scope amendment](docs/architecture/adr/ADR-0016-dns-proxy-mvp-before-tunnels.md).
MVP roadmap work has started with **P0-01 architecture/security review**.
This directory contains specifications, development tools and review documents;
no application workspace exists yet.

- [Development roadmap](docs/planning/DEVELOPMENT_ROADMAP.md)
- [Current execution boundary and repository handoff](docs/planning/EXECUTION_STATUS.md)
- [MVP boundary](docs/planning/MVP_SCOPE.md)
- [P0-01 contracts and ingress decision](docs/planning/P0_01_FOUNDATION_REVIEW.md)
- [MVP threat model](docs/security/THREAT_MODEL.md)
- [Control D reference analysis](docs/research/CONTROLD_ANALYSIS.md)
- [Proposed Profiles / DNS Endpoints / filters / proxy model](docs/architecture/DNS_PRODUCT_MODEL.md)
- [Actionable backlog](docs/planning/IMPLEMENTATION_BACKLOG.md)
- [Architecture and proposed decisions](docs/architecture/ARCHITECTURE_OVERVIEW.md)
- [Environment and verification](docs/environment/ENVIRONMENT_REPORT.md)
- [Quality strategy](docs/quality/QUALITY_STRATEGY.md)
- [English source specification](Routira_Project_Specification_EN.md)

Windows PowerShell development environment: `. ./tools/activate.ps1`.
This uses local Rust tooling and existing Visual Studio without persistent PATH edits.
Read AGENTS.md before work. The user authorized starting MVP roadmap work on 2026-10-15;
P0-01 is in review before dependent implementation begins.
