# v0.1.0-beta.2 Release Preparation Plan

> **For agentic workers:** Use inline execution and verification-before-completion. This task prepares existing software for release; it adds no application behavior.

**Goal:** Prepare the user-selected v0.1.0-beta.2 prerelease with reviewed notes and verified local assets.

**Architecture:** Build the existing dev source with the locked dependencies. Identify the executable by source commit and SHA256, following beta.1's separate binary-source and release-tag convention.

**Tech Stack:** Rust, JavaScript, GNOME extension assets.

**Spec:** User request to prepare a release; version confirmed as v0.1.0-beta.2. Existing gates are in docs/rust-release-checklist.md.

## Constraints

- Preserve existing local configuration and Graphify changes.
- Retain the beta's deferred supervised acceptance cases; automated tests do not replace them.
- Keep Cargo package version 0.1.0, as in beta.1; identify the prerelease by tag and executable hash.
- Prepare local files only; publication, main integration, and tagging remain pending.

## Review Focus

- Do not carry beta.1's binary hash or source identity into beta.2.
- Describe mouse recording restrictions and Minecraft 1.16 default labels accurately.
- Distinguish local candidate verification from downloaded public asset verification.

## Task 1: Candidate and documentation

- [x] Run all documented deterministic checks and the existing key capture browser smoke test.
- [x] Build release executable and run release embedded-asset tests.
- [x] Stage executable and matching checksum in target/release-preparation/v0.1.0-beta.2/; verify checksum and runtime dependencies.
- [x] Update README.md, docs/rust-release-notes.md, and docs/rust-release-checklist.md with beta.2 changes and fresh evidence; retain beta.1 checklist as history.
- [x] Review the release diff, document remaining publication/live gates, and report artifact paths.

No new tests are needed for release prose; existing application tests provide the candidate verification.
