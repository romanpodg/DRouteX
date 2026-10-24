# DRouteX project instructions

These instructions apply only to this repository. User communication is Russian;
code and technical documentation are English. Reuse existing decisions and work.

## Start here and resolve authority

Start at [DEVELOPMENT_ROADMAP](docs/planning/DEVELOPMENT_ROADMAP.md), then read the
current execution card and the affected task's contract, evidence and callers.
Load only documents and skills relevant to the requested change.

| Question | Authoritative source |
|---|---|
| Product requirements and release scope | Preserved [EN specification](DRouteX_Project_Specification_EN.md), accepted [ADR-0016](docs/architecture/adr/ADR-0016-dns-proxy-mvp-before-tunnels.md) and [ADR-0017](docs/architecture/adr/ADR-0017-profile-endpoint-dns-product-model.md); [MVP_SCOPE](docs/planning/MVP_SCOPE.md) summarizes them |
| Accepted choices and unresolved questions | [OPEN_DECISIONS](docs/planning/OPEN_DECISIONS.md) and the owning task's accepted contract; proposed ADRs do not grant approval |
| Task status, dependencies and delivery tags | [backlog.json](docs/planning/backlog.json); generated backlog/dependency/traceability Markdown is a rendering |
| Active work, authorization and open gates | Current execution card in [EXECUTION_STATUS](docs/planning/EXECUTION_STATUS.md) |
| Required checks and completion | [QUALITY_STRATEGY](docs/quality/QUALITY_STRATEGY.md); [TESTING_STRATEGY](docs/quality/TESTING_STRATEGY.md) defines network proof boundaries |
| Actual review, CI and service results | Owning task's `*_REVIEW.md`, tied to exact commits/runs; refresh external results before relying on them for a merge |
| Security reporting and control boundaries | [SECURITY](SECURITY.md) and [THREAT_MODEL](docs/security/THREAT_MODEL.md), with linked implementation evidence |
| Build commands, tools and provenance | [Rust guide](docs/agents/rust.md) and [SKILLS_AND_TOOLS](docs/environment/SKILLS_AND_TOOLS.md) |

Historical checkpoints, old holds, setup reports and local `.setup` state are
supporting evidence, not current authorization. Do not revive a completed PR loop.
If current sources disagree, verify relevant evidence and ask about an unresolved
consequential choice before dependent work; continue independent work.
Direct owner instructions take precedence over repository and skill guidance.

## Scope and standing authorization

- MVP is DNS routing and separately authorized TCP proxying on one Linux VPS.
  Tunnels, managed protocol engines and privileged network execution are post-MVP.
  Debian 13 x86_64 is the first native acceptance target; Windows is the development host.
- The owner authorized roadmap continuation, GitHub commits/pushes, PR creation and
  monitoring, and autonomous normal merges subject to the review/check policy below.
  Production deployment, new infrastructure/costs, force-pushes, protection changes
  and generic privileged host operations require separate authorization.
- On 2026-10-23 the owner explicitly authorized using the owner's authenticated
  GitHub account to comment in `romanpodg/DRouteX` PRs and request Codex review there,
  including `@codex review`, to improve code and fix bugs. This permission persists
  across sessions; no separate confirmation is needed for each comment/request.
  Public comments must exclude secrets and private account/environment data.
  Follow SECURITY.md's private channel for undisclosed vulnerability details.
- Ask about new consequential choices or unclear requirements: architecture,
  dependencies/licenses, security/access, user experience, infrastructure/costs,
  scope or development process. Explain options and recommend one. Previously
  accepted choices, routine implementation details and necessary reversible fixes
  within the authorized scope do not need repeated approval. A review finding is
  evidence to investigate, not permission to change requirements.
- Finish and merge the current implementation PR before beginning the next.
  Donor PR #2 remains reference-only: use only components/scenarios mapped in
  [the accepted reuse assessment](docs/planning/EXECUTION_STATUS.md#accepted-selective-reuse-from-pr-2).
  Do not import, merge or close that donor wholesale.
- Use Control D as the primary reference for DNS-proxy/SmartDNS concepts, onboarding,
  Profiles and Endpoints. Verify vendor behavior, distinguish it from DRouteX proposals,
  and discuss meaningful departures. Reference behavior does not override accepted scope.
- The [DRouteX rename](docs/planning/DROUTEX_RENAME.md) includes accepted incompatibility
  of old cryptographic data. Qualification permits only the reviewed dependency,
  feature and platform profile, not every capability of that library.
- Original code is Apache-2.0. Approved third-party licenses are Apache-2.0, MIT,
  ISC, BSD-3-Clause, Unicode-3.0 and Zlib; preserve texts/notices and modified-source
  marking. New dependencies or license exceptions retain their owning approval gates.
- Existing lab and automation permissions remain scoped to their recorded tasks.
  Do not infer new reminders, privileged runners or host-network changes from them.

## Architecture and security

- Control Plane stores drafts, compiles and publishes. Data Plane consumes immutable
  validated snapshots and survives CP/database loss with valid last-known-good state.
  Authenticate snapshots, reject replay, and preserve durable sequence/clock floors.
- Policy is deterministic: priority, specificity and stable tie-breaker; reject
  unresolved conflicts. DNS upstream selection does not select traffic egress.
- Keep resolver capabilities separate from public EndpointId and transparent TCP
  identity. A shared public IP grants group access, not per-device identity. Never
  select a gateway Profile from the last DNS query, headers or untrusted learning.
- Gateway needs current ingress and destination authorization. Unknown/ambiguous
  targets fail closed. Check actual resolved addresses on each resolution/retry,
  reject internal/metadata/self/loop targets, and pin checked numeric addresses at
  connect time. Recheck changing generation, revocation, lease and clock facts.
- Keep IPv4/IPv6 policy coherent. DNS-only cannot enforce ECH/QUIC routing or all
  endpoint traffic. DIRECT is relative to the decision point; strict proxy failure
  never silently becomes DIRECT. Unsupported capabilities remain unavailable.
- API/node run unprivileged. MVP needs no NET_ADMIN or custom root helper; only
  justified DNS bind permission/socket activation is allowed. Later helpers accept
  typed actions, never shell commands; enrollment remains controller-authenticated,
  single-use and short-lived. No homegrown crypto or origin HTTPS MITM.
- Redact secrets in Debug/errors/logs/API/audit, encrypt at rest with separate key
  ownership/lifecycle, and keep keys outside ordinary config/database/backups.
  Follow [KEY_LIFECYCLE](docs/security/KEY_LIFECYCLE.md) and
  [GATEWAY_BINDING](docs/security/GATEWAY_BINDING.md); later secure learning,
  authentication and origin dial gates are not passed by library qualification.

## Implementation and validation

- Before coding inspect Git status, applicable instructions, the task's prerequisites,
  acceptance criteria and relevant callers. Preserve unrelated work.
- On Windows dot-source `. ./tools/activate.ps1` before Cargo commands. Tooling is
  project-local; do not install a persistent PATH. Use the pinned compiler/MSRV.
- Prefer domain newtypes/enums, explicit ownership, small public APIs, typed library
  errors and contextual boundary errors. No uncontrolled unwrap/expect/panic on
  production input. Avoid unsafe; any exception needs justification and independent review.
- Bound async queues, buffers and concurrency; use deadlines, cancellation-safe
  operations and graceful shutdown. Do not block Tokio or hold inappropriate locks
  across await. Keep blocking persistence in an appropriate bounded context.
- Run checks proportionate to the change under QUALITY_STRATEGY, including relevant
  feature sets and real Linux integration gates. Risky logic requires negative/failure
  regressions. Instruction-only changes use documentation checks; do not run unrelated
  Cargo/network suites or audit tests/docs merely for metric scores.
- Focus coverage/Code Health on application code and critical build/CI/deployment
  scripts. Tests, fixtures, docs and auxiliary planning/setup/graph tools are not
  independent metric targets. Fix concrete relevant defects; never rewrite or add
  tests solely to raise scores. Behavior and security come first; no zero-risk claims.
- Before every commit/push inspect the exact staged files and diff; stage scoped
  paths. Keep credentials, caches, SDKs, generated graphs, logs, reports, build output
  and editor state out of Git unless a specific artifact is intended for versioning.
  `.gitignore` does not protect tracked files. No destructive Git or publication
  outside the authorized task/repository scope.
- After changes review the whole diff and report files, reasons, actual checks,
  external results and limits. Update current task evidence/status when it changes;
  update ADRs only for actual architectural decisions, preserving base specifications.

## PR review and merge policy

"Close a PR" means merge reviewed changes into main. An authorized PR may merge
only after actual Codex Code Review through the owner's linked ChatGPT account
and applicable checks on the current head. Record review/head identity. Self-review,
a request, reaction, silence or quota error is not completed Code Review.

Unresolved serious defects block merging regardless of count. Minor issues may be
deferred only when immediate intervention is unnecessary: record the finding,
impact, reason, owner/task or revisit trigger in OPEN_DECISIONS and link it from PR
evidence. Apply the external-finding disposition process in QUALITY_STRATEGY;
failed stays failed, missing evidence stays unverified. Never bypass required
checks, approvals or branch protections. Follow up on fixes with relevant tests
and actual current-head review/check evidence before merging.

## Code Review Rules

- Flag authorization inferred from IDs, SNI, headers, a previous DNS query or an
  unverified observation. DNS Profile selection and gateway permission are separate;
  derive authority from trusted current ingress and explicit grant scope.
- Flag new admission using stale generation, revoked/expired leases or reset
  sequence/clock floors. Preserve current checks at consumption/final admission;
  already admitted bounded work may finish only within the owning contract.
- Flag unchecked or re-resolved dial targets, origin-resolution loops and silent
  strict-proxy fallback to DIRECT. Check and pin actual addresses before connect;
  planned origin/connector stages remain explicitly unavailable until implemented.

## Skills and graph navigation

Use relevant project Rust skills from `.agents/skills/`, as indexed in
SKILLS_AND_TOOLS; do not load the entire bundle or install an overlapping system.
`setup-rust-ci` and `setup-rust-pre-commit` are user-invoked. Ponytail supplements
contracts/tests; hooks are not automatically trusted. No paid/API backend by default.

When the user types `/graphify`, read its installed skill first. For codebase
questions, run `graphify query "<question>"` first when `graphify-out/graph.json`
exists, even if generated files are dirty. Use path/explain for focused relations
and the wiki index for broad navigation; read GRAPH_REPORT only if needed.
Skip only for stale/incorrect graph tasks or an explicit user instruction. Confirm
sources/tests, distinguish EXTRACTED from INFERRED and planned from implemented.
After code changes run `graphify update .` (AST-only, no API cost); keep graphs local.

## Windows native computer use routing

Native Windows UI MUST use `mcp__node_repl__js` with
`const { sky } = await import("@oai/sky");`. Test `await sky.list_apps()` before
claiming native control is unavailable. `mcp__cua_repl` is browser-only here;
use it for browser interaction and the native backend for Windows apps.
