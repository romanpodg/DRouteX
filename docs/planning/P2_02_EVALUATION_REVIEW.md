# P2-02 deterministic scoped selection evidence

Status: D38 owner-accepted; implementation candidate under review, not merged.
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
Actual Linux/current-head CI, external coverage/health assessment and required
linked-account implementation/final reviews remain pending.
