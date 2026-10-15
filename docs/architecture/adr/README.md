# Architecture decision records

ADR-0001–0015 and ADR-0017 remain **Proposed** on 2026-10-15. ADR-0016 is **Accepted** as the user-authorized DNS/proxy MVP scope amendment. The user authorized starting roadmap work and Control D research/plan adaptation; concrete technical contracts remain in P0-01 review. Administrator bootstrap plus later role assignment remains a confirmed requirement.

[P0-01 review](../../planning/P0_01_FOUNDATION_REVIEW.md) connects the proposals to
three network flows, five failure cases and the [threat model](../../security/THREAT_MODEL.md).
Public changing-IP, DNS-only network/group access is confirmed; concrete
registration/binding/snapshot/Linux contracts remain for owner review.

| ADR | Topic |
|---|---|
| [ADR-0001](ADR-0001-workspace-and-process-topology.md) | Workspace and process topology |
| [ADR-0002](ADR-0002-deterministic-policy-and-routing-semantics.md) | Deterministic policy and routing semantics |
| [ADR-0003](ADR-0003-dns-only-compatibility-and-destination-binding.md) | DNS-only compatibility and destination binding |
| [ADR-0004](ADR-0004-protocol-adapter-and-capability-boundaries.md) | Protocol adapter and capability boundaries |
| [ADR-0005](ADR-0005-secrets-and-restricted-privileged-operations.md) | Secrets and restricted privileged operations |
| [ADR-0006](ADR-0006-node-authentication-and-enrollment.md) | Node authentication and enrollment |
| [ADR-0007](ADR-0007-sqlite-and-future-postgresql-migration.md) | SQLite and future PostgreSQL migration |
| [ADR-0008](ADR-0008-native-deployment-and-optional-compose.md) | Native deployment and optional Compose |
| [ADR-0009](ADR-0009-licensing-and-external-engine-distribution.md) | Licensing and external-engine distribution |
| [ADR-0010](ADR-0010-runtime-snapshots-and-durable-activation.md) | Runtime snapshots and durable activation |
| [ADR-0011](ADR-0011-dns-cache-isolation-and-configuration-consistency.md) | DNS cache isolation and configuration consistency |
| [ADR-0012](ADR-0012-internal-messages-and-public-api-formats.md) | Internal messages and public API formats |
| [ADR-0013](ADR-0013-windows-development-and-linux-network-verification.md) | Windows development and Linux network verification |
| [ADR-0014](ADR-0014-metrics-and-structured-event-ownership.md) | Metrics and structured event ownership |
| [ADR-0015](ADR-0015-versioning-and-compatibility-policy.md) | Versioning and compatibility policy |
| [ADR-0016](ADR-0016-dns-proxy-mvp-before-tunnels.md) | **Accepted:** DNS/proxy MVP before tunnels (CR-0001) |
| [ADR-0017](ADR-0017-profile-endpoint-dns-product-model.md) | **Proposed:** Profiles, DNS Endpoints, personal secure DNS, catalogs and separate proxy grants (CR-0002) |
