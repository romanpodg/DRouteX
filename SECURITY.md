# Security policy

## Project status and supported versions

DRouteX is in pre-release development. The DNS/proxy MVP has not been released or
accepted for production deployment. There is no supported production release or
security maintenance/LTS commitment. Reports about main and active development
PRs are welcome; include the exact commit because version 0.1.0 alone does not
identify a reviewed implementation. See [current execution status](docs/planning/EXECUTION_STATUS.md)
for accepted components and remaining runtime/release gates.

## Report a vulnerability privately

Use [GitHub's private vulnerability reporting form](https://github.com/romanpodg/DRouteX/security/advisories/new).
The repository owner receives the report through GitHub. Do not put vulnerability
details, credentials or private captures into public issues or PR comments.
If the private form is unavailable, open a public issue containing only a request
for a private security contact; omit the affected feature and technical details.

Include:

- Affected commit/branch, OS, configuration and required access or prerequisites.
- Expected and observed behavior, impact and minimal reproducible steps or a
  self-contained proof of concept using synthetic data and systems you control.
- Relevant sanitized logs and whether a fix or workaround is known. Remove real
  credentials, resolver capabilities, keys and unrelated traffic/person information.

The owner will assess reports, discuss reproduction and coordinate remediation
and disclosure in the private report. No guaranteed response/remediation timeline,
paid bounty or security certification is promised. Please allow private assessment
before public disclosure and discuss disclosure timing in the report.

## Security boundaries and verification

The [threat model](docs/security/THREAT_MODEL.md) describes assets, trust boundaries,
residual risks and control evidence. [Key lifecycle](docs/security/KEY_LIFECYCLE.md)
and [gateway authorization](docs/security/GATEWAY_BINDING.md) define narrower contracts.
Accepted design, implemented library behavior, merged checks and full product
acceptance are distinct; passing CI or dependency qualification is not a penetration test.

The MVP requires separately authorized DNS/gateway access, checked destination
addresses, bounded resources, protected secrets and fail-closed proxy behavior.
Shared NAT access is group authority; DNS settings alone cannot enforce all device
traffic or guarantee ECH/QUIC routing. Tunnels and privileged network execution
remain later work. Testing should use authorized isolated environments; this policy
does not authorize probing third-party systems or deploying a public service.
