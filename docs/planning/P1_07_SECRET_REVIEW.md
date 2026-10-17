# P1-07 secret references and external key lifecycle review

Date: 2026-10-18. Status: **isolated qualification in progress; production adoption pending**.
Base: verified P1-04/PR #7 merge `644348cfe53a4b76f8a921cf3e4886b2486e0cb8`.
P1-05 requires both P1-04 and P1-07; P1-07 is the next dependency-ready task.

## Owner decisions and boundaries

D30: external 256-bit master key provisioned through a protected file outside the DB
and backup directories. Secret operations refuse a missing/wrong key. New writes use
the active rotated key; retain old keys for decryption/re-encryption and historical
backup restoration. No implicit startup key generation or keys inside DB/backups.

D31: separately qualify aws-lc-rs 1.18.2 and zeroize 1.9.1 with AES-256-GCM-SIV
([RFC 8452](https://www.rfc-editor.org/rfc/rfc8452.txt)), fresh random nonces and
record ID/purpose authentication, then adopt only after successful checks. Cipher
implementation is from the library. D32: owner approved official packaged NASM
objects for Windows development; Linux uses package source C/assembly. No NASM
installation, native library override, FIPS module or compliance claim is authorized.

The standalone tests do not implement a Routira encrypted store or accept real
secrets. Exact envelope/key-registration, import/export limits and protected
persistence interfaces still require their design gate before dependent code.
Resolver token verification/lifetimes/wire format remain P3-07; snapshot authority
P1-05 and API/RBAC P6. AEAD authentication alone does not prevent a database rollback
or prove freshness, authorization, backup safety or redaction of a future API.

## Initial qualification evidence

Six Windows probes passed, including all 24 official AES-256-GCM-SIV RFC vectors,
full ciphertext/tag corruption, wrong key/nonce/record context, truncation and
invalid sizes. OS-random and explicit zeroization are smoke checks, not proofs
about entropy or residual copies. fmt and warnings-denied Clippy passed. Logs:
`.setup/p1-07/windows-qualification.log`, `clippy.log`, `features.log`.

Locked inventory contains 15 registry artifacts, with archive SHA256 matching both
Cargo.lock and official crates.io version metadata; none is yanked. Packaged
license/notice hashes are in [the qualification inventory](../operations/SECRET_QUALIFICATION_INVENTORY.csv).
All four cargo-deny categories passed after advisory refresh; only unused approved
license warnings remain. No duplicate packages in the current target tree.

aws-lc-sys 0.46.0 bundles AWS-LC 5.11.0; the official
[release](https://github.com/aws/aws-lc-rs/releases/tag/v1.18.2) identifies that native
version. Native source and packaged Windows object provenance are recorded in the
[package readme](../../tests/secret-dependencies/README.md); independent binary-to-source
reproducibility is not claimed. The official [Rust wrapper](https://github.com/aws/aws-lc-rs/security/advisories)
and [native advisories](https://github.com/aws/aws-lc/security/advisories) list five
March 2026 X.509/CRL/PKCS7/AES-CCM findings each; their corrected wrapper versions
0.38/0.39 and native 1.69/1.71 precede the locked 0.46.0/native 5.11.0. Those APIs are
outside this AEAD profile; no advisory exception or suppression was added. This is
a point-in-time review of published advisories, not a guarantee of no vulnerabilities.

The first minimal Windows build lacked NASM. After owner D32, the upstream
prebuilt-nasm feature compiled and probes passed without new tools. Exact-profile
actual Linux CI and independent Code Review must complete before production adoption.
