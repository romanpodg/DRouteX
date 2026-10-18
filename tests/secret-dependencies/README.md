# Secret dependency qualification

Standalone test package, outside the production workspace. No production encryption,
key provisioning, persistence or secret-export interface is adopted by this package.

Owner-approved candidates: aws-lc-rs 1.18.2, defaults off, non-fips/alloc/prebuilt-nasm;
zeroize 1.9.1, defaults off, alloc. AES-256-GCM-SIV uses the library implementation,
not a DRouteX cipher. The non-FIPS profile makes no FIPS compliance claim. Nonce
constants/repeated keys in official vectors are test-only; production must request
a fresh OS-backed random nonce and bind the approved record context.

The owner separately approved upstream packaged NASM objects for Windows development
without installing NASM. The official aws-lc-sys 0.46.0 crate checksum covers these
objects as well as AWS-LC 5.11.0 source (upstream commit fa9bc8d6f7cfb2cf849b247fb06bbbfc41ecc9d6).
There are 26 packaged objects; SHA256 of sorted UTF-8 `name:SHA256` lines, LF-separated without final newline:
`22733ecfb50e0636cb920f4062c1934034e1cb201817e7359d8adc3f2b2c7c7d`. Reproducible object-to-source equivalence was
not independently demonstrated. Linux builds native C/assembly from package sources.
No external system crypto-library override or FIPS module is selected.

`rfc8452_vectors.rs` contains all 24 AES-256 vectors from RFC 8452 section C.2,
[official plaintext](https://www.rfc-editor.org/rfc/rfc8452.txt), downloaded source
SHA256 `ca48ec466b401ce5d68d6d1628127a1800adcc16f2573646aaa3d7df284d3c17`. Test vectors are intentionally public; no production key,
token or user credential is present. Six tests cover golden encryption/decryption,
every ciphertext/tag byte, altered key/nonce/AAD, truncation, invalid sizes and
OS-random/explicit-zeroization smoke checks. The last check proves neither entropy
quality nor absence of copies in registers, allocator pages, dumps or OS buffers.

Run with the pinned repository toolchain:

```sh
cargo fmt --manifest-path tests/secret-dependencies/Cargo.toml -- --check
cargo clippy --manifest-path tests/secret-dependencies/Cargo.toml --all-targets --locked -- -D warnings
cargo test --manifest-path tests/secret-dependencies/Cargo.toml --locked
cargo deny --manifest-path tests/secret-dependencies/Cargo.toml --locked check
```

The [inventory](../../docs/operations/SECRET_QUALIFICATION_INVENTORY.csv) fingerprints
all 15 registry archives and their packaged notices; actual release target notices
remain P11-05. Current validation/advisory limits are in the
[P1-07 review](../../docs/planning/P1_07_SECRET_REVIEW.md).

## P1-05 signature qualification

Owner D34 additionally approved isolated Pure Ed25519 qualification in the same
locked profile. The `snapshot_auth` target covers all five public RFC8032 section7.1
vectors and message/key/signature/import negatives. See the
[P1-05 review](../../docs/planning/P1_05_SNAPSHOT_REVIEW.md) for source hash and scope.
Both existing Windows/Linux CI jobs run this target. No production signature API,
snapshot codec, key format, replay or rotation policy is adopted here.

## D35 strict-verifier qualification

The owner approved ed25519-dalek exactly3.0.0 with all default/optional features
disabled, retaining AWS-LC signing. `strict_snapshot_auth` qualifies library
weak-key refusal, strict individual verification and canonical public-key import
by comparing library point re-encoding to the supplied bytes. ZIP-215 parsing alone
does not establish canonicality. No custom curve arithmetic, blacklist or ordinary
verification fallback is used. The test wrapper is not the production trust API.

Six tests cover the five Pure Ed25519 RFC8032 vectors, AWS-LC cross-verification
with the DRouteX domain, per-byte tampering, malformed sizes, small-order keys and
aliases, a nonweak noncanonical key, noncanonical scalars/points and weak R. The
identity-forgery differential explicitly demonstrates AWS-LC acceptance and strict
verification refusal without a signing private key. Known public vector seeds
remain test-only. These checks do not prove all possible curve/subgroup properties.

[SNAPSHOT_VERIFIER_INVENTORY](../../docs/operations/SNAPSHOT_VERIFIER_INVENTORY.csv)
fingerprints all35 locked registry archives/notices, including the20 additions;
the original15 versions/checksums are preserved. Exact Windows/Linux execution and
review evidence are in [P1-05](../../docs/planning/P1_05_SNAPSHOT_REVIEW.md).
Production adoption is conditional on successful qualification and independent
review; this package alone supplies no durable replay, deployment or runtime proof.
