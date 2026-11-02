# v0.3.1 Release Preparation Plan

Goal: prepare a patch candidate containing the reviewed extension connection recovery fix, without publication.

Architecture: release/v0.3.1 in an isolated worktree, based on dev c9ea7e94f0e8e3a869d8c749fc7e99307fac3730 plus the reviewed uncommitted repair fix. Bump only toolkit Cargo metadata; preserve dependencies. Freeze exact inputs and stage executable/checksum under the primary checkout target/release-preparation/v0.3.1/.

- [x] Verify remote identity and absence of matching release refs; isolate reviewed changes.
- [x] Bump toolkit package/lockfile to 0.3.1; prepare notes and checklist.
- [x] Run full CI checks and isolated browser regression; record all results.
- [x] Build and stage exact executable/checksum; verify source manifest, embedding and ELF requirements.
- [x] Refresh Graphify and obtain independent release review.
- [x] Deliver candidate paths and readiness limits.

Preparation does not install the candidate, merge main, tag, push, or publish. The earlier developer-build installation was refused on a helper hash mismatch; automatic in-place upgrade remains unvalidated. Preserve historical release records and existing installed service.

Verification: 461 Rust tests, native keyd 94 upstream/28 toolkit cases, JS harnesses and 14 UI tests, fixture self-test, schemas, browser regression, optimized build and six release asset tests passed. Graph updated (2,903 nodes, 7,098 edges). The exact pair and source/ELF/environment records are staged in the primary checkout.

Independent release review: PASS, no actionable findings. Preparation remains uncommitted/unpublished in release/v0.3.1. Primary development changes preserved; no installation attempted for the candidate.

2026-11-02: Operator explicitly requested publication followed by installation on this device. Local release commit, squash integration into main, annotated tag and publication are now authorized; development synchronization is not part of this step.
