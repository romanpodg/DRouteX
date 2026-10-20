# P2-03 publication validation contract

Status: D39 owner-accepted2026-10-19 by direct reply accepting the complete local
proposal after PR13 merge. PR13 merged normally at ecc8a9e; all11 exact-main
jobs39296627839 and processed Codecov were verified before P2-03 implementation.
Implementation/final reviews and actual P2-03 Windows/Linux checks remain required.

## Existing authority and gap

CR-0002 requires unresolved contradictory rules to prevent publication, independent
DNS Endpoint and gateway group scopes, pinned licensed catalog revisions and
Profile-private A/AAAA/CNAME data. D37 supplies canonical names; accepted D38 supplies
fixed layer/priority/specificity ranking and evaluation-time refusal of conflicting
highest-ranked effects. P2-03 supplies publication-time validation, not a new rank.
P1-02 already checks references, IDs, port/domain syntax and legacy/gateway collisions;
its ReferenceCatalog proves existence only, not licensing or catalog bytes.

Control D remains the category/BYPASS reference:
https://docs.controld.com/docs/advanced-rules-creation-guide (checked 2026-10-19).
Its broad wildcard grammar, combined Profiles and displayed list order are not
DRouteX requirements. CNAME and RRset constraints are informed by RFC1034 section3.6.2
and RFC2181 sections5.2/10.1, not an authorization to change DNS runtime scope.
https://www.rfc-editor.org/rfc/rfc1034
https://www.rfc-editor.org/rfc/rfc2181

## Accepted consequential choices

1. Reject a same-Profile/same-plane contradiction if there exists any canonical
   name (and actual TCP port for TCP) where different effects tie at the highest
   applicable D38 rank. A conflict covered entirely by higher-ranked rules is not
   an unresolved effective conflict and does not itself block publication.
   Detect overlap symbolically over the finite D37 suffix/mode and TCP interval
   conditions, including the 253-byte/127-label name limits. Do not invent sample
   names as proof of absence, enumerate ports or expand wildcard names. Disabled
   rules do not participate in effective matching but still count toward bounds
   and must retain structurally valid references. Equivalent actions are permitted.
   Stable IDs diagnose issues; they never choose between contradictory effects.
   Conservative rejection of every overlapping contradictory pair would be simpler
   but would reject configurations whose higher-ranked rule resolves every query;
   this alternative needs explicit owner selection rather than silent adoption.

2. Canonicalize private record owners/targets using D37, keyed by Profile plus name.
   Multiple distinct A/AAAA answers are permitted; each RRset uses a consistent TTL.
   Equivalent duplicate records with the same TTL do not create an ambiguity.
   A canonical owner cannot combine CNAME and A/AAAA or distinct CNAME targets;
   reject self-aliases and all local alias cycles. Use iterative graph validation,
   bounded by the existing aggregate input ceiling, with no recursion-based depth
   overflow or arbitrary new chain limit. A CNAME target absent from this Profile's
   local table is external, not a missing local node; it may resolve later via the
   normal resolver path. Never look it up in another Profile or claim connectivity.
   DNS runtime query/response limits and actual external resolution remain P3 gates.

3. Validate every selected/pinned catalog reference and dependency against explicit
   trusted revision facts (kind, identity, content hash, provenance, approved usable
   license/validation state). Unknown, removed, hash-mismatched, unapproved or invalid
   revisions and dependency cycles block publication, with stable involved IDs.
   Absence of facts is unavailable, not an empty successful protection list.
   This validation cannot authenticate arbitrary caller facts: actual import/hash/
   license provenance construction stays P2-07 and later compiler P2-05 must consume
   that verified output. No internet downloader, new format, feed, library, dataset
   license exception or silent bypass of unresolved dependencies is authorized.

4. Reuse P1-02 resource/scope checks for removed referenced outbounds, Profiles,
   listeners and ambiguous source/listener grants, including IPv4/IPv6 separation.
   Preserve separate DNS/gateway validation and reject union/last-query inference.
   Publication orchestration consumes the config and policy validators; avoid a
   config/policy dependency cycle or treating a parsed document as publication proof.
   No signed D11 schema change, snapshot compiler, API service or network execution.

5. Reuse existing 10,000 aggregate resource/prepared-entry and canonical-name bounds;
   no new configurable numeric limit or registry dependency. Validation is pure,
   deterministic and independent of DB/network. Return a deterministic typed first
   blocking diagnostic with stable involved resource IDs and Profile/plane context;
   source names, actions/secrets and unbounded raw document content stay out of
   errors. Equivalent duplicate records retain deterministic source identity.
   Complete rich/multiple diagnostic and Explain output remains P2-06.

## Evidence required before implementation acceptance

Reordered-input invariance; exact/apex/descendant/Unicode and overlapping TCP cases;
partially versus fully shadowed conflicts; disjoint Profiles/planes; boundary names
without impossible descendants; disabled entries; multi-address RRsets/TTL mismatch;
CNAME canonical aliases/cycles/long bounded chains/external and cross-Profile names;
removed references, missing/unapproved/mismatched catalogs and dependency cycles;
maximum input size and sanitized errors. Meaningful independent oracle/selection
checks must validate symbolic conflict detection rather than mirroring its code.

Windows/Linux default/no-default Q-RUST, dependency/license checks, linked-account
independent implementation/final Code Reviews, actual scoped Codecov/CodeScene and
normal PR merge remain required. No approval is a passed test. Serious unresolved
findings block merge. Later work and changed consequential choices retain their gates.

## Staged component representation

PolicySet.validate_conflicts checks its entire bounded prepared rule set using
suffix-trie atoms and TCP interval events. Exact node names and unlisted next-label
branches partition the canonical namespace; the253-byte/127-label constraints can
make descendant atoms empty. With room for three bytes,26^3 ordinary lowercase LTR
labels exceed all10000 possible listed children. With room for only one/two bytes,
the finite ASCII label alphabet is exhausted and the qualified D37 parser checks
bidi/context validity; no sampled-name absence or handwritten IDNA is used.
Private immutable D38 ranks are reused, with the query evaluator's highest-rank
logic shared rather than replacing its accepted comparison semantics.

PreparedPrivateRecord uses parsed canonical names and explicit A/AAAA/CNAME values.
validate_private_records checks RRsets and a Profile-local graph; Profile existence
is separately rechecked at the config boundary. The existing10000 records bound
also bounds derived graph nodes/edges. CatalogPin contains a typed revision and
expected32-byte hash; CatalogRevisionFacts carries caller-trusted actual hash,
license/provenance/validation state and dependency pins. Supplied pins/facts/edges
share the existing10000-entry ceiling before maps are built. Identity kind is part
of lookup; duplicate facts are refused. Each expected hash is checked even for an
already reached identity. Only selected transitive data is admitted; unselected
unavailable license facts grant nothing. Actual hashing/import/license evidence
and retained provenance construction remain P2-07; public fact fields do not
authenticate themselves or approve a dataset license.

CheckedConfiguration.validate_publication_resources rechecks existing resources,
scope assignments and references against CURRENT inventory, requires pins for all
selected service/filter revisions and gateway service grants (including disabled
selections), then checks catalog dependencies and canonical private records.
It does not invent catalog conditions or compile policy. P2-05 MUST bind this
config/resource check AND validate_conflicts on the COMPLETE prepared generation
to its actual verified P2-07 data. Neither standalone Result is publication,
signature or runtime authority. Existing D11 bytes and serialized config remain
unchanged. Error variants carry bounded typed IDs and context, not names/effects.
