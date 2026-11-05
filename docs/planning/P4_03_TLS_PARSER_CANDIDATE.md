# P4-03 TLS parser candidate — review pending

The owner authorized the P4-02 through P4-04 tasks while P3-06 remains open.
The owner separately authorized isolated qualification of the candidate below.
Subsequent owner authorization covers continued implementation. The current
worktree integrates this parser behind the non-default `experimental-tls`
feature for a preview listener; it does not grant production readiness. The
independent security audit and real DNS-to-TLS acceptance remain outstanding.

Candidate for isolated qualification: `tls-parser` 0.12.2. The crates.io
metadata declares MIT/Apache-2.0 and Rust 1.63, compatible with the current
license allowlist and Rust 1.99 MSRV. This is metadata, not a completed license,
advisory, maintenance or independent security audit. Archive source inspection
found a generic `Unknown` extension representation and an old draft ESNI name;
standard ECH extension 0xfe0d is not named by that release. Qualification must
prove the raw extension type is retained even with fragmentation, and explicitly
reject ECH or another ambiguous encrypted-name signal; SNI alone is never access
or routing authority. It must also cover malformed/duplicate extensions, exact
record/handshake size and time bounds, complete parsing, byte preservation,
fragmentation across records and reads, cancellation and allocation limits.

Rustls 0.23.46 already has a separate prior TLS qualifier but no production
adoption for this role. Its public Acceptor ClientHello view exposes server name,
ALPN and other selected fields, but the inspected view does not expose arbitrary
extension types. That observation is not proof that it can safely reject ECH
without further qualified support. Neither candidate is approved here.

Before production adoption, inspect the actual resolved transitive dependency and
license graph and notices, current advisories, feature profile, Linux/Windows
behavior and independent review; preserve the existing dependency gates. A parser
error or unavailable qualification cannot become an ordinary direct route.

## Isolated qualification in progress

On 2026-11-03 the owner explicitly authorized isolated qualification of
`tls-parser` 0.12.2. The separate `tests/tls-parser-qualification` workspace
pins that exact release with default features off; at that time it was not a
dependency of any production crate. Crates.io identifies it as not yanked and gives SHA-256
`22c36249c6082584b1f224e70f6bdadf5102197be6cfa92b353efe605d9ac741`.
The current isolated lock contains 33 third-party packages, including build and
procedural-macro dependencies. `cargo deny` passed advisories, bans, licenses
and sources after advisory fetch; it warned about duplicate `syn` 1 and 2.
No independently completed security audit of this parser has been established.

The fixed `client-hello.hex` fixture is a public ClientHello generated using
Python 3.12 `ssl.MemoryBIO` with OpenSSL 3.5.8 and the synthetic hostname
`origin.example.test`; it contains no private key. SHA-256 of the hex file is
`866cd405919d002ac751dacf02b75d3f317f53a2ff2734b872b658665692ec33`.
Sixteen local Linux tests pass, including every byte split of that handshake
across two TLS records, every TCP prefix, preservation of original bytes,
0xfe0d ECH as an unknown raw extension, old ESNI, duplicate/truncated extensions,
record and buffer ceilings, loopback TCP fragmentation and an incomplete-hello
deadline, and 3052 deterministic byte mutations without panic.
This finite corpus is not a fuzzing or memory-exhaustion proof.

Raw parser behavior has two concrete integration hazards: an incomplete SNI
entry after a valid entry is silently discarded; and unparsed bytes inside a
ClientHello's declared handshake body are discarded by
`parse_tls_message_handshake`. Raw parsing also accepts an empty cipher-suite list
and a ClientHello without the null compression method. Its extension-list parser may
also return success
with remaining malformed bytes. Callers must not treat its success as validation
of all input. The isolated test-only composition explicitly checks full handshake
body and SNI-list consumption, rejects empty cipher lists, missing null compression,
duplicate extensions, ECH and ESNI, and
bounds input and record counts. A mutation removing its ECH check makes its ECH
regression fail. This fixture is not production code and does not prove correct
handling of every TLS extension, hostname, stream cancellation, quotas, timeouts,
concurrent connections, gateway cancellation, or verified gateway admission. Source alone does not
establish the independent audit required by P4-03.

Local rustfmt, Clippy and locked tests pass; the qualifier declares no default
features, so the default and no-default invocations select the same profile. Windows execution and current-head
external review/CI remain unverified. The proposed workflow changes would run
the isolated tests on Windows and Linux and check the lock's dependency graph;
they are not evidence that CI has run. Qualification and production adoption
remain open, and P3-06/P4-03 must continue to fail closed.

The first local Clippy attempt failed on a test-helper style lint; after using
`as_chunks`, the same Clippy command passed. A later quality-validator attempt
failed because the Graphify activation path selected a Python without PyYAML.
The local activation was corrected to expose only the Graphify executable, and
the validator then passed. These corrections do not alter the recorded failed
attempts or establish remote CI execution.

On 2026-11-04 the 0.12.2 isolated qualifier again passed all 16 Linux tests,
and locked offline cargo-deny passed with duplicate-`syn` and unused-license
allowlist warnings. A public search found the upstream project's documentation
and fuzzing claims, but did not establish a relevant independent audit of this
version or composition; see the [upstream repository](https://github.com/rusticata/tls-parser)
and [version documentation](https://docs.rs/tls-parser/0.12.2/tls_parser/).
The available environment has only the Linux Rust target and no Wine runtime;
no Windows result is claimed. The raw parser's documented permissive cases
remain reasons not to switch on production TLS handling.

On 2026-11-05 the owner supplied a screenshot reporting that all 16 isolated
TLS parser tests passed on Windows in a clean worktree at merge commit
`836d655`, with no failures or skipped tests. This is owner-reported evidence;
the Windows version, invoked commands, feature profile and raw logs were not
supplied. The earlier statements that Windows execution was unverified describe
the state before this report. This result does not constitute an independent
parser audit or production TLS/gateway acceptance.
