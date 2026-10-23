# Quality strategy

Status: active development policy. Use [EXECUTION_STATUS](../planning/EXECUTION_STATUS.md)
and the linked task reviews for current stage and exact-head results. Historical
setup results do not describe the current workspace. Frontend remains planned.
Specification §§16,19,22,24 plus accepted CR-0001/ADR-0016 are authoritative over generic skill defaults. Original engine/tunnel acceptance remains pending for G3.

## Mandatory bundles

**Q-RUST** (for each compatible selected feature set):

```sh
cargo fmt --all -- --check
cargo check --workspace --all-targets --locked
cargo clippy --workspace --all-targets --all-features --locked -- -D warnings
cargo test --workspace --locked
cargo deny --locked check
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
For instruction-only changes inspect the full diff and run applicable planning,
source-preservation and link checks, including added root Markdown files. The existing
planning validator does not scan every root file. Do not run unrelated Cargo/network
suites or coverage/score campaigns when application, operational-script and CI behavior
are unchanged. Documentation validation does not advance runtime review/CI gates.
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

## Evidence locations

| Question | Evidence source |
|---|---|
| Workspace, features and build commands | [Rust guide](../agents/rust.md), Cargo manifests and the active workflow |
| Current implementation and remaining checks | Current execution card and owning task's review |
| Tool installation and historical environment | [Environment report](../environment/ENVIRONMENT_REPORT.md), dated snapshots only |
| Actual Codecov/CodeScene results | Exact commits/runs in task reviews; integration docs describe setup/history |
| Native product acceptance | Relevant Q-LINUX task evidence; fixture success alone is insufficient |

## Coverage and code health

Use cargo-llvm-cov LCOV plus frontend LCOV as separate reports/flags. Target at least 90%
of new testable security-sensitive policy/parsing logic with meaningful negative/property
tests. Global coverage starts informational against a real baseline, not an arbitrary 90%.
Coverage does not prove route correctness, cryptographic safety or crash resilience.

CodeScene compares changed code with measured baseline, prioritizes actual hotspots and
coupling/health decline. New modules aim for healthy maintainability; accepted regressions
have owner/rationale. The integration is activated; an individual result is verified
only by its actual completed analysis on the relevant commit.

## Definition of Done

Each applicable task must satisfy spec and approved decisions, fmt/static analysis,
relevant unit/integration/negative/failure tests, meaningful coverage, reviewed security,
documented public contracts/architectural changes, no secrets and no unrelated changes.
Configured Codecov report generation and upload must succeed where applicable.
Review applicable CodeScene findings under the disposition process below.
The owner excludes auxiliary planning/setup/graph tools, tests, fixtures and docs
as independent metric targets. Missing/unavailable evidence is not a pass.
A required network behavior that was only mocked stays unverified and the task stays open.

## External findings and merge decisions

1. Record the service, exact head/base, run/link, raw status and concrete findings.
   A failed check remains failed even when its findings are accepted for deferral.
2. Inspect affected application code and critical operational scripts. Functional
   or security defects in any file still matter; an excluded file's score alone
   does not authorize metric-driven cleanup. Scores alone do not establish severity.
3. Fix serious defects and verify them before merging. A minor finding may be
   deferred only when immediate intervention is unnecessary and source inspection
   plus proportionate evidence supports preserving behavior. Record impact, rationale,
   evidence, owner/task or revisit trigger in [OPEN_DECISIONS](../planning/OPEN_DECISIONS.md)
   and link the disposition from the PR/task review. Existing task-specific accepted
   dispositions apply only while their evidence and scope still hold.
4. Ask the owner if severity, combined impact, deferral authority or a condition is
   unclear; continue independent work. Do not silently waive missing acceptance evidence.
5. A documented minor disposition permits a project acceptance decision under the
   existing owner policy; it does not make an external check green or authorize
   merging past a required GitHub check. Required checks, approvals, current-head
   Codex review and branch protections must all be satisfied. Do not suppress
   findings, weaken checks/protections or use an administrative bypass to merge.

```text
change → fmt/Clippy → unit/integration/negative tests → dependency/security scan
       → coverage → configured Codecov → configured CodeScene
       → architecture/spec review → PR ready
```

Local pre-commit hooks are optional convenience, never a substitute for CI. No hooks were
trusted or installed as a Git execution gate. Ponytail/Rust guidance supplements requirements;
it cannot remove boundary validation, address rechecks, errors or required acceptance tests.
