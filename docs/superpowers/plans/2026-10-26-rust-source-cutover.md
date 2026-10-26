# Rust Source Cutover Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Remove the retained Python implementation and make Rust the repository's sole supported product and contributor workflow without touching installed recovery state.

**Architecture:** Delete the Python package, tests, packaging metadata, and obsolete source-side policy templates. Replace the one Python-dependent Rust test fixture with a native child process, then rewrite active setup and developer documentation for the embedded Rust executable. Keep historical acceptance evidence and migration safety checks while release acceptance remains open.

**Tech Stack:** Rust, Cargo, Node.js, GNOME Shell 50 GJS assets, Markdown.

**Spec:** [Rust source cutover design](../specs/2026-10-26-rust-source-cutover-design.md).

## Global Constraints

- Source-only change: do not invoke live install/uninstall, keyd, maccel, GNOME extension changes, or mutate user/runtime/root recovery files.
- Keep the Rust installer checks for active Python-style services and unresolved Python-era recovery; keep user profiles, backups, and historical evidence.
- Keep `static/`, `extension/`, shared fixtures, Rust policy templates, and their exact paths.
- Logout/login under live remapping, guided uninstall, and final release artifact checks remain release gates; do not present source cutover as release sign-off.
- Do not commit, merge, push, tag, or publish without separate user authorization. The prior Subagent-Driven preference applies after plan approval.

## Review Focus

- A running legacy service holding `mcsr-toolkit-service.lock` still blocks Rust startup when Python is unavailable: Task 1 native child test.
- Removal of Python files does not remove any input or recovery guard used by Rust: Task 2 source audit and full Rust suite.
- A new user can follow README install/repair/uninstall commands without a Python package, missing helper source, or invalid local link: Task 3 command/link audit.
- Historical Wayland findings remain identifiable as Python-era evidence while active resize instructions use current Rust paths: Task 3 document audit.
- Release notes and checklist distinguish source completion from pending live logout/uninstall and exact artifact verification: Task 3 status assertions and final review.

---

### Task 1: Native legacy-lock test fixture

**Files:** Modify `tests/rust_runtime_process.rs`.

**Interfaces:** Preserve `active_python_style_service_lock_blocks_rust_start`; add a gated child test in the same integration-test executable that opens the supplied lock file, takes `flock(LOCK_EX)`, prints `LOCKED`, and waits until terminated. Retain the original assertion that Rust startup refuses a competing lock without invoking maccel.

- [ ] Change the parent test to spawn its child with `PATH=/nonexistent` while it still calls `python3`; run `cargo test --locked --test rust_runtime_process active_python_style_service_lock_blocks_rust_start -- --exact` and record the expected missing-executable failure (RED).
- [ ] Replace `Command::new("python3")` with `std::env::current_exe()` and an exact test-name argument; implement the gated Rust child using the same lock path and Unix `flock`. Keep bounded child cleanup even when an assertion fails.
- [ ] Run the focused parent/child tests and `cargo fmt --check`; require GREEN. Search active Rust tests/build files for interpreter invocation; migration strings and historical evidence are allowed.
- [ ] Obtain an independent task review of lock semantics, child lifecycle, and no-Python execution.

### Task 2: Remove Python product and test tree

**Files:** Delete `pyproject.toml`, tracked `src/mcsr_toolkit/*.py`, tracked `tests/test_*.py`, `packaging/mcsr-toolkit-keyd.sudoers`, and `packaging/mcsr-toolkit-maccel.sudoers`. Modify `.github/workflows/rust.yml` only to add `node integration/live-ui-acceptance.mjs self-test` to the existing JavaScript gate.

**Interfaces:** Rust executable, embedded assets, `packaging/mcsr-toolkit-rust-*.sudoers`, and migration refusal behavior are unchanged.

- [ ] Record the exact tracked deletion list with `git ls-files` and confirm each target is Python-only or the obsolete Python helper policy. Retain `tests/fixtures/` and historical Markdown.
- [ ] Delete only those tracked files; add the browser driver self-test to CI. Run `git diff --check`, `cargo test --locked`, both Node harnesses, the browser self-test, and strict schema validation.
- [ ] Verify the named Python source/test/metadata files and legacy templates are absent on disk and appear only as intended deletions in `git diff --name-status`; check `rg 'python3|src/mcsr_toolkit|pyproject.toml' Cargo.toml build.rs src tests .github` and classify any surviving migration strings.
- [ ] Obtain an independent task review of deletions, retained Rust guards, and CI behavior.

### Task 3: Rust-only active documentation

**Files:** Rewrite `README.md`; update `integration/resize-wayland.md`, `docs/rust-release-checklist.md`, and `docs/rust-release-notes.md`. Keep `integration/acceptance-wayland.md` and prior plans/specs as historical records unless a link repair is required.

**Interfaces:** Public commands use `mcsr-toolkit doctor|install|repair|uninstall` and `/usr/local/bin/mcsr-toolkit serve`; root helper and Rust policy paths remain exactly those in `src/install.rs` and `src/main.rs`.

- [ ] Replace the Python setup, virtual environment, old helper deployment, Python unit override, and Python developer-check sections of README with the actual Rust artifact verification, guided install/repair/uninstall, UI, recovery, and Rust/Node/schema checks. Keep the observe-only default and explicit live-keyd gate clear.
- [ ] Make `integration/resize-wayland.md` a current Rust/extension observation guide while preserving dated measurements as historical evidence. Remove runnable Python smoke/deployment commands and local links to removed Python source/templates; use the new extension UUID.
- [ ] Mark source cutover complete but live logout/uninstall and release artifact validation pending in the checklist and draft release notes. Preserve Python-era data-retention and migration safeguards; update the old deletion gate to record that the user chose source removal before live gates.
- [ ] Verify every local Markdown link in active docs resolves, every displayed command names existing executable modes/files, and `rg` finds no active instruction to install Python or deploy old helper sources. Run Node harnesses and `git diff --check`; obtain independent docs review.

### Task 4: Final deterministic and release-boundary verification

**Files:** No further implementation file is expected; correct only findings from verification/review.

- [ ] Run `cargo fmt --check`, `cargo clippy --all-targets --locked -- -D warnings`, `cargo test --locked`, `cargo build --release --locked`, `cargo test --release --locked --test rust_assets`, both Node harnesses, `node integration/live-ui-acceptance.mjs self-test`, and `glib-compile-schemas --strict --dry-run extension/schemas` from the cutover worktree.
- [ ] Audit tracked files and active documentation for Python product dependencies and broken links. Use `ldd` and `readelf -d` on the locally built executable to confirm no Python runtime library; this is a local candidate check, not final published-artifact provenance.
- [ ] Obtain independent whole-change review of behavior preservation, source deletion, documentation, and pending release gates. Confirm the worktree status and staged diff before reporting. Do not run desktop acceptance, remove installed legacy helpers/data, or publish.

## Handoff

The user chose to remove Python source before the still-pending live logout and uninstall tests. Their earlier choice was Subagent-Driven development. After approval of this plan, offer the execution modes required by `AGENTS.md`; selecting a mode authorizes only local implementation mechanics and does not authorize commits or system changes.
