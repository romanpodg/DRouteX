# P2-03 publication validation evidence

Status: D39 owner-accepted; implementation candidate under review, not merged.
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
required linked-account implementation/final reviews and normal merge remain pending.
