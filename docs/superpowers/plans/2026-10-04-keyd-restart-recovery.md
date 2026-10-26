# Keyd Restart Recovery Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Replace every toolkit `keyd reload` path with bounded restart recovery, then prove that focus loss clears dynamic bindings and chat state without leaving unsafe keyboard input.

**Architecture:** The self-contained, root-owned helper owns a fixed `clear` command that restarts `keyd.service`, checks health within a bounded deadline, and stops the service on failure. The unprivileged focus service persists suspension before `keyd bind reset`, invokes only that exact helper command through passwordless sudo, and serializes cleanup with activation, apply, and restore. Install and restore continue to use authenticated elevation and the same bounded restart routine; live activation stays gated on the recorded desktop proof.

**Tech Stack:** Python 3.11+, pytest, Ruff, mypy, Ubuntu GNOME Wayland, keyd 2.5.0, systemd, sudoers, GNOME Shell extension.

**Spec:** `docs/superpowers/specs/2026-10-03-mcsr-toolkit-design.md`. This is a focused addendum to `docs/superpowers/plans/2026-10-03-mcsr-toolkit.md`; its restart and recovery instructions supersede that plan's reload instructions in Tasks 1, 4, and 10 and source notes.

## Global Constraints

- The user service remains unprivileged and bound to `127.0.0.1`; only the fixed root-owned helper touches `/etc/keyd` and `/var/lib/mcsr-toolkit/keyd`.
- `clear` is the sole passwordless sudo action: `/usr/local/libexec/mcsr-toolkit-keyd clear`; install and restore still require authenticated `pkexec` elevation.
- The helper remains self-contained under isolated `/usr/bin/python3 -I`, accepts no path or shell command from the browser, and leaves `~/.config/keyd/app.conf` untouched.
- Existing conflicting keyboard configs are rejected; toolkit files retain atomic writes, validated snapshots, and rollback to the original state, including an initially empty `/etc/keyd`.
- No routine cleanup, install, restore, rollback, or live acceptance step calls `keyd reload`. Ubuntu keyd 2.5.0 has no `keyd check` command.
- No production remaps or Task 9 browser UI work proceeds until the supervised focus, chat, modal, heartbeat, and crash-soak proof passes.
- Do not install the helper or sudoers rule, alter system keyd state, or start the live probe until this plan is explicitly approved. The design/security rule is already approved; plan approval gates execution.
- Do not commit, push, merge, or rewrite history without an explicit user request; this overrides the writing-plans skill's example commit steps.

## File map

| Path | Responsibility |
| --- | --- |
| `src/mcsr_toolkit/privileged.py` | Fixed root helper `clear`, bounded restart/health/stop, install and restore rollback |
| `src/mcsr_toolkit/keyd.py` | Unprivileged `bind reset` and exact `sudo -n ... clear` adapter |
| `src/mcsr_toolkit/focus.py`, `focus_watchdog.py`, `focus_pipe.py`, `__main__.py` | Effective-focus transitions, persisted suspension, single lock, retry, status |
| `tests/test_privileged.py`, `test_keyd.py`, `test_focus.py`, `test_focus_watchdog.py`, `test_focus_pipe.py`, `test_api.py`, `test_apply.py` | Failure injection and state-order regression tests |
| `packaging/mcsr-toolkit-keyd.sudoers` | Exact per-user sudoers template deployed under `/etc/sudoers.d/` |
| `README.md`, `integration/keyd-focus-probe.md`, `integration/acceptance-wayland.md` | Deployment, preflight, live proof, rollback, acceptance results |

## Review Focus

These five user-visible failure modes need explicit tests in the owning tasks:

1. `systemctl restart keyd.service` returns success but the service crashes later: Task 1 checks immediate health and stops an unhealthy daemon; Task 3 detects later loss, and Task 4 soaks longer than the observed 26-second crash delay.
2. `systemctl stop keyd.service` itself fails: Task 1 returns a distinct error and Task 3 keeps status unsafe and retry pending, without saying mappings are inactive.
3. An install into initially empty `/etc/keyd` fails after writing either file: Task 1 restores both files to absence, then verifies baseline daemon state or stops it.
4. A modal or heartbeat loss races an in-flight activation/apply: Task 3 serializes state and rechecks effective focus before every activation, and its tests force both interleavings.
5. Sudoers matches an extra helper argument or an untrusted helper copy: Task 2 tests the exact command and preflights ownership, mode, path, and sudo permission before live activation.

---

### Task 1: Replace privileged reload with bounded restart and rollback

**Files:** Modify `src/mcsr_toolkit/privileged.py`, `tests/test_privileged.py`.

**Interfaces:** Keep `KeydInstaller.install(fragment: str) -> str` and `restore(backup_id: str) -> None`. Add `restart_keyd_service(runner: Callable[[list[str], float], subprocess.CompletedProcess[str]], clock: Callable[[], float], sleep: Callable[[float], None]) -> None` for fixed `keyd.service`; allow at most five seconds to become active, then require 0.5 seconds of healthy checks. `main(["clear"]) -> int` accepts no payload and returns success only after healthy restart. Preserve the current fixed config and backup paths.

- [ ] **Write failing tests:** `test_clear_accepts_no_extra_argument`, `test_clear_restarts_and_checks_health`, `test_restart_timeout_stops_daemon`, `test_failed_stop_reports_unsafe_state`, `test_install_failure_restores_initially_empty_config`, and `test_restore_failure_preserves_current_snapshot`. Assert exact `systemctl` argument arrays, bounded polling, both toolkit files' final contents or absence, and no `reload` command. Use injected command results and a fake monotonic clock; avoid sleeping in unit tests.
- [ ] **Run red:** `.venv/bin/python -m pytest tests/test_privileged.py -q`; expect failures in the new restart/clear cases.
- [ ] **Implement:** Replace the `reload_keyd` callback and `_reload_keyd` with the fixed restart routine. Use `/usr/bin/systemctl` with exact `restart`, `is-active --quiet`, and `stop` argument arrays for `keyd.service`, each subprocess limited to five seconds. Poll health at 0.1-second intervals through the startup and brief stability window; on restart/health failure stop the service and report stop failure separately. Use the same routine after install, restore, and snapshot rollback. Keep backup validation before mutation and do not swallow rollback or stop errors.
- [ ] **Run green:** `.venv/bin/python -m pytest tests/test_privileged.py -q`; all pass. Then `.venv/bin/ruff check src/mcsr_toolkit/privileged.py tests/test_privileged.py` and `.venv/bin/mypy src`; both pass.

### Task 2: Wire the exact clear command and deployment preflight

**Files:** Modify `src/mcsr_toolkit/keyd.py`, `tests/test_keyd.py`; create `packaging/mcsr-toolkit-keyd.sudoers`; modify `README.md` only for this task's helper deployment instructions.

**Interfaces:** Keep `KeydCommands.activate_game() -> None`; change `KeydCommands.clear_state() -> None` to issue `keyd bind reset` first and `/usr/bin/sudo -n /usr/local/libexec/mcsr-toolkit-keyd clear` second. Keep `PrivilegedKeydAdapter.install/rollback/restore` on authenticated `/usr/bin/pkexec`; add a read-only preflight that verifies the fixed helper's root ownership and nonwritable path chain, executable mode, and that `sudo -n -l /usr/local/libexec/mcsr-toolkit-keyd clear` succeeds for the service login.

- [ ] **Write failing tests:** `test_clear_resets_before_exact_sudo_command`, `test_clear_attempts_helper_even_if_reset_fails`, `test_clear_reports_helper_failure`, `test_preflight_rejects_mutable_helper_path`, and `test_preflight_rejects_missing_exact_sudo_permission`. Assert no call to `keyd reload`, no variable sudo arguments, and that install/restore still use `pkexec`. A reset error must not skip the helper restart; report both failures if both occur. Test the sudoers template has one command with no wildcard or argument suffix.
- [ ] **Run red:** `.venv/bin/python -m pytest tests/test_keyd.py -q`; expect the new command/preflight cases to fail.
- [ ] **Implement:** Use fixed argument arrays only. Put `<desktop-user> ALL=(root) NOPASSWD: /usr/local/libexec/mcsr-toolkit-keyd clear` in the reviewed template; deployment substitutes the actual login name, writes root-owned mode `0440` under `/etc/sudoers.d/`, and validates with `visudo -cf` before live use. Document `sudo -n -l` as a permission check that does not execute `clear`, plus helper path and hash review. Keep installation and restore authenticated.
- [ ] **Run green:** `.venv/bin/python -m pytest tests/test_keyd.py -q`; all pass. Then `.venv/bin/ruff check src/mcsr_toolkit/keyd.py tests/test_keyd.py` and `.venv/bin/mypy src`; both pass.

### Task 3: Persist and serialize focus suspension

**Files:** Modify `src/mcsr_toolkit/focus.py`, `src/mcsr_toolkit/focus_watchdog.py`, `src/mcsr_toolkit/focus_pipe.py`, `src/mcsr_toolkit/__main__.py`, `tests/test_focus.py`, `tests/test_focus_watchdog.py`, `tests/test_focus_pipe.py`, `tests/test_api.py`, `tests/test_apply.py`.

**Interfaces:** `FocusKeydBridge.clear_state() -> None` persists the existing `.suspended.json` marker before the Task 2 reset and clear; it removes the marker only after confirmed success when the active profile remains valid. `FocusKeydBridge.clear_recovery() -> None` retains the marker on failure and drops the active profile only after successful cleanup. `ServiceRuntime._suspend_activation()`, `_retry_clear()`, `_before_apply()`, `_prepare_restore()`, and `_schedule_reconcile()` use one `RLock` for focus and mutation transitions; `activate_game()` rechecks `GnomeWindowBus.get_focus()` and refuses modal or stale focus. `GET /api/status` exposes cleanup pending and any stop failure without claiming safe inactivity.

- [ ] **Write failing tests:** `test_blur_persists_marker_before_reset`, `test_successful_clear_retains_profile_for_focus_reentry`, `test_failed_clear_retries_after_service_restart`, `test_stop_failure_is_visible_and_never_reports_safe`, `test_keyd_crash_after_clear_suspends_activation`, `test_modal_during_activation_blocks_new_bind`, `test_heartbeat_loss_during_apply_blocks_activation`, and `test_restore_serializes_with_pending_cleanup`. Force ordering with barriers/fake adapters, not timing sleeps; assert reset precedes helper clear, marker survives errors, and activation occurs only after a fresh effective-focus query.
- [ ] **Run red:** `.venv/bin/python -m pytest tests/test_focus.py tests/test_focus_watchdog.py tests/test_focus_pipe.py tests/test_api.py tests/test_apply.py -q`; expect the new suspension/race cases to fail.
- [ ] **Implement:** Route focus blur, Shell modal, FIFO disconnect, heartbeat timeout, startup, apply, restore, and reconciliation through the same serialized suspension path. Persist the marker before the first reset; keep it and the recovery profile across process restart after any failed clear. Retry under the same lock, block activation while pending, and only resume after confirmed cleanup and a new GNOME focus/modal query. Poll keyd health while live activation is enabled so a later daemon crash suspends activation and appears in status. Preserve the existing `--enable-live-keyd` gate and observe-only default. Surface stop failure explicitly; asynchronous modal delivery may leave a brief input interval, so do not claim zero leakage.
- [ ] **Run green:** `.venv/bin/python -m pytest tests/test_focus.py tests/test_focus_watchdog.py tests/test_focus_pipe.py tests/test_api.py tests/test_apply.py -q`; all pass. Then `.venv/bin/ruff check src tests` and `.venv/bin/mypy src`; both pass.

### Task 4: Prove live behavior and empty-config rollback before UI

**Files:** Modify `integration/keyd-focus-probe.md`; create or modify `integration/acceptance-wayland.md`; adjust only source or tests exposed by a failed check.

**Interfaces:** A dated pass/fail record with commands, versions, focus events, daemon state, journal observations, ordinary typing observations, and cleanup. This is the release gate for original-plan Task 9, not a simulated unit-test substitute.

- [ ] **Deploy after plan approval:** Install the reviewed helper at `/usr/local/libexec/mcsr-toolkit-keyd` as root-owned executable mode `0755`; substitute the actual service login into the exact Task 2 sudoers template, validate the candidate with `visudo -cf`, then install it under `/etc/sudoers.d/` as root-owned mode `0440`. Record the previous helper/sudoers state for rollback. Do not install a broad sudo rule or enable live activation yet.
- [ ] **Preflight without further system mutation:** Confirm helper source review and deployed copy hash match; verify root ownership/mode for helper and ancestors, sudoers `visudo -cf` and exact `sudo -n -l` result, active `keyd` group for the service login, current `/etc/keyd` snapshot, and the keyd panic sequence. Keep live activation off if any preflight fails.
- [ ] **Run the supervised probe:** With an operator at the keyboard, install only the restricted candidate through authenticated elevation, begin with a harmless remap, and record actual dynamic bind activation. Repeatedly check Minecraft focus entry and blur, `T`/`/` chat entry, `Enter`/`Esc` exits, blur while chat is open, Shell authentication modal entry/exit, heartbeat loss, and ordinary typing in another app. Record any input delivered during the known asynchronous modal interval; treat unsafe input as failure and stop the probe.
- [ ] **Run crash soak and rollback:** After the final restart, observe `keyd.service` and journal for longer than 26 seconds with no SIGSEGV or restart loop. Restore the original config through the authenticated helper, including the empty-config case; verify both managed files are absent if absent initially, keyd is healthy or explicitly stopped on failure, and ordinary typing remains normal. Record exact commands and timestamps.
- [ ] **Gate:** Mark the proof PASS only if every focus/chat/modal/heartbeat/typing/rollback check passes and the soak completes; otherwise leave `--enable-live-keyd` off and Task 9 unstarted. Update the acceptance record with the measured modal interval and any remaining limits.

### Task 5: Reconcile documentation, verification, and independent review

**Files:** Modify `README.md`, `integration/keyd-focus-probe.md`, `integration/acceptance-wayland.md`, and `docs/superpowers/plans/2026-10-03-mcsr-toolkit.md` only as needed to remove obsolete reload instructions and link this addendum.

**Interfaces:** A runnable deployment and recovery guide plus a recorded independent review of the complete recovery diff. Original-plan Task 9 starts only after Task 4 PASS.

- [ ] **Update documentation:** Replace all operational reload directions with the fixed helper restart/clear flow, exact sudoers deployment and removal, `visudo` and `sudo -n -l` preflight, authenticated install/restore, empty-config rollback, stop-failure recovery, and the modal timing limit. Verify commands against the target machine before presenting them as accepted procedure.
- [ ] **Run deterministic checks:** `.venv/bin/python -m pytest -q`, `.venv/bin/ruff check .`, `.venv/bin/mypy src`, `node tests/extension_harness.cjs`, and `git diff --check`; record results and repair concrete failures. Search tracked source and docs for remaining `keyd reload` directions; historical failure notes may retain the term when clearly marked historical.
- [ ] **Independent review:** Give a reviewer the revised spec, this addendum, raw diff, relevant source, test results, and live record. Require explicit review of reset-before-clear order, fixed sudo scope, stop failure/status honesty, lock ordering, profile recovery after restart, rollback to empty config, and the async focus interval. Repair material findings and rerun affected checks before calling the gate complete.
- [ ] **Report:** Summarize passing and pending evidence, remaining risks, and Task 9 gate status. Leave all Git changes uncommitted unless the user explicitly requests a commit.
