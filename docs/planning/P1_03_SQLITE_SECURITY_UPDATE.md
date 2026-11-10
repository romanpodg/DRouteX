# P1-03 SQLite security update

Date: 2026-11-10. Baseline: PR127 head
`e2a3327959becd6937371779ece5635d3070a9f4`. The owner explicitly approved
qualification of libsqlite3-sys 0.39.1 / SQLite 3.54.1 on Windows and Debian,
with adoption conditional on successful checks. SQLx 0.9.0, Tokio 1.53.3,
the accepted feature profile, storage contract and license policy are unchanged.

## Trigger and scope

Run [42851496750](https://github.com/romanpodg/DRouteX/actions/runs/42851496750)
failed its dependency/coverage job on
[RUSTSEC-2026-0459](https://rustsec.org/advisories/RUSTSEC-2026-0459),
alias CVE-2026-119375. Codecov upload was consequently skipped. All nine other
jobs passed. This failure is retained; it is not a flaky test or an exemption.

Both the production storage graph and isolated storage qualifier selected
libsqlite3-sys 0.38.2, which bundles affected SQLite 3.53.2. The advisory describes
uninitialized page-cache memory disclosure when a process opens an attacker-written
database and sibling WAL with mismatching page sizes. DRouteX uses WAL, so the
dependency is relevant; no reachable remote DRouteX exploit was established.
Private database permissions remain necessary and are not a substitute for fixing
the bundled library. No advisory suppression or source/license exception is added.

The existing SQLx vendored manifest now requires `>=0.39.1, <0.40.0`, with both
consuming lockfiles selecting exactly 0.39.1. No driver Rust source changes.
The application's startup gate also requires exactly SQLite 3.54.1; the gate
continues to reject an unexpected native library rather than being removed.
This is a locally authorized follow-up to the original upstream constraint patch,
not a claim that upstream commit 94aafe3 contains this new range. The original
archive manifest is retained; SOURCE.json and its verifier identify the new range.

## Provenance and licenses

- Official registry archive SHA-256:
  `14a5f01b87b92705d07b433c06f96cbc764637d81a0bffb5734ce17cf0f9a2b3`.
  Both lockfiles and the downloaded archive match.
- Bundled `sqlite3/sqlite3.c` SHA3-256:
  `e2b869d3003b33c942b7dd7063604fbc53553e1b66609570df39b9b7b558ee81`,
  matching the [official SQLite 3.54.1 release](https://sqlite.org/releaselog/3_54_1.html).
- Registry license remains MIT; packaged notice/license hashes are updated in
  [qualification](../operations/SQLITE_QUALIFICATION_INVENTORY.csv) and
  [storage](../operations/STORAGE_INVENTORY.csv) inventories. No other package row
  changes. These historical inventories do not claim to be a complete current
  shipped-target notice bundle.
- Declared libsqlite3-sys MSRV is 1.88.0; checks use the project's pinned Rust
  1.99.0. No compiler or dependency feature change is required.

## Verification

Windows and unprivileged Debian 13.7 x86_64 each pass the seven existing native
storage qualification tests, qualifier formatting and Clippy with denied warnings.
The version test requires actual `sqlite_version()` 3.54.1. Other cases exercise
WAL/foreign keys and disabled extension loading, parameters/constraints, migration
rollback/checksums, transaction isolation/rollback, SQLITE_FULL recovery, bounded
BUSY/recovery and deadlines/cancellation/read-only access.

Fresh cargo-deny checks of the workspace and storage qualifier pass advisories,
bans, licenses and sources, exit 0, using RustSec database
`28d73fc95e5972af3403225a217def57dcaba3c4`. Existing duplicate warnings remain.
The first local audit attempt failed because its temporary std-replacement-data
cache was not a Git repository; using a dedicated temporary directory succeeded.
That setup failure is not counted as a passing audit.

Windows workspace all-target/all-feature Clippy and formatting pass. The selected
all-feature application bundle passes 41 core, 50 gateway, 13 security and 10
storage tests, plus nine compile-fail doctests. This does not repeat the earlier
Windows DNS IPv6 tests or erase their recorded host socket failures.

The first full Debian application run passed DNS (123), gateway (51), core (60)
and the preceding configuration/policy/security cases, then failed 13 of 23 storage
tests. The application's startup check still required SQLite 3.53.2 and correctly
returned Unavailable for the new library. The native qualifier alone did not catch
this integration omission. Updating that exact version gate is part of this fix;
the failed run is retained and the relevant native bundle is rerun.

After the startup correction, the focused Debian storage run passes all 23 tests;
Windows storage formatting, Clippy and all ten applicable tests also pass again.
Separate local standards/spec review passes found no remaining concrete defect.

The final native Debian `cargo test --workspace --all-features --locked` run
passes, including 60 core, 123 DNS, 51 gateway and 23 storage tests, the existing
Linux-only OpenSSL check and all workspace doctests. Workspace formatting and
all-target/all-feature Clippy pass; focused storage Clippy also passes after the
version-gate correction. All final runs report zero failed, ignored or filtered
tests. Debian execution uses the same ordinary UID 1000 and zero effective
capabilities recorded in the [negative-path report](P3_06_NEGATIVE_PATH_REVIEW.md).

Current-head hosted CI/account review remain separate gates, with exact head and
results recorded in PR127. Qualification does not constitute an independent
native-code security audit or acceptance of unselected SQLite/SQLCipher features.
