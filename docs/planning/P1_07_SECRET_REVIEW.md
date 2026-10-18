# P1-07 secret references and external key lifecycle review

Date: 2026-10-18. Status: **isolated qualification passed; implementation in review; final exact-head checks pending**.
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

The standalone tests do not implement a DRouteX encrypted store or accept real
secrets. The envelope, keyring, import/export limits and protected persistence
profile subsequently passed the owner's D33 design gate before dependent code.
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
actual Linux CI and independent Code Review subsequently passed as recorded below.

## Published qualification and independent review

[Draft PR #8](https://github.com/romanpodg/DRouteX/pull/8), qualification head
`203c76f11a2e745c74d9c3cfe95d3b6396ea8796`, passed all 11 jobs of
[run 39056761899](https://github.com/romanpodg/DRouteX/actions/runs/39056761899).
Both actual Windows and Linux secret jobs passed six probes/24 official vectors,
fmt and warnings-denied Clippy; the dependency job passed the locked secret audit.
Production workspace tests also passed. No production crypto dependency or encrypted
store was added by this qualification.

The first independent built-in review stopped at quota and is not completed review.
Its retry against verified base `644348cfe53a4b76f8a921cf3e4886b2486e0cb8` completed
with exit 0 and no confirmed defects, using the signed-in ChatGPT account with
`sandbox_mode=read-only` and `approval_policy=never`. Reviewer session:
`01a14b2e-2c51-7d43-a310-83d061b93ed8`; local log/exit:
`.setup/p1-07/codex-review-qualification-retry.log` / `.exit`.
The reviewer verified formatting, planning/CI boundaries and every vector against
the checksum-verified RFC source. Fresh Clippy/tests could not write target in the
read-only sandbox; separate exact-head CI supplied that evidence. Post-result
rollout-recording errors are retained in the log and do not replace its completed
review result. This reviews the qualification, not a future secret implementation.

Codecov processed two report sessions for the exact head and base comparison;
backend remains 1635/1746. Both patch checks succeeded. The changed covered lines
are six auxiliary planning lines (6/6), not encrypted-store product coverage.
Raw local evidence: `.setup/p1-07/codecov-qualification.json` and
`codecov-compare-qualification.json`.

[CodeScene 7999538](https://codescene.io/projects/86145/delta/results/7999538)
failed two gates: duplicate qualification-test code (9.39) and complexity of
`ValidatorTests.test_foundation_acceptance_and_execution_limit_are_required`
(10.00 to 9.69). One other gate passed. These excluded test/auxiliary metric
signals remain failed, with source-based disposition P1-07-CS-AUX in
[OPEN_DECISIONS](OPEN_DECISIONS.md#p1-07-review-dispositions); no suppression or
score-only rewrite was made.

## Remaining implementation gate

The owner accepted the complete bounded envelope, external keyring, encrypted
SQLite persistence and internal-decryption/export profile on2026-10-18 (D33).
The original local proposal is `.setup/p1-07/implementation-design-proposal.md`;
the implementation contract is [KEY_LIFECYCLE](../security/KEY_LIFECYCLE.md).
It includes a64KiB record limit, up to32 retained keys and Linux-first protected
file loading; Windows file loading and user-facing plaintext export remain gated.
P1-07 remains in progress and PR #8 remains draft until implementation, its
negative/rotation/restore evidence, independent review and exact-head checks pass.
P1-05/P1-06 and live verification/admission remain gated.

## Implementation under validation

The five-crate production workspace now adopts the exact qualified AWS-LC/zeroize
profile. All newly added registry versions/checksums match the standalone lock;
existing SQLx/SQLite versions and driver patch are unchanged. `droutex-security`
owns bounded redacted/zeroizing plaintext, context-authenticated binary envelopes,
external keyring parsing and Linux protected-file loading. It exposes no plaintext
serialization or user export. OS memory/library internals are not covered by the
owned-buffer zeroization claim. See the accepted key-lifecycle contract for limits.

Storage migration4 adds a separate encrypted-record table. Inserts are immutable;
re-encryption uses exact expected-envelope comparison with unchanged reference and
purpose. Loads bound blob allocation and reject corrupt headers or substituted
references. These operations reuse the existing bounded writer/deadline/rollback
checks without changing `database::save` or its revision transaction ordering.
P1-03/P1-04 transaction-maintenance triggers were revisited against these changes:
the new one-statement operations live in `secrets.rs`; no metric-only split of
configuration transactions is justified. Actual failure tests and independent
implementation review still determine acceptance.

Windows fmt, all-target/all-feature warnings-denied Clippy, default workspace tests,
no-default all-target check, rustdoc and all four locked cargo-deny categories
passed. Workspace:45 tests, including7 security/10 storage, plus6 compile-fail
examples. A temporary removed expected-context guard made its negative regression
fail; restoring it passed (`.setup/p1-07/context-regression-red.log` / `green.log`).
Implementation head `b4732c6085c368927e12d62745aa17ccb9810e06` passed all 11 jobs of
[run 39061026897](https://github.com/romanpodg/DRouteX/actions/runs/39061026897),
including actual Linux keyfile permission/link, schema3 upgrade, durable backup
restore, late-write rejection and corrupt-blob checks. Linux:58 workspace/23 storage
tests; Windows:45/10; both defaults and no-default tests, plus6 compile-fail examples.
Independent read-only/never review against base644348c completed with exit0 and no
confirmed defects (session `01a14b42-a732-71b2-8313-cd93c0450679`, local
`.setup/p1-07/codex-review-implementation.log` / `.exit`). Fresh Cargo checks were
blocked in its read-only sandbox; the separate exact-head CI passed.

Codecov processed both reports and the exact comparison: backend1986/2103;
new Rust351/357. Aggregate patch358/364 includes7 auxiliary planning lines and
is not the product patch. New envelope/keyring/material/storage source files are
fully covered; the Linux keyfile source is61/63. This is execution evidence, not
a proof of cryptographic safety or runtime authorization. CodeScene7999861 failed
maintenance gates; the source-based dispositions below remain failed, not suppressed.

## Proc ownership investigation

A proposed UID change in83600f2 used `/proc/self/status` because proc files may
become root-owned for non-dumpable processes. That head passed all11 jobs of
run39061713818 and an independent read-only/never review with no confirmed defects
(session `01a14b4b-3629-7ae2-8712-c6e791e4cb62`, local
`.setup/p1-07/codex-review-uid-fix.log` / `.exit`). These are historical results.

A subsequent actual unprivileged WSL probe disproved the supposed defect in the
original directory-stat approach: with dumpable0 and effective UID65534, the
process directory stayed owned by65534 while status/stat files became owned by0.
Linux v6.12 `task_dump_owner` explicitly preserves effective ownership for
world-readable/executable process directories. See the
[primary kernel source](https://github.com/torvalds/linux/blob/v6.12/fs/proc/base.c#L1782-L1790).
The speculative status parser and its field-selection test are removed; the
original simpler UID lookup remains, with a corrected explanatory comment.
Published history is retained through ordinary commits.

The first probe's assertion that directory ownership would change failed; it is
not passed evidence. The follow-up probe succeeded and records the distinction in
`.setup/p1-07/uid-nondumpable-wsl-detail.log`. WSL also reported an unavailable
systemd user session for nobody; the isolated Python process still ran as UID65534.
No service/runtime/systemd acceptance is inferred from this OS diagnostic.

Final exact-head CI, independent final evidence review and merge verification
remain required. No known serious defect is deferred by the maintenance records.
