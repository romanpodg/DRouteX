# P1-06 local activation and recovery contract

Status: D36 owner-accepted on 2026-10-18; implementation reviewed/tested on3c12bba.
See [evidence](P1_06_ACTIVATION_REVIEW.md). PR11 subsequently passed final checks/
review, merged at6b58b89 and passed all11 main jobs39154498259 plus both processed
Codecov reports. Approval alone did not pass tests or reviews.
P1-05/PR9 merged normally at521bfa76fbc27e14cd4981638af775ba92c6299f;
all11 exact-main jobs39148795165 passed and both exact-merge Codecov reports
processed (Rust backend2582/2719). The failed CodeScene maintenance findings retain
their recorded dispositions in [OPEN_DECISIONS](OPEN_DECISIONS.md).

## Accepted D36 profile

The owner selected the private filesystem generation store, independently of CP
SQLite and its backups. The alternative DP SQLite journal was not selected.
This accepts the bounded subset below, not ADR-0010/0012 in their entirety.

- Linux x86_64 durable storage, operated by a non-root service account; Windows
  verifies the explicit in-memory model and refuses durable operations as unsupported.
- Three fixed generation slots: active, previous and staged/recyclable. One
  serialized writer; an OS file lock also excludes a second process opening the
  same store. At most64MiB actual resources and8MiB envelope per slot;10,000 pins.
  The strict versioned JSON activation manifest is limited to16KiB. Artifact names
  derive from typed resource identities, never wire-supplied paths.
- Private0700 directories and0600 regular single-link files, correct owner, no
  symlinks. Explicit provisioning checks placement outside CP/database/backup roots.
  Reopening requires only the DP root, expected node and independently provisioned
  public trust; absent CP/database paths do not prevent recovery.
- Prepare immutable resources and the exact signed envelope; verify scope, signature,
  resource pins and checked configuration; flush files/directories; reload and
  authenticate persisted bytes; atomically replace the same-directory manifest
  committing active identity and sequence floor together, then sync its directory.
- A staged generation cannot serve decisions. Once the commit completes, publish
  one shared immutable generation. A decision captures both controlled DNS/gateway
  views once. No per-consumer callback can partially activate a version. Readers
  retain captured data during a switch. A previous generation with outstanding
  reader pins cannot be recycled: a further activation returns Busy. Pins belong
  to decisions, not entire TCP sessions; future services must bound their lifetime.
- The reader pointer lock never covers filesystem I/O. APIs are synchronous and
  blocking; async integration must call them from a blocking context. Cancellation
  before commit preserves active state. A signal after commit cannot undo it or
  change an applied result into a pre-commit cancellation.
- A rename/directory-sync failure with uncertain outcome never reports applied or
  rollback. The old in-memory generation may continue; further activation requires
  reopening/recovery. Recovery chooses the complete committed generation before
  ingress. Known inactive fixed slots and incomplete manifest temporaries are
  safely removed; no arbitrary recursive cleanup or history accumulation.

## Retention, trust and replay

One monotonic node sequence spans signing-key changes. Equal/lower sequences are
refused; gaps are allowed. First provisioning is a separate explicit operation
requiring a new directory. Opening missing/corrupt initialized state never invents
floor0. The DP sequence is independent of CP database restore.

Retained verification authenticates exact persisted node/snapshot/source/sequence,
signing identity and envelope digest. Its result is a separate Rust type and cannot
act as a fresh-admission proof. No sequence subtraction or floor reset is used.
All actual resources must match the signed pins, with neither missing nor extra
artifacts. Secret artifacts remain bounded encrypted envelopes with matching purpose.

CP/database loss preserves the last committed usable generation. Missing, corrupt
or revoked active state refuses new decisions; recovery never selects the previous
generation as a fallback. Revocation blocks new pins under the replaced trust;
existing captured decisions may finish. A valid new active signer need not retain
the previous generation's revoked key. Returning to old content requires a new
authorized signed envelope with a higher sequence.

Local storage addresses crashes and corruption. Malicious rollback of the entire
volume and every trust/state file needs a separately selected external monotonic
anchor; D36 does not claim that protection. Total state loss requires operator
recovery without automatically recreating the old node scope/floor.

## Acceptance boundary

Controlled consumers verify generation coherence; actual DNS/gateway listeners,
compiled policies, client DNS TTLs, sessions and revocation admission integration
remain at P2/P3/P4. No privileged networking, engine, remote downloader, new registry
dependency or paid service is introduced.

Required evidence: bounded input and path refusals; scope/pin/signature/replay and
rotation/fresh rollback; cancellation; reader concurrency/backpressure; CP loss;
write/flush/ENOSPC and uncertain-commit failures; real unprivileged Linux process
exit/reopen at every durable transition. Fault injection is distinguished from
physical power-loss and actual volume exhaustion. Complete Windows/Linux CI,
dependency/license checks and independent implementation/final-evidence reviews
must precede normal merge. P2 remains planned until P1-06 is finished and merged.
