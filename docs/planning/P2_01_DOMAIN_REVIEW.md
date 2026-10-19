# P2-01 domain qualification evidence

Status: isolated qualification passed; conditional adoption implemented in draft
[PR12](https://github.com/romanpodg/DRouteX/pull/12). Implementation/final acceptance
is pending, not implied by qualification or owner approval.
Owner-accepted [D37](P2_01_DOMAIN_CONTRACT.md) requires actual Windows/Linux,
dependencies/licenses and independent review before implementation adoption.

The standalone policy-dependencies crate pins idna1.1.0 with explicit
std/compiled_data and all29 already-qualified registry versions/checksums. Local
verification compared every archive SHA256 and every extracted source file against
those archives, then hashed all license/notice files in the
[inventory](../operations/POLICY_DOMAIN_DEPENDENCY_INVENTORY.csv). No registry
version/checksum changed. Local offline cargo-deny advisories/bans/licenses/sources
passed; fresh CI dependency checks also passed at qualification head below.

Windows default/no-default qualification passes8 test groups, fmt and Clippy.
The exact unmodified official Unicode17 corpus contains6391 rows:549 shared
successes,5551 mandatory refusals and289 flag-specific D37 outcomes all checked;
2 unpaired-surrogate rows cannot exist at the Rust str boundary; their corresponding
WTF-8 byte forms are separately refused at the UTF-8/library boundaries. This is
the selected nontransitional profile, not a blanket
Unicode/strict-IDNA conformance claim. CDN/hyphen/underscore cases,1024/253/63/127
bounds, canonical equivalence/idempotence, malformed ACE/bidi/joiners and punctuation
negatives passed locally and in actual Windows/Linux CI.

Qualification head a60b3e4dd60c2ae7ede71f78840489efea0cd84d passed all11 jobs of
[run39156440746](https://github.com/romanpodg/DRouteX/actions/runs/39156440746),
including both default/no-default isolated8-test groups on Windows/Linux and fresh
cargo-deny advisories/bans/licenses/sources. Actual linked-account independent
read-only/never Codex review against6b58b89 completed at unchanged head a60b3e4,
session01a1505d-f3c3-7491-9281-ce5be624a3fa, exit0 with no confirmed defect. Reviewer
formatting/planning and existing test binary passed; read-only build locks prevented
rebuilding/Clippy there. Separate Windows/Linux CI provides actual build evidence.
No production source/root manifest changed before these qualification gates passed.

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

## Implementation candidate

The pure policy crate now owns CanonicalDomain, sanitized DomainError and an
explicit DomainMatcher. Raw strings cannot be matched until parsed. Names are
bounded before IDNA processing and validated after the mandatory library roundtrip;
matching checks label boundaries and arbitrary-depth descendants without expanding
patterns. The only root-lock change is the direct policy->idna edge; no registry
version/checksum changes, feature relaxation, custom IDNA or new license exception.

Config DomainMatch derives the three accepted semantics without changing its
wire shape/text. Custom rules, gateway target names, private record names and CNAME
targets use the same canonical DNS validation. Outbound/socket/TLS admission keeps
its existing checks and later P4 authority. Original Unicode/case/root-dot strings
remain in serialized configuration and authenticated D11 payloads; golden snapshot
data is unchanged. Previously tolerated invalid DNS text is now refused, without
changing signed-format normalization or granting traffic authority.

Six production public-API tests cover the official selected-profile corpus,
Unicode/ACE/nontransitional equivalence, malformed ACE/errors,1024/253/63/127 limits,
apex/child/grandchild/sibling/lookalikes and generated idempotence/depth matrices.
The official-vector checker is shared with the independent qualification crate to
exercise both boundaries against the same immutable expected data. Four config
regressions cover all wire modes, rules/grants/private DNS fields, all three formats
and text preservation. A signed encode/verify/re-encode test proves domain matching
does not rewrite signed bytes; the existing independent snapshot golden still passes.
A compile-fail example prevents raw string queries. Windows whole-workspace fmt,
check/all-targets, Clippy/all-features, default/no-default tests and offline deny
passed, as did isolated default/no-default8-test groups after sharing the checker.
Planning17 tests,9 negative validator cases,238 local links and actionlint passed.
Actual implementation Linux CI, external coverage and linked-account implementation/
final reviews remain pending.

Independent parallel Rust standards/spec source passes found no actionable defect
in the candidate. They ran no tests and do not substitute for actual CI or the
required linked-account Codex review.

Qualification CodeScene review8006853 failed all3 maintenance gates solely on
auxiliary planning/test files: validator9.05->9.02, planning tests9.69->9.66 and
qualifier8.09 (complex/large method and Bumpy Road Ahead). No application source
was changed at that qualification head. The check stays failed; source-scoped
disposition P2-01-CS-AUX is recorded in OPEN_DECISIONS. Implementation CodeScene
results must still be inspected for changed application code, not assumed passed.
