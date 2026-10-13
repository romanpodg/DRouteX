# Veyra

Self-hosted SmartDNS and traffic routing platform. Design and requirements: [Veyra_Project_Specification_RU.md](Veyra_Project_Specification_RU.md).

Current state: policy core, CLI and DNS engine (stage 1a, plan 2 of 4 — see `docs/superpowers/plans/`).

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
