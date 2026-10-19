# Pure DNS policy names and matchers

D37 uses the qualified idna1.1.0 nontransitional DNS/CDN profile. Parse a
CanonicalDomain before matching; original configuration text stays in the config
schema and signed payload. This crate performs no I/O and grants no network access.

```rust
use droutex_policy::{CanonicalDomain, DomainMatcher, DomainMatchMode};

let base: CanonicalDomain = "BÜCHER.Example.".parse()?;
let query: CanonicalDomain = "child.xn--bcher-kva.example".parse()?;
let matcher = DomainMatcher::new(base, DomainMatchMode::SubdomainsOnly);
assert!(matcher.matches(&query));
# Ok::<(), droutex_policy::DomainError>(())
```

| Mode | Base | Descendants at any depth |
|---|---|---|
| Exact | yes | no |
| DomainAndSubdomains | yes | yes |
| SubdomainsOnly | no | yes |

Matching respects label boundaries; `badexample.com` does not inherit an
`example.com` rule. Names contain no glob syntax. The config wire modes
`exact`/`suffix`/`wildcard` map to the rows above respectively.

Input is at most1024 UTF-8 bytes before library work. Output is lowercase ASCII
without one optional final root dot, at most253 bytes,127 labels and63 bytes per
label. Underscores and non-strict hyphen positions are intentional DNS/CDN support;
the mandatory same-library roundtrip refuses malformed ACE. Errors omit input.
This is not URL parsing, TLS hostname validation, address resolution, DNS wire-name
decoding or gateway destination authorization. Precedence, conflicts, catalogs and
compilation remain later P2 tasks. See the
[accepted contract](../../docs/planning/P2_01_DOMAIN_CONTRACT.md) and
[qualification evidence](../../docs/planning/P2_01_DOMAIN_REVIEW.md).
