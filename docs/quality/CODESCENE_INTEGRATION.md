# CodeScene integration

Status on 2026-10-15: portable native CLI **1.0.53**, build
c46e3cba9e527cb84281355e7cd6d7d2ead627df, installed in .tools/codescene.
Version/help/auth status were executed. CLI OAuth is not signed in; browser analysis
is verified below. No CLI analysis or MCP connection has been verified.
The user authorized configuring CodeScene and Codecov for
[romanpodg/Routira](https://github.com/romanpodg/Routira) on 2026-10-15.
GitHub CLI admin access and CodeScene browser sign-in are verified.
Project [Routira, ID 86145](https://codescene.io/projects/86145/config) was created.
The analysis ref is `docs/review-dns-proxy-foundation`, which contains the existing
planning tools; return it to `main` after the owner merges that branch. The first
review-branch analysis is [job 7965503](https://codescene.io/projects/86145/jobs/7965503/results).
It completed successfully: Python/PowerShell were detected; Code Health needs
attention and there is insufficient commit history for a useful technical-debt trend.
These are tooling/repository results, not application quality evidence.
The user confirmed adding Routira to the pre-existing CodeScene Access GitHub
App. The saved grant covers Routira plus the previously selected repository;
other repository access was not changed. PR integration is enabled with the
existing Minimal Safety Net profile. Individual findings go into Check Run details;
always-comment and target-ref-only filtering are disabled.
The first [PR review 7965631](https://codescene.io/projects/86145/delta/results/7965631)
was delivered on commit `32a458d` as `CodeScene Code Health Review (main)`.
The analysis completed but its quality gate **failed** with 15 findings of four
types across five Python tools: `build_doc_graph.py`, `capture_setup_inventory.py`,
`render_planning.py`, `verify_planning.py` and `verify_quality_proposals.py`.
The findings concern complex/large methods and nested conditional blocks. The PR
introduces these tools relative to the initial main; this is not evidence that the
last documentation commit caused all findings. No finding was suppressed or gate
relaxed. Tool refactoring is separate from service activation and product approval.
The follow-up remediation separates rendering, validation, inventory verification
and document graph extraction into focused functions. Fourteen regression tests
check unchanged generated planning output, CI authorization guards, resource/hash
mismatches and graph refresh restrictions. They run in both planning CI OS jobs;
Graphify stays optional for these tests. Existing quality gates remain unchanged.
Local CLI review was attempted but requires separate CLI credentials; the GitHub
check on each published commit remains the source of CodeScene result evidence.
Daily analysis is saved for **03:00 Asia/Omsk**. No paid plan was selected.
The initial PR monitor will inspect actual results; repository
creation/publication alone does not establish a baseline or successful integration.

[Official CLI documentation](https://www.codescene.io/docs/cli/index.html) documents native
Windows installation and file/delta analysis. We inspected its PowerShell installer: it
downloads/unpacks CLI+runtime DLL, persists user PATH and removes temp files. Instead we
downloaded the official portable archive locally and changed only process PATH.
CLI output supports `cs delta [branch [branch]]`, `--staged`, `--output-format json`
and `--error-on-warnings`; branch names must actually exist.

[Supported languages](https://www.codescene.io/docs/usage/language-support.html) include Rust
and TS/React. The completed tooling analysis does not establish product-language acceptance.

## Concrete activation plan

1. Sign into [CodeScene](https://codescene.io/login) with GitHub and select the
   free Community Edition for this public repository. Account creation/terms and
   any new security-sensitive GitHub permission grant require the user's action-time
   confirmation. CLI sign-in is separate: `cs auth login`, then `cs auth status`.
   Do not put access tokens in tracked files or chat. CI credentials use approved secret
   storage; source code/service data upload requires account-side access selection.
2. Create/choose the Routira GitHub repository; grant the CodeScene application only
   required repository access and configure the project/PR integration.
3. Use the existing Python planning tools for an initial supported-code analysis if
   the service can analyze the review branch. Main currently has no product baseline;
   do not merge PR #1 or change its target to manufacture one. After first cohesive
   implementation, run a product baseline and record exact engine/config
   version, score availability and review rules. Git history is needed for meaningful hotspots.
4. For local change review use `cs delta` or `cs delta --staged`; for actual branches
   use `cs delta main feature-branch --error-on-warnings` after inspecting installed help.
   Inspect file-level `cs review --help`/`cs check --help` before using options.
5. Configure PR no-unaccepted-regression and new-module health gates against measured
   baseline. Do not choose a fictional universal score threshold.
6. Track deteriorating/co-changing modules and architectural hotspots each release; combine
   results with Graphify/source inspection and functional/security/network evidence.

## MCP scope and blockers

[Official MCP source](https://github.com/codescene-oss/codescene-mcp-server) is a separate
CodeScene server. A CLI install does not create MCP access. No CodeScene tools are exposed
in this session. Do not install/configure the server until its exact version, required account
permissions and source-analysis effects are reviewed. P0-04/D16 owns this optional addition.

Remaining verification: the current-head remediation check, an actual implementation
baseline and owner acceptance of the product contracts.
Browser login does not sign the CLI in.
Continue mandatory local checks.
CLI startup update check can be disabled for deterministic help with
`$env:CS_DISABLE_VERSION_CHECK = '1'`; this does not bypass authentication or analysis.
