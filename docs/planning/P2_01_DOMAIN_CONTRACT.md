# P2-01 bounded domain profile

Status: D37 owner-accepted2026-10-19. Qualification head a60b3e4 passed Windows/Linux
CI39156440746, dependency/license checks and independent linked-account review.
Conditional adoption is implemented; implementation/final-evidence acceptance is pending. P1-06/PR11 merged normally
at6b58b892cf2e9c58499cd60d4ccad9c503a1d656; all11 main jobs39154498259 passed and
both exact-merge Codecov reports processed (Rust3322/3503).

## Accepted scope and limits

Qualify exactly idna1.1.0 with std/compiled_data, retaining all existing29 registry
versions/checksums and approved licenses. CanonicalDomain represents a DNS policy
name, never a URL, credential, socket destination or proxy-authorization proof.

Input is bounded to1024 UTF-8 bytes before library processing. Use nontransitional
UTS46 with URL deny-list, Hyphens::Allow and VerifyAllowRootDot. Require a library
ASCII->Unicode->ASCII roundtrip with successful status and identical canonical
bytes to enforce the already accepted malformed-ACE refusal; no handwritten
IDNA/Punycode or permissive fallback. The raw selected primitive alone fails the
nested-ACE regression; the strict convenience function breaks accepted CDN names.

Canonical output is lowercase ASCII, with one optional final root dot removed;
at most253 bytes,127 nonempty labels and63 bytes per label. Only ASCII letters,
digits, hyphen and underscore are accepted after mapping. Non-strict hyphen positions
and underscore names preserve DNS/CDN compatibility; this does not assert HTTPS
hostname validity. Reject empty/root-only names, empty labels/multiple final dots,
invalid ACE/Unicode/bidi/joiners, controls, punctuation and limits. Do not trim
whitespace, parse URLs/ports/paths, coerce addresses or guess search suffixes.
Single-label/private names are allowed; no public-suffix dependency is added.

## Matching and integration

CR-0002 already requires explicit Exact, DomainAndSubdomains and SubdomainsOnly.
The existing wire modes exact/suffix/wildcard map respectively to those meanings.
Exact matches the base only; DomainAndSubdomains includes base and arbitrary-depth
descendants; SubdomainsOnly excludes the base. Match at label boundaries, so
badexample.com and example.com.attacker never inherit an example.com rule. There
is no glob/regex/substring expansion and no interpretation of server-* syntax.

[Control D custom rules](https://docs.controld.com/docs/custom-rules) documents
bare-domain descendants, apex-excluding *.domain and a broader wildcard grammar.
DRouteX's explicit-mode model is the accepted CR-0002 boundary, not an implicit
import of that grammar. Precedence/conflicts/actions remain P2-02/03/04.

Keep original configuration text in the signed D11 schema. Validation/matching
derive typed canonical names; never rewrite old signed bytes or silently change
snapshot/compiler formats. Catalog imports P2-07 precede P2-05 compilation.
Actual DNS wire-name decoding and gateway destination admission retain P3/P4 gates.

## Evidence required

Official nontransitional vectors under the selected flags; ACE/Unicode, case,
normalization/root-dot forms, sharp-s/final-sigma distinctions, invalid bidi/joiners/
ACE, CDN/underscore behavior,63/253/1024 limits, idempotence/property matrices and
the complete apex/child/grandchild/sibling/lookalike matrix. Qualify every locked
archive, extracted source and license notice; Windows/Linux default/no-default,
fmt/check/Clippy/deny and independent implementation/final-evidence reviews are
required. No privileged networking, new license exception, downloader or paid tool.
