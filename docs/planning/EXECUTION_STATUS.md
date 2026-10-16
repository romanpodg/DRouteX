# Bounded execution goal and repository handoff

Date: 2026-10-17. Current stage: **P1-02 implemented locally; independent review and Linux CI pending; no PR yet**.
Repository: [romanpodg/Routira](https://github.com/romanpodg/Routira).
Foundation: [PR #1](https://github.com/romanpodg/Routira/pull/1) merged on
2026-10-16 at `c43c2be0d40248f81bb01be9964ba56925b368f5`. Independent Codex
review found a fixture stdout buffering defect; the fix and its regression test
passed a follow-up review. All six final-head and main Actions jobs passed.
The owner accepted P0-04 and authorized merging PR #1 after actual Codex Code
Review and applicable current-head checks, with no unresolved serious defect.
Minor deferrals must be recorded; unclear requirements/conditions require an owner
question. Finish and merge the current implementation PR before the next one.
The owner subsequently instructed continuation of the accepted DNS/proxy MVP plan.
P1-02 is the current stage; technical choices retain their owning gates.
Force-pushing and branch-protection changes remain unauthorized.

The repository is now public. With explicit owner approval the existing Codecov
project was activated without changing OIDC, tokens, access or plan. Re-upload
attempt 2 of [run 38865306388](https://github.com/romanpodg/Routira/actions/runs/38865306388)
passed; Codecov processed both backend/planning reports for the exact merge commit.
The [service report](https://app.codecov.io/github/romanpodg/routira/commit/c43c2be0d40248f81bb01be9964ba56925b368f5)
shows Rust config coverage of 5/8 lines (62.50%); aggregate 437/443 includes auxiliary
Python tooling and is not application coverage. P1-01 subsequently verified a
meaningful new-code PR comparison below; the actual fork test remains unverified.

For P0-05 the owner selected **HTTP CONNECT over TLS** and approved isolated
compatibility/transitive-license qualification of Tokio 1.53.3, Hickory-Proto
0.26.3, Rustls 0.23.46 and Tokio-Rustls 0.26.6 using AWS-LC. Production workspace
adoption is not implied. The owner separately approved the dependency license
allowlist recorded in D14. See the
[dependency review](../operations/DEPENDENCIES.md).

[PR #3](https://github.com/romanpodg/Routira/pull/3) qualifies that inventory.
On head `1ed74f933fdbfc5cf3265a4a4c82648fd1278234`, all seven Actions jobs passed,
including Linux DNS/TLS tests and both locked dependency checks. Independent Codex
Code Review completed without a confirmed defect after an earlier quota-interrupted
attempt. Windows tests/Clippy and the fresh 41-observation WSL lab also passed.
CodeScene's only failed gate is a test-method size score outside the owner's
independent metric scope; the check remains failed and its disposition is in
[OPEN_DECISIONS](OPEN_DECISIONS.md#p0-05-review-dispositions). No production networking
dependency, runtime, certificate provisioning or dataset was adopted. Final
documentation evidence subsequently passed its independent review and all seven
current-head jobs. PR #3 merged normally at
`a77ed5adce184ffe3214452af0b1e6d83db2cd92`; all seven main jobs also passed.

The owner then selected UUIDv4 public resource IDs and qualification of uuid 1.28.0.
Its official archive/checksum/license, two Windows probe tests and cargo-deny passed.
P1-01 uses only std parsing/formatting, with no generator, crypto, API or listener.
The [domain review](P1_01_DOMAIN_REVIEW.md) records 17 distinct ID types, separate
confidential material/public metadata, scoped DNS associations and independent
gateway identity data. Seven integration and four compile-fail tests, fmt/check/
Clippy/rustdoc and no-default checks passed locally. New executable domain code
has 72/72 instrumented lines covered; full workspace is 77/80. This is local data,
not runtime authentication proof. [PR #4](https://github.com/romanpodg/Routira/pull/4)
implementation head `4d79b7a95f094851bd2b73c4c924a85c7231de44` passed independent
read-only built-in Code Review against main `a77ed5adce184ffe3214452af0b1e6d83db2cd92`,
all seven jobs of [run 38911280732](https://github.com/romanpodg/Routira/actions/runs/38911280732)
and all three CodeScene gates (review 7987296). Codecov processed both reports and
the exact-head/base comparison: new Rust 71/71, backend 76/79 (96.20%), backend
patch check 100%. The aggregate patch includes five auxiliary Python lines; it is
not a product total. Final evidence head `0a09f3a38afafe91ad4031fb7657091210d8cd67`
also passed independent review, all seven jobs of run 38912299627, all three
CodeScene gates and the exact-base/head Codecov comparison. PR #4 merged normally
at `cd2a9c011e4950d80458cfaa72baddd1ac4243c2`; all seven main jobs in run
38912831307 passed. Local review and merge records are in
`.setup/p1-01/autopilot-completed.json`.

P1-02 is on `config/validate-external-boundaries` from that verified merge. The owner
approved the five parser/path dependencies, strict three-format input profile,
1 MiB/depth 32/10,000-entry limits and safe diagnostics. Qualification passed before
config-crate adoption. Local full/default and no-default tests, Clippy, all-target
check, root dependency audit and LCOV passed. The [config review](P1_02_CONFIG_REVIEW.md)
and [wire contract](../../api/schemas/README.md) distinguish import validation from
live admission, policy compilation and dataset/secret/runtime gates. Independent
review and Linux CI are pending; no P1-02 PR or P1-03 implementation exists yet.

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

Current backlog: **6 completed (P0-01 through P0-05 and P1-01), 1 in progress (P1-02), 81 planned**.
Six of the 61 MVP tasks have task evidence; P0-04 closes by explicit owner acceptance
with the deferred evidence below, not by claiming every acceptance test passed.
The application foundation exists; there is no running DNS resolver, TCP gateway,
controller, frontend, storage or product network-test evidence. P1/P2 features are
not passed by a workspace bootstrap. PR #4 is merged. P1-02 must complete its own
review/checks and merge before any next implementation PR; task status cannot bypass that gate.

Prepared work includes preserved EN/RU specifications, accepted CR-0001/CR-0002,
17 ADRs (only ADR-0016/0017 Accepted), threat model, W1-W3/F1-F5 design walkthroughs,
188 requirement mappings, the 88-task DAG and installed tooling. The MVP contains
61 tasks; tunnels/helper/multi-node remain later work.

## Accepted selective reuse from PR #2

On 2026-10-15 the owner selected **option A** after the read-only comparison:
continue on PR #1 and reuse suitable PR #2 components and test scenarios only
when their existing roadmap tasks are authorized and prerequisites are complete.
This accepts a reuse strategy, not the old Veyra specification, task completion,
dependency selection, whole-branch merge or blanket cherry-pick. No donor code has
been imported. P0-03 was in_review when this strategy was selected and is now
complete after the native run below; P0-04 is accepted and P0-05 qualification is complete.

Reviewed references: PR #1 at
`5628f2ae3306bf86b22283b19003008fd3246bd7`; [PR #2](https://github.com/romanpodg/Routira/pull/2)
at `3542426b744306ac71ca255161f5ebf65d3e1189`. This is a static source assessment;
the donor was not checked out, built or executed. Its reported 147 tests and live
curl result were not independently reproduced. Later donor changes need review.

| Candidate from PR #2 | Owning tasks | Required adaptation before reuse |
|---|---|---|
| policy name/pattern primitives and normalization tests | P2-01 | Current explicit match modes/types; approved IDNA dependency |
| permutation tests, validation/conflict diagnostics and Explain ideas | P1-02/P2-02/P2-03/P2-06 | CR-0002 layers and priority, typed resources, bounded input, endpoint/dataset/generation provenance |
| DNS framing, truncation, forwarding and TTL/negative-cache tests | P3-01/P3-02/P3-03 | Separate access context, bounded connections/queues/deadlines and scoped cache generations |
| TLS fixtures, fragmentation, numeric dial and byte-preservation scenarios | P4-02/P4-03/P4-04 | Separate destination grants, validated CNAME/address provenance, complete self-target checks, parser/ECH and lifecycle validation |

Do not transplant these old decisions:

- [Gateway authorization](https://github.com/romanpodg/Routira/blob/3542426b744306ac71ca255161f5ebf65d3e1189/crates/veyra-gateway/src/gateway.rs#L118)
  reduces source admission to a boolean and unions all Profile/Device routes.
  Its [cross-profile allowance test](https://github.com/romanpodg/Routira/blob/3542426b744306ac71ca255161f5ebf65d3e1189/crates/veyra-policy/tests/gateway.rs#L103)
  must become a denial regression. Keep separate exact-address, expiring legacy
  and gateway grants; DNS capability never establishes transparent TCP identity.
- The old specificity/scope/action ordering and CIDR admission model do not replace
  CR-0002's custom/service/filter/default layers, within-layer priority or distinct
  Endpoint/credential/group identities. Old BLOCK and HTTPS/SVCB answer choices do
  not close D09; a shared raw-upstream cache is not the accepted scoped cache design.
- [Egress self detection](https://github.com/romanpodg/Routira/blob/3542426b744306ac71ca255161f5ebf65d3e1189/crates/veyra-gateway/src/egress.rs#L69)
  omits additional local public addresses behind wildcard binds;
  [origin lookup](https://github.com/romanpodg/Routira/blob/3542426b744306ac71ca255161f5ebf65d3e1189/crates/veyra-dns/src/upstream.rs#L55)
  collects unrelated Answer addresses without owner/CNAME-chain validation.
  Repair those boundaries and keep test-only egress bypasses out of production APIs.
- [DNS TCP tasks](https://github.com/romanpodg/Routira/blob/3542426b744306ac71ca255161f5ebf65d3e1189/crates/veyra-dns/src/server.rs#L95)
  have no connection cap; query permits do not bound connections and writes have
  no deadline. Network components need bounded lifecycle and tracked shutdown.
- The unimplemented old apply/deploy plan defers authenticated snapshots and
  replay protection. Follow P1-05/P1-06 instead, preserving coordinated recovery,
  increasing activation sequence and revocation through rollback.

Keep the current Routira workspace, Rust 1.99.0, Apache-2.0 posture, specifications,
ADRs and backlog. Do not import the old root manifests/lockfile or large plans as
authority. P0-05 reviews dependency versions/licenses and the selected protected
outbound. The donor tree/manifests declare no license; establish contribution
permission/license and preserve attribution before copying its code or fixtures.
Every actual port needs a focused diff review and tests against current contracts;
donor tests do not establish native Debian or product acceptance.

## P0-04 CI evidence

The existing workflow now includes locked cargo-deny checks and separate Rust
LCOV/OIDC upload, retaining the Windows/Linux workspace and planning matrix.
The [CI review](P0_04_CI_REVIEW.md) records exact positive/negative runs, coverage
and security boundaries. All three intentional lint/test/upload failures were
observed on a disposable same-repository branch, then that ref was removed.
The private fork policy requires approval with no secrets/variables or write
tokens. On 2026-10-16 the owner accepted P0-04 after reviewing the disclosed
limitations. The real fork PR test remains deferred; the Codecov comparison/web-report
gap is now resolved in [OPEN_DECISIONS](OPEN_DECISIONS.md#deferred-p0-04-evidence).
PR #1 subsequently passed actual Code Review after the fixture fix and merged;
the service-report gap is closed by the processed main and P1-01 reports above.

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

On 2026-10-16 the owner supplied a separate Debian 13.4 x86_64 KVM VPS and approved
tcpdump/libpcap installation plus execution of the reviewed fixtures. Verified SSH
host identity and the source hashes preceded the run. All 41 protocol observations,
both source-mismatch pairs and the four cleanup modes passed on kernel
6.12.85+deb13-amd64. The 1,621-packet capture and report were downloaded and their
hashes matched the VPS; private keys/raw evidence remain ignored and access-limited.
Host network state matched before/after; subsequent checks found no fixture
processes/named namespaces and only SSH listening, as before. Source, two installed
packages and root-private evidence remain on the test VPS for reproducibility.
Exact versions, installed-state identity, hashes and limitations are in the
[P0-03 review](P0_03_NETWORK_HARNESS.md#native-debian-13-acceptance-2026-10-16).
P0-03 is complete; product networking/security and systemd acceptance remain later work.

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
Native Debian fixture acceptance passed in P0-03; product DNS/TLS/proxy/systemd
acceptance remains in later integration tasks.

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
is now complete after the owner-approved native Debian 13 run. On 2026-10-16 the
owner authorized publishing those results and proceeding through P0-04 only.
See [P0-04 review](P0_04_CI_REVIEW.md) for scope and actual check evidence.
The owner subsequently authorized continuation; P0-05 qualification is now complete.
New infrastructure still requires its own owner decision.
The owner now permits autonomous merge under the PR policy in [AGENTS.md](../../AGENTS.md).
Actual Code Review and current-head validation subsequently passed after the
fixture buffering fix, and PR #1 merged. Production deployment and unrelated
reviewer messages remain unauthorized.

The owner authorized the existing monitor-routira-foundation-pr heartbeat to run
the bounded P0-04 review/fix/merge loop in the automation chat every 15 minutes,
with routine successful-run alerts disabled. It was paused after PR #1 merged.
Reuse that heartbeat for authorized roadmap continuation; preserve its cadence and
notification policy. It must not repeat the completed PR #1 loop or send routine
unchanged-state updates. Await owner answers for consequential unresolved choices.
