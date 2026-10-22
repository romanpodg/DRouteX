# D45: authorization before public DNS transport

Date: 2026-10-22. **Accepted by direct owner reply: P4-01 before P3-01.**
This decision accepts execution order only; D46 separately records technical approval.
Prerequisite: normal PR106 merge, all eleven exact-main jobs and both processed
Codecov reports. P2-06 is not complete until this evidence is verified.

## Existing authority and observed gap

The original owner continuation goal is P1-05, P1-06 and all seven P2 tasks.
P3-01 is the next DNS transport task. Its current backlog prerequisites are P0-03
and P2-05. P4-01 is independently ready after P2-05/P0-01. The DAG does not force
public DNS to wait for P4-01.

The accepted P0-01 foundation, however, explicitly requires:

> Pin maximum lease duration, clock-uncertainty handling, exact quota values and the
> registration/replacement API in P4-01 before exposing a listener.

Source: docs/planning/P0_01_FOUNDATION_REVIEW.md, public-ingress D05/D08 section.
DNS_PRODUCT_MODEL.md also requires bounded expiry during CP loss and says numeric
limits and clock handling are D08 implementation prerequisites. Current config
LegacyGrant has exact source/listener/group/Endpoint associations, but no expiry
or live authorization state. CompiledConfiguration.legacy_scope and
LegacyDnsAssignment explicitly represent static desired scope, not live grants.
Treating these static tables as public admission would invent missing authority.

## Accepted order

1. Close P2-06 only after PR106 exact-main verification.
2. Design and agree the complete P4-01/D08 authorization profile first, then
   qualify any necessary dependency/config/codec changes, implement and normally
   merge it with actual Windows/Linux negative tests and independent account review.
   Exact-address legacy DNS and gateway scopes remain separate. Expiry during CP
   loss, clock uncertainty, applied revocation and manual bootstrap need explicit
   choices; this order proposal does NOT decide their values or approve new fields.
3. Then agree the P3-01 bounded UDP/TCP transport and production dependency profile,
   qualify and implement it against actual current authorization, rather than
   assigning live meaning to existing static config. Real loopback/lab traffic
   remains distinct from authorization to expose a public production listener.
4. Continue P3-02..05 and P3-06 against that independently authorized route context.
   Keep early P6-01 authentication mandatory before external management/grant APIs
   and personal DoH/DoT P3-07. This proposal does not postpone or waive that gate.

This is an execution-order clarification, not a change to product scope or a new
MVP dependency on post-MVP work. All existing tasks and security acceptance remain.
If accepted, document the public-admission prerequisite in the planning DAG before
P3 implementation; the exact minimal edge/wording must preserve the existing
acyclic MVP graph and be checked by the planning validator.

## Alternative

Qualify and implement an explicitly non-public, loopback-only P3-01 transport
component first, then integrate P4-01 authorization before any public listener.
This produces real framing/limit tests earlier, but leaves P3-01 admission acceptance
and the public resolver boundary open across two stages. It needs a separately
agreed narrowed acceptance profile; no such exception is currently accepted.

## What acceptance means

Accept only the recommended order and public-admission prerequisite. A separate
reviewable P4-01 profile must still be approved before consequential implementation.
No package installation, new dependency, numeric lease/clock/quota policy, config
field, API, live grant, public bind, privileged operation or network exposure is
authorized by this proposal. Existing PR106 completion proceeds independently.
