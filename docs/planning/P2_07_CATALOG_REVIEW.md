# P2-07 local catalog import evidence

Status: accepted D41 local catalog components and D42 security correction passed
implementation review and actual Windows/Linux CI/coverage. Final review/checks
and normal merge remain required. P2-05/06 and runtime stay planned.
See [accepted contract](P2_07_CATALOG_CONTRACT.md). PR15 prerequisite completed at
b4c104c: implementation/final linked-account reviews, all11 exact-main39481364319
jobs and both Codecov reports (Rust4329/4535) before any P2-07 code.

## Implementation

Policy parses explicit UTF-8 domain/blocking-hosts formats into D37 conditions;
all modes are explicit, sinks never become local DNS answers, no URL/Adblock/glob
or arbitrary hosts redirection. Raw duplicates/aliases count before normalization;
canonical conditions sort/deduplicate without copying name strings. Errors expose
only typed reasons and line positions; Debug shows counts, not domain data.

Config scans service JSON depth/array counts before buffered parsing, then reuses
the qualified bounded duplicate-key/object/depth boundary. Version/kind/fields/types,
mode/UUID/hash and complete selected dependency closure are strict. Source bytes,
format/mode, source hash, source/owner/license/notices and approval evidence remain
retained. Approval is an explicit trusted-operator revision/raw-hash/license/evidence
object with no feed-controlled Boolean or Deserialize; it cannot authenticate the
caller, and real P6 authorization remains mandatory. Dependency licenses do not
approve a dataset. Synthetic fixtures are original Apache-2.0, not external feeds.

Fixed versioned structs encode sorted canonical conditions/dependency pins and
actual metadata/source hash into resource bytes; qualified SHA-256 hashes those
actual bytes for existing D11 pins. Raw source hash differs from resource hash;
original source/comment/order changes require another immutable revision. A bounded
writer checks JSON expansion BEFORE extending output. Existing1MiB input and64MiB
generation resource constants move to common, preserving old config/core aliases
and exact values; no registry dependency, lockfile, feature or current wire/signed
codec change. P1-06 still owns actual durable activation; no filesystem downloader.

The store prepares all candidates and validates the complete bounded retained set
before its only exclusive mutation. Successful imports retain prior immutable
revisions, identical reimport is idempotent and rebinding an identity refuses.
Keeping history within the same accepted inventory/resource budgets prevents
remove-then-rebind; there is no deletion/latest/automatic publish/Profile action.
Preparation and whole-set validation are private; VerifiedCatalogRevision is exposed
only after successful commit and cannot deserialize claimed facts. Exported D39
facts still require P2-05 binding actual data/config/resources to one generation.

## Checks so far

Focused Windows checks passed4 policy,9 config integration and2 scanner/writer
unit groups. Actual local Unicode/hosts/service fixtures, independent Python stdlib
json/hashlib canonical golden and dependency hashes, mode/apex/subdomain behavior,
BOM/CRLF/comments, sinks/aliases, duplicate/unknown/positional JSON fields/types,
malformed formats, metadata/source/approval mismatches and private/action fields,
exact raw/retained aggregate and input+metadata bounds,65 oversized borrowed inputs,
immutable identities and failed multi-candidate last-known-good updates are covered.
A deliberate mutation clearing the old store before validation made the regression
fail (0 retained revisions instead of1); source was restored before normal checks.
Preflight/prepare/validate/commit are separate immutable phases for auditing the
single mutation point, not a metric-only split. Focused tests and all-features
Clippy passed after that extraction. Full Q-RUST and independent/external evidence
are recorded when completed; pure tests do not prove runtime or feed acquisition.


Full Windows Q-RUST initially passed149 tests per default/no-default profile,
workspace/all-targets checks, all-features Clippy/warnings denied, rustdoc and fresh
cargo-deny advisories/bans/licenses/sources (existing duplicate warnings retained).
Independent standards read-only source pass found no confirmed actionable defect.
Spec pass found service entry/dependency validation discarded required positions.
The fix preserves only whitelisted entries/dependencies array indices via qualified
serde_path_to_error and typed zero-based variants, dropping input/parser strings.
A new regression checks domain/type/mode indices and UUID/hash/kind dependency
indices with redaction; all10 config integration groups passed after the correction.
Read-only spec follow-up confirmed the finding resolved with no new confirmed
defect. Full updated Windows Q-RUST passed150 tests per default/no-default profile,
fmt, workspace/all-targets checks, all-features Clippy/warnings denied and rustdoc.
The previous fresh deny remains applicable: no dependency/lockfile/license change.
Graphify AST-only update completed (3962 nodes/8393 edges); missing SQL parser and
zero-node Cargo.toml warnings remain explicit. No semantic/API backend or complete
graph evidence is claimed; all generated graph artifacts remain untracked.
Q-DOC passed17 tests with missing-D41/source and unfinished-P2-04 negative mutations,
9 negative validator cases/265 links, preserving88 tasks/61 MVP/188 requirements,
17 ADRs/2 Accepted and source specifications unchanged.


## Actual implementation review and CI security gate

Implementation971b32661dacf9435f765ebf625c3e663b4744e0 passed linked-account
read-only/never Code Review01a15ad2-7b4f-7180-8673-c7299c92518e, exit0, no confirmed
defect. Formatting and14 tests in existing binaries passed; full rebuild/Clippy/
cargo test were denied by the read-only lock. Actual parent and CI checks remain
separate evidence, not inferred successes.

CI39485189479 completed FAILURE: nine jobs succeeded, Rust dependencies/coverage
failed and the dependent Codecov upload was skipped. Workspace Windows/Linux
and existing qualifier jobs passed. Dependency failure is the fresh official
RUSTSEC-2026-0382 advisory in the existing test-only TLS qualification lockfile,
rustls-webpki0.103.15. Production workspace has no Rustls dependency, but the gate
still blocks merge. Coverage was NOT generated/uploaded at this head. Original
failed run/logs remain retained, no ignore, suppression or status exception.
The official stable0.103.16 ISC archive/checksum/MSRV were inspected read-only;
owner decision was pending at that failed head. D42 now authorizes exact0.103.16
qualification/conditional adoption; [security evidence](../operations/WEBPKI_SECURITY_UPDATE.md)
records the minimal candidate lock/inventory update, actual Windows/upstream tests
and full artifact/license checks. Linux/current-head CI and independent review
remain required; the original failed run is not retroactively passed.

CodeScene8044833 FAILED all3 maintenance gates. Source findings policy/catalog.rs8.96,
config/catalog/store.rs9.39 and streaming scan.rs9.39 were assessed against actual
source invariants and regression/review evidence. Explicit minor impact/rationale/
P2-05/grammar/budget/pairing revisit triggers are recorded in
[OPEN_DECISIONS](OPEN_DECISIONS.md); tests/auxiliary findings remain outside the
owner's independent metric scope. No score-only split or successful CodeScene claim.

## Qualified correction and complete candidate checks

Candidate3f203a6f462c68746e2d4ee0f53f444a70b627e8 passed all11 jobs of
[CI39490608943](https://github.com/romanpodg/DRouteX/actions/runs/39490608943). Actual
workspace counts: Windows150/Linux177 per default and no-default profile. Linux
DNS/TLS qualifier passed all3 groups; local exact tracked Windows qualifier passed
fmt/Clippy/all3 groups. All5 manifest dependency checks passed all4 categories
against freshly fetched advisory data. D42 qualification is successful with no
new exception or production TLS adoption; the original failed run stays failed.

Independent current-head linked-account read-only/never Code Review
01a15af5-e9e0-7942-89fc-ce5aa6b06394 exited0, no confirmed defect. Formatting and16
existing-binary tests passed; fresh rebuild/Clippy/test remained read-only-lock
restricted. Parent and actual CI evidence above provides those execution results.

[Exact-head Codecov](https://app.codecov.io/github/romanpodg/droutex/commit/3f203a6f462c68746e2d4ee0f53f444a70b627e8)
processed both backend/planning reports (complete,2 sessions). Rust-only totals are
4951/5177 instrumented lines (95.63%); actual backend patch comparison against
PR15 merge is612/642 Rust lines (95.33%). Auxiliary Python is excluded from those
application figures; no metric-only tests/refactor was introduced.

CodeScene8045334 again FAILED the same3 gates with identical source/auxiliary
findings and scores. Current code is unchanged from the assessed implementation;
OPEN_DECISIONS dispositions remain applicable, with owning P2-05/grammar/budget
re-review triggers. No suppression, claimed successful health gate or check bypass.
Q-DOC passed17 tests/9 negative cases/274 links after security evidence additions,
with the same88 tasks/61 MVP/188 requirements and unchanged source specifications.
Final documentation commit still requires independent review and current-head CI;
normal merge and exact-main jobs/processed reports are not yet claimed.

Owner accepted complete D43 P2-05 profile after the still-required reviewed PR16
merge/exact-main prerequisite. This records the decision only; no compiler
implementation or next implementation PR has begun. Final AST-only graph update
completed3974 nodes/8409 edges; SQL parser/zero-node manifest warnings remain
explicit and generated graph artifacts stay untracked.
