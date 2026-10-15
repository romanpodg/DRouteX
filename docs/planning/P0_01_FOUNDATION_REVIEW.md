# P0-01: DNS/proxy foundation review

Date: 2026-10-15. Status: **Accepted by the owner in this chat on 2026-10-15**.
Authority: [MVP scope](MVP_SCOPE.md), [roadmap](DEVELOPMENT_ROADMAP.md),
[CR-0001](../architecture/adr/ADR-0016-dns-proxy-mvp-before-tunnels.md).
This is a design walkthrough, not executable or network test evidence.

Updated after the user's Control D research request: accepted
[CR-0002](../architecture/adr/ADR-0017-profile-endpoint-dns-product-model.md) and the
[DNS product model](../architecture/DNS_PRODUCT_MODEL.md) now form this review packet.
The owner explicitly accepted this package and CR-0002 before authorizing P0-02.
They replace the earlier manual-IP-only mechanism proposal. The
[research report](../research/CONTROLD_ANALYSIS.md) separates vendor documentation
from Routira design inferences; no vendor runtime behavior was independently tested.

## Execution authorization and confirmed requirements

The user instructed this chat to begin from MVP_SCOPE.md on 2026-10-15 and to use
Graphify, Ponytail and the installed Rust skills. This authorizes starting roadmap
work at P0-01. It does not silently accept all proposed technical ADRs or satisfy
P0-01's owner review of the concrete security contracts.

The user then explicitly selected **remote devices with changing public IPs,
public access, and no VPN**, initially asking for DNS-setting-only setup and
individual device authorization. After the shared TCP gateway limitation was
explained, the user chose **DNS settings only; access and profile shared by the
network's external IP**. That group-access tradeoff is accepted; it is not a device
login. Unrestricted recursion or a public unauthenticated TCP relay remains outside
the security boundary. The Linux target has not been supplied by the operator.

## Review scope and affected boundaries

P0-01 defines ingress identity, destination authorization, process ownership,
snapshot/failure semantics and the test target. Its deliverables are this review,
the [threat model](../security/THREAT_MODEL.md), and references from existing ADRs.
At the initial review there were no application callers, fixtures, database migrations or exports.

P0-02 consumes these contracts for the minimal common/config/policy workspace.
P0-03 consumes the platform and network scenarios for disposable fixtures.
P4-01/P3-06 consume the approved identity/binding choice before exposing SmartDNS.
P1-05/P1-06 consume the snapshot invariants. No prerequisite is waived by this review.

## Public ingress decision: D05/D08

Accepted legacy/gateway choice: public DNS-only clients can share a network/group
context. Accepted CR-0002 also supports personal DoH/DoT Endpoints whose resolver
capabilities select distinct DNS Profiles across changing IPs and shared NAT.
This does not prove the device identity of an unrelated transparent TCP connection.

Manual administrator registration of a current public IP, bounded expiry and scoped
service/port authority remains the bootstrap/fallback. Proposed optional IP learning
uses only a valid secure DNS observation, explicit administrator opt-in and a
subsequent authenticated grant publication. It never silently broadens prefixes,
switches gateway Profile to the last DNS endpoint, or merges conflicting permissions.
DDNS polling and a router updater are deferred. See the product model for bounded
observation delivery, controller-outage behavior and derived-grant revocation.

Use exact IPv4 /32 and IPv6 /128 entries, not automatic prefix/range expansion.
Reject overlapping or ambiguous profile assignments. Different IPv6 source
addresses require explicit entries in the same group; a shared IPv4 NAT grant
does not authorize a client's unrelated IPv6 address. Validate source identity
from the actual transport peer, never a DNS field or untrusted forwarded header.

Replacing/revoking a grant is a coordinated DNS/gateway publication; acknowledge
revocation only after applied. The runtime enforces expiry even when the controller
is unavailable. Old DNS answers and old-generation grace never renew a grant.
An IP change cannot be detected immediately without additional trusted signaling:
the former address remains a risk until expiry or applied replacement/revocation.
All other users behind an allowed NAT/CGNAT address can share the grant; the UI must
say so before registration. The source-IP rule does not prevent spoofed UDP packets,
so DNS reflection/rate/response limits and host prerequisites remain mandatory.

Pin maximum lease duration, clock-uncertainty handling, exact quota values and the
registration/replacement API in P4-01 before exposing a listener. No indefinite
grant or extension on controller loss is permitted.

Changing DNS settings cannot by itself attach a Routira credential to an unrelated
TCP connection. SNI indicates a requested server name; it is not a Routira login
([RFC 6066, section 3](https://www.rfc-editor.org/rfc/rfc6066.html#section-3)).
Authenticating a DoH request does not authenticate a subsequent ordinary TLS flow
to a shared gateway. This is an architectural inference from the separate protocols,
not a limitation that a client label or DNS transaction ID can fix.

| Candidate | Client requirement | Authorization and limitation | Decision state |
|---|---|---|---|
| Authenticated explicit proxy over TLS | Client/application supports proxy credentials and a protected connection | Credential authorizes that connection; target checks and quotas still apply. Does not transparently cover all applications or DNS-only TVs | Not selected; requires a separate secured-ingress decision |
| Personal DoH/DoT Endpoint | Configure a supported OS/browser DNS URL or hostname | Resolver capability selects DNS Profile; separate gateway group grant still required | Accepted CR-0002 core MVP path |
| Administrator registration or opt-in secure observation | Clients only change DNS; operator enables the chosen grant mode | Temporary network/group grant; same-NAT/reassignment/expiry/IPv6 limitations above | Group scope accepted; concrete publication/learning mechanism proposed |
| DNS setting alone plus individual authorization on a shared gateway | No additional client capability | No trustworthy per-device proof reaches the ordinary TCP connection; do not claim this combination works | User chose group access instead |

DoH supports HTTP authentication mechanisms, but particular clients may not expose
them ([RFC 8484, section 8.2](https://www.rfc-editor.org/rfc/rfc8484.html#section-8.2)).
CR-0002 makes personal DoH/DoT required in the revised accepted MVP, with separate
transport tests. A secret DNS hostname/path is a credential if used for authorization and
must not be exposed in history, access logs, screenshots or ordinary API responses.

An explicit proxy is distinct from the already planned proxy **outbound**. Selecting
that ingress requires its own auth/TLS/revocation/rate-limit/client-compatibility
criteria; it must not appear as an unreviewed expansion of P4-05.

## Accepted foundation contracts and retained implementation choices

| Contract | Proposed behavior | Owning tasks / decisions |
|---|---|---|
| Process boundary | Separate unprivileged controller and DNS/gateway runtime. Pure policy has no database, network or OS calls. Add only immediately used crates | P0-02; ADR-0001 |
| Trusted identity | Distinguish secure resolver capability, legacy group, gateway group and untrusted labels. A valid capability selects DNS Endpoint/Profile, not hardware identity or transparent TCP device authority | P1-01/P3-07/P4-01/P6-03; D05/D08 |
| Product policy | Reusable Profiles, one active Profile per DNS Endpoint, custom/service/filter/default layers, pinned catalog revisions and isolated private records | P2-02/P2-07/P6-03/P6-04; CR-0002 |
| Listener exposure | Explicit address/port/transport and access scope; no automatic wildcard bind or unauthenticated public resolver/proxy. Private management stays separate from public data access | P4-01/P10-02/P11-01; D04 |
| Destination binding | Approved ingress context + profile + service/domain + port + active route generation. Visible SNI is only a candidate. Unknown, ambiguous or unauthorized destinations are denied | P4-01/P3-06; D08 |
| Origin connection | Independent approved resolver; validate every returned IPv4/IPv6 address and every CNAME result used to dial. Deny reserved/private/metadata/self targets and loops; dial only a checked, pinned address | P4-02/P4-05; ADR-0003 |
| TLS passthrough | Preserve the origin certificate and encrypted bytes. Never terminate origin TLS to manufacture client authentication. Reject unsupported/ambiguous SmartDNS paths with an accurate reason | P4-03/P10-05 |
| Selected outbound | Proposed first candidate: HTTP CONNECT through a TLS-authenticated proxy; protect credentials and send the checked numeric target. Fail strict proxy routes closed. Exact transport/version/compatibility remain D19/P0-05 | P4-05/P4-06; D19 |
| DNS semantics | Keep answer/upstream selection separate from traffic egress. Isolate caches by effective policy and validation context. Define BLOCK, DNSSEC and HTTPS/SVCB behavior before DNS implementation | P2/P3; D09/D10 |
| Snapshot publication | Immutable content + source revision + schema + node scope + authentication + increasing activation sequence. Reject tampering/replay. Stage and durably coordinate complete DNS/gateway generations | P1-05/P1-06; D11 |
| Recovery and revocation | Rollback republishes old content under a fresh increasing sequence. Old cached DNS answers grant no authority. Applied revocations deny new flows; old-generation grace never restores revoked access | P1-06/P3-06/P6-03; D08 |
| Secrets and administration | Standard cryptographic libraries; separate master-key lifecycle, redacted secrets, private authenticated management, roles/MFA/audit and last-owner protection | P1-07/P6-01/P10-02; D01/D15 |
| Resource and privilege boundary | Bounded queues, connections, parsing, deadlines and retries. No NET_ADMIN, tunnel engines or custom privileged executor; only justified narrow DNS bind permission | P10-02/P10-04/P11-01 |

At implementation time use private constructors for validated stable values and
enums for distinct identity/action states. Keep crate exports small; do not introduce
a generic protocol registry or elaborate typestate framework ahead of a caller.
Address safety, route revocation and active-generation checks are changing facts
and must still be checked at the operation boundary.

Snapshot encoding/algorithm, key rotation details and numeric resource limits are
not approved here: D11/D15/D18 must close before their dependent implementation.
Last-known-good availability never extends an expired/revoked network grant.
Republishing old content must not resurrect revoked grants; reauthorization requires
an explicit new authorized change rather than an incidental rollback side effect.

## Three network-flow walkthroughs

| Flow | Design steps and expected observable evidence | Current result |
|---|---|---|
| W1: personal or group DNS policy | Secure capability selects Endpoint/Profile, or legacy source selects its unexpired group. Two secure Endpoints behind one NAT can differ. Apply layered BYPASS/BLOCK/local answers without secret logging or private-answer scope leaks | P2/P3/P4-08; Foundation accepted; D08/D09 implementation values pending; no runtime pass |
| W2: authorized SmartDNS to direct VPS egress | Authorize the registered network and target in one active route context; publish compatible gateway answer; independently resolve/check/pin origin; forward TLS unchanged. Verify original certificate and VPS egress; unknown/expired ingress or target denied | Contract traced to P4-01/P4-02/P4-03/P4-08; D05 group scope accepted, D08 mechanism in review; no runtime pass |
| W3: authorized gateway to protected proxy outbound | Start with valid ingress/binding; independently resolve/check/pin origin; authenticate the selected protected proxy connection and request that numeric target. Verify actual proxy egress and unchanged origin TLS | Contract traced to P0-05/P4-05/P4-08; D19 handshake not performed; no runtime pass |

For W3, CONNECT establishes a tunnel after proxy authorization; it is not itself
transport encryption ([RFC 9110, section 9.3.6](https://www.rfc-editor.org/rfc/rfc9110.html#section-9.3.6)).
Requiring TLS to the proxy and checked numeric targets is the proposed Routira design.

## Five failure-scenario walkthroughs

| Scenario | Required result | Pending executable evidence |
|---|---|---|
| F1: bad resolver secret, forged SNI, unknown ingress or conflicting learned scope | Invalid/revoked credential denied; no learning from untrusted metadata; no last-query gateway Profile switch. TCP remains group scoped; expired/revoked grants denied; reassigned old IP risk lasts until expiry/revocation | P3-07/P4-01/P6-03/P10-05, including expiry during CP loss and same-NAT adversary |
| F2: CNAME/rebinding returns private, metadata, self or mixed-family unsafe addresses | Deny unsafe dial candidates on every resolution/retry; never let the proxy resolve an unchecked name | P4-02/P4-05/P10-05, including IPv4-mapped IPv6 and origin-resolution loops |
| F3: selected proxy is down, auth fails or resources/deadlines are exhausted | Deny strict new flows; no implicit DIRECT. Bound resource use, cancel work, release sockets and report accurate version/reason | P4-05/P4-06/P10-04 |
| F4: tampered/stale snapshot, crash during activation, or old DNS answer after revocation | Reject replay/tampering; expose complete generation or guard new ingress; recover durably. Revoked bindings stay denied; rollback uses a new sequence | P1-05/P1-06/P3-06/P10-06 |
| F5: controller/database loss and subsequent fresh-host restore | Independent runtime continues with valid durable last-known-good. Restore encrypted consistent backup with separately supplied key; missing/corrupt key fails safely | P1-06/P1-07/P6-07/P10-06/P11-03 |

All eight walkthroughs were checked against the proposed boundaries and task
ownership. They establish review coverage only. Packet captures, TLS/egress proof,
failure injection and restore execution do not exist yet.

## Platform and exit conditions

Windows remains the development host; local Rust 1.99.0/Cargo 1.99.0 version commands
were rechecked after activating tools/activate.ps1. No workspace exists yet.
Proposed first native acceptance target: Debian 13 x86_64, subject to the operator's
answer. Debian lists amd64 among supported architectures in its
[release notes](https://www.debian.org/releases/trixie/release-notes/whats-new.en.html#supported-architectures).
This is target selection evidence, not Routira compatibility evidence. Record the
actual image/package/kernel versions in P0-03; WSL cannot replace native systemd proof.

The owner explicitly accepted P0-01/CR-0002 in this chat on 2026-10-15, including
D08 foundation semantics and Debian 13 x86_64 as the first native acceptance target.
Acceptance covers secure endpoint credentials, opt-in observations, layered filtering,
private-record isolation and runtime safety. The owner then approved Rust 1.99.0/MSRV,
Edition 2024, no external dependencies and limited Windows/Linux CI for P0-02.
P0-01 is completed. P0-03/P0-05 are not started. ADR-0017 is Accepted; other technical
ADRs retain Proposed status for detailed mechanisms outside this foundation decision.
No tunnel prerequisite has been added; the existing planning validator checks the
61-task revised MVP dependency closure and preserved source specifications.

## Owner decision and next execution bound

Owner decision recorded: explicit acceptance of P0-01/CR-0002 and continuation
through P0-02. The following foundation is accepted; this is not blanket acceptance
of every technical ADR:

| Area | Foundation accepted at P0-01 |
|---|---|
| Product model | CR-0002: reusable Profiles, one active Profile per DNS Endpoint, personal DoH/DoT, pinned local filter/service revisions and Profile-isolated private records; custom > service > filter > default policy layers |
| Public access | Secure DNS capability selects a DNS Profile. Legacy DNS and transparent TCP use separate, exact /32 or /128 network/group grants; same-NAT peers share authority. No per-device TCP identity claim |
| Changing IPs | Manual authenticated registration/replacement is bootstrap/fallback. Secure-query observation learning is disabled by default, explicitly scoped, bounded and published by the controller; conflicts are denied, never merged or switched by the latest query |
| Runtime safety | Separate unprivileged controller/runtime; authenticated immutable generations and last-known-good; expiry/revocation remains effective during outages/rollback; independent checked numeric origin dial, unchanged origin TLS and strict proxy failure denies access |
| First platform and next task | Windows development; Debian 13 x86_64 as the accepted first native acceptance target, with actual image/kernel proof in P0-03. Then P0-02 creates only immediately needed common/config/policy crates with cross-host build checks |

Numeric lease/clock/resource values, snapshot encoding/algorithm, key lifecycle,
DNS response compatibility, exact protected outbound and the operator's actual lab
remain open at their owning gates (D03/D08/D09/D11/D15/D18/D19). Accepting the
foundation does not choose these silently or claim runtime compatibility.

The owner-authorized execution bound is P0-02, followed by its reviewable diff and
validation evidence. It does not authorize PR merging or completion of later tasks.
Any later requested contract change must be recorded explicitly before dependent work.

## Verification for this review

Regenerate with python3.12 tools/render_planning.py and verify with
python3.12 tools/verify_planning.py --self-test. The revised validator checks 88 tasks,
188 mappings, the 61-task MVP closure, links/ADR statuses and nine intentionally
invalid cases, including missing secure-DNS/catalog gates. Actual results are in
[planning-verification.json](../environment/planning-verification.json), not inferred
from the design. Original EN/RU source hashes must remain unchanged.
Additional assertions require recorded owner acceptance, completed prerequisites,
the P0-02 execution bound and exactly three workspace crates. Cargo workspace checks
are tracked in EXECUTION_STATUS.md; native Linux network/recovery tests remain P0-03.
The owner decision is not executable network evidence.
CodeScene/Codecov remain unavailable as product evidence.
