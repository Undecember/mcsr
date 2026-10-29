# Live keyd startup from an empty baseline

Status: approved by the user. Scope is the two startup blockers observed during
the supervised managed-keyd live test at `5ae4c41`.

## Intent

Allow live mode to start from an empty, restored `/etc/keyd`, so the user can
Preview and Apply the first profile. After a daemon restart, wait for verified
IPC capability readiness before reporting healthy input service state.

## Evidence

See [the live validation record](../evidence/2026-10-29-f3-keyd/live-validation.md).
The managed daemon responds to its capability probe but rejects `bind reset`
with exit 255 when no configs are loaded. Its IPC_BIND handler starts with
success false and only sets it while evaluating a loaded config.

`src/keyd_helper.rs` already polls for health, but its real `is-active` action
propagates a capability-probe connection failure as an error. The poll aborts
on that error even when systemd has only just started the daemon.

## Approach and alternatives

The proposed focused fix is to correct the two boundary behaviors:

- Extend `packaging/keyd/mcsr.patch` so the exact IPC bind expression `reset`
  succeeds when the config list is empty. With configs loaded, preserve the
  existing reset evaluation and held-key cleanup. Other bind expressions must
  retain their existing rejection behavior when no configs exist.
- Make the helper's existing health polling tolerate initial IPC unavailability
  while still requiring the running daemon's complete capability response.
  Preserve the five-second initial readiness deadline, 500 ms stability window,
  200 ms maximum sampling gap, and stop-on-failure behavior. A daemon that never
  becomes ready, loses readiness during stability, or lacks required
  capabilities must not be accepted as healthy. Include probe execution time
  in deadline accounting and preserve useful failure diagnostics.

Suppressing reset failures in the toolkit would hide real cleanup failures.
Installing a placeholder config would add artificial managed state and root
backup obligations merely to permit startup. Neither fits this recovery flow.

## Scope and constraints

Keep pristine vendored source and its checksum manifest unchanged; maintain the
daemon change in the existing patch. Add deterministic daemon IPC coverage
without root, real keyboards, or desktop input injection. Test the production
health-probe decision as well as the helper polling adapter so fake-only tests
cannot miss the observed early abort.

Do not change F3 routing, focus/chat rules, authentication scope, backup lineage,
or capability requirements. The uninstall/reinstall helper bootstrap gap is a
separate issue. Preserve the primary checkout's untracked evidence and cache.
Implementation belongs on an isolated `fix/*` worktree based on `dev`.
No commit, merge, push, or system installation is part of code implementation.

## Acceptance and verification

- Regressions fail before the fix and pass afterward: empty-config IPC reset
  succeeds; invalid binds remain rejected; populated resets still release held
  keys; transient initial IPC failures can recover; permanent failure,
  incompatible capabilities, stability loss, and deadline expiry remain errors.
- Verify the real IPC bind dispatcher using a device-free harness; existing
  event tests alone do not execute that path.
- Run targeted helper/runtime tests, the full locked Rust suite, fmt, Clippy,
  managed keyd engine/IPC tests, extension/UI harnesses, release build, and
  release asset tests. Refresh Graphify after code changes.
- Independently review cleanup correctness, deadline handling, and failure
  paths after deterministic verification.
- A subsequent supervised install and live startup must succeed from empty
  `/etc/keyd` before resuming Minecraft F3 acceptance. Automated checks do not
  establish that desktop result.

The installed service remains in healthy observe-only mode during development.
