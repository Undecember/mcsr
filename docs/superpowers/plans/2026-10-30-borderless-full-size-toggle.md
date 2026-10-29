# Borderless Full-Size Toggle Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [x]`) syntax for tracking.

**Goal:** Toggle normal resize presets between their size and a monitor-sized borderless window, keeping Minecraft and enabled local Standard Settings windowed.

**Architecture:** The extension selects the target from observed geometry and verifies it through the existing asynchronous resize lifecycle. A typed toggle method lets Rust accept verified full-size output without relaxing fixed-size eye checks. Windowed file edits join existing Preview/Apply/Restore transactions, separately from keyboard control state and historical backup validation.

**Tech Stack:** Rust, GNOME Shell 50 extension JavaScript, local Minecraft/Standard Settings files, existing Rust/Node test harnesses.

**Spec:** `docs/superpowers/specs/2026-10-30-borderless-full-size-toggle-design.md` (approved).

## Global Constraints

- Both states remain Minecraft windowed mode; never call compositor fullscreen mutators or synthesize F11.
- Full-size dimensions come from the focused window's current monitor in Mutter's logical coordinates.
- Keep exact frame/content size and position verification. Report constraints; do not alter desktop panels/global settings.
- Direction follows observed geometry, not remembered keypresses or active-preset status.
- Eye zoom and shifted eye retain fixed-size semantics; normal transitions restore baseline sensitivity.
- Preserve F3/chat routing, focus/modal gating, generation checks, serialization, cancellation, and keyd recovery.
- Do not edit running-game files; preserve unrelated settings, disabled entries, missing configs, shared-config refusals, and old backups.
- Do not repurpose F11, automatically resize at launch, or fix default-layout/reinstall-bootstrap issues here.
- Preserve the reviewed startup fixes and evidence. No commits, merges, pushes, or installed-system changes during implementation.

## Review Focus

- Stale focus, changed monitor, replaced window, or reconfiguration during a toggle cannot publish success for an old target (Tasks 1–2).
- Manual resize/move and Alt+Tab must not leave a stale toggle direction (Task 1).
- Missing toggle support or malformed replies fail without legacy fallback or eye sensitivity changes (Task 2).
- Fullscreen booleans never enter keyboard binding maps or retroactively invalidate old backups (Task 3).
- Config-only fullscreen edits appear in Preview, defer while running, and restore exactly (Task 3).

## Preparation

- [x] Read the approved design, this plan, live evidence, and applicable AGENTS.md. Use Graphify before code exploration.
- [x] Use the worktree skill to prepare `.worktrees/feat-borderless-full-size-toggle` on `feat/borderless-full-size-toggle` from `dev`, verifying `/.worktrees/` is ignored. Its prerequisite is the uncommitted reviewed startup fix: materialize only those reviewed source/test/docs outputs from `.worktrees/fix-keyd-live-startup` as a separately recorded baseline patch. Do not commit, merge, copy build directories, or discard source worktrees. Preserve the new plan/spec/evidence too. Seed any task worktrees from this same recorded baseline.
- [x] Record the dependency patch digest and checkout status. Baseline: `cargo test --locked --test rust_gnome --test rust_sensitivity --test rust_runtime --test rust_apply --test rust_standard_settings` and `node tests/extension_harness.cjs`; distinguish pre-existing failures.

## Shared Interfaces and Dependencies

Tasks 1 and 2 share this exact contract; neither changes it independently:

- Keep fixed-size `ResizeFocused(u width, u height) -> (u actual_width, u actual_height)` unchanged.
- Add `ToggleResizeFocused(u width, u height) -> (u actual_width, u actual_height, b full_size)` on `org.mcsr.Toolkit`.
- The extension verifies the selected target before replying. False requires dimensions equal the requested preset; true requires verified monitor geometry. Rust validates returned dimensions in `100..=16384` and rejects mismatched preset replies.
- Add `GnomePeer::toggle_resize_focused(width: u32, height: u32)` returning a future of `Result<(u32, u32, bool), GnomeError>`. Default returns unsupported, never fixed-size emulation. `SessionPeer` makes the typed call.
- Add `full_size: bool` to `ResizeResult`. Normal presets use toggle; eye/shift-eye retain fixed-size resize and return false.
- Use one focus-generation-bound record for the latest successful preset ID and full-size mode. Derive `active_preset` (null for full-size) and `window_mode` (`preset`, `full_size`, or null) from that record; retain existing invalidation boundaries.

Tasks 1, 2, and 3 are independently implementable against these interfaces. Task 4 requires all three verified outputs.

### Task 1: Extension geometry toggle

**Files:** `extension/extension.js`, `tests/extension_harness.cjs`.

- [x] Add failing tests for `ToggleResizeFocusedAsync` and `(uub)` replies: thin → full → thin, decorated/already-borderless clients, manual movement/resizing, Alt+Tab without remembered state, new game objects, multiple monitors/nonzero or negative origins, logical-coordinate scaling, and constrained output.
- [x] Cover initial fullscreen refusal before mutation; fullscreen/focus/monitor/window change while pending; concurrent requests; invalid monitor data; exact content/frame/position mismatch; subprocess failure; timeout; and disable. Retain fixed-size method coverage.
- [x] Run `node tests/extension_harness.cjs` to observe RED for missing toggle behavior.
- [x] Extend XML and reuse preparation/observation/cleanup. Select the monitor at request start. Exact undecorated frame/content preset dimensions and centered position select full monitor bounds; otherwise select preset bounds. Revalidate the same window/monitor during asynchronous work. Store reply mode on the request so legacy replies remain `(uu)` and toggles use `(uub)`.
- [x] Rerun the extension harness to GREEN. Verify no fullscreen mutators, F11 synthesis, or global desktop-setting changes were added.

### Task 2: Typed Rust toggle, status, and sensitivity

**Files:** `src/gnome.rs`, `src/runtime.rs`, necessary peer adapters and tests in `tests/rust_gnome.rs`, `tests/rust_sensitivity.rs`, `tests/rust_runtime.rs`, `tests/rust_runtime_process.rs`.

- [x] Add failing bridge tests for typed toggle results, unsupported method, mismatched preset dimensions, invalid full-size dimensions, stale/non-Minecraft/modal focus, and timeout. Preserve strict fixed-size mismatch rejection.
- [x] Add controller tests proving normal presets call toggle and restore baseline sensitivity in both directions; eye/shift-eye call fixed-size resize and retain current behavior. Errors must not apply eye sensitivity.
- [x] Test preset → full-size → preset status and rejection of stale publication after blur/quick refocus or profile replacement. Both status fields must derive from the same generation-bound record.
- [x] Run targeted tests to RED; implement the Shared Interfaces, typed session method, validation, controller branch, and status publication. Missing old-extension support must produce an actionable upgrade error without fallback. Update only necessary fake peers.
- [x] Run `cargo test --locked --test rust_gnome --test rust_sensitivity --test rust_runtime --test rust_runtime_process` and affected unit tests to GREEN.

### Task 3: Transactional windowed settings

**Files:** `src/options.rs`, `src/standard_settings.rs`, `src/apply.rs`; `src/api.rs` only if preview details need plumbing. Tests: options unit tests, `tests/rust_standard_settings.rs`, `tests/rust_apply.rs`, `tests/rust_api.rs`.

**Interfaces:** Add `render_windowed_options(bytes: &[u8]) -> Result<Vec<u8>, OptionsError>`. Do not put fullscreen into `OptionsDocument.bindings`, `ControlPlan`, or `ManagedState`. Keep `prepare_local(game_dir, bindings)` as the legacy control-only wrapper; add `prepare_local_with_windowed(game_dir: &Path, bindings: &IndexMap<String, String>, windowed: bool) -> Result<Vec<ConfigEdit>, StandardSettingsError>`.

Windowed policy is active only when `!profile.presets.is_empty()`. Profiles with no presets retain control-only behavior. Removing presets does not invent a fullscreen preference; explicit Restore reverses managed changes.

- [x] Add failing options tests: true → false, false unchanged, missing fullscreen appends false, duplicate/malformed fullscreen refusal, preserved LF/CRLF/CR and absent final newline. Test that ordinary control rendering cannot duplicate fullscreen.
- [x] Add failing modern Standard Settings tests for scalar boolean and enabled object boolean (`{enabled:true,value:true}`) edits; disabled/missing entries stay unchanged. Reject malformed active fullscreen types without newly validating unrelated unsupported entries. For legacy text, change nonblank `fullscreen:true`, preserve blank/missing, reject duplicate/malformed active entries. Retain unsafe/shared-path and missing-file protections.
- [x] Add Apply tests where only fullscreen changes: Preview ready with human-readable per-instance details, running instance deferred, exact options/mod backup and Restore, atomic refusal across instances, drift refusal, and idempotent reconciliation. Do not invent physical-key mappings for this boolean setting.
- [x] Add older backup fixtures with presets and fullscreen true that still restore. Keep prior-profile Restore validation control-only: never replay the new policy onto historical backup contents or weaken hash/lineage validation.
- [x] Observe targeted RED, then implement the narrow render helper and policy plumbing through `prepare` and `validate_prepared`, retaining legacy wrappers.
- [x] Run affected unit tests and `cargo test --locked --test rust_standard_settings --test rust_apply --test rust_api` to GREEN.

### Task 4: Combined verification, guidance, and review

**Files:** `static/index.html`, `static/app.js` where status/help needs it; existing UI tests; `README.md`, `integration/resize-wayland.md`, live evidence, generated Graphify outputs.

- [x] Assemble reviewed outputs with authorized local operations. Resolve interfaces semantically, preserve the startup prerequisite, and inspect all tracked/new files. No history operations or installation.
- [x] Explain normal-preset toggling, monitor sizing, windowed preparation, unchanged eye behavior, and closing/restarting a currently fullscreen game. Show `Full-size borderless` for current full-size status without displaying a thin preset as active. Add meaningful UI fixture coverage.
- [x] Run `cargo fmt --check`, `cargo clippy --all-targets --locked -- -D warnings`, `cargo test --locked`, `python3 packaging/keyd/test.py`, `node tests/extension_harness.cjs`, `node tests/ui_harness.cjs`, `node integration/live-ui-acceptance.mjs self-test`, and `glib-compile-schemas --strict --dry-run extension/schemas`. Require zero failures.
- [x] Run `cargo build --release --locked`, `cargo test --release --locked --test rust_assets`, and SHA256 the artifact. Run `graphify update .`, `git diff --check`, and check new files too.
- [x] Obtain independent review of geometry/cancellation, bridge compatibility, configuration backup/Restore, old backups, and sensitivity cleanup. Repair material findings and rerun affected verification/review.
- [x] Record precise automated evidence, retaining native fullscreen/default-layout failures as unresolved. Present the local candidate and worktree location without claiming live acceptance. Subsequent supervised deployment must verify repeated B toggles, Alt+Tab, eye behavior when used, and F3/chat priority.

## Self-review

The geometry decision, monitor handling, eye exception, configuration policy,
and compatibility boundaries have explicit owners and tests. Shared protocol
types are defined once. Task 3 stays outside keyboard maps and preserves old
backup replay. All five review-focus conditions have regression owners. Native
F11 repair, implicit deployment, and unrelated environment fixes remain excluded.


## Parallel execution ledger

Design and plan explicitly approved; Parallel SDD selected. Baseline is dev `5ae4c41` plus the recorded reviewed startup prerequisite patch in `docs/superpowers/evidence/2026-10-30-borderless-toggle-baseline.json`. Task 1/2/3 share no writable files. Task 4 depends on their reviewed outputs.

| Task | Dependencies | Wave | Ownership | Worktree / branch | State | Verification / review |
| --- | --- | --- | --- | --- | --- | --- |
| 1 Extension | startup baseline | 1 | extension + extension harness | `.worktrees/feat-borderless-toggle-extension` / `feat/borderless-toggle-extension` | complete | RED/GREEN harness; schema; independent PASS |
| 2 Rust toggle/status | startup baseline | 1 | gnome/runtime + their Rust tests | `.worktrees/feat-borderless-toggle-runtime` / `feat/borderless-toggle-runtime` | complete | targeted/library suites + private wire probes pass; independent PASS |
| 3 Windowed files | startup baseline | 1 | options/settings/apply + corresponding tests | `.worktrees/feat-borderless-toggle-settings` / `feat/borderless-toggle-settings` | complete | 71 integration + 2 unit independently passed; review PASS |
| 4 Integration | 1, 2, 3 | 2 | UI/docs/generated graph + combined output | `.worktrees/feat-borderless-full-size-toggle` / `feat/borderless-full-size-toggle` | complete | UI RED/GREEN, all combined checks and release assets passed; independent integration PASS |

Baseline checks: original extension harness passed in integration checkout; settings baseline 61 tests, Rust bridge/runtime baseline 56 tests passed in their actual task worktrees before edits. Task 1 independently reviewed PASS, including official Mutter 50 content-coordinate verification and extra cancellation probes.

Task 3 independent PASS: original wrappers/Restore validation retained, full hash/lineage protection and historical two-generation restore tested. Root independently reran settings/API/Apply (71) and options units (2). Task 2 root rerun passed 123 reported tests (includes child-process test result). Independent review of locks and typed wire compatibility is in progress.

Wave 1 barrier complete: all three tasks independently PASS. Wave 2 assembled exact reviewed bytes in dependency checkout; task patch digests recorded separately. Reviewer also verified actual SessionPeer on private D-Bus for valid, missing-method, and malformed-signature replies.

Combined verification: full `cargo test --locked` passed (405 reported, 23 suites), fmt/Clippy passed, managed-keyd 94+28/held-reset/IPC tests passed, extension/UI harnesses and 14 JavaScript tests passed, live-UI fixture and schema dry-run passed. Graphify AST refresh completed (2643 nodes, 6503 edges). Independent final integration review PASS. Release build/assets pending.

Final: Clippy, release build, six release asset tests, and independent integration review passed. Candidate SHA256 `df034359d2ec42f7ab261b2fab97ed7451184210655070b334936c04f32808c0`. Full validation and coverage boundaries: `docs/superpowers/evidence/2026-10-30-borderless-toggle-validation.md`. All worktrees retained. No commit, merge, push, or installation. Supervised desktop acceptance remains pending.
