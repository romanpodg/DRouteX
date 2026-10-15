# P0-03 network harness review

Date: 2026-10-15. Status: **In review: WSL fixture evidence passed; native Debian 13 pending**.
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
This is fixture evidence, **not Routira DNS/gateway/security acceptance**.

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
  fallback succeeds. These are fixture controls, not implemented Routira controls.
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

## Remaining gate

No native Debian 13 VM/VPS or immutable image was supplied. WSL does not establish
native Debian/kernel/systemd compatibility. Keep P0-03 **in_review** until this
trusted lab is run on the agreed native target with its exact image/kernel/package
identity and reviewed evidence. No new host, Docker service or cloud cost was created.
The Rust workspace remains three crates; P0-04/P0-05 and runtime services remain planned.
