# ADR-0011: DNS cache isolation and configuration consistency

Status: **Proposed**. Date: 2026-10-15. Specification: §§3,9,10,22.

## Context

Rewritten/validated answers and different profiles/upstreams cannot safely share an indistinguishable cache.

## Decision

Cache keys include normalized name/type/class/upstream/policy scope/response mode/validation provenance and suitable generation. TTL/negative expiry bounded. Bind DNS rewrite route generation to gateway authorized contexts.

Under accepted CR-0002, effective Profile, pinned dataset/private-record revision and
endpoint access mode must not leak answers across scopes. Raw resolver secrets are
never cache/metric/log labels. Private DNS answers grant no gateway dial permission.

## Alternatives

One global name→IP map; clear DNSSEC validity ignored; retain revoked route until TTL forever.

## Reasons

Prevents cross-profile leakage and misleading authenticity claims.

## Consequences

Independent origin resolver never returns the policy gateway address; route grace allowed only while still authorized.

## Risks

Memory overhead and old TTLs; client caches persist beyond local clearing; revocation creates failures rather than guaranteed immediate reconnect.

## Validation approach

P3-03 virtual clocks/concurrent scopes; P3-05 signed RRsets; P3-06 old TTL/new policy tests.
