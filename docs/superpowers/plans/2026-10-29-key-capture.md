# Binding input capture implementation plan

> **For agentic workers:** Use superpowers:executing-plans for inline execution.

**Goal:** Record keyboard and supported mouse bindings with a five-second timeout in every binding field.

**Architecture:** Share a browser recorder and physical-code translator across existing inputs, preserving their values and profile serialization. Extend the backend key vocabulary and Minecraft mouse translation; constrain destinations to supported runtime inputs.

**Tech Stack:** Existing plain JavaScript/CSS, Node test runner and DOM harness, Rust.

**Spec:** docs/superpowers/specs/2026-10-29-key-capture-design.md (approved).

## Global constraints

Five-second timeout, no Cancel button; manual entry retained. Minecraft actions and other non-binding inputs remain unchanged. Unsupported events never overwrite values. Preserve left/right modifiers. Suppress initiating and follow-up gestures. No new production dependencies.

## Review focus

Initiating clicks and keyboard activation must not record themselves. Mouse clicks must not remove rows or navigate. Held modifiers and released modifiers must serialize correctly. Blur, detached inputs and profile replacement must stop recording. GNOME keyboard accelerators must not accept mouse buttons.

### Task 1: Supported runtime names and Minecraft mouse mappings

- [x] Add Rust regression tests for supported names, mouse control encode/decode/remap/restore and invalid shortcut destinations; run them and observe failures.
- [x] Implement minimal profile validation and mouse translation, respecting runtime destination constraints.
- [x] Run cargo test, expecting no failures.

### Task 2: Shared recorder and field integration

Files: static/app.js, static/app.test.cjs, static/index.html, static/style.css, tests/ui_harness.cjs.

Interface: keyboardKey(code) returns backend name or null; createInputRecorder(root, view, clock) exposes start(options) and stop(); options specify input, status, shortcut, allowMouse and onChange.

- [x] Add Node tests using real EventTarget with a controlled clock for key translation, timeout, focus loss, repeats, unsupported events, modifiers, mouse buttons and follow-up gesture suppression. Run to observe failures.
- [x] Implement physical-code mapping and recorder, then run Node tests.
- [x] Extend DOM harness for capture controls on all binding fields, action-picker exclusions, serialization, invalidation and detached-input cancellation. Run to observe failures.
- [x] Integrate recorder buttons/status into mapping, chat and preset fields; add concise help and responsive styles; run frontend tests and DOM harness.

### Task 3: Verification and review

- [x] Run cargo test, node --test static/app.test.cjs, node tests/ui_harness.cjs, and node tests/extension_harness.cjs.
- [x] Use Playwright for real-browser capture, mouse follow-up, keyboard navigation and responsive layout checks.
- [x] Review diff, resolve material issues, run graphify update ., and report evidence and remaining browser limitations.

## Execution notes

Implemented and reviewed inline with a focused backend worker and an independent reviewer. The reviewer found a rejected mouse gesture could click Remove after timeout; a failing regression test and the real-browser reproduction now pass after retaining suppression through release.

Runtime inspection found managed keyd matches keyboards only. Mouse recording is enabled for native Minecraft controls and emitted bindings, while mouse-triggered crafting/Ninjabrain/chat inputs require explicit device enrollment. Those source fields record keyboard inputs and explain the restriction; device enrollment is outside this change.

Validation: full Rust suite passed with `cargo test -- --test-threads=1`; 14 Node tests, DOM harness, extension harness, Chromium capture checks, formatting and diff checks passed. Ordinary parallel Rust runs exposed intermittent existing fixture failures (`authenticated_status_command_has_a_deadline`: executable text file busy; `native_restore_publishes_prior_profile_before_marker_removal`: job polling timeout); both passed in the serialized full suite. Graphify was updated.
