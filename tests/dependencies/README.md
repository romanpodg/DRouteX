# P0-05 dependency qualification

This independent test package qualifies owner-approved candidates; it is not a
production dependency, workspace member, reusable proxy or running DRouteX service.
Its own lockfile pins all registry artifacts. The production workspace stays free
of third-party dependencies until a consuming implementation task adopts them.

With Rust 1.99.0 and an existing OpenSSL executable on PATH:

```sh
python tests/dependencies/prepare_certificates.py .setup/dependency-certificates
export DROUTEX_QUALIFICATION_CERT_DIR="$PWD/.setup/dependency-certificates"
cargo fmt --manifest-path tests/dependencies/Cargo.toml -- --check
cargo clippy --manifest-path tests/dependencies/Cargo.toml --all-targets --locked -- -D warnings
cargo test --manifest-path tests/dependencies/Cargo.toml --locked
cargo deny --manifest-path tests/dependencies/Cargo.toml --locked check
```

On Windows first dot-source `tools/activate.ps1`, generate certificates with the
existing WSL OpenSSL if needed, and set the environment variable with PowerShell.
All certificates/private keys are disposable, valid for one day and kept outside
Git. No trust store is changed. Tests bind only ephemeral IPv4 loopback ports.
If a sandbox prohibits loopback or native compiler access, run the same authorized
tests in the normal development environment; do not disable certificate checks.

Three tests check an independent DNS vector and truncated input, trusted TLS plus
numeric CONNECT bytes/407/binary payload, and wrong hostname/untrusted issuer.
Both TLS peers are owned by a single five-second deadline. The synthetic CONNECT
fixture does not dial the numeric destination. It qualifies library interoperability,
not production authorization, DNSSEC, origin pinning or arbitrary proxy parsing.
The independent Python/OpenSSL reference proxy is exercised by the separate trusted
[network lab](../network/README.md). No privileged lab runs in PR CI.
