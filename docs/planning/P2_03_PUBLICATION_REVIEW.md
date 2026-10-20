# P2-03 publication validation evidence

Status: completed in [PR14](https://github.com/romanpodg/DRouteX/pull/14), normally
merged at a50c6a432c08e1a4422ae625a25b392e54a62d42 after final review/checks.
All11 exact-main jobs39476373259 passed and both Codecov reports processed.
PR13/P2-02 merged normally at ecc8a9e9cc903b6264b502cfdc407eb4fd70d5ab; all11
exact-main jobs39296627839 passed and Codecov processed both reports (Rust3638/3828)
BEFORE this implementation began. See the [accepted contract](P2_03_PUBLICATION_CONTRACT.md).
An owner approval alone is not a successful test or review.

## Scope and implementation

The pure policy crate provides complete prepared-set conflict detection using
symbolic canonical suffix-trie atoms and TCP interval events. It reuses D38 rank
and query selection comparison, preserving highest-rank refusal and equivalent
UUID ties without changing serialized config or signed D11 bytes. Entirely
shadowed contradictions pass; partial name/port holes remain blocking.
Near253 bytes, one/two-byte label candidates are exhausted using the qualified
D37 parser; longer space has more lowercase three-byte branches than all bounded
inputs. No wildcard-name or individual-port expansion, handwritten IDNA, I/O,
new dependency/feature/lockfile, async task, unsafe or runtime authority is added.
Active-set versions avoid repeating checks along empty trie paths; effect classes
use Eq once per rank, without Clone/Hash/Ord on opaque actions.

Separate bounded catalog and private-record components use deterministic iterative
graph traversal. Catalog expected hashes are checked even for revisited identities;
typed kinds, missing/duplicate/removed data, license/provenance/validation state,
dependency cycles and aggregate pin/fact/edge budget are explicit. Caller facts
cannot authenticate themselves; actual import/hash/license/provenance construction
remains P2-07. No real catalog import, dataset permission or format is claimed.
Private records use canonical owner/target names, separate Profile keys and RRset
TTL, permitting multiple A/AAAA/equivalent duplicates and external CNAME targets;
mixed/distinct CNAME data and local cycles refuse. Names/actions stay out of errors.

The config boundary revalidates immutable checked input against CURRENT references,
requires pinned catalog data for service/filter selections and gateway services,
and checks private records while preserving original text. Existence-only catalog
IDs and stale import checks cannot satisfy this component. It is deliberately not
a full compiler: P2-05 must bind config/resource checks AND the COMPLETE prepared
policy conflict check to the SAME verified P2-07 data/generation. Neither standalone
Result is publication, signature or network authority. Later tasks stay planned.

## Checks so far

Windows focused checks passed5 conflict groups,6 catalog/private groups and2 config
integration groups. The conflict oracle uses the separately tested D38 evaluator
over complete quotients of a finite fixture grammar in256 variants and reversed
input order, not an implementation-shaped trie oracle. Negative cases cover
partially/fully shadowed ties, interval unions/gaps/65535, nonexistent descendants,
full one/two-byte near-limit alphabets, IDNA/bidi, Profile/plane isolation and9999
prepared TCP rules. Resource tests cover every fact/refusal state, repeated pins
with mismatched hashes, kind separation, reordered graphs, a5000-node catalog chain
at the exact10000 aggregate budget and a10000-record CNAME chain/cycle, TTL/alias
conflicts, independent Profiles, external targets and sanitized diagnostics.
Config integration demonstrates actual stale-reference/unapproved-data/canonical
CNAME refusal with unchanged serialized source text. Initial bidi fixture failure
was an oversized test label, corrected to respect63-byte labels; no product behavior
was relaxed. These tests do not claim full runtime or catalog importing acceptance.

Full Windows Q-RUST passed: fmt, workspace/all-targets default/no-default check,
all-features Clippy/warnings denied, workspace tests in both profiles, cargo-deny
advisories/bans/licenses/sources and rustdoc. Existing duplicate warnings remain
disclosed. Independent parallel standards/spec read-only source passes found no
confirmed actionable defect; they changed no files and ran no Cargo commands.
Q-DOC passed17 tests,9 negative validator cases and250 links, preserving88 tasks,
61-task MVP closure,188 requirements and17 ADRs/2 Accepted. Actual current-head
Windows/Linux CI, external coverage/health assessment,
required final-head review/checks and normal merge remain pending.

## Actual implementation acceptance

At unchanged implementation5fa3a37, actual linked-account read-only/never Code Review
completed with no confirmed defect, session01a15554-9ad7-7932-8118-b80ce4f73375,
exit0. Formatting and13 existing binary tests passed there; rebuild/Clippy were
refused by the read-only Cargo build lock. That limitation is not a successful
rebuild; parent Windows Q-RUST and actual CI separately prove those checks.
All11 exact-head jobs39299342954 succeeded, including Windows/Linux default/no-default
workspace, all existing qualification jobs, fresh deny for the workspace and four
qualification manifests and backend/planning generation/OIDC uploads.
Saved actual logs confirm119 Windows and146 Linux tests per default/no-default
workspace profile and all five four-category deny summaries successful.

Codecov processed both exact-head reports, state complete/sessions2. Rust backend
is4062/4274; composite4519/4734 includes auxiliary Python and is not application
coverage. The exact-base ecc8a9e comparison covers439/462 new Rust lines (95.02%):
config publication49/61, parser3/3, shared ordering15/15, catalog53/56,
conflicts217/221, graph19/20, validation entry16/19 and private records67/67.
Misses include formatting/error-source accessors, the A/AAAA config-adapter arms
before the supplemental fixture below, and defensive/closing lines. They are not
claimed executed at this head. Coverage alone is not a correctness proof.

CodeScene8023118 failed all3 maintenance gates. Source assessment and the three
independent reviews found no confirmed functional defect. conflicts.rs7.03,
private.rs8.96, graph.rs9.24 and catalog.rs9.49 maintenance observations have explicit
impact/rationale/revisit triggers in [OPEN_DECISIONS](OPEN_DECISIONS.md), including
re-review before P2-05 compilation/P2-07 data integration or semantic/bound changes.
The accepted complete symbol/interval reasoning and iterative graph invariants
remain together; no score-only splitting, suppression, weakened tests or green claim.
Test/auxiliary findings9.04/9.69 and validator/planning9.00->8.97/9.64->9.60 remain
outside the owner's independent metric scope; the repository-wide check stays failed.

## Final supplemental regression and next gate

One additional config integration group checks the existing full configuration
fixture: disabled filter and gateway-only service each independently require their
pin; A/AAAA adapters preserve separate families; canonical AAAA owners with different
TTL refuse; successful validation preserves serialization. This closes real scope/
adapter test gaps, not a score-only target. Local fmt,3 config groups and all-feature
Clippy passed; read-only standards/spec follow-ups found no defect. Production
source/manifests/workflows remain byte-identical to5fa3a37. Current-head CI/coverage
and linked-account final review must verify the supplemental test/evidence diff.

The owner accepted [D40 strict actions](P2_04_ACTION_CONTRACT.md) after reviewed
PR14 normal merge and exact-main checks. That is planning authority, not current
P2-04 implementation or a successful test. Later backlog tasks remain planned.

## Final review, normal merge and exact-main acceptance

Final head9516517741e75bc9c6d52edce3b215096a115b0b adds only the supplemental
fixture test and four evidence/planning documents; application source, manifests and
workflows are byte-identical to5fa3a37. All11 jobs39300467355 passed, with actual
Windows120/Linux147 tests per default/no-default profile. Both Codecov reports
processed: Rust4071/4274 and exact-base Rust patch444/462 (96.10%). CodeScene8023237
remained failed with the disclosed application findings plus the supplemental
test's Large Method9.57, within the P2-03-CS-AUX scope/disposition. No suppression.

The first final linked-account attempt exited1 on quota; no successful review was
claimed. A PR-body update was also interrupted by automatic approval quota review
without execution. After owner continuation, a fresh normally approved request and
read-only/never final review completed: session01a15a98-5e07-7d20-b353-7a25b59703da,
exit0, no confirmed defect. Formatting and three existing binary groups passed;
read-only rebuild limitations are separate from actual parent and CI evidence.

PR14 merged normally2026-10-20T18:37:24Z at a50c6a4, without force/admin merge or
check/protection changes. Exact-main run39476373259 passed all11 jobs; Codecov
processed both reports, Rust4071/4274. These gates completed before D40/P2-04 code
began. The owner choice to wait for ordinary statuses was honored throughout.
