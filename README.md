# Veyra

Self-hosted SmartDNS and traffic routing platform. Design and requirements: [Veyra_Project_Specification_RU.md](Veyra_Project_Specification_RU.md).

Current state: policy core and CLI (stage 1a, plan 1 of 4 — see `docs/superpowers/plans/`).

## Build and test

```bash
cargo test --workspace
cargo clippy --workspace --all-targets -- -D warnings
```

## Try it

```bash
cargo run -p veyra-cli -- validate configs/examples/home.yaml
cargo run -p veyra-cli -- explain --config configs/examples/home.yaml --profile home login.media.example
cargo run -p veyra-cli -- explain --config configs/examples/home.yaml --device work-laptop git.corp.example
```
