# MVP transport and engine inventory

Date: 2026-10-16. Accepted scope: [ADR-0016](adr/ADR-0016-dns-proxy-mvp-before-tunnels.md)
and [CR-0002](adr/ADR-0017-profile-endpoint-dns-product-model.md).
This is a capability inventory, not a claim of implemented runtime support.

| Path | MVP treatment | Evidence / remaining gate |
|---|---|---|
| Restricted DNS UDP/TCP | Native Rust; Hickory-Proto 0.26.3 wire-format candidate | P0-05 parser qualification; actual forwarding/cache/policy is P3 |
| Personal DoT/DoH | Native TLS candidate: Rustls 0.23.46 / Tokio-Rustls 0.26.6 | TLS compatibility test only; endpoint capability, HTTP protocol and certificate lifecycle stay P3-07/P6 |
| Authorized TCP gateway → DIRECT | Native stream path; DIRECT means exit at gateway VPS | Independent lab reference exists; actual binding/origin validation/passthrough is P4-01..04 |
| Gateway → protected proxy outbound | **Owner selected HTTP CONNECT over TLS on 2026-10-16** | Exact proxy certificate/hostname validation, credentials only inside TLS, checked numeric CONNECT destination; actual implementation P4-05/P4-08 |
| Public explicit inbound proxy | Separate unapproved access/product choice | No listener or unrestricted forwarding authorized by selecting an outbound |
| SOCKS5 or unprotected CONNECT outbound | Not selected for first MVP connector | No plaintext credential transport or implicit fallback; further transport requires a decision |
| WireGuard/AmneziaWG | Post-MVP P5 | No engine/kernel/client version accepted or supported by this inventory |
| Managed Xray/sing-box, Shadowsocks, Hysteria, TUIC, Trojan, OpenVPN | Post-MVP protocol/engine gates | No binary import, distribution, supervisor, crypto implementation or implied capability |
| TUN/UDP relay/privileged routing helper/multi-node | Post-MVP P10-01/P5/P8/P9 | No NET_ADMIN, root runtime or generic shell executor |

CONNECT semantics follow [RFC 9110 §9.3.6](https://www.rfc-editor.org/rfc/rfc9110.html#section-9.3.6).
The TLS connection authenticates the configured proxy; it does not authenticate an
arbitrary origin or replace Routira's route authorization. The connector must send
the checked numeric IPv4/IPv6 address, avoid proxy-side hostname re-resolution, and
keep end-to-end origin TLS bytes unchanged. Authentication/verification/connection
failure must never silently become DIRECT. Production deadlines, address bans,
ingress/grant isolation, revocation and secret storage retain their owning gates.

[Control D custom rules](https://docs.controld.com/docs/custom-rules) are the DNS/redirect
product reference. Its documented IP/proxy redirection does not specify Routira's
outbound implementation; HTTPS CONNECT is an explicit Routira choice, not a claim
about Control D's internal transport. No proprietary catalog or code was copied.

Exact candidates, artifact hashes, licensing, build evidence and limitations are in
[DEPENDENCIES](../operations/DEPENDENCIES.md). A qualification pass does not advertise
any P3/P4 runtime capability as available.
