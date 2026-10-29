# MCSR Toolkit v0.1.0-beta.3

MCSR Toolkit is a local setup and keybind configurator for Minecraft speedrunning on Ubuntu GNOME Wayland. This beta is distributed as a single Rust executable. It includes the loopback browser service, UI assets, GNOME Shell 50 extension, guided setup and recovery commands, and narrowly scoped privileged helper modes.

## New in beta.3

- Fix uninstall on systems that provide `/usr/bin/keyd.rvaiya` without `/usr/bin/keyd`. Recovery uses the trusted helper to validate configuration, restart keyd, and verify daemon health, avoiding the client reset that fails when no configurations are loaded.
- Preserve the cause of root recovery verification failures in `doctor`, including helper hash mismatch, authentication failure, and malformed status. Unverified recovery still blocks installation and uninstall, including when only protected policy files remain.
- Include exit status, stdout, and stderr in failed-command diagnostics, so a command that exits without output no longer produces an unexplained blank error.

The control picker, keyboard/mouse input recording, expanded key support, and other beta.2 features remain included.

## Highlights

- Preview physical → emitted → Minecraft key mappings, then apply or restore instance files with retained backups.
- Observe-only service startup; live keyd remapping requires an explicit local service configuration.
- Focus-aware keyd and optional maccel integration, Minecraft window resize shortcuts, and guided `doctor`, `install`, `repair`, and `uninstall` commands.
- Rust state is kept separate from Python-era state. The installer refuses unresolved Python recovery, and uninstall retains profiles, backups, and Python-era recovery data.

## Download and install

**Target:** Ubuntu with GNOME Shell 50 on x86_64 Wayland. `keyd` is required for keyboard integration; `maccel` is optional for eye sensitivity. The extension UUID is `mcsr-toolkit@undecember.github.io`.

Download `mcsr-toolkit` and its matching `mcsr-toolkit.sha256` asset. Keep them together, verify the checksum, make the executable runnable, then inspect prerequisites:

```sh
sha256sum --check mcsr-toolkit.sha256 && chmod +x ./mcsr-toolkit && ./mcsr-toolkit doctor
```

Inspect the `doctor` report before continuing. Run install separately after reviewing prerequisites and recovery state:

```sh
./mcsr-toolkit install
```

If install stages a GNOME extension and reports that setup is incomplete, log out of GNOME Wayland, log back in, then run `./mcsr-toolkit repair` from the same verified executable. For interrupted setup or uninstall, use `doctor` and the documented recovery guidance in the [README](https://github.com/Undecember/mcsr/blob/v0.1.0-beta.3/README.md#repair-and-uninstall). Review recovery state before continuing; do not delete markers or recovery records to bypass a refusal.

A downloaded executable must match the installed root helper before it can verify protected recovery state. If `doctor` reports a helper hash mismatch, run `/usr/local/bin/mcsr-toolkit doctor` from the matching installed version to inspect recovery. This diagnostic does not update the installation or authorize overwriting the helper; a mismatched existing helper still requires manual review.

To uninstall, first restore toolkit-managed keyd and maccel state and verify ordinary input, then run the matching installed `/usr/local/bin/mcsr-toolkit uninstall`. Older installed uninstallers retain their original behavior until updated; downloading this candidate alone does not fix them. Review the README's [uninstall and drop-in guidance](https://github.com/Undecember/mcsr/blob/v0.1.0-beta.3/README.md#repair-and-uninstall) before removing user service overrides.

## Beta status and known limitations

Earlier supervised Rust checks passed for installation and repair, browser Save/Preview/Apply/Restore, live Backward remapping, the Enter dual-role chat case, focus clearing, and recovery to the recorded keyd, sensitivity, and Minecraft options baselines. Those checks predate beta.2 and beta.3 and do not validate the recording/picker behavior or the new uninstall recovery sequence on a live Minecraft device. The beta.3 fix has regression and helper/rollback test coverage; successful live uninstall with the rebuilt candidate has not been observed. The detailed evidence and limits are in the [Wayland acceptance record](https://github.com/Undecember/mcsr/blob/v0.1.0-beta.3/integration/acceptance-wayland.md).

- A live GNOME logout/login while a key remap is active has not been tested. The operator deferred this case for beta feedback.
- Lock/unlock and service crash/restart recovery were observed for eye sensitivity in observe-only mode. They have not been tested while live keyd remapping is active; see the dated [Rust acceptance record](https://github.com/Undecember/mcsr/blob/v0.1.0-beta.3/integration/acceptance-wayland.md#2026-10-26-rust-enter-dual-role-live-acceptance).
- Successful guided uninstall with rollback and authentication prompt counts has not been validated on the supervised Rust device. The old installed uninstaller failed with a missing `/usr/bin/keyd`, then a silent `bind reset` failure; beta.3 addresses those reproduced causes in source, without claiming live acceptance. Review the checklist before using uninstall and report problems through the [GitHub issue tracker](https://github.com/Undecember/mcsr/issues).
- Focus changes are asynchronous. A brief interval may occur before keyd bindings clear; the observed checks do not establish zero input leakage during focus transitions.
- A separate Restore cycle using the installed binary with the new-instance Restore fix was not rerun. Existing observed Restore and recovery outcomes are documented in the acceptance record.

## Candidate verification and artifact identity

The artifacts were built and verified locally. Automated verification results, publication status, and public download verification are recorded in the [release checklist](https://github.com/Undecember/mcsr/blob/v0.1.0-beta.3/docs/rust-release-checklist.md#v010-beta3-candidate--2026-10-29).

The executable's binary source base is commit `e3090238f9317927dfab175bef3cce1c896f6be8`. The release tag commit may be newer because it includes release documentation. Application source, embedded assets, and `Cargo.lock` match that source base; local agent configuration and generated Graphify files are outside the binary inputs.

| Item | Value |
| --- | --- |
| Binary source base | `e3090238f9317927dfab175bef3cce1c896f6be8` |
| Executable SHA256 | `98154ab5152ab73029bf4f9ac753514870abed6b6b4ad6803453cfadd20afc14` |
| Executable size | `12,730,704` bytes |
| Build environment | Ubuntu 26.04.1 LTS, Linux 7.0.0-38-generic, `x86_64-unknown-linux-gnu`; Rust/Cargo 1.98.1, Node.js 24.21.0, GLib 2.88.0 |

The staged candidate is under `target/release-preparation/v0.1.0-beta.3/` in the primary checkout. Its dynamic library requirements resolve locally, and its highest required GLIBC symbol version is `2.39`; no Python runtime library is linked. This is local dependency verification, not a test on other Ubuntu versions.

The matching `mcsr-toolkit.sha256` must be published beside that executable. Verify the exact downloaded public files after publication; local candidate verification does not establish their identity. Build and deterministic verification commands are in the [README](https://github.com/Undecember/mcsr/blob/v0.1.0-beta.3/README.md#build-and-deterministic-checks). This project is licensed under [MIT](https://github.com/Undecember/mcsr/blob/v0.1.0-beta.3/LICENSE).

The Cargo package version remains `0.1.0`, following beta.1's convention. This prerelease is identified by tag `v0.1.0-beta.3` and the exact executable SHA256 above.
