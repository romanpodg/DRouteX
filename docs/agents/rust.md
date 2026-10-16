# Rust workspace posture (P0-02 foundation, P1-01 domain model)

Owner-approved on 2026-10-15: Rust toolchain/MSRV **1.99.0**, Edition **2024**,
resolver 3, initially no third-party dependencies and a minimal common/config/policy workspace.
The toolchain is pinned in rust-toolchain.toml; package metadata requires Rust 1.99.
MSRV is the oldest compiler verified here, not a claim about older compilers.

All packages inherit workspace metadata and lints. Unsafe code is forbidden and
warnings are denied. Clippy uses its standard lint set with `-D warnings`; no
blanket lint suppression, panic-on-input path, external engine or network service
is introduced. There are no optional/default features; default, no-default and
all-features checks currently address the same code.

Dependency direction is config → policy → common (config also uses common's types).
Common defines RuleLayer and P1-01 resource identity/state types; pure policy owns the accepted layer order; config rejects
missing/duplicate/reordered/extra layers with a typed error. This is a useful
foundation contract, not rule matching, config parsing or a running resolver.
P1-01's [domain review](../planning/P1_01_DOMAIN_REVIEW.md) separates model evidence
from later verifier/crypto/schema/runtime acceptance. P1/P2 retain those owning gates.

Cargo.lock is tracked. Internal path dependencies have exact workspace versions;
the production registry dependency is owner-approved uuid 1.28.0, std parse/format
only. It introduces no Windows/Linux runtime dependency beyond uuid itself.
The all-target lock records 19 registry artifacts; exact provenance is in
[UUID_INVENTORY](../operations/UUID_INVENTORY.csv). No ID/token RNG feature,
project build script or executable package is added.
All packages have publish=false. The owner selected Apache-2.0 for original Routira
code during P0-02. LICENSE contains the official text; package metadata inherits it.
P0-05 owner approval expanded deny.toml for third-party dependencies to Apache-2.0,
MIT, ISC, BSD-3-Clause and Unicode-3.0, preserving their texts/notices. Original
Routira code remains Apache-2.0. The standalone tests/dependencies package qualifies
the proposed network stack; it is not a production workspace dependency. Exact
versions, audit decisions and test limits are in
[DEPENDENCIES](../operations/DEPENDENCIES.md). P0-04 owns the full CI policy.

On Windows, dot-source `. ./tools/activate.ps1` before Cargo commands. Run:

```sh
cargo fmt --all -- --check
cargo check --workspace --all-targets --locked
cargo check --workspace --all-targets --no-default-features --locked
cargo clippy --workspace --all-targets --all-features --locked -- -D warnings
cargo test --workspace --locked
cargo test --workspace --no-default-features --locked
cargo deny --locked check
```

The Workspace CI job runs format/check/Clippy/tests on Windows and Linux with
read-only permissions and no secrets or privileged network setup. P0-04 adds an
Ubuntu cargo-deny/LLVM coverage job and a separate trusted OIDC upload job; exact
versions, actual runs and deferred evidence are in the
[CI review](../planning/P0_04_CI_REVIEW.md). Native Debian 13 fixture acceptance
passed in P0-03; product networking and systemd acceptance remain at later gates.
