# Uninstall keyd recovery implementation plan

> **For agentic workers:** Use superpowers:executing-plans for approved inline execution, or the user-selected execution skill. Track the steps below.

**Goal:** Fix uninstall on this device and expose the real cause of unverified recovery status.

**Architecture:** Preserve the privileged recovery and uninstall transaction boundaries. Reuse the runtime’s keyd executable selection. Do not equate a failed reset with clear recovery: permit the empty-configuration case only after trusted recovery checks and daemon reload/restart establish that no toolkit remaps remain. Preserve authentication and helper hash checks, while reporting their errors.

**Tech Stack:** Existing Rust executable and test suite; installed keyd 2.5.0.

## Scope and constraints

- Work on `fix/uninstall-keyd` in `.worktrees/fix-uninstall-keyd`, based on `dev`.
- No deployment, live uninstall, commits, merges, or changes to installed helpers.
- Keep user backups, unrelated keyd configurations, policies, and pending recovery records safe.
- Preserve refusal on unknown recovery, unsafe paths, failed authenticated checks, and failed daemon recovery.
- Update the existing README operational guidance.

## Review focus

Missing keyd executable; empty configurations; unrelated keyd configurations; failed reset with active/uncertain recovery; mismatched helper and cancelled authentication. A nonzero exit with empty output must include the exit status.

## Task 1: Uninstall recovery

Files: `src/install.rs`, `src/keyd.rs`, existing helper tests in `src/keyd_helper.rs` if helper changes are required, `tests/rust_install.rs`.

- [x] Add failing regression tests for selecting `keyd.rvaiya` before the fallback, empty-configuration uninstall, and failure paths retaining helpers and restarting the previous toolkit service.
- [x] Run focused regression tests and confirm the failures.
- [x] Reuse keyd path selection and adjust recovery ordering so clearing toolkit state can reload the daemon before any necessary reset; require trusted verification rather than ignoring reset errors. Keep non-toolkit configurations intact.
- [x] Test reset failure with active/unknown recovery, daemon restart failure, and successful existing uninstall rollback behavior.
- [x] Run focused install/keyd/helper tests.

## Task 2: Recovery diagnostics and validation

Files: `src/install.rs`, existing tests, `README.md`.

- [x] Add failing tests proving helper mismatch/authentication errors remain visible and still block mutation; test malformed status and nonzero commands with empty stderr.
- [x] Preserve the root status verification error through discovery/preflight and include exit status in failed-command diagnostics.
- [x] Document that a downloaded version may be unable to verify a differently versioned installed helper, and explain running the matching installed doctor.
- [x] Run `cargo fmt --check`, `cargo test --all-targets`, and `cargo clippy --all-targets -- -D warnings`.
- [x] Run `graphify update .`; report graph tooling limitations rather than hiding them.
- [x] Review the complete diff independently for recovery safety and requirement coverage. Leave changes uncommitted for review.
