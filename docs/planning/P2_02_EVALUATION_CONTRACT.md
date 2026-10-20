# P2-02 deterministic scoped evaluation contract

Status: D38 owner-accepted2026-10-19 by direct reply accepting the complete local
proposal after PR12 merge. PR12 merged normally at6fa4759, all11 exact-main jobs
39291553252 passed and both Codecov reports processed before implementation began.
P2-02 merged normally at ecc8a9e after linked-account implementation/final reviews,
all11 final-head39295759968 and exact-main39296627839 jobs. Both Codecov reports
processed; no owner exception, admin/force or check bypass.

## Accepted foundations and reference

Custom > service > filter > default, one active Profile per DNS Endpoint,
independent gateway network-group Profile, canonical D37 domains, no implicit
Control D glob grammar. Explicit BYPASS stops lower rule layers; default BYPASS
is only a fallback. Neither removes security, authentication, destination admission,
DNSSEC or quota checks. DNS upstream selection never chooses traffic egress.
Control D documents those category/BYPASS concepts at
https://docs.controld.com/docs/advanced-rules-creation-guide and
https://docs.controld.com/docs/profiles . It does not establish this proposed
numeric/specificity comparator; DRouteX must select its own deterministic contract.
Multiple enforced Profiles, Geo rules and arbitrary wildcard grammar remain outside
the accepted DRouteX MVP. No reference changes owner-accepted scope.

## Accepted observable comparator

1. Fixed category rank: custom, then service, then filter; default only if no
   enabled applicable explicit rule matches.
2. Within the same category, larger signed i32 priority wins. The existing full
   i32 wire range remains accepted; compare directly, never subtract/negate values.
3. Equal category/priority: more canonical base labels wins. At the same label
   count, Exact > SubdomainsOnly > DomainAndSubdomains, because these successively
   constrain the matched name set. Thus `child.example.com` wins over a same-priority
   `example.com` descendant matcher; for the same base, descendants-only wins over
   base-plus-descendants on a child query, and exact wins on the base query.
4. Ports are eligibility constraints, not an extra hidden priority rank. Matching
   traffic scopes require the actual nonzero TCP destination port; no DNS rule gets
   a fabricated port and no unknown protocol/metadata is guessed. Overlapping port
   rules with otherwise equal winning keys follow the tie rule below.
5. Stable resource UUID bytes ascending choose only among equivalent selected
   effects. Different effects at an otherwise equal winning key are an ambiguity
   error; stable ID never conceals a contradiction. No built-in BLOCK-over-BYPASS,
   rule-list order or HashMap iteration preference.

Example: a broad custom rule at100 overrides an exact custom rule at50, because
explicit priority precedes specificity. A service rule at i32::MAX cannot override
a matching custom rule at i32::MIN. Explicit DNS resolve/BYPASS beats lower filter
rules; default resolve/BYPASS does not disable filters. These are selection results,
not permission to skip later security gates.

## Scope, bounds and staged implementation

DNS consumes only its selected Endpoint/Profile binding, traffic only its selected
network-group/Profile binding. Do not union Profiles/grants, infer gateway identity
from DNS, or treat copied IDs/metadata as authentication. P3/P4 retain actual runtime
admission. Evaluator output is not a proxy/destination authority or live listener.
Queries use D37 CanonicalDomain and explicit DNS/TCP shapes; raw names/URLs do not
enter matching and unsupported metadata is refused rather than guessed.

Keep the pure Rust policy layer free of I/O, network/DB/global state and new registry
dependencies. Bound prepared rules using the existing10000 aggregate configuration
rule/resource ceiling; no wildcard expansion. Catalog payload importing/revision
qualification remains P2-07, full compiler/snapshot format remains P2-05. A typed
prepared-rule selection primitive can evaluate the existing categories without
inventing catalog payloads or adopting a downloader. Actions remain typed/separate
DNS and traffic choices; P2-04 owns capabilities, failure policies and final effects.

P2-02 refuses contradictory highest-key matches at evaluation; P2-03 separately
adds comprehensive publication-time conflict/dependency/private-CNAME diagnostics.
No warning-only unsafe publication or arbitrary stable-ID resolution is introduced
as an interim behavior. Keep original config text and signed D11 format unchanged.
Explain/simulation integration remains P2-06; selection must preserve stable source
identity to support that later task.

## Acceptance evidence

Permutation tests over input order and maps; cross-layer extreme priorities; exact/
descendant/sibling and Unicode/ACE cases; disabled rules; same-key equivalent versus
contradictory effects; actual/overlapping/missing TCP ports; default versus explicit
BYPASS; DNS/network scope separation. Exercise existing10000-rule/input limits and
sanitized typed failures. Windows/Linux default/no-default Q-RUST, dependency checks,
actual linked-account independent implementation/final reviews and scoped Codecov/
CodeScene assessment remain required before normal merge. No production-root tests,
new license exception, paid integration, silent architecture change or skipped gate.

## Prepared API representation

The pure PolicySet owns immutable Profile rules and checked endpoint/group maps.
DNS queries require the registered enabled Endpoint/Profile binding; TCP uses a
separately established gateway identity's group and actual NonZeroU16 port. Metadata
associations are not authentication; generation/current grant verification remain
with callers at P3/P4. Unknown, disabled or mismatched scopes fail closed.

DomainRule caches its immutable rank once; queries make two allocation-free passes:
find the globally highest rank and least source identity, then reject contradictory
effects at that rank. Lower-ranked contradictions cannot conceal a higher winner.
Stable source keys use resource UUID bytes, then service revision UUID bytes only
when the rule UUID is equal and effects equivalent. No new identifier is minted.
Full publication diagnostics and contradictory catalog provenance remain P2-03/07.

One TCP interval per prepared entry avoids expanding individual ports; OR conditions
use entries with the same source/effect. Profiles, Endpoint/group associations and
all prepared entries (including disabled entries) share the10000 aggregate budget.
No hidden narrower-port-range priority or wildcard expansion is introduced. This
is an input-selection primitive, not an import/compiler API that may skip unresolved
catalog references. Catalog provenance IDs alone prove neither data nor license.

The two action type parameters preserve the existing distinct config DnsAction and
TrafficAction without duplicating their effects or introducing a cyclic crate
dependency. Comparable opaque effects are borrowed in results and never executed
or formatted; actual capability/failure resolution belongs to P2-04. Original
config serialization and signed D11 bytes stay unchanged. No new registry
package, feature or lockfile change is required.
