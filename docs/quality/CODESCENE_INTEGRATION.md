# CodeScene integration

Current P1-01 evidence (2026-10-16):
[review 7987296](https://codescene.io/projects/86145/delta/results/7987296) on
implementation head `4d79b7a95f094851bd2b73c4c924a85c7231de44` passed all three
quality gates for PR #4 against main `a77ed5adce184ffe3214452af0b1e6d83db2cd92`.
The application domain changes are in scope. Expected-document-change suggestions
are historical cochange observations, not confirmed functional defects. No gate,
suppression, access or analysis setting was changed to obtain this pass. Detailed
review/test limits are in the [domain review](../planning/P1_01_DOMAIN_REVIEW.md).

Historical P0-04 evidence:
[review 7980486](https://codescene.io/projects/86145/delta/results/7980486) on
`e4f4143` failed score/complexity gates in the planning validator, auxiliary graph
tool and two network fixtures. These are outside the owner's independent metric
targets. No suppression, gate change or score-only rewrite was made. The overall
check for that historical commit remains failed; see the [CI review](../planning/P0_04_CI_REVIEW.md).

## Initial setup record

Status on 2026-10-15: portable native CLI **1.0.53**, build
c46e3cba9e527cb84281355e7cd6d7d2ead627df, installed in .tools/codescene.
Version/help/auth status were executed. CLI OAuth is not signed in; browser analysis
is verified below. No CLI analysis or MCP connection has been verified.
The user authorized configuring CodeScene and Codecov for
[romanpodg/DRouteX](https://github.com/romanpodg/DRouteX) on 2026-10-15.
GitHub CLI admin access and CodeScene browser sign-in are verified.
Project [DRouteX, ID 86145](https://codescene.io/projects/86145/config) was created.
The analysis ref is `docs/review-dns-proxy-foundation`, which contains the existing
planning tools; return it to `main` after the owner merges that branch. The first
review-branch analysis is [job 7965503](https://codescene.io/projects/86145/jobs/7965503/results).
It completed successfully: Python/PowerShell were detected; Code Health needs
attention and there is insufficient commit history for a useful technical-debt trend.
These are tooling/repository results, not application quality evidence.
The user confirmed adding DRouteX to the pre-existing CodeScene Access GitHub
App. The saved grant covers DRouteX plus the previously selected repository;
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
2. Create/choose the DRouteX GitHub repository; grant the CodeScene application only
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

The continuation inspected [review 7971972](https://codescene.io/projects/86145/delta/results/7971972)
on PR head `023ce200ebf9b3e1b53bb0a1218576d8db092e46`. The check completed with
**failure**: one Excess Number of Function Arguments rule in
`tools/build_doc_graph.py:add_markdown_links`, Code Health 9.69. Two other quality
gates passed. This is a score-only finding in an auxiliary graph tool, outside the
owner's selected quality-improvement scope. No concrete functional defect was
established by that result; no further refactor, suppression or gate change was made.
See the [execution status](../planning/EXECUTION_STATUS.md) for the inspected CI head.

The owner subsequently accepted P0-01/CR-0002. Remaining verification: actual
product implementation/runtime evidence. Later commits require their own check results.
Browser login does not sign the CLI in.
Continue mandatory local checks.
CLI startup update check can be disabled for deterministic help with
`$env:CS_DISABLE_VERSION_CHECK = '1'`; this does not bypass authentication or analysis.

P0-02 follow-up: [review 7972404](https://codescene.io/projects/86145/delta/results/7972404)
on implementation commit `7ac5f14deb25bdb687b0aee2008e6fda11734be1` completed with
the same one auxiliary argument-count finding (9.69). No application-code finding
was reported in its Check Run details; the repository-wide gate is still failed.
No score-only auxiliary cleanup, suppression or service configuration change was made.

## P1-02 application boundary evidence

[Review 7991872](https://codescene.io/projects/86145/delta/results/7991872) on
`a708e34b27a82fe5b4682b1761b34569144c94c7` failed new-code health: binding.rs
collision predicates 8.41, access.rs/rules.rs explicit helper arguments 9.69,
and a test helper 9.69 outside the independent metric scope. Two other gates
passed. This includes application code, so the source and wildcard/dual-stack/
scope/order tests were inspected; independent re-review found no confirmed
functional defect after four separate strictness/path fixes. No score-only
rewrite, suppression or service threshold change was made. Finding, impact,
rationale and revisit triggers are recorded in OPEN_DECISIONS P1-02-CS and the
[P1-02 review](../planning/P1_02_CONFIG_REVIEW.md). The check remains failed.
