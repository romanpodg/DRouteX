# P2-06 generation-bound Explain and simulation evidence

Status: accepted D44 implemented and locally checked. Supplementary independent
standards/spec reviews found no confirmed defect. Mandatory linked-account
implementation/final reviews, actual current-head Windows/Linux CI, external
reports, normal merge and exact-main verification remain required.

The complete [accepted contract](P2_06_EXPLAIN_CONTRACT.md) records direct owner
acceptance and the prerequisite fulfilled BEFORE code: normal PR105 merge
445f847e74d0b04d32d619be02865c36268648dc, all11 exact-main39552210853 jobs and both
processed reports(Rust5507/5759). P2-05 final evidence is recorded separately.

## Implemented boundary

Pure policy Explain calls the existing D38 evaluator and highest/conflict routine;
it introduces no alternate winner or action precedence. The selected Profile and
plane provide every prepared entry: enabled state, canonical condition/mode,
priority/labels, source, domain/port eligibility and selected/equivalent/lower-rank
reason. Stable semantic order includes enabled state for otherwise equal entries.
The original10000 aggregate constructor/compiler limits bound the full trace;
there is no truncation, caller-selected larger limit or whole-policy enumeration.
Borrowed metadata/actions cannot outlive their owner; opaque actions are omitted
from Debug. No cloning of actions, mutable rule cache or new registry dependency.

Config adapts the selected source and actual canonical resource pins into an
allow-listed Serialize-only diagnostic shape. Service/filter revisions retain
their real resource hashes; raw source/approval/provenance text is not exported.
Configured predicted action is separate from D40 resolved intent/typed refusal.
Missing observations remain Unknown; existing declaration kind/administrative
state and current references remain authoritative. Native direct is explicit;
strict refusal never becomes an automatic DIRECT fallback.

DNS Endpoint/Profile association and independent gateway identity are checked.
Gateway diagnostics require exact static grant/group/listener/source/target/port
and selected outbound scope; policy-denied flows still require request scope.
Static scope remains neither ingress authentication nor destination/address
admission. No new permit or diagnostic RBAC token is created from public IDs.
ECH/QUIC/group/NAT/IP-family limitations and unknown context remain explicit.
A supplied QUIC flow contradicting TCP transport is refused, rather than returning
a successful TCP intent. UDP/unknown transport retains D40 unsupported refusal.

Only core Decision assembles the complete SimulationReport from its own immutable
Generation: node/snapshot/source, actual sequence/content hash, config, compiled
tables and pins. Caller-supplied generation labels or policy/resolver objects
cannot enter that constructor. Private fields and no Deserialize prevent report
construction/relabeling through the Rust API. Serialized JSON remains a prediction,
never admission or current-generation proof. Every report fixes
network_test_performed=false/effective_route=null, even with assumed Available.

Private A/AAAA/CNAME output includes selected-Profile owner record IDs/types only.
RDATA, external CNAME resolution, response/TTL/RCODE or private/rule precedence are
not invented; a matching private owner explicitly reports pending_p3 and that the
effective DNS answer is undetermined. Source addresses, device labels, proxy host/
credentials, capabilities, config/resources/signing material and invalid raw input
are excluded from routine output/errors/Debug. No automatic query logging occurs.

No HTTP endpoint, external diagnostic visibility permission, connector, resolver,
network probe or OS-network execution is added. P6 must authorize Profile visibility
before external integration; possession of an internal pin is not user/RBAC
authorization. D11 bytes, D36 activation/recovery, D38/D39/D40 semantics and D43
compiler budgets remain. All manifests, locks, features and existing CI are unchanged.

## Actual local verification

Windows workspace fmt/check, all-features warning-free Clippy, full all-features
and no-default runs passed175 tests each, including three new compile-fail
report/trace authority and lifetime examples. Rustdoc passed. Fresh RustSec fetch
and all5 locked owning manifests passed advisories/bans/licenses/sources with no
new exception; existing duplicate warnings are not suppressed.

Five policy integration groups cover hand-expected rank/source/default/status and
permutation ordering, actual-selector parity, independent TCP ports/Profile,
unknown/mismatched scope/conflict refusal, equal-source enabled-state ordering and
all9998 rule entries at the existing aggregate10000 boundary versus10001 refusal.
Five config groups cover actual catalog hash linkage, service/filter/default,
reordered golden outputs, ECH/QUIC/NAT/IPv4/IPv6 contexts, every static scope dimension
including policy-denied requests, absent/disabled/unavailable/unknown observations,
private A/AAAA/CNAME type-only output/cross-Profile isolation, and a passive trap.
An exhaustive unit table preserves all13 typed D40 refusal codes and safe references.

Core tests activate real CP-built/signed catalogs into actual generations and
compare the complete reviewed report. Failed candidates preserve it, a successful
new generation changes only its own view, and held old reports stay unchanged.
Unknown/mismatched scope and invalid domain return no partial report or raw input.
The existing actual Linux reopen/CP-loss test now also compares complete simulations
before/reopen/failed-candidate; actual execution is pending CI, not inferred locally.

Two real implementation defects were reproduced before their fixes: contradictory
TCP/QUIC assumptions initially returned a selection; same-source entries differing
in enabled state initially changed trace order under permutation. Their regressions
failed on behavior before refusal/stable ordering and pass afterward. Initial golden
capture used the wrong crate-relative temporary path; this was fixture setup, not
a security regression. All temporary capture code was removed before full checks.
The exact reviewed examples are ordinary intended API contract fixtures, not logs.

Eight additional deliberate behavioral mutations were detected and exact original
bytes restored before full checks: false port eligibility, silent trace truncation,
DNS Profile mismatch bypass, request-scope bypass on a denied flow, cross-Profile
private-owner leak, fabricated generation, false network-test claim, and an
introduced loopback connect. The last is detected by a passive nonblocking local
listener; it proves detection of that accidental contact, not product connectivity
or absence of every conceivable side effect. Actual production adapters expose
no connector/probe interface and contain no network operation.

[Output contract/examples](../../api/schemas/explain/README.md) provide strict
JSON Schema2020-12 and reviewed complete/selection goldens. The planning regression
checks independently enforce false/null markers, exact property sets, complete
counts, private type-only fields, catalog hash shape and IP/transport limitations.
No standalone full JSON-Schema validator is installed; formal engine validation is
not claimed and no unapproved package was installed for it. This output-only schema
does not validate untrusted management requests or grant visibility/admission.

Supplementary standards and spec passes were separate read-only agents under the
Rust review skill; both inspected the current diff/new files and actual machine
logs, found no confirmed defect and made no source changes. They do not replace
mandatory linked-account Code Review or actual Linux/external checks.

AST-only graph update completed4490 nodes/9365 edges/296 communities, no API/paid
backend. Missing SQL parser/zero-node Cargo.toml and changed community-label warnings
remain explicit; graph artifacts stay untracked and are supplementary navigation.
Planning passed18 tests,9 negative cases and source preservation; final link totals
are refreshed with this evidence. All88 tasks/61 MVP/188 requirements remain.
No external Codecov/CodeScene, CI or networking success is claimed before observation.
