# P3-04 local DNS answers: work in progress

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

Work is not accepted or merged. The targeted local scope/type/alias test and
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
review, current-head CI/reports and applicable Linux/Windows acceptance remain
to be recorded. No deployment is claimed.
