# Development environment report

Inspection/setup date: **2026-10-15**, user timezone Asia/Omsk.
Historical initial-setup snapshot below. P0-02 subsequently added the Rust workspace;
P0-03 now has owner-approved WSL and native Debian 13 labs with passing fixture
and cleanup evidence (native run: 2026-10-16). No
DRouteX production service/frontend exists. Current state is recorded in
[execution status](../planning/EXECUTION_STATUS.md) and
[network harness review](../planning/P0_03_NETWORK_HARNESS.md).
Current scope revision: user accepted DNS/proxy MVP first (CR-0001/ADR-0016); tunnels,
managed engines and custom privileged networking helper are post-MVP. Environment setup
results below are unchanged by this planning-only revision.

## Host, repository and capabilities

| Item | Verified result |
|---|---|
| Primary host | Native Windows, NT 10.0 build 26300, x86_64; not a Linux shell |
| Shell | PowerShell Core 7.6.5 |
| Git | 2.52.0.windows.1; initially no repository; local main initialized without commits/remotes/push |
| Existing source | Only EN/RU specification documents, both preserved byte-for-byte |
| Codex | Desktop environment; bundled CLI 0.160.0 with plugin marketplace/add, MCP and app-server registry APIs |
| Codex skill discovery | Actual app-server skills/list forceReload, no model/thread turn; 21 required entrypoints found, zero registry errors |
| GitHub | User romanpodg authenticated via keyring outside sandbox; no DRouteX remote selected/created |
| Node/npm/pnpm | Node 24.16.0, npm 11.13.0, pnpm 11.9.0; existing installations reused |
| Python/uv | Managed Python 3.12.13 at python3.12.exe; uv 0.11.20; plain python.exe is nonworking WindowsApps alias |
| WSL | Existing Ubuntu and docker-desktop; Ubuntu kernel 6.18.33.2-microsoft-standard-WSL2 x86_64 verified |
| WSL Rust | No cargo/rustc found in Ubuntu during inspection; primary Rust installed natively, Linux harness still future |
| Docker | CLI 29.7.2 exists; daemon unavailable in inspection, no install/start required |
| Existing native compiler | Visual Studio 18 Community, MSVC 14.51.36231; existing toolchain reused |
| Windows SDK | Missing system SDK libraries caused kernel32.lib link error; official portable Microsoft CPP/CPP.x64 10.0.28000.2705 added locally |
| Existing development/CI | No initial application, tests, manifests, AGENTS.md or CI config; proposed docs/examples added |
| Native UI | User routing instructions preserved in AGENTS.md; no native UI task performed or availability claim made |

Sandbox observations differed from the actual user environment: first GitHub/WSL/uv/plugin
reads appeared unavailable under the isolated account. Read-only user-context checks confirmed
valid GitHub authentication, existing WSL/Graphify and installed Ponytail. Initial failures are
not recorded as absent user installations.

CLI MCP list in actual user context: node_repl and cua_repl enabled; bundled code-review and
codex_app entries disabled in standalone CLI. Desktop exposes its own app/code-review/browser/
Node tools and connector metadata. No CodeScene or Graphify MCP connection was configured.
CLI listing, desktop tool exposure and successful external authentication are different evidence.

Existing global skills (unmodified): architecture-contracts, codex-budget-mode,
provider-development, security-boundaries, stage-workflow, validation-engineering,
system skills and hatch-pet. Musicbot-specific architecture/provider guidance is not adopted
as DRouteX architecture. Existing uv free-claude-code tooling was neither used nor modified.

## Installed/reused Rust and analysis tools

| Tool | Version | Scope / proof |
|---|---|---|
| Rust/Cargo | 1.99.0, host x86_64-pc-windows-msvc | .tools/rustup and .tools/cargo; exact toolchain pinned |
| rustup | 1.29.1 | Official rustup-init SHA-256 checked; no persistent PATH edit |
| rustfmt | 1.10.0-stable | Installed component and version verified |
| Clippy | 0.1.99 | Installed component and version verified |
| rust-analyzer | 1.99.0 | Installed component and version verified |
| LLVM tools | LLVM 23.1.1 toolchain component | Installed component verified; coverage production run not performed |
| cargo-nextest | 0.9.148 | Official Windows archive digest and binary hash verified; version checked |
| cargo-llvm-cov | 0.9.1 | Official digest/hash verified; workspace/LCOV/output-path help checked |
| cargo-deny | 0.20.2 | Official digest/hash verified; version checked |
| cargo-audit | 0.22.2 | Official digest/hash verified; version checked |
| Graphify graphifyy | 0.9.54 | Existing isolated uv receipt/package metadata/version; project integration added |
| CodeScene cs | 1.0.53 | Local portable official archive; version/help/auth checked; OAuth not signed in |
| actionlint | 1.7.12 | Official archive digest verified; both workflow proposals passed |
| PyYAML | 6.0.3 | Isolated .setup/yaml-venv only, for local proposal validation; not runtime dependency |

Only the exact 1.99.0 Rust toolchain is retained; temporary stable alias installation removed.
No nightly/Miri/fuzz/udeps/machete/Criterion/loom was installed speculatively.
All runtime crates/engines remain proposed, not installed as application dependencies.

Latest stable candidate crate metadata was read from crates.io:
Tokio 1.53.3 (MSRV 1.71), Hickory server/resolver 0.26.3 (1.88), Axum 0.8.9 (1.80),
rustls 0.23.46 (1.71), SQLx 0.9.0 (1.94.0), Serde 1.0.229 (1.56), tracing 0.1.44 (1.65.0).
Rust 1.99 satisfies individual advertised minima. **Joint dependency resolution, selected
features and Linux engine compatibility are unverified until P0-02/P0-05.** No dummy crate
or promise of arbitrary dependency compatibility is used.

## Setup evidence, preservation and reproducibility

- Official sources were read before installing Ponytail/Graphify/Rust guidance; third-party
  executable hooks were inspected, not automatically trusted.
- Rust installer and four Cargo tools used upstream integrity hashes. Portable SDK archives
  matched official NuGet catalog SHA-512 hashes. CodeScene had an official HTTPS download and
  locally recorded SHA-256; no unprovided publisher signature/checksum verification is claimed.
- Installed Rust skill resources were compared byte-for-byte with pinned upstream: 16 skills,
  52 files. App-server discovery returned every expected skill and four Ponytail capabilities.
- Edition 2024 standalone temporary Rust program compiled, linked with MSVC/SDK and ran its
  assertion successfully. This proves developer compilation, not DRouteX behavior.
- PowerShell activation run twice without duplicate tool PATH entries. It only changes the
  current process environment; the user PATH, execution policy and installed VS were unchanged.
- EN SHA-256: 78ca71c61755152aa4e369d9b927a8aa73cc6afc8c11bc94c3ceb9c58dc275d9.
- RU SHA-256: 25fb27b433fc6b2e72ccf0392a731721be0505e562e539bb93cb8fb20debee46.

Machine-readable installation versions/URLs/hashes:
[SETUP_INVENTORY.json](SETUP_INVENTORY.json).
[Planning verification](planning-verification.json) checks source hashes, all numbered
specification headings, task contracts and release boundaries, mapping records, acyclic
prerequisites, ADR structures/statuses and local links. The revised baseline has 87 tasks
(60 MVP), 187 trace records, 15 proposed ADRs plus accepted ADR-0016, and five negative
validator cases including an injected MVP dependency on post-MVP tunnels.
[Quality proposal verification](quality-proposal-verification.json) checks three YAML files
and two deliberately unsafe authorization configurations. actionlint validates Actions syntax.

Graphify performs native local AST extraction of planning tools, plus a deterministic importer
of explicit document headings/citations, source requirement mappings and proposed task edges.
No paid/cloud/API semantic backend is used. Navigation/query/path/explain and clustering were
tested locally. The graph is a **planning/citation baseline**, not an implemented architecture
or deep semantic interpretation. EXTRACTED means an explicit document relationship; planned
prerequisites never prove implemented code coupling. Derived JSON/report/HTML remain ignored.
Rebuild: `graphify extract . --code-only --no-cluster --max-workers 1`, then run
`tools/build_doc_graph.py` with the isolated Graphify Python (use --refresh-planning
for intentional document-heading replacements, which checks their importer provenance) and
`graphify cluster-only . --no-label`. Refresh the ast-baseline.json from the new native
extraction when planning-tool code changes; source documents require the importer again.

## Resolved failures and remaining limitations

| Observation | Resolution / next concrete action |
|---|---|
| Sandbox Git schannel credential error | Official clones downloaded in authorized user context; no TLS bypass |
| WindowsApps python alias failure | Existing managed python3.12.exe reused |
| Initial Rust/link.exe not found | Local Rust installed; existing VS Developer Shell activated |
| kernel32.lib absent even outside sandbox | Portable Microsoft SDK headers/libraries installed and verified; runtime smoke now passes |
| Skills initially only copied from local checkout | Reinstalled same exact selected content through pinned GitHub ref; reproducible skills-lock.json |
| Plugin skill names initially missed | Namespace-aware registry probe now verifies ponytail:skill names |
| Graphify ambiguous duplicate task labels | Canonical JSON task nodes retained; duplicate MD task-heading nodes omitted by importer |
| Docker daemon unavailable | No Docker dependency for native plan; verify/start approved Docker lab only when Compose tests needed |
| CodeScene sign-in missing | User runs cs auth login, selects repository and establishes actual baseline; no score claimed |
| Codecov not configured | Select repo/access/OIDC, promote reviewed proposal after actual workspaces and execute first verified PR |
| Public Codecov YAML validator transfer rejected | Automatic approval review rejected external file transfer; no upload attempted afterward; local checks used |
| Cargo/frontend application checks unavailable | Run after approved P0-02/P7-01 create real workspaces; no tests/coverage invented |
| Real Linux DNS/proxy/WAN acceptance absent | Select D03/D07/D19 lab/VPS/proxy; execute MVP gates; Windows smoke cannot replace them. D13 engines/clients are post-MVP |

Preparation supports starting **approved foundation work**. Networking/release acceptance and
external service integration are still gated by their recorded evidence requirements.
