# P1-05 snapshot authentication review

Date: 2026-10-18. Status: **isolated Ed25519 qualification in progress; codec gated**.
Base: verified P1-07/PR #8 merge `36cb5e38e215b1f53a10a3cde1dccbda9c25b5c2`.
Prerequisites P1-04/P1-07 are complete. No snapshot producer or consumer is adopted.

## Owner decision D34

The owner directly approved qualification of Pure Ed25519 in the already pinned
aws-lc-rs1.18.2 profile. The Control Plane alone will hold a separate protected
external signing private key; Data Plane consumers receive operator-trusted public
verification keys. Do not reuse an AES master key or trust a key supplied by the
snapshot itself. No new dependency, tool installation or license exception.
Production signature adoption requires successful isolated Windows/Linux/RFC8032
tests and independent Code Review. Canonical snapshot bytes, metadata binding,
sequence/replay persistence and key rotation remain the separate D11 gate before
codec implementation. ADR-0010/0012 remain Proposed, not implicitly accepted.

## Qualification scope

The existing standalone `tests/secret-dependencies` package and its Windows/Linux
CI jobs run a separate `snapshot_auth` test target. Its locked15-artifact profile,
feature selection, native AWS-LC source and Windows packaged-object provenance
are unchanged from [P1-07](P1_07_SECRET_REVIEW.md). The package remains outside the
production workspace; no application source or production lockfile changes.

All five Pure Ed25519 vectors from [RFC8032 section7.1](https://www.rfc-editor.org/rfc/rfc8032.txt)
exercise exact public keys/signatures and verification, including empty,1023-byte
and64-byte messages. Downloaded source SHA256:
`ed63657ff389301282b169b0abde9b5dd2c7e4d524fdfa5da6ff3094fc93c4c3`.
The64-byte message vector still uses Pure Ed25519, not Ed25519ph.
Only intentionally public RFC key material is committed. Negative probes mutate
every signature/public-key/message byte, append/truncate messages, reject wrong
trusted keys and inconsistent seed/public pairs, malformed key/signature lengths,
noncanonical scalar/point encodings and truncated PKCS#8. Library PKCS#8 export/
import preserves identity/signatures; this does not select a production key format.
No handwritten cipher, signer, verifier or point arithmetic is introduced.
These probes are compatibility evidence, not exhaustive cryptographic proof or
snapshot replay/recovery acceptance.

## Concrete SQLite fixture repair

P1-07 main run39062875433 attempt1 failed initial `connect_with` with `PoolTimedOut`
at `tests/storage-dependencies/qualification.rs:317`. Its150ms pool acquire budget
also bounded cold worker/WAL startup under CI load. All application jobs passed;
one failed-only diagnostic retry passed all11 jobs. Both attempts remain in local
P1-07 evidence. The fixture now uses5s, retaining its actual saturated-pool
`PoolTimedOut`,10ms caller cancellation, aborted transaction rollback and read-only
write-refusal assertions under the existing15s outer deadline. SQLite's separate
150ms busy timeout and every production timeout are unchanged.

## Evidence

Windows passed all5 new signature probes, all6 existing AEAD probes, fmt and
all-target warnings-denied Clippy. The repaired SQLite scenario passed in5.14s
with all timeout/cancellation/rollback/read-only assertions. Fresh locked cargo-deny
passed advisories/bans/licenses/sources; unused allowlist-license warnings do not
change policy. Planning passed17 unit tests,9 negative cases and209 local links.
AST graph update passed (3076 nodes/6800 edges); SQL parser absence and semantic
document limits remain as recorded in P1-07. Logs stay under `.setup/p1-05/`.
Exact-head Linux CI and independent read-only/never review remain required before
qualification acceptance; no pass is inferred from earlier P1-07 checks.
