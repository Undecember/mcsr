# Live Keyd Startup Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [x]`) syntax for tracking.

**Goal:** Start live mode from empty keyd configuration and wait safely for daemon IPC after restart.

**Architecture:** Fix empty-config reset in the maintained keyd IPC patch. Separate bounded daemon readiness from plain systemd activity and feed it into the existing helper health loop. Preserve failure cleanup and capability verification.

**Tech Stack:** Rust, maintained C keyd patch, Python device-free test runner.

**Spec:** `docs/superpowers/specs/2026-10-29-keyd-live-startup-design.md` (approved).

## Global Constraints

- Preserve the five-second initial readiness deadline, 500 ms stability window, 200 ms maximum sampling gap, and stop-on-failure behavior.
- Include probe execution time in deadline accounting and preserve useful failure diagnostics.
- Keep pristine vendored source and its checksum manifest unchanged; maintain the daemon change in the existing patch.
- No root, real keyboards, or desktop input injection in regression tests.
- Do not change F3 routing, focus/chat rules, authentication scope, backup lineage, or capability requirements.
- The uninstall/reinstall helper bootstrap gap is a separate issue.
- Preserve the primary checkout's untracked evidence and cache.
- Implementation belongs on an isolated `fix/*` worktree based on `dev`.
- No commit, merge, push, or system installation is part of code implementation.
- The installed service remains in healthy observe-only mode during development.

## Review Focus

- Empty-config success must apply only to exact `reset`, including rejection of malformed/trailing expressions (Task 1).
- A populated reset must release held outputs and retain normal validation (Task 1).
- Active systemd state with unavailable or incompatible IPC must never count as ready (Task 2).
- Slow probes must consume the readiness budget; readiness arriving after the deadline must fail (Task 2).
- Capability loss during stability and failure to stop must retain unsafe-state diagnostics (Task 2).

## Preparation

- [x] Use `superpowers:using-git-worktrees` to create `.worktrees/fix-keyd-live-startup` on `fix/keyd-live-startup` from `dev`. Verify `/.worktrees/` is ignored; if needed use the repository-local exclude file, following AGENTS.md.
- [x] Copy the approved design, this plan, and the untracked live evidence into the worktree without moving or changing the primary copies. Read applicable AGENTS.md instructions there. Use Graphify before new code exploration.
- [x] Run baseline targeted checks: `cargo test --locked --test rust_keyd_helper --test rust_runtime` and `python3 packaging/keyd/test.py`. Record pre-existing failures separately.

### Task 1: Empty-config reset through the real IPC dispatcher

**Files:**
- Modify: `packaging/keyd/mcsr.patch`, `packaging/keyd/test.py`.
- Create: `packaging/keyd/tests/ipc-reset.c`.
- Reference: `vendor/keyd/src/daemon.c`, `packaging/keyd/tests/reset.c`.

**Interfaces:** Exercise patched `handle_client(int con)` with an AF_UNIX socketpair in the harness; do not start the daemon or open device nodes. Keep all dispatcher branches present. The existing engine harness remains responsible for physical event cleanup assertions.

- [x] Add a device-free IPC harness that includes the staged patched daemon dispatcher, linking the required C code and stubbing only hardware-facing operations not exercised by the test. Register its compile/run in `packaging/keyd/test.py`.
- [x] Assert empty configs + IPC_BIND `reset` returns IPC_SUCCESS. Assert ordinary binds, invalid expressions, `reset `, and `resetx` return IPC_FAIL with empty configs. With a real parsed config/keyboard attached, assert reset succeeds and an invalid expression fails. Retain the existing held-F3/B reset cleanup test.
- [x] Run `python3 packaging/keyd/test.py`; observe failure specifically at empty-config reset before changing the patch.
- [x] Extend the daemon IPC_BIND hunk in `mcsr.patch` to acknowledge exact reset only when no configs exist. Preserve evaluation for populated configs and all other rejection behavior. Do not edit vendor files or their hashes.
- [x] Rerun `python3 packaging/keyd/test.py`; require all upstream, toolkit, held-key reset, and new IPC cases to pass.

### Task 2: Bounded readiness after restart

**Files:**
- Modify: `src/keyd_helper.rs`, `src/keyd.rs`, `tests/rust_keyd_helper.rs`.
- Test: unit tests within `src/keyd_helper.rs`/`src/keyd.rs` for production probe behavior; integration tests in `tests/rust_keyd_helper.rs` for polling and cleanup.

**Interfaces:**
- Add `HelperSystem::readiness(&mut self, budget: Duration) -> Result<bool, String>`; update both existing test adapters. `Ok(false)` means not ready, an error supplies diagnostic detail, and neither counts as healthy.
- Keep `systemctl("is-active")` a service-state check. Production `readiness` must require it plus complete daemon capability validation, using a shared deadline for both subprocesses.
- Preserve `verify_managed_daemon(binary: &str) -> Result<(), String>` for existing callers; introduce a timeout-aware variant for readiness that takes the remaining command budget and reuses the same response validator.

- [x] Write regressions for initial IPC probe errors followed by successful readiness: clear succeeds only after 500 ms stable readiness and never calls stop. Demonstrate the pre-fix early-abort behavior with a failing test before modifying polling behavior.
- [x] Add a production probe test seam around command execution. Test active service + connection failure, active service + missing capability, inactive service, and successful complete capabilities; ensure failed probes cannot yield healthy state. Observe the relevant pre-fix failure.
- [x] Add deterministic clock/budget tests: never-ready times out and stops; a slow initial probe arriving after five seconds is rejected; every command receives at most the remaining budget; failed health after stability begins stops immediately; a gap above 200 ms fails; inability to stop retains the unsafe error. Keep existing helper rollback tests.
- [x] Implement bounded readiness and polling. Retry initial not-ready observations/errors until the deadline; retain the most recent diagnostic on timeout. Once stability starts, loss of readiness or probe error must stop and fail. Do not suppress errors in `KeydController::clear`.
- [x] Run `cargo test --locked --test rust_keyd_helper` and the affected unit tests until green. Run `cargo test --locked --test rust_keyd --test rust_runtime --test rust_apply` for cleanup, first Apply, and restart regressions.

### Task 3: Validation, maintenance notes, and review

**Files:**
- Modify: `packaging/keyd/README.md`, `docs/superpowers/evidence/2026-10-29-f3-keyd/live-validation.md`.
- Update: Graphify outputs in the implementation worktree.

- [x] Document exact empty-config reset behavior and the bounded readiness contract in the existing managed-keyd guide. Append new automated evidence to the live record, preserving observed failures and pending Minecraft checks.
- [x] Run `cargo fmt --check`, `cargo clippy --all-targets --locked -- -D warnings`, `cargo test --locked`, `python3 packaging/keyd/test.py`, `node tests/extension_harness.cjs`, `node tests/ui_harness.cjs`, `node integration/live-ui-acceptance.mjs self-test`, and `glib-compile-schemas --strict --dry-run extension/schemas`. Require zero failures.
- [x] Run `cargo build --release --locked`, `cargo test --release --locked --test rust_assets`, and SHA256 the resulting executable. Require a successful build and all release asset tests passing.
- [x] Run `graphify update .` after code changes and `git diff --check`; review the complete raw diff and status, including new files.
- [x] Obtain one independent review focused on reset scope, readiness deadline accounting, cleanup failure paths, and test fidelity. Repair material findings and rerun affected checks.
- [x] Present the reviewed local fix and test evidence. Preserve the worktree and branch; do not commit or integrate. A subsequent supervised deployment and empty-baseline live startup are required before claiming desktop acceptance or continuing Minecraft F3 checks.

## Plan self-review

The two implementation tasks cover the approved scope and all five review-focus conditions. The IPC test executes the actual dispatch branch missed by engine-only tests. Readiness tests cover production error classification and polling, including elapsed command time. The reinstall/bootstrap issue remains excluded. All Git integration and installed-system changes remain separate from implementation.


## Parallel execution ledger

Baseline: `5ae4c41` (`dev`). Design and plan approved; user selected Parallel SDD.
Dependency graph: Task 1 → Task 3; Task 2 → Task 3. Task 1 and Task 2 own disjoint files.

| Task | Dependencies | Wave | Owned files | Worktree / branch | State | Verification / review |
| --- | --- | --- | --- | --- | --- | --- |
| 1: Empty reset IPC | none | 1 | packaging/keyd patch, runner, IPC fixture | `.worktrees/fix-keyd-empty-reset` / `fix/keyd-empty-reset` | complete | baseline + RED/GREEN IPC/engine tests; independent PASS |
| 2: Bounded readiness | none | 1 | keyd helper/controller, Rust helper/runtime tests | `.worktrees/fix-keyd-readiness` / `fix/keyd-readiness` | complete | helper 24 + keyd units 12 passed; independent PASS |
| 3: Integration and validation | 1, 2 | 2 | docs, combined output, Graphify | `.worktrees/fix-keyd-live-startup` / `fix/keyd-live-startup` | complete | full combined checks and release assets passed; integration review PASS |

Outputs will be transferred as reviewed patches and new files without commits or merges.

Baseline evidence: untouched integration checkout passed `cargo test --locked --test rust_keyd_helper --test rust_runtime` (40 tests); Task 1 baseline engine suite passed before IPC regression addition. Task 1 independent review returned PASS.

Wave 1 complete: both isolated outputs independently PASS. Wave 2 materialized both reviewed tracked diffs and the new IPC fixture without commits/merges; source worktrees retained.

Combined checks: full `cargo test --locked`, `cargo fmt --check`, engine/IPC suite (94 upstream + 28 toolkit + held-key cleanup + IPC), extension/UI harnesses, live-UI fixture, schema dry-run, and AST-only Graphify refresh passed. Clippy and release checks pending.

Final: full combined verification, Clippy, release build and six release asset tests passed. Independent integration review PASS. Candidate SHA256 `765c78b46bdd2c1fe34ed120f38d3b531f79ee7157cb1f0f6981e9cf189268a0`. Worktrees retained, no commits/merges/pushes/system installation; desktop acceptance pending. Detailed evidence is appended to the live validation record.
