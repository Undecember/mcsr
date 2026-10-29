# Managed Panel Visibility Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [x]`) syntax for tracking.

**Goal:** Stop the stock GNOME top panel covering focused, toolkit-managed borderless Minecraft windows, restoring normal panel behavior whenever eligibility ends.

**Architecture:** The extension records successfully resized windows, evaluates focus/session/geometry eligibility, and owns only its temporary panel-visibility intervention. Event handlers react immediately, with the existing heartbeat as fallback. Restoration recomputes GNOME's current visibility policy rather than blindly showing the panel. No new Rust protocol or privileged operation is needed.

**Tech Stack:** GNOME Shell 50 JavaScript extension, existing actual-extension Node harness, existing Rust asset/build verification.

**Spec:** `docs/superpowers/specs/2026-10-30-managed-panel-visibility-design.md` (approved).

## Global Constraints

- Eligibility begins only after a successfully verified managed borderless resize; Minecraft class alone is insufficient.
- Hide only for the same focused, windowed, undecorated window overlapping the panel on its monitor. Cover both thin and full-size geometry.
- Restore immediately on focus/overview/modal/session changes and when window/geometry/routing ownership ends. Polling is not the sole restoration path.
- Respect current GNOME visibility policy and an already hidden baseline; do not blindly show a panel GNOME currently hides.
- Preserve focus delivery, geometry verification, F3/chat routing, and recovery behavior.
- No fullscreen transitions, window reparenting, profile fields, persistent desktop settings, keyd/helper changes, panel strut/layout changes, or replacement panels.
- Support stock GNOME Shell 50. If required hooks are unavailable, leave the panel under normal Shell control and report a diagnostic.
- Preserve all reviewed startup/toggle prerequisite source and live evidence. No commits, merges, pushes, or deployment during implementation.

## Review Focus

- A failed resize, merely matching class, or another-monitor window must never claim panel ownership (Task 1).
- Focus, overview entry, modal/lock/session transitions, and window close must restore without waiting for the heartbeat (Task 1).
- Restoration must respect another app's true fullscreen and an initially hidden panel; repeated normal Shell visibility refreshes must not cause recursion or leave coverage (Task 1).
- Routing replacement and disable/re-enable must forget old windows and remove only this feature's handlers (Task 1).
- Missing hooks, reentrant notifications, or panel actor replacement must not damage resize/focus delivery or manipulate unrelated UI (Task 1).

## Preparation

- [x] Use Graphify before exploration; read the design, plan, applicable AGENTS.md, and current working-tree diffs.
- [x] Use the worktree skill to create `.worktrees/feat-managed-panel-visibility` on `feat/managed-panel-visibility` from `dev`; verify `/.worktrees/` is ignored. Seed it with the reviewed, uncommitted startup + borderless-toggle source/test/docs outputs from `.worktrees/feat-borderless-full-size-toggle`, recording the baseline revision, exact file list, and patch/file digests. Do not assume its HEAD contains those changes. Preserve source worktrees and new plan/spec/evidence; do not copy build directories or perform history operations.
- [x] Run baseline `node tests/extension_harness.cjs` and relevant Rust GNOME/runtime/sensitivity suites before edits. Keep the installed service in observe-only mode.

### Task 1: Panel ownership, lifecycle, and regressions

**Files:** `extension/extension.js`, `tests/extension_harness.cjs`.

**Interfaces and implementation decisions:**

- Keep all D-Bus methods and replies unchanged.
- Add extension-local helpers `_initPanelVisibility()`, `_syncPanelVisibility()`, `_restorePanelVisibility()`, and `_disablePanelVisibility()`; cleanup must be idempotent. A helper for marking/clearing managed windows may use a `WeakSet` and a routing-generation boundary.
- Mark a window only in the successful verified-resize completion path, for fixed-size and toggle operations. Failed/cancelled requests cannot mark ownership. Clear marks on routing replacement/reset and disable.
- Eligibility reads actual focused window identity, managed membership, windowed/undecorated/minimized state, overview/modal/session state, and positive-area stage-coordinate intersection between window and stock `panelBox`. Require the panel's monitor to match the window's current monitor. Validate finite positive rectangles.
- Validate stock panel actor/signals and the GNOME visibility-restoration hook at initialization. Use installed GNOME 50 source/current primary documentation to verify the exact signal/property names and coordinate APIs before implementation; use Context7 first for API documentation. Known source: `js/ui/layout.js` creates `panelBox` as tracked Shell chrome and its `_updateVisibility()` recomputes fullscreen-driven actor visibility.
- Observe panel visibility changes so normal Shell refreshes cannot leave it covering an eligible window. Use a reentrancy guard for feature-initiated hide/restoration notifications. Record an owned hide only when this feature intervenes on a visible panel; an already hidden baseline alone does not grant ownership.
- On release, drop the feature override before invoking GNOME's current visibility policy. Do not unconditionally `show()` or set visible true. Cache/validate necessary stock hooks so an unavailable hook cannot initiate a hide that cannot be restored. Missing/unsupported hooks should disable this feature with a useful, bounded Shell diagnostic while preserving existing resizing.
- Reevaluate from existing focus/modal/heartbeat paths and new overview/session/panel/monitor handlers. Observe relevant geometry/decoration/fullscreen/minimization/unmanaged signals on the current managed window; disconnect old window handlers on focus/window changes. Persistent and per-window handler ownership must be explicit.
- In disable/reset paths, stop feature eligibility first, restore panel policy, disconnect handlers, and clear state. Do not change another feature's connections or global panel configuration.

- [x] Extend the harness boundary doubles to dispatch real production signal callbacks, panel visibility notifications, and normal Shell visibility recomputation. Do not reproduce the feature predicate in a fake. Include at least one actual enable → disable → enable lifecycle, not only manually initialized helpers.
- [x] Add failing cases for successful thin/full-size overlap hiding; failed/cancelled/unmanaged/decorated/fullscreen/minimized windows; invalid geometry; edge-touch with no positive overlap; another monitor; and manual movement outside the panel.
- [x] Add immediate-event cases for focus loss/regain, overview showing/hidden, modal/session/lock changes, geometry/window changes, and close. Assert visible results before advancing the heartbeat.
- [x] Add initially hidden and other-app-fullscreen restoration cases, repeated Shell visibility refreshes, reentrant notify handling, routing reset/replacement, missing hooks, panel replacement, and disable/re-enable. Assert restored policy and that stale post-disable callbacks cannot alter visibility or focus delivery.
- [x] Run `node tests/extension_harness.cjs`; observe RED because the panel remains over an eligible verified window or because the new lifecycle is missing. Confirm failures describe absent behavior, not fixture typos.
- [x] Implement the helpers and lifecycle integration above without changing resize results or producer/consumer routing. Keep it in the existing extension file so asset packaging remains unchanged.
- [x] Run `node tests/extension_harness.cjs`, `node tests/ui_harness.cjs`, and `glib-compile-schemas --strict --dry-run extension/schemas` to GREEN. Inspect handler cleanup, missing-hook diagnostics, and raw diff before independent review.

### Task 2: Documentation, combined verification, and review

**Files:** `README.md`, `integration/resize-wayland.md`, a focused record under `docs/superpowers/evidence/`, generated Graphify outputs. No unrelated product changes.

- [x] Document when the stock panel hides/restores, the managed-window/overlap requirement, and the stock GNOME 50 scope. Preserve current native-fullscreen, Alt+Tab, and temporary F-key limitations without claiming them fixed.
- [x] Add supervised checks for thin/full-size top-edge visibility, usable game input where the bar was, switching apps/overview/modal transitions, and disable/re-enable restoration. Include another-monitor/no-overlap checks when available; do not claim them from harness results.
- [x] Run `cargo fmt --check`, `cargo clippy --all-targets --locked -- -D warnings`, `cargo test --locked`, `python3 packaging/keyd/test.py`, `node tests/extension_harness.cjs`, `node tests/ui_harness.cjs`, `node --test static/app.test.cjs`, `node integration/live-ui-acceptance.mjs self-test`, and `glib-compile-schemas --strict --dry-run extension/schemas`. Require zero failures.
- [x] Run `cargo build --release --locked` and `cargo test --release --locked --test rust_assets`, then SHA256 the resulting candidate. The new extension must be embedded in the artifact.
- [x] Run `graphify update .` after code changes, `git diff --check`, and whitespace checks for new files. Verify prerequisite patch/source digests remain intact outside the intentional extension/harness/docs changes.
- [x] Obtain independent review of eligibility/ownership, current-policy restoration, event ordering, reentrancy, teardown, and actual harness fidelity. Repair material findings and rerun affected checks/review.
- [x] Save exact automated results, candidate checksum, remaining limitations, and worktree/branch locations. Leave all source uncommitted and worktrees intact. Deployment/live acceptance and merging remain separately authorized steps.

## Self-review

One coupled extension/harness task owns the visibility state machine; documentation
and final validation follow it. All five review-focus conditions have regression
coverage. Existing interfaces and prerequisite behavior stay fixed. The plan
does not assume that an actor hide alone proves live input-region or restoration
behavior; those remain explicit supervised acceptance checks.
