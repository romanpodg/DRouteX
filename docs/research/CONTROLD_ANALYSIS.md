# Control D: DNS product and architecture reference

Research date: 2026-10-15. Scope: official public documentation and the public
ctrld repository README. No paid account, production configuration, packet capture
or service/backend source audit was performed. Vendor descriptions below are
documented behavior, not independently measured implementation evidence.

## What the product separates

Control D separates reusable **Profiles** from **Endpoints**. A Profile contains
filters, service actions, custom domain rules and response/security options.
An Endpoint exposes its own resolver configuration and applies assigned Profiles.
Several Endpoints can reuse a Profile; profile scheduling/composition is supported.
Sources: [Profiles](https://docs.controld.com/docs/profiles),
[Endpoints](https://docs.controld.com/docs/devices).

Filters are category blocklists, either vendor-maintained or third-party. Services
are domain collections for particular applications/sites, with Block/Bypass/Redirect
actions. These are different resources: a service exception can override a category
block. Sources: [Filters](https://docs.controld.com/docs/filters),
[Services](https://docs.controld.com/docs/services).

The documented precedence is custom rules, service rules, filters, then the default
rule. Explicit Bypass can override a lower category; default Bypass is only the
fallback. Security/validation options are not all disabled by Bypass. Multiple
Profiles are combined before category matching. This does not mean each Profile
runs to completion in sequence. Source:
[matching order](https://docs.controld.com/docs/advanced-rules-creation-guide).

Custom rules also supply profile-scoped A/AAAA/CNAME answers for private domains.
Control D's plain domain rule includes subdomains; its wildcard syntax has further
specificity rules. Routira should expose match modes explicitly instead of assuming
the two products' rule languages are interchangeable. Source:
[Custom Rules](https://docs.controld.com/docs/custom-rules).

## Secure DNS, legacy DNS and proxy access are different

Secure DNS uses a personal Resolver ID. The documented unrestricted secure resolver
can be queried from any source IP by someone who knows that secret ID. An optional
authorized-IP restriction adds an IP gate. This is possession of a resolver
capability, not hardware attestation or an administrator account login. Source:
[Require Authorized IPs](https://docs.controld.com/docs/restricted-resolver).

Legacy DNS provides IP-address resolvers for older clients. Its access/profile
selection depends on an authorized source network in the documented legacy setup.
Documentation describes distinct IPv6 handling; do not infer that every legacy
transport has identical allocation/identity behavior. Routira's one-VPS design
cannot assume Control D's address resources. Sources:
[Legacy DNS](https://docs.controld.com/docs/legacy-resolver),
[IP not authorized](https://docs.controld.com/docs/ip-not-authorized).

Control D can learn a source network after a successful secure query. This is a
separate setting; requiring an already authorized IP can prevent learning a new IP
and therefore needs another bootstrap path. Source:
[Authorize by Secure DNS](https://docs.controld.com/docs/auto-authorize-ip).

For legacy roaming networks, Control D also supports DDNS polling and a browser
using secure DNS as a heartbeat. The browser can update the network's observed
address even when a TV/router uses legacy DNS. These mechanisms still require a
capable companion, router/DDNS service or operator action; a bare UDP query does
not prove ownership of a new address. Sources:
[Dynamic DNS](https://docs.controld.com/docs/dynamic-dns),
[Heartbeat Device](https://docs.controld.com/docs/update-ip-via-heartbeat-device).

Transparent proxy access uses source-network authorization separately from DNS
endpoint selection. Control D documents failure when DNS and proxy source addresses
differ, and documents authorizing an IPv6 /64 for its proxy firewall. Routira must
not copy that prefix expansion or claim that a DNS Resolver ID authenticates every
later TCP connection. Source:
[DNS/proxy IP mismatch](https://docs.controld.com/docs/ip-mismatch-between-dns-and-proxy).

Router client discovery and per-client metadata can be supplied by ctrld. Manual
client-name suffixes can separate analytics. Neither a user-supplied suffix nor a
public internet MAC field is independently trustworthy device authentication.
Source: [Device Clients](https://docs.controld.com/docs/device-clients).

## Redirection and deployment lessons

Redirect changes DNS answers toward transparent proxy infrastructure; Bypass uses
ordinary resolution. Control D warns that broad default redirection can break
nonstandard protocols. Its documented automatic/manual location modes use its
distributed infrastructure; a single Routira VPS does not acquire city selection
or global anycast by adopting the UI vocabulary. Source:
[Default Rule](https://docs.controld.com/docs/default-rule).

The public ctrld project is a DNS forwarder with encrypted upstreams, multiple
listeners, LAN discovery and split-domain forwarding. It is not the complete hosted
profile-management and proxy backend. This research uses its documented boundary;
it does not claim access to the hosted implementation. No installer or router
configuration command was executed. Source:
[official ctrld repository](https://github.com/Control-D-Inc/ctrld).

## Changes recommended for Routira

| Reference concept | Routira adaptation | MVP boundary |
|---|---|---|
| Profile and Endpoint separation | Reusable policy, independent named DNS endpoint and rotatable resolver capability | One active Profile per Endpoint first |
| Personalized secure DNS | Native DoH and DoT endpoint configuration, usable from changing IPs | Required G2 feature in the revised proposal; both separately tested |
| Filter categories and Services | Versioned locally imported blocklists and small curated service bundles with provenance | No copied proprietary feed, ML engine or unverified protection claim |
| Rule hierarchy | Explicit category precedence, documented match modes, deterministic conflicts and Explain | Preserve Routira's stricter ambiguity rejection |
| Private DNS | Both personalized encrypted resolvers and profile-scoped private A/AAAA/CNAME records | Private answer does not authorize a VPS connection into that network |
| Automatic address authorization | Opt-in secure-query observations producing bounded, revocable network grants | Manual bootstrap/replacement remains available; no default learning or /64 expansion |
| Proxy destinations | VPS direct egress or one verified protected external proxy | No public explicit proxy inbound, geo network or tunnel requirement |
| Operational visibility | Endpoint/profile/action/rule/dataset revision and actual DNS/proxy source mismatch | Privacy-limited metadata; no invented DNS-to-TCP attribution |

The earlier manual-IP-only proposal was too narrow for the requested experience.
The earlier identity discussion also needs a precise qualification: distinct
personalized secure DNS endpoints can apply different DNS Profiles behind one NAT.
Ordinary transparent TCP sessions still cannot carry that resolver capability;
their network authorization is a separate, coarser boundary. This is an inference
from the documented distinction above, not a claim of per-device traffic control.

The resulting Routira design is in
[DNS_PRODUCT_MODEL](../architecture/DNS_PRODUCT_MODEL.md) and proposed
[ADR-0017 / CR-0002](../architecture/adr/ADR-0017-profile-endpoint-dns-product-model.md).
Research and plan editing were authorized by the user. The concrete proposal is not
silently marked accepted, and no implementation/test gate is passed by this report.
