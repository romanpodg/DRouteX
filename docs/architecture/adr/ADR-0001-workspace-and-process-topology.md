# ADR-0001: Workspace and process topology

Status: **Proposed**. Date: 2026-10-15. Specification: §§5,15,16.

Scope: Current MVP uses separate controller and DNS/proxy runtime only. Executor/engine processes below are post-MVP under CR-0001.

## Context

The spec permits fewer processes but requires Control/Data Plane independence and isolated engines.

## Decision

Use a modular workspace with separate unprivileged controller and DNS/proxy runtime. Add a distinct restricted executor and independent engines only in G3. Instantiate only crates required by the current task.

## Alternatives

Single everything-in-one process; full microservices; one crate per speculative feature.

## Reasons

CP outage cannot affect forwarding; privilege and failure isolation justify process boundaries without distributed complexity.

## Consequences

A local node coordinator owns DNS/gateway generation activation; later split component processes only on evidence.

## Risks

Node-runtime shared failure domain; poor common-crate growth; Windows/Linux integration differences.

## Validation approach

P0-02 cross-host compile; P1-06/P4-08/P10-02 current crash/permission tests. P5-08/P10-01 add actual engine/helper tests at G3.
