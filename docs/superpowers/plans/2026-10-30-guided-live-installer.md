# Guided Live Installer Implementation Plan

> **For agentic workers:** Use superpowers:executing-plans inline with TDD.

**Goal:** Make installation resumable and live by default with actionable failures.
**Architecture:** A prerequisite adapter handles host checks/offers; existing
installer transactions retain ownership, rollback and marker guarantees.
**Tech Stack:** Rust, systemd, GNOME settings, existing command runner.
**Spec:** docs/superpowers/specs/2026-10-30-guided-live-installer-design.md

## Global Constraints

Doctor is read-only. Never remove recovery evidence to bypass a refusal.
Preserve input sources and unrelated XKB options. No profile application.

## Review Focus

- Masked/unavailable services must not be silently unmasked.
- Missing account group and stale manager groups require distinct handling.
- Declined/noninteractive offers must remain actionable and resumable.
- Failed live startup and failed fallback must preserve rollback guarantees.
- Existing settings backups must never be overwritten on repeated repair.

### Task 1: Authenticated helper diagnostics

Files: src/install.rs (RealRootSetup and regression tests).
- [x] Write real subprocess EOF/stderr regressions; run to see failure.
- [x] Capture stderr without pipe deadlock and include bounded diagnostics.
- [x] Run focused tests.

### Task 2: Guided prerequisites and read-only live checks

Files: src/install_prerequisites.rs, src/lib.rs, src/main.rs.
Produces: InstallOptions, prerequisite report, preparation pending reason.
- [x] Write tests for package/mask, group/session, consent, settings backup.
- [x] Run failing tests; implement offers and checks with injected command runner.
- [x] Expose --observe-only and --yes; doctor reports live readiness separately.
- [x] Run focused tests.

### Task 3: Live-default transaction and fallback

Files: src/install.rs, packaging/mcsr-toolkit.service, tests/rust_install.rs,
README.md and release notes.
Consumes: prerequisite preparation pending reason and InstallOptions.
- [x] Write regressions for default live command, pending session, live-only
  fallback, failed fallback rollback, idempotent repair, command mismatch.
- [x] Run failing tests; implement mode-aware command verification and readiness.
- [x] Keep marker on verified fallback; preserve original rollback on failure.
- [x] Document setup offers, readiness, fallback, backup and restoration.
- [x] Run cargo test --locked; cargo fmt --check; cargo clippy --all-targets
  --locked -- -D warnings; node tests/extension_harness.cjs; graphify update .
- [x] Independent final review; fix material findings with regression tests.


## Verification record

- `cargo test --locked`: 443 passed, 0 failed across 24 suites.
- `cargo fmt --check` and `cargo clippy --all-targets --locked -- -D warnings`: passed.
- Extension/UI harnesses and live-UI fixture self-test: passed.
- Managed keyd tests: 94 upstream + 28 toolkit event cases, held-key cleanup,
  and IPC reset passed.
- Strict schema compilation dry-run, release build, and six release asset tests: passed.
- Independent review identified delayed first-heartbeat handling and exhaustion
  of temporary input-source candidates. Both received failing regression tests
  and verified fixes.
- Graphify AST update completed. No desktop settings, installed toolkit binary,
  or running service were changed by this source implementation.

Live GNOME/apt/authentication installation acceptance remains pending; automated
fixtures verify orchestration and recovery without mutating the desktop host.
