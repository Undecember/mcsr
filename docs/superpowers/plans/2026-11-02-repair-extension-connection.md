# Repair extension connection

Goal: recover the missing focus heartbeat observed during live repair without requiring manual extension toggling.

Approach: classify a missing heartbeat separately from other readiness failures. After the normal bounded startup wait, stop the producer, disable and verify release of the owned extension, re-enable it, restart the service, and require stable readiness again. Retry once; preserve existing observe-only fallback and recovery markers on persistent failure. Do not restart unrelated extensions or reconnect on unrelated readiness errors.

- [x] Reproduce stale connection with installer regression tests; observe failure.
- [x] Implement classified disconnect and one bounded restart within installer rollback scope.
- [x] Cover persistent disconnect, release failure, transient heartbeat, and unrelated routing failures.
- [x] Run installer tests, full Rust suite, formatting and clippy; refresh Graphify.

Live evidence: disabling/re-enabling the toolkit extension restored extension_connected; subsequent repair exited successfully and doctor reported live mode ready. No physical keypress was verified.

Verification: the stale-connection and healthy-precheck/disconnect regressions both failed before their fixes. Full `cargo test --locked`, `cargo fmt --check`, `cargo clippy --all-targets --locked -- -D warnings`, release build, and release asset tests passed. Independent review found the identical-path race; it was fixed and re-reviewed without further findings. Graphify updated.

Deployment attempt: `target/release/mcsr-toolkit repair` refused with `root status helper hash differs; manual review required`. The new executable remains at target/release/mcsr-toolkit; installed executable and helper retain matching prior hashes. No privileged hash checks were bypassed. The existing service still reports live activation and a connected extension; status contains a resize cancellation after focus loss/Shell modal entry.
