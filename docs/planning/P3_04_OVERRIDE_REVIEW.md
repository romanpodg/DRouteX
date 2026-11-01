# P3-04 local DNS answers: implementation evidence

On 2026-11-02 the owner requested P3-04 and selected NXDOMAIN for BLOCK,
with BLOCK taking precedence over a profile-local record and then ordinary
DIRECT/upstream resolution. P3-03 is complete; P3-05 remains a separate task.

The current implementation uses the admitted endpoint's selected Profile and
the same immutable compiled generation. A matching local owner returns the
requested A or AAAA RRset, or its CNAME; a local owner without the requested
type returns empty NOERROR, without leaking the query to an upstream. A CNAME
is returned without recursively fetching its target. Gateway actions remain
unavailable. Local records are DNS data, never a gateway destination grant.
Existing publication validation checks conflicting records and profile-local
CNAME cycles. Local responses do not assert DNSSEC authenticity. DO and CD
queries remain NOTIMP under the existing forwarding contract; validation and
HTTPS/SVCB handling belong to P3-05. The resolver's cache stores upstream
answers only; local replies are generated from the current generation.

Responses are bounded by the existing 10,000-entry publication limit and the
TCP message ceiling; oversized local RRsets fail explicitly rather than
returning partial success. This is not a claim that every published RRset is
answerable. UDP uses the existing bounded truncation path. Local answer
generation holds a 4 MiB node-wide wire budget reservation.

The targeted local scope/type/alias test and
UDP BLOCK/override/no-upstream test and two authenticated Profile UDP/TCP test
have passed on the development host. The scope test was first observed failing
with a deliberately incorrect address. The workspace default test run passed
outside the filesystem sandbox; the cross-Profile test was added afterward and
passed separately. The no-default-features workspace run including the new test
also passed. Initial sandbox runs failed 19 durable-store tests because
the sandbox presents root ancestry as nobody-owned. A subsequent unsandboxed
run exposed three tests expecting the old BLOCK SERVFAIL; their assertions were
updated to the selected NXDOMAIN, and the corrected default run passed. Clippy,
offline workspace cargo-deny and planning self-test passed. Independent account
review and current-head CI/reports were pending at that checkpoint. No deployment is claimed.

PR122 initial CI 41340903278 failed the Linux planning job: its negative test
still treated P3-04 as outside the allowed boundary. The test now attempts to
start P3-05 instead, preserving the rejection of unauthorized later work. The
initial failure remains failed; the corrected head requires its own checks.

CodeScene 8227241 on `550df65` failed its three gates, identifying complexity
in local answer construction and the planning boundary validator. The following
revision separates record conversion from answer assembly and the P3-04 gate
from the existing planning boundary checks. Existing selection, admission,
response and publication-boundary conditions remain in place. The old result
remains failed; current-head analysis and review are required.

On `e724798`, [CodeScene 8227388](https://codescene.io/projects/86145/delta/results/8227388)
still fails two gates solely for the large test and auxiliary planning function;
its critical-rules gate passes. The [specific dispositions](OPEN_DECISIONS.md#p3-04-review-dispositions)
retain those failures and set revisit triggers without suppressing any check.
Actual Codex Code Review of `e724798e7cebe310f39e43a684888901bb3d57cc`
completed without reported findings; the later documentation-only head then
required its own review and checks.

## Completed implementation and main verification

PR [#122](https://github.com/romanpodg/DRouteX/pull/122) merged normally as
`923eec226908d434a2f230ae4b34613b4334c18f`. Actual Codex Code Review of final
head `a121eb88ab7f21e8dc410bb054130ef3b5ddca2c` completed without reported
findings. PR-head run 41343984001 passed all 11 Foundation jobs and Codecov
checks. Exact-main run [41344649618](https://github.com/romanpodg/DRouteX/actions/runs/41344649618)
passed all 11 jobs; Codecov's exact-commit report is complete and both planning
and backend uploads have state `merged`. These results do not claim product
deployment or the later DNSSEC/HTTPS/SVCB acceptance.

Final-head [CodeScene 8227395](https://codescene.io/projects/86145/delta/results/8227395)
remains failed with the same two non-required test/auxiliary-tool findings
assessed in [OPEN_DECISIONS](OPEN_DECISIONS.md#p3-04-review-dispositions);
the critical-rule gate passed. GitHub GraphQL reported the CodeScene check as
not required; the protection-settings API returned 403. No result was suppressed
or represented as passing. P3-05 remains planned.
