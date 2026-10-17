# P0-05 DNS/proxy dependency inventory

Date: 2026-10-16. Status: **P0-05 qualification complete; no production adoption**.
The owner selected HTTP CONNECT over TLS and approved this exact candidate set for
isolated compatibility and transitive-license checks. Original Routira code remains
Apache-2.0. The owner separately approved Apache-2.0, MIT, ISC, BSD-3-Clause and
Unicode-3.0 for dependencies, with their texts and notices preserved. The owner
separately approved Zlib on 2026-10-17, preserving copyrights/notices and marking
modified sources. Original Routira licensing is unchanged. Other licenses
and new dependencies retain owner review. No legal compatibility opinion is implied
by a technical SPDX/policy check.

## Exact candidates and provenance

| Crate | Version | Registry license expression | Declared Rust minimum | Archive SHA-256 |
|---|---|---|---|---|
| [Tokio](https://docs.rs/crate/tokio/1.53.3) | 1.53.3 | MIT | 1.71 | ce3335fa71841cda333a58d7615b03901380ecf09d59b3296d21f8bbac0dde4e |
| [Hickory-Proto](https://docs.rs/crate/hickory-proto/0.26.3) | 0.26.3 | MIT OR Apache-2.0 | 1.88 | 12b92608f679a6fa515dd1d15c1ff89443026e391200a2c840c7afcba482893d |
| [Rustls](https://docs.rs/crate/rustls/0.23.46) | 0.23.46 | Apache-2.0 OR ISC OR MIT | 1.71 | 48e13bd8c0e9365c43cfa5c9e8f9ad49d3c8444926c9aac819e0e4dc503c8fdf |
| [Tokio-Rustls](https://docs.rs/crate/tokio-rustls/0.26.6) | 0.26.6 | MIT OR Apache-2.0 | 1.71 | c9cc2678c2cdd569ef8215e2afd7954ada2ae20b4fdd2c5fe6139a3b02d105db |

The exact official crates.io version metadata and static.crates.io archives were
downloaded on 2026-10-16. SHA-256 matched the registry checksum for all four; none
was yanked. Cargo.toml and license files were inspected from the archives before
building. Registry checksums establish artifact consistency, not an independent
publisher signature. Cargo's locked resolution verifies transitive checksums.
The approved compiler/MSRV tested here is Rust 1.99.0, Edition 2024; upstream minima
are declarations, not tests on those older compilers.

The intentional [qualification package](../../tests/dependencies/README.md) is outside
the production workspace. Its [Cargo.lock](../../tests/dependencies/Cargo.lock) is
the resolution authority: 107 registry packages, plus the test package. The
[inventory](DEPENDENCY_INVENTORY.csv) records every locked package's version, SPDX
expression, registry checksum and hashes of top-level license/notice files. It
includes target-specific packages not built on Windows/Linux and optional lockfile
entries; it must not be read as the list linked into a shipped binary. The actual
target feature graph must be recorded again at production adoption/distribution.
Four target-only packages (jni, jni-macros, jni-sys-macros and r-efi) have license
metadata but no top-level license/notice text in their downloaded package; their
inventory text-hash cell is intentionally empty. They are not built for either
tested Windows/Linux target. Do not distribute those targets until exact upstream
texts/attribution are obtained and preserved; revisit if the supported target set
or dependency feature graph changes. A metadata-only license pass is not notice proof.

Tokio enables only rt-multi-thread/macros/net/io-util/time for the probe; no full,
process or io-uring feature. Hickory uses std defaults for wire parsing; this does
not adopt its resolver, DNSSEC policy or an application-level IDNA contract.
Rustls/Tokio-Rustls retain their default AWS-LC and TLS 1.2/1.3 configuration;
Rustls's default post-quantum preference remains enabled. No alternate crypto
provider, FIPS claim, custom verifier or TLS interception is introduced.

## Transitive licenses and audit decisions

AWS-LC resolves to aws-lc-rs 1.18.2 and aws-lc-sys 0.46.0. Their compound SPDX
expressions contain **AND** obligations; accepting one license is insufficient.
The native aggregate LICENSE covers AWS-LC/BoringSSL/OpenSSL/other contributions,
including Apache-2.0, ISC, MIT and BSD-3-Clause. Preserve that complete license and
all referenced notices at redistribution; do not relabel the native code Apache-only.
Unicode/ICU data/code introduces Unicode-3.0 obligations. OR alternatives may be
satisfied by an approved permissive option; the policy does not approve LGPL,
Unlicense, CC0, LLVM exceptions or every alternative merely because metadata lists it.

Initial cargo-deny 0.20.2 against Apache-only policy exited **4**: advisories, bans
and sources passed; licenses failed. After explicit owner approval, a fresh advisory
fetch and the full locked check exited **0**: all four categories passed. No advisory
exception, unlicensed exception, source allowlist bypass or suppression was added.
Known advisory status is dated evidence, not a guarantee against unknown defects.

The all-target graph reports duplicate syn 2.0.119/3.0.9: Android-only JNI macros
require the older line while current proc macros require the newer line. Keep both
locked for qualification instead of forcing an incompatible upgrade or suppressing
the finding. Revisit at production target selection or the next dependency update;
the target-specific graphs, binary contribution and notice inventory must be checked.
No non-approved target is thereby supported.
Windows/Linux cargo tree --duplicates shows no competing syn versions; getrandom
0.4.3 appears in separate host-build and target-runtime feature contexts, not as
two locked versions. Retain these contexts; no public type crosses between them.

## Actual tests and remaining evidence

Windows x86_64/MSVC with the project-local Rust 1.99.0 built the approved defaults
without installing CMake/NASM or another compiler; AWS-LC used the existing native
toolchain/package-provided assets. Three qualification tests passed: independent DNS
wire vector/truncation, trusted TLS with numeric CONNECT/407/binary bytes, and wrong
hostname/untrusted CA rejection. The TLS server is a bounded loopback test peer,
not a production outbound connector or independent interoperability reference.
Sandboxed socket/native compiler access failed; running the same authorized local
test outside that restriction passed. The tracked package also passed Clippy with
warnings denied. Linux qualification passed in CI. Each of the three tests was
observed failing with an intentionally corrupted expectation, then restored; no
mutation is present in tracked source. The final tracked-source run used its own
build directory to avoid reusing the exploratory package's mutation-test cache.

[Actions run 38867986135](https://github.com/romanpodg/Routira/actions/runs/38867986135)
passed all seven jobs on `1ed74f933fdbfc5cf3265a4a4c82648fd1278234`, including
Linux qualification (3 tests, fmt/Clippy), both platform workspace/planning jobs,
both locked dependency inventories and OIDC uploads. The exact head's Codecov report
is processed; Rust config remains 5/8 instrumented lines, not aggregate tool coverage.
The independent read-only built-in Codex Code Review completed on 2026-10-16 with
no confirmed defect. Its sandbox prevented repeating every test/advisory command;
the actual local and CI results above provide that execution evidence. An earlier
review was interrupted by account limits and is not counted as completed.

[CodeScene 7981306](https://codescene.io/projects/86145/delta/results/7981306)
failed only the new test's Large Method score (9.60); two other gates passed.
Tests are outside the owner's independent metric-improvement scope. No functional
defect was identified, suppression added or score-only refactor performed; the
repository-wide check remains failed. The disposition and target-only notice gaps
are recorded in [OPEN_DECISIONS](../planning/OPEN_DECISIONS.md#p0-05-review-dispositions).

Existing OpenSSL **3.5.5** in the approved Ubuntu WSL lab generated disposable CA
and proxy identities. No root trust store, deployment certificate, ACME account or
private key lifecycle was configured. The helper uses standard OpenSSL commands;
all keys/certificates stay in ignored .setup. Operator certificate provisioning,
renewal, trust source and restart behavior remain D21/P3-07/P11 decisions.

The independent Python stdlib/OpenSSL network fixture previously passed the
protected numeric CONNECT path, proxy auth/target denials, dual-stack origin TLS
certificate/egress checks and strict outage handling in P0-03. The fresh P0-05 WSL
run on 2026-10-16 again passed all 41 observations, normal/setup-failure/SIGTERM/
SIGKILL cleanup, zero remaining processes and unchanged host networking. Report
SHA-256: `163eb09cdba791addaeb2d04f51163228d79c0efc7a62654720fd60e15adba31`.
This is fixture interoperability evidence, not Routira implementation proof.
Native Debian 13 product, systemd, origin-validation and real external-proxy acceptance
remain later gates.

## Dataset and update inventory

No operator filter/service dataset has been supplied or bundled. There is therefore
no dataset source/license/hash to attest and no claimed malware/adult/service coverage.
The accepted [product model](../architecture/DNS_PRODUCT_MODEL.md) requires bounded
local imports at P2-07: source/owner/license/format/version/hash plus validation and
last-known-good rollback, with revisions pinned in snapshots. A missing feed is
unavailable, never an empty successful protection feature. Remote feed downloading
and proprietary categories are outside the accepted MVP. Do not copy Control D data.

Only controlled official registry updates are eligible; change exact versions and
lockfile together, inspect source/checksum/license/advisory/feature changes, rerun
qualification and review before adoption. Do not use cargo update as silent approval.
At P11-05, generate distribution notices from the actual shipped target tree, retain
complete third-party texts/copyrights and native bundled notices, then verify the
release archive contains them. This task does not distribute dependency binaries.
Managed-engine versions/licenses/packaging remain explicitly deferred in the
[engine matrix](../architecture/ENGINE_MATRIX.md).

## Subsequent P1-01 identity dependency

After P0-05 completion the owner separately selected UUIDv4 resource IDs and
qualification of uuid 1.28.0. The [domain review](../planning/P1_01_DOMAIN_REVIEW.md)
and [UUID inventory](UUID_INVENTORY.csv) record that std parse/format-only adoption.
The 107-artifact inventory above remains the P0-05 network qualification snapshot;
it is not the production UUID target tree or a release notice bundle. The network
candidate stack remains outside production crates.

## Subsequent P1-02 configuration dependencies

The owner separately approved parser/path qualification and the strict input
profile. The [configuration review](../planning/P1_02_CONFIG_REVIEW.md) records
compatibility, feature limits and adoption in routira-config only. The
[config inventory](CONFIG_INVENTORY.csv) covers all 51 current production-lock
registry archives and notice hashes; it supersedes neither historical snapshot
nor the release requirement to include complete target-specific notices. Raw
parser errors are never exported; all three tested parsers can echo input values.
