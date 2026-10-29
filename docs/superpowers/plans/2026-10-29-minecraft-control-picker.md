# Minecraft Control Picker Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [x]`) syntax for tracking.

**Goal:** Select Minecraft controls by default key and action without typing internal IDs.

**Architecture:** Keep the current profile API and embedded asset layout. Add a searchable native select to control rows, with an explicit custom-ID field for unknown controls. Search filters options without changing the chosen control.

**Tech Stack:** Vanilla JavaScript, HTML, CSS, Node test runner, existing Rust suite.

**Spec:** `docs/superpowers/specs/2026-10-29-minecraft-control-picker-design.md`

## Global Constraints

- Use Minecraft Java 1.16 defaults.
- Keep the existing physical key field and profile/API format.
- Preserve unknown IDs when loading and saving profiles.
- Rebuild Graphify after implementation.

## Review Focus

- Filtering must not replace an existing selection or invalidate its saved preview.
- Unknown and custom IDs must survive a save and reload.
- Both empty selections and duplicate actions must retain validation.
- Keyboard and mouse defaults and unbound actions must be selectable.
- A selection change must invalidate the preview even when no text input changes.

### Task 1: Searchable picker and profile integration

**Files:** Modify `static/app.js`, `static/index.html`, `static/style.css`, `tests/ui_harness.cjs`, `static/app.test.cjs`.

**Interfaces:** Existing `profileFromRows(rows)` still consumes `[controlId, physicalKey]` pairs. Add `findMinecraftControls(query)` returning objects `{id, key, action}` matching default key or action, for the UI and focused search tests.

- [x] Add failing tests for default key/action filtering and the actual UI save payload, including unknown IDs, custom selection, incomplete/duplicate rows, and preview invalidation.
- [x] Run `node --test static/app.test.cjs` and `node tests/ui_harness.cjs`; confirm new behavior fails before implementation.
- [x] Add the Java 1.16 catalog, filtering, native select, custom-ID fallback, explicit control-row serialization, labels, and matching styles. Keep a selected option available while filtering.
- [x] Run both Node commands; expect all tests to pass. Verify keyboard selection, filtering without losing selection, and responsive layout in a real browser if available.
- [x] Run `cargo test`, update the graph with `graphify update .`, and inspect `git diff --check` and the final diff.

## Execution notes

Work inline in the supplied checkout on `dev`; preserve existing unrelated configuration and graph changes. No merge, push, or publication is requested. The user approved implementation; proceed without another plan approval round.

## Verification results

- Task 1 complete: new search and picker tests failed before implementation and passed afterward.
- Passed `node --test static/app.test.cjs`, `node tests/ui_harness.cjs`, `node tests/extension_harness.cjs`, and `node integration/live-ui-acceptance.mjs self-test`.
- Passed real Chrome checks for native keyboard selection, filtering without losing selection or preview, save/reload, custom IDs, adding rows, and mobile layout; no page errors.
- Verified all catalog IDs/default keys against the official Mojang Minecraft 1.16.1 client constructor and mappings.
- Passed `cargo test --locked --no-fail-fast -- --test-threads=1`, `cargo fmt --check`, and `git diff --check`.
- Earlier Rust runs encountered API job polling timeouts. The same timeout reproduced in a pristine HEAD checkout; the final serial suite passed without backend modifications.
- Independent reviewer found no actionable issues. Graph updated with `graphify update .`.
- Integrated directly into `dev`, where implementation took place; no separate feature branch was created.

### Integration recheck

JavaScript tests, UI/extension harnesses, acceptance self-test, formatting, and diff checks passed again. The full serial Rust recheck passed every target except `rust_api`, where `restore_holds_profile_lock_until_job_is_reserved` intermittently observed HTTP 200 instead of 409. This exact failure also reproduced on a pristine HEAD checkout, as did the previously observed API job timeout. These pre-existing backend races remain unchanged by the picker work.
