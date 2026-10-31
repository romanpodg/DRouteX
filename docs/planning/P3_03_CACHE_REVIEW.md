# P3-03 cache implementation evidence

Status: in progress under owner-accepted [D54](P3_03_CACHE_CONTRACT.md).
Local component evidence is recorded below; current-head account review, hosted
Windows/Linux CI, configured processed reports and normal merge/main gates remain
required. No completion, deployment, DNSSEC validation or full RR transparency is
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
