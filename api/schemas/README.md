# Configuration import contract, version 1

This is the P1-02 import boundary, implemented by
[`parse_configuration`](../../crates/droutex-config/src/schema/parser.rs).
It does not create listeners, verify credentials, publish policy or admit traffic.
Callers supply the format and a trusted `ReferenceCatalog`; documents cannot
declare trusted devices, upstreams, secrets or verified dataset revisions.
The authoritative field definitions are the
[typed schema](../../crates/droutex-config/src/schema/model.rs). An independently
written [complete JSON fixture](../../crates/droutex-config/tests/fixtures/configuration-v1.json)
shows the wire shape; its synthetic UUIDs need the catalog in the integration test.
OpenAPI and management endpoints remain P6-02.

## Encoding and limits

Accept exactly one UTF-8 JSON, YAML or TOML document with `schemaVersion: 1`.
The three formats use the same typed model. Input is limited to 1,048,576 bytes,
collection nesting to 32 (including the root collection), and resources/rules
to 10,000 in aggregate. That count includes every top-level resource plus each
Profile's custom rule, service rule and filter selection. Reference lists remain
bounded by the input/depth limits. Dataset payloads are not embedded; their
separate versioned imports and complexity limits belong to P2-07.

Reject duplicate keys, unknown fields, unsupported versions/transports/actions,
incorrect types, invalid UUIDv4 identifiers, duplicate resource/rule IDs, missing
references and invalid access/port scopes. No implicit last-key-wins behavior.
YAML uses strict booleans and rejects anchors, aliases, merge keys, includes and
unsupported tags. Includes and property substitution are disabled; strings such
as `${NAME}` remain literal data and never read the process environment.

Minimal empty drafts are valid and activate no services:

```json
{"schemaVersion": 1}
```

```yaml
schemaVersion: 1
```

```toml
schemaVersion = 1
```

## Resources and policy scope

Field names are camelCase; tagged values are snake_case. All IDs are UUIDv4
strings. Collections default to empty, while required fields have no guessed
fallback. The root collections are:

| Collection | Boundary |
|---|---|
| `profiles` | Required explicit DNS and traffic defaults; fixed custom/service/filter/default layers |
| `endpoints` | Exactly one Profile; optional Device metadata; per-listener DoH/DoT credentials or legacy UDP/TCP grants |
| `resolverCredentials` | Public ID, owning Endpoint and state only; no bearer secret or verifier |
| `listeners` | Numeric socket address, purpose, enabled flag and explicit IPv6 wildcard `dualStack` intent |
| `networkGroups` | Explicit independent traffic Profile; no last-DNS-query selection |
| `legacyGrants` | Exact source IP, DNS listener, group and Endpoint; ambiguous source/listener assignments rejected |
| `gatewayGrants` | Separate source/listener/group plus nonempty target or service scope, ports and outbounds |
| `outbounds` | `vps_direct` or `http_connect_tls`; TLS host/port/server identity and optional existing secret reference |
| `privateRecords` | Owning Profile, name, TTL and typed A/AAAA/CNAME data |

Enabled secure Endpoints require an associated active credential record for each
secure access binding. Unknown, cross-Endpoint and duplicate references fail.
Lifecycle metadata is not proof of token possession, expiry or current revocation.
Gateway grants cannot derive authority from resolver credentials or DNS actions.

DNS actions are `resolve` with an upstream reference, `block`, or `gateway` with an
outbound reference. Traffic actions are `deny` or `outbound`. Custom/service rule
actions explicitly name their `plane` and `action`. Domain match modes are
`exact`, `wildcard` or `suffix`, with the bare base name in `value`; no `*` syntax.
Priority is an explicit signed integer. DNS rules cannot carry TCP port ranges.
Ranges include both endpoints and require `1 <= from <= to <= 65535`.
Filter/service selections reference caller-known immutable revision IDs.

Enabled listener collisions account for UDP versus TCP, IPv4/IPv6 wildcards,
explicit dual stack and IPv6 scope IDs. IPv4-mapped IPv6 bindings are rejected.
DoH, DoT and gateway TCP use the same TCP socket namespace. This static check
does not probe OS occupied ports or configure IPv6 socket options.

## Errors and downstream gates

`ConfigError` exposes a stable enum code, a whitelisted field path with numeric
indices, and an optional parser-supplied numeric location. Syntax errors identify
`$`; type/semantic failures may have no source location. Column units follow the
parser. Buffered tagged-enum errors can identify the enclosing object rather than
its leaf. Unknown input keys, original messages, values and source chains are
discarded. Callers must use this entry point for untrusted input, and must not log
the input or bypass it by deserializing the public raw DTOs directly.

`CheckedConfiguration` has a private inner value and read-only document access.
It certifies this import boundary only. Domain IDNA/canonicalization and rule
conflicts belong to P2-01/P2-02; verified dataset contents to P2-07; private-record
resolution/CNAME behavior to P3-04; live secret/clock/source/destination admission
to P1-07/P3-07/P4-01. Unavailable outbound intent never authorizes DIRECT fallback.
Persistence, authenticated snapshots, TLS provisioning and runtime network
acceptance retain their roadmap gates.

The [immutable snapshot candidate v1 contract](snapshot/README.md) records the
accepted D11/D35 framing, canonicalization, strict trust and sequence boundary.
Candidate verification does not imply durable publication or runtime activation.
