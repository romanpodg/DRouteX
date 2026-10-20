# P2-04 strict action resolution evidence

Status: D40 pure action components implemented; independent implementation review,
actual Windows/Linux CI, external coverage/health assessment and final-head review,
checks and normal merge remain required. Later tasks remain planned.
The prerequisite [PR14](https://github.com/romanpodg/DRouteX/pull/14) merged normally
at a50c6a432c08e1a4422ae625a25b392e54a62d42; all11 exact-main39476373259 jobs passed
and both Codecov reports processed (Rust4071/4274) before implementation began.
See [D40 accepted contract](P2_04_ACTION_CONTRACT.md).

## Implementation and boundaries

The policy crate exposes separate DNS, TCP and borrowed local-answer intents.
Known upstream references yield ordinary DNS intents; Block and compatible gateway
ingress remain distinct. Native outbounds require enabled configuration, explicit
available facts and supported TCP capability. Missing IDs/facts, disabled/unavailable
or unknown observations, unsupported TCP and UDP/unknown requests refuse. A protected
proxy failure becomes denial even when a direct outbound is available. Direct is
eligible only when explicitly selected; no automatic fail-open or schema field.
Client origin, gateway VPS and protected proxy exit decision points are separate.

The config adapter revalidates CURRENT references and reconciles observations
against declared native kind and administrative state. Duplicate, unknown and
kind-mismatched observations refuse; missing observations remain unknown. Inputs
reuse the existing10000 aggregate inventory budget before indexing. Existing typed
actions are consumed owned or borrowed through Borrow; no parallel wire vocabulary,
dependency/feature/lockfile or serialization/signed-format change is added.
Selections preserve Profile/source and typed Endpoint/group provenance. Even a
generic policy deliberately using the same action type on both planes cannot pass
the wrong scope to an adapter. DNS activity cannot switch gateway group/Profile.

Local A/AAAA/CNAME data borrow the actual prepared record and require matching
Profile and canonical owner. Debug omits names, targets and addresses. Classification
does not establish visibility/authentication, full RRset response or precedence.
P2-05 must bind COMPLETE prepared policy/resource checks to the same generation;
P3/P4 still own identity, destination authorization, checked/pinned addresses,
actual probing/freshness/deadlines, TLS, response codes and network failure handling.
No intent alone is a publication, connector/admission permit or proven network result.

## Verification so far

Focused Windows tests passed six policy and six config groups: both native kinds,
every single missing prerequisite crossed with actual TCP/UDP/unknown and gateway
readiness, strict failure while direct is available, deterministic duplicate handling,
exact aggregate bound, A/AAAA/CNAME canonical Profile privacy and borrowed identity,
owned/borrowed actual action adapters with separate Profiles/source/context,
current-reference/kind/admin reconciliation and unsupported future wire kinds.
A deliberate mutation replacing connection-failure denial with direct made the
strict failure regression fail with the expected Deny/ConnectionFailed mismatch;
the source was restored before normal verification. Initial mutation harness used
Windows PowerShell with scripts disabled; that was not a product test. The rerun
inherited the activated toolchain and demonstrated the intended regression failure.
Full Windows Q-RUST passed: fmt, workspace/all-targets default/no-default check,
all-features Clippy/warnings denied,133 tests per default/no-default profile including
the new compile-fail borrowed-record lifetime example, rustdoc and fresh cargo-deny
advisories/bans/licenses/sources. Initial RustSec refresh failed within the restricted
network; the ordinary approved network retry succeeded. Existing duplicate-crate
warnings remain visible. No dependency or license change was introduced.
Q-DOC passed17 tests with new missing-D40/source and unfinished-P2-03 mutations,
9 negative validator cases and258 links;88 tasks/61 MVP/188 requirements/17 ADRs,
2 Accepted, with source specifications unchanged. The first planning test exposed
its old later-work fixture now pointing to the current stage; moved that negative
case to still-planned P2-05, without weakening the execution-boundary validator.
Network fixture unit checks passed4, with the existing explicit Linux-only skip.
Current-head Linux CI, independent reviews and external evidence remain pending.


## Supplementary source review correction

Independent parallel spec review found no confirmed actionable defect. Standards
review identified eager copying of an unbounded borrowed upstream catalog before
checking the aggregate limit. ReferenceCatalog itself has no whole-inventory cap;
the adapter now preflights upstream plus declared outbound counts with checked
subtraction BEFORE reference validation, indexing or collection. Added exact10000
success and10001 refusal at the config boundary. This fixes a concrete allocation
risk; it is not a metric-driven refactor. Read-only standards follow-up confirmed the finding resolved with no other confirmed defect. Affected six config groups and all-features Clippy passed after the fix.
Graphify AST-only update completed locally (3771 nodes/8006 edges); missing SQL
parser and zero-node Cargo.toml warnings remain explicit, no semantic/API backend
or complete graph coverage claim. Generated graph artifacts remain untracked.
