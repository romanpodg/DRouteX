# P3-02 isolated DNS client qualification contract (D51)

Status: accepted by direct owner reply on2026-10-22, recorded2026-10-24.
The owner selected the complete isolated qualification profile before dependent
code. Checked P3-01 completion, including PR112 merge8f234c44 and all11
exact-main40097388131 jobs/both processed reports, now fulfills the prerequisite.
This authorizes tests-only qualification, not production adoption.
The later owner-accepted [D52 amendment](P3_02_CLIENT_VALIDATION_CONTRACT.md)
qualifies only the composed role with mandatory ingress/caller adapters.
The raw TCP client remains unqualified for the standalone strict role.

## Accepted decision

Authorize isolated qualification of exactly hickory-net0.26.3, defaults disabled,
with only its tokio feature, against the existing exact Hickory-Proto0.26.3 and
Tokio1.53.3 profiles. This qualifies a UDP/TCP client building block; application
upstream definitions, fallback/address/loop rules and runtime limits need a
separate complete owner-accepted P3-02 profile before production integration.

The existing proto-only approval does not authorize this new networking crate.
The official metadata/archive were read, not executed or adopted. Its manifest
places UDP/TCP clients in hickory-net and permits MIT OR Apache-2.0, already
accepted alternatives. Official archive SHA256:
c480823ed7c2c5d0f09c41020cb6b7c28029ce60ec42dc942158dcf22f8e0a4d
The independently calculated archive hash matches crates.io metadata. This is
identity evidence only, not completed dependency/license/security qualification.

## Scope and constraints

- A separate tests-only Cargo workspace/lockfile, not production dependencies.
  Preserve existing exact selected versions. Resolve and freeze its entire graph,
  independently verify official archive checksums/license texts/notices, and run
  cargo-deny advisories/bans/licenses/sources. No new license exception or advisory
  suppression. Any required change to an existing pinned version comes back to
  the owner rather than being silently bundled.
- UDP/TCP only. No TLS/DoH/DoT/QUIC/mDNS/DNSSEC/resolver/recursor features,
  certificate stores, OS resolver, external network DNS or privileged setup.
- Reuse existing Windows/Linux Workspace jobs for actual default/no-default
  isolated tests and the existing dependency-check job for a seventh deny bundle.
  No new runner, token, secret, permission or untrusted privileged workflow.
- Disposable controlled unprivileged IPv4/IPv6 loopback fixtures. All mandatory
  platform cases run without ignores/filters. Known development-host IPv6 failure
  remains explicit; clean CI IPv6 is mandatory. No host network changes.
- Qualification must inspect and exercise request identity/peer/question matching,
  receive/parse bounds, cancellation and background-task ownership. Existing
  exchange convenience methods can spawn background tasks; production adoption
  cannot assume that dropping a handle aborts them. If the required bounded owned
  lifecycle cannot be demonstrated, the library is not accepted for that role.
- No public service, live CP/DP behavior, cache or actual gateway route changes.
  Qualification does not claim P3-02 completion or approve an upstream protocol
  security policy. Plaintext UDP/TCP does not authenticate an on-path peer.

## Required evidence

1. Independent-wire real UDP/TCP exchanges for IPv4 and IPv6, ordinary and
   auxiliary record types, split/partial/truncated TCP and response identity.
2. Wrong peer/port/ID/question/class/type/QR, malformed/trailing/declared-count
   input, oversized or inconsistent frames and valid response after refused
   foreign packets. Finite receive attempts/deadlines; no attacker-driven
   unbounded processing or detached retry loop.
3. Actual dropped/delayed/refused upstream, bounded timeout, cancellation during
   connect/read/write and release/rebind/abort/join of all owned resources.
4. Fresh Windows/Linux fmt/Clippy/tests, all current dependency bundles, full
   archive/license/advisory evidence, meaningful negative cases and two completed
   linked-account read-only/never implementation/final Code Reviews. Normal
   exact-head merge and all exact-main checks/reports remain mandatory.
5. Only successful qualification can support a later conditional production
   adoption proposal. Failure preserves production unchanged; a different
   dependency, custom wire client or relaxed requirement needs a new decision.

## Reference and alternatives

Hickory's official UDP-client documentation describes per-request source-port
randomization. RFC5452 requires transaction/question/provenance matching. The
inspected exact UDP source has these checks, but only controlled qualification
and review can establish fit for DRouteX's narrower owned/deadline profile.
Actual TCP qualification subsequently exposed a question-matching role gap;
the [evidence](P3_02_CLIENT_QUALIFICATION_REVIEW.md) keeps that failure open.
https://docs.rs/hickory-net/0.26.3/hickory_net/udp/struct.UdpClientStream.html
https://datatracker.ietf.org/doc/html/rfc5452
https://crates.io/crates/hickory-net/0.26.3

The owner selected qualification before application behavior. The unselected
alternative was postponing the client and retaining current admitted-query
SERVFAIL while selecting another separately approved transport/library.

The direct owner acceptance satisfies the dependency/qualification decision gate
in [AGENTS](../../AGENTS.md). Research/download alone did not grant that approval.
P3-02 application behavior, runtime resources and production adoption remain
separate consequential decisions. P3-02 stays planned during this isolated stage.
