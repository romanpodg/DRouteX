# DRouteX pure simulation output v1

The accepted [D44 contract](../../../docs/planning/P2_06_EXPLAIN_CONTRACT.md)
defines in-process diagnostics. This directory creates no HTTP endpoint or
authentication/visibility permission. P6 must authorize the selected Profile
before returning any trace or private-owner existence to an external caller.

[SimulationReport](../../../crates/droutex-core/src/deployment/explain.rs) can only
be constructed by a pinned Decision. Its node, snapshot, source revision,
monotonic generation and canonical content hash come from that same generation,
as do its checked configuration, actual catalog pins and compiled policy tables.
Reports are owned predictions about that pin; retaining one does not prove it is
still active or confer access. Serialized JSON is never an admission token.

- [simulation-v1.schema.json](simulation-v1.schema.json) specifies the strict
  JSON Schema2020-12 output shape, including false-only network_test_performed
  and null-only effective_route. Unknown output fields are excluded.
- [simulation-v1.example.json](simulation-v1.example.json) is the reviewed complete
  generation example, also exercised as a core golden test.
- [selection-examples.json](selection-examples.json) contains reviewed unbound
  selection diagnostics used by the config integration tests: service/filter/default,
  private-owner precedence pending P3, ECH context, QUIC refusal, shared NAT scope
  and separate IPv4/IPv6 limitations. These are values of the report's selection
  property; alone they contain no authenticated generation claim.

The trace covers every prepared entry in the selected Profile and plane,
including disabled and nonmatching entries, within the existing10000 aggregate
prepared-entry budget. inspected_entries and emitted_entries equal trace length;
there is no pagination, silent truncation or hidden second winner algorithm.
Order is descending D38 rank, stable source UUID bytes, canonical domain, port
interval and enabled-before-disabled for otherwise equal metadata. Duplicate
identical entries remain identical. selected means the winning source at the
highest rank; several OR intervals from the same winning source may be selected.
equivalent_tie identifies other highest-rank sources with equivalent effects.
Different highest-rank effects return an error instead of a successful report.
Default is explicit only when no enabled eligible entry matches.

Configured predicted_action is separate from resolved_intent. Availability and
compatibility inputs are caller-supplied simulation assumptions; absent facts
remain Unknown, and observations cannot enable a disabled declaration. Scope
errors return no partial trace. TCP requires an independently established gateway
identity, explicit nonzero port and matching static grant/source/listener/target/
outbound scope; static comparisons do not authenticate ingress or safe addresses.
DNS never selects that gateway group. A supplied QUIC flow contradicts TCP
transport and is refused; UDP/unknown transport is not a native TCP route.

The reason chain preserves category/priority/specificity/mode/port/tie semantics,
source rule/revision IDs and actual canonical resource hashes, without raw source,
approval text or a fabricated author. DNS-only ECH/QUIC/bypass coverage and TCP
name recovery are not established; group scope cannot identify an individual NAT
peer, and one IP family supplies no evidence for the other. Private-owner output
includes only selected-Profile record IDs/types; no RDATA, external CNAME lookup,
answer precedence, TTL/RCODE or effective DNS answer is invented before P3.

No connector, resolver, network probe or OS-network action is invoked. The sole
passive loopback trap in tests detects an accidentally introduced connect; it is
not a product Network Test or a connectivity result. Debug/errors contain safe
counts/codes, never raw invalid domain strings, source addresses, private values,
capabilities, proxy host/credentials, encrypted resources or signing material.
