# Encrypted secret records and key lifecycle

Status: owner-accepted D30-D33 profile; implementation/acceptance in progress.
First server target: unprivileged Debian 13 x86_64. No production deployment here.

`droutex-security` owns key material and redacted, zeroizing plaintext buffers.
`droutex-storage` persists only structurally checked encrypted envelopes in the
existing SQLite single-writer boundary. Configuration DTOs carry SecretRefId;
neither storage nor configuration imports decrypt. Explicit trusted internal
decryption is required by each consumer, with expected reference and typed purpose.
This does not authenticate the caller; API/provisioning/export/RBAC remain P6.
No generic plaintext UI/API/CLI/file export is provided by P1-07.

## Record v1

All integers are unsigned, big-endian. Exact bytes, no trailing extensions:

| Field | Bytes |
|---|---:|
| Magic/version `DXSEC\0\x00\x01` | 8 |
| Suite1 (AWS-LC AES-256-GCM-SIV) | 1 |
| Public key identifier (nonzero opaque bytes) | 16 |
| SecretRefId (validated UUIDv4) | 16 |
| Purpose1 resolver verifier,2 proxy outbound credential | 1 |
| Fresh OS-random nonce | 12 |
| Plaintext/ciphertext length,1..65536 | 4 |
| Ciphertext | length |
| Authentication tag | 16 |

AAD is ASCII `DRouteX secret record` followed by NUL and the entire 58-byte header.
The expected context is supplied independently by the trusted consumer. Unknown
version/suite/purpose, invalid IDs/lengths and trailing bytes are rejected before
cryptography. A structurally valid envelope is not authenticated until decryption.
AEAD does not prove freshness, prevent DB rollback or verify resolver tokens;
P1-05/P3-07 retain those gates. Snapshot authentication uses a separate purpose/key
design; encryption keys are not signing keys.

Logical secret changes create a new SecretRefId. Rotation re-encrypts a record
under the active key with a new nonce, preserving its reference/purpose. SQLite
replacement requires the exact previously read envelope; stale updates Conflict.
Missing keys and failed authentication refuse decryption/re-encryption. There is
no fallback or hidden retry. A newly provisioned key cannot be identified as wrong
without an authenticated existing record; syntax validation does not prove that
the operator supplied the intended keys. Never reuse a key identifier for different
key bytes. Each historical backup needs its matching retained keys.

## External keyring v1

Binary file: magic/version `RTKEY\0\x00\x01` (8 bytes), key count (1 byte,1..32),
active public key ID (16 bytes), then count entries of ID16/key32. Entries are in
strictly increasing key-ID byte order; duplicate/zero IDs, missing active key,
invalid sizes, unknown versions and trailing bytes are refused. The entire file
is secret. Keys are operator-provisioned raw256-bit values, never generated at
startup or stored in the DB, logs, argv, ordinary configuration or backups.

Linux x86_64 loading requires an absolute path without symlink ancestors, a service-owned
private 0700 parent and a service-owned 0600 regular file with one hardlink. Validate
the opened handle, bound reads, and reject paths within the caller-declared existing
database/backup roots. Those trusted roots must accurately describe storage.
The loader is synchronous; async services must call it from blocking work.
The effective service UID comes from trusted Linux process-directory ownership;
the kernel preserves it for this directory even when the process is non-dumpable.
Malicious same-UID mutation and a compromised OS/procfs are outside this boundary.
Windows protected-file loading fails unsupported; portable in-memory parsing and
crypto tests are available. A Windows provider requires separate ACL acceptance.

Provision a replacement keyring atomically, then explicitly reload/restart the
owning service. New writes use its active key. Keep old keys for record
re-encryption and retained backup restore; no automatic key deletion/pruning.
The 32-key bound fails explicitly. Remove a retired key only after separately
checking every retained record and backup that requires it. Key loss is not
recoverable from ciphertext. Full encrypted backup/export orchestration remains
P6-07; migration copies contain encrypted records and never the external keyring.

Memory zeroization is best effort for owned buffers; no claim covers arbitrary
caller copies, registers, crypto-library internals, dumps or OS/swap pages.
