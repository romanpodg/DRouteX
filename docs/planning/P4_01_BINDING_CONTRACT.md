# P4-01 bounded runtime authorization contract (D46)

Status: **Accepted by the owner on2026-10-22**, via direct chat reply accepting the
complete profile in .setup/p4-01/binding-design-proposal.md after checks. D45 order
P4-01 before P3-01 was separately accepted. P2-06 prerequisite is fulfilled:
normal merge671c9f6154add92486c90f5a620d4dbefa9a510a, all eleven exact-main39662936089
jobs and both processed Codecov reports(Rust5980/6262). No implementation began
before these decisions. Adoption requires the platform/dependency/review gates below.

CR-0002, D08/D11/D36/D38-D44 and the original specification remain authoritative.
This accepted profile includes the explicit staged learning boundary in section7.

## Product reference and deliberate DRouteX behavior

Control D documents Endpoint-scoped authorized source IPs for Legacy DNS, optional
learning following successful Secure DNS, and manual bootstrap when unknown IPs are
already restricted. Disabling learning stops new observations; it does not itself
describe removal of already authorized IPs. Sources checked2026-10-22:

- https://docs.controld.com/docs/ip-management
- https://docs.controld.com/docs/auto-authorize-ip
- https://docs.controld.com/docs/restricted-resolver

DRouteX already accepted a stricter split: secure DNS selects its own Endpoint;
gateway authorization has a separate group/Profile/target/port/outbound scope.
Learned input must not directly mutate authority. Bounded leases, clock handling,
no implicit gateway grants and the limits below are DRouteX proposals, not claims
about Control D implementation or its numeric limits.

## 1. Immediately useful implementation boundary

Implement shared current authorization in droutex-core, beside the Deployment
which owns durable activation, rather than create an unused gateway crate. Keep
generic identity/policy types and the existing static associations pure. This
component checks actual peer context provided by its trusted in-process transport
caller; a caller-supplied public ID, arbitrary DNS header or SNI is not proof.

P4-01 adds no listening socket, connector, origin resolver, TLS/token verifier,
HTTP API, privilege, system networking operation or production exposure. P3-01
will bind real DNS transports to this gate; P4-02/03/04 will add checked origins,
passthrough and actual stream lifetime. Those later network checks remain open.
No new external dependency or crypto primitive is proposed.

## 2. Signed lease data and compatibility

Retain strict schemaVersion1 as an association-only legacy configuration. Its
canonical bytes, hashes, D11 signatures and retained restore vectors must remain
unchanged. It never grants runtime DNS/gateway admission, even if static scope
matches. Do not silently turn an old static grant into an indefinite lease.

Introduce strict schemaVersion2 for lease-bearing configurations. Each legacy and
gateway grant must include a typed lease with integer Unix-second issuedAt and
expiresAt, and an explicit cause. The initial accepted cause would be manual.
Use checked integers in [0,253402300799], no floating/string timestamps, scheduled
activation, implicit now, absent expiry, wildcard duration or renewal on access.
Require 60 <= expiresAt-issuedAt <= 86400 seconds. Time structure is checked at
config/CP/DP boundaries; runtime checks the actual interval on each admission.
An already expired lease may remain valid historical data but cannot admit traffic.
Existing10k aggregate entries,1MiB configuration,32-depth/8MiB D11 and64MiB generation
limits remain. Leases are part of their existing grant entry, not an extra cap.

Use the existing authenticated D11 header layout/hash/Ed25519 algorithms, with
payload-schema discriminator2 for schemaVersion2; discriminator1/configuration1
stay byte-for-byte compatible. Refuse mixed or unsupported versions. Old software
must reject v2; new software may restore exact v1 for diagnostics/static history,
but no live admission. Never rewrite or re-sign an old envelope automatically.
Signing/trust keys and replay floors do not change or reset during this upgrade.
v2 publication still requires complete D43 compilation and a fresh higher sequence.

## 3. Exact scope and ambiguity

Derive legacy DNS binding from the current enabled listener/Endpoint and exactly
one current unexpired legacy grant for (listener,actual peer IP). Use the current
Endpoint's Profile. UDP and TCP are separate listener grants, with shared group
quotas below. Deny unknown, disabled, expired or ambiguous cases without another
Profile's private information. Recheck every TCP DNS query, not only connection setup.

For gateway requests independently derive exactly one group from the current
gateway listener/actual peer IP. Match that grant's exact address, canonical target,
nonzero TCP port and policy-selected outbound. Select the group's traffic Profile
using the real D38 evaluator; Deny or unavailable D40 intent stays denied. SNI is
only a candidate name. The caller cannot choose an alternate Profile/outbound by
supplying a DNS Endpoint or requesting a more permissive grant ID. Existing service
scope uses only its revision's own domains, with existing D43 catalog hashes.

Overlapping active grants for a source/listener are rejected, even if nominally
equivalent; never union permissions or choose by last query/UUID. Multiple targets,
ports and outbounds belong in one explicit gateway grant. IPv4 and IPv6 are distinct
exact addresses; no prefix expansion or mapped-address alias authorizes both.
Socket-family normalization must be explicit at P3/P4 transport integration.
Internal/metadata/self target address checks, independent origin resolution and
checked-address pinning are still mandatory P4-02 checks, not established here.

## 4. Publication, replacement and revocation

Manual bootstrap is a trusted controller-side operation which builds lease-bearing
configuration, runs full publication validation and signs it. It is not a public
registration endpoint; operator/session/role verification and actor audit belong
to P6. A request to replace an address removes the old grants and inserts explicitly
selected new DNS/gateway scopes in one signed generation. Do not copy a DNS Profile
into gateway policy or implicitly publish both planes. Revocation is removal in a
fresh generation; it is acknowledged only after durable activation succeeds.

New admissions always use Deployment's current admitted generation. An old Decision,
old report, DNS cache entry or previously successful check cannot create a new flow.
Activation and current authorization must have a documented linearization point;
no stale-generation permit can be consumed after an applied replacement/revocation.
Typed results have private constructors, no Deserialize/Clone permission upgrade,
and are single-attempt scope evidence, not origin/connect permission. P4-02 must
recheck current authorization at its final connect-admission boundary. P3-01 likewise
rechecks current admission per request. Test the corresponding core race now;
actual socket/connector races remain owning-stage integration evidence.

An already admitted bounded DNS request may finish on its pin. Policies for actual
established TCP stream termination/maximum lifetime remain P4-04; this component
must not claim it has terminated a network stream. No old-pin grace admits a new flow.
Corrupt/new-candidate failure retains the still-valid last-good generation; it
never extends that generation's leases. CP/database loss likewise cannot renew them.

## 5. Clock contract and durable non-extension

Operational prerequisite: the node operator supplies a correctly synchronized
host UTC clock. This component does not prove UTC accuracy, implement NTP or defeat
a malicious administrator/whole-volume rollback; D36's external-anchor exclusion
continues. It must detect and refuse ordinary discontinuity, uncertainty and durable
state failure rather than silently lengthen a lease.

Use std SystemTime and Instant internally, with a monotonic anchor and an explicit
2-second uncertainty allowance. Compare wall time with anchor UTC plus monotonic
elapsed time. If their difference exceeds2s, time goes out of range or arithmetic
fails, latch admission unavailable. For an available clock, require the entire
conservative interval [min(wall,predicted)-2s,max(wall,predicted)+2s] inside
[issuedAt,expiresAt). This can start a lease late or end it early; no skew grace
extends its end. Tests inject a private clock, not a caller-selected production now.

Before admitting within a new5-second time window, durably checkpoint a high-water
upper bound covering every admitted interval in that window. Reuse the serialized
DP journal transaction and Linux file/directory sync protocol, within its16KiB cap;
do not add a database or unbounded journal. On restart require the conservative
wall lower bound to reach that persisted upper bound before new admission. This
can briefly deny otherwise valid requests after an ordinary restart; it prevents
restart plus a backward clock from recovering time already consumed by a lease.
No automatic reset on new config, key rotation, rollback or CP outage is allowed.

Journal schema2 carries this clock floor; exact schema1 retains static recovery
but has no live authorization. Initialize/upgrade the clock floor only in an
explicit successful fresh v2 activation; failure/corruption/uncertain commit denies
admission and never provisions a new empty floor. Old software must reject the
upgraded journal. Window refresh is a serialized local maintenance operation
independent of CP. The fast admission path must not perform filesystem I/O; it
denies when a required checkpoint is absent. Future async transport integration
must run refresh on a bounded blocking worker, never block Tokio. A latched clock
requires explicit trusted local re-anchor after clock repair, retaining the floor;
this is an internal API, not an unauthenticated public reset command.
Windows provides the explicit in-memory model; actual durable acceptance is Linux.

## 6. Fixed initial admission quotas

These are conservative proposed ceilings, not measured throughput promises.
Use monotonic token buckets; counters are shared by (node,network group,plane),
not reset by source/ID/listener/sequence changes, DNS query or ordinary renewal.

| Resource | Per group | Per node |
|---|---:|---:|
| Legacy DNS admissions/second | 100, burst200 | 2000, burst4000 |
| In-flight admitted DNS requests | 64 | 512 |
| New gateway attempts/second | 20, burst40 | 200, burst400 |
| In-flight gateway admission attempts | 64 | 1024 |

Unknown/ungranted peers create no per-source state. Bound group counters by the
existing prepared-resource budget; retain only counters needed by current groups
or bounded outstanding attempts. Replacement never refills an existing bucket;
new groups start empty and refill by elapsed time, including after restart, so
config churn/restart cannot mint a burst. Removing/readding an ID cannot escape
the node-wide bound. Checked arithmetic, RAII release and cancellation/error paths
must enforce counts. No unbounded queue or per-request log/history. DNS socket,
message/response amplification, parser, TCP-connection and deadline limits still
need their own accepted P3-01 profile before a public bind.

## 7. Secure-observation learning boundary

Learning remains disabled by default. P4-01 implements the manual lease path and
the publication refusal boundary: legacy packets, raw peer headers, Endpoint IDs,
unverified/disabled/revoked credential metadata, observation-shaped JSON and a
Boolean claiming verification cannot produce an automatically learned lease.
Do not expose a constructor that fabricates verified capability evidence.

Actual successful Secure DNS authentication and its bounded observation producer
remain P3-07, after P6-01. Until that producer and its CP authorization contract are
implemented/reviewed, automatic observation-based publication is unavailable, even
if an opt-in intent is present. P4-01 must explicitly report that unavailable state;
it cannot claim real DoH/DoT learning passed. The future full producer profile must
bind Endpoint/current credential, configured allowed group/planes/targets/outbounds,
receipt freshness, revocation cause and a maximum learned lease (not decided here).
Restricted unknown networks still need manual bootstrap. Enabling that feature or
adding a new learned cause is a separate accepted P3-07/config-version change.

This is a conscious staged acceptance boundary: P4-01 closes manual/current
authorization and refusal of untrusted learning; real secure learning stays on
P3-07's explicit checklist. It must remain visibly unverified until actually tested.

## 8. Validation and adoption gates

No adoption before actual Windows/Linux checks, all existing dependency/license
bundles and independent implementation/final linked-account Code Reviews. Preserve
normal current-head merge and exact-main checks; no status or protection exception.

Required evidence includes exact v1 canonical/restore compatibility and v2 vectors;
tampered/mixed versions and replay; missing/invalid/too-long/future/expired leases;
wrong source/listener/Endpoint/group/Profile/target/port/outbound and ambiguous scopes;
same-NAT independent DNS Profiles without last-query gateway switch; IPv4/IPv6
separation; manual replacement/revocation racing admission; held old pins; CP loss;
forward/backward clock movement, suspend-like jumps, boundary arithmetic, restart
inside a persisted window, checkpoint failure/corruption and no floor reset; quota
churn/saturation/cancellation; and refusal of every unsupported learning input.
Real Linux durable tests must reopen without CP/DB and prove still-valid admission,
expiry and revoked denial, with actual crash/uncertain-commit modes where applicable.
Compile-fail boundaries must prevent static/diagnostic/old-pin data becoming a live
permission. No external public network service, trusted-root runner or new package
installation is needed for these tests. Network-path security is not claimed before
P3/P4 integration. Update documentation, task acceptance and deferred evidence clearly.

## Owner decision evidence

The owner directly accepted the full profile after checks in chat on2026-10-22.
That includes manual-first authorization, v2 compatibility,24h maximum manual
lease, conservative clock checkpoint/denial behavior, fixed admission quotas and
unavailable automatic learning until P3-07. This acceptance does not pass tests or
reviews, authorize public exposure or claim that later network integration exists.
