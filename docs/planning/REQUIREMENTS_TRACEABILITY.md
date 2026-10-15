# Requirements traceability

**Proposed implementation plan, not product test evidence.**

Base source: [Routira_Project_Specification_EN.md](../../Routira_Project_Specification_EN.md); SHA-256 78ca71c61755152aa4e369d9b927a8aa73cc6afc8c11bc94c3ceb9c58dc275d9.

Accepted CR-0001: [ADR-0016](../architecture/adr/ADR-0016-dns-proxy-mvp-before-tunnels.md). Original source wording is preserved; effective release applicability is explicit below.

All numbered headings/subsections were considered. Parent rows aggregate children. Delivery tags derive from mapped tasks and may span releases; they do not mark deferred requirements complete.

| ID / source section | Requirement or planned interpretation | Kind | Effective scope | Task deliveries | Work items |
|---|---|---|---|---|---|
| R-1 | Concept and Goals: all mapped child obligations below | Section rollup | See child requirements | FUTURE, MVP, POST_MVP_PROTOCOLS | F-01, F-03, F-05, P0-01, P1-06, P2-06, P4-01, P5-08, P6-08, P7-07 |
| R-1.1 | Self-hosted DNS and route management for personal/home infrastructure; no commercial public VPN claim | Explicit requirement | MVP + later | FUTURE, MVP | P0-01, P7-07, F-01 |
| R-1.2 | Self-hosted DNS and route management for personal/home infrastructure; no commercial public VPN claim | Explicit requirement | MVP + later | FUTURE, MVP | P0-01, P7-07, F-01 |
| R-1.3 | Self-hosted DNS and route management for personal/home infrastructure; no commercial public VPN claim | Explicit requirement | MVP + later | FUTURE, MVP | P0-01, P7-07, F-01 |
| R-1.4 | Rust core, safe defaults, explain/capability-aware UX, no MITM, atomic changes, privacy and gradual complexity | Explicit requirement | MVP | MVP, POST_MVP_PROTOCOLS | P1-06, P2-06, P4-01, P5-08, P6-08 |
| R-1.5 | No universal DNS access/QUIC/DPI guarantee, own cryptography, session migration or commercial tenancy | Technical constraint | Scope guard | FUTURE, MVP | P0-01, F-01, F-03, F-05 |
| R-2 | Primary Use Cases: all mapped child obligations below | Section rollup | See child requirements | FUTURE, MVP, POST_MVP_MULTI_NODE, POST_MVP_PROTOCOLS | F-02, F-03, P10-05, P2-04, P3-06, P4-05, P4-06, P5-04, P5-05, P5-08, P7-02, P7-04, P9-02, P9-03 |
| R-2.1 | Selective domain action and exit selection; DIRECT relative to current decision point | Explicit requirement | MVP | MVP, POST_MVP_PROTOCOLS | P2-04, P4-06, P5-04 |
| R-2.2 | TV DNS-only supports restricted compatible TCP and visibly limited ECH/QUIC/nonstandard/application compatibility | Technical constraint | MVP | MVP | P3-06, P7-02, P10-05 |
| R-2.3 | WG/AWG tunnel and peer identity deferred by CR-0001; client split routing still requires actual support | Explicit requirement | Post-MVP protocols | FUTURE, POST_MVP_PROTOCOLS | P5-04, P5-05, F-02 |
| R-2.4 | Allocate DNS/TCP/admin/proxy endpoints in MVP; WG/AWG/Xray listeners and conflicts deferred | Explicit requirement | MVP DNS/proxy; engines later | MVP, POST_MVP_PROTOCOLS | P4-05, P7-04, P5-08 |
| R-2.5 | Cross-node tunnel termination and explicit verified TUN/proxy bridge, not packet protocol conversion | Explicit requirement | Post-MVP | POST_MVP_MULTI_NODE | P9-03 |
| R-2.6 | Allowed outbound failover with session/TTL limits; inbound switching requires supporting client | Technical constraint | MVP + later | FUTURE, MVP, POST_MVP_MULTI_NODE | P4-06, P9-02, F-03 |
| R-3 | Fundamental Technical Limitations: all mapped child obligations below | Section rollup | See child requirements | FUTURE, MVP | F-01, F-02, F-03, P10-05, P11-04, P2-02, P3-04, P3-05, P4-01, P4-03, P4-06, P6-03, P7-03, P7-04 |
| R-3.1 | Ordinary DNS does not route traffic; rewritten address needs authorized original-destination context | Technical constraint | MVP | MVP | P3-04, P4-01, P11-04 |
| R-3.2 | ECH hidden inner SNI prevents reliable SNI-only dispatch; fail closed or use explicitly supported alternative | Technical constraint | MVP | MVP | P4-03, P10-05, P7-03 |
| R-3.3 | QUIC/HTTP3 is UDP; TCP-only DNS gateway cannot promise interception or client QUIC disablement | Technical constraint | MVP + future | FUTURE, MVP | P3-05, P10-05, F-01 |
| R-3.4 | Handle HTTPS/SVCB ALPN/IP hints/ECH/alternative endpoints; no blanket record removal as universal fix | Technical constraint | MVP | MVP | P3-05 |
| R-3.5 | Separate normal verifiable DNSSEC answers from local overrides; rewritten response cannot claim authentic signature | Explicit requirement | MVP | MVP | P3-05, P7-04 |
| R-3.6 | Own DoH/DoT/hardcoded IP/cache bypass DNS; full endpoint control requires client/router cooperation | Technical constraint | MVP + future | FUTURE, MVP | P10-05, P11-04, F-03 |
| R-3.7 | Shared public NAT IP cannot identify individual device or link DNS to later TCP authorization | Explicit requirement | MVP | MVP | P2-02, P4-01, P6-03 |
| R-3.8 | CDN/shared IP, pinning, geo service checks, TTL/session lag and IPv6 policy limit route guarantees | Technical constraint | MVP + later | FUTURE, MVP | P4-06, P10-05, P11-04, F-02 |
| R-4 | Operating Modes: all mapped child obligations below | Section rollup | See child requirements | FUTURE, MVP, POST_MVP_MULTI_NODE, POST_MVP_PROTOCOLS | F-02, P10-01, P10-05, P11-04, P3-01, P3-04, P3-06, P4-01, P4-04, P4-05, P5-04, P5-05, P5-06, P7-02, P9-01 |
| R-4.1 | Restricted policy DNS caches/filters/forwards; ordinary origin-IP response leaves direct client path | Explicit requirement | MVP | MVP | P3-01, P3-04 |
| R-4.2 | Compatible approved SmartDNS TCP identifies origin, preserves website certificate and accounts for caching/SVCB/IPv6 | Explicit requirement | MVP | MVP | P3-06, P4-01, P4-04, P10-05 |
| R-4.3 | Authenticated/isolated explicit proxy supplies destination; UDP capability and public listener require separate verification | Proposed architecture | MVP + later | MVP, POST_MVP_MULTI_NODE, POST_MVP_PROTOCOLS | P4-05, P5-06, P9-01 |
| R-4.4 | Full/split tunnel and TUN/OS routing deferred by accepted CR-0001 | Proposed architecture | Post-MVP protocols | FUTURE, POST_MVP_PROTOCOLS | P5-04, P5-05, P10-01, F-02 |
| R-4.5 | Capability matrix accurately distinguishes DNS-only, limited gateway, explicit proxy and full tunnel | Explicit requirement | MVP | MVP | P7-02, P11-04 |
| R-5 | High-Level Architecture: all mapped child obligations below | Section rollup | See child requirements | MVP, POST_MVP_MULTI_NODE, POST_MVP_PROTOCOLS | P0-01, P0-02, P1-05, P1-06, P10-01, P5-02, P5-08, P8-04 |
| R-5.1 | CP manages entities/publication, DP continues last-valid config without dashboard/database | Explicit requirement | MVP + later | MVP, POST_MVP_MULTI_NODE | P0-01, P1-06, P8-04 |
| R-5.2 | Logical Rust services may share a small process set while engines remain independent; avoid gratuitous microservices | Proposed architecture | MVP | MVP, POST_MVP_PROTOCOLS | P0-02, P5-02 |
| R-5.3 | DNS/router use snapshots; independent engine restarts/failures; privileged operations isolated from HTTP | Explicit requirement | MVP | MVP, POST_MVP_PROTOCOLS | P1-05, P5-08, P10-01 |
| R-6 | Network Flows: DNS, SmartDNS, Tunnels: all mapped child obligations below | Section rollup | See child requirements | MVP, POST_MVP_MULTI_NODE, POST_MVP_PROTOCOLS | P2-04, P3-04, P3-06, P4-01, P4-02, P4-04, P5-04, P9-02, P9-03 |
| R-6.1 | Real ordinary DNS flow returns upstream origin; client-origin connection remains direct | Explicit requirement | MVP | MVP, POST_MVP_PROTOCOLS | P3-04, P5-04 |
| R-6.2 | SmartDNS redirect and TCP passthrough use coherent authorized route context and loop-free trusted origin resolution | Explicit requirement | MVP | MVP | P3-06, P4-01, P4-02, P4-04 |
| R-6.3 | AWG termination followed by real VLESS outbound requires verified separate engineering bridge | Explicit requirement | Post-MVP | POST_MVP_MULTI_NODE | P9-03 |
| R-6.4 | Distinct DIRECT/BLOCK/PROXY_NODE/NODE_GROUP/OUTBOUND/UPSTREAM_DNS/LOCAL_DNS_OVERRIDE semantics | Explicit requirement | MVP + later | MVP, POST_MVP_MULTI_NODE | P2-04, P9-02 |
| R-7 | Multi-Protocol Infrastructure: all mapped child obligations below | Section rollup | See child requirements | FUTURE, MVP, POST_MVP_MULTI_NODE, POST_MVP_PROTOCOLS | F-01, F-03, P0-05, P1-01, P10-04, P11-05, P4-06, P5-01, P5-02, P5-07, P5-08, P8-02, P9-02 |
| R-7.1 | Model inbound/outbound/engine/adapter distinctly; Xray is engine, VLESS+REALITY protocol/transport/security | Explicit requirement | MVP | MVP, POST_MVP_PROTOCOLS | P1-01, P5-01 |
| R-7.2 | Native DNS/TCP proxy first; WG/AWG and managed Xray/sing-box require independent later engine gates | Proposed architecture | Post-MVP protocols/future | FUTURE, MVP, POST_MVP_PROTOCOLS | P0-05, P5-08, F-01 |
| R-7.3 | Version/transport-specific capabilities; unknown is unavailable; no universal support switch | Explicit requirement | MVP + later | POST_MVP_MULTI_NODE, POST_MVP_PROTOCOLS | P5-01, P8-02 |
| R-7.4 | General engine lifecycle/peer/export traits deferred; MVP needs a narrow typed TCP outbound connector | Proposed architecture | Post-MVP protocols | POST_MVP_PROTOCOLS | P5-01, P5-07 |
| R-7.5 | Verify/pin MVP dependencies and limits; external engine process supervision deferred until engines are introduced | Explicit requirement | MVP dependencies; engines later | MVP, POST_MVP_PROTOCOLS | P0-05, P5-02, P10-04, P11-05 |
| R-7.6 | Outbound switch possible; inbound requires client; sessions interrupted and strict fail-open forbidden by default | Technical constraint | MVP + later | FUTURE, MVP, POST_MVP_MULTI_NODE | P4-06, P9-02, F-03 |
| R-8 | Policy Engine and Rule System: all mapped child obligations below | Section rollup | See child requirements | FUTURE, MVP, POST_MVP_MULTI_NODE | F-02, P1-02, P1-05, P1-06, P2-01, P2-02, P2-03, P2-04, P2-06, P2-07, P6-04, P6-05, P6-06, P7-03, P9-04 |
| R-8.1 | Normalized exact/wildcard/domain/profile exceptions; documented apex semantics; richer CIDR/protocol/group rules staged | Explicit requirement | MVP + later | FUTURE, MVP | P2-01, P2-02, P6-04, F-02, P2-07 |
| R-8.2 | Trusted identity/profile, canonical match, priority-specificity-stable tie, conflict/capability/failure handling and reasons | Explicit requirement | MVP | MVP | P2-02, P2-03, P2-04 |
| R-8.3 | Illustrative policy format becomes versioned validated schema; example is not working deployed config | Proposed architecture | MVP | MVP | P1-02, P6-04 |
| R-8.4 | Explain rule/action/exit/limits/revision predicts policy; separate actual network test proves connectivity | Explicit requirement | MVP | MVP | P2-06, P6-06, P7-03 |
| R-8.5 | Validate/simulate/compile/stage/activate/health/rollback; monotonic hash/version; no partial node state or restored-session promise | Explicit requirement | MVP + later | MVP, POST_MVP_MULTI_NODE | P1-05, P1-06, P6-05, P9-04 |
| R-9 | DNS Engine: all mapped child obligations below | Section rollup | See child requirements | MVP | P0-02, P1-05, P3-01, P3-02, P3-03, P3-04, P3-05, P3-06, P3-07, P4-02, P7-04 |
| R-9.1 | Restricted UDP/TCP forwarder; personal DoH/DoT endpoint transports required by proposed CR-0002, separately tested; local policy/cache/metrics | Base requirement with proposed CR-0002 extension | MVP; personal DoH/DoT required in proposed CR-0002 | MVP | P3-01, P3-02, P3-03, P3-07 |
| R-9.2 | Pin compatible Hickory server/resolver features with Tokio/rustls; custom first-party policy layer | Proposed architecture | MVP | MVP | P0-02, P3-01 |
| R-9.3 | Name/type/class/upstream/mode/scope/validation cache distinction prevents profile leakage | Explicit requirement | MVP | MVP | P3-03, P1-05 |
| R-9.4 | Safe compatible overrides, coherent IPv4/IPv6, signed-answer integrity, loop and pattern/conflict checks | Explicit requirement | MVP | MVP | P3-04, P3-05, P3-06, P4-02 |
| R-9.5 | DNS listener/upstream/cache/local records/security/query/latency/health settings reflect real state | Explicit requirement | MVP | MVP | P7-04 |
| R-10 | Proxy Gateway and Routing: all mapped child obligations below | Section rollup | See child requirements | FUTURE, MVP, POST_MVP_MULTI_NODE, POST_MVP_PROTOCOLS | F-01, P10-04, P4-01, P4-02, P4-04, P4-05, P4-06, P4-07, P5-04, P9-03 |
| R-10.1 | Controlled TCP target, generation decision, passthrough, quotas/timeouts, SSRF/rebinding prevention and metrics | Explicit requirement | MVP | MVP | P4-02, P4-04, P4-06, P4-07 |
| R-10.2 | Ingress/service/profile/route authorize target independently from SNI; ambiguous/unknown fails closed | Explicit requirement | MVP | MVP | P4-01 |
| R-10.3 | Policy-independent origin resolver avoids self-gateway loop; every re-resolution rechecks forbidden addresses | Explicit requirement | MVP | MVP | P4-02 |
| R-10.4 | Supported local proxy/interface/route/protected-node dispatch selected through actual capability matrix | Proposed architecture | MVP + later | MVP, POST_MVP_MULTI_NODE, POST_MVP_PROTOCOLS | P4-05, P5-04, P9-03 |
| R-10.5 | TCP first with backpressure/FD/allocation limits; UDP separate flow identity/timeout/amplification workstream | Explicit requirement | MVP + future | FUTURE, MVP | P4-04, P10-04, F-01 |
| R-11 | Control Plane, Node Agent, and Synchronization: all mapped child obligations below | Section rollup | See child requirements | MVP, POST_MVP_MULTI_NODE | P1-06, P6-01, P6-05, P6-07, P6-08, P8-01, P8-02, P8-03, P8-04, P8-06, P9-01, P9-02 |
| R-11.1 | API/RBAC/config/registry/compiler/secrets/version/audit/migration/backup control responsibilities | Explicit requirement | MVP | MVP | P6-01, P6-05, P6-07, P6-08 |
| R-11.2 | Local DNS/proxy coordinator preserves last-known-good; enrollment, engine management and remote agent deferred | Explicit requirement | MVP local snapshots; managed agent later | MVP, POST_MVP_MULTI_NODE | P1-06, P8-01, P8-02, P8-03, P8-04 |
| R-11.3 | Install separately, verify controller, one-use token, pending approval, scan, baseline and healthcheck; no permanent admin SSH | Explicit requirement | Post-MVP | POST_MVP_MULTI_NODE | P8-01, P8-06 |
| R-11.4 | Nodes/groups with pinned/healthy strategies; comparable latency/weights later; ping insufficient service proof | Explicit requirement | Post-MVP | POST_MVP_MULTI_NODE | P9-01, P9-02 |
| R-12 | Routira Web Dashboard: all mapped child obligations below | Section rollup | See child requirements | FUTURE, MVP, POST_MVP_MULTI_NODE, POST_MVP_PROTOCOLS | F-04, P1-01, P2-07, P3-07, P5-07, P5-08, P5-09, P6-01, P6-03, P6-04, P6-05, P6-06, P6-07, P6-08, P7-01, P7-02, P7-03, P7-04, P7-05, P7-06, P9-01, P9-04, P9-05 |
| R-12.1 | Responsive navigation covers overview/network/operations/system; API equivalent for every significant action | Proposed architecture | MVP + later | FUTURE, MVP, POST_MVP_MULTI_NODE | P7-01, P7-06, P9-01, F-04 |
| R-12.2 | Fast actual component/traffic/latency/degraded/recent-state dashboard; missing engine metric unavailable rather than zero | Explicit requirement | MVP | MVP | P7-05 |
| R-12.3 | Filtered activity with trace/revision/outcome; DNS and connections distinct, correlated only with reliable evidence | Explicit requirement | MVP | MVP | P6-08, P7-05 |
| R-12.4 | Device/profile/mode/onboarding/config/revoke/export with limitations and no unnecessary secret redisplay | Explicit requirement | MVP | MVP | P6-03, P7-02 |
| R-12.5 | Reusable Profile policy separated from named DNS Endpoint, resolver capability and network gateway grant; one active Profile per Endpoint initially | Proposed architecture | MVP | MVP | P1-01, P6-03, P7-02 |
| R-12.6 | Priority/scope/matches/target/failure/modes, prominent limitations, simulate/validate/draft/publish/diff | Explicit requirement | MVP | MVP | P6-04, P7-03 |
| R-12.7 | Actual node engine/interface/version/health details; external proxy has no fabricated agent/OS telemetry | Explicit requirement | MVP local; remote later | MVP, POST_MVP_MULTI_NODE | P9-01, P7-04, P7-05 |
| R-12.8 | Engine listeners/version/capability/rotation/export UI deferred; DNS/proxy settings delivered in P7-04 | Explicit requirement | Post-MVP protocols | POST_MVP_PROTOCOLS | P5-07, P5-08, P5-09 |
| R-12.9 | DNS upstream/cache/listener/override/rate/privacy/optional TLS certificate management | Explicit requirement | MVP | MVP | P3-07, P7-04 |
| R-12.10 | Versioned editable/import/export domain groups, approved updates, wildcard/CDN overlap analysis and no guaranteed lists | Explicit requirement | MVP manual; subscriptions later | FUTURE, MVP | P6-04, F-04, P2-07 |
| R-12.11 | Policy Explain, DNS Test, Node Reachability and real Route Test with observed egress/reasons/revision | Explicit requirement | MVP + later | MVP, POST_MVP_MULTI_NODE | P6-06, P7-05, P9-05 |
| R-12.12 | History/diff/dry-run/approval/per-node actual states and explicit rollback | Explicit requirement | MVP + later | MVP, POST_MVP_MULTI_NODE | P6-05, P7-06, P9-04 |
| R-12.13 | Users/roles/MFA/private management/certs/tokens/session/retention/secrets/backup/restore/update controls | Explicit requirement | MVP | MVP | P6-01, P6-07, P6-08, P7-06 |
| R-13 | Design System and UX Principles: all mapped child obligations below | Section rollup | See child requirements | MVP | P11-04, P7-01, P7-02, P7-03, P7-05, P7-06 |
| R-13.1 | Minimal light/dark design, honest status colors, technical monospace, reduced motion and functional mobile | Proposed architecture | MVP | MVP | P7-01, P7-05 |
| R-13.2 | Accessible navigation/tables/wizard/badges/diffs/confirmed destructive actions and masked config viewer | Proposed architecture | MVP | MVP | P7-01, P7-02, P7-03, P7-06 |
| R-13.3 | Progressive disclosure never hides leak warnings; direct fallback requires separate explicit consent; terminology clear | Explicit requirement | MVP | MVP | P7-03, P7-06, P11-04 |
| R-13.4 | Configured/applied/healthy/tested/connected are separate states, not inferred from form save | Explicit requirement | MVP | MVP | P7-05, P7-06 |
| R-14 | Device Onboarding and Profiles: all mapped child obligations below | Section rollup | See child requirements | FUTURE, MVP, POST_MVP_PROTOCOLS | F-02, F-03, P0-05, P1-01, P11-04, P2-02, P5-07, P5-09, P6-03, P7-02 |
| R-14.1 | Personal secure DNS or legacy network DNS setup without VPN; router helpers/profile stacks and tunnel exports deferred | Proposed architecture | MVP DNS/proxy; tunnel clients later | FUTURE, MVP, POST_MVP_PROTOCOLS | P0-05, P7-02, P11-04, P5-09, F-02 |
| R-14.2 | Secure resolver capability selects DNS endpoint policy; legacy and transparent gateway use separate network grants; no internet MAC/client-alias/shared-IP device identity | Explicit requirement | MVP | MVP | P1-01, P2-02, P6-03 |
| R-14.3 | Preserve multiple DNS/proxy profiles and protected access; native tunnel/engine URI/QR exports deferred | Explicit requirement | MVP DNS/proxy profiles; engine exports later | MVP, POST_MVP_PROTOCOLS | P7-02, P5-07, P5-09 |
| R-14.4 | Custom client/split routing/leak diagnostics/protocol switching and opt-in telemetry are separate undertaking | Deferred feature | Future | FUTURE | F-03 |
| R-15 | Rust Technology Stack: all mapped child obligations below | Section rollup | See child requirements | FUTURE, MVP, POST_MVP_PROTOCOLS | F-01, P0-01, P0-02, P0-05, P10-01, P10-02, P10-04, P11-01, P5-02, P5-08, P7-01 |
| R-15.1 | Rust/React candidate stack requires pinned mutually compatible dependencies/features and upgrade tests | Proposed architecture | MVP | MVP | P0-02, P0-05, P7-01 |
| R-15.2 | Choose strong typing/memory safety; demonstrate performance by measurement, not language guarantees | Proposed architecture | MVP | MVP | P0-01, P10-04 |
| R-15.3 | First-party Rust manages proven external engines; do not reimplement cryptography; BoringTun only evaluated if justified | Explicit requirement | MVP + later | FUTURE, POST_MVP_PROTOCOLS | P5-02, P5-08, F-01 |
| R-15.4 | MVP services stay unprivileged and expose no OS routing actions; restricted helper mandatory before later tunnel/engine privileges | Explicit requirement | MVP no NET_ADMIN; helper later | MVP, POST_MVP_PROTOCOLS | P10-02, P11-01, P10-01 |
| R-16 | Target repo layout, not required empty crate scaffolding; instantiate cohesive crates only as needed | Proposed architecture | MVP | MVP | P0-02 |
| R-16.1 | Formatting/lints/errors/cancellation/logging/lockfiles/security and actual Linux integration on each applicable change | Explicit requirement | MVP | MVP, POST_MVP_PROTOCOLS | P0-04, P1-07, P10-01, P10-05 |
| R-17 | Domain Model and Data Schema: all mapped child obligations below | Section rollup | See child requirements | FUTURE, MVP, POST_MVP_MULTI_NODE, POST_MVP_PROTOCOLS | F-05, P1-01, P1-02, P1-03, P1-05, P2-07, P5-07, P6-03, P6-07, P9-01 |
| R-17.1 | Typed entities include managed node vs external proxy, engines/credentials, policies/revisions/audit and aggregated metrics | Proposed architecture | MVP + later | MVP, POST_MVP_MULTI_NODE | P1-01, P6-03, P9-01, P2-07 |
| R-17.2 | Node/engine/binding/reference/credential/snapshot/secret/version/identity-evidence invariants preserved | Explicit requirement | MVP | MVP, POST_MVP_PROTOCOLS | P1-02, P1-05, P5-07, P6-03 |
| R-17.3 | SQLite WAL/migrations/backup/bounded writes; PostgreSQL only justified concurrency/HA; minimal aggregate history | Explicit requirement | MVP + future | FUTURE, MVP | P1-03, P6-07, F-05 |
| R-18 | API and Internal Contracts: all mapped child obligations below | Section rollup | See child requirements | MVP, POST_MVP_MULTI_NODE | P1-04, P2-06, P6-01, P6-02, P6-05, P6-06, P6-07, P6-08, P7-03, P8-01, P8-03 |
| R-18.1 | Versioned /api/v1 OpenAPI and common UI/API access; RBAC/CSRF/Origin on every mutation | Explicit requirement | MVP | MVP | P6-01, P6-02 |
| R-18.2 | Endpoint groups are future contracts; security/revocation/deployment/diagnostics/export/backup tests required | Proposed architecture | MVP + later | MVP, POST_MVP_MULTI_NODE | P6-02, P6-05, P6-06, P6-07, P8-01 |
| R-18.3 | Explain includes decision/profile/rule/node/version/warnings and false network_test_performed for simulation | Proposed architecture | MVP | MVP | P2-06, P7-03 |
| R-18.4 | Versioned actor/entity/node/correlation/outcome events never contain tokens/keys/passwords | Proposed architecture | MVP + later | MVP, POST_MVP_MULTI_NODE | P1-04, P6-08, P8-03 |
| R-19 | Security and Privacy: all mapped child obligations below | Section rollup | See child requirements | MVP, POST_MVP_PROTOCOLS | P0-01, P0-05, P1-05, P1-07, P10-01, P10-02, P10-05, P11-04, P11-05, P3-01, P4-02, P6-01, P6-08, P7-05 |
| R-19.1 | Threat model covers amplification/open proxy/SSRF/admin/enrollment/supply chain/rebinding/leaks/fail-open | Explicit requirement | MVP | MVP | P0-01, P10-02, P10-05 |
| R-19.2 | Baseline restrictions apply to enabled features: ACL/auth/address rechecks/limits/private TLS/MFA/RBAC/secrets/snapshots/verified updates. MVP denies privileged OS actions; custom helper required before later privileged tunnel features | Explicit requirement | MVP | MVP, POST_MVP_PROTOCOLS | P1-05, P1-07, P3-01, P4-02, P6-01, P10-01, P10-05 |
| R-19.3 | Aggregate/minimal optional per-domain retention, separated admin audit, no HTTPS MITM or anonymity guarantee | Explicit requirement | MVP | MVP | P6-08, P7-05, P11-04 |
| R-19.4 | Exact engine/crate licenses and distribution conditions reviewed; Routira license remains unselected | Open decision | MVP | MVP | P0-05, P11-05 |
| R-20 | Monitoring, Diagnostics, and Resilience: all mapped child obligations below | Section rollup | See child requirements | FUTURE, MVP, POST_MVP_MULTI_NODE, POST_MVP_PROTOCOLS | F-04, P1-06, P2-04, P2-06, P3-02, P4-07, P5-07, P6-06, P6-08, P7-05, P8-02, P8-04, P9-02 |
| R-20.1 | Actual bounded DNS/gateway/engine/node/security metrics with units and trustworthy identity/version | Explicit requirement | MVP + later | MVP, POST_MVP_MULTI_NODE | P4-07, P6-06, P7-05, P8-02 |
| R-20.2 | Process/transport/route/service health distinct; successful ping/handshake not service acceptance | Explicit requirement | MVP + later | MVP, POST_MVP_MULTI_NODE | P6-06, P8-02, P9-02 |
| R-20.3 | LKG on CP loss; retain invalid version; strict exit fail closed; allowed resolver fallback; expiry/unknown/IPv6 denial | Explicit requirement | MVP + later | MVP, POST_MVP_MULTI_NODE, POST_MVP_PROTOCOLS | P1-06, P2-04, P3-02, P5-07, P8-04 |
| R-20.4 | Stable actionable reason codes for identity/DNSSEC/ECH/QUIC/nodes/engine/ports/egress/loops | Proposed architecture | MVP | MVP | P2-06, P6-06, P7-05 |
| R-20.5 | In-app/log events suffice first; optional external notifications need explicit opt-in, severity and anti-spam | Deferred feature | MVP + future | FUTURE, MVP | P6-08, F-04 |
| R-21 | Deployment and Operations: all mapped child obligations below | Section rollup | See child requirements | FUTURE, MVP, POST_MVP_MULTI_NODE, POST_MVP_PROTOCOLS | F-02, P10-04, P11-01, P11-02, P11-03, P5-08, P5-09, P5-10, P6-01, P6-07, P7-02, P7-06, P8-06, P9-05 |
| R-21.1 | Single VPS first; controller/nodes then home/router topologies with explicit threat boundaries | Proposed architecture | MVP + later | FUTURE, MVP, POST_MVP_MULTI_NODE | P11-01, P8-06, P9-05, F-02 |
| R-21.2 | Native systemd primary; Compose optional constrained; no Docker requirement; deb/rpm later | Explicit requirement | MVP | MVP | P11-01, P11-02 |
| R-21.3 | MVP host/ports/IP/firewall/DNS/time/egress preflight; TUN/AWG/module checks deferred with the feature | Explicit requirement | MVP DNS/proxy; tunnel preflight later | MVP, POST_MVP_PROTOCOLS | P11-01, P5-10 |
| R-21.4 | MVP admin/MFA/DNS/proxy/profile/diagnostics/backup onboarding; node/engine/tunnel provisioning later | Proposed architecture | MVP DNS/proxy; engine onboarding later | MVP, POST_MVP_PROTOCOLS | P6-01, P7-02, P7-06, P11-01, P5-09 |
| R-21.5 | Protocol/TLS/listener endpoints allocated explicitly; same :443 combination not universally compatible | Technical constraint | MVP | MVP, POST_MVP_PROTOCOLS | P5-08, P11-01 |
| R-21.6 | Encrypted backup/recovery before migrations; versioned staged health/rollback and client export compatibility | Explicit requirement | MVP | MVP, POST_MVP_PROTOCOLS | P6-07, P11-03, P5-08 |
| R-21.7 | Measure DNS QPS/flows/throughput/RTT/engine resources; no fixed tiny-VPS streaming capacity guarantee | Technical constraint | MVP | MVP | P10-04 |
| R-22 | Testing and Quality Criteria: all mapped child obligations below | Section rollup | See child requirements | MVP, POST_MVP_MULTI_NODE, POST_MVP_PROTOCOLS | P0-04, P1-02, P10-01, P10-02, P10-03, P10-04, P10-05, P10-06, P11-05, P2-02, P3-03, P3-07, P5-08, P7-07, P9-03 |
| R-22.1 | Unit/property coverage for matching/conflict/capability/schema/cache/auth/redaction/CIDR boundaries | Explicit requirement | MVP | MVP | P1-02, P2-02, P3-03, P10-02 |
| R-22.2 | Real DNS/gateway/peer/Xray/inter-node/crash/LKG/storage integration per enabled feature | Explicit requirement | MVP + later | MVP, POST_MVP_MULTI_NODE, POST_MVP_PROTOCOLS | P3-07, P5-08, P9-03, P10-06 |
| R-22.3 | All 11 limitation cases become observable positive/negative acceptance evidence | Explicit requirement | MVP | MVP | P10-05, P7-07 |
| R-22.4 | Sustained/peak DNS/TCP/slow streams/memory/error/log cardinality/load and resource degradation evidence | Explicit requirement | MVP | MVP | P10-04 |
| R-22.5 | Parser fuzz/SSRF/rebinding/unauthorized/loops/segmentation/RBAC/secrets/SBOM/license checks | Explicit requirement | MVP | MVP, POST_MVP_PROTOCOLS | P10-01, P10-02, P10-03, P10-05, P11-05 |
| R-22.6 | Backend/frontend/dependency/release CI; kernel tests on isolated trusted runners, never arbitrary PR root production | Explicit requirement | MVP | MVP | P0-04, P11-05 |
| R-23 | Development Phases and MVP Scope: all mapped child obligations below | Section rollup | See child requirements | FUTURE, MVP, POST_MVP_MULTI_NODE, POST_MVP_PROTOCOLS | F-01, F-02, F-03, F-04, F-05, P0-01, P0-02, P0-05, P1-05, P11-06, P2-06, P3-06, P4-08, P5-10, P6-01, P7-07, P8-06, P9-05 |
| R-23.1 | Foundation contracts/security/test harness precede broad UI; no implementation during planning session | Explicit requirement | Foundation | MVP | P0-01, P0-02, P1-05, P2-06, P6-01, P0-05 |
| R-23.2 | User-approved DNS/proxy MVP supersedes original first-release WG/AWG/Xray ordering; those integrations remain explicit post-MVP work | Explicit requirement | MVP amended by CR-0001 | MVP, POST_MVP_PROTOCOLS | P3-06, P4-08, P7-07, P11-06, P5-10 |
| R-23.3 | Secure enrollment/sync, groups/outbounds/failover/path diagnostics and managed/external separation | Explicit requirement | Post-MVP | POST_MVP_MULTI_NODE | P8-06, P9-05 |
| R-23.4 | Justified extra engines/UDP/TUN/complex policy/router integration require separate approved gates | Deferred feature | Future | FUTURE | F-01, F-02 |
| R-23.5 | Optional client/inbound switch/subscriptions/analytics/updates/HA/PostgreSQL only when justified | Deferred feature | Future | FUTURE | F-03, F-04, F-05 |
| R-23.6 | P0 correctness/security before protocol/UI priorities; avoid simultaneous architecture rewrite and new engine | Explicit requirement | All phases | FUTURE, MVP | P0-01, P11-06, F-01 |
| R-24 | Definition of Done and Product Decisions: all mapped child obligations below | Section rollup | See child requirements | MVP, POST_MVP_PROTOCOLS | P0-01, P0-05, P10-07, P11-05, P11-06, P5-10, P7-07 |
| R-24.1 | DNS/proxy/security/install/restore/status checklist applies now; original WG/AWG/Xray/isolation items deferred to P5-10 | Explicit requirement | MVP checklist amended by CR-0001 | MVP, POST_MVP_PROTOCOLS | P7-07, P10-07, P11-06, P5-10 |
| R-24.2 | Retain Rust/React/SQLite/Hickory/TCP/Linux/fail-closed; three engine integrations are post-MVP under CR-0001; license open | Proposed architecture | MVP DNS/proxy; protocols later | MVP, POST_MVP_PROTOCOLS | P0-01, P0-05, P11-05, P5-10 |
| R-25 | Risks, Trade-offs, and Open Questions: all mapped child obligations below | Section rollup | See child requirements | MVP | P0-01, P0-05, P10-04, P10-05, P11-05, P4-01 |
| R-25.1 | SmartDNS expectations/ECH/SVCB/bridge/AWG/helper/ports/NAT/IPv6/license/updates/scope risks have owners and tests | Risk | All phases | MVP | P0-01, P0-05, P10-05, P11-05 |
| R-25.2 | User confirmed multi-admin roles; distro/public DNS/ports/IPv6/services/license/clients/failure defaults remain registered choices | Open decision | Before dependent tasks | MVP | P0-01, P0-05, P4-01, P10-04 |
| R-25.3 | Nine suggested ADR areas covered by 15 proposed ADRs with alternatives/reasons/risks/validation | Proposed architecture | Foundation | MVP | P0-01 |
| R-26 | Use consistent glossary; distinguish protocol/engine, DNS/traffic, entry/exit and fail-closed semantics | Proposed architecture | All phases | MVP | P11-04 |
| R-27 | Revisit primary sources before pinning dependencies/engines and implementation; citations are not proof of compatibility | Proposed architecture | All phases | MVP | P0-02, P0-05 |

## Baseline security restrictions

| ID | Source obligation (unchanged) | Kind | Effective scope | Task deliveries | Work items |
|---|---|---|---|---|---|
| R-19.2-01 | By default, DNS listens only on trusted addresses/networks; public DNS requires explicit configuration and access restrictions. | Explicit base requirement | MVP | MVP | P3-01, P7-07, P10-07 |
| R-19.2-02 | Explicit proxies require authentication or equally reliable network isolation. | Explicit base requirement | MVP | MVP | P4-01, P7-07, P10-07 |
| R-19.2-03 | The Gateway does not accept arbitrary destinations without service/route authorization checks. | Explicit base requirement | MVP | MVP | P4-01, P7-07, P10-07 |
| R-19.2-04 | Deny loopback, private, link-local, multicast, metadata, and the system's own endpoints (unless a narrowly limited administrative override applies). | Explicit base requirement | MVP | MVP | P4-02, P7-07, P10-07 |
| R-19.2-05 | Verify destinations **after every DNS resolution**, not only by the domain string. | Explicit base requirement | MVP | MVP | P4-02, P7-07, P10-07 |
| R-19.2-06 | Enforce rate limits, timeouts, per-client quotas, and circuit breakers. | Explicit base requirement | MVP | MVP | P10-04, P7-07, P10-07 |
| R-19.2-07 | The administrative dashboard must not be internet-exposed without authentication, TLS, and protective controls; prefer a private management network. | Explicit base requirement | MVP | MVP | P6-01, P7-07, P10-07 |
| R-19.2-08 | MFA, RBAC, auditing, secure session cookies, and CSRF/Origin protection. | Explicit base requirement | MVP | MVP | P6-01, P7-07, P10-07 |
| R-19.2-09 | Encrypt secrets at rest using a sound key-management scheme; do not store an encryption key alongside encrypted data without considering the threat model. | Explicit base requirement | MVP | MVP | P1-07, P7-07, P10-07 |
| R-19.2-10 | Root/system capabilities belong only to a privileged helper with a restricted API. | Explicit base requirement | MVP denies OS network actions; custom helper before privileged G3 features | MVP, POST_MVP_PROTOCOLS | P10-02, P11-01, P10-01 |
| R-19.2-11 | Configuration snapshots are signed/authenticated and checked for rollback and sequence violations. | Explicit base requirement | MVP | MVP | P1-05, P7-07, P10-07 |
| R-19.2-12 | Obtain updates from verified sources, with pinned versions/checksums/signatures where available. | Explicit base requirement | MVP | MVP | P11-05, P7-07, P10-07 |

## Original MVP checklist with amended applicability

| ID | Source obligation (unchanged) | Kind | Effective scope | Task deliveries | Work items |
|---|---|---|---|---|---|
| R-24.1-01 | Users can install Routira on a supported Linux VPS using the documentation. | Explicit base requirement | MVP | MVP | P11-01, P7-07, P10-07 |
| R-24.1-02 | DNS/proxy do not become open public resolvers/proxies by default. | Explicit base requirement | MVP | MVP | P3-01, P7-07, P10-07 |
| R-24.1-03 | The UI lets users add a DNS profile, device, and rule and test the rule. | Explicit base requirement | MVP | MVP | P7-02, P7-07, P10-07 |
| R-24.1-04 | DIRECT and BLOCK DNS policies work and produce explainable reasons. | Explicit base requirement | MVP | MVP | P3-04, P7-07, P10-07 |
| R-24.1-05 | The TCP SmartDNS proxy transparently communicates ECH/QUIC/IPv6 limitations. | Explicit base requirement | MVP | MVP | P7-03, P7-07, P10-07 |
| R-24.1-06 | Configuration versions are validated, applied atomically, and rolled back. | Explicit base requirement | MVP | MVP | P1-06, P7-07, P10-07 |
| R-24.1-07 | WireGuard inbound is managed via a Rust adapter and tested. | Explicit base requirement | Post-MVP protocols under CR-0001; original checkbox pending | POST_MVP_PROTOCOLS | P5-04, P5-10 |
| R-24.1-08 | AmneziaWG inbound is managed via a Rust adapter and tested with pinned client/engine versions. | Explicit base requirement | Post-MVP protocols under CR-0001; original checkbox pending | POST_MVP_PROTOCOLS | P5-05, P5-10 |
| R-24.1-09 | Xray/VLESS+REALITY inbound is managed via a Rust adapter and tested. | Explicit base requirement | Post-MVP protocols under CR-0001; original checkbox pending | POST_MVP_PROTOCOLS | P5-06, P5-10 |
| R-24.1-10 | A normal Xray crash does not stop DNS or WG. | Explicit base requirement | Post-MVP protocols under CR-0001; original checkbox pending | POST_MVP_PROTOCOLS | P5-08, P5-10 |
| R-24.1-11 | Secrets are hidden in APIs/logs/exports. | Explicit base requirement | MVP | MVP | P1-07, P7-07, P10-07 |
| R-24.1-12 | Backup and documented restore have been verified in practice. | Explicit base requirement | MVP | MVP | P11-03, P7-07, P10-07 |
| R-24.1-13 | Tests cover NAT, IPv6, DNSSEC override, ECH, node failures, SSRF, and DNS loops. | Explicit base requirement | MVP | MVP | P10-05, P7-07, P10-07 |
| R-24.1-14 | The UI distinguishes `configured`, `applied`, `healthy`, and `tested`. | Explicit base requirement | MVP | MVP | P7-05, P7-07, P10-07 |

## Network limitation acceptance matrix

| ID | Source case and expected result | Kind | Effective scope | Task deliveries | Work items |
|---|---|---|---|---|---|
| R-22.3-01 | Ordinary TCP/TLS with visible SNI → Gateway works for authorized domain | Explicit base requirement | MVP supported DNS/proxy boundary; DNS-only endpoint limits explicit | MVP | P10-05, P7-07 |
| R-22.3-02 | ECH / unavailable original hostname → Predictable failure or supported alternative path; **never** incorrect routing | Explicit base requirement | MVP supported DNS/proxy boundary; DNS-only endpoint limits explicit | MVP | P10-05, P7-07 |
| R-22.3-03 | HTTP/3/QUIC in DNS-only mode → Explicitly `unsupported/limited`, with no false guarantees | Explicit base requirement | MVP supported DNS/proxy boundary; DNS-only endpoint limits explicit | MVP | P10-05, P7-07 |
| R-22.3-04 | Signed DNS RRset + local override → Modified response is not claimed to have valid DNSSEC | Explicit base requirement | MVP supported DNS/proxy boundary; DNS-only endpoint limits explicit | MVP | P10-05, P7-07 |
| R-22.3-05 | IPv6 when an IPv4 proxy policy applies → No bypass of strict policy | Explicit base requirement | MVP supported DNS/proxy boundary; DNS-only endpoint limits explicit | MVP | P10-05, P7-07 |
| R-22.3-06 | Multiple clients behind NAT → No false per-client authorization | Explicit base requirement | MVP supported DNS/proxy boundary; DNS-only endpoint limits explicit | MVP | P10-05, P7-07 |
| R-22.3-07 | Origin DNS resolves to private IP → Block or enforce additional policy | Explicit base requirement | MVP supported DNS/proxy boundary; DNS-only endpoint limits explicit | MVP | P10-05, P7-07 |
| R-22.3-08 | Policy update fails → Retain previous working version | Explicit base requirement | MVP supported DNS/proxy boundary; DNS-only endpoint limits explicit | MVP | P10-05, P7-07 |
| R-22.3-09 | Node outage → Apply precisely the selected fail-closed/fallback behavior | Explicit base requirement | MVP proxy/component failure; remote node outage at G4 | MVP, POST_MVP_MULTI_NODE | P4-06, P10-06, P9-02 |
| R-22.3-10 | Split-tunnel client → Clearly distinguish client-side DIRECT from server-side DIRECT | Explicit base requirement | Post-MVP split-tunnel clients under CR-0001 | FUTURE, POST_MVP_PROTOCOLS | P5-04, P5-10, F-02 |
| R-22.3-11 | Revoked credentials → New connections are no longer accepted after confirmed revocation deployment | Explicit base requirement | MVP DNS/proxy access revocation; peer credential tests at G3 | MVP, POST_MVP_PROTOCOLS | P6-03, P4-08, P5-07, P5-10 |

## Accepted user scope amendment

| ID | Accepted instruction | Kind | Effective scope | Task deliveries | Work items |
|---|---|---|---|---|---|
| R-CR-0001 | Release DNS/proxy MVP without VPNs, managed engines, TUN or a custom privileged networking helper; retain real later protocol acceptance | Accepted user instruction | Overrides first-release ordering in §§23.2/24.1/24.2; no implementation approval | MVP, POST_MVP_PROTOCOLS | P4-08, P7-07, P11-06, P5-10 |

## Proposed Control D reference design

| ID | Proposed design | Kind | Effective scope | Task deliveries | Work items |
|---|---|---|---|---|---|
| R-CR-0002 | Separate Profiles, DNS Endpoints, resolver capabilities, filter/service revisions, private records and gateway grants; personal DoH/DoT in revised MVP | Proposed design | Authorized research and plan edits; technical acceptance pending P0-01 | MVP | P1-01, P2-07, P2-02, P3-07, P4-01, P6-03, P7-02, P7-04 |

Total mapping records (including rollups, checklist details, accepted amendment and proposed design): **188**.

Task status is recorded in backlog.json. A mapping or deferral is neither implementation nor a passed acceptance test.
