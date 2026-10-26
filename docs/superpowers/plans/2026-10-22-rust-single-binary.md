# Rust Single-Binary Migration Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Replace the Python service and privileged helpers with one downloadable Rust executable and a guided installer, while preserving MCSR Toolkit behavior on Ubuntu GNOME Shell 50 x86_64.

**Architecture:** A Rust executable contains the user service, narrowly scoped root helper command modes, installer, browser assets, and GNOME extension archive. The installed user service remains unprivileged; a verified root-owned copy handles fixed keyd and maccel operations. Rust uses fresh state and refuses cutover from unresolved Python recovery state.

**Tech Stack:** Rust stable, Cargo.lock, Tokio, Axum, Serde, zbus, a pinned HTTPS client, embedded static assets, the existing GJS extension and browser JavaScript, systemd user service, keyd, optional maccel.

**Spec:** [Rust single-binary design](../specs/2026-10-22-rust-single-binary-design.md)

## Global Constraints

- First binary release: Ubuntu GNOME Shell 50 on x86_64. Test the release artifact on that target.
- Users download one executable and need neither Python nor Rust at runtime. The installer extracts the embedded GJS extension; keyd and optional maccel remain external.
- Preserve existing HTTP route paths/JSON shapes, profile JSON shape, focus FIFO messages, GNOME D-Bus interface, keyd fragment grammar, and observable Apply/Restore semantics.
- Store Rust recovery state under `$XDG_DATA_HOME/mcsr-toolkit/rust-v1/`, separate from Python's `$XDG_DATA_HOME/mcsr-toolkit/profile.json` and `state/`. Do not import, overwrite, or delete Python profile, backups, or unresolved recovery records.
- Start the installed service in observe-only mode. Do not enable live keyd or maccel during unattended installation.
- Keep privileged commands fixed-target, root-owned, argument-limited, and separate from the HTTP server. Passwordless sudo permits only exact keyd `clear` and maccel `eye`/`restore` commands.
- Before any live privileged installation or keyboard/sensitivity activation, obtain the separate approval required by AGENTS.md. Do not merge, push, tag, or commit during plan execution without the applicable explicit request.
- Tasks 1–13 use automated tests, fixtures, mocks, and temporary directories only. Defer every check involving the real GNOME session, systemd user unit, `/etc/keyd`, physical keyboard, Minecraft window, or maccel device to final Task 14.

## Review Focus

1. An active Python service or unresolved `mutation.json`, keyd snapshot, or maccel baseline must block cutover without changing input state (Task 13 test).
2. A symlinked or user-writable helper path or root recovery record must fail closed before any privileged write (Tasks 9 and 10 tests).
3. An interrupted multi-instance Apply must leave a durable marker and must not resume live activation on restart (Task 4 test).
4. A stale maccel eye call after blur, normal preset, or service restart must not reapply eye sensitivity (Tasks 10 and 12 tests).
5. A bad HTTP Host, cross-origin write, or missing session token must not mutate profile or instance files (Task 6 test).

## File map and task boundaries

Create `Cargo.toml`, `Cargo.lock`, `build.rs`, `src/main.rs`, and `src/lib.rs` for the executable and library. New Rust modules live under `src/`: `assets.rs`, `profile.rs`, `options.rs`, `instances.rs`, `mapping.rs`, `apply.rs`, `api.rs`, `downloads.rs`, `focus.rs`, `gnome.rs`, `keyd.rs`, `keyd_helper.rs`, `maccel.rs`, `maccel_helper.rs`, `runtime.rs`, `install.rs`, and `paths.rs`. Cargo integration tests use `tests/rust_*.rs`; existing Python and JavaScript tests remain available as reference until cutover. Do not edit the Python implementation merely to make a Rust test pass. Keep the existing `static/` and `extension/` sources during migration, then remove Python source, tests, and packaging only after the installed Rust artifact passes acceptance.

Tasks 1–7 establish the data, API, and GNOME components; Task 12 integrates them into an observe-only Rust service. Tasks 8–12 implement input and sensitivity behavior behind disabled live gates and review it with simulated dependencies. Task 13 implements the guided installer against a fake system. Task 14 performs release checks and the single final real-environment acceptance session. Execute in order; a later task may rely only on the interfaces named in earlier tasks. Run focused tests after each task and the full Rust suite at each stage boundary. Review each task independently before continuing, especially the root helpers.

### Task 1: Rust artifact and embedded assets

**Files:** Create `Cargo.toml`, `Cargo.lock`, `build.rs`, `src/lib.rs`, `src/main.rs`, `src/assets.rs`, `tests/rust_assets.rs`; reuse `static/*` and `extension/*`.

**Interfaces:** Produce `assets::web_asset(path: &str) -> Option<(&'static [u8], &'static str)>` and `assets::extension_archive() -> &'static [u8]`. `main` parses explicit `serve`, `install`, `uninstall`, `doctor`, and `privileged` modes; unimplemented modes return an error until their tasks land.

- [ ] Write `rust_assets` tests asserting `/` returns embedded HTML, `/app.js` returns JavaScript, unknown paths return none, and the embedded extension archive contains `metadata.json`, `extension.js`, and a compiled schema.
- [ ] Run `cargo test --test rust_assets`; confirm missing module/interface failure.
- [ ] Implement compile-time embedding and deterministic extension archive generation. Fail the build if schema compilation or required entries fail.
- [ ] Run `cargo test --test rust_assets`, `cargo build --release --locked`, `node static/app.test.cjs`, and `node tests/extension_harness.cjs`; all pass.

### Task 2: Profile validation and mapping contract

**Files:** Create `src/profile.rs`, `src/mapping.rs`, `src/options.rs` (document type only), `tests/rust_profile.rs`, `tests/fixtures/profile-default.json`, and `tests/fixtures/keyd-chat.conf`; read `src/mcsr_toolkit/models.py`, `bindings.py`, `keyd.py`, and their tests.

**Interfaces:** Produce `profile::Profile` with Serde JSON shape matching Python, `profile::parse_profile(bytes: &[u8], keys: &KeySet) -> Result<Profile, ProfileError>`, and `mapping::plan_controls(options: &OptionsDocument, profile: &Profile, previous: Option<&ManagedState>) -> Result<ControlPlan, MappingError>`; `ControlPlan` exposes preview rows, conflicts, next state, and rendered keyd bindings.

- [ ] Write failing tests for default profile roundtrip, `s`→`k` Backward preview using an in-memory `OptionsDocument`, left-hand snippet goals without macro output, chat-key conflicts, duplicate controls/presets, and unsupported key names. Compare saved Python fixture JSON and keyd fragment bytes.
- [ ] Run `cargo test --test rust_profile`; confirm missing contract failure.
- [ ] Implement exact validation limits from `models.py` and the existing mapping order; use an injected keyd vocabulary provider in tests and preserve the offline fallback behavior.
- [ ] Run `cargo test --test rust_profile` and `cargo clippy --all-targets -- -D warnings`; both pass.

### Task 3: Minecraft options and MCSRLauncher discovery

**Files:** Complete `src/options.rs`, create `src/instances.rs`, `tests/rust_instances.rs`; reuse `tests/fixtures/options-1.16.txt`.

**Interfaces:** Produce `options::parse_options(bytes: &[u8]) -> Result<OptionsDocument, OptionsError>`, `options::render_options(options: &OptionsDocument, plan: &ControlPlan) -> Vec<u8>`, `instances::discover_instances(root: &Path) -> Result<Vec<Instance>, DiscoveryError>`, and `instances::locate_launcher_root(managed_dir: &Path, override_root: Option<&Path>, process_cwds: &[PathBuf]) -> Option<PathBuf>`.

- [ ] Write failing tests asserting exact line-ending and unrelated-setting preservation, Minecraft 1.16 key aliases, `.minecraft` fallback, unsafe launcher roots rejected, malformed/missing `instance.json` reported, and every valid instance discovered.
- [ ] Run `cargo test --test rust_instances`; confirm failure.
- [ ] Implement parse/render and discovery using explicit path validation and process-state inputs rather than caller-supplied arbitrary file targets.
- [ ] Run `cargo test --test rust_instances` and `cargo fmt --check`; both pass.

### Task 4: Transactional Apply, Restore, and reconciliation

**Files:** Create `src/apply.rs`, `src/paths.rs`, `tests/rust_apply.rs`; reference `src/mcsr_toolkit/apply.py` and `tests/test_apply.py`.

**Interfaces:** Produce `ApplyCoordinator::preview(&self, profile: &Profile) -> Result<ApplyPreview, ApplyError>`, `apply(&self, profile: &Profile, keyd: &dyn KeydAdapter) -> Result<ApplyReport, ApplyError>`, `restore(&self, backup_id: &str, keyd: &dyn KeydAdapter) -> Result<ApplyReport, ApplyError>`, and `reconcile_new_instances(&self, profile: &Profile, keyd: &dyn KeydAdapter) -> Result<ApplyReport, ApplyError>`. `KeydAdapter` exposes `install(&Profile)`, `checkpoint()`, `restore_to(&[String])`, and `rollback()` with `Result` returns. Persist Rust-only `managed`, `lineage`, `backup`, and `mutation` records under `$XDG_DATA_HOME/mcsr-toolkit/rust-v1/`.

- [ ] Write failing tests for two closed instances updated together, a running instance deferring activation, idempotent Apply, new-instance reconciliation, LIFO Restore, changed-file/launcher-root rejection, and injected failure after the first options write leaving a durable mutation marker that blocks restart activation.
- [ ] Run `cargo test --test rust_apply`; confirm failure.
- [ ] Implement private owned directories, atomic temp-file replacement plus file/directory sync, backup-before-mutation ordering, and marker cleanup only after complete success.
- [ ] Run `cargo test --test rust_apply`; all cases pass, including byte-exact Restore of the original fixture.

### Task 5: Official downloads

**Files:** Create `src/downloads.rs`, `tests/rust_downloads.rs`; reference `src/mcsr_toolkit/downloads.py`.

**Interfaces:** Produce `downloads::resolve_release(product: Product, client: &dyn HttpClient) -> Result<ReleaseAsset, DownloadError>` and `downloads::download_release(asset: &ReleaseAsset, destination: &Path, client: &dyn HttpClient) -> Result<DownloadResult, DownloadError>`.

- [ ] Write failing mock-server tests for launcher and Ninjabrain release resolution, HTTPS/host allowlists, redirect rejection or revalidation, expected JAR/ZIP structure, truncated transfer, and atomic replacement without corrupting a previous download.
- [ ] Run `cargo test --test rust_downloads`; confirm failure.
- [ ] Implement bounded HTTP timeouts, asset checks, and temporary-file install; never pass remote values to a shell.
- [ ] Run `cargo test --test rust_downloads`; all cases pass.

### Task 6: Loopback API and serialized jobs

**Files:** Create `src/api.rs`, `tests/rust_api.rs`; reference `src/mcsr_toolkit/api.py`, `static/app.js`, and `tests/test_api.py`.

**Interfaces:** Produce `api::router(Arc<ApiState>) -> axum::Router` with `GET /api/session`, `/api/status`, `/api/profile`, `/api/jobs/{id}`; `PUT /api/profile`; `POST /api/preview`, `/api/apply`, `/api/restore`, `/api/download/{product}`; and embedded web routes. `ApiState` owns a one-job-at-a-time queue, profile lock, per-process token, and injected coordinator/download/runtime adapters.

- [ ] Write failing tests for response JSON and status codes, saved-profile stale check, observe-only Apply refusal, one-job queue behavior, bad Host, cross-origin writes, missing/wrong session token, and static assets served from the executable. Verify rejected requests leave profile/options bytes unchanged.
- [ ] Run `cargo test --test rust_api`; confirm failure.
- [ ] Implement loopback-only binding and the existing Host/Origin/token rules; preserve UI-facing error codes and wait for jobs to drain on shutdown.
- [ ] Run `cargo test --test rust_api` and `node static/app.test.cjs`; both pass.

### Task 7: GNOME bridge, focus, and resize in observe-only mode

**Files:** Create `src/focus.rs`, `src/gnome.rs`, `tests/rust_focus.rs`, `tests/rust_gnome.rs`; reuse `extension/extension.js`.

**Interfaces:** Produce `FocusEvent`, `FocusState::observe(&mut self, event: FocusEvent) -> FocusTransition`, `FocusState::expire(&mut self, now: Instant) -> FocusTransition`, `FocusState::disconnect(&mut self) -> FocusTransition`, async `GnomeBridge::configure_presets(&self, profile: &Profile) -> Result<(), GnomeError>`, async `GnomeBridge::resize_focused(&self, width: u32, height: u32) -> Result<(u32, u32), ResizeError>`, and a cancellable FIFO/D-Bus listener.

- [ ] Write failing tests for Minecraft class matching, focus heartbeat timeout, Shell modal clearing, disconnect clearing, stale preset signal rejection, non-Minecraft resize refusal, and exact-size mismatch reported as failure.
- [ ] Run `cargo test --test rust_focus --test rust_gnome`; confirm failure.
- [ ] Implement existing FIFO message and session D-Bus method/signal contract; emit effective-focus changes to runtime without activating keyd yet.
- [ ] Run focused Rust tests, `node tests/extension_harness.cjs`, and `glib-compile-schemas --strict --dry-run extension/schemas`; all pass.

### Task 8: Unprivileged keyd controller

**Files:** Create `src/keyd.rs`, `tests/rust_keyd.rs`; reference `src/mcsr_toolkit/keyd.py`, `focus.py`, and `focus_watchdog.py`.

**Interfaces:** Implement Task 4's `KeydAdapter`; expose `KeydController::activate(&self, profile: &Profile, focus: &FocusState) -> Result<(), KeydError>`, `clear(&self) -> Result<(), KeydError>`, and `preflight_clear(&self) -> Result<(), KeydError>`.

- [ ] Write failing tests for chat-layer grammar, activation only with effective Minecraft focus, modal/blur/disconnect cleanup, `keyd bind reset` before helper clear, activation blocked after cleanup failure, and no claim of safe input when keyd stop fails.
- [ ] Run `cargo test --test rust_keyd`; confirm failure.
- [ ] Implement command execution through fixed argument vectors and a serial transition lock. Keep live activation disabled on the device until Task 14 supervised acceptance.
- [ ] Run `cargo test --test rust_keyd`; all cases pass.

### Task 9: Root-owned keyd helper

**Files:** Create `src/keyd_helper.rs`, `tests/rust_keyd_helper.rs`; reference `src/mcsr_toolkit/privileged.py` and `tests/test_privileged.py`.

**Interfaces:** Dispatch only `privileged keyd install`, `restore`, and `clear` from the root-owned executable path. Install accepts the restricted fragment on stdin; restore accepts only a recorded backup ID; clear accepts no payload or extra arguments.

- [ ] Write failing tests for exact command/argument validation, fragment injection rejection, conflicting `/etc/keyd` config refusal, symlink/user-writable ancestors and snapshots, snapshot restoration, bounded restart health, and stop-on-failed-restart.
- [ ] Run `cargo test --test rust_keyd_helper`; confirm failure.
- [ ] Port the fixed targets and rollback behavior. Test through injected filesystem/systemctl adapters; do not touch real `/etc/keyd` in automated tests.
- [ ] Run focused tests and independent security review of this task's diff; resolve material findings before live deployment.

### Task 10: Root-owned maccel helper

**Files:** Create `src/maccel_helper.rs`, `tests/rust_maccel_helper.rs`; reference `src/mcsr_toolkit/maccel_privileged.py` and its recovery design.

**Interfaces:** Dispatch only `privileged maccel eye` and `restore`; eye reads the generation token from stdin. Root-owned runtime records hold a generation and the exact signed Q32 baseline. Fixed sysfs path and driver fingerprint are copied from the approved source.

- [ ] Write failing tests for exact command/path restrictions, unsupported fingerprint, malformed raw value, non-`1` baseline, write/readback failure, repeated eye, concurrent eye/restore, symlink/permission attacks, and a stale queued eye call rejected after restore rotates generation.
- [ ] Run `cargo test --test rust_maccel_helper`; confirm failure.
- [ ] Port the locked baseline-before-write protocol and exact readback recovery; keep tests on fake sysfs/runtime paths.
- [ ] Run focused tests and independent high-risk review; resolve material findings before live deployment.

### Task 11: Unprivileged sensitivity and preset controller

**Files:** Create `src/maccel.rs`, extend `src/gnome.rs`, create `tests/rust_sensitivity.rs`; reference `src/mcsr_toolkit/sensitivity.py` and `resize.py`.

**Interfaces:** Produce async `SensitivityController::startup_restore(&self) -> Result<(), SensitivityError>`, async `eye(&self) -> Result<(), SensitivityError>`, async `restore(&self) -> Result<(), SensitivityError>`, `needs_restore(&self) -> bool`, and async `PresetController::apply(&self, profile: &Profile, preset_id: &str, shifted: bool, focus: &FocusState) -> Result<ResizeResult, PresetError>`. Controllers serialize helper calls internally; focus invalidation remains observable while a call is pending.

- [ ] Write failing tests for F7 eye after exact resize, Shift+F7 preserving current multiplier, F6 restoring exact baseline even if resize fails, focus loss during an awaited eye call, and unavailable/failed helper blocking new eye writes.
- [ ] Run `cargo test --test rust_sensitivity`; confirm failure.
- [ ] Implement operation generation/cancellation, fresh focus checks before and after helper calls, and startup restore before eye availability.
- [ ] Run focused tests and independent review; all cases pass.

### Task 12: Service runtime and lifecycle

**Files:** Create `src/runtime.rs`, extend `src/main.rs`, create `tests/rust_runtime.rs`; reference `src/mcsr_toolkit/__main__.py`.

**Interfaces:** Produce async `ServiceRuntime::start(config: RuntimeConfig) -> Result<Self, RuntimeError>`, async `run(&mut self) -> Result<(), RuntimeError>`, and async `shutdown(&mut self) -> Result<(), RuntimeError>`. `RuntimeConfig` contains XDG paths, port, and explicit `enable_live_keyd`; startup acquires a single-process lock, runs recovery, starts API/bridge/watchdog, and observe-only by default.

- [ ] Write failing process tests for duplicate service refusal, startup mutation-marker blocking, pending eye restoration before accepting F7, focus disconnect cleanup, SIGTERM draining jobs and restoring sensitivity, crash/restart stale-eye rejection, and active Python process conflict.
- [ ] Run `cargo test --test rust_runtime`; confirm failure.
- [ ] Wire Tasks 4–11 with serialized lifecycle transitions. Keep the same extension connection/status semantics and stop on unsafe cleanup failure.
- [ ] Run `cargo test --locked`, both Node harnesses, schema validation, and a process-level observe-only smoke test with fake FIFO/D-Bus peers; all pass. Defer the real GNOME 50 probe to Task 14.

### Task 13: Guided install, repair, and uninstall

**Files:** Create `src/install.rs`, `tests/rust_install.rs`; update `packaging/mcsr-toolkit.service`, both sudoers templates, and `README.md` after tests; retire manual Python setup instructions only after cutover.

**Interfaces:** Implement `install::preflight(system: &dyn System) -> InstallReport`, `install::install(system: &dyn System, artifact: &Artifact) -> Result<(), InstallError>`, `install::repair(system: &dyn System, artifact: &Artifact) -> Result<(), InstallError>`, and `install::uninstall(system: &dyn System) -> Result<(), InstallError>`. `System` abstracts filesystem, pkexec, systemd, GNOME extension commands, and sudoers validation for deterministic tests. Install the service executable at `/usr/local/bin/mcsr-toolkit` and its verified root-owned helper copy at `/usr/local/libexec/mcsr-toolkit`.

- [ ] Write failing tests for one-file download installation, GNOME 50/x86_64 preflight, active Python service or unresolved Python records refusing before mutation, byte-identical reinstall, staged failure restoring previous user unit, root helper hash/ownership verification, exact sudo rule, and uninstall refusing to remove helpers while keyd or maccel recovery is pending.
- [ ] Run `cargo test --test rust_install`; confirm failure.
- [ ] Implement staged installation marker, authenticated system setup, user extension extraction/enablement, user service start in observe-only mode, idempotent repair, and cleanup preserving all user/recovery data. Wire `doctor` to the read-only preflight report.
- [ ] Run focused tests, `visudo -cf` on temporary generated rule fixtures without installing them, and an install/repair/uninstall test against a fake `System` rooted in a temporary directory. Do not invoke real pkexec, systemd, GNOME extension commands, keyd, or maccel in this task.

### Task 14: Release artifact, docs, and supervised acceptance

**Files:** Create `.github/workflows/rust.yml`, a release checklist under `docs/`, and `LICENSE` after the user selects license terms; update `README.md`, `integration/acceptance-wayland.md`, and extension metadata with the final unique UUID/project URL. Remove `pyproject.toml`, `src/mcsr_toolkit/*.py`, and `tests/test_*.py` only after all parity and acceptance gates pass; preserve reusable fixture files.

**Interfaces:** The release output is one Linux x86_64 executable plus published checksum. The installed user unit and helper copy derive from that exact executable; no Python interpreter is invoked at runtime.

- [ ] Add a clean-checkout CI build/test job running `cargo fmt --check`, `cargo clippy --all-targets -- -D warnings`, `cargo test --locked`, both Node harnesses, schema validation, release build, and an embedded-asset smoke test.
- [ ] After Tasks 1–13 and independent reviews pass, obtain separate approval for privileged installation and live input/sensitivity changes. In one supervised Ubuntu GNOME 50 x86_64 acceptance session, perform the first real observe-only GNOME probe, install the release artifact, UI Apply/Restore, new instance reconciliation, chat/movement, blur/modal/password/lock/logout, exact resize, eye restoration, shutdown/restart, and uninstall. Record results and any timing limitations.
- [ ] Verify normal input, keyd configuration recovery, original maccel raw value, no root recovery record, no Python process/runtime dependency, and retained user backups after uninstall.
- [ ] If a live case fails, preserve recovery evidence, fix the cause, rerun automated checks, and repeat only the affected live case and its cleanup checks. Keep the release blocked until every acceptance case passes.
- [ ] Complete the release license, UUID, URL, release notes, checksum, and tag-convention decisions with the user. Review the final staged diff and full checks. Do not create a release commit, tag, merge, push, or publish without the corresponding explicit approval.

## Execution and approval gates

Use a `feat/*` branch in a separate worktree from `dev` for implementation. Keep each task reviewable and rerun its deterministic checks after review fixes. The Python version's prior live acceptance is supporting evidence, while the Rust binary still needs one final device acceptance pass because its installer, timing, and recovery behavior are new. Privileged deployment and live keyboard/sensitivity checks require separate device authorization at Task 14. The plan itself authorizes no installation, commit, merge, push, or release. After the user approves this revised written plan, offer the three execution modes required by AGENTS.md; the user's earlier preference was Subagent-Driven.
