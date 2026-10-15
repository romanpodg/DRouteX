# Bounded execution goal and repository handoff

Date: 2026-10-15. Current boundary: **P0-03 in review; WSL fixtures passed, native Debian 13 pending**.
Repository: [romanpodg/Routira](https://github.com/romanpodg/Routira).
Review: [PR #1](https://github.com/romanpodg/Routira/pull/1), open against main.
No merging, force-pushing, branch-protection change or later task implementation is authorized.

## Owner decisions and current progress

The owner explicitly accepted [P0-01/CR-0002](P0_01_FOUNDATION_REVIEW.md) in this chat
on 2026-10-15 and instructed continuation through P0-02. Accepted foundation:
Profiles, personal DoH/DoT Endpoints, separate legacy/gateway group grants, opt-in
bounded source observation, private-record isolation, fixed policy layers and safe
snapshot/origin/failure constraints. Debian 13 x86_64 is the first native acceptance
target. Detailed cryptography, lease/clock/resource values, exact outbound transport
and actual operator lab retain their owning gates.

The owner also approved Rust 1.99.0/MSRV, Edition 2024, no external dependencies,
minimal common/config/policy crates and a limited Windows/Linux CI job. After a
real cargo-deny license failure the owner chose **Apache-2.0** for original Routira
code, rather than deferring the check. LICENSE and workspace metadata record it;
third-party licenses are preserved. D14's external notices/inventory work remains.

Current backlog: **2 completed (P0-01/P0-02), 1 in_review (P0-03), 85 planned**.
Two of the 61 MVP tasks are complete; the WSL fixture pass does not close P0-03.
The application foundation exists; there is no running DNS resolver, TCP gateway,
controller, frontend, storage or product network-test evidence. P1/P2 features are
not passed by a workspace bootstrap. No task beyond P0-03 is started.

Prepared work includes preserved EN/RU specifications, accepted CR-0001/CR-0002,
17 ADRs (only ADR-0016/0017 Accepted), threat model, W1-W3/F1-F5 design walkthroughs,
188 requirement mappings, the 88-task DAG and installed tooling. The MVP contains
61 tasks; tunnels/helper/multi-node remain later work.

## P0-03 continuation and actual lab evidence

The user asked for the next step and explicitly selected the existing WSL lab.
The approved missing nftables/tcpdump packages and their distro dependencies were
installed there. Windows networking, production hosts, Docker and cloud resources
were not configured. The lab uses Python stdlib/OpenSSL, so no new application
crate/dependency or empty routira-testkit was added.

The [P0-03 review](P0_03_NETWORK_HARNESS.md) records 41 actual dual-stack protocol
observations, four portable boundary tests, a bounded IPv4/IPv6 capture with source
mismatch pairs, preserved origin certificate/observed egress, strict proxy outage,
and normal/setup-failure/SIGTERM/SIGKILL cleanup. All passed in WSL; the tested host
network state matched before/after. Keys/config use private tmpfs; local captures
and reports stay untracked. The CI addition runs only unprivileged boundary tests.

No native Debian 13 VM/VPS/image was supplied. P0-03 stays in_review for that proof;
this is fixture readiness, not Routira networking/security or systemd acceptance.

## P0-02 implementation and local evidence

The three crates provide shared RuleLayer vocabulary, the pure accepted precedence
constant and typed configuration rejection of reordered/missing/duplicate/extra
layers. No generic engine framework, wire parser or network service is introduced.
See [Rust posture](../agents/rust.md) for dependency direction, inherited warning/unsafe
policy and exact commands. Cargo.lock contains only the three local packages.

On Windows with tools/activate.ps1, fmt/check/Clippy passed, including no-default
and all-features static checks. Two Rust integration tests passed with default and
no-default features. Inverting the validation branch made both tests fail; restoring
it returned them to green. cargo-llvm-cov produced local coverage/p0-02.lcov:
**62.50% of 8 instrumented source lines**, with the Display implementation uncovered.
The accepted/rejected validation outcomes are tested; branch coverage was not measured.
This is local Rust evidence, not a Codecov service report or a coverage target waiver.

The first offline cargo-deny attempt lacked a cached RustSec database. After fetching
it, advisories/bans/sources passed but licenses failed on the then-unlicensed local
packages. After applying the selected Apache-2.0 license and a narrow explicit
allow-list, `cargo deny --locked --offline check` passed advisories, bans, licenses
and sources against the fetched database. No third-party dependency is present.
No failure is hidden by an exclusion or an advisory suppression.

The planning validators check owner acceptance, prerequisite completion, the P0-02
bound, exactly three crates, 61-task MVP closure, 188 mappings, 17 ADRs and preserved
EN/RU hashes. Nine intentionally invalid planning cases and six unsafe CI mutations
are rejected. Fifteen Python regression tests passed with a workspace-local Temp;
the system Temp path was denied by this Windows sandbox. No dependency installation
was needed: the existing quality-venv provides Python 3.12.13/PyYAML 6.0.3.

The limited Workspace CI job runs format/check/Clippy/tests on Ubuntu and Windows
with pinned Rust, read-only permissions and no credentials/upload/network privileges.
For implementation commit `7ac5f14deb25bdb687b0aee2008e6fda11734be1`,
[Actions run 38760651619](https://github.com/romanpodg/Routira/actions/runs/38760651619)
passed both Workspace jobs, both Planning jobs and Codecov upload.
This is Windows/Linux compile/test evidence, not Debian network compatibility. It does not complete
P0-04's full product CI/coverage/dependency/failure-injection acceptance.
Native Debian DNS/TLS/proxy/systemd acceptance remains P0-03 and later integration.

## Implementation check and earlier quality evidence

[CodeScene review 7972404](https://codescene.io/projects/86145/delta/results/7972404)
completed on implementation head 7ac5f14 with the same one auxiliary argument-count
finding (9.69), and two other quality gates passed. No application-code finding is
reported by that Check Run. The repository-wide check remains failed; no successful
product Code Health score is claimed. This finding is outside the owner's selected
quality-improvement scope, as recorded in AGENTS.md; it is not suppressed or treated
as a P0-02 application defect.

## Earlier remote evidence

At the start, local HEAD and open PR #1 matched `023ce200ebf9b3e1b53bb0a1218576d8db092e46`.
[Actions run 38756886961](https://github.com/romanpodg/Routira/actions/runs/38756886961)
passed Planning on Ubuntu and Windows and Codecov upload; the planning patch check
also passed. [CodeScene review 7971972](https://codescene.io/projects/86145/delta/results/7971972)
failed one argument-count rule in tools/build_doc_graph.py:add_markdown_links (9.69).
That auxiliary score-only finding is outside the owner's quality-improvement scope;
no refactor, suppression or gate change was made. Later heads need their own results.
Copilot's only review reported quota exhaustion, not successful code review.

[CodeScene integration](../quality/CODESCENE_INTEGRATION.md) and
[Codecov integration](../quality/CODECOV_INTEGRATION.md) preserve service activation
and earlier report evidence. No product CodeScene/Codecov pass is inferred from setup.

## Stopping condition and continuation

P0-02 is closed after full-diff review, local checks and actual Windows/Linux build
results. The owner subsequently authorized P0-03 and the WSL lab variant. P0-03
remains in_review for native Debian 13 evidence; P0-04/P0-05 are not started.
Stop before their implementation or new infrastructure without the owner decision.
No automatic merge, production deployment or unsolicited reviewer message is authorized.

The user will manually reopen the chat after usage limits and requested no new
reminder. No reminder/automation was created for this goal. The previously configured
monitor-routira-foundation-pr is separate; its scheduler state was not checked or
changed in this continuation.
