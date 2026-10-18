# P1-05 snapshot authentication review

Date: 2026-10-18. Status: **initial qualification completed; production signature acceptance blocked by open owner security gate**.
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


## Qualification result and subsequent security hold

Head `0c3084daf16d54288791461c83f15573c191ae23` passed all11 jobs of
[run39063908865](https://github.com/romanpodg/DRouteX/actions/runs/39063908865),
including actual Windows/Linux signature and repaired SQLite probes. Independent
read-only/approval-never built-in review against base36cb5e3 completed exit0 with
no confirmed defects (session01a14b66-8a1f-73b1-bc2f-aa10fdbceab6; local
`.setup/p1-05/codex-review-qualification.log` / `.exit`). Planning/fmt passed;
five tests ran on an existing binary; fresh Cargo/Clippy writes were blocked in
read-only. CodeScene8000064 remains failed on the excluded auxiliary validator
finding recorded in P1-05-CS-AUX. Codecov processed the exact qualification head;
no product signature coverage is inferred from that auxiliary-only patch.

The owner subsequently accepted the complete D11 profile, including binary header,
canonical JSON,8MiB bound, separate protected PKCS#8 signing key, trusted public
keys and one increasing sequence stream across rollback/key changes. That accepted
profile does not waive newly discovered security defects. The uncommitted local
prototype's negative test then exposed weak-key acceptance at the new trust boundary
(all-zero key/signature). The upstream implementation explicitly accepts the
identity-point case as well. See P1-05-WEAK-TRUST in OPEN_DECISIONS. Correct RFC keys
passing does not establish weak-key refusal.

The owner chose to leave the security gate open rather than authorize the proposed
new strict-verifier dependency qualification. At that point no such qualification
or adoption had started. The draft PR must not merge; prototype code remains local and
unreviewed. Preserve the intentionally failing security regression, distinct from
the published, completed qualification review. Snapshot codec/configuration checks
and an independent Python hashlib/struct plus unprivileged WSL OpenSSL golden passed
locally; this is not complete production acceptance. WSL reported an unavailable
systemd user session for nobody; the direct --exec OpenSSL command succeeded.
No runtime/systemd acceptance is claimed. Exact local progress/checks and the
open gate are in `.setup/roadmap-autopilot-state.json`.

## Reopened weak-key resolution after the rename

The owner instructed work on resolving weak keys on 2026-10-18. A fresh baseline
showed that the old all-zero signature probe passes after changing the signed
domain to DRouteX; this is a message-dependent forgery example, not fixed key
validation. Two additional regressions independently demonstrate weak-key import
and acceptance of an identity-key/identity-R/S=0 forgery without a signing key.
Both were observed failing on the unchanged verifier (Cargo exit101); see
`.setup/p1-05/weak-trust-independent-red.log`. The test-file rustfmt check passed.
The owner subsequently selected D35 qualification and conditional adoption after
successful checks. PR #9 stays draft and the prototype stays local while evidence
is pending. The new regression failures remain required evidence.

## D35 strict-verifier qualification

Owner selection2026-10-18 authorizes ed25519-dalek exactly3.0.0, defaults off,
weak-key refusal at trust import and strict individual verification, retaining
AWS-LC signing. No new license exception, handwritten curve arithmetic, blacklist
or ordinary-verification fallback is authorized. Library ZIP-215 parsing also
accepts noncanonical points: the qualified import profile compares the library's
point re-encoding with the original bytes and refuses mismatches, preserving
canonical key identity. This is not a claim of arbitrary subgroup validation.

The standalone package adds20 locked artifacts to the previous15, without changing
their versions/checksums. The [strict inventory](../operations/SNAPSHOT_VERIFIER_INVENTORY.csv)
records SHA256 of all35 registry archives and their packaged licenses/notices;
local verification matched every archive. Current advisory fetch and all four
cargo-deny categories passed under the existing license allowlist.

Windows first native build failed MSVC archive rewrite inside the sandbox; the
unrestricted offline retry using the existing target cache passed all16 then-present
tests (6 AEAD,5 original signatures,5 strict). A sixth strict case additionally
checks nonweak noncanonical key import. Exact final Windows/Linux CI and independent
qualification review remain pending; no production strict-verifier adoption is
claimed by this document. The original public SnapshotTrust regressions remain red
until qualification permits the implementation fix.
