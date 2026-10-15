# CodeScene integration

Status on 2026-10-15: portable native CLI **1.0.53**, build
c46e3cba9e527cb84281355e7cd6d7d2ead627df, installed in .tools/codescene.
Version/help/auth status were executed. OAuth is not signed in. No analysis, Code Health
score, baseline, GitHub PR integration or MCP connection has been verified.
The user has selected [romanpodg/Routira](https://github.com/romanpodg/Routira) and
will connect CodeScene. The initial PR monitor will inspect actual results; repository
creation/publication alone does not establish a baseline or successful integration.

[Official CLI documentation](https://www.codescene.io/docs/cli/index.html) documents native
Windows installation and file/delta analysis. We inspected its PowerShell installer: it
downloads/unpacks CLI+runtime DLL, persists user PATH and removes temp files. Instead we
downloaded the official portable archive locally and changed only process PATH.
CLI output supports `cs delta [branch [branch]]`, `--staged`, `--output-format json`
and `--error-on-warnings`; branch names must actually exist.

[Supported languages](https://www.codescene.io/docs/usage/language-support.html) include Rust
and TS/React. This establishes documented support, not a working analysis of this empty product.

## Concrete activation plan

1. User signs into CodeScene: `cs auth login`, then `cs auth status`.
   Do not put access tokens in tracked files or chat. CI credentials use approved secret
   storage; source code/service data upload requires account-side access selection.
2. Create/choose the Routira GitHub repository; grant the CodeScene application only
   required repository access and configure the project/PR integration.
3. After first cohesive implementation, run a real baseline and record exact engine/config
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

External blockers: CodeScene sign-in/account authorization, selected remote repository,
actual implementation baseline and first verified PR run. Continue mandatory local checks.
CLI startup update check can be disabled for deterministic help with
`$env:CS_DISABLE_VERSION_CHECK = '1'`; this does not bypass authentication or analysis.
