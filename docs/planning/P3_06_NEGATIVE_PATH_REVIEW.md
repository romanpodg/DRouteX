# P3-06 controlled negative-path evidence

Reviewed 2026-11-10. Baseline runtime:
`ac80c2d054342241b8cad09b6f6c251eb1542975`. This follow-up adds only tests and
evidence. No production admission, timeout, origin or relay behavior changed.

## Scope and results

The owner authorized the four negative-path groups and testing on the connected
dedicated Debian server. The tests exercise the actual preview listener, signed
deployment/admission, ClientHello inspection, origin response interpretation,
connection/relay and owned shutdown with real loopback TCP/UDP sockets.

Origin DNS and the final dial use explicit test-only substitution. ClientHello
is synthetic; the new tests do not perform cryptographic TLS or establish a
public gateway/origin route. No external domain, public test listener, firewall
or host DNS change was needed. The existing Linux OpenSSL test separately ran a
real local TLS/certificate/hostname check; it also retains its controlled dial.

The [new cases](../../crates/droutex-gateway/src/listener/preview/negative.rs)
cover the requested groups:

| Group | Case and assertion |
|---|---|
| Wrong SNI/ECH/unapproved target | `wrong_sni_and_ech_refuse_before_origin_with_a_working_positive_control`: unauthorized name and ECH return no bytes and produce neither origin DNS nor TCP traffic. The same fixture then completes a valid bidirectional exchange, preventing a broken setup from counting as successful refusal. |
| Lease expiry | `lease_expiry_during_a_live_connection_blocks_both_directions`: a valid exchange precedes real conservative clock expiry. The test requires explicit core `AdmissionCode::Lease`, then checks no newly supplied client or origin bytes cross the connection. |
| Configuration and revocation | Separate `generation_replacement_during_transfer_blocks_both_directions` and `route_revocation_during_transfer_blocks_both_directions` replace the signed generation or remove the route after an acknowledged exchange. Both directions must then close without forwarding late bytes. |
| Origin failure and strict policy | `unavailable_origin_closes_the_client_without_a_successful_relay` keeps the numeric port reserved but non-listening. `origin_loss_after_exchange_closes_client_without_reconnecting` closes an established origin after valid traffic, observes client EOF/reset and no reconnect in the bounded observation window. `unsupported_protected_policy_never_reaches_a_working_direct_origin` confirms a real authorized `ProtectedProxy` intent and proves the preview never connects its available Direct test origin. Protected transport itself remains unimplemented. |

These cover pending reads awakened by post-change input. They do not cover every
publication schedule, saturated pending writes, an encrypted long-running TLS
transfer, or every origin/proxy outage. Fresh requests through cached DNS answers
remain covered separately by the earlier manual public-origin probe.

## Fixture controls and failure history

- The ordinary host clock API runs in a bounded blocking context; no fake clock,
  enlarged production deadline or relaxed lease check is introduced. The manual
  lease retains its minimum legal lifetime by starting in the past, while its
  remaining lifetime is short enough for a bounded test.
- New fixtures share a test-only semaphore so their native inventory subprocess
  startup does not compete with another new fixture. Existing tests and product
  queues/limits are unchanged.
- Initial Windows and Debian lease attempts hit `CheckpointRequired` instead of
  lease expiry. An initial Windows positive control also timed out. Fixtures now
  use the existing gateway test's conservative clock window, and the short lease
  expires before its next checkpoint boundary. Earlier failures remain failures;
  they are not counted as lease-expiry proof or as a production fix.
- Review found a test-only race between a successful authority check and a
  heartbeat crossing expiry. Heartbeats were removed from the bounded expiry
  wait; the initial bidirectional proof and explicit Lease refusal remain.
- Clippy initially rejected redundant `matches!(result, Ok(_))`; it was replaced
  with `result.is_ok()` without changing refusal assertions.
- Windows CI on `6d561d4` in run 42856025377 passed 49 gateway cases but failed
  the lease observer on `AdmissionCode::Unavailable`. The fixture's concurrent
  clock refresh sets `checkpoint_failed` while committing its floor, intentionally
  returning Unavailable; a due floor can similarly return CheckpointRequired.
  The observer now retries only these maintenance refusals within its existing
  12-second bound. Only explicit Lease completes the wait. Other refusals still
  fail, the clock worker must join successfully, and the initial bidirectional
  exchange plus both post-expiry byte denials remain mandatory. No runtime
  admission check, lease or deadline is relaxed.
- Windows mutation checks compiled and failed behaviorally when current authority checks
  were disabled, and independently when `ProtectedProxy` was incorrectly allowed
  through the Direct connector. Failure was the missing bounded connection
  closure. Both mutations were restored before final verification.

## Actual platform evidence

Pinned compiler on both hosts: Rust 1.99.0 (`b940084d7`).

| Command/check | Windows native | Debian 13.7 x86_64 native |
|---|---|---|
| `cargo fmt --all -- --check` | PASS | PASS |
| `cargo clippy -p droutex-gateway --all-targets --all-features --locked -- -D warnings` | PASS | PASS |
| `cargo test -p droutex-gateway --all-features --locked` | 50 tests and one doctest PASS | 51 tests and one doctest PASS, including the Linux-only OpenSSL test |
| `cargo clippy -p droutex-gateway -p droutex-dns --all-targets --all-features --locked -- -D warnings` | Gateway portion checked above; full bundle not repeated | PASS |
| `cargo test -p droutex-dns --all-features --locked` | Not repeated; earlier owner-host IPv6 failures remain recorded | 123 tests PASS, including IPv6 and native filesystem/checkpoint cases |

Full runs above report zero failed, ignored or filtered tests. The seven new
negative cases pass on both hosts. The Windows OpenSSL case is not compiled;
the differing totals are not equivalent TLS evidence.

Debian application tests ran as a separate ordinary UID 1000 with effective
capabilities `0000000000000000`, not as the SSH setup account. Build concurrency
was one on the small server. Rust was installed in that test user's profile;
existing host services and network settings were preserved.

The initial remote copy was a tracked-only archive of the baseline, with only
the module declaration and new test file overlaid. For initial commit `e2a3327`,
local/remote SHA-256 values matched for the tested source:

- `listener/preview.rs`:
  `d38cda4077f3598e1d8f9179f64bf18f5700b892845aacb026bce93de019bd43`
- `listener/preview/negative.rs`:
  `3a6b44e7c5c5e5745533a1450eccd2f99137c14cb39622023c8f048086447221`

The initial commit packages those tested sources with this evidence. Remote
access keys, addresses, credentials, caches and build output are not part of the
commit. Current-head hosted CI/account review remain separate gates.

The clock-maintenance observer follow-up passes five focused lease repetitions
on each host, then the full gateway suite: Windows 50 tests and one doctest;
Debian 51 tests and one doctest including OpenSSL. Applicable formatting/Clippy
also pass. The repetitions intentionally filter unrelated cases; final full
gateway runs have zero failed, ignored or filtered tests. This test-only change
uses the separately qualified libsqlite3-sys 0.39.1 / SQLite 3.54.1 profile; it
does not change production code or require repeating unrelated DNS/storage runs.
The updated `negative.rs` SHA-256, normalized to LF, is
`66769d63b5afc583a98e6d039ed2d177ebb09c0defb719c779015e89dd4adaeb`.
Separate local standards/spec passes found no concrete remaining defect.

## Acceptance boundary

The owner's earlier native Linux deferral is superseded for these component
checks by the 2026-11-10 server authorization and the actual Debian results.
This does not complete native service deployment, public ingress, protected
proxy egress, all network-failure races, privilege/package acceptance or the
independent external parser security audit. P3-06/P4-03 remain open; no release,
public exposure or bypass of checks is implied by this evidence.
