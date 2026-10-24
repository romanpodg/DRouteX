# Skills and developer tool usage

Installation/provenance snapshot: 2026-10-15. Product spec and user requirements
outrank generic skill preferences. Historical availability/setup observations below
are not current implementation status; use [EXECUTION_STATUS](../planning/EXECUTION_STATUS.md)
and the [Rust guide](../agents/rust.md) for active work and commands.

## Installed capabilities and invocation

Ponytail 5.2.0: official user-level Codex plugin, discovered through actual plugin list and
app-server registry. [Upstream installation](https://github.com/DietrichGebert/ponytail/blob/main/INSTALL.md)
was inspected before using marketplace/add. On Codex, invoke
`$ponytail:ponytail`, `$ponytail:ponytail-review`,
`$ponytail:ponytail-audit`, `$ponytail:ponytail-debt`.
Review concrete change/callers; audit full repository only when useful; debt collects deliberately
deferred shortcut comments. No application/debt audit result is invented for this planning tree.

Hooks inspected: SessionStart, UserPromptSubmit and SubagentStart. Codex paths maintain
plugin-data mode flags, read rule/config files, run bounded local git/file mapping and emit
instruction context; optional default-mode changes write mode configuration. No hook trust
was enabled. Existing user configuration/other plugins were preserved. Skills are usable on
the next Codex turn; restarting the desktop may be needed to refresh the plugin catalog.

Graphify 0.9.54: existing isolated official package **graphifyy**, CLI **graphify**.
`graphify install --project --platform codex` added .codex/skills/graphify with references
and appended AGENTS.md. Its generated intentional no-op PreToolUse hook was inspected and
removed because AGENTS/skill navigation provides useful behavior without an extra executable hook.
No MCP connection or cloud trial was enabled. Invoke `$graphify` in Codex. Use query/path/
explain for scoped navigation and god-nodes for planning hub review; confirm sources/tests.
Full semantic docs extraction needs the assistant/backend; this session instead imports only
explicit local document relationships. Do not claim that a sparse AST-only graph understands Markdown.

Selected Rust guidance: one coherent [rewrite-rs/skills](https://github.com/rewrite-rs/skills)
system pinned to **ae0fd62485d071437ae5e6c4036c986b0dd122a0**.
16 project skills plus all 52 resources installed by skills CLI 1.7.2 in copy mode.
Skills-lock source is GitHub/pinned ref, not an ephemeral local checkout.
Actual Codex registry found all; each entry point is linked below.

The Rust skill bundle is BSD-3-Clause: original [license](../../.agents/skills/LICENSE)
is retained. Graphify package metadata declares Apache-2.0 and includes an MIT component:
its original [LICENSE](../../.codex/skills/graphify/LICENSE),
[LICENSE-MIT](../../.codex/skills/graphify/LICENSE-MIT) and
[NOTICE](../../.codex/skills/graphify/NOTICE) are preserved alongside the copied skill.
Ponytail's user-level installed plugin retains its own MIT license. These developer-tool
licenses do not decide DRouteX's application or engine-distribution license.

| Skill | Use when / invocation |
|---|---|
| [idiomatic-rust](../../.agents/skills/idiomatic-rust/SKILL.md) | Idiomatic ordinary Rust; $idiomatic-rust |
| [ownership-not-clone](../../.agents/skills/ownership-not-clone/SKILL.md) | Ownership/cloning/shared mutation; $ownership-not-clone |
| [rust-errors](../../.agents/skills/rust-errors/SKILL.md) | Typed errors and boundary failures; $rust-errors |
| [type-driven-design](../../.agents/skills/type-driven-design/SKILL.md) | Security domain states and constructors; $type-driven-design |
| [rust-api-design](../../.agents/skills/rust-api-design/SKILL.md) | Public contracts/traits/compatibility; $rust-api-design |
| [async-rust](../../.agents/skills/async-rust/SKILL.md) | Tokio, cancellation/Send/blocking; $async-rust |
| [rust-concurrency](../../.agents/skills/rust-concurrency/SKILL.md) | Channels/locks/atomics/thread ownership; $rust-concurrency |
| [rust-performance](../../.agents/skills/rust-performance/SKILL.md) | Profile-driven allocation/throughput; $rust-performance |
| [rust-observability](../../.agents/skills/rust-observability/SKILL.md) | Structured/redacted events and spans; $rust-observability |
| [unsafe-rust](../../.agents/skills/unsafe-rust/SKILL.md) | Only justified unsafe invariants; $unsafe-rust |
| [rust-testing](../../.agents/skills/rust-testing/SKILL.md) | Behavior and negative/property/integration tests; $rust-testing |
| [rust-code-review](../../.agents/skills/rust-code-review/SKILL.md) | Review against standards and specification; $rust-code-review |
| [rust-supply-chain](../../.agents/skills/rust-supply-chain/SKILL.md) | Advisory/license/source decisions; $rust-supply-chain |
| [rust-serde](../../.agents/skills/rust-serde/SKILL.md) | Validating serialization boundaries; $rust-serde |
| [setup-rust-ci](../../.agents/skills/setup-rust-ci/SKILL.md) | User-invoked only; after actual posture exists; $setup-rust-ci |
| [setup-rust-pre-commit](../../.agents/skills/setup-rust-pre-commit/SKILL.md) | User-invoked optional hook setup; $setup-rust-pre-commit |

The alternative leonardomso/rust-skills at fd2a861ab0406a4ac536a55274d14ea6fd1ca9c9
was evaluated as an overlapping monolithic rule index; not installed. Porting/macros/FFI/
unrelated setup/router skills were not installed without need.

Read-only availability task: app-server skills/list forceReload discovered all 21 expected
entrypoints without starting a model turn or touching application logic. Installed files and
resource hashes matched upstream. That discovery-only run did not review or test
application behavior; subsequent implementation/review evidence is in owning task reports.

## Instruction conflict resolutions

- Requested warning-free fmt/Clippy/test baseline wins over skills' generic suggestion to omit
  -D warnings when lints exist. P0-02 records the deliberate repo posture for consistent local/CI checks.
- Type-driven design cannot remove validation of changing external facts: DNS address rechecks,
  credential revocation, capabilities and route authorization remain mandatory.
- Ponytail's minimum-code/one-test examples never reduce the spec's negative/property/Linux
  DNS/proxy MVP acceptance matrix. Under accepted CR-0001, real WG/AWG/Xray gates remain
  required for the later protocol expansion; they are not current MVP prerequisites.
- Unsafe skill Miri guidance does not cause automatic nightly installation; isolated justified
  investigation requires separate evidence and does not replace FFI/network tests.
- Setup/pre-commit skills are user-invoked; installation did not authorize hook execution.
  Active CI is defined in the tracked workflow, not by this historical setup session.
  No destructive probe commits/resets from generic skill examples are executed.
- Graphify query-first guidance is scoped navigation assistance; fresh relevant source inspection,
  specification and test evidence remain authoritative. Local graph outputs are never committed.

## Reproduction and activation

PowerShell process activation:

```powershell
. ./tools/activate.ps1
rustc --version
cargo nextest --version
cargo llvm-cov --version
cargo deny --version
cargo audit --version
```

Exact native Rust installation, after downloading and verifying the official rustup-init binary:

```powershell
$env:CARGO_HOME = Join-Path $PWD '.tools/cargo'
$env:RUSTUP_HOME = Join-Path $PWD '.tools/rustup'
& ./.setup/rustup-init.exe -y --no-modify-path --profile minimal --default-toolchain 1.99.0 --component rustfmt,clippy,rust-analyzer,llvm-tools-preview
```

Download origin: https://static.rust-lang.org/rustup/dist/x86_64-pc-windows-msvc/rustup-init.exe
with adjacent .sha256; inspected binary version 1.29.1 and SHA-256
6f4bef66261261fcb43131be8720bab817d403a09edec7455c371974b90bdb7e.

Cargo prebuilt archive URLs/digests and verified binary hashes are in SETUP_INVENTORY.json;
download exact official archives, compare digest, unpack and place executables in
.tools/cargo/bin. The reviewed local installer used .setup/download-tools.ps1; .setup is
deliberately untracked. Linux equivalents, once a lab is chosen:

```sh
rustup toolchain install 1.99.0 --profile minimal --component rustfmt,clippy,rust-analyzer,llvm-tools-preview
cargo +1.99.0 install cargo-nextest --version 0.9.148 --locked
cargo +1.99.0 install cargo-llvm-cov --version 0.9.1 --locked
cargo +1.99.0 install cargo-deny --version 0.20.2 --locked
cargo +1.99.0 install cargo-audit --version 0.22.2 --locked
```

SDK portable packages from the official Microsoft NuGet publisher:
Microsoft.Windows.SDK.CPP and Microsoft.Windows.SDK.CPP.x64 10.0.28000.2705.
Download exact flatcontainer .nupkg URLs, verify catalog SHA-512, unpack to the .tools paths
used by activate.ps1. This is developer-only SDK content; no deployment runtime dependency.
Existing Visual Studio compiler is still needed; a new machine must install native MSVC via
the supported Microsoft installer after reviewing system effects.

Ponytail install:

```powershell
codex plugin marketplace add DietrichGebert/ponytail --ref 2961b52fc1fe6a5a87c7a959a79512e5c2f3ffe5
codex plugin add ponytail@ponytail
```

Graphify: only if missing, `uv tool install graphifyy==0.9.54`;
then `graphify install --project --platform codex`, merge/preserve instructions and review hooks.

Rust skills restore: `npx skills@1.7.2 experimental_install` restores the pinned
skills-lock.json; verify actual installed help before using installer flags. The original install
selected only the 16 names with `--agent codex --copy --yes`, using the pinned GitHub tree URL.

Optional validation-only YAML environment:
`uv venv .setup/yaml-venv --python python3.12`,
`uv pip install --python .setup/yaml-venv/Scripts/python.exe PyYAML==6.0.3`.
Then run tools/verify_quality_proposals.py locally. It makes no network requests.
