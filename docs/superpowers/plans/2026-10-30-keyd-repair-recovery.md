# Automatic keyd recovery in repair

**Goal:** Make `mcsr-toolkit repair` perform the external keyd recovery procedure discussed in this session, without manual removal of uncertainty records.

**Design:** Before ordinary repair preflight, allow only the known keyd recovery blockers. Validate the desktop-owned uncertainty record and installed helper. Stop the toolkit and hold its runtime lock. Run an authenticated fixed-target helper recovery operation under the existing root keyd lock: validate configurations and snapshots, reset the keyd start limit, restart and verify service/IPC health. Preserve configured profiles and backups. Remove only the unchanged uncertainty record after verified recovery; retain it and leave the toolkit stopped on failure. Installation, doctor, mutation recovery, and maccel recovery retain their existing checks.

**Approaches:** Blind marker deletion is unsafe. Reusing ordinary `clear` cannot reset a start-limit failure and should not silently defeat restart throttling during ordinary operation. An explicit authenticated `keyd recover` operation keeps that authority confined to requested repair.

**Review ruling:** A free helper lock does not rule out a delayed privileged child acquiring it later. Capture a root-owned generation in request stdin before privilege escalation, check it under the operation lock, and rotate it during recovery. Reject unfenced production mutations. Keep exact helper argv and sudoers scope unchanged. Detect a failed managed keyd service even without existing recovery records.

**Execution:** Inline in the existing checkout, preserving the user's current installer changes. No commits or system deployment. The user's request adopts the recovery procedure from the preceding conversation.

- [x] Add regression tests for successful recovery before preflight and for failure, foreign/malformed markers, concurrent runtime/helper operations, and changed markers.
- [x] Implement the repair recovery phase and the fixed helper recovery operation; keep ordinary clear's restart-limit behavior unchanged.
- [x] Verify helper and installer tests, then the Rust suite and formatting/diff checks. Build the binary and update Graphify.
- [x] Independent review of the recovery changes and their failure paths.

## Verification

- `cargo test --locked --quiet`: final run passed (RTK summary: 456 passed, 24 suites). An earlier full run hit `clean_shutdown_releases_the_service_lock`; the isolated test and full rerun both passed. Runtime locking behavior was not changed, apart from exposing its existing guard for repair.
- `cargo clippy --all-targets --locked -- -D warnings`, formatting, and `git diff --check`: passed.
- Release binary built; `cargo test --release --locked --test rust_assets --quiet`: 6 passed.
- `graphify update .`: completed, 2870 nodes / 7043 edges. AST extraction noted unsupported symbol files and refreshed community names; no semantic extraction was requested.
- Reviewer findings about late privileged children and late recovery children were fixed with generation fencing; reviewer confirmed no remaining findings.
- No system executable/helper deployment or live desktop recovery acceptance was performed.
