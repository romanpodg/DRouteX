# P2-01 domain qualification evidence

Status: isolated qualification candidate; no production dependency adopted.
Owner-accepted [D37](P2_01_DOMAIN_CONTRACT.md) requires actual Windows/Linux,
dependencies/licenses and independent review before implementation adoption.

The standalone policy-dependencies crate pins idna1.1.0 with explicit
std/compiled_data and all29 already-qualified registry versions/checksums. Local
verification compared every archive SHA256 and every extracted source file against
those archives, then hashed all license/notice files in the
[inventory](../operations/POLICY_DOMAIN_DEPENDENCY_INVENTORY.csv). No registry
version/checksum changed. Local offline cargo-deny advisories/bans/licenses/sources
passed; fresh CI checks remain required.

Windows default/no-default qualification passes8 test groups, fmt and Clippy.
The exact unmodified official Unicode17 corpus contains6391 rows:549 shared
successes,5551 mandatory refusals and289 flag-specific D37 outcomes all checked;
2 unpaired-surrogate rows cannot exist at the Rust str boundary; their corresponding
WTF-8 byte forms are separately refused at the UTF-8/library boundaries. This is
the selected nontransitional profile, not a blanket
Unicode/strict-IDNA conformance claim. CDN/hyphen/underscore cases,1024/253/63/127
bounds, canonical equivalence/idempotence, malformed ACE/bidi/joiners and punctuation
negatives passed locally. Linux/independent review have not yet run.

The bare selected to_ascii primitive accepts nested xn--xn--a--gua.pt, contrary
to the required malformed-ACE refusal. A captured negative preserves that fact.
The same-library ASCII->Unicode->ASCII roundtrip rejects it while preserving the
accepted compatibility positives. No handwritten IDNA, changed hyphen policy,
library patch or per-vector waiver was introduced. Independent source investigation
confirmed the missing decoded-prefix validation path and the library-only remedy.

Initial exploratory Unicode16 tests failed3 cases for newly assigned CJK names;
Unicode18 tests failed9 successes outside the pinned data repertoire. The versioned
Unicode17 corpus matches the selected fixed dependency set. These observations are
retained as failures, not silently counted as passed vectors or used to update
dependencies. Library/data/feature changes require renewed qualification.

The official fixture's copyright/header and Unicode License V3 are preserved;
source URL, SHA and exact test scope are in the
[qualification README](../../tests/policy-dependencies/README.md). No test download,
paid service or privileged operation is added. Later production matching must
still prove the accepted apex/subdomain matrix and config/snapshot integration.
