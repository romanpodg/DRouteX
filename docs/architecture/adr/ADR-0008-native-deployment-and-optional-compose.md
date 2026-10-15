# ADR-0008: Native deployment and optional Compose

Status: **Proposed**. Date: 2026-10-15. Specification: §§21,22.

Scope: CR-0001 MVP native deployment needs DNS/TCP service permissions only, no TUN, VPN module, NET_ADMIN or custom privileged networking helper.

## Context

DNS/proxy services can deploy natively without Docker. Later TUN/kernel modules/nftables introduce additional platform and privilege requirements.

## Decision

Native Linux/systemd primary; separate users and resource/capability limits. Compose for compatible components with documented networking and narrow privileges.

## Alternatives

Compose privileged:true for all services; Windows production gateway; Docker-only installer.

## Reasons

Meets Linux routing and portable self-hosting expectations.

## Consequences

Pin supported distro/architecture and DNS/proxy service behavior via D03; grant only narrow bind capability/socket activation if required. WSL is a useful lab, not native production deployment proof. Pin VPN kernels/modules separately before G3.

## Risks

Port/module/IPv6/firewall conflicts; AWG userspace/kernel mismatch; container namespace differences.

## Validation approach

P11-01/P11-02 real fresh-host preflight/install and independent service failures.
