# DNS/proxy MVP threat model

Date: 2026-10-15. Status: **Proposed; P0-01 owner review pending**.
Scope: [accepted MVP](../planning/MVP_SCOPE.md), specification sections 3/5/10/19/22,
and [foundation review](../planning/P0_01_FOUNDATION_REVIEW.md).
No implemented security control or successful penetration test is claimed.
Proposed [CR-0002 product model](../architecture/DNS_PRODUCT_MODEL.md) extends this
review to personal DoH/DoT capabilities, versioned catalogs and opt-in learned grants.

## Assets and actors

Protect the VPS and adjacent networks, destination authorization, policy integrity,
administrator sessions, access credentials, backup/master keys, service availability
and private DNS/connection metadata.

Actors include the owner, later administrators/operators/read-only users, authorized
remote clients with changing IPs, unauthorized internet clients, another device
sharing a client's NAT, malicious origins/upstreams/proxies and compromised local
processes. Host-root compromise can defeat local services and is outside their
containment guarantee; protected backups still need a separately held recovery key.

## Trust boundaries and authority

| Boundary | Required authority and checks | Failure posture |
|---|---|---|
| Internet to DNS/gateway | Explicit ingress authorization appropriate to the selected client capability, quotas and independently authorized target | No public anonymous recursion or relay; D05/D08 must close before exposure |
| Browser to controller | TLS, authenticated session, role permission, MFA where required, CSRF/Origin controls | Deny unauthorized mutations; audit sanitized action/result |
| Controller to database/secret store | Validated drafts/references, explicit secret ownership and encrypted sensitive material | Invalid draft cannot replace active policy |
| Publisher to local runtime | Authenticated scoped immutable snapshot, supported schema and increasing durable sequence; restricted local IPC | Reject forged, wrong-node and stale state; retain valid last-known-good |
| Runtime to origin resolver/dial | Independent resolver; validate every address actually used, deny reserved/private/metadata/self targets and loops, pin numeric target | Fail closed on unsafe or ambiguous target |
| Runtime to protected outbound proxy | Authenticate proxy endpoint and credentials over protected transport; preserve checked numeric destination | Never delegate unchecked target resolution or silently switch to DIRECT |
| Service to host | Unprivileged identity; minimum read/write paths and narrow DNS bind permission | No shell/route/firewall/TUN/engine execution API in MVP |
| Backup to fresh host | Encrypted consistent SQLite/secrets backup and separately supplied recovery key | Missing/wrong key fails without overwriting a working configuration |

## Threat register and planned verification

| ID | Threat / impact | Required control | Evidence owner |
|---|---|---|---|
| T01 | Open recursive DNS / reflection and resource exhaustion | Authorized ingress, bounded UDP/TCP handling, response/concurrency/rate limits, operator host access controls | P3-01/P3-02/P10-04/P11-01 |
| T02 | Open proxy or forged SNI / unauthorized traffic | Authorize ingress independently of SNI; profile/service/domain/port allowlist in the active generation | P4-01/P4-03/P10-05 |
| T03 | Dynamic IP, CGNAT, IPv6 or address reassignment / wrong principal | Accepted group scope, never device identity. Private authenticated exact-address administration, expiry, applied replacement/revocation, same-NAT warnings and separate IPv4/IPv6 grants; no automatic ownership-detection claim | D05/D08; P4-01/P6-03/P10-05 |
| T04 | Rebinding/CNAME/private IPv6/mapped IPv4/self target / SSRF | Recheck actual dial addresses after every resolution/retry, pin safe address, reject metadata/internal/self/loop targets; no unchecked remote DNS at proxy | P4-02/P4-05/P10-05 |
| T05 | Cache poisoning or policy-scope collision / wrong answer or route | Transaction/upstream validation, policy/generation/validation-aware caching; distinguish signed upstream data from rewritten answers | P3-02/P3-03/P3-04/P3-05 |
| T06 | ECH/QUIC/SVCB/DNS bypass / false routing promise | Explicit compatibility decisions; unsupported or ambiguous gateway flow denied. DNS-only does not enforce all endpoint traffic | P3-05/P4-03/P7-04/P10-05 |
| T07 | Snapshot tampering/replay or crash / partial or revoked policy | Authenticate canonical scoped content, durable monotonic sequence, coordinated activation/journal; rollback as new publication; revoked grants never restored by grace | P1-05/P1-06/P3-06/P10-06 |
| T08 | Proxy outage/auth failure / unintended direct exit | Explicit strict failure semantics, bounded retry/deadline and honest reason/status; no implicit fallback | P4-05/P4-06/P10-04 |
| T09 | Session theft, CSRF, privilege escalation, last-owner removal / management takeover | Server-side auth/RBAC/MFA/session/revocation checks, closed bootstrap and protected owner lifecycle | P6-01/P6-08/P10-02 |
| T10 | Credential/master-key/backup/log leak / impersonation or data loss | Separate key lifecycle, standard encryption, scoped delivery, redacted Debug/API/errors/audit; encrypted restore proof | P1-07/P6-07/P10-02/P11-03 |
| T11 | Slow clients/floods/oversized inputs/disk full / outage | Bounded queues, memory/FD/parser limits, timeout/cancellation/backpressure and graceful shutdown; preserve active state on failed writes | P10-04/P10-06 |
| T12 | Dependency compromise or privileged runner / host compromise | Pinned verified sources and licenses, advisory checks, no untrusted privileged CI, no root application/network executor | P0-04/P0-05/P10-02/P11-05 |
| T13 | Resolver capability leak via URL/SNI/log or copied settings / DNS and learned-network access theft | High entropy, separate public EndpointId, protected provisioning, redaction, rotation/applied revocation; DoT identifier visibility documented; client suffix untrusted | P1-01/P3-07/P6-03/P10-02 |
| T14 | Poisoned/oversized catalog or cross-profile private record / false policy or data disclosure | Bounded local formats, approved provenance/license/hash, revision pinning, import failure rollback and cache/profile isolation; private answer never authorizes proxy dial | P2-07/P2-05/P3-03/P3-04 |
| T15 | Learned IP permission union or last-query Profile switch / gateway privilege escalation | Separate DNS/gateway grant authority; opt-in observed-peer events, authenticated publication, conflict rejection, bounded expiry/queues and cause-linked revocation; no learning from headers/legacy/alias | P3-07/P4-01/P6-03 |

## Secret ownership and lifecycle

| Material | Owner and storage boundary | Lifecycle / allowed disclosure |
|---|---|---|
| Administrator and client credentials | Auth subsystem; password verifiers or encrypted credential material as appropriate | Bootstrap/issue/rotate/revoke; plaintext only in explicitly authorized provisioning, never routine DTO/log/audit |
| Proxy outbound credential | Controller secret store; scoped runtime delivery | Encrypted at rest, authenticated encrypted transport to proxy; revoke/rotate independently of policy labels |
| Snapshot authentication material | Publisher/runtime trust boundary | Separate from content and ordinary configuration; scoped keys/rotation/replay recovery designed in D11/D15 |
| Encryption/recovery key | Operator-controlled provisioning outside database/backup payload | Supply separately at startup/restore; document rotation and loss recovery; never commit or include in ordinary backups |
| Session/CSRF/bootstrap material | Auth subsystem and protected session storage | Short-lived or single-use as applicable; revoke on relevant access changes; no argv, URLs or telemetry exposure |

Cryptographic algorithms, canonical format and exact retention/expiry/resource values
remain decisions before their owning implementation tasks. No homegrown cryptography,
origin HTTPS MITM or temporary plaintext credential logging is permitted.

## Residual risks and release evidence

Public changing-IP, VPN-free access with DNS-only clients is confirmed. The user
accepted network/profile access shared by the external IP after discussing the
lack of per-device proof in ordinary transparent TCP. Personal secure DNS capabilities
can select different DNS Profiles behind the same NAT. Manual registration remains
bootstrap/fallback; proposed opt-in secure observations publish bounded group grants.
The product cannot present those group
grants as per-device authorization or promise automatic address-change detection.
Same-NAT/CGNAT peers can share the grant; a reassigned old IP can retain access until
expiry or applied replacement/revocation. An IPv4 grant does not cover unrelated
IPv6 addresses. Old-generation recovery cannot renew or restore revoked grants.
Secure DNS endpoint selection alone does not secure an ordinary shared TCP gateway.
New observations cannot authorize networks during controller loss; existing grants
still expire. A revoked resolver capability loses its derived grant authority.
Adding an authenticated explicit inbound changes client setup and needs an explicit
scope/compatibility decision; it was not selected for this DNS-only ingress.

DNS-only clients can use cached answers, other resolvers, direct IPv6 and QUIC.
Fail-closed is a guarantee inside Routira's authorized proxy boundary. Origin and
proxy operators still observe relevant traffic metadata. Private administrator
access does not solve data-plane client authorization.

Use W1-W3 and F1-F5 in the foundation review as the design walkthrough. Later release
evidence requires actual DNS/TCP/TLS/proxy traffic, origin-certificate/egress checks,
same-NAT adversaries, IPv4/IPv6, revocation, outage/crash recovery, quota tests and
fresh-host encrypted restore on the supported native Linux target. A document review,
mock or open listening socket cannot pass those gates.

WG/AWG/Xray, TUN, managed engines, privileged network execution and remote-node
enrollment remain post-MVP. This model must be extended before those boundaries exist.
