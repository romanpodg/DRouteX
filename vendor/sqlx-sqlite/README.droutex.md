# SQLx SQLite driver patch

Owner-approved source patch for P1-03, conditional on successful qualification.
Original package: `sqlx-sqlite 0.9.0` from the official crates.io archive.
Archive SHA-256: `488e99c397a62007e4229aec669a179816339afc6d2620ca6fa420dbee2e982c`.
Release source commit: `003b698e99e024f3621b8043a2426fde5b741171`.

The original code/build change widened the `libsqlite3-sys` upper bound:
`<0.38.0` became `<0.39.0`, matching
[upstream commit 94aafe3](https://github.com/launchbadge/sqlx/commit/94aafe3a68884d923b0798a767c8d7f6cfda89d2).
On 2026-11-10 the owner approved qualification and conditional adoption of
libsqlite3-sys 0.39.1 / SQLite 3.54.1 for
[RUSTSEC-2026-0459](https://rustsec.org/advisories/RUSTSEC-2026-0459).
The same manifest line now requires `>=0.39.1, <0.40.0`; both consuming
lockfiles pin 0.39.1. This follow-up is a DRouteX constraint change, not the
original upstream patch. No Rust driver source or feature selection changes.
Cargo.toml.orig remains the unmodified package's upstream manifest for provenance.

The packaged LICENSE-APACHE and LICENSE-MIT contain relative pointer text.
They are replaced here with the complete root license texts from the exact
release commit above. Copyright/authorship and licensing remain MIT OR Apache-2.0;
DRouteX's own Apache-2.0 license does not relabel this dependency.

SOURCE.json records original archive identity, the three changed package files
and SHA-256 fingerprints of the intended snapshot, normalized to LF so Git's
platform newline conversion does not report a false source change. The separate
readme and provenance manifest are additions. Run
`python tests/storage-dependencies/verify_vendor.py` before building.

Remove this local patch after an official SQLx release permits the fixed native
dependency and passes compatibility, license/native-source and independent review
checks. Do not replace it with a floating Git dependency or an unreviewed update.
