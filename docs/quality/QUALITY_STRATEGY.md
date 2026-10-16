# Quality strategy

Status: P0-04 foundation CI active; actual-run evidence in the CI review, frontend absent.
Specification §§16,19,22,24 plus accepted CR-0001/ADR-0016 are authoritative over generic skill defaults. Original engine/tunnel acceptance remains pending for G3.

## Mandatory bundles

**Q-RUST** (for each compatible selected feature set):

```sh
cargo fmt --all -- --check
cargo check --workspace --all-targets --locked
cargo clippy --workspace --all-targets --all-features --locked -- -D warnings
cargo test --workspace --locked
cargo deny check
```

The requested warning-free baseline is intentional. P0-02 records lint posture in
Cargo.toml and docs/agents/rust.md; Rust skills must not weaken it. Allow/expect exceptions
need rationale and narrow scope. All-features runs only when features can coexist:
also test defaults, no-default and each supported exclusive/backend/transport set.
P0-04 records the explicit matrix in its [CI review](../planning/P0_04_CI_REVIEW.md).
The current supported compiler and MSRV are both pinned to Rust 1.99.0; no rolling
latest-stable compatibility claim is made. Add separate jobs when another compiler
or a distinct feature combination becomes supported.

Nextest: `cargo nextest run --workspace --locked` is a supported alternative for
unit/integration runs; retain `cargo test --workspace --doc --locked` because
nextest does not replace doctests. Do not execute the same full test suite repeatedly
unless new changes/failures justify it.

Dependency checks: cargo-deny advisories/bans/licenses/sources with approved license
posture; cargo-audit useful scheduled independent RustSec scan. Lockfiles are tracked;
unknown/ignored advisories require owner/expiry/rationale, not blanket suppression.

**Q-WEB:** pinned package manager/lockfile, TypeScript check, lint, Vitest+matched
@vitest/coverage-v8, relevant component/browser tests, production build and coverage.
Use responsive/keyboard/reduced-motion/accessibility checks for changed flows.
No runtime tooling is added just to satisfy planning-session checks.

**Q-LINUX:** actual supported Linux namespace/VM/VPS with real DNS/TCP/TLS, a selected
protected proxy outbound, packet/egress evidence and service-permission/failure/recovery checks.
MVP must pass without WG/AWG/Xray/TUN or NET_ADMIN. Pinned engine/client/tunnel/helper tests
are additional G3 gates. Dedicated isolated runner with a trusted workflow; no root on
production or for arbitrary fork code.

**Q-DOC:** task fields/prerequisites/traceability valid, MVP dependency closure excludes
future work, relative links resolve, ADR status accurate and source specs unchanged.
User-authorized CR-0001 is explicit; original deferred obligations remain traceable.
Accepted CR-0002 has recorded owner evidence: validate the revised secure-DNS
and catalog prerequisite gates, 61-task MVP closure and ADR-0017's Accepted state.
Q-LINUX now includes personal DoH/DoT capability/roaming and separate gateway grant
tests under that revised plan; no product evidence is implied by planning checks.

The [foundation workflow](../../.github/workflows/planning.yml) reuses the existing
Windows/Linux workspace and planning jobs, adds locked dependency checks and Rust
LCOV, and isolates OIDC uploads. Application and historical planning reports use
separate flags. The [Rust quality proposal](proposals/rust-quality.yml.example)
remains a reference; it is not a second active workflow. Actual positive/negative
run evidence and the deferred fork test are recorded in the P0-04 review.

## Verification state now

| Check | State |
|---|---|
| rustc/cargo/rustfmt/Clippy/rust-analyzer/LLVM tools | Installed; versions/components verified |
| Edition 2024 native compile/link/assertion | Executed successfully with portable Windows SDK and existing MSVC |
| nextest/llvm-cov/deny/audit | Installed; production and standalone qualification lockfiles audited; actual evidence in EXECUTION_STATUS.md |
| fmt/check/clippy/test/deny/coverage on Routira | Three-crate workflow active; P1-01 Linux/Windows CI passed, including default/no-default and four compile-fail examples |
| Feature combinations/MSRV/dependency resolution | Owner approved Rust 1.99.0/MSRV; P1-01 adds qualified uuid 1.28.0 std parse/format only, without RNG or optional application features |
| Frontend type/lint/Vitest/browser coverage | Planned; no application package exists |
| Real Linux DNS/proxy/namespaces/VPS | P0-03 WSL and native Debian 13 dual-stack fixtures/pcap/four cleanup modes passed; product networking/systemd acceptance remains pending; engine/tunnel tests deferred to G3 |
| Codecov/CodeScene | P1-01 implementation head 4d79b7a: CodeScene 7987296 passed all three gates; Codecov processed exact main comparison with 71/71 new Rust lines and successful backend patch check. Historical failed auxiliary score checks remain recorded without suppression; actual fork test remains deferred |
| Planning graph/schema/link/DAG checks | Evidence recorded in environment report after final verification |

## Coverage and code health

Use cargo-llvm-cov LCOV plus frontend LCOV as separate reports/flags. Target at least 90%
of new testable security-sensitive policy/parsing logic with meaningful negative/property
tests. Global coverage starts informational against a real baseline, not an arbitrary 90%.
Coverage does not prove route correctness, cryptographic safety or crash resilience.

CodeScene compares changed code with measured baseline, prioritizes actual hotspots and
coupling/health decline. New modules aim for healthy maintainability; accepted regressions
have owner/rationale. It is an additional review gate only after actual account activation.

## Definition of Done

Each applicable task must satisfy spec and approved decisions, fmt/static analysis,
relevant unit/integration/negative/failure tests, meaningful coverage, reviewed security,
documented public contracts/architectural changes, no secrets and no unrelated changes.
Configured Codecov report generation/upload and applicable CodeScene review of
application code/critical operational scripts must succeed. The owner excludes
auxiliary planning/setup/graph tools, tests, fixtures and documentation as independent
metric targets; record their score-only findings without cleanup or suppression.
A failed repository-wide check is still reported as failed.
When external integration is not configured, record it as unavailable; do not fabricate a pass.
A required network behavior that was only mocked stays unverified and the task stays open.

```text
change → fmt/Clippy → unit/integration/negative tests → dependency/security scan
       → coverage → configured Codecov → configured CodeScene
       → architecture/spec review → PR ready
```

Local pre-commit hooks are optional convenience, never a substitute for CI. No hooks were
trusted or installed as a Git execution gate. Ponytail/Rust guidance supplements requirements;
it cannot remove boundary validation, address rechecks, errors or required acceptance tests.
