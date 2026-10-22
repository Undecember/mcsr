# MCSR Toolkit maccel Sensitivity Recovery Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Make focused Minecraft eye zoom safely set maccel sensitivity to `0.02083` and restore the exact previous value, without granting the desktop user access to maccel's root-executed reset scripts.

**Architecture:** A self-contained root-owned helper owns the fixed `SENS_MULT` sysfs write, baseline record, generation token, and recovery lock. The unprivileged service invokes only fixed `eye` and `restore` commands and separates window resize from sensitivity activation so it can check GNOME focus between them. It reports eye zoom unavailable or recovery pending when the helper cannot prove restoration. Normal window presets remain usable without the helper.

**Tech Stack:** Python 3.11+, pytest, Ruff, mypy, Ubuntu GNOME Wayland, maccel 0.6.0 kernel module, systemd user service, fixed sudoers commands.

**Spec:** [maccel sensitivity recovery design](../specs/2026-10-06-maccel-sensitivity-recovery-design.md), supplementing the [original toolkit design](../specs/2026-10-03-mcsr-toolkit-design.md).

**Parent plan:** [MCSR Toolkit implementation plan](2026-10-03-mcsr-toolkit.md). This addendum replaces its direct `maccel set` eye zoom steps.

## Global Constraints

- Keep the service unprivileged and localhost-only. The browser must not choose a privileged command, value, or target path.
- Do not add the desktop login to `maccel`, change its udev rule, invoke `maccel set`, or write `/var/opt/maccel/resets`.
- Install no helper, sudoers rule, or other privileged state until source review is complete and the user separately approves that exact deployment.
- Preserve existing keyd behavior and the empty `/etc/keyd` baseline. Normal F6 resizing must remain available without sensitivity support.
- Preserve the exact readable pre-zoom raw multiplier; `1` is only a reference normal value. A shifted eye shortcut changes size without changing sensitivity.
- A failed write, readback, or restore must not report eye zoom or normal sensitivity as confirmed. Keep recovery evidence and block another eye write until recovery succeeds.
- The GNOME focus signal is asynchronous. Recheck focus immediately before the eye write and after it; record the remaining brief interval instead of claiming zero leakage.
- A newer preset or any cleanup invalidates an older pending eye handler. An `asyncio.Lock` serializes preset handlers across awaited focus calls; the existing `threading.RLock` coordinates synchronous focus and restore work.
- Keep all repository work uncommitted unless the user separately requests a commit. User-selected execution mode for plan nuggets is Subagent-Driven.

## File map

| File | Responsibility |
| --- | --- |
| `src/mcsr_toolkit/maccel_privileged.py` | Self-contained root helper: fixed commands, raw sysfs IO, locked generation and baseline recovery state |
| `src/mcsr_toolkit/sensitivity.py` | Unprivileged fixed-command adapter, helper preflight, generation handoff, visible recovery status |
| `src/mcsr_toolkit/resize.py` | Preset controller invokes sensitivity adapter; normal/shifted behavior and recovery state |
| `src/mcsr_toolkit/__main__.py` | Startup restore, serialized focus transitions, post-write focus check, shutdown and retry |
| `packaging/mcsr-toolkit-maccel.sudoers` | Exact `eye` and `restore` sudo commands for the service login |
| `tests/test_maccel_privileged.py`, `tests/test_sensitivity.py`, `tests/test_resize.py`, `tests/test_api.py`, `tests/test_focus.py` | Privilege boundary, state transitions, focus and status regressions |
| `README.md`, `integration/resize-wayland.md`, `integration/acceptance-wayland.md` | Reviewed deployment, recovery, and measured live acceptance |

## Review Focus

1. A prior `eye` helper is queued when the service dies: startup `restore` must rotate generation so the old helper cannot write afterward (Task 1 process test; Task 3 service test).
2. The service loses effective Minecraft focus during resize or a helper call: post-operation validation restores sensitivity, and status does not claim active eye zoom (Task 3).
3. A helper writes the eye value but exits or readback fails: the baseline record survives and a later `restore` recovers the exact prior raw value (Task 1).
4. The readable normal value differs from `1`: eye zoom and repeated eye calls preserve and restore that exact original value (Task 1; Task 2).
5. Helper or sudo permission is absent while maccel `get` succeeds: normal resize works, but status marks eye zoom unusable and no sensitivity write is attempted (Task 2; Task 3).
6. F7 awaits focus while F6 is queued, or an awaited F7 outlives disconnect/shutdown: the old handler must not apply eye sensitivity after the newer action or cleanup (Task 3).

---

### Task 1: Implement the isolated root helper

**Files:** Create `src/mcsr_toolkit/maccel_privileged.py`, `tests/test_maccel_privileged.py`.

**Interfaces:** `main(argv: list[str] | None = None) -> int` accepts only `eye` or `restore`, with no extra arguments. `restore` returns a new opaque generation on stdout only after verified restoration; `eye` reads one generation token from stdin before taking the lock and returns the verified multiplier. A test-only `HelperPaths` value can inject disposable paths into internal functions; the deployed `main` always constructs fixed `/sys/module/maccel/parameters/SENS_MULT`, read-only `/sys/module/maccel/srcversion`, and `/run/mcsr-toolkit-maccel/` paths and ignores environment overrides. It supports only the observed `srcversion` `4AFA0F5B3EBD2F98EE54E56` for Q32 writes. The script's isolated shebang imports only the standard library.

- [ ] Write failing tests for command/argument rejection, unexpected ownership/modes and symlinks, malformed or stale generation, missing or different driver fingerprint, unsupported fixed-point representation, and exact 32-fraction-bit conversion. For this installed driver, `1` is raw `4294967296` and `0.02083` rounds to raw `89464169`, matching upstream `fpt_rconst`.
- [ ] Run `.venv/bin/python -m pytest tests/test_maccel_privileged.py -q` and confirm the new cases fail before implementation.
- [ ] Implement fixed-path safe opens, root-owned `0700` directory, `0600` durable records, bounded exclusive lock, baseline-before-write with fsync, verified sysfs readback, idempotent restore, and generation rotation. Validate finite positive signed fixed-point baselines. Reject any unexpected path or file state without writing.
- [ ] Add failure-injection tests for partial eye write/readback failure, restore failure, repeated eye preserving the first baseline, exact restoration of a non-`1` baseline, and a missing module. Add a process test where a stale eye helper has read its old token but waits for the lock while restore rotates generation; it must then reject without writing. Confirm no test path reaches `/var/opt/maccel/resets` or calls the maccel CLI.
- [ ] Run focused pytest, `.venv/bin/ruff check src/mcsr_toolkit/maccel_privileged.py tests/test_maccel_privileged.py`, and `.venv/bin/mypy src`; all pass. Submit this privilege boundary for independent Sol high review before Task 2.

### Task 2: Add the unprivileged fixed-command adapter

**Files:** Create `src/mcsr_toolkit/sensitivity.py`, `tests/test_sensitivity.py`, `packaging/mcsr-toolkit-maccel.sudoers`; modify `src/mcsr_toolkit/resize.py`, `tests/test_resize.py`.

**Interfaces:** `SensitivityAdapter.preflight() -> SensitivityStatus`, `startup_restore() -> None`, `eye() -> None`, and `restore() -> None`. The adapter invokes only `/usr/bin/sudo -n /usr/local/libexec/mcsr-toolkit-maccel eye` or `restore`, supplying the current generation to `eye` on stdin. It never accepts a value or path from a profile or API. `PresetController` receives this adapter as a dependency and separates an exact eye-size resize result from sensitivity activation so the runtime can check focus between them. It calls `restore` for normal presets or focus loss. The sudoers template is parameterized only by the desktop username and names the two exact commands, with no wildcard or arbitrary arguments.

- [ ] Write failing tests for the fixed subprocess argument arrays and stdin token, helper/root-path and exact sudo permission preflight, status when maccel is readable but helper/sudo is absent, startup restore token rotation, failed eye/restore recovery, and non-`1` baseline restoration. Ensure normal preset and Shift+F7 do not require a helper write.
- [ ] Run `.venv/bin/python -m pytest tests/test_sensitivity.py tests/test_resize.py -q` and confirm the new cases fail.
- [ ] Implement the adapter and controller integration, with an explicit resize phase and a separately callable eye activation phase. Remove direct `maccel set` calls from production eye zoom. Keep `maccel get param sens-mult` as a read-only observation, and make eye availability depend on both that read and helper preflight. Bound command timeouts; on ambiguous timeout or nonzero exit retain pending recovery and require a successful `restore` before another `eye`.
- [ ] Run the focused tests, Ruff, and mypy. Verify the sudoers template with `visudo -cf` using a temporary username-substituted candidate; do not install it. Obtain independent security review of command scope and status behavior.

### Task 3: Integrate startup, focus, and shutdown recovery

**Files:** Modify `src/mcsr_toolkit/__main__.py`, `src/mcsr_toolkit/api.py` only if status shape needs adjustment, `tests/test_focus.py`, `tests/test_api.py`, and focused runtime tests in `tests/test_sensitivity.py`.

**Interfaces:** `ServiceRuntime.start()` completes sensitivity startup recovery before it accepts preset signals. An `asyncio.Lock` serializes complete preset transitions across awaited D-Bus calls; the existing `_preset_lock` coordinates synchronous focus and restore work. A monotonically increasing local action epoch is captured by each handler; a newer preset, focus loss, extension disconnect, or shutdown invalidates older pending eye work. For eye zoom, the handler awaits an exact resize, checks that its epoch and connection are current, checks GNOME effective Minecraft focus again, invokes eye activation only if still current and focused, and rechecks immediately after the helper returns. `ServiceRuntime.close()` invalidates and drains/cancels pending handlers before final restore, reports a failure while retaining root recovery state. Focus blur, Shell modal, heartbeat timeout, extension disconnect, and a normal preset all reach the same idempotent restore path. `GET /api/status` distinguishes absent helper, usable eye zoom, and recovery pending without claiming restored sensitivity on failure.

- [ ] Write failing tests for startup with an orphaned old generation, focus loss during window resize, focus loss while helper runs, modal entry, heartbeat timeout, extension disconnect, clean shutdown, failed restore with retry, and normal F6 availability without helper. Add controlled `asyncio.Event` tests for F7 paused at the pre-write focus await followed by F6, and for a pending F7 during disconnect and shutdown; none may write eye after the later action/restore.
- [ ] Run focused tests red. Implement the smallest runtime changes with an async transition lock, action epoch checks under the synchronous lock, tracked preset tasks, a fresh pre-write D-Bus focus check, and a post-write check. Invalidate pending work before cleanup and await or cancel tracked tasks on disconnect/shutdown; ensure a late task cannot use a fresh helper generation. Preserve current keyd focus behavior and the known asynchronous modal timing limit.
- [ ] Run focused tests green, then the full Python suite, Ruff, mypy, both Node harnesses, GNOME schema dry run, and `git diff --check`. Submit the complete code diff for independent Sol high security/recovery review; repair material findings and rerun affected checks.

### Task 4: Document, deploy by separate approval, and run live acceptance

**Files:** Modify `README.md`, `integration/resize-wayland.md`, `integration/acceptance-wayland.md`; create a dated acceptance note as needed in the established integration location.

**Interfaces:** A runnable helper/sudoers setup and recovery guide, plus an evidence-based pass/fail record for focused 400×300 eye zoom, `0.02083` sensitivity, shifted behavior, exact baseline restoration, and the tested focus transitions.

- [ ] Document source hash/ownership checks, exact sudoers candidate validation, manual `restore`, removal order, and the upstream reset-script risk. Record that `maccel get` alone does not prove writes are available. Verify commands and local links. Preserve the earlier movement and F6 evidence.
- [ ] Before any privileged installation, finish the code review and present the exact helper source/hash, sudoers contents, installation commands, and rollback plan to the user for separate security-sensitive approval. Without approval, stop at documented, uninstalled code and mark live eye zoom pending.
- [ ] After approval, install only the reviewed root-owned helper and exact sudoers rule; verify hash, ownership, ancestors, `visudo`, and `sudo -n -l` without invoking eye. Record initial maccel raw and CLI values, `/var/opt/maccel/resets` ownership, and empty keyd configuration.
- [ ] With a fresh isolated observe-only service and Minecraft focused, use a background sampler to verify F6 at 800×600 and baseline sensitivity, Shift+F7 at 400×300 without sensitivity change, F7 at 400×300 with about `0.02083`, Shift+F7 during eye without sensitivity change, and F6 restoring the exact baseline. In separate eye cycles, verify blur, Shell modal, extension disconnect, clean shutdown, and startup recovery. Do not type into authentication fields during the probe.
- [ ] End with the helper baseline record absent, original raw and CLI multiplier restored, GNOME shortcuts cleared, toolkit service stopped, and `/etc/keyd` unchanged. If any restore fails, keep the evidence and use the reviewed `restore` command; do not report completion until the independent read confirms the baseline. Rerun relevant deterministic checks and ask an independent reviewer to inspect the final diff and acceptance record.

## Handoff

The user previously selected Subagent-Driven execution for subsequent plan nuggets. After the user approves this written plan, follow that mode with per-task implementation and independent reviews. Approval of this plan does not authorize the privileged installation in Task 4, a commit, push, or merge.
