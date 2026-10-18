# P2-01 domain dependency qualification

Owner decision D37,2026-10-19, authorizes qualification and conditional adoption of
the exact idna1.1.0 profile; it does not pass its tests or adopt a production dependency.
This isolated crate is outside the application workspace. All29 registry packages
and checksums are unchanged from verified main6b58b89; std/compiled_data are explicit.
See [inventory](../../docs/operations/POLICY_DOMAIN_DEPENDENCY_INVENTORY.csv).

Qualification uses Uts46 with URL ASCII deny-list, Hyphens::Allow and
VerifyAllowRootDot, followed by a mandatory same-library ASCII->Unicode->ASCII
roundtrip. The bare primitive accepts a nested ACE label that violates the required
malformed-ACE refusal. The roundtrip rejects it without handwritten IDNA or a
stricter CDN-breaking fallback. Preserve the captured regression and compatibility
positives. Policy bounds/alphabet are independent of URL/hostname authorization.

The fixture is the unmodified official
[Unicode17 IdnaTestV2](https://www.unicode.org/Public/17.0.0/idna/IdnaTestV2.txt),
SHA256 beb5d0be20e896189b03209a82fdc34f06351502bbd4b8e2523583fc2954d9cf.
Its original copyright/header and [Unicode License V3](fixtures/Unicode-3.0.txt)
are retained. The full spec is referenced, not copied. See
[UTS46](https://www.unicode.org/reports/tr46/) for the flag-dependent criteria.
Seven upstream trailing-whitespace lines are retained to preserve the official
fixture SHA; source whitespace checks exclude only this immutable data file.

There are6391 source rows:549 shared successful conversions,5551 mandatory
refusals,289 outcomes derived from the fixture's independent ASCII result under
D37's different hyphen/STD3/root-dot flags, and2 unpaired-surrogate inputs that
cannot exist as Rust str. Their corresponding WTF-8 byte forms are separately
refused at both the UTF-8 and library boundaries. This checks the
selected nontransitional profile, not full Unicode or strict-IDNA conformance.

The older Unicode16 corpus bundled with idna has3 expected-repertoire differences
for newly assigned CJK characters; the newer Unicode18 corpus has9 successes
outside the pinned data repertoire. Those failed exploratory checks are recorded,
not waived as passes or used to silently update dependencies. Unicode17 gives the
qualified pinned behavior; a data/version change requires renewed qualification.

Run fmt/Clippy/test/default/no-default and cargo-deny with this manifest on Windows
and Linux. Production adoption requires actual successful evidence and independent
review; approval alone never passes the gate. No remote downloads happen in tests.
