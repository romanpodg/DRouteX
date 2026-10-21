# Immutable snapshot candidates v1/v2

Owner-accepted D11 profile, with D35 strict verification. P1-05 produces a checked
candidate, not an active runtime generation. P1-06 owns durable replay state,
publication authorization, staging, activation, recovery and last-known-good
retention. P2 owns compiled policy; this payload is checked configuration.

## Framing and authentication

The binary envelope is at most8MiB, including its121-byte header and64-byte
Pure Ed25519 signature. Integers are unsigned, big-endian. Header fields in order:

| Field | Bytes |
|---|---:|
| Magic/version `DXSNAP\0\x01` | 8 |
| Payload schema1 or2, matching configuration.schemaVersion | 4 |
| Signature suite1 (Pure Ed25519) | 1 |
| SigningKeyId | 16 |
| NodeScopeId | 16 |
| SnapshotId | 16 |
| Source ConfigurationRevisionId | 16 |
| Nonzero sequence | 8 |
| Payload length | 4 |
| SHA256 of canonical payload | 32 |

The payload follows the header, then the signature. Authenticate the bytes
`DRouteX snapshot v1\0` followed by the complete header and payload. No unsigned
metadata, trailing bytes, compression or extension fields are accepted. Unknown
schema/suite, malformed IDs/lengths, bad hash/signature and wrong scope are refused.

The CP signs with qualified AWS-LC. The DP imports independently provisioned public
keys with ed25519-dalek3.0.0, refuses weak keys and noncanonical library point
re-encodings, and calls verify_strict for every signature. ZIP-215 parsing alone
is insufficient. No key supplied by a snapshot becomes a trust root. Empty or
over32-key trust sets and duplicate signing-key IDs are refused. Signing material
is separate from AES master keys and resolver secrets. The accepted PKCS#8 DER
file boundary is bounded to4KiB, with Linux protected-file checks and Windows
in-memory tests; provisioning is explicit and out of band.

## Canonical content and resource pins

The typed UTF-8 JSON payload contains complete checked configuration, resolver
credential-to-SecretRefId links and external resource pins. Revalidate configuration
against the exact trusted resources. Raw credentials and private signing/AES keys
are absent. Secret pins fingerprint encrypted envelopes and bind their purpose.
Device/upstream/catalog pins must match independently trusted actual artifact bytes;
self-reported snapshot hashes or database IDs do not establish trust.

Canonicalization materializes typed defaults, normalizes parsed IDs/IP/socket text,
sorts object keys, resource/rule tables by typed identities, and explicitly set-like
lists by their declared keys. It preserves rule priorities, matcher text and policy
layer order. Do not sort arbitrary arrays or infer domain matching here. Equivalent
accepted JSON/YAML/TOML imports produce equal canonical content. This is the bounded
DRouteX-v1 profile, not general RFC8785. Authenticated noncanonical payloads are
refused by comparison with canonical re-encoding.

Configuration retains its1MiB/depth32/10,000-resource limits, including normalized
size. Snapshot JSON depth is at most40; external pins at most10,000. Invalid,
duplicate, missing, cross-purpose, oversized or unsupported content fails closed.
The public golden fixture is checked against an independent OpenSSL signature.

## Sequence, rotation and recovery boundary

One increasing nonzero u64 sequence stream belongs to each permanently provisioned
node scope, across signing-key rotation and rollback. Reject equal/lower sequences;
gaps are allowed and overflow refuses. First provisioning at floor0 is explicit.
P1-05 checks a caller-trusted floor without advancing it or persisting it.

P1-06 must atomically recheck/persist the floor with activation, reject missing or
corrupt initialized replay state, and retain exact required artifacts independently
of the CP/database. Restarting an authenticated retained last-known-good generation
is distinct from publishing it again. Rollback publishes old content with a fresh
authorized sequence/signature. Install new public trust before switching signers;
removing a trusted key rejects subsequent candidates signed by that key. Coherent
handling of an already active generation and operator recovery remain P1-06 work.
No in-memory check proves durable anti-replay, live DNS/proxy admission, RBAC or
multi-node enrollment.

## D46 payload schema2 and retained schema1

[Accepted D46](../../../docs/planning/P4_01_BINDING_CONTRACT.md) introduces signed
manual leases in strict configuration2 and authenticated payload-schema2. The
existing magic, header layout, signature suite/context and hash remain unchanged;
the authenticated discriminator prevents mixed-version substitution. Old readers
reject2. New readers retain exact v1 canonical/signature/restore compatibility for
associations and diagnostics, but v1 never grants runtime admission. Expiry does
not renew on rollback/republication. Journal2's independent clock floor is durable
local state, not a resettable field in a signed snapshot. No replay/key floor resets.
