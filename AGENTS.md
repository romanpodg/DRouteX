# DRouteX project instructions

P3-01/PR109 remains current. Candidate5117e5f passed all11 CI39761170349 jobs:
225 Windows/257 Linux tests per feature profile, no ignored/filtered cases;
Codecov Rust7462/7857, patch770/841. Evidence/test head9e7ff15 also passed all11
CI39763152705 jobs. Linked-account read-only/never implementation review
01a16317-f124-7290-aafa-7ca351381e18 found a confirmed wildcard UDP wrong-source
address defect. Owner now accepted D49: explicit UDP IPv4/IPv6 addresses only;
refuse0.0.0.0/:: before workers, preserve schema/TCP wildcard, and qualify later
UDP destination-aware I/O separately. The small correction and negative/positive
regressions received clean linked-account corrective review
01a1640b-eb5c-77c2-b4d3-21b773595406 on d00c054; actual CI39797242225 and final
evidence review/head/normal merge/main gates remain open. Corrective logs passed
229 Windows/261 Linux tests per feature profile, none ignored/filtered; Linux
Workspace metadata still says in_progress although its steps/log completed
successfully, so the all11-job status gate is unconfirmed. No CI exception used.
Codecov processed both
corrective reports: Rust7489/7860, patch797/844. CodeScene8081708 still failed all3
gates(runtime7.61, wire7.91, auxiliary9.15/8.75); concrete minor dispositions
remain recorded without suppression. Actual
local native DNS29passed/1IPv6failed outside sandbox; new D49 cases passed and
mandatory clean-CI IPv6 remains enabled. CodeScene8076431 failed all3 gates;
three concrete minor source/auxiliary maintenance dispositions are recorded
without suppression, separately from the wildcard correction gate. See
P3_01_LISTENER_REVIEW.md/current local autopilot state before proceeding.

Current gate2026-10-22: P0/P1/P2 and P4-01 are complete. PR107 merged normally at
02b6695928a64c37de5e073a4bdb5162f1928c54 after linked-account read-only/never
implementation01a1602b-028d-7192-b217-97743133bb9e and final
01a162ab-81e6-7102-bc79-696105883a15 reviews, no confirmed defect. The initial
quota-interrupted final attempt was not counted. All11 final39672366016 and
exact-main39747433154 jobs passed; both Codecov sessions processed(Rust6689/7013;
patch717/759). CodeScene8066731/8066873 remain failed with identical concrete
minor source/excluded auxiliary dispositions in P4_01_BINDING_REVIEW.md and
OPEN_DECISIONS, no suppression/bypass. D46 manual-first v2 current authorization,
conservative clock floor/quotas are implemented; v1 remains association-only.
P2-06 simulation stays pure; automatic secure learning remains unavailable until
P3-07/P6, actual final origin checks/pinning remain P4-02.
Owner accepted full D47 conditionally on checked PR107; that prerequisite passed
before qualification code. See P3_01_LISTENER_CONTRACT.md.
P3-01 runtime is current after completed D47 qualification PR108. Qualifier candidate
083d83d and final62f2f27 passed all11 CI39750969047/39752553613 jobs, all9 tests
in each Windows/Linux feature profile and all six deny bundles. Linked-account
read-only/never implementation01a162d2-a03c-7250-8a6c-1e2545d019ca and final
01a162da-85ac-73d0-85fd-7d35bee10e7a reviews completed without confirmed defect.
Normal merged36d7b2c5233822113d1085ac85dc9dd422fa401 passed all11 exact-main
39753424789 jobs and both Codecov sessions(Rust6689/7013). All94 archive checksums
match previously qualified identities; all28 production lock additions match
that exact qualifier graph, no old registry identity removed. CodeScene8074936/
8075138 failed only identical excluded auxiliary hotspots, explicitly recorded
without suppression. Accepted runtime code is in progress, not completed or
merged: real seven-crate workspace, bounded UDP/TCP and D46 current source gate.
Actual runtime platform/review/coverage/final-head/normal merge/main gates remain
open. Owner accepted D48 timeout clarification:1500ms processor/remaining500ms reply
inside the original2s total; expired/lost admission never gets a fallback.
Local development-host IPv6 UDP/TCP fails independently in Python; clean CI
passed mandatory IPv6 tests, and no host networking changes were made.
D47 includes UDP1232/3x cap, bounded TCP/ingress/buffers/deadlines, actual D46 per-query
admission, local clock maintenance and graceful shutdown. Until later upstream/
policy handlers, valid admitted queries return SERVFAIL; no fabricated DNS success.
No public service deployment, privileged operation, TLS/recursion/cache/learning or
management endpoint is authorized here. Later implementation stays planned. No
owner CI-status exception was used. D42 WebPKI0.103.16 remains qualified only in
the existing TLS qualifier, resolving RUSTSEC-2026-0382.

## Authority and current scope

Owner-authorized DRouteX rename (2026-10-18) is a separate maintenance change;
see docs/planning/DROUTEX_RENAME.md. It includes accepted incompatibility of
old cryptographic data, but does not resolve P1-05 or authorize signature adoption.

DRouteX is a self-hosted DNS, selective-routing and multi-protocol network platform.
Read `DRouteX_Project_Specification_EN.md` with the accepted scope amendment
`docs/architecture/adr/ADR-0016-dns-proxy-mvp-before-tunnels.md` as product authority.
The EN/RU base specifications are preserved. MVP is DNS routing and authorized TCP
proxying only; tunnels, managed protocol engines and privileged network execution
are post-MVP. The user authorized beginning MVP roadmap work on 2026-10-15.
P0-01/CR-0002, P0-02 and P0-03 are complete. On 2026-10-15 the owner authorized the next
step, P0-03, and approved the existing WSL Ubuntu lab with Python stdlib/OpenSSL,
isolated namespaces and lab iproute2/nftables/tcpdump. WSL fixture/cleanup checks
passed. On 2026-10-16 the owner supplied a dedicated Debian 13 KVM VPS and approved
tcpdump/libpcap installation and the isolated P0-03 run. All 41 protocol observations
and four cleanup modes passed there with unchanged host networking. This proves
fixture readiness, not DRouteX runtime or systemd acceptance.
On 2026-10-16 the owner authorized publishing P0-03 to PR #1 and proceeding through
P0-04 CI only. Private fork workflows require owner approval and receive no secrets
or write tokens; the owner deferred a real fork PR test.
on 2026-10-16 the owner accepted P0-04 with that test and unavailable Codecov
comparison/web-report evidence explicitly deferred, not passed. PR #1 was reviewed,
fixed and merged as c43c2be0d40248f81bb01be9964ba56925b368f5. The owner then
authorized continuing the plan. P0-05/PR #3 completed qualification and merged as
a77ed5adce184ffe3214452af0b1e6d83db2cd92 after two independent Code Reviews and
seven successful final-head/main CI jobs. P1-01/PR #4 merged at
cd2a9c011e4950d80458cfaa72baddd1ac4243c2 after independent implementation/final-evidence
Code Reviews, all seven final-head/main CI jobs, CodeScene and an actual Codecov
backend patch comparison (71/71 new Rust lines). P1-02/PR #5 merged normally at
57ce7973228d1c2153601b58f82c08c9cd9302fa after implementation/final-evidence reviews
and all seven final-head/main CI jobs. Codecov processed both exact merge reports;
the failed CodeScene maintenance gate stays recorded in P1-02-CS. P1-03/PR #6 merged
normally at 65c076d6bb00eb7615cc7e697f3f4caeed9e9d7f after fixing automatic-rollback
and startup-cancellation defects, independent re-review and all nine final-head/main
jobs. Both exact-merge Codecov reports are processed; maintenance failures remain
recorded. P1-04/PR #7 merged normally at644348cfe53a4b76f8a921cf3e4886b2486e0cb8
after independent implementation/final reviews and all nine head/main jobs; exact-merge
Codecov processed. Global draft provenance does not invent legacy authors/times.
P1-07/PR #8 merged normally at36cb5e38e215b1f53a10a3cde1dccbda9c25b5c2.
Independent final read-only/never review and all11 head/main jobs completed; main
attempt1 cold-pool fixture timeout is retained and attempt2 passed. Exact-merge
Codecov processed both reports. P1-05 is current: owner approved Ed25519 qualification
in pinned aws-lc-rs1.18.2 with separate CP signing private key and DP public trust keys.
Initial Ed25519 qualification head0c3084d passed all11 jobs and independent review.
Owner accepted the bounded D11 codec/replay/rotation profile and D35 isolated
ed25519-dalek3.0.0 qualification (defaults off) with conditional adoption of weak-key
import refusal and strict verification, retaining AWS-LC signing. Qualifier3bd7db9
passed all11 Windows/Linux CI jobs and independent read-only/never review. The
weak-key defect is fixed in implementation cd0e2c5, which passed independent review
and all11 jobs39147507106. Actual Codecov Rust patch is597/621; maintenance failures
are explicitly recorded, not passed. P1-05 candidate acceptance is verified; PR9
finalization still requires current-head checks and final review before normal merge.
P1-06 durable activation/recovery is not implemented. Do not begin the next
implementation PR before PR9 merges; future technical choices retain their gates.
See P1-05-WEAK-TRUST and .setup/roadmap-autopilot-state.json. Historical P1-07 decisions: owner accepted external256-bit protected-file key/rotation,
conditional AWS-LC1.18.2/zeroize1.9.1 AES-256-GCM-SIV qualification and official
packaged NASM objects for Windows development (Linux source build). No production
crypto adoption before successful qualification. Head203c76f passed actual Windows/Linux
qualification, all11 CI jobs and independent read-only/never review. Envelope/keyring/
persistence/export profile D33 is owner-accepted and implemented in reviewed PR8. Snapshot/publication P1-05/06,
verification P3-07 and RBAC P6 remain gated; see D26-D33 and task reviews.
The owner
approved the strict JSON/YAML/TOML profile and qualified Serde 1.0.229, serde_json
1.0.154, toml 1.1.8+spec-1.1.0, serde-saphyr 2.0.0 and serde_path_to_error 0.1.20
for the config boundary. See D24/D25 and the P1-02 review for exact limits/evidence. The owner
selected UUIDv4 public resource IDs and qualification of uuid 1.28.0; its official
archive/license/checksum, Windows vector tests and cargo-deny passed. Production
use is limited to the std parse/format profile for domain identities; generators,
token verification, crypto/lifetime/size policy and actual network admission retain
P1-07/P3-07/P4-01 gates. The owner selected HTTP
CONNECT over TLS as the first protected outbound and approved isolated compatibility
and transitive-license qualification of Tokio 1.53.3, Hickory-Proto 0.26.3,
Rustls 0.23.46 and Tokio-Rustls 0.26.6 with AWS-LC. This is not production adoption
or approval of new license exceptions. The owner separately approved Apache-2.0,
MIT, ISC, BSD-3-Clause and Unicode-3.0 for third-party dependencies, and separately
Zlib on 2026-10-17, preserving texts/copyrights/notices and modified-source marking;
original DRouteX code remains Apache-2.0.
The repository is public. The existing
Codecov repository was activated with owner approval, preserving OIDC and access;
the exact main commit now has processed backend/planning reports. Later technical
choices retain their gates. No production-root or
untrusted privileged PR runner is authorized.
The user authorized initial GitHub commits/pushes, PR creation and activity monitoring
on 2026-10-15. Repository: romanpodg/DRouteX; current task and execution
boundary are recorded in `docs/planning/EXECUTION_STATUS.md`. On 2026-10-16 the
owner also authorized autonomous merging after Code Review and checks under the
PR policy below. Force-pushing and changing branch protections remain unauthorized.
The owner selected option A for PR #2 on 2026-10-15: keep PR #1 as the foundation,
reuse only reviewed components/test scenarios at their owning roadmap gates, and
preserve CR-0002. See EXECUTION_STATUS.md for donor provenance and exclusions.
Public remote access from changing IPs without VPN is required. The user accepted
network/group access for legacy DNS and ordinary transparent TCP.
The user authorized Control D research and plan adaptation; accepted CR-0002 in
`docs/architecture/DNS_PRODUCT_MODEL.md` separates reusable Profiles, personal
DoH/DoT Endpoints, filter/service revisions, private records and gateway group grants.
Manual IP updates are bootstrap/fallback, not the only proposed DNS access path.
Keep resolver capabilities separate from public EndpointId and transparent TCP
identity; never switch gateway Profile based on the last DNS query or learn grants
from untrusted input. Exact contracts are in `docs/planning/P0_01_FOUNDATION_REVIEW.md`.
CR-0002/ADR-0017 and P0-01 foundation constraints are accepted. Detailed crypto,
limits and outbound choices remain at their owning gates; other technical ADRs
are not blanket-approved. Debian 13 x86_64 is the accepted first native test target.
P0-02 uses owner-approved Rust 1.99.0/MSRV, Edition 2024, no external dependencies
and a limited Windows/Linux workspace CI job. The owner selected Apache-2.0 for
original DRouteX code; third-party licenses are preserved. The owner authorized the
existing P0-04 heartbeat in its automation chat, with routine successful-run alerts disabled;
this does not authorize unrelated reminders or automations.
User communication is Russian; code and
technical documentation are English.

## Project collaboration and quality policy

These rules apply only inside the DRouteX repository. Do not propagate them to
global agent instructions or other projects.

- "Close a PR" means merge its reviewed changes into main, as explicitly clarified
  by the owner. The agent may merge an authorized PR after actual Codex Code Review
  through the owner's linked ChatGPT account and applicable checks on the current
  head. A self-review, requested review, reaction or quota error is not completed
  Code Review. Never bypass required checks, approvals or branch protections.
- Unresolved serious defects block merging. Assess severity, combined impact and
  the number of remaining issues; no count makes a serious defect acceptable.
  Minor issues may be deferred only when immediate intervention is unnecessary:
  record the finding, impact, reason for deferral and owning task or revisit trigger
  in the existing planning register, and link it from the PR evidence. Owner-approved
  missing evidence stays explicitly unverified, never recorded as a passed test.
- Finish and merge the current implementation PR before beginning the next PR.
  Existing donor PR #2 remains reference-only under the accepted reuse strategy;
  this rule does not authorize importing, merging or closing that donor PR.
- Ask the owner immediately when a requirement, condition, severity/deferral decision
  or other consequential choice is unclear; do not invent missing requirements.
  Continue independent authorized work while awaiting the answer.
- Focus Code Health and coverage work on application code and critical build, CI
  and deployment scripts. The owner explicitly selected this scope. Tests,
  documentation, fixtures and auxiliary planning/setup tools are not independent
  targets for quality scoring, coverage targets or metric-driven refactoring.
  Fix concrete functional defects when relevant; do not improve every file merely
  because an external service flags it. A score-only finding in excluded files
  does not authorize a cleanup campaign.
- Preserve behavior and security ahead of metrics. Do not split, rewrite or add
  code/tests solely to raise CodeScene or Codecov numbers. Proceed with a quality
  change only when its benefit and safety are supported by source inspection and
  proportionate regression evidence. If that confidence is missing, explain the
  uncertainty and ask the owner before proceeding. Never claim zero regression risk.
  Run relevant tests to validate actual changes; do not start separate audits of
  tests or documentation, or run unrelated checks for an instruction-only edit.
- Before every commit/push, inspect the exact staged files and diff. Keep caches,
  local SDKs/tools, generated graphs, temporary files, logs, reports, build output,
  credentials and editor state out of GitHub unless a specific artifact is explicitly
  intended for version control. Use scoped staging; .gitignore does not protect
  already tracked files. Preserve unrelated work.
- Ask the owner in chat about unclear requirements and consequential choices before
  choosing an alternative or implementing dependent work. This includes architecture,
  dependencies, security/access behavior, user experience, infrastructure/costs,
  scope and changes to the development process. Explain the options and tradeoffs
  briefly and offer a recommendation; do not silently treat it as accepted. Continue
  independent authorized work while waiting and reuse decisions already given.
- Use Control D as the primary reference for DNS-proxy and SmartDNS concepts,
  onboarding, Profiles, DNS Endpoints and selective routing. DRouteX is inspired
  by and intended to develop that concept further. Verify relevant vendor behavior,
  distinguish documented facts from DRouteX proposals, and discuss meaningful
  departures or extensions with the owner. Reference behavior never overrides
  DRouteX's accepted scope, security boundaries or P0-01 approval gate.

## Windows native computer use routing (preserved user instructions)

Native Windows UI interaction MUST use `mcp__node_repl__js` with
`const { sky } = await import("@oai/sky");`. Use `await sky.list_apps()` before
claiming native control is unavailable. Never use `mcp__cua_repl` for native apps;
it is browser-only here. When both surfaces are needed, use the native backend
for Windows and `mcp__cua_repl` for browser interaction.

## Architecture and security

- Control Plane stores drafts, compiles and publishes; Data Plane consumes immutable
  validated snapshots and survives Control Plane/database loss with last-known-good.
- Policy is deterministic: explicit priority, specificity, stable tie-breaker;
  reject unresolved conflicts. DNS upstream selection does not select traffic egress.
- DNS-only cannot guarantee ECH/QUIC routing or per-device identity behind NAT.
  DIRECT is relative to the decision point. Keep IPv4/IPv6 policy consistent.
- Gateway requires authorized destination binding; unknown/ambiguous targets fail
  closed. Check resolved addresses on every resolution, pin checked addresses at
  connect time, prohibit internal/metadata/self targets and origin-resolution loops.
- Protocols and engines differ. Unknown capabilities are unavailable. Pin and verify
  external engines; isolate failures. Do not implement cryptography or HTTPS MITM.
- API/node run unprivileged; MVP needs no NET_ADMIN or custom root networking helper.
  Narrow DNS bind capability/socket activation is allowed. Post-MVP helper accepts
  typed actions, never shell commands. Later remote enrollment is single-use,
  short-lived and controller-authenticated.
- Redact secrets everywhere, encrypt at rest with a separate key lifecycle, authenticate
  snapshots and prevent replay. Strict proxy failure never silently becomes DIRECT.

## Skills and navigation

Project Rust skills live in `.agents/skills/`; the inventory, upstream revisions and
invocation rules are in `docs/environment/SKILLS_AND_TOOLS.md`. Use relevant skills,
not the entire bundle. `setup-rust-ci` and `setup-rust-pre-commit` are user-invoked.
Ponytail plugin: `$ponytail:ponytail`, `$ponytail:ponytail-review`,
`$ponytail:ponytail-audit`, `$ponytail:ponytail-debt`. Reuse existing code, choose
the simplest complete safe solution, review concrete defects and risky logic.
Ponytail never overrides required security, compatibility or tests. Hooks are not
automatically trusted. Do not install the overlapping alternative Rust rule system.
Graphify is supplementary navigation evidence. Read sources and run tests; distinguish
EXTRACTED from INFERRED relationships and planned architecture from implemented code.
No paid/API backend is enabled by default; local graph artifacts stay untracked.

## Engineering workflow and completion

Before coding: inspect Git status and applicable instructions, read the task and
spec sections, verify prerequisites and acceptance criteria, inspect relevant callers.
On this Windows checkout, dot-source `. ./tools/activate.ps1` before Cargo commands;
Rust and portable SDK tooling are project-local and no persistent PATH was installed.
Use domain newtypes/enums, explicit ownership, typed library errors and contextual
boundary errors. No uncontrolled unwrap/expect/panic on production input paths.
Bound async queues/concurrency; apply deadlines, cancellation-safe operations and
graceful shutdown. Do not block Tokio or hold inappropriate locks across await.
Avoid unsafe; justify and independently review any exception. Keep public APIs small.

For affected application code and critical operational scripts, run applicable
fmt/check/clippy/test/deny checks in `docs/quality/QUALITY_STRATEGY.md`, including
feature combinations and real Linux integration gates. Negative and failure
tests are mandatory for risky logic. CodeScene/Codecov status is external and unverified
until an actual run succeeds. Never invent coverage, CI, network or health results.

After a task: review the whole change; report changed files, reasoning, actual checks,
coverage/health when available and risks; update backlog and ADRs. Definition of Done
is in the quality strategy. No dummy crates, destructive Git, unsolicited pushes,
production-root test runners, credentials in Git, or silent spec changes.

Start at `docs/planning/DEVELOPMENT_ROADMAP.md` and follow its gates.

## graphify

This project has a knowledge graph at graphify-out/ with god nodes, community structure, and cross-file relationships.

When the user types `/graphify`, use the installed graphify skill or instructions before doing anything else.

Rules:
- For codebase questions, first run `graphify query "<question>"` when graphify-out/graph.json exists. Use `graphify path "<A>" "<B>"` for relationships and `graphify explain "<concept>"` for focused concepts. These return a scoped subgraph, usually much smaller than GRAPH_REPORT.md or raw grep output.
- Dirty graphify-out/ files are expected after hooks or incremental updates; dirty graph files are not a reason to skip graphify. Only skip graphify if the task is about stale or incorrect graph output, or the user explicitly says not to use it.
- If graphify-out/wiki/index.md exists, use it for broad navigation instead of raw source browsing.
- Read graphify-out/GRAPH_REPORT.md only for broad architecture review or when query/path/explain do not surface enough context.
- After modifying code, run `graphify update .` to keep the graph current (AST-only, no API cost).
