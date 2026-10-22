# P4-01 current authorization evidence

Status: **completed, normally merged and exact-main verified**. D45 execution order
and the complete [D46 contract](P4_01_BINDING_CONTRACT.md) were directly accepted
on2026-10-22. Before code, PR106 merged normally at671c9f6154add92486c90f5a620d4dbefa9a510a;
all11 exact-main39662936089 jobs and both Codecov reports processed(Rust5980/6262).
There is no public listener, connector, registration API or automatic learned grant.

## Current change and compatibility

The existing six-crate workspace gains typed bounded manual leases, strict v2
configuration/D11 header matching and journal clock floors. v1 canonical/signed
vectors remain exact and association-only. Core owns current-generation source/
scope/time checks, single-attempt non-Clone evidence, shared rate/concurrency quotas,
and local durable clock refresh/repair. Real D38 selection/D40 action resolution
and exact compiled service domains/catalog pins still decide gateway candidates.
See [operational boundary](../security/GATEWAY_BINDING.md).

No registry package/version/license/crypto algorithm is added. Core tests reuse
the existing first-party droutex-policy workspace crate as a dev dependency; the
manifest/lockfile edge is intentional. There is no new production dependency or
Cargo feature, public endpoint, privileged operation or network execution.

## Actual checks so far

Current Windows workspace all-features and no-default profiles each passed199
tests, including18 admission cases and compile-fail boundaries. All-target check,
fmt and all-features warning-free Clippy passed. All five current cargo-deny
bundles passed advisories/bans/licenses/sources after fresh advisory fetch;
existing duplicate-version warnings remain. Planning passed19 tests,9 negative
validator cases and305 local links, preserving both source specifications.

One real defect was reproduced then fixed: invalid gateway targets did not consume
attempt-rate credit, allowing unlimited expensive policy checks from an otherwise
granted source. The failing behavioral regression is retained locally as
.setup/p4-01/invalid-attempt-rate-red.log. Quota reservation now precedes target
evaluation; refusal releases concurrency without refunding rate. The regression
and expanded admission run passed. Compile errors during test construction and a
test's initially premature clock re-anchor are not counted as product regressions.

Independently written JSON/YAML/TOML v2 fixtures produce the same canonical signed
vector (1409 bytes): envelope SHA256
afb8b1bd13fc3c799fdb730dc1619122d54d50a0d6dd5b2eae0ab3e28a26dbd4,
payload c61b75c7ed0360abbba615bac0f5ec0c90b63cdf639b459c0e5b8b3eace46c63.
Only public RFC8032 fixture material is used. Local Git-bundled OpenSSL3.5.4
independently verified its signature and rejected a changed byte(exit1). Python
stdlib independently checked header/hash, canonical object bytes and exact lease
fields. The temporary capture writer was removed; only intended public fixtures
are version-controlled. This does not qualify a new cryptographic primitive.

Supplementary standards/spec review found a clock-slew expiry revival and a quota
inventory race between preflight and durable commit. The clock regression failed
behaviorally before the fix: the conservative upper time now never decreases,
every monotonic reading is checked, and repair cannot lower observed time.
Publication now pauses new acquisitions until commit/reconciliation completes;
RAII cleanup resumes after success or known failure without holding a lock over
I/O. The exact canonical gateway domain/port is retained and rechecked. Added
positive IPv6, same-IP independent DNS Profiles and stronger concurrent-generation
quota evidence passed. Both independent supplementary re-reviews found no
remaining confirmed defect.
They cannot replace the required linked-account reviews.

Seven targeted behavioral mutations independently disabled upper-time retention,
every-sample monotonic checking, publication pause, exact destination port, current
generation, checkpoint coverage and gateway attempt rate. Every regression test
failed behaviorally; exact source bytes were restored. Harness pattern mismatches
were corrected and are not counted as detected mutations.

## Implementation CI and account review

Implementation efd3f28962be222911c5fc521581545a584d74b9 passed all11 actual
[CI39671360399 jobs](https://github.com/romanpodg/DRouteX/actions/runs/39671360399).
Windows passed199 workspace tests and Linux230 in each default/no-default profile.
Actual Linux durable tests passed CP-loss restore/expiry, conservative restart,
pre-rename checkpoint failure, post-rename uncertain commit/reopen, applied removal,
initialized-floor corruption and admission pause/error cleanup during real commit.
All existing dependency/TLS/storage/secret/IDNA, planning and coverage jobs passed.
No real socket/connector, automatic learning or systemd acceptance is claimed.

Linked-account implementation Code Review01a1602b-028d-7192-b217-97743133bb9e
completed with exit0, read-only sandbox and approval never: no confirmed defect.
Its fmt and9 validator tests passed. Its18 cached binary tests are supplemental;
source correspondence was not established. Read-only Cargo rebuilding/Clippy was
unavailable, so actual exact-source execution evidence comes from the parent
Windows runs and the independent CI Windows/Linux jobs above, not those binaries.

Codecov processed the exact implementation commit with2 sessions and complete
state. Rust application coverage is6689/7013 (95.38%); Rust patch717/759 (94.46%).
Planning coverage is separate. The excluded critical/large test-method metrics do
not justify rewriting tests. CodeScene8066731 failed all three maintenance gates:
application admission9.39, quota9.54 and candidate9.69 plus excluded tests/planning.
Every concrete signal, impact, reason and revisit trigger is recorded in
[the existing register](OPEN_DECISIONS.md#p4-01-review-dispositions). No suppression,
score-only rewrite, advisory exception or branch-protection bypass was used.

## Final review, normal merge and exact main

Final head fecb869aa0538e1eecf87bfdce8b92b612327c5b passed all11 CI39672366016
jobs and processed both Codecov sessions with the same Rust/patch totals. Final
CodeScene8066873 failed with exactly identical finding text and numeric tables;
all four recorded dispositions remain applicable without suppression.

The first final review was interrupted by a quota error and was not counted.
After the owner continued, linked-account read-only/never final review
01a162ab-81e6-7102-bc79-696105883a15 completed with exit0/no confirmed defect.
Fmt and9 planning checks passed; read-only build-lock restrictions prevented Rust
rebuild/Clippy. An after-verdict rollout-record warning is retained. Actual Rust
execution comes from the independently successful current-head CI, not the review.

[PR107](https://github.com/romanpodg/DRouteX/pull/107) merged normally on2026-10-22
at02b6695928a64c37de5e073a4bdb5162f1928c54 with an exact-head match, no admin/force
or protection bypass. All11 [exact-main39747433154 jobs](https://github.com/romanpodg/DRouteX/actions/runs/39747433154)
passed and Codecov processed both sessions(Rust6689/7013). The owner accepted full D47 P3-01 conditionally on this prerequisite, fulfilled
before qualification code. P3/P4/P6 runtime integration/secure
learning remains at its owning stages; manual/current authorization is complete.
