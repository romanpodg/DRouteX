# P2-07 bounded WebPKI security qualification

Owner decision D42 (2026-10-21) authorizes qualification and conditional adoption of
exactly rustls-webpki0.103.16 in the existing standalone TLS qualifier. Successful
Windows/Linux checks, full dependency/license checks and independent linked-account
review are required. This is a PR16 qualification candidate, not production TLS
adoption. The production workspace lockfile and existing11-job workflow are unchanged.

## Reason and exact artifact

The original [CI39485189479](https://github.com/romanpodg/DRouteX/actions/runs/39485189479)
failed before coverage on RUSTSEC-2026-0382/CVE-2026-111943 in WebPKI0.103.15.
The [official advisory](https://github.com/RustSec/advisory-db/blob/main/crates/rustls-webpki/RUSTSEC-2026-0382.md)
describes acceptance of paths containing distrusted emSign intermediate keys.
[Upstream PR546](https://github.com/rustls/webpki/pull/546) adds SPKI distrust
throughout the path, including leaves and trust anchors; stable0.103.16 is patched.
The failed run remains evidence; no advisory/check suppression or status exception.

Official crates.io/static.crates.io archive SHA-256:
`4443b1b6d39d7c663139942cc4558262562d2c84123bb6282ef177009ca8b281`.
It matches registry metadata and the candidate lockfile. The ISC license is already
allowed; LICENSE SHA-256 is
`5b698ca13897be3afdb7174256fa1574f8c6892b8bea1a66dd6469d3fe27885a`.
The declared minimum is Rust1.71; actual compiler tests use approved Rust1.99.0,
not that older minimum. Registry checksums establish consistency, not independent
publisher authentication.

Only the WebPKI version/checksum changes in tests/dependencies/Cargo.lock. All106
other registry packages, direct versions, features and AWS-LC provider are retained.
All107 archives matched lockfile checksums;7107 extracted files matched those
archives. Existing106 package license/notice hashes and SPDX expressions matched
the prior inventory, including its disclosed target-only notice gaps. The
[inventory](DEPENDENCY_INVENTORY.csv) updates only the WebPKI row; the
[original qualification](DEPENDENCIES.md) remains dated historical evidence.

## Actual isolated checks

Windows/MSVC: existing fmt and warnings-denied Clippy passed; all3 original
DNS/TLS/CONNECT positive/negative groups passed against the candidate. Disposable
one-day certificates came from the previously approved WSL OpenSSL; no trust store
was changed. A fresh four-category cargo-deny passed with existing duplicate
warnings, no new exception. Fresh locked/offline four-category checks also passed
for all5 existing manifests after fetching the current advisory DB.

The published source/LICENSE matches upstream release commit
`8ecdcc76f56b68fda67a6adf7976f5eed5006a4c`. The official GitHub source archive
SHA-256 is `00713e09dcb3c082df981fd4ee1e6e490b830e7eead6befe9df77c34cad31335`.
The crate archive omits test_utils, so the exact upstream repository source was
used for the original unit test `verify_cert::tests::distrusted_spki`.
Only an empty workspace table and isolated test lock resolution were adjusted;
AWS-LC was aligned to already qualified1.18.2/sys0.46.0. No upstream Rust source
modification is used for the successful test. All76 test-only registry archives
were checksum-verified; cargo-deny passed under the existing license policy.
Missing packaged notice texts for asn1-rs-impl/yasna and target-only wasip2 were
obtained from their exact official VCS commits. Target-only r-efi retains its
metadata/AUTHORS notice limitation and is not built/distributed for these tests.
No new dependency, test-only lockfile, upstream source or notice gap enters the
production workspace or standalone TLS qualifier.

Temporarily bypassing both distrust checks in the isolated copy made that test
fail (exit101); restoring the exact original source made it pass again. No mutation
is present in the successful source or tracked files.

The upstream test passed on Windows: valid/unrelated paths succeed, distrusted
root/intermediate/leaf/shared cross-signed SPKIs fail, and an alternate valid
cross-sign remains usable. This is synthetic offline chain validation, not live
emSign traffic or a downloaded compromised private key.

Reproduction in an isolated copy of that exact official source after dependency
and license checks:

```sh
cargo test --manifest-path <isolated-source>/Cargo.toml --locked --lib --features aws-lc-rs distrusted_spki
```

Linux/current-head CI and independent implementation/final reviews remain pending.
The correction is not accepted as complete until that evidence is recorded in the
[P2-07 review](../planning/P2_07_CATALOG_REVIEW.md). No coverage success is inferred
from the failed original run or the local qualifier.
