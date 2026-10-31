# P3-03 cache implementation evidence

Status: in review under owner-accepted [D54](P3_03_CACHE_CONTRACT.md).
Implementation-head account review and hosted Windows/Linux gates passed as
recorded below. Final whole-PR review, final-head checks and normal merge/main
gates remain required. No completion, deployment, DNSSEC validation or full RR transparency is
claimed by local checks.

## Authorization and implementation

On 2026-11-01 the owner directly accepted all proposed D54 choices and requested
implementation plus Project/Roadmap updates. Baseline `main` was independently
confirmed at8cf65d43181c031c9adbd51d0bca8e648c438afc. P3-02's checked completion
and its existing temporary empty-RDATA disposition were preserved. Backlog and
authorization validator now enforce P3-03; later tasks stay planned.

The private cache stores complete bounded templates using exact scope/upstream/
generation keys. It separates upstream success/provenance from generated failure,
keeps authority checks on hits and misses, and carries expiry to bounded client
I/O. The [allocation audit](P3_03_CACHE_ALLOCATION.md) records actual persistent
capacity, transient decoder limits and the conservative receive/send timing.
No library pin, feature, dependency, wire codec, upstream resource schema, host
networking, workflow, public deployment or new infrastructure change was made.

## Local application checks

Pinned Rust1.99.0 and the preinstalled isolated cloud toolchain were used. Workspace
default and no-default each pass341 tests, zero failed/ignored/filtered. Both
all-target check profiles and workspace all-target/all-feature Clippy pass.
All seven locked dependency/license bundles pass, without suppression or exception.

The cache adds12 pure tests with injected monotonic instants and10 real application
tests. They exercise TTL/SOA/expiry boundaries, complete-response storage, capacity
and FIFO, all enabled key dimensions, current ID/question/EDNS reconstruction,
actual separate peer groups/Endpoints/Profiles, UDP/TCP IPv4/IPv6 reuse, expiry
followed by upstream work, fallback warm reuse, generation/rollback/rejected
publication, simultaneous independent misses, cancellation and decoded-hit races.
The race test holds a decoded hit across publication or signing-trust revocation
and proves no reply/new upstream I/O; an expired hit instead becomes a bounded miss.
The pending-send TTL regression preserves expiry while waiting to write.

Seven removed-check mutations fail their intended pure regressions: positive and
negative TTL caps, scope-aware key equality, scope capacity, SOA owner, exact expiry
and pending-send TTL aging. Restoring the source returns all12 pure tests to green.
An eighth mutation resets the shared checkpoint phase when attaching upstream
cancellation and fails the dedicated transition regression. Its source is restored.
The preliminary broad cache substring also selected real-network tests inside the
restricted sandbox; that environment-only failed attempt is retained separately
and is not counted as mutation evidence.

Existing D53 record matrices and message-ceiling tests now explicitly use TTL0
where their purpose requires two actual upstream exchanges. The changed-self
inventory regression uses a new name for the next actual send. Their destination,
record/frame, count, truncation and failure assertions remain in place; separate
D54 counters prove warm hits. The legal empty NULL/unknown defect is characterized
in every record section of ordinary A and SOA-bearing NXDOMAIN replies on both
upstream transports. Repeated failures reach the upstream again; a valid explicitly
permitted fallback is cacheable. No failed preservation result was relabeled.

The first local cache test attempt exposed a fixture issue: parsing an uppercase
name through the IDNA FromStr path lowercases it. The case-preservation regression
now uses Name::from_ascii and actual mixed-case wire input. A hand-written TXT
RDLENGTH typo was corrected before counting its TC/TCP test as passed. The first
whole DNS attempt retained three failures: the two cold-fixture expectations
above and native UnsafePath from sandbox ownership. A corrected whole DNS run
passed96 tests before the final race/overflow/send-age additions. Final workspace
runs include all99 DNS tests and retain zero ignored/filtered tests.

The filesystem sandbox presents `/` and `/tmp` as nobody-owned; the native durable
fixture correctly refuses that ancestry. Re-running as the same unprivileged
UID1000 outside that filesystem sandbox sees real root-owned ancestors and passes.
No owner/permission requirement or fixture assertion was weakened. Real loopback
tests require the authorized runtime network surface. These environment failures
remain failed attempts, not platform waivers.

Local command transcripts remain outside Git under `/workspace/droutex-cloud/d54-*`.
These logs are supporting local evidence, not portable CI artifacts or a claim
about Windows, exact-main, Codecov/CodeScene or product/systemd acceptance.

## Shared checkpoint gate correction

Initial implementation head e17e6b7 was published in [PR117](https://github.com/romanpodg/DRouteX/pull/117).
Source review found that using independent checked-submit helpers around the
cache could grant an additional D50 checkpoint wait before a later miss entered
D53's Live gate. The implementation now constructs one Live before lookup, uses
it for cache consumption/insertion and every upstream role, and attaches registry
cancellation without resetting the original phase/wait flag. The actual-original
authority and deadline are unchanged. The new transition regression and mutation
prove a second refusal ends work. Both corrected DNS profiles pass100 tests with
zero failed/ignored/filtered; Clippy passes. Earlier workspace341/99 counts above
belong to the preceding implementation, not this corrective delta's hosted proof.

Initial CodeScene8220425 on e17e6b7 remains failed: template eligibility/encoding,
FIFO insertion and the forwarder integration trigger maintenance complexity rules;
the raw independent fixture also scores below10. Specific source-based dispositions
are in [OPEN_DECISIONS](OPEN_DECISIONS.md#p3-03-cache-review-dispositions). They do
not turn the service green or waive a required check. Current-head analysis must
be read again after the correction.

The actual Codex CLI review attempt on e17e6b7 did not complete: after authorization
was established from accepted D54, standing review permission and public repository
visibility, the ChatGPT backend returned401 and could not refresh the configured
login. The interrupted attempt is unverified, not a completed review. Request the
explicitly authorized GitHub Codex review on the corrected head; require an actual
result, not a request or reaction. Branch-protection inventory also returns403
with the present integration token; do not infer its settings or bypass protection.

## Checked implementation head

Corrected implementation head `01fe380eb67980e59a524e299db1ce751fb9bd2a` passes all11
[CI41247958631](https://github.com/romanpodg/DRouteX/actions/runs/41247958631)
jobs. The original e17e6b7 run41247262426 also passed all11, but does not prove the
shared-gate correction. Direct job-log reads confirm309 Windows/342 Linux tests
per default/no-default profile, zero failed/ignored/filtered, client37/37 with all
mandatory IPv6, policy8/8 and codec/transport9/9 per profile/platform. GitHub CLI's
aggregate archive exposed only one job and explicit job reads were empty; the
GitHub connector supplied both complete platform logs. Missing CLI output was not
counted as evidence. The platform logs include the current native durable/inventory,
record matrices and new cache/gate regressions.

Actual owner-requested GitHub Codex implementation review completed on01fe380:
[explicit receipt6549957939](https://github.com/romanpodg/DRouteX/pull/117#issuecomment-6549957939)
and [execution summary6549938095](https://github.com/romanpodg/DRouteX/pull/117#issuecomment-6549938095)
report completed Code Review, exact commit and no findings/unresolved findings.
This is an actual service result, not the earlier failed CLI login, a request,
reaction, silence or a self-review. The summary may later show a newer final review;
this dated record preserves the checked implementation result.

Both exact-head Codecov reports were independently read from the public uploads
API and are `merged`, flags planning/backend; the commit report is complete.
The backend report has10327/10795 covered lines across Rust application and test/
helper code, not production-only coverage. Upload job success alone was not used
as processing evidence.

[CodeScene8220469](https://codescene.io/projects/86145/delta/results/8220469)
on01fe380 remains failed with the same template/FIFO/raw-fixture findings and
Forwarder.answer7.25->6.54. Actual source assessment, current regression/mutation
proof and completed account review support the specific maintenance dispositions
in OPEN_DECISIONS. The shared checkpoint defect was fixed, not deferred. Do not
suppress findings or turn the external result green. The legacy protection endpoint
returns403, but independent branch metadata reports main `protected=false`, the
active-branch-rules endpoint returns[] and GraphQL marks CodeScene not required.
No protection or required check was changed or bypassed. Recheck before merge.

Graphify update was attempted and is unavailable in this cloud image (command127);
no existing graph is present and no replacement/backend was installed. Source,
contract and test evidence remain authoritative.

## Project and remaining external gates

The authenticated GitHub CLI identifies the owner account, but Projects GraphQL
returns `Resource not accessible by integration` for the owner's projects. No
connected browser-control tool is available. The owner was asked for the Project
link and read/write access while implementation continued. Repository Roadmap,
backlog and execution card track actual progress; no external Project update is
claimed. Retry once the required access is available.

Before acceptance, record actual implementation/final account review identities,
exact heads and hosted checks/reports, investigate concrete findings, then perform
normal merge and exact-main verification. Required missing evidence keeps P3-03
open. Existing P3-02 failures/dispositions and D53 security/lifecycle boundaries
are not waived by the cache's local success.
