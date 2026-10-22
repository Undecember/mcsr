# MCSR Toolkit Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Build a local Ubuntu GNOME Wayland web toolkit that downloads the official MCSR tools, applies focus-aware keyd mappings, keeps all MCSRLauncher instance controls aligned, and manages window size and optional eye zoom sensitivity.

**Architecture:** A Python user service serves a small browser UI and owns profile, download, instance, apply, and focus logic. A dedicated keyd config holds an inactive chat layer. The GNOME Shell extension reports effective input focus and handles focused-window resizing; the user service applies dynamic game bindings on Minecraft focus and uses the fixed root helper to restart keyd on blur. Privileged keyd installation is a narrow separate operation.

**Tech Stack:** Python 3.11+, FastAPI, Uvicorn, Pydantic, HTTPX, pytest, Ruff, mypy, vanilla HTML/CSS/JavaScript, GNOME Shell JavaScript (GJS), keyd, optional maccel. Pin Python dependencies in `pyproject.toml` and use no frontend build tool.

**Spec:** `docs/superpowers/specs/2026-10-03-mcsr-toolkit-design.md`

**Recovery update (2026-10-04):** Follow [the keyd restart recovery addendum](2026-10-04-keyd-restart-recovery.md) for all keyd cleanup, install, restore, rollback, deployment, and live verification. Use its reset-and-helper-clear sequence and bounded service restart; the older direct reload steps below are obsolete. Task 9 remains gated until the addendum's supervised proof passes.

## Global Constraints

- Target Ubuntu GNOME on Wayland with keyd already installed; support one desktop user on one device.
- Bind the HTTP server to `127.0.0.1`; run it as the user, never as root.
- Match any focused Minecraft window; other applications must receive ordinary keys.
- Keep toolkit keyd rules in a dedicated config and fragment. Refuse conflicting existing device configs and leave `app.conf` untouched.
- Update every MCSRLauncher-managed instance, including instances discovered later; do not touch other launchers.
- Ninjabrain Bot mappings emit shortcuts already configured in that bot; do not edit its preferences.
- Search snippets are left-hand typing goals, not text-expansion macros.
- Use `0.02083` for eye zoom and `1` as the reference normal maccel multiplier; shifted eye zoom leaves sensitivity unchanged.
- Preserve existing user files, validate before activation, and keep restorable backups.
- Do not commit, push, merge, or rewrite Git history without an explicit user request. This overrides the skill's example commit steps.

## File map

| Path | Responsibility |
| --- | --- |
| `pyproject.toml`, `README.md` | Install, run, check, and Ubuntu prerequisites |
| `src/mcsr_toolkit/models.py`, `profile.py` | Typed profile and persisted user state |
| `src/mcsr_toolkit/bindings.py` | Physical-to-emitted mapping, control preview, conflicts |
| `src/mcsr_toolkit/instances.py`, `options.py` | Launcher discovery and lossless `options.txt` edits |
| `src/mcsr_toolkit/keyd.py`, `privileged.py`, `focus.py`, `focus_watchdog.py`, `focus_pipe.py` | Restricted keyd rules, narrow install, effective focus, modal and heartbeat recovery |
| `src/mcsr_toolkit/resize.py`, `extension/` | Resize preset IPC, GNOME focus and hotkeys, maccel state |
| `src/mcsr_toolkit/downloads.py` | Official release lookup and safe JAR download |
| `src/mcsr_toolkit/apply.py` | Backup, apply, deferred edits, restore, per-instance results |
| `src/mcsr_toolkit/api.py`, `static/` | Local API and browser pages |
| `tests/`, `integration/` | Deterministic tests and live desktop acceptance procedure |

## Review Focus

These five cases deserve explicit tests in their owning tasks:

1. An existing keyd config already claims the keyboard: Task 4 must preserve it and reject installation.
2. `options.txt` differs between launcher instances or contains duplicate keys: Task 3 must preserve unrelated lines and report ambiguity per instance.
3. A profile is applied twice or an instance is created afterward: Tasks 3 and 7 must avoid remapping drift and bring the new instance up to date.
4. Minecraft loses focus while chat or eye zoom is active: Tasks 4 and 5 must restore ordinary keys and normal sensitivity.
5. A remote release response redirects to an unapproved host, is malformed, or serves a non-JAR: Task 6 must keep the previous download intact and report failure.

---

### Task 1: Prove the GNOME Wayland integration path

**Files:** Create `integration/keyd-focus-probe.md`, `integration/keyd-focus-probe.sh`, `integration/gnome-probe/metadata.json`, `integration/gnome-probe/extension.js`.

**Interfaces:** Produces the tested Minecraft window predicate (GNOME window class/title and keyd mapper class/title), keyd version and config syntax, the shell resize method, and a written pass/fail record consumed by Tasks 4 and 5.

- [ ] Inspect the installed keyd commands, mapper, GNOME Shell version, and a live Minecraft window. Record actual window identifiers; do not guess from `java` alone.
- [ ] Record the initial mapper's failure on Shell authentication modals and toggled chat state. Validate revised focus events, the reset-and-helper-clear sequence on blur, chat entry/exit, and modal entry/exit with harmless live remaps. This Ubuntu keyd build has no `check` command; validate the restricted generated grammar before activation.
- [ ] Build the minimal GNOME extension probe that resizes only the focused Minecraft window with Mutter's `Meta.Window.move_resize_frame` and reports its resulting frame rectangle.
- [ ] Run the probe on Ubuntu GNOME Wayland and record commands, observed results, versions, cleanup, and the final predicate in `integration/keyd-focus-probe.md`. Stop and revise the design/plan if focus or chat state cannot be reset reliably. Do not continue to the UI on a failed probe.

### Task 2: Define the profile and validation contract

**Files:** Create `pyproject.toml`, `src/mcsr_toolkit/__init__.py`, `src/mcsr_toolkit/models.py`, `src/mcsr_toolkit/profile.py`, `tests/test_profile.py`.

**Interfaces:** `Profile` contains `controls: dict[str, str]` (Minecraft action ID to physical key), `crafting: dict[str, str]` (physical key to emitted key), `ninjabrain: dict[str, str]` (physical key to shortcut), `chat: ChatKeys`, `presets: list[ResizePreset]`, `launcher_root: Path | None`; `load_profile(path: Path) -> Profile`, `save_profile(path: Path, profile: Profile) -> None`.

- [ ] Write tests for default chat keys `t`, `slash`, `enter`, `esc`, valid `s -> k`, duplicate physical shortcuts, invalid key names, duplicate preset shortcuts, nonpositive dimensions, and a round-trip saved profile. Run `pytest tests/test_profile.py -q` and see the tests fail before implementation.
- [ ] Implement the typed models, a canonical key-name table sourced from `keyd list-keys` where available, and atomic profile persistence under the user's XDG data directory. Configure Ruff and mypy in `pyproject.toml`. Keep the optional launcher root in the profile, but validate it as an MCSRLauncher root before using it.
- [ ] Run `pytest tests/test_profile.py -q`; expected: all pass. Add a concise README setup section with `python -m venv`, install, and test commands that match `pyproject.toml`.

### Task 3: Discover instances and compose Minecraft controls

**Files:** Create `src/mcsr_toolkit/instances.py`, `src/mcsr_toolkit/options.py`, `src/mcsr_toolkit/bindings.py`, `tests/test_instances.py`, `tests/test_options.py`, `tests/test_bindings.py`, `tests/fixtures/options-1.16.txt`.

**Interfaces:** `locate_launcher_root(managed_dir: Path, configured_root: Path | None, running_processes: list[ProcessInfo]) -> Path | None`; `discover_instances(launcher_root: Path) -> list[Instance]`; `parse_options(text: str) -> OptionsDocument`; `plan_controls(document: OptionsDocument, profile: Profile, prior: ManagedState | None) -> ControlPlan`; `render_options(document: OptionsDocument, plan: ControlPlan) -> str`. `ControlPlan` exposes preview rows `(physical, emitted, action)` and conflicts. `ManagedState` stores original physical assignments and last emitted values per instance for idempotence.

- [ ] Read the MCSRLauncher source for its `launcher/instances/<id>/instance.json` and `minecraft/options.txt` layout (with legacy `.minecraft` fallback); add fixtures for two valid instances, unrelated directories, missing `options.txt`, and a symlink escaping the launcher root. Test root lookup from the toolkit-managed launcher directory, a running launcher's working directory, and an explicit override; report no root when none of those identifies MCSRLauncher. Run discovery tests red.
- [ ] Write options and binding tests proving `s -> k` updates Backward to `key.keyboard.k`, unrelated settings and formatting survive, other affected controls are updated, duplicate `options.txt` action entries and duplicate emitted control keys are reported, and a second apply produces identical output. Run those tests red.
- [ ] Implement path-confined discovery, lossless line editing, key-name translation for the supported Minecraft options format, mapping composition, and the per-instance managed-state record. Reject unsupported control formats with a per-instance error; never replace the whole file with a template.
- [ ] Run `pytest tests/test_instances.py tests/test_options.py tests/test_bindings.py -q`; expected: all pass. Document the supported MCSRLauncher layout and `options.txt` format in README.

### Task 4: Generate and install focus-aware keyd rules

**Files:** Create `src/mcsr_toolkit/keyd.py`, `src/mcsr_toolkit/privileged.py`, `tests/test_keyd.py`, `tests/test_privileged.py`.

**Interfaces:** `render_keyd_fragment(profile: Profile) -> str`; `game_bindings(profile: Profile) -> list[str]`; `KeydCommands.activate_game/clear_state`; `FocusController.on_focus/on_disconnect`; `KeydInstaller.install/restore`. The root-owned helper accepts only a restricted toolkit fragment and fixed targets under `/etc/keyd/`, then restarts keyd with fixed arguments.

- [ ] Write tests for game remaps, chat identity bindings and entry/exit actions, focus/modal reset, heartbeat failure, an existing keyboard config, invalid keyd output, and a failed restart that restores previous files. Run them red.
- [ ] Implement the probe-proven restricted config syntax. Keep chat rules in an inactive toolkit fragment and apply game bindings only for effective Minecraft focus. Never emit keyd `command(...)` or shell text from profile input.
- [ ] Implement a self-contained root-owned helper with fixed target paths, temporary writes, backup/rollback, restricted grammar validation, and a bounded keyd service restart with health checking. The user service invokes it through authenticated elevation, not by running as root. Reject path traversal and symlink target changes.
- [ ] Run `pytest tests/test_keyd.py tests/test_privileged.py -q`; expected: all pass. On the probe machine, repeat the focus/chat checks with generated rules and verify the keyd panic sequence remains documented.

### Task 5: Resize the focused window and manage eye zoom sensitivity

**Files:** Create `extension/metadata.json`, `extension/extension.js`, `extension/schemas/org.mcsr.toolkit.gschema.xml`, `src/mcsr_toolkit/resize.py`, `tests/test_resize.py`, `integration/resize-wayland.md`.

**Interfaces:** The extension exposes a session-bus `ResizeFocused(width: uint32, height: uint32) -> (applied_width: uint32, applied_height: uint32)` method and reports a preset ID to the user service. `apply_preset(profile: Profile, preset_id: str, shifted: bool) -> ResizeResult`; `restore_sensitivity() -> None`. The extension owns focus validation; the service owns `maccel` calls.

- [ ] Write tests for invalid or missing focused window, size limits, an eye zoom preset applying `0.02083`, a shifted eye zoom leaving DPI unchanged, returning to a normal preset restoring `1`, and focus loss during eye zoom restoring sensitivity. Run `pytest tests/test_resize.py -q` red with a fake extension bus and fake maccel runner.
- [ ] Implement the GNOME extension using the predicate proved in Task 1, Mutter resize, preset shortcut registration, and a user-session D-Bus interface. Return the observed frame size; do not report success merely because a resize request was sent.
- [ ] Implement the Python preset controller with an allowlisted `maccel` invocation, availability/status detection, and a single active-preset state. Keep the prior normal multiplier when readable; use `1` as the documented reference. Ensure an extension disable or Minecraft focus loss restores sensitivity.
- [ ] Run `pytest tests/test_resize.py -q` and perform `integration/resize-wayland.md` on GNOME Wayland with Minecraft and another app open; expected: only Minecraft resizes, actual dimensions are reported, and DPI returns to normal.

### Task 6: Download the two official JARs

**Files:** Create `src/mcsr_toolkit/downloads.py`, `tests/test_downloads.py`.

**Interfaces:** `resolve_release(product: Literal["launcher", "ninjabrain"]) -> ReleaseAsset`; `download_release(asset: ReleaseAsset, destination: Path) -> DownloadResult`. `ReleaseAsset` carries version, official source URL, asset URL, and filename; `DownloadResult` carries version, final path, and byte count.

- [ ] Verify the current official MCSR Ranked Linux launcher link and Ninjabrain GitHub release asset naming; record the selected resolution rules in a test fixture, not a hard-coded version. Write mocked HTTP tests for success, offline, malformed metadata, non-HTTPS or unallowlisted redirect, non-JAR response, and interrupted download preserving the previous file. Run tests red.
- [ ] Implement allowlisted official sources, bounded HTTP timeouts and size, streamed temporary download, JAR ZIP header/content validation, and atomic replacement in an XDG data subdirectory. Display resolved version and path; do not execute downloaded JARs.
- [ ] Run `pytest tests/test_downloads.py -q`; expected: all pass. Perform one live download check of each official source when network access is available.

### Task 7: Coordinate apply, deferred instances, and restore

**Files:** Create `src/mcsr_toolkit/apply.py`, `tests/test_apply.py`.

**Interfaces:** `preview(profile: Profile) -> ApplyPreview`; `apply(profile: Profile) -> ApplyReport`; `reconcile_new_instances(profile: Profile) -> ApplyReport`; `restore(backup_id: str) -> ApplyReport`. `ApplyReport` includes per-instance updated/deferred/failed entries and keyd/extension status.

- [ ] Write tests for successful multi-instance apply, one malformed instance, a running Minecraft process, a newly created instance, repeat apply, keyd install failure, and restore after a partial apply. Run `pytest tests/test_apply.py -q` red with temporary launcher roots and fake system adapters.
- [ ] Implement one validation/preview pass before writes; stage backups and proposed files; install keyd only after preparation succeeds; atomically replace closed-instance options; record deferred running instances; roll back toolkit-managed changes on failure and report any remaining per-instance failures.
- [ ] Reconcile deferred edits when their processes exit and scan for new instances when the service starts and after each apply. Use launcher instance paths and process working directories rather than a broad `java` name match. Persist enough managed state to make repeated reconciliation idempotent.
- [ ] Run `pytest tests/test_apply.py -q`; expected: all pass. Document backup location and explicit restore behavior.

### Task 8: Expose a local API and user service

**Files:** Create `src/mcsr_toolkit/api.py`, `src/mcsr_toolkit/__main__.py`, `packaging/mcsr-toolkit.service`, `tests/test_api.py`.

**Interfaces:** `create_app(dependencies: Dependencies) -> FastAPI`; routes `GET /api/status`, `GET /api/profile`, `PUT /api/profile`, `POST /api/preview`, `POST /api/apply`, `POST /api/restore`, `POST /api/download/{product}`, `GET /api/jobs/{id}`. Responses expose structured validation and per-instance errors; downloads and apply are jobs so the UI stays responsive.

- [ ] Write API tests for local binding configuration, valid profile/preview/apply, accepted validated launcher-root override, rejected off-origin write, rejected bad Host, rejected invalid path/command data, concurrent apply rejection, and job failure reporting. Run `pytest tests/test_api.py -q` red.
- [ ] Implement FastAPI routes, static asset serving, loopback-only Uvicorn startup, local session token and Origin/Host checks for writes, job serialization, and adapter injection. Never expose the privileged helper or raw subprocess command through an API route.
- [ ] Add a `systemd --user` unit so focus/resize and deferred reconciliation continue when the browser tab closes. Run `pytest tests/test_api.py -q`; expected: all pass. Verify the service command in a clean virtual environment.

### Task 9: Build the browser UI

**Files:** Create `static/index.html`, `static/app.js`, `static/style.css`, `integration/acceptance-wayland.md`.

**Interfaces:** UI calls Task 8 routes and presents setup status, official downloads, control/crafting/Ninjabrain/chat configuration, resize presets, mapping preview, apply results, and restore.

- [ ] Build the single-page vanilla JavaScript UI. Keep snippet examples as typing goals, show physical → emitted → action rows, block Apply on validation conflicts, and show per-instance failures or deferrals without hiding successful results.
- [ ] Exercise keyboard navigation and the complete profile → preview → apply → restore flow in a browser against a fake launcher root; record observations in `integration/acceptance-wayland.md`.

### Task 10: Complete documentation and end-to-end verification

**Files:** Modify `README.md`, `integration/acceptance-wayland.md`; adjust only files exposed by verification failures.

**Interfaces:** Produces a runnable installation guide and a recorded acceptance result for the spec.

- [ ] Document installation, keyd group/elevation requirements, GNOME extension enablement, optional maccel, launcher-root override, service startup, backup/restore, and troubleshooting for failed focus or resize detection. Verify each command on the target Ubuntu GNOME Wayland machine.
- [ ] Run `pytest -q`, `ruff check .`, `mypy src`, the restricted keyd validator and live daemon candidate check, `glib-compile-schemas --strict --dry-run extension/schemas`, and the live acceptance procedure in `integration/acceptance-wayland.md`. Record tool versions, commands, and outcomes; repair concrete failures and rerun affected checks.
- [ ] Review the final diff against the spec, including preservation of unrelated files and the five Review Focus cases. Leave Git changes uncommitted unless the user separately requests a commit.

## Source notes for implementers

- MCSRLauncher source uses `launcher/instances/<id>/instance.json`, and `BasicInstance.getGamePath()` chooses `minecraft/` with a legacy `.minecraft/` fallback: [InstanceManager](https://github.com/MCSRLauncher/Launcher/blob/main/src/main/kotlin/com/redlimerl/mcsrlauncher/launcher/InstanceManager.kt), [BasicInstance](https://github.com/MCSRLauncher/Launcher/blob/main/src/main/kotlin/com/redlimerl/mcsrlauncher/data/instance/BasicInstance.kt).
- keyd permits a device ID in only one top-level `.conf`; this first release refuses competing configs. Ubuntu keyd 2.5.0 lacks `check`, so restrict candidate grammar and verify a live service restart: [keyd manual](https://github.com/rvaiya/keyd/blob/master/docs/keyd.scdoc).
- Exact Wayland window sizing is available through Mutter's `Meta.Window.move_resize_frame`: [Mutter API](https://gnome.pages.gitlab.gnome.org/mutter/meta/class.Window.html).
