# P2-02 deterministic scoped selection evidence

Status: D38 implementation accepted at d275d3fcd58d50e88b1e31f38eed61ce127702f8;
[PR13](https://github.com/romanpodg/DRouteX/pull/13) final-head review/checks and normal
merge remain required. No next implementation begins before that gate.
[PR12/P2-01](https://github.com/romanpodg/DRouteX/pull/12) merged normally at
6fa475976df6a9194dcab3ccdbc3c759308cd7b0. All11 exact-main jobs39291553252 succeeded
and both exact-merge Codecov reports processed before this implementation started.
The owner's complete [accepted contract](P2_02_EVALUATION_CONTRACT.md) is the scope;
no approval alone is a successful test or review.

## Implementation boundary

The pure policy crate owns immutable bounded PolicySet data, separate Endpoint and
gateway group associations, prepared domain/TCP interval entries and borrowed
PolicySelection results. No registry/lock/feature changes, I/O, unsafe, async tasks,
new service process or runtime authority is added. Public config DNS/traffic effect
types gain Eq/PartialEq only, without a serialized shape or signed D11 change.
Two genuine action types motivate generics, avoiding duplicate effect vocabulary or
a cyclic config/policy dependency; comparison is not execution/capability resolution.

Private immutable DomainRule rank caches explicit category, full signed i32 priority,
canonical label count and accepted mode specificity. Query selection uses two
allocation-free borrowed iterator passes: find the globally highest rank and least
source identity; then refuse contradictory effects at that rank. A lower-rank
conflict cannot hide a higher winner. Stable source UUID bytes, with service revision
UUID secondary for equal rule IDs, only select equivalent effects. No action weight,
port-width priority, map/list order or integer negation/subtraction enters ordering.

The10000 aggregate budget includes Profiles, Endpoint/group associations and all
prepared rule entries, including disabled ones. TCP ranges are inclusive nonzero
u16 intervals, never expanded into ports; repeated source/effect entries encode
OR scopes under the same budget. Duplicate associations/dangling Profiles fail.
DNS checks its registered enabled Endpoint/Profile binding; TCP independently maps
its gateway group. Neither public IDs nor this table authenticate a client, verify
catalog bytes/licenses or grant destination authority. P2-03/04/05/06/07 and P3/P4
retain comprehensive validation, effects/capabilities, full compilation, Explain,
imports and actual network admission.

## Checks so far

Windows focused tests passed13 evaluator groups plus the existing6 name groups;
a config integration test exercises real borrowed DnsAction/TrafficAction values,
explicit resolve/BYPASS overriding a filter, default resolve leaving filters active,
TCP eligibility and preserved source configuration strings/serialization. No actual
catalog import or network connection is claimed by the synthetic prepared conditions.
All four policy compile-fail contracts (including name boundary) passed in the
whole-workspace run: ownership, DNS/gateway separation and missing-port refusal.

Evaluator tests cover exhaustive permutations of bounded candidate fixtures,
extreme cross-layer/within-layer priorities, specificity and equivalent UUID ties,
stable contradictory-top errors including duplicate identities, lower-key conflicts,
disabled/nonmatching entries, Unicode/ACE/descendant/lookalike queries, TCP boundaries/
overlap/OR/unknown metadata, independent Profile scopes, duplicate/dangling inputs,
exact10000 and over-limit entries and sanitized opaque-effect diagnostics.

Independent parallel Rust standards/spec source passes found no actionable defect;
they changed nothing and ran no Cargo checks. Windows full Q-RUST passed: fmt,
workspace/all-targets check for default/no-default profiles, all-features Clippy
with warnings denied, workspace tests for default/no-default profiles, rustdoc and
cargo-deny (advisories, bans, licenses and sources). Existing duplicate-version
warnings remain disclosed; no registry, dependency or lockfile changed.
Q-DOC passed17 tests,9 negative validator cases and243 links, preserving88 tasks,
61-task MVP closure,188 requirements,149 source headings and17 ADRs/2 Accepted.

## Actual implementation acceptance

At unchanged implementation head d275d3f, actual linked-account read-only/never
Code Review completed with no confirmed defect, session
01a15533-a43b-73d2-8a54-1236ffa87a30, exit0. The reviewer ran formatting and existing
test binaries; read-only build-lock refusal prevented rebuilding/Clippy there.
That limitation is not a successful rebuild: the parent Windows run and real CI
below separately provide actual build/Clippy/test evidence.

All11 jobs of CI39294636237 completed successfully on the exact head. Saved actual
logs confirm106 Windows and133 Linux workspace tests per default/no-default
profile (including four policy compile-fail contracts), plus the isolated IDNA
qualification and DNS/TLS, SQLite and secret qualification jobs. Fresh cargo-deny
passed advisories/bans/licenses/sources for the workspace and all four qualification
manifests. Backend/planning report generation and OIDC uploads succeeded.
No network/runtime behavior beyond those existing qualification fixtures is claimed.

Codecov processed both exact-head reports, state complete/sessions2. Rust backend
is3638/3828; the composite4092/4285 includes auxiliary Python and is not application
coverage. The exact-base6fa4759 comparison covers232/239 new Rust lines (97.07%):
evaluate/mod.rs62/68, ordering.rs55/56, scope.rs115/115. The seven missed lines are
the public RuleSource.layer accessor and the filter source-key arm; neither is
claimed executed. The two Eq derives add no executable patch lines. Coverage is
not a correctness or authorization proof.

CodeScene8022611 failed hotspot/new-code maintenance gates; the third gate passed.
Its findings are exclusively tests and auxiliary planning tools: evaluator tests9.27,
config integration test9.28, test_planning_tools.py9.66->9.64 and
verify_planning.py9.02->9.00. There is no reported application-source violation.
These files are outside the owner's independent metric scope; source inspection
and actual regressions found no concrete functional defect. Preserve the failed
repository-wide check and P2-02-CS-AUX disposition in
[OPEN_DECISIONS](OPEN_DECISIONS.md), without score-only rewrite or suppression.

The owner separately accepted D39 [publication validation](P2_03_PUBLICATION_CONTRACT.md)
after reviewed PR13 normal merge and exact-main checks. That is planning authority,
not current P2-03 implementation or a successful test. Later backlog tasks remain
planned; final evidence review/current-head checks and normal merge are pending.
