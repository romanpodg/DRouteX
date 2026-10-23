# Rust workspace development

Rust toolchain/MSRV **1.99.0**, Edition **2024**, resolver 3 are owner-approved.
[rust-toolchain.toml](../../rust-toolchain.toml) and [Cargo.toml](../../Cargo.toml)
are the build authority. MSRV means the oldest compiler verified here.
All packages inherit workspace metadata/lints and are non-publishable libraries.
Unsafe code is forbidden and warnings are denied; Clippy uses `-D warnings`.
Do not weaken these rules through generic skill defaults.

## Workspace and dependency boundaries

The workspace contains seven crates; manifests define exact dependency direction.

| Crate | Responsibility |
|---|---|
| droutex-common | Domain IDs, shared states and wire-independent primitives |
| droutex-policy | Pure domain normalization, matching and deterministic policy evaluation |
| droutex-security | Redacted secret material, envelope encryption and snapshot signatures |
| droutex-config | Bounded import, compilation, validation and snapshot/config contracts |
| droutex-storage | SQLite persistence, revision metadata and encrypted-record storage |
| droutex-core | Local activation/recovery and current generation/lease/quota admission |
| droutex-dns | Bounded UDP/TCP transport library; acceptance follows the P3-01 gates |

Current acceptance is in [EXECUTION_STATUS](../planning/EXECUTION_STATUS.md).
Library/socket tests do not establish a deployed resolver, TCP gateway or frontend.
No daemon/application package is introduced by this guide.

[Cargo.lock](../../Cargo.lock) is tracked. Use the reviewed exact dependency and
feature profiles in manifests; isolated qualifier packages under tests/ are
separate from production adoption. Adding a feature, dependency or platform
provider beyond its accepted profile retains the owning decision/qualification gate.
The SQLx SQLite patch and its notices remain under
[vendor provenance](../../vendor/sqlx-sqlite/README.droutex.md).
Dependency/license inventories and qualification evidence are linked from the
[decision register](../planning/OPEN_DECISIONS.md) and owning task reviews.
The original P0-02 dependency-free and P1-01 uuid-only states are historical.

## Commands and platform evidence

On Windows dot-source `. ./tools/activate.ps1` before Cargo commands. This uses
local Rust/SDK tools and existing MSVC without persistent PATH edits.

```sh
cargo fmt --all -- --check
cargo check --workspace --all-targets --locked
cargo check --workspace --all-targets --no-default-features --locked
cargo clippy --workspace --all-targets --all-features --locked -- -D warnings
cargo test --workspace --locked
cargo test --workspace --no-default-features --locked
cargo deny --locked check
```

Run applicable bundles from [QUALITY_STRATEGY](../quality/QUALITY_STRATEGY.md).
Manifests define the feature surface; defaults/no-default/all-features are tested
where compatible, without claiming distinct coverage when they select identical code.
Do not rerun a full suite without a changed requirement, source change or failure.

The [active workflow](../../.github/workflows/planning.yml) defines the actual
Windows/Linux workspace, qualifier and coverage jobs. Test jobs run unprivileged,
without secrets; the trusted OIDC upload job has separate authority.
The actual fork test remains owner-deferred. Native Debian 13 product/systemd
acceptance is separate from completed P0-03 fixture runs and hosted Linux CI.
Keep mandatory IPv6 CI evidence; a development-host network failure is not a skip
or permission to modify host networking.
