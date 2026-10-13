# Veyra

Self-hosted SmartDNS and traffic routing platform. Design and requirements: [Veyra_Project_Specification_RU.md](Veyra_Project_Specification_RU.md).

Current state: policy core, CLI, DNS engine and gateway (stage 1a, plan 3 of 4 — see `docs/superpowers/plans/`).

## Build and test

```bash
cargo test --workspace
cargo clippy --workspace --all-targets -- -D warnings
```

## Try the policy

```bash
cargo run -p veyra-cli -- validate configs/examples/home.yaml
cargo run -p veyra-cli -- explain --config configs/examples/home.yaml --profile home login.media.example
cargo run -p veyra-cli -- explain --config configs/examples/home.yaml --device work-laptop git.corp.example
cargo run -p veyra-cli -- explain --config configs/examples/home.yaml --source 192.168.1.50 media.example
```

## Run the DNS engine

```bash
cargo run -p veyra-dataplane -- --config configs/examples/home.yaml
dig @127.0.0.1 -p 10053 media.example   # 203.0.113.10: proxied, sent to the de-01 gateway
dig @127.0.0.1 -p 10053 ads.example     # 0.0.0.0: blocked
dig @127.0.0.1 -p 10053 example.org     # forwarded to 1.1.1.1
```

Only sources listed under `admission:` get answers; everyone else is ignored. Windows `nslookup` ignores `-port`: to use it, set `listen: ["127.0.0.1:53"]` in a copy of the config and run `nslookup media.example 127.0.0.1`.

## Run the gateway

The example's `gateway:` section makes the same process de-01's gateway: TLS on 10443 routed by SNI and HTTP on 10080 routed by `Host`, for the names DNS sends to de-01. The gateway resolves origins through the `default` upstream group, dials them on 443 and 80, and never dials private, loopback or other special-purpose addresses. TLS is passed through, not terminated: the client sees the origin's own certificate.

The example's names are made up. To see a real passthrough, route a real domain to de-01 in a copy of the config (`domains: ["suffix:example.com"]`), start `veyra-dataplane` with it, and run:

```bash
curl --resolve example.com:10443:127.0.0.1 https://example.com:10443/
```

Connections for other names, and from sources not under `admission:`, are closed at once. On Linux the gateway serves at most one connection per 8 files the process may open (`ulimit -n`, `LimitNOFILE` under systemd), so a stock limit of 1024 allows 128; raise the limit to serve more. On a server, the firewall must reject, not drop, traffic to ports the gateway does not serve: the kernel's TCP reset and ICMP port unreachable are what make clients fail fast (spec §10.1).
