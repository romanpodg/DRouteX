# P2-05 complete compilation contract (D43)

Owner accepted this complete profile2026-10-21. Prerequisite satisfied before code:
PR16 merged normally at e8c5e6105ffa7426115eda4e7243d8f26eea5d17 after independent
implementation/security/final reviews and all11 final-head39546423405 jobs. All11
exact-main39546838691 jobs passed; Codecov processed both reports (Rust4951/5177).
D42 WebPKI security correction qualified and merged without suppression.
Existing D38/D39/D40/D41 semantics remain binding. This accepted profile fixes consequential
compiler/resource/application details. Implementation/reviews/checks remain required.

1. Build one owned immutable compiled generation from exact canonical SnapshotContent,
   actual resource bytes/pins and complete checked configuration. Construct all
   Profile, Endpoint and independent gateway-group tables, resource closures and
   grant/private tables together. Recheck current references, D39 resources AND
   COMPLETE prepared-set conflicts from these same bytes. Disabled/unassigned
   declarations still consume bounds and retain required references; no separately
   successful Result or user-claimed facts can be substituted as authority.

2. Accepted explicit expansion semantics: custom DNS/traffic rules stay in their
   selected plane; selected service revision expands only ITS OWN listed conditions
   into the Profile-selected action. Dependency pins are validation prerequisites,
   NOT implicit condition/action inheritance. Gateway service target scopes likewise
   use only explicitly listed conditions. Unknown/missing/unlicensed dependencies
   still reject the entire compilation. An operator who needs additional service
   domains lists them explicitly in the curated bundle. Later dependency composition
   would require another explicit owner decision/format semantics, not inference.

3. Accepted filter extension: selected filter conditions compile as DNS Block AND
   a separate TCP Deny in that independently selected gateway Profile, both at the
   accepted filter layer. Custom/service rules still win by D38. DNS never changes
   the gateway Profile or establishes a gateway identity. This extends the vendor's
   documented DNS filtering concept to DRouteX's independent gateway policy; it is
   the accepted DRouteX extension, not a claim about Control D transport enforcement.
   The owner selected separate two-plane denial for consistent selected-Profile policy.

4. Keep private A/AAAA/CNAME records in a separately validated canonical Profile
   table; choose no private-record/rule precedence or RCODE here. Compile static
   grant source/listener/group/target/service/port/outbound scope without minting a
   live grant/binding permit from public IDs. P3-07/P4 own actual authentication,
   revocation/expiry/source checks, origin resolution, checked/pinned addresses and
   connect/TLS/SSRF. A typed static scope is not network admission.

5. Structural unsupported/missing/inconsistent config/resource kinds reject
   compilation. Do not persist observed Available as a permission in the snapshot.
   Disabled/unknown/unavailable live outbound facts yield existing D40 request-time
   refusal; strict failure never turns into DIRECT. Compilation may retain a
   declared unavailable/disabled native outbound as a reference, not an available
   route. Current trusted observations and actual freshness/connect checks remain
   runtime responsibilities. No probes/network execution during compilation.

6. Reuse existing numeric limits: one10000 aggregate prepared-entry budget for
   Profiles, Endpoint/group associations, expanded DNS/TCP/port rule entries,
   canonical private records and static grant lookup/target/port entries, including
   disabled declarations. Compute expansion counts with checked arithmetic BEFORE
   allocations/copies; no multiplicative Profile x catalog x ports expansion outside
   the budget. Keep existing32-depth/1MiB import,8MiB signed snapshot and64MiB resource
   budgets. No new dependency, numeric setting or wire configuration field.

7. CP complete-publication builder gates its signing path on all those checks and
   pins exact verified canonical catalog resources. DP first verifies existing D11
   signature/scope/replay and exact resource hashes, then strictly decodes canonical
   D41 resource-v1 and repeats complete compilation before fresh activation/recovery.
   A catalog payload's license Boolean/evidence text alone cannot authorize import;
   CP uses real D41 source/approval objects, DP consumes CP-authenticated pinned
   canonical resources. Raw source hash in a DP resource is CP-attested provenance;
   absent original raw bytes are NOT claimed independently rehashed on DP. All
   canonical conditions/dependencies/metadata and resource hashes are checked there.

8. Preserve D11 codec/cryptographic golden bytes and original config spelling.
   Compiled tables are deterministic derived memory, not an incompatible new signed
   compiled payload. Same exact source/resources/fixed metadata give same D11 bytes
   and same decisions; semantic-equivalent differently spelled source may retain
   different source hashes, as already required. Low-level codec remains a library
   primitive, not a bypass of the complete publisher/controller integration path.
   Correctly signed snapshots containing malformed catalog/policy data now fail
   COMPLETE compilation, even if the former existence-only boundary accepted their
   opaque bytes. This is explicit stronger validation, not a codec rewrite.

9. Attach the compiled view to the same core Generation used by both consumer
   views; compile before constructing/committing a new or recovered generation.
   CP/DB loss leaves the pinned compiled last-known-good usable. Failed candidates
   preserve it; corruption/revocation and rollback keep all accepted D36 semantics,
   no automatic old-slot fallback or replay reset. No independent mutable global
   catalog/rule cache may cross generations. Compile/ownership adapters live where
   existing config action types and core generation integration belong, keeping
   policy pure and avoiding config/policy cycles, self-referential unsafe or new
   networking dependencies. P2-06 adds complete Explain/simulation afterward.

Required evidence: real CP builder->signed D11->DP canonical resource verification->
complete compiled generation fixtures; D38 interpreter/compiler differential tests
with custom/service/filter/default and exact port/scope/mode cases; deterministic
reordered input/hash vectors; complete hidden conflict and licensing/current-pin
refusal; explicit service dependency non-inheritance and separate filter TCP denial;
preflight expansion boundaries; correct-signature malformed catalog/policy refusal
before activation/recovery; same pinned DNS/gateway generation under failure/CP loss;
existing replay/rotation/rollback/format vectors, redaction and compile-fail typed
boundaries. Actual Windows/Linux Q-RUST/deny, independent implementation/final
linked-account reviews, Codecov/CodeScene, normal merge and exact-main gates.
Pure compilation tests cannot claim actual DNS/CONNECT/admission success.

Primary reference checked2026-10-21: Control D documents Services as domain groups,
Filters as DNS resolution blocks and category order Custom > Service > Filter >
Default. It does not establish DRouteX's separate gateway identity or dependency
composition semantics; the departures above received explicit owner acceptance as D43.
https://docs.controld.com/docs/services
https://docs.controld.com/docs/filters
https://docs.controld.com/docs/advanced-rules-creation-guide

Direct owner reply: accept the complete profile after checked PR16 merge. No implementation before that prerequisite.
