# ADR-0005: Secrets and restricted privileged operations

Status: **Proposed**. Date: 2026-10-15. Specification: §§15,17,19,21.

Scope: CR-0001 keeps secret protection in MVP; custom privileged executor and route/interface/firewall mutation are G3 only. MVP denies those operations and needs no NET_ADMIN.

## Context

Public administration and root routing changes create separate trust domains.

## Decision

Unprivileged controller/node services; MVP has no route/interface mutation, no custom root executor, and only narrow DNS bind capability/socket activation if needed. At G3 use authenticated local IPC to a minimal allowlisted executor. Standard encrypted secret storage with separately provisioned master key; scoped references/delivery and redacted types.

## Alternatives

Run API as root; shell command runner; keys beside DB; plaintext configs everywhere.

## Reasons

Least privilege, explicit validation and reviewed crypto avoid unnecessary host-wide exposure.

## Consequences

Executor permits named interfaces/routes/firewall/engine plans only; recovery and key rotation documented.

## Risks

Helper can still damage approved networking; master-key loss; plaintext may briefly exist in protected engine files.

## Validation approach

MVP: P1-07 rotation/redaction/restore and P10-02 denied OS operations/service permissions. G3: P10-01 injection/peer-credential tests and independent review.
