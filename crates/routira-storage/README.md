# Control-plane SQLite storage

The crate stores immutable `ConfigurationRevisionId` revisions and catalog identity/
SHA-256 records. It does not select an active revision, publish policy, authenticate
a token or admit DNS/TCP traffic. Policy/packet crates do not depend on this crate.

`ConfigurationWrite::parse` reuses the bounded JSON/YAML/TOML input boundary and
takes ownership of the original document plus checked types. A save transaction
preserves the complete document and indexes Profile/Endpoint assignments, public
credential metadata, separate legacy DNS/gateway grants and catalog references.
Full rules, private records and destination/port/outbound scopes stay in the
document; consumers must not reconstruct authority from the smaller indexes.
Reads call `StoredConfiguration::check` with current caller-trusted references.
Catalog hashes are immutable claims, not verified payloads or source/license approval;
dataset import remains P2-07.

Credential persistence accepts public IDs/state and `SecretRefId` links only.
Unknown/duplicate links are rejected before queueing. Protected verifier/token
bytes and their lifecycle remain P1-07; actual verification remains P3-07.

One Tokio owner task owns one SQLite connection and a private storage lock. Sender
clones address that same task. The owner-approved queue holds 64 pending commands;
the executing command is separate. SQLite busy timeout is 1 second; response
deadline is 5 seconds. Full/closed queues and timeouts return stable typed errors
without retry. Expired/abandoned queued commands are skipped; an operation already
running may commit after its caller times out or drops. Reread its revision ID.
`Storage::close` drains prior commands and closes the connection; dropping the
lifecycle owner aborts its task without claiming that COMMIT rolled back.

Migrations use compiled-in SQL and SQLx transactional/checksum tracking. Missing,
changed, dirty or future migration history is rejected before changing the journal
or schema. An upgrade first creates a consistent SQLite `VACUUM INTO` copy, including
WAL content, in the same private directory. Only a completed, synced copy receives
the `.backup.db` suffix; a failed copy blocks migration and can leave `.pending`.
These local migration rollback copies contain configuration metadata, with the same
file access boundary as the DB. Encrypted export/restore and key lifecycle remain
P1-07/P6-07; no complete backup product or power-loss proof is claimed.

The first server target is Linux (accepted native target Debian 13). Supply a trusted
local parent directory and run the service unprivileged. Opening creates only the
final directory (0700) and files (0600); unsafe existing modes, symlinks, hardlinks
and a second owner fail without chmod or replacing data. SQLite WAL requires a
suitable local filesystem. Malicious same-UID or privileged filesystem mutation
is outside this directory boundary. Other platforms return `UnsupportedPlatform`
until their access-control model is implemented and reviewed; Windows compilation,
queue/error tests and standalone SQLite qualification do not prove Windows ACLs.

No raw SQL/SQLx errors, paths, connection handles, secret bytes or database-restore
API are exposed. Debug output redacts revision documents; errors retain only stable
codes and the existing sanitized config diagnostic when applicable.
