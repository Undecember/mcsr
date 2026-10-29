# Rust single-binary release checklist

## v0.1.0-beta.2 candidate — 2026-10-29

Status: **[`v0.1.0-beta.2` published as a GitHub prerelease](https://github.com/Undecember/mcsr/releases/tag/v0.1.0-beta.2)** at `2026-10-29T05:36:47Z`. Release-only `main` and synchronized `dev` point to tagged release commit `105a32bde56a0694dac894520ae079f81388d2b0` at publication. Target support remains Ubuntu GNOME Shell 50 on x86_64 Wayland; the automated build host is Ubuntu 24.04.5 LTS and does not establish live GNOME acceptance.

- [x] Version selected: `v0.1.0-beta.2`; retain Cargo package `0.1.0`, following beta.1's tag/hash convention.
- [x] Identify application source base: `8b81d6f7a681492e2dcb88eaf625abc84f4119ce` with its existing `Cargo.lock`. Local configuration and generated graph changes are excluded from binary inputs.
- [x] Run formatting, Clippy, Rust tests, extension/UI harnesses, live-UI fixture self-test, and strict schema validation.
- [x] Run existing browser key capture smoke test with isolated API responses; no live device operations.
- [x] Build release executable and run release embedded-asset tests.
- [x] Stage the executable and matching checksum under `target/release-preparation/v0.1.0-beta.2/`; verify the exact staged files and inspect runtime dependencies.
- [x] Review beta.2 release notes and README, including supported mouse recording destinations and retained live acceptance limitations.
- [x] Integrate reviewed release changes into release-only `main` and tag the release commit `v0.1.0-beta.2`.
- [x] Publish the reviewed notes as a GitHub prerelease with the exact executable and checksum assets.
- [x] Download the public assets and verify their checksum and expected executable hash.

### Verification evidence

Completed on 2026-10-29 UTC using Rust/Cargo 1.91.1, Node.js 24.14.1, and GLib 2.80.0:

- `cargo fmt --check` and `cargo clippy --all-targets --locked -- -D warnings`: passed.
- `cargo test --locked`: 317 passed, 41 filtered out across 21 suites; no failures.
- `node tests/extension_harness.cjs`, `node tests/ui_harness.cjs`, and `node integration/live-ui-acceptance.mjs self-test`: passed.
- `glib-compile-schemas --strict --dry-run extension/schemas`: passed.
- `cargo build --release --locked` and `cargo test --release --locked --test rust_assets`: passed; all 5 asset tests passed.
- `playwright-cli -s=beta2-release run-code --filename=tests/key_capture_browser.cjs` against an isolated static server with mocked API responses: passed in headless Chrome. Covered keyboard capture, all five supported mouse buttons, click-through suppression, timeout, shortcuts, profile save, and mobile layout.
- `sha256sum --check mcsr-toolkit.sha256` in the staged candidate directory: `mcsr-toolkit: OK`. Executable SHA256: `4f8b21e648fdc7d7d8fc96bb27d7b5e3e0e290643fae6c66856450760662f00d`.
- `ldd` resolved every dependency; `readelf -d` lists only `libgcc_s.so.1`, `libm.so.6`, `libc.so.6`, and `ld-linux-x86-64.so.2`; no Python runtime library dependency.

Candidate files: `target/release-preparation/v0.1.0-beta.2/mcsr-toolkit` and `mcsr-toolkit.sha256`. The ignored `target/` directory contains local artifacts, not committed release assets. Do not rebuild or replace the executable without updating its checksum and these notes.

### Publication verification

- Published the reviewed release notes and exactly two assets: `mcsr-toolkit` (12,417,648 bytes) and `mcsr-toolkit.sha256` (79 bytes).
- Downloaded both assets through their public URLs without authentication; each is byte-identical to the staged candidate. The downloaded checksum verifies executable SHA256 `4f8b21e648fdc7d7d8fc96bb27d7b5e3e0e290643fae6c66856450760662f00d`.
- Two complete CI runs passed for release commit `105a32bde56a0694dac894520ae079f81388d2b0`: [run 40823957432](https://github.com/Undecember/mcsr/actions/runs/40823957432) and [run 40824015737](https://github.com/Undecember/mcsr/actions/runs/40824015737).
- The first main CI attempt hit the API test helper's roughly half-second job-wait deadline in three tests. The same API suite passed locally and in the other CI runs; the failed run was retried. Application source and the published executable were unchanged.

### Remaining live gates

The existing supervised results below are historical beta.1 evidence. Beta.2's control picker, recording behavior, and expanded bindings have not been tested on the supervised Minecraft/GNOME device. Live logout/login while remapped, lock/unlock and crash/restart during active keyd remapping, guided uninstall rollback/prompt counts, and a separate installed-binary new-instance Restore cycle remain deferred. Automated tests and browser mocks do not close those gates. Stable-release acceptance is not claimed.

## Historical v0.1.0-beta.1 preparation record

The following record retains the original binary identity and acceptance evidence. Its unchecked publication items describe the historical preparation state, not the beta.2 candidate.

Status: **preparing the user-authorized `v0.1.0-beta.1` GitHub prerelease**. Supervised Rust evidence and failures are recorded in [the Wayland record](../integration/acceptance-wayland.md). The operator deferred live keyd logout/login, lock/unlock, crash/restart, and guided uninstall rollback/prompt-count tests to beta users and feedback; these are disclosed limitations, not stable-release sign-off criteria for this beta. Exact release asset verification still needs to be completed before publication. The target is Ubuntu GNOME Shell 50 on x86_64 Wayland.

## Source cutover

- [x] The user chose Rust source removal before the remaining live gates. Python package source, tests, metadata, and obsolete helper templates have been removed; Git history and dated acceptance records retain the migration evidence.
- [x] Active setup and developer guidance now describe the Rust executable. The Rust installer still refuses an active Python-style service and unresolved Python-era recovery. Preserve Python-era user data, profiles, backups, and root recovery evidence through acceptance and uninstall.
- [x] The beta executable's source base is `cef7a3336afe4ef69bc334d601f3d01ccab57be2`. This is the source identity for the binary, not necessarily the later commit that carries release documentation and the tag.

## Candidate and reproducible checks

- [x] Confirm the candidate comes from the reviewed source and `Cargo.lock`, on the intended binary source base `cef7a3336afe4ef69bc334d601f3d01ccab57be2`.
- [x] Run `cargo fmt --check`, `cargo clippy --all-targets --locked -- -D warnings`, `cargo test --locked`, `node tests/extension_harness.cjs`, `node tests/ui_harness.cjs`, `node integration/live-ui-acceptance.mjs self-test`, and `glib-compile-schemas --strict --dry-run extension/schemas`.
- [x] Run `cargo build --release --locked` and `cargo test --release --locked --test rust_assets`. The asset test checks browser HTML/JavaScript and the archive's extension, final UUID/project URL, and compiled schema.
- [x] In `target/release`, run `sha256sum mcsr-toolkit > mcsr-toolkit.sha256` and `sha256sum --check mcsr-toolkit.sha256`. The notes record the binary SHA256, build environment, and source base. The exact downloadable asset still needs a final checksum recheck.
- [x] Check the executable's runtime library dependencies with `ldd` and `readelf -d`; confirm no Python library dependency and a supported target environment.
- [ ] After live installation, inspect the runtime process for absence of Python. After publication, verify the checksum of the exact downloaded release executable.
- [x] Confirm the MIT license, `mcsr-toolkit@undecember.github.io` extension UUID, `https://github.com/Undecember/mcsr` project URL, and release notes are accurate.

The [Rust CI workflow](../.github/workflows/rust.yml) performs the clean-checkout checks and retains a candidate artifact. A CI artifact is not a public release. Its checksum must be verified again after download; release uploads need a new verification of the exact files being published.

## Supervised live acceptance

Obtain separate authorization for privileged installation and live keyboard/sensitivity operations before device actions. Record the operator, target Ubuntu/GNOME/kernel, CPU, keyd and maccel versions, driver fingerprint, Minecraft version, UTC time, binary SHA256, original `options.txt` hashes, original keyd configuration, and exact maccel raw/CLI baseline. Preserve Python and Rust recovery records and user backups. Confirm that no legacy Python service is active and resolve any Python-managed remap or unresolved recovery record before Rust service cutover; source removal does not imply legacy data has been cleared.

### 2026-10-26 Enter dual-role run

The reviewed app and helper matched SHA256 `28f234d99be62a27903043c3282eb248ba5ad29ff599792fdac035bf2e8d84f1`, root-owned mode `0755`. Browser Save/Preview showed exactly one Backward S→K mapping; Apply created backup `000000000000000018e210a390b1c2c0` and changed the options hash from `d23566cf7e5697bbc41c26b2c3efa0f744327068e30c577134c0595de7921d48` to `442751db8abe8f1a8b5b58d70ed6e11c83c35b9e5eee889e6d520639534784ab`. Physical S moved Backward, Enter opened chat and sent the message to close it, Slash/Esc worked, and outside-game `s/t` typed normally both before and after Restore. A harmless GNOME Polkit prompt accepted the password. Focus transitioned true→false with clear count 7 and no errors. Browser Restore completed after reviewed recovery; final options match the original hash, `/etc/keyd` is empty, lineage is null, managed and recovery state are empty, service is observe-only/connected/error-free, keyd is active/enabled, and maccel is raw `4294967296` / CLI `1.000000000`. The original T-chat profile is restored. See the [dated acceptance record](../integration/acceptance-wayland.md) for the recovery sequence and driver cleanup fix.

**Deferred for beta feedback:** The operator explicitly skipped logout/login (“can't logout for now”), so live logout/login while a remap is active remains untested. No root fallback timer was created. Lock/unlock and service crash/restart recovery were observed only for eye sensitivity in observe-only mode; they were not tested while keyd remapping was active. The Enter dual-role case passed; its focus clear was asynchronous, and the observation does not establish zero leakage during focus transitions. Guided uninstall with rollback and prompt counts is also untested. These cases are listed here for a later supervised run and are disclosed in the beta release notes; the operator chose to proceed with the beta without waiting for them.

- [x] Run the first real observe-only Rust GNOME probe; record focus, heartbeat, modal, and resize observations without activating keyd or sensitivity changes. Observations and limitations are recorded in the [Wayland acceptance record](../integration/acceptance-wayland.md), including the October 24 install/focus/resize run and October 26 service startup check.
- [ ] Confirm `/usr/bin/gdbus` is available. Read the `org.gnome.Shell` session-bus owner PID with `gdbus call --session --dest org.freedesktop.DBus --object-path /org/freedesktop/DBus --method org.freedesktop.DBus.GetConnectionUnixProcessID org.gnome.Shell`; confirm it identifies the active `/usr/bin/gnome-shell` process. Repeat after logout/login to establish that the owner process changed before `repair` advances.
- [ ] Install the verified release artifact through the guided installer. For a new or changed extension, confirm the first call reports incomplete staging, writes the exact four embedded files and Shell restart marker, and starts no root setup or new user service. Repeat `repair` in the same Shell session and confirm it remains staged. Log out, log back in, confirm `gnome-extensions list --user` recognizes the UUID, then run `repair`. Before root setup, verify the user-owned `mcsr-toolkit` and `rust-v1` directories are `0700`, marker bytes remain intact, and `XDG_DATA_HOME` plus Python `state/` permissions have not changed. Confirm `mcsr-toolkit@local` becomes INACTIVE, the `org.mcsr.Toolkit` interface at `/org/mcsr/Toolkit` disappears from `org.gnome.Shell`, and the new UUID becomes ACTIVE; Enabled=Yes alone is insufficient. Only after `repair` succeeds, verify the installed user executable and root helper have the candidate's SHA256, ownership and permissions, exact sudoers policy, extracted extension UUID/schema, and observe-only user unit. Run diagnostic `doctor` and inspect service logs; root status inspection may recreate a volatile keyd lock after reboot. GNOME [documents](https://gjs.guide/extensions/development/debugging.html#restarting-gnome-shell) logout/login for a Wayland Shell restart.
- [ ] If the new UUID is in State ERROR, confirm `repair` does not start root setup, records the prior `@local` state, and leaves only the new UUID configured to load for the next Shell session. Log out and back in; run `repair` only after checking both UUID states. It must require new ACTIVE before service setup. On failure, verify `@local` is restored or follow the explicit manual recovery path while retaining the marker, old files, and helper. Never log out with both UUIDs enabled. A `handoff-uncertain` marker must block automatic repair until extension and service state are reviewed.
- [ ] Record authentication prompts separately for extension staging and post-login `repair`. Staging must start no root setup transaction; protected root status inspection may still request one combined status prompt. Post-login `repair` may require one helper bootstrap prompt on a fresh host and one setup prompt; verified existing helpers need no bootstrap prompt. More than one setup prompt or more than one helper bootstrap prompt is a failed acceptance result. Keep the setup prompt open through user-service readiness and confirm the transaction commits only after readiness.
- [ ] If a previous attempt left `rust-v1/install-pending.json`, preserve it and replace a mismatched root helper only through the separately reviewed manual helper replacement procedure. Verify the installed helper's SHA256, root ownership, mode, and safe ancestors before `repair`. Run `doctor`, verify no unresolved keyd/maccel recovery, then use `repair`; inspect the exact service and policy files afterward. Do not delete the pending marker or historical root snapshots to make preflight pass.
- [ ] Review and remove the documented Python `mcsr-toolkit.service.d/override.conf` only after Python Restore and recovery checks. Confirm `systemctl --user show -P DropInPaths mcsr-toolkit.service` is empty and the resolved `ExecStart` is exactly `/usr/local/bin/mcsr-toolkit serve`; confirm the service stays active and `/api/session` responds before accepting installation.
- [ ] In the browser UI, save/preview, Apply and Restore a profile; verify exact instance options and backup lineage. Create a new eligible instance and confirm reconciliation.
- [ ] With explicit live activation, verify chat/movement remaps inside Minecraft, ordinary typing after blur, Shell modal and password prompts, and keyd clear/recovery. Include lock/unlock and logout/login; note any asynchronous interval rather than claiming zero leakage.
- [ ] Check exact normal and eye window resize, shifted shortcuts, baseline and eye sensitivity, and restoration on normal preset, blur, modal, extension disconnect, shutdown, crash/restart, lock/logout as applicable. Record sampled timing limitations.
- [ ] Uninstall. Confirm ordinary input, original keyd configuration or safe stopped state, exact original maccel raw and CLI values, no root maccel recovery record, no Python process/runtime dependency, user backups and Python-era data retained, and no active Rust user unit or shortcut.
- [ ] Count uninstall authentication prompts. Existing verified policies or protected root history may require one combined status prompt; fixed-target root removal requires one more. More than two prompts is a failed acceptance result. On an abort, check that the old service, unit, policy, and helper are restored; on interrupted commit, retain the pending marker and reviewed helper for repair.
- [ ] If root removal completed but the final response or marker removal failed, use `repair` only after verifying all fixed root targets, the user unit, enabled service, and extension are absent. It may reconcile the marker without the deleted helper; protected historical root data remains untouched. A partial root removal requires manual review before another attempt.

If a future supervised case fails, retain evidence and recovery tooling, investigate the cause, rerun relevant automated checks, and repeat the affected live case and cleanup checks before treating that behavior as accepted. Preserve Python-era recovery data until its state has been verified; source removal does not remove that data or clear recovery safeguards.

## Release decision

- [x] Add dated Rust acceptance results, including failures, recovery, measurements, and remaining limits, to [the Wayland record](../integration/acceptance-wayland.md).
- [x] Record the binary source base and built executable SHA256 in the [beta release notes](rust-release-notes.md): `cef7a3336afe4ef69bc334d601f3d01ccab57be2` and `28f234d99be62a27903043c3282eb248ba5ad29ff599792fdac035bf2e8d84f1`.
- [ ] Verify the checksum asset against the exact downloaded executable intended for publication.
- [x] Beta version selected: `v0.1.0-beta.1`; stable `v0.1.0` acceptance is not claimed.
- [ ] Review the final staged diff and checks for the exact artifact intended for publication. Preserve the repository's release workflow and obtain any separately required approval for commits, merges, tags, pushes, or publication. Keep `main` release-only and tag each release commit.
