# P0-03 isolated network fixtures

This is a trusted, manual Linux lab, not DRouteX DNS/gateway implementation or
product acceptance. Review the scripts before privileged execution. Never wire
this command to arbitrary PR code, a production host or a production root runner.
The owner approved development in the existing WSL Ubuntu on 2026-10-15 and the
separate Debian 13 VPS lab on 2026-10-16. Both fixture runs passed; see the recorded
evidence below. Product networking and systemd acceptance remain later work.

Requirements: Python 3.12+, OpenSSL CLI, iproute2, nftables, tcpdump, util-linux
(unshare/mount/setpriv) and GNU timeout. No Python/Rust dependency is added.
Use the distribution's signed packages on an explicitly approved disposable lab.

From the repository root in that lab:

```sh
sudo python3 -I tests/network/run.py
```

On this development host the approved equivalent is:

```powershell
wsl -d Ubuntu -u root -- python3 -I /mnt/c/Users/roman/Desktop/DRouteX/tests/network/run.py
```

Portable, unprivileged checks (also suitable for fork CI):

```sh
python3 -m unittest discover -s tests/network -p 'test_*.py'
```

## Topology and boundaries

The supervisor creates an outer private mount/network/PID namespace before any
network mutation. The inner setup refuses execution unless PID 1 has a different
mount/network namespace and only loopback. Its /run and secret directory are
private tmpfs. No listener, interface, route or firewall rule is added to the host.
Host links/addresses/routes/named namespaces/nftables state are compared before/after.

Seven nested namespaces model clients a/b/mismatch, edge NAT, DNS/gateways, origin
and proxy. A/B share exact IPv4 and IPv6 SNAT addresses; mismatch has a separately
routed source. Only internal links exist; the data-plane namespace has no internet
default route. Addresses are isolated fixture ranges, not permission for a production
gateway to dial reserved/private targets. No TUN, VPN, Docker or engine is required.

All protocol servers and client probes run as uid/gid 65534 with no-new-privileges.
Only topology setup and bounded tcpdump capture are privileged. There are at most
eight workers per service, finite parser/queue/backlog limits and socket deadlines.
The external timeout bounds each namespace tree to 60 seconds plus 5 seconds to kill.

Roles use high fixture-only ports: DNS 5353, DoT 8853, DoH 8443, origin TLS 9443,
protected proxy 9444, direct gateway 8444 and proxy gateway 8445. The fixture DNS
parser intentionally supports only uncompressed single-question A/AAAA packets.
DoH is bounded HTTP/1.1 POST, not an HTTP/2 or general HTTP implementation.
The gateway is a fixed test relay, not the future DRouteX SNI/binding engine.

Each run creates a private CA and separate origin/resolver/proxy certificates with
OpenSSL, checks CA/hostname trust and the original origin certificate fingerprint,
and generates ephemeral resolver/proxy credentials. It never installs a trusted CA
on the host or disables certificate verification. CONNECT accepts only the exact
numeric fixture origin/port after authentication over verified TLS. This fixture
does not select or approve P0-05/D19's production proxy transport.

## Evidence and cleanup

The run checks both address families: UDP/TCP DNS, secure endpoint Profiles across
shared NAT and changed source, unknown capabilities, source mismatch denial, direct
and protected-proxy TLS forwarding with observed origin-side source addresses,
untrusted CA/wrong hostname, proxy authentication/target denial and proxy outage
without direct fallback. Pcap pairs confirm the distinct DNS and TCP source case.

Normal completion, an injected partial setup failure, SIGTERM and SIGKILL of the
namespace tree must leave no fixture processes or named namespaces. PID namespace
exit and private mounts provide the hard-kill backstop. Repeated TERM is ignored
only after the first interruption starts bounded teardown. If the outer supervisor
itself is killed, the independent timeout bounds remaining children; an empty host
temporary-directory entry or partial ignored artifact may remain, but keys/config
are in private tmpfs. This is not a host crash/power-loss acceptance test.

Reports and a bounded, snaplen-128 pcap are written under `.setup/network/<run>/`,
which stays untracked. Pcaps can contain ephemeral DNS capability metadata; keep
them local. Keys/config disappear with the private mount. Do not publish raw captures
or substitute fixture passes for DRouteX authorization/security tests.
See [recorded evidence](../../docs/planning/P0_03_NETWORK_HARNESS.md).

The isolation/lifetime design uses [unshare's namespace semantics](https://man7.org/linux/man-pages/man1/unshare.1.html)
and [ip-netns lifecycle](https://man7.org/linux/man-pages/man8/ip-netns.8.html).
TLS uses [Python's SSL library](https://docs.python.org/3.12/library/ssl.html), not custom cryptography.
