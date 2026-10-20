# P2-04 action and failure decision contract

Status: D40 owner-accepted2026-10-20 by direct reply accepting the complete local
strict profile after PR14 merge. The prerequisite was satisfied by normal PR14 merge a50c6a4, all11 exact-main
CI39476373259 jobs and both processed Codecov reports before implementation.
D40 authorizes P2-04 only; later technical choices retain their owning gates.

## Accepted foundations

CR-0002/D38 separate DNS Endpoint/Profile from gateway group/Profile and separate
DNS upstream choice from traffic egress. Explicit BYPASS stops lower rule layers,
not authentication, DNSSEC, SSRF, destination authorization or quotas. D39 validates
publication inputs; P2-05 binds complete compilation and verified catalog data.
The native MVP exposes VPS_DIRECT and HTTP CONNECT inside verified TLS only.
Tunnels, engines, node routing and privileged networking are post-MVP.
Control D category/BYPASS reference, checked2026-10-19:
https://docs.controld.com/docs/advanced-rules-creation-guide
Vendor wildcard/combined-profile behavior does not override DRouteX's accepted model.

## Accepted bounded profile

1. Add pure typed action-resolution components, not a connector/DNS server. Consume
   existing distinct selected DNS/traffic action values and explicit caller-trusted
   current capability/availability facts. Unknown, disabled, unavailable, missing
   or inconsistent outbounds/capabilities cannot become an available route. Public
   facts cannot authenticate themselves; actual probing, freshness/deadlines,
   grant/binding/address checks, pinning and connection errors remain P3/P4.
   Keep provenance/Profile/decision point available for later Explain; no fabricated
   connectivity, health or network-test result.

2. DNS Resolve/BYPASS yields an ordinary DNS/upstream intent, Block a DNS-denial
   intent, and Gateway a compatible SmartDNS gateway intent requiring independently
   established destination binding. DNS intents do not select a client's TCP egress
   or prove ECH/QUIC compatibility. Return explicit unavailable/unsupported outcomes
   when the required gateway/transport facts are absent. DNS wire response/RCODE,
   DNSSEC and actual redirect addresses/HTTPS-SVCB behavior retain P3-04/05/06 gates.
   Profile-local A/AAAA/CNAME values remain separate local-answer data, never a
   proxy target grant. Do not invent private-record versus rule precedence in this
   action component; P3-04 owns response application, while P2-05 retains separately
   validated record tables. Preserve their distinct typed classification/intent.

3. Traffic Deny stays denial. Explicit administrator-selected VPS_DIRECT names the
   gateway VPS as its decision point; DNS BYPASS normally leaves origin connection
   at the client, and a proxy's further exit is a distinct decision point. The
   selected protected route is HTTP CONNECT over verified TLS for actual TCP only.
   UDP/unknown transport and tunnel/node/engine capabilities stay unsupported;
   no arbitrary action string, target protocol or fabricated future resource is
   silently mapped to a supported native outbound.

4. Accepted MVP failure profile: strict protected-proxy failure always denies.
   There is NO automatic fail-open/fallback in this slice. Explicit direct opt-in
   is an administrator rule/default deliberately selecting the existing VPS_DIRECT
   outbound, not a fallback inferred from proxy loss. Future automatic fail-open
   would need explicit Profile permission plus actual audit implementation and its
   own owner decision; the present schema has neither, so it cannot be smuggled in
   as a Boolean or treated as already authorized. No new config field or silent
   signed-D11 incompatibility is proposed. Alternative: design explicit fail-open
   permission/audit contracts before P2-04 implementation, with added schema and
   security/testing scope; it must not be chosen without owner approval.

5. No new registry dependency, config serialization, numeric limit, catalog format,
   network execution or privileged process. Reuse bounded current inventories and
   typed IDs, sanitized errors and ownership; no plaintext credentials in decisions
   or Debug. Unsupported capability/failure produces explicit refusal, never an
   empty successful route. P2-05, P2-06 and P3/P4 keep compiler, Explain, actual
   response/admission/transport and live failure integration gates.

## Required evidence

Distinct DNS/traffic/local/private intents and decision points; missing/disabled/
unknown/unavailable capabilities and outbounds; actual TCP versus unsupported UDP/
unknown/node/tunnel input; explicit direct selection versus failed protected route;
no DNS-to-gateway authority upgrade, unchanged configuration/signed vectors,
reordered bounded inventories and redacted diagnostics. Future live integration
must prove real strict failures; pure unit tests cannot claim that evidence now.
Windows/Linux default/no-default Q-RUST, deny/license checks, linked-account
independent implementation/final reviews, real Codecov/CodeScene assessment and
normal PR merge remain required. Serious unresolved defects block merging.
