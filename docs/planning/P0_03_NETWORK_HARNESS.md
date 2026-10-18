# P0-03 network harness review

Date: 2026-10-16. Status: **Complete: WSL and native Debian 13 fixture evidence passed**.
The owner instructed the next roadmap step after P0-02 and explicitly approved
the existing WSL Ubuntu, Python stdlib/OpenSSL, namespaces and required lab packages.
No P0-04/P0-05 work, production networking or privileged PR runner is authorized.

## Implementation and scope

[The lab](../../tests/network/README.md) provides seven isolated namespaces with
dual-stack clients a/b/mismatch, edge SNAT, DNS/gateways, a genuine TLS origin and
a TLS-authenticated HTTP CONNECT fixture. There is no production engine or Rust
testkit placeholder. No third-party Python or Rust package was added. The accepted
protected outbound for the product still belongs to D19/P0-05.

Only the manual setup/capture is privileged. Socket servers and probes use uid/gid
65534, bounded concurrency/input/deadlines and no-new-privileges. Private mount,
network and PID namespaces are checked before mutation; /run and secrets use tmpfs.
The independent timeout bounds a namespace tree to 60 seconds plus a 5-second kill
grace. The scripts neither install a CA nor disable TLS verification. Exact trust,
artifact and hard-kill limitations are documented with the lab command.

## Actual evidence

Local successful report: `.setup/network/be4cca357efdb71a/report.json`.
It and its pcap remain untracked; no keys/capabilities/raw traffic are published.
This is fixture evidence, **not DRouteX DNS/gateway/security acceptance**.

| Environment | Actual value |
|---|---|
| OS | WSL Ubuntu 26.04 LTS, x86_64 |
| Kernel | 6.18.33.2-microsoft-standard-WSL2 |
| Python interpreter | 3.14.4 |
| OpenSSL library | 3.5.5, 27 Jan 2026 |
| iproute2 package | 6.19.0-1ubuntu1 |
| nftables package | 1.1.6-1 |
| openssl package | 3.5.5-1ubuntu3.5 |
| python3 metapackage | 3.14.3-0ubuntu2; interpreter version recorded separately |
| tcpdump package | 4.99.6-1 |
| util-linux package | 2.41.3-3ubuntu2.2 |

Four portable unprivileged boundary tests passed on Windows. The full WSL run
recorded **41 protocol observations**, including both address families:

- Actual UDP/TCP DNS frames; personal DoH POST and DoT with valid, verified TLS.
- Distinct Profiles a/b behind the same source, and the same capabilities from a
  changed source. Legacy DNS remains a shared network context; unknown secure
  credentials and unauthorized legacy TCP are denied.
- Direct and protected-proxy gateway paths preserve the origin certificate and
  expose different actual origin-side peer addresses. Wrong hostname/untrusted CA,
  missing proxy authentication and arbitrary target requests are denied.
- After stopping the proxy process, new protected-path attempts fail; no direct
  fallback succeeds. These are fixture controls, not implemented DRouteX controls.
- Pcap confirms DNS source 198.18.0.1 versus TCP source 198.18.12.2, and IPv6
  fd42:726f:7574::1 versus fd42:726f:7574:c::2. The other-source gateway is denied.

Capture: 1,641 packets, IPv4 and IPv6, snaplen 128, at most 5,000 packets per run.
SHA-256: `aabf72ace5afd544245eb60be745baea3bd9972d9da98227f0270907a30d472c`.
Immediate capture mode prevents buffered final IPv6 packets from being lost at stop.

| Lifecycle case | Actual exit | Evidence |
|---|---:|---|
| Normal | 0 | All protocol probes pass; processes/namespaces removed |
| Injected partial setup failure | 23 | Failure reported; processes/namespaces removed |
| SIGTERM of namespace tree | 143 | Bounded graceful cleanup; repeated TERM cannot interrupt teardown |
| SIGKILL of namespace tree | -9 | Private mounts/PID namespace release; no fixture process remains |

Host links, addresses, IPv4/IPv6 routes, named namespaces and nftables state matched
before/after the run. The hard-kill case targets the namespace tree; it is not a
machine power-loss test or a guarantee that killing the outer supervisor removes
every empty temporary-directory entry. Secrets remain in private tmpfs.

Tested source SHA-256 (before publication):
run.py `7b2a5155b5ac563f9d777e3a2e4c09728f5a84f18573c55993363546dca33417`;
fixtures.py `87e5cc02883facb67314709a3bcc58e7c99ab576b203f0ef25c19ff3ca9a33a6`.
Earlier runs exposed and fixed ambiguous ip-link syntax, nftables statement
separators, strict CA key usage, capture buffering and repeated-TERM teardown.
Failures were not treated as passes; no TLS/security gate was relaxed.

## Native Debian 13 acceptance: 2026-10-16

The owner supplied a separate test VPS and explicitly approved installing
tcpdump 4.99.5-2/libpcap0.8t64 1.10.5-2, copying the reviewed fixture files and
executing all four lifecycle modes. SSH host identity was verified against the
fingerprint supplied through the provider console. Private SSH keys and connection
details remain local and untracked. No other package was upgraded or removed.

| Environment | Actual value |
|---|---|
| OS / virtualization | Debian 13.4 (trixie), x86_64, KVM VPS |
| Resources observed | 1 vCPU, 1,973 MiB RAM, 25 GiB root filesystem |
| Kernel / package | 6.12.85+deb13-amd64 / linux-image-6.12.85+deb13-amd64 6.12.85-1 |
| Python / OpenSSL | 3.13.5 / OpenSSL 3.5.5, 27 Jan 2026 |
| Packages | python3 3.13.5-1; openssl 3.5.5-1~deb13u2; iproute2 6.15.0-1; nftables 1.1.3-1; tcpdump 4.99.5-2; libpcap0.8t64 1.10.5-2; util-linux 2.41-5 |

The provider's original image checksum was not supplied. Measured installed-state
identity: /etc/os-release SHA-256
`55179f383ae6c3edc9238d185f2dfa73bf8d048ce0273b2be9adb77128432b26`;
kernel image SHA-256
`e175037d019cd29d00bf43282bf6494e51ec71bf540965c13a1b94323f7e4a59`.
The two source hashes above matched on the VPS before execution; no fixture source
change was needed. Root-owned source was placed under
`/opt/droutex-p003-20261016-5628f2a` with read/traverse access for the unprivileged
services. Namespace topology/capture alone used root.

All **41 protocol observations** passed again, including both address families,
same-NAT distinct secure Profiles, changed-source secure DNS, source-mismatch
gateway denial, original origin certificate, observed direct/proxy egress,
invalid TLS trust/auth/target denial and strict proxy outage without DIRECT fallback.
The normal run captured **1,621 packets** with both source-mismatch pairs. Normal,
setup-failure, SIGTERM and SIGKILL returned 0/23/143/-9 respectively, with zero
remaining fixture processes. Host interfaces, addresses, routes, named namespaces
and nftables state matched before/after. A subsequent SSH check found no fixture
processes/named namespaces and only SSH listening on TCP 22, as before.

Local evidence: `.setup/network/debian13-20261016-69e86b4354ac8cf4/`.
Report SHA-256: `79166809c50bf6adc4b2f4f70b2e67ed247e34cbd10e38bbc0e8146bfd146fa9`.
Pcap SHA-256: `cd1ada9a8cb692572efd6dbb1cfb70bc7b8a32c9e0c735130f3d7cf572d85696`.
Downloaded report/capture hashes matched the VPS; local copies are restricted to
the Windows owner and remain ignored. The VPS retains the source, installed lab
packages and root-private evidence for reproducibility; no fixture service remains.

After recording completion, planning render/validation passed with nine invalid
task cases rejected, and all 15 planning regression tests passed. The accepted
three-task execution boundary, later-task stop and original EN/RU bytes were
preserved. No new remote CI, Codecov or CodeScene result is claimed.

P0-03 is complete. This closes native fixture readiness, not DRouteX service,
systemd deployment, host crash/power-loss or product security acceptance. The Rust
workspace remains three crates. P0-04/P0-05 and runtime services remain planned.
