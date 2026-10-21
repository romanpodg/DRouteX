# P2-06 accepted Explain and simulation contract (D44)

Accepted directly by the owner on2026-10-21, conditional on the prerequisite below.
Prerequisite fulfilled before code: PR105 normal merge445f847e74d0b04d32d619be02865c36268648dc, all11 exact-main39552210853 jobs and both processed Codecov reports(Rust5507/5759).

Date: 2026-10-21. The complete owner decision and actual checked prerequisite above authorize implementation; validation/review/merge gates remain required.
Specification sections8.4/18.3, CR-0002 and accepted D38-D43 remain authoritative.

1. Provide pure, typed, bounded Explain/simulation contracts for the CURRENT
   immutable compiled generation and independently selected DNS/gateway scope.
   One pinned core Decision supplies its actual node/scope, monotonic generation,
   source content hash, checked config and compiled tables together. A caller may
   not substitute a supplied generation label or combine another generation's
   policy, catalog pins or associations into this complete report. A lower-level
   generic policy trace remains explicitly an unbound selection diagnostic, not
   an authenticated-generation report or permission. Reopen and held old Decisions
   produce reports from their own immutable generation without CP/database access.

2. Reuse the actual D38 selector and its rank comparison, never implement an
   alternate Explain winner algorithm. Report the selected Profile, DNS Endpoint
   or independent gateway group, plane/decision point, canonical requested name,
   rule source (custom UUID, service-rule UUID plus service revision, filter
   revision, or explicit Profile default), and pinned resource hash for each
   referenced catalog. The reason chain exposes fixed layer, larger priority,
   label count, explicit match mode, port eligibility, and stable source ordering
   ONLY between equivalent effects. A contradictory highest rank remains an error
   and never gets explained as an arbitrary successful winner.

3. Include a deterministic per-entry trace of the selected Profile and selected
   plane only: enabled/disabled, domain match, TCP port match when applicable,
   selected, equivalent tie, or lower rank. An entry means an existing bounded
   prepared rule, so a service domain or TCP interval can appear separately with
   the same source. Use stable semantic ordering independent of input order;
   preserve identical duplicate entries without inventing identities. State how
   many prepared entries were inspected and emitted. Reuse the existing10000
   aggregate prepared-entry cap; no unbounded string reasons or caller-selected
   larger limit, silent truncation, whole-config dump or catalog enumeration.
   Default is explicit when no enabled eligible entry matches. This is a complete
   rule-selection trace, not all steps of a future DNS/network pipeline.

4. Preserve DNS/gateway identity separation. DNS scope must match its existing
   Endpoint/Profile binding; gateway simulation uses a separate group/identity
   and required nonzero actual destination port. Public identifiers and static
   scope comparisons are neither authentication nor live grants. Report network
   group attribution as network/group scope, never a uniquely authenticated
   device behind NAT. IPv4/IPv6 remain distinct provided address/grant facts with
   identical policy ranking, never infer that one family authorizes the other.
   Missing/unknown/mismatched scope refuses without another Profile's trace.

5. Output separately the configured predicted action and D40 resolved intent or
   typed refusal from explicit caller-trusted observation inputs. Missing outbound
   observations remain Unknown; disabled declarations cannot be enabled by an
   observation. Observation input is a simulation assumption, not a probe or proof
   of freshness, connectivity, authenticated ingress or safe checked destination.
   Keep network_test_performed=false and effective_route=null for EVERY pure
   simulation, including an assumed Available protected proxy or VPS_DIRECT.
   Name any predicted exit by typed configured identity/kind and decision point,
   not an invented observed exit IP/location. Strict failures remain refusals,
   never simulated automatic DIRECT. Existing typed D40 transport limitations
   still apply; UDP/QUIC cannot be presented as a successful TCP proxy route.

6. Include typed limitations relevant to supplied mode/transport/context:
   DNS-only routing does not guarantee ECH/QUIC/bypass coverage; a simulated TCP
   decision does not prove the destination name can be recovered from an ECH
   flow; NAT/group evidence does not identify an individual peer; IPv4 evidence
   does not establish IPv6 authorization. Unknown contextual facts stay Unknown,
   not silently converted into support or success. No SNI inspection, DNS lookup,
   CONNECT/TLS/network test, OS-route/firewall execution or live inference occurs.

7. Private A/AAAA/CNAME data remains the separately validated D43 table. Explain
   may state that a canonical owner exists in the SELECTED Profile and identify
   its record IDs/types, but does not export RDATA or follow CNAME externally.
   It must explicitly state private-record response/rule precedence is unresolved
   until P3; it cannot claim the ordinary policy winner is the effective DNS
   answer when a private owner is present. Actual RCODE/TTL/answer generation,
   response precedence and cache behavior retain their P3 gates.

8. Reports are explicit diagnostic artifacts, not automatic query/audit logs.
   Use an allow-listed output shape: canonical requested name, public identifiers,
   source/rank/reason enums, declared actions/intents/refusals, generation/pin
   hashes and limitations. Exclude resolver credentials/capabilities, URLs with
   tokens, private keys, secret/encrypted bytes, proxy username/password/host,
   raw config/catalog/source/approval text and private record values. Source IPs
   and device labels are not echoed. Debug/errors retain existing safe summaries;
   invalid raw domain/input strings do not appear in failures.

9. This stage exposes in-process Rust library APIs and a strict versioned JSON
   output schema/golden examples in api/schemas/explain, using existing qualified
   serialization dependencies only. It creates no HTTP diagnostic endpoint,
   authentication role, CLI network service, log retention or user-facing bypass.
   Complete reports are constructed only from a caller-held pinned Decision and
   checked scope. Holding that local view is not user/RBAC authorization. Future
   P6/P7/API integration MUST authorize the requested Profile before invoking or
   returning diagnostics, including private-owner existence; there is no public
   diagnostic exposure until that integration passes its own gate. Directly
   injecting a client-supplied Profile ID is never a visibility permit.

10. Keep D11 signed bytes, D36 activation/recovery, D38 selection, D39 validation,
    D40 actions and D43 compiler budgets unchanged. No new registry dependency,
    wire configuration field, numeric configuration option, live authority or
    executable feed. Policy owns generic selection trace, config owns existing
    action/catalog adapters and output serialization, core binds actual generation;
    avoid a policy->config cycle or fake freely constructible authority token.

Required evidence: selector/Explain parity on the same compiled generation;
independent expected winner/rank/default cases; deterministic golden JSON under
input/resource permutations; custom/service/filter/default and equivalent ties;
ports, disabled/unknown/mismatched scope and conflict refusal; real typed catalog
revision/hash linkage; distinct DNS/gateway Profiles; ECH/QUIC/NAT/IPv4/IPv6 honest
limitations; private-owner precedence explicitly unresolved; missing/unavailable
outbound strict refusal with effective_route always null; no-network adapter/test
double proving zero network operations; secret/RDATA/invalid-input redaction;
cross-generation/held-old/reopen isolation; bounded full trace and compile-fail
authority/lifetime boundaries. Actual Windows/Linux Q-RUST/deny, independent
implementation/final linked-account reviews, Codecov/CodeScene, normal merge and
exact-main verification remain mandatory. Pure simulation does not pass P3/P4/P6.

Reference facts checked2026-10-21: Control D's Activity Log displays performed DNS
queries with action and Filter/Service/Custom/Default reason/context after Full
Analytics is enabled. Its rules guide documents category priority. These pages
do not establish DRouteX generation-bound pure simulation, an effective network
route, transport admission or access-control behavior. DRouteX's additional
contracts above are owner-accepted DRouteX choices, not vendor claims.
https://docs.controld.com/docs/analytics
https://docs.controld.com/docs/activity-log
https://docs.controld.com/docs/advanced-rules-creation-guide
