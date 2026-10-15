# Testing strategy

Status: Proposed. Base specification §22 plus accepted CR-0001 define applicability.
DNS/proxy MVP and later engine/tunnel expansion have separate acceptance gates.
Proposed [CR-0002](../architecture/adr/ADR-0017-profile-endpoint-dns-product-model.md)
adds mandatory personal DoH/DoT, layered catalogs/private records and separate
legacy/gateway access grants to the revised MVP plan. No transport evidence exists yet.

## MVP layers and proof boundaries

Pure model/policy tests use deterministic fixtures, virtual time and proptest where justified.
Assert precedence, IDNA/wildcards, invalid references/states, trusted identity, rejection of
unsupported future capabilities, snapshot determinism and explain reason codes.
Differential interpreter/compiler and permutation tests guard behavior, not implementation shape.

DNS integration uses actual UDP/TCP frames and controlled upstreams: A/AAAA/CNAME/TXT,
required auxiliary records, HTTPS/SVCB, truncation/framing, TTL/negative cache, scope/mode/
validation separation, malformed traffic and loops. Validate normal signed RRsets separately
from overrides; never claim a rewrite retains original DNSSEC authenticity.

Gateway integration uses a real TLS origin and its genuine certificate. Exercise authorized
SmartDNS ingress, selected protected proxy outbound, fragmentation, half-close/backpressure,
actual checked dial address, proxy handshake/egress, quotas, timeout/cancellation and unchanged
TLS bytes. A listening socket or mocked proxy cannot establish route/service acceptance.
Diagnostics are allowlisted and RBAC protected. P4-08 verifies a real VPN-free vertical slice.

Storage/recovery uses actual SQLite migrations/WAL/transactions/backups and fresh-host restore
with a separately supplied master key. Inject disk-full, failed flush/apply and process loss
at each DNS/gateway generation transition; assert last-known-good service under CP/DB outage.

Web tests cover login/MFA, later administrator creation/roles, rule draft/validate/simulate/
publish, DNS/proxy/device/profile administration, diagnostics and restore UX.
Choose Vitest/component/browser dependencies at P7-01. VPN QR/config export is deferred.

## MVP adversarial matrix

| Case | Expected evidence / owning task |
|---|---|
| Rule reorder/equal-priority contradiction | Stable output or explicit rejection, P2-02/P2-03 |
| Unicode/trailing dot/wildcard apex | Canonical documented matching, P2-01 |
| NAT/shared public IP | No inferred individual authorization, P4-01/P6-03 |
| Two personal secure endpoints behind one NAT / roaming | Distinct DNS Profiles and valid capability across IP change; no implied per-device gateway identity, P3-07/P4-08 |
| Unknown/copied/revoked resolver capability / client alias | Deny invalid/revoked access; alias cannot select privileges; copied capability scope explicit and secrets redacted, P3-07/P6-03/P10-02 |
| Secure-IP learning / restricted-IP bootstrap / controller outage | No learning from legacy/invalid token/forwarded headers; no last-query Profile switch; bounded publication queue and local expiry, P3-07/P4-01/P4-08 |
| Catalog import/update and private records | Pinned revisions, layer precedence, malformed/oversized/unsupported input rollback, private answer scope isolation without SSRF permission, P2-07/P2-02/P3-04 |
| Authorized SNI / missing or forged SNI / ECH | Authorized passthrough or accurate fail-closed reason, P4-01/P4-03 |
| QUIC/HTTP3 in DNS-only TCP mode | Honest unsupported/limited result, P3-05/P10-05 |
| HTTPS/SVCB and signed overrides | Coherent supported path and no false DNSSEC authenticity, P3-05 |
| IPv6/mapped IPv4/private CNAME/rebinding | Actual dial target checked/pinned, P4-02/P10-05 |
| Self gateway/upstream recursion | Rejection without runaway traffic, P3-02/P4-02 |
| New flow after confirmed access revocation | Denied with applied revision; existing-flow policy explicit, P6-03/P4-08 |
| Selected proxy unavailable/auth failure | Strict route denied; no silent DIRECT fallback, P4-05/P4-06 |
| Controller/database loss | Local DNS/proxy serves last-known-good and recovers, P1-06/P10-06 |
| Tampered/stale/wrong-scope snapshot | Rejected; rollback uses a new authorized epoch, P1-05 |
| Activation/power loss | Complete local generation or guarded failure, P1-06/P10-06 |
| CSRF/fixation/role escalation/last-owner removal | Server-side denial and audit, P6-01/P6-08 |
| Debug/API/log/backup secret leak | No plaintext outside authorized delivery, P1-07/P10-02 |
| Floods/slow TLS/large rules/FD exhaustion | Bounded resources, measured limits and recovery, P10-04 |
| Host permissions/arbitrary OS requests | No NET_ADMIN/custom root helper; denied network mutation, P10-02/P11-01 |

DNS-only acceptance cannot force an endpoint that bypasses DNS to follow its rules.
Fail-closed applies to the controlled proxy boundary. Shared CDN IP never proves domain identity.
MVP passes on hosts without WG/AWG/Xray/sing-box, VPN clients or TUN.

## Deferred protocol and multi-node evidence

G3 uses adapter fakes only for lifecycle unit tests. Acceptance requires actual pinned
WG/AWG/Xray clients/engines, tunnel interfaces, credentials, observed routing and compatible
exports. Parser success or process health alone cannot prove transport/egress.
P5-09 adds engine/peer/export UI and P5-10 verifies original deferred §24.1 cases.

| Future case | Gate and owner |
|---|---|
| WG/AWG routing and tunnel client segmentation | G3 P5-04/P5-05/P5-10 |
| Xray crash and cross-engine isolation | G3 P5-08/P5-10 |
| Peer credential rotation/revocation | G3 P5-07/P5-10 |
| Unauthorized helper IPC/path/argument injection and interrupted OS operation | G3 P10-01/P5-10 |
| Remote enrollment/identity/replay/expiry/partition | G4 P8-01/P8-03/P8-05 |
| Node outage/failover/partial rollout divergence | G4 P9-02/P9-04/P9-05 |

These tests remain pending; CR-0001 changes release applicability, not their required rigor.

## Platform matrix and runners

Windows: policy/schema/storage/API/user-space DNS/TCP/frontend static/unit checks.
WSL Ubuntu: DNS/TCP fixture after checking required namespace/socket tools. MVP needs no
/dev/net/tun, nftables manager or VPN module. Linux namespace lab setup may require privileges;
that does not grant host-network mutation to deployed services.
Native Linux VM/VPS: actual DNS/proxy/TLS/egress, IPv4/IPv6, systemd permissions, installation,
restart/upgrades/restore. Supported target and proxy transport are D03/D19.
Before G3 additionally verify TUN/modules/nftables/engine/client compatibility in D13; the
existing WSL kernel is not production AWG evidence.

Tests clean only verified fixture namespaces/files, never production interfaces, and release
resources on failure. A disposable trusted runner precedes privileged lab tests. Public/fork
PRs run unprivileged checks; do not expose private runner credentials or root to arbitrary code.

## Specialized tools

Fuzzing/nightly is isolated and justified by exposed parsers; stable regression corpora remain
runnable. Miri verifies justified unsafe investigations, not real networking. Loom requires
custom concurrent primitives; Criterion requires a measured policy/DNS/TCP regression target.
No optional runtime/test tools are installed merely because they exist.

Retain commands/tool/target/proxy versions, packet captures and egress observations,
seed/corpus, resource limits, redacted logs and artifact checksums as acceptance evidence.
