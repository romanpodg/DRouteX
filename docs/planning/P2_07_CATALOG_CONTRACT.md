# P2-07 catalog import contract

Status: D41 owner-accepted2026-10-21 by direct reply accepting the complete local
profile after PR15. Prerequisite satisfied before implementation: normal merge
b4c104c8e87acdb388b6ee5455dfc5bd01ab331d, all11 exact-main39481364319 jobs and both
processed Codecov reports (Rust4329/4535).
Next order remains P2-07 -> P2-05 -> P2-06, as the existing roadmap requires catalogs
before complete compilation. The owner acceptance applies to this complete bounded profile, not later compiler,
Explain, runtime or other proposed technical ADRs.

## Accepted bounded local profile

1. Offline byte inputs only. Select an explicit format: UTF-8 one-domain-per-line,
   blocking hosts, or strict versioned JSON service bundle. No URL fetching,
   proprietary feeds, Adblock/regex/partial wildcard grammar, executables or engines.
   Real third-party feeds require their own actual source/license approval; none
   are approved by the third-party dependency license allowlist.

2. A domain/hosts list requires an explicit caller-selected D37 match mode for the
   entire revision (Exact, DomainAndSubdomains, SubdomainsOnly), with no default
   or inferred bare-domain semantics. JSON service entries specify the same modes
   individually. Each service entry is a domain condition, not an action/transport
   or Profile/grant; service actions remain existing Profile selections. Typed
   dependencies use immutable revision IDs AND expected content hashes; no latest
   alias, network target or invented node/tunnel reference.

3. Plain lists permit blank lines and # comments; non-comment lines contain exactly
   one domain, normalized by the already qualified D37 implementation. Hosts input
   permits only literal blocking sinks 0.0.0.0,127.0.0.1,::,::1 followed by one or
   more domain aliases; these IPs are discarded, never imported as private answers.
   Other addresses, URLs, ports, decorators/wildcards, malformed/unsupported lines
   refuse the complete update; no partial success or silent record skipping.
   UTF-8 BOM is allowed only at start, LF/CRLF supported; spaces and tabs separate
   hosts fields. Invalid UTF-8, NUL and controls other than that defined whitespace
   refuse. Equivalent canonical duplicate conditions can deduplicate after counting
   all raw entries, and are reported in the import summary.

4. Reuse already accepted bounded profiles: maximum1 MiB per raw input,32 nested
   JSON collections,10000 aggregate entries/dependency edges per import batch,
   before collections/normalization; the whole batch and produced resource payloads
   also stay within the accepted D36 64 MiB per-generation resource budget.
   D37 still applies1024-byte name input and
   253/63/127 canonical bounds. Comments are charged to byte limit, raw duplicates
   and aliases to entry limit. Original bytes, notices and metadata are bounded
   within that input budget. Empty/comment-only results refuse; they cannot erase
   existing protection. No new numeric/configurable limit or registry dependency.
   JSON rejects duplicate/unknown fields, wrong schema/kind/types and trailing data.

5. Retain actual original bytes, format/mode, bounded source/owner/license/notice
   metadata and source SHA-256. Construct deterministic canonical resource bytes
   with explicit schema version, typed revision identity, metadata, original-byte
   hash, sorted canonical conditions and dependency pins. Existing qualified SHA-256
   hashes these ACTUAL resource bytes for D11 resource pins; do not trust input's
   claimed digest or hash ambiguous string concatenation. Raw source hash changes
   when comments/order/format change; canonical conditions remain deterministic.
   A revision identity cannot silently acquire different resource bytes.

6. License approval is an explicit trusted operator boundary bound to the exact
   revision/source hash/license evidence, not an Approved=true claim read from
   untrusted payload. No dataset approval is invented. Only complete successful
   normalization/hash/provenance/license/dependency verification constructs a
   verified immutable revision and its D39 facts. Facts alone still cannot prove
   authenticity; P2-05 binds complete data/pins/config to one generation. Preserve
   actual license/notice texts and provenance for local export/snapshot resources.

7. Prepare candidate revisions and validate the whole selected dependency closure
   before atomically replacing the local accepted revision set. A malformed,
   oversize, conflicting identity/hash, missing/unlicensed/cyclic dependency or empty
   update leaves the previous accepted set intact. No automatic publish or DP
   activation, unknown/latest resolution or Profile ownership inference. The
   exported immutable resource payload is suitable for existing storage/snapshot
   boundaries; P2-05 owns complete compilation/publication. No new filesystem
   transaction/network downloader is smuggled into this pure import slice.

8. Pure policy models/domain+hosts parsing stay in policy; strict service/metadata
   decoding, existing qualified serialization and SHA-256 boundary stay in config,
   avoiding a policy->config cycle or new registry dependency. Keep public APIs
   small, redacted diagnostics and line/index errors without echoing domain data.

## Evidence before adoption

Own Apache-2.0 local fixtures, not unapproved proprietary datasets. Real local
byte format fixtures; Unicode/ACE/CDN normalization; mode/apex/subdomain behavior;
multiple hosts aliases and explicit sinks; comments/BOM/CRLF; duplicate canonical
names and deterministic ordering; exact byte/entry/depth boundaries and malformed
formats; original/canonical hashes and dependency identity; license/provenance
refusal; immutable identity, atomic failed-update last-known-good preservation,
Profile-independent catalog data and unchanged signed/config vectors. Windows/Linux
default/no-default Q-RUST and deny, independent linked-account implementation/final
Code Reviews, actual Codecov/CodeScene assessment, normal merge and exact-main checks.
No actual network/feed/systemd/runtime integration success is claimed by pure tests.

## Reference facts versus proposal

Checked2026-10-21: Control D custom rules document bare domain including descendants,
explicit *.domain excluding apex, plus a larger wildcard grammar. Folders support
JSON export/reimport. These are reference behavior, not DRouteX authority:
https://docs.controld.com/docs/custom-rules
https://docs.controld.com/docs/folder-rules
DRouteX deliberately requires explicit existing D37 mode and a smaller offline
format, preserving accepted CR-0002 rather than inferring vendor wildcard semantics.

Alternative: broader hosts/redirection/Adblock/download formats or implicit match
semantics now would add user-visible security, dependency/license and parser scope;
recommend the explicit bounded profile above. That broader alternative was not selected by the owner.
