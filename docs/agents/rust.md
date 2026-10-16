# Rust workspace posture (P0-02)

Owner-approved on 2026-10-15: Rust toolchain/MSRV **1.99.0**, Edition **2024**,
resolver 3, no third-party dependencies and a minimal common/config/policy workspace.
The toolchain is pinned in rust-toolchain.toml; package metadata requires Rust 1.99.
MSRV is the oldest compiler verified here, not a claim about older compilers.

All packages inherit workspace metadata and lints. Unsafe code is forbidden and
warnings are denied. Clippy uses its standard lint set with `-D warnings`; no
blanket lint suppression, panic-on-input path, external engine or network service
is introduced. There are no optional/default features; default, no-default and
all-features checks currently address the same code.

Dependency direction is config → policy → common (config also uses common's types).
Common defines RuleLayer; pure policy owns the accepted layer order; config rejects
missing/duplicate/reordered/extra layers with a typed error. This is a useful
foundation contract, not rule matching, config parsing or a running resolver.
P1/P2 retain their full model/schema/normalization/compiler acceptance criteria.

Cargo.lock is tracked. Internal path dependencies have exact workspace versions;
there are no registry/git dependencies, build scripts or executable packages.
All packages have publish=false. The owner selected Apache-2.0 for original Routira
code during P0-02. LICENSE contains the official text; package metadata inherits it.
deny.toml currently allows only that approved license. P0-05 must review external
dependency licenses before expanding the list; P0-04 owns the full CI policy.

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
