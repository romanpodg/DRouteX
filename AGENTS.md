# Routira project instructions

## Authority and current scope

Routira is a self-hosted DNS, selective-routing and multi-protocol network platform.
Read `Routira_Project_Specification_EN.md` with the accepted scope amendment
`docs/architecture/adr/ADR-0016-dns-proxy-mvp-before-tunnels.md` as product authority.
The EN/RU base specifications are preserved. MVP is DNS routing and authorized TCP
proxying only; tunnels, managed protocol engines and privileged network execution
are post-MVP. The user authorized beginning MVP roadmap work on 2026-10-15.
The owner explicitly accepted P0-01/CR-0002 on 2026-10-15 and authorized work
through P0-02 only. P0-02 is completed with Windows/Linux CI and local supply-chain evidence.
Stop after P0-02; later tasks retain their prerequisites and authorization gates.
The user authorized initial GitHub commits/pushes, PR creation and activity monitoring
on 2026-10-15. Repository: romanpodg/Routira; current review PR #1 and execution
boundary are recorded in `docs/planning/EXECUTION_STATUS.md`. This does not authorize
merging, force-pushing, changing branch protections or starting P0-02 before review.
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
original Routira code; third-party licenses are preserved. No new reminder is authorized.
User communication is Russian; code and
technical documentation are English.

## Project collaboration and quality policy

These rules apply only inside the Routira repository. Do not propagate them to
global agent instructions or other projects.

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
  onboarding, Profiles, DNS Endpoints and selective routing. Routira is inspired
  by and intended to develop that concept further. Verify relevant vendor behavior,
  distinguish documented facts from Routira proposals, and discuss meaningful
  departures or extensions with the owner. Reference behavior never overrides
  Routira's accepted scope, security boundaries or P0-01 approval gate.

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
