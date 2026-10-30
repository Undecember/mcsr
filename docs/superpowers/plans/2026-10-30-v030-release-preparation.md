# v0.3.0 Release Preparation Plan

> **For agentic workers:** Use superpowers:executing-plans to execute these tasks inline.

**Goal:** Prepare a verified v0.3.0 candidate from reviewed dev without publication.

**Architecture:** Isolate release metadata in release/v0.3.0. Preserve runtime sources and dependency versions. Stage the executable and checksum with a source-input manifest and explicit acceptance limits.

**Tech Stack:** Rust/Cargo, Node.js, native keyd tests, GNOME schemas, Playwright.

**Spec:** docs/rust-release-checklist.md

## Constraints

- Keep main release-only. Preparation does not merge, tag, push or publish.
- Retain historical release records and user data; do not install the candidate during preparation.
- The live repair record establishes developer-build recovery and startup, not full candidate focus/chat/lifecycle acceptance.

## Tasks

- [x] Confirm dev/remote identity and create isolated release branch.
- [x] Bump only toolkit package metadata to 0.3.0; prepare notes and README.
- [x] Run Rust, native keyd, JavaScript, schema and isolated browser checks.
- [x] Build release executable and verify embedded assets.
- [x] Stage exact executable/checksum, source-input manifest, environment and ELF/GLIBC identity.
- [x] Refresh Graphify and obtain independent release review (PASS; no material findings).
- [x] Prepare concrete candidate handoff with remaining approval gates.

## Review Focus

Verify unchanged dependencies/runtime sources, exact artifact identity, complete inputs including native keyd, accurate live-test limits and preserved historical release records.

## Approved local integration

The operator approved committing preparation, squash-merging into main and
creating annotated tag v0.3.0 on 2026-10-30. Publication and main-to-dev
synchronization retain separate approval gates.
