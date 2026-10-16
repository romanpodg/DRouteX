# ADR-0013: Windows development and Linux network verification

Status: **Proposed**. Date: 2026-10-15. Specification: §§16,21,22.

Scope: MVP acceptance uses actual Linux DNS/TCP/proxy/TLS services without VPNs or TUN. Engine/kernel tests below belong to G3 and remote-node tests to G4.

## Context

Pure policy/config works on Windows; OS/kernel routing requires actual Linux evidence.

## Decision

Windows static/unit/frontend/user-space tests; disposable Linux VM/namespaces and approved engines for privileged integration. WSL only after capability checks. Dedicated trusted runner never executes arbitrary PR code with root.

## Alternatives

Windows-only mocks; production self-hosted root runner; acceptance through fake engines.

## Reasons

Matches platform behavior and isolates test privileges.

## Consequences

Pin supported kernel/clients; captures and egress observations retained; no nightly ordinary-build requirement.

## Risks

WSL lacks target AWG module; CI lacks privileges; remote WAN services change.

## Validation approach

P0-03 harness cleanup; P4-08/P10 current real DNS/proxy gates and P11 install/restore. Later P5/P8/P9 validate engines, tunnels and multiple nodes.

P0-03 evidence on 2026-10-15: the owner approved the existing WSL Ubuntu lab.
[Real dual-stack DNS/TLS/proxy fixtures and failure cleanup](../../planning/P0_03_NETWORK_HARNESS.md)
passed there. Native Debian 13/image/kernel/systemd acceptance remains pending;
this result does not change the distinction between WSL and native Linux proof.
