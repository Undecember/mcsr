# Rust source cutover

## Intent

Make the repository's supported product and contributor workflow Rust-based. The Rust executable already contains the service, installer, privileged helper modes, browser assets, and GNOME extension archive. Remove the retained Python implementation and its runnable setup path now, while keeping live logout and uninstall acceptance as separate gates for the first public release. This choice advances the source cutover without claiming release readiness.

## Approaches

- **Chosen: remove Python source now, retain recovery evidence.** Delete the Python package, packaging metadata, Python tests, and legacy source-side sudoers templates. Rewrite active setup and developer instructions for the Rust executable. Preserve user data, installed legacy recovery tools, and dated acceptance records. The user chose this sequence so code removal does not wait for a convenient logout window.
- Wait for all live acceptance before removing source. This follows the original migration plan's sequencing but leaves the repository presenting two setup paths until the operator can complete logout and uninstall checks.
- Archive Python under a legacy directory. This keeps a second implementation in the release source and requires contributors to distinguish active from archived code. Git history already retains the exact old files, so an in-tree archive adds little recovery value.

## Boundaries

Remove `pyproject.toml`, `src/mcsr_toolkit/`, `tests/test_*.py`, and the Python helper sudoers templates `packaging/mcsr-toolkit-keyd.sudoers` and `packaging/mcsr-toolkit-maccel.sudoers`. Keep shared fixtures, Rust source and tests, browser assets, GJS extension, compiled-schema inputs, Rust policy templates, and the existing historical design and acceptance records. Do not delete or reinterpret user profiles, instance backups, Python-era state, root keyd snapshots, maccel recovery records, or pending installer markers. Do not change installed binaries, services, sudoers, keyd, maccel, or the current Minecraft instance as part of this source change.

The Rust installer must retain its refusal of an active Python-style service or unresolved Python recovery. Its test for a competing service lock currently spawns `python3`; replace that test fixture with a Rust child process holding the same lock so the Rust test suite has no Python interpreter dependency. Preserve the test's observable refusal and cleanup checks. Any Python-specific strings needed to detect or explain old installations remain valid migration behavior, rather than a product runtime dependency.

Rewrite the README as the Rust-only installation, repair, uninstall, UI, safety, and contributor guide. Its commands must name the actual binary modes and installed paths. Replace runnable Python-only instructions in the resize guide with current Rust or direct GNOME observation commands, and label older measurements as historical evidence. Keep the dated Wayland acceptance record and prior plans/specs intact as history, while fixing active links to removed files. Update the release checklist and notes to distinguish **source cutover complete** from **live acceptance pending**. The README must not tell new users to install a Python virtual environment or deploy the old helpers.

## Verification and release boundary

Run Rust formatting, strict Clippy, the full locked Rust suite, release build and embedded-asset tests, both JavaScript harnesses, the browser acceptance self-test, and strict GSettings schema validation. Check that tracked Python product/test/packaging files are gone, active documentation has no broken local links or runnable commands requiring deleted paths, and `ldd`/`readelf` show no Python runtime library requirement. Test the native competing-lock fixture without relying on `python3`. Review the deletion list and documentation diff independently before integration.

The live keyboard logout/login check, guided uninstall with rollback and prompt counts, and exact release-artifact verification remain on the release checklist. A source-only cutover does not authorize privileged cleanup or publication. Do not merge, push, tag, or publish without the separate approvals required by `AGENTS.md`.
