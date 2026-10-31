# Bounded execution goal and repository handoff

## Current execution card

Reviewed 2026-11-01. This card is the current work/authorization entry point;
[backlog.json](backlog.json) is canonical for task status/dependencies/delivery.
Task reviews hold exact acceptance evidence. Later dated evidence supersedes
historical checkpoints below; reconcile conflicts before dependent work.

- The owner accepted the complete [D54 cache contract](P3_03_CACHE_CONTRACT.md)
  by direct reply on 2026-11-01 and requested implementation plus Project/Roadmap
  updates. P3-03 is in progress under that exact profile; later tasks remain planned.
  The [application evidence](P3_03_CACHE_REVIEW.md) records implementation progress
  and remaining platform, allocation, account review and merge gates.
  P3-02's temporary empty-RDATA compatibility disposition remains unchanged.
- Completed: P0, P1, all seven P2 tasks, P4-01 and P3-01. PR112 corrected the
  strict durable fixture without weakening its counter/failure assertions.
  Its normal merge8f234c44a0c47fc3ef4583e775a591cd90158185 passed all11
  exact-main40097388131 jobs:244 Windows/276 Linux tests per default/no-default
  profile, zero failed/ignored/filtered. Both reports processed(Rust7902/8287).
  The earlier failed40095014751 and skipped upload remain recorded as failed.
- PR112 actual implementation review01a16dfe-6ed2-7573-8db0-7d2edb88e721 and
  final whole-PR review01a16e09-7ae3-7c43-a9d3-2c6b36eaffda completed without
  concrete defects. Final-head40096734289 passed all11 jobs/both reports and
  all3 CodeScene8109468 gates. A capacity-interrupted final-review attempt is
  unverified and was not counted. See [P3-01 evidence](P3_01_LISTENER_REVIEW.md).
- Completed additional scope: owner-accepted [D51 isolated DNS-client qualification](P3_02_CLIENT_QUALIFICATION_CONTRACT.md),
  exactly hickory-net0.26.3 defaults-off/tokio against the existing Proto/Tokio
  profiles. Its checked P3-01 prerequisite is fulfilled. Only a separate tests
  workspace/lockfile, mandatory controlled UDP/TCP/platform/bounds/lifecycle and
  dependency/license checks are authorized. [Qualification evidence](P3_02_CLIENT_QUALIFICATION_REVIEW.md)
  tracks actual results. PR113 completed the implementation/final reviews and
  normal head/merge/exact-main/report gates on2026-10-28.
- The owner accepted [D52 bounded ingress/caller-validation profile](P3_02_CLIENT_VALIDATION_CONTRACT.md)
  on2026-10-24 before dependent adapters. The tests-only composed role now has
  mandatory question/frame validation, pre-filter I/O budgets, one original500ms
  deadline and explicit owned cleanup. Local default/no-default each33/37 pass;
  four mandatory raw/composed IPv6 probes fail on this host, without ignores or
  filters. Seven removed-check mutations fail as intended; sources are restored.
  A failed-bind reservation defect has a red/green regression. The earlier UDP
  fixture timeout remains unattributed; clean mandatory CI now passes both profiles.
  Full graph/archive audit and the local seventh deny bundle passed. Production
  adoption was not authorized by D51/D52 alone; accepted D53 below supplies its
  separate conditional authority. [Evidence](P3_02_CLIENT_QUALIFICATION_REVIEW.md)
  distinguishes original raw-library behavior from composed-role proof.
- PR113 final head435bc37 passed all11 CI40630686323 jobs; normal merge
  f136ac82d7e6421dbb139f022a98575ea26ca8ff passed all11 exact-main40631471007
  jobs. Actual full logs show244 Windows/277 Linux tests per default/no-default
  profile and client37/37 per profile/platform, including mandatory IPv6, with
  zero failed/ignored/filtered. All seven deny bundles pass and both reports
  processed(Rust7902/8287). Implementation account review01a172e1-ffa9-7b13-88ab-312f92c93875
  and final review01a180b2-c3c3-7630-89e7-b325ffd4c659 completed without confirmed
  defects. CodeScene8161842 remains failed with the existing
  [specific dispositions](OPEN_DECISIONS.md#p3-02-qualification-review-dispositions).
  Earlier failed CI40191024088/40630402694 remain failed. The corrected test-only
  readiness precondition and deterministic commit-window regression pass; original
  receive-timeout causation remains unproven. [Evidence](P3_02_CLIENT_QUALIFICATION_REVIEW.md#corrected-final-head-and-normal-merge).
- The owner accepted the complete [D53 forwarding contract](P3_02_FORWARDING_CONTRACT.md)
  by direct reply on2026-10-28 after checked PR113 completion. It authorizes
  bounded P3-02 implementation and conditional exact-client adoption, including
  immutable numeric public upstream groups, explicit fallback, current admission,
  fixed deadlines and owned resource/cleanup limits. No public deployment or
  infrastructure change is authorized. PR114 merged normally at3d866cb and all11
  exact-main40633658696 jobs/both processed reports passed before implementation.
  P3-02 subsequently completed its independent application gates below. D51/D52
  alone still confer no production authority; D53 remains the exact bounded profile.
- Completed: P3-02 through normal [PR115](https://github.com/romanpodg/DRouteX/pull/115)
  merge `d70210246637321c1513a392ba86528565933b6a`. Final head `2a2fe8b` passes all 11
  CI `41190906637` jobs; all 11 exact-main `41191488105` jobs also pass. Full logs confirm
  286 Windows/319 Linux tests per default/no-default profile, client 37/37 with all
  mandatory IPv6, zero failed/ignored/filtered, seven successful deny bundles and
  both processed reports (main Rust 9956/10417). Actual implementation/whole-PR final
  account reviews and current-head corrective delta review completed. The Linux
  constructor P1 is fixed; prior failed reviews/runs and local IPv6 failures stay
  recorded as failed. See [final application evidence](P3_02_FORWARDING_REVIEW.md#completed-implementation-and-normal-merge).
- The legal empty-NULL/unknown RDATA compatibility defect remains unfixed under
  the [temporary minor disposition](OPEN_DECISIONS.md#p3-02-forwarding-review-dispositions).
  A valid empty record in any section of an otherwise ordinary response can cause
  explicit fallback or SERVFAIL. Failed preservation evidence is retained; full RR
  transparency, a vendor patch or another profile is not accepted. CodeScene 8217152
  remains failed with rechecked specific dispositions. This prerequisite evidence does not complete P3-03 or authorize deployment.
- Recorded CodeScene failures and concrete minor dispositions remain failed and
  documented in [OPEN_DECISIONS](OPEN_DECISIONS.md#p3-01-review-dispositions).
  No suppression, branch-protection bypass or CI-status exception is authorized.
- D49 allows explicit UDP IPv4/IPv6 bind addresses only; reject 0.0.0.0/:: before
  workers, preserving schema/TCP wildcard support. D48 allocates 1500ms processing
  plus the remaining 500ms reply within the original 2s total. Lost/expired
  admission never receives a fallback.
- No public deployment, management endpoint, TLS/recursion/cache/secure learning or
  actual origin connector is authorized by P3-01/P3-02. Resolve now uses the bounded
  D53 upstream handler; other policy handlers remain unavailable. Development-host IPv6 failure does not
  waive mandatory clean-CI IPv6 or authorize host-network changes.

## Development guidance maintenance, 2026-10-23

The owner authorized implementing all seven documentation-review proposals:
remove stale holds, shorten AGENTS and define source authority, consolidate
clarifications, reconcile external-finding dispositions, update security control
status/evidence, add SECURITY.md and add focused PR review rules. This is maintenance
of the current project, not a next implementation task or a change to runtime contracts.
Historical results remain in task reports/Git history; base EN/RU specs and ADR
acceptance states are preserved. The owner also authorized GitHub-account PR
comments/Codex review requests and chose GitHub Private Vulnerability Reporting.
Its API state changed from disabled to enabled and was read back as enabled on
2026-10-23. No response/remediation SLA is promised.
Documentation validation: the planning validator and its nine negative cases passed;
88 tasks, 61-task MVP closure, 188 requirement mappings and 17 ADRs (two Accepted)
remain consistent, and both source specifications retain their qualified hashes.
Changed Markdown paths and heading anchors were checked, including new SECURITY.md;
the full diff and whitespace checks passed. No Cargo/network suite was run for prose
changes, and no runtime CI/coverage/review pass is claimed by this validation.
The validator's generated report was restored to its prior bytes to avoid publishing
an unrelated generated artifact; this maintenance changes Markdown only.
The local document-graph refresh was refused: the historical AST baseline would
reduce graph.json from 4944 to 1381 nodes, including unrelated native/source nodes.
graph.json was preserved without forcing an overwrite. Use current sources for
changed instructions; repair the graph refresh baseline at a separate tooling task.

## Historical implementation and authorization record

The following records preserve earlier checkpoints, including superseded holds.
Their then-current words and test counts do not override the execution card,
completed task evidence or later owner decisions.

Repository: [romanpodg/DRouteX](https://github.com/romanpodg/DRouteX).
Owner-authorized project maintenance: [DRouteX rename](DROUTEX_RENAME.md).
Foundation: [PR #1](https://github.com/romanpodg/DRouteX/pull/1) merged on
2026-10-16 at `c43c2be0d40248f81bb01be9964ba56925b368f5`. Independent Codex
review found a fixture stdout buffering defect; the fix and its regression test
passed a follow-up review. All six final-head and main Actions jobs passed.
The owner accepted P0-04 and authorized merging PR #1 after actual Codex Code
Review and applicable current-head checks, with no unresolved serious defect.
Minor deferrals must be recorded; unclear requirements/conditions require an owner
question. Finish and merge the current implementation PR before the next one.
The owner subsequently instructed continuation of the accepted DNS/proxy MVP plan.
P2-06 completed normal merge671c9f6, all11 main39662936089 jobs and both processed
reports(Rust5980/6262), after implementation/final linked-account reviews. See
[P2-06 evidence](P2_06_EXPLAIN_REVIEW.md). The owner accepted
[D45 order](AUTHORIZATION_BEFORE_DNS.md) and full [D46 profile](P4_01_BINDING_CONTRACT.md);
P4-01 completed normal PR107 merge02b6695, all11 main39747433154 jobs and both
reports(Rust6689/7013), after both linked-account reviews. See
[P4-01 evidence](P4_01_BINDING_REVIEW.md). Full [D47 transport](P3_01_LISTENER_CONTRACT.md)
was accepted after that checked prerequisite; isolated qualifier PR108 merged atd36d7b2c5233822113d1085ac85dc9dd422fa401; all11 exact-main39753424789 jobs and both Codecov sessions passed. Actual bounded listener implementation is current.
Later technical choices retain their owning gates.
PR9 merged at521bfa76fbc27e14cd4981638af775ba92c6299f after independent
implementation/final-evidence reviews and all11 final-head jobs. All11 exact-main
jobs39148795165 passed and both Codecov reports processed. The owner then accepted
the complete [D36 local activation profile](P1_06_ACTIVATION_CONTRACT.md).
P1-06 subsequently merged normally at6b58b892cf2e9c58499cd60d4ccad9c503a1d656;
all11 main jobs39154498259 passed, and both exact-merge Codecov reports processed
(Rust3322/3503). The owner accepted the [D37 domain profile](P2_01_DOMAIN_CONTRACT.md)
and conditional IDNA qualification/adoption. Qualification head a60b3e4 passed all11
Windows/Linux jobs39156440746, fresh dependency checks and independent linked-account
read-only/never review01a1505d-f3c3-7491-9281-ce5be624a3fa with no confirmed defect.
Bounded production normalization/matching subsequently merged normally in
[PR12](https://github.com/romanpodg/DRouteX/pull/12) at6fa475976df6a9194dcab3ccdbc3c759308cd7b0.
All11 final-head/main jobs completed successfully; both exact-merge reports processed
(Rust3406/3589). The earlier queued metadata recovered after the owner selected
waiting; no exception or check bypass was used. See [evidence](P2_01_DOMAIN_REVIEW.md).
The owner accepted [D38 scoped selection](P2_02_EVALUATION_CONTRACT.md) after that
merge. P2-02 subsequently merged normally in [PR13](https://github.com/romanpodg/DRouteX/pull/13)
at ecc8a9e9cc903b6264b502cfdc407eb4fd70d5ab; all11 exact-main jobs39296627839 passed
and both Codecov reports processed (Rust3638/3828). See [evidence](P2_02_EVALUATION_REVIEW.md).
The owner accepted [D39 publication validation](P2_03_PUBLICATION_CONTRACT.md) after
that gate. P2-03 merged normally at a50c6a432c08e1a4422ae625a25b392e54a62d42 after
linked-account implementation/final reviews and all11 final-head jobs. All11 exact-main
jobs39476373259 passed and both Codecov reports processed (Rust4071/4274).
See [completed evidence](P2_03_PUBLICATION_REVIEW.md). D40 owner-accepted
[strict actions](P2_04_ACTION_CONTRACT.md) subsequently merged normally at
b4c104c8e87acdb388b6ee5455dfc5bd01ab331d after linked-account implementation/final
reviews and all11 current-head jobs. All11 exact-main39481364319 jobs passed,
both Codecov reports processed (Rust4329/4535); [evidence](P2_04_ACTION_REVIEW.md).
Owner accepted [D41 local catalog imports](P2_07_CATALOG_CONTRACT.md) after this gate;
P2-07 subsequently completed normally at e8c5e6105ffa7426115eda4e7243d8f26eea5d17
with implementation/security/final linked-account reviews, all11 final-head39546423405
and exact-main39546838691 jobs, and both processed Codecov reports (Rust4951/5177,
patch612/642). Owner D42 WebPKI0.103.16 qualification resolved the original advisory
without suppression/production TLS adoption; see [completed evidence](P2_07_CATALOG_REVIEW.md).
Owner accepted complete [D43 compilation](P2_05_COMPILATION_CONTRACT.md) after that
checked merge, fulfilled before code. P2-05 binds exact actual resources/source and
complete policies to one core Generation. [Completed evidence](P2_05_COMPILATION_REVIEW.md)
records both linked-account reviews, normal PR105 merge445f847, all11 exact-main
39552210853 jobs and both processed reports(Rust5507/5759; patch553/586).
Owner accepted the full [D44 Explain contract](P2_06_EXPLAIN_CONTRACT.md) after that
checked prerequisite, fulfilled before P2-06 code. Bounded pure generation-bound
Explain/simulation is current; actual Windows/Linux checks, independent reviews,
external reports and normal merge remain required. P3/P4/P6 remain planned.
P1-06's candidate passed independent Codex re-review and all11 jobs of39153071723;
actual Codecov processed both reports. See [evidence](P1_06_ACTIVATION_REVIEW.md)
for the retained first Linux failure, resolved review findings,94.01% Rust patch
coverage and explicitly failed CodeScene maintenance gates/dispositions.
Force-pushing and branch-protection changes remain unauthorized.

The repository is now public. With explicit owner approval the existing Codecov
project was activated without changing OIDC, tokens, access or plan. Re-upload
attempt 2 of [run 38865306388](https://github.com/romanpodg/DRouteX/actions/runs/38865306388)
passed; Codecov processed both backend/planning reports for the exact merge commit.
The [service report](https://app.codecov.io/github/romanpodg/droutex/commit/c43c2be0d40248f81bb01be9964ba56925b368f5)
shows Rust config coverage of 5/8 lines (62.50%); aggregate 437/443 includes auxiliary
Python tooling and is not application coverage. P1-01 subsequently verified a
meaningful new-code PR comparison below; the actual fork test remains unverified.

For P0-05 the owner selected **HTTP CONNECT over TLS** and approved isolated
compatibility/transitive-license qualification of Tokio 1.53.3, Hickory-Proto
0.26.3, Rustls 0.23.46 and Tokio-Rustls 0.26.6 using AWS-LC. Production workspace
adoption is not implied. The owner separately approved the dependency license
allowlist recorded in D14. See the
[dependency review](../operations/DEPENDENCIES.md).

[PR #3](https://github.com/romanpodg/DRouteX/pull/3) qualifies that inventory.
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
not runtime authentication proof. [PR #4](https://github.com/romanpodg/DRouteX/pull/4)
implementation head `4d79b7a95f094851bd2b73c4c924a85c7231de44` passed independent
read-only built-in Code Review against main `a77ed5adce184ffe3214452af0b1e6d83db2cd92`,
all seven jobs of [run 38911280732](https://github.com/romanpodg/DRouteX/actions/runs/38911280732)
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
live admission, policy compilation and dataset/secret/runtime gates. PR #5 initial
head passed all seven CI jobs, but independent review found three strictness/path
defects. Each was reproduced and fixed with regression tests. Both published
heads passed all seven CI jobs. Follow-up review exposed a related
tagged-enum sequence path; map-only field adapters now reject it. All 18 config
tests and local checks pass. Head `a708e34b27a82fe5b4682b1761b34569144c94c7`
passed independent read-only re-review without confirmed defects and all seven
jobs of run 38956580440. Codecov backend is 837/894, new Rust 761/815, backend patch
check successful at 93.37%. CodeScene 7991872 remains failed on the recorded
maintenance signals; no suppression or score-only rewrite. Final evidence head
`921d5dd8ab3b44da26702bdc9ca1b6765fce5508` passed independent review and all seven
jobs of run 38957381635. CodeScene 7991963 repeated the recorded failed gate.
PR #5 merged normally at `57ce7973228d1c2153601b58f82c08c9cd9302fa`; all seven main
jobs of run 38957933166 passed and both exact merge Codecov reports are processed.
Local review/merge evidence: `.setup/p1-02/autopilot-completed.json`.

P1-03 is on `storage/sqlite-persistence` from that verified merge. The owner approved
SQLx 0.9.0/Tokio 1.53.3 with defaults off and SQLite bundled/migrate/runtime-tokio,
then separately Zlib and a minimal upstream driver constraint patch to select
libsqlite3-sys 0.38.2/SQLite 3.53.2. Original native SQLite 3.51.3 is not adopted:
its FTS5 CVE fixes are absent despite the successful RustSec audit. Seven Windows
probes, Clippy, archive/native hashes and all cargo-deny categories pass on the
patched qualification profile. Head `5f0277972257edaf3d4ef91e533be70d1672f417` in
[draft PR #6](https://github.com/romanpodg/DRouteX/pull/6) then passed independent
read-only qualification review, all nine CI jobs of run 38988891185, CodeScene
and actual Codecov patch checks. That qualification is not the storage implementation
review. The qualified profile is now adopted in the new storage crate; its full
revision/index transaction, metadata/ref boundary, queue and migration backup logic
are under implementation validation. Windows check/Clippy, 32 workspace tests and
four compile-fail examples passed; actual storage Linux tests and implementation
review remain pending. The owner accepted one writer, queue 64, busy 1s, response 5s with
explicit overload/uncertain commit-time timeout, and metadata/SecretRefId links
only until P1-07. See [P1-03 evidence](P1_03_STORAGE_REVIEW.md); no plaintext verifier,
token generation/verification, live admission or deployment is authorized here.

P1-03 implementation review found an acknowledged-write loss after SQLite automatic
rollback. The writer now drains pending rollback and checks zero transaction depth
before acknowledgment, then closes an inconsistent connection without retry. The
fix passed independent re-review and actual Linux reopen regressions. Head
`20c7a3fea34126fc4d449de358bdf72bd08d88d8` also fixes an omitted storage path in the
Codecov backend flag and guards coverage against missing workspace crates.
Its independent review and all nine jobs of run 39049354119 passed. Actual service
storage patch is 638/680 (93.82%), backend 1475/1574; both patch checks succeeded.
CodeScene 7998950 stays failed with maintenance dispositions in OPEN_DECISIONS.
P1-03 is in_review until final evidence review/checks and normal PR #6 merge.

Final review on `4708e53` found that the ordinary response timeout could drop a
slow upgrade before migration completion, making every reopen repeat its backup.
The owner accepted D28: a separate task owns initialization and its lock through
completion/abandoned-result cleanup, retaining the five-second caller response
deadline. The regression was observed red then green on Windows; new Linux CI and
independent re-review of this correction are pending. PR #6 remains unmerged.

P1-03 startup correction head `6f3eea1` passed independent re-review without confirmed
defects and all nine final-head jobs (39052537622), with actual Linux 42 workspace/
15 storage tests. Codecov backend patch passed at 94% (new storage 658/700); backend
1495/1594. CodeScene 7999240 remains failed with existing maintenance dispositions.
PR #6 merged normally at `65c076d6bb00eb7615cc7e697f3f4caeed9e9d7f`; all nine main jobs
(39053065629) passed and both exact-merge Codecov reports are processed. Full history:
`.setup/p1-03/autopilot-completed.json`. P1-03 is complete; do not repeat its PR loop.

P1-04 starts on `config/track-draft-provenance` from that verified main merge.
The owner accepted one global complete-configuration draft chain, expected-parent
conflicts without automatic merge, and atomic trusted actor/UTC/correlation/revision
provenance, without configuration payload in audit. Missing legacy author/time is
not invented; RBAC remains P6. PR #7 implementation head `ab349f7` passed independent
read-only Code Review and all nine jobs in 39054676807 (Linux46/18 storage,
Windows36/8, four compile-fail examples). Exact-head Codecov processed both reports:
backend 1635/1746, backend patch143/155 (92.25%), planning100%. CodeScene7999375
failed constructor/transaction maintenance gates; source-based minor dispositions
are in P1-04-CS. Final head b63fd6e passed independent final-evidence review and all
nine jobs39055351917. PR #7 merged644348c; all nine main jobs39055754490 passed and
both exact-merge Codecov reports are processed. Full local history:
`.setup/p1-04/autopilot-completed.json`. Do not repeat completed PR loops.

P1-07 now starts from that main merge on `security/protect-secret-material`. Owner
accepted external256-bit key/rotation, conditional AWS-LC/zeroize AEAD qualification
and official packaged Windows NASM objects. Six Windows probes/24 RFC vectors and
15-artifact locked audit passed. Qualification head203c76f in draft PR #8 passed
all11 jobs39056761899, including actual Linux probes, and independent read-only/never
review after a quota-interrupted attempt. Codecov processed both reports; the6/6
patch is auxiliary planning code, not product crypto coverage. CodeScene7999538
failed excluded test/auxiliary metrics; P1-07-CS-AUX records the disposition.
The owner accepted the bounded envelope/keyring/persistence/export profile D33 on
2026-10-18. PR #8 subsequently merged36cb5e3 after independent final review and
all11 exact-head jobs39062254994. Main run39062875433 passed all11 jobs on attempt2;
attempt1 cold SQLite pool startup exceeded150ms. The fixture budget is corrected
in the next qualification change; failure/retry evidence stays local. Both exact-merge
Codecov reports processed, backend1986/2103. CodeScene7999941 stays failed with
source-based minor maintenance dispositions. Local history: `.setup/p1-07/autopilot-completed.json`.
P1-05 initial Ed25519 qualification is complete and the owner accepted D11;
D35 strict-verifier adoption and the candidate codec are reviewed/tested in cd0e2c5.
See [snapshot review](P1_05_SNAPSHOT_REVIEW.md).
See D30-D33 and the secret review.
See D29 and [the revision review](P1_04_REVISION_REVIEW.md).

## Historical owner decisions and foundation progress

The owner explicitly accepted [P0-01/CR-0002](P0_01_FOUNDATION_REVIEW.md) in this chat
on 2026-10-15 and instructed continuation through P0-02. Accepted foundation:
Profiles, personal DoH/DoT Endpoints, separate legacy/gateway group grants, opt-in
bounded source observation, private-record isolation, fixed policy layers and safe
snapshot/origin/failure constraints. Debian 13 x86_64 is the first native acceptance
target. Detailed cryptography, lease/clock/resource values, exact outbound transport
and actual operator lab retain their owning gates.

The owner also approved Rust 1.99.0/MSRV, Edition 2024, no external dependencies,
minimal common/config/policy crates and a limited Windows/Linux CI job. After a
real cargo-deny license failure the owner chose **Apache-2.0** for original DRouteX
code, rather than deferring the check. LICENSE and workspace metadata record it;
third-party licenses are preserved. D14's external notices/inventory work remains.

The pre-PR9 checkpoint had 11 completed tasks. PR9, P1-06 and all seven P2 tasks
subsequently completed their recorded merge/main gates; see the current card and
task reviews. The earlier PR9-before-P1-06 hold is satisfied and must not restart
a completed PR loop. Product networking/deployment acceptance remains separate.

On 2026-10-18 the owner reaffirmed continuation under the already renamed DRouteX:
P1-05 after a separately authorized weak-key resolution, then P1-06, then P2.
The [roadmap continuation order](DEVELOPMENT_ROADMAP.md#current-continuation-order-under-droutex)
records acceptance boundaries, including P2-07 before P2-05 and later real-service
DNS/gateway evidence. No security resolution or new dependency is authorized by
this ordering. At that historical checkpoint, read-only GitHub verification confirmed rename
PR #10 merged at `9f1aa92b50bd2efd745a96104b0d3c9694764d38` and PR #9 open/draft
at `0c3084daf16d54288791461c83f15573c191ae23`. PR9's later merge supersedes that status.

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
`5628f2ae3306bf86b22283b19003008fd3246bd7`; [PR #2](https://github.com/romanpodg/DRouteX/pull/2)
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

- [Gateway authorization](https://github.com/romanpodg/DRouteX/blob/3542426b744306ac71ca255161f5ebf65d3e1189/crates/veyra-gateway/src/gateway.rs#L118)
  reduces source admission to a boolean and unions all Profile/Device routes.
  Its [cross-profile allowance test](https://github.com/romanpodg/DRouteX/blob/3542426b744306ac71ca255161f5ebf65d3e1189/crates/veyra-policy/tests/gateway.rs#L103)
  must become a denial regression. Keep separate exact-address, expiring legacy
  and gateway grants; DNS capability never establishes transparent TCP identity.
- The old specificity/scope/action ordering and CIDR admission model do not replace
  CR-0002's custom/service/filter/default layers, within-layer priority or distinct
  Endpoint/credential/group identities. Old BLOCK and HTTPS/SVCB answer choices do
  not close D09; a shared raw-upstream cache is not the accepted scoped cache design.
- [Egress self detection](https://github.com/romanpodg/DRouteX/blob/3542426b744306ac71ca255161f5ebf65d3e1189/crates/veyra-gateway/src/egress.rs#L69)
  omits additional local public addresses behind wildcard binds;
  [origin lookup](https://github.com/romanpodg/DRouteX/blob/3542426b744306ac71ca255161f5ebf65d3e1189/crates/veyra-dns/src/upstream.rs#L55)
  collects unrelated Answer addresses without owner/CNAME-chain validation.
  Repair those boundaries and keep test-only egress bypasses out of production APIs.
- [DNS TCP tasks](https://github.com/romanpodg/DRouteX/blob/3542426b744306ac71ca255161f5ebf65d3e1189/crates/veyra-dns/src/server.rs#L95)
  have no connection cap; query permits do not bound connections and writes have
  no deadline. Network components need bounded lifecycle and tracked shutdown.
- The unimplemented old apply/deploy plan defers authenticated snapshots and
  replay protection. Follow P1-05/P1-06 instead, preserving coordinated recovery,
  increasing activation sequence and revocation through rollback.

Keep the current DRouteX workspace, Rust 1.99.0, Apache-2.0 posture, specifications,
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
crate/dependency or empty droutex-testkit was added.

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
[Actions run 38760651619](https://github.com/romanpodg/DRouteX/actions/runs/38760651619)
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
[Actions run 38756886961](https://github.com/romanpodg/DRouteX/actions/runs/38756886961)
passed Planning on Ubuntu and Windows and Codecov upload; the planning patch check
also passed. [CodeScene review 7971972](https://codescene.io/projects/86145/delta/results/7971972)
failed one argument-count rule in tools/build_doc_graph.py:add_markdown_links (9.69).
That auxiliary score-only finding is outside the owner's quality-improvement scope;
no refactor, suppression or gate change was made. Later heads need their own results.
Copilot's only review reported quota exhaustion, not successful code review.

[CodeScene integration](../quality/CODESCENE_INTEGRATION.md) and
[Codecov integration](../quality/CODECOV_INTEGRATION.md) preserve service activation
and earlier report evidence. No product CodeScene/Codecov pass is inferred from setup.

## Historical stopping conditions and continuation

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

The owner authorized the existing monitor-droutex-foundation-pr heartbeat to run
the bounded P0-04 review/fix/merge loop in the automation chat every 15 minutes,
with routine successful-run alerts disabled. It was paused after PR #1 merged.
Reuse that heartbeat for authorized roadmap continuation; preserve its cadence and
notification policy. It must not repeat the completed PR #1 loop or send routine
unchanged-state updates. Await owner answers for consequential unresolved choices.

The historical 2026-10-18 P1-05 weak-key hold was superseded by owner-approved
D35 qualification/adoption, completed implementation/final reviews and normal
PR9 merge. P1-06 then completed under D36. The original defect, qualification,
597/621 patch result and failed maintenance checks are preserved in
[P1-05 evidence](P1_05_SNAPSHOT_REVIEW.md), [P1-06 evidence](P1_06_ACTIVATION_REVIEW.md)
and OPEN_DECISIONS. Those old holds are not current commands or new approvals.
