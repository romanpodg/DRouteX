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
decoding or gateway destination authorization. Catalog imports, publication-time
conflict/dependency diagnostics and full snapshot compilation retain later P2 gates. See the
[accepted contract](../../docs/planning/P2_01_DOMAIN_CONTRACT.md) and
[qualification evidence](../../docs/planning/P2_01_DOMAIN_REVIEW.md).

## Scoped prepared selection (D38)

PolicySet owns bounded immutable Profile rules and DNS Endpoint/gateway group
associations. Evaluate DNS using a current registered Endpoint/Profile binding;
evaluate TCP using an independently established GatewayNetworkIdentity and actual
nonzero destination port. These public metadata types do not authenticate traffic.
Unknown/disabled/mismatched scopes fail; DNS history cannot select a gateway Profile.

Fixed custom > service > filter ranks precede larger numeric priority, then more
base labels and Exact > SubdomainsOnly > DomainAndSubdomains. Equal winning ranks
with different effects error; ascending source UUID bytes select only equivalent
effects (service revision UUID is secondary for equal rule UUIDs). The default is
used only when no explicit rule applies. Port intervals affect eligibility only;
no implicit strongest-action or narrowest-port preference exists.

Prepare DomainRule from explicit RuleMetadata, a D37 DomainMatcher and a typed
opaque action; TcpRule additionally has Any or a validated inclusive TcpPortRange.
An OR of intervals uses repeated source/effect entries, never individual-port
expansion. Profiles, endpoint/group associations and all prepared entries including
disabled entries share the10000-entry budget. Source IDs prove neither catalog
verification nor licenses. Full catalog preparation/compilation is deliberately
not exposed here; callers must not skip unresolved references.

Action type parameters preserve distinct existing DNS/traffic schema types without
a cyclic dependency. Selection borrows the effect from its policy owner, never
executes/formats it, and preserves Profile/scope/source IDs for later Explain work.
Ranks are cached once; queries use two borrowed iterator passes without allocating
or cloning actions. Actual authorization, capabilities, failure handling and
snapshot generation remain at their owning gates. See the
[accepted evaluation contract](../../docs/planning/P2_02_EVALUATION_CONTRACT.md) and
[evidence](../../docs/planning/P2_02_EVALUATION_REVIEW.md).

## Offline publication validation (D39)

PolicySet.validate_conflicts checks reachable highest-rank contradictions across
the complete prepared DNS/TCP set; entirely shadowed contradictions are allowed.
Canonical suffix-trie atoms cover exact names and possible unlisted descendants,
including finite name bounds and qualified-parser bidi checks near the byte cap.
TCP uses interval events and opaque Eq effect classes without enumerating ports.
Errors preserve Profile/plane/source IDs; no effect or private name is formatted.

validate_private_records consumes PreparedPrivateRecord values, checks canonical
Profile-private RRsets/TTL and CNAME coexistence/targets/cycles with iterative graph
traversal. Absent targets remain external without looking in other Profiles.
validate_catalog_dependencies requires typed hash pins and explicit trusted
license/provenance/validation facts for every selected transitive revision. Actual
data import, hashing, retained provenance and license evidence remain P2-07; these
public facts cannot authenticate themselves or approve a dataset. Existing10000
entry bounds apply before graph construction, including supplied dependency edges.

The config crate separately rechecks CURRENT references and private/catalog inputs.
P2-05 must bind both component checks and the complete compiled policy to the SAME
verified generation. No Result here is signing, network or publication authority.
See [D39](../../docs/planning/P2_03_PUBLICATION_CONTRACT.md) and
[evidence](../../docs/planning/P2_03_PUBLICATION_REVIEW.md).
