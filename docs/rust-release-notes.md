# MCSR Toolkit v0.1.0-beta.2

MCSR Toolkit is a local setup and keybind configurator for Minecraft speedrunning on Ubuntu GNOME Wayland. This beta is distributed as a single Rust executable. It includes the loopback browser service, UI assets, GNOME Shell 50 extension, guided setup and recovery commands, and narrowly scoped privileged helper modes.

## New in beta.2

- Choose Minecraft controls by their default key and action name, search the list, or enter a custom control ID for modded bindings. Labels use Minecraft Java 1.16 defaults rather than the instance's current bindings; existing custom IDs are preserved.
- Use **Record input…** beside binding fields to record physical keyboard keys or supported mouse buttons. Recording expires after five seconds, preserves the previous value on timeout or focus loss, and keeps manual entry available for desktop-reserved keys.
- Record left/right modifiers in shortcuts and use the expanded navigation, punctuation, lock, and keypad key support.
- Native Minecraft controls and emitted crafting/Ninjabrain bindings can record left, middle, right, back, and forward mouse buttons. Managed keyd configuration matches keyboards, so physical crafting/Ninjabrain sources and chat fields record keyboard inputs; resize presets require keyboard shortcuts. Extra mouse buttons depend on browser delivery. Wheel and pointer movement recording are not supported.

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

If install stages a GNOME extension and reports that setup is incomplete, log out of GNOME Wayland, log back in, then run `./mcsr-toolkit repair` from the same verified executable. For interrupted setup or uninstall, use `doctor` and the documented recovery guidance in the [README](https://github.com/Undecember/mcsr/blob/v0.1.0-beta.2/README.md#repair-and-uninstall). Review recovery state before continuing; do not delete markers or recovery records to bypass a refusal.

To uninstall, first restore toolkit-managed keyd and maccel state and verify ordinary input, then run `./mcsr-toolkit uninstall`. Review the README's [uninstall and drop-in guidance](https://github.com/Undecember/mcsr/blob/v0.1.0-beta.2/README.md#repair-and-uninstall) before removing user service overrides.

## Beta status and known limitations

Earlier supervised Rust checks passed for installation and repair, browser Save/Preview/Apply/Restore, live Backward remapping, the Enter dual-role chat case, focus clearing, and recovery to the recorded keyd, sensitivity, and Minecraft options baselines. Those checks predate beta.2 and do not validate the new recording and picker behavior on a live Minecraft device. The detailed evidence and limits are in the [Wayland acceptance record](https://github.com/Undecember/mcsr/blob/v0.1.0-beta.2/integration/acceptance-wayland.md).

- A live GNOME logout/login while a key remap is active has not been tested. The operator deferred this case for beta feedback.
- Lock/unlock and service crash/restart recovery were observed for eye sensitivity in observe-only mode. They have not been tested while live keyd remapping is active; see the dated [Rust acceptance record](https://github.com/Undecember/mcsr/blob/v0.1.0-beta.2/integration/acceptance-wayland.md#2026-10-26-rust-enter-dual-role-live-acceptance).
- Guided uninstall with rollback and authentication prompt counts has not been tested on the supervised Rust device. Review the checklist before using uninstall and report problems through the [GitHub issue tracker](https://github.com/Undecember/mcsr/issues).
- Focus changes are asynchronous. A brief interval may occur before keyd bindings clear; the observed checks do not establish zero input leakage during focus transitions.
- A separate Restore cycle using the installed binary with the new-instance Restore fix was not rerun. Existing observed Restore and recovery outcomes are documented in the acceptance record.

## Candidate verification and artifact identity

This is a locally prepared prerelease candidate, not a published release. Automated verification results and remaining publication steps are recorded in the [release checklist](https://github.com/Undecember/mcsr/blob/v0.1.0-beta.2/docs/rust-release-checklist.md#v010-beta2-candidate--2026-10-29).

The executable's binary source base is commit `8b81d6f7a681492e2dcb88eaf625abc84f4119ce`. The release tag commit may be newer because it includes release documentation. Application source, embedded assets, and `Cargo.lock` match that source base; local agent configuration and generated Graphify files are outside the binary inputs.

| Item | Value |
| --- | --- |
| Binary source base | `8b81d6f7a681492e2dcb88eaf625abc84f4119ce` |
| Executable SHA256 | `4f8b21e648fdc7d7d8fc96bb27d7b5e3e0e290643fae6c66856450760662f00d` |
| Build environment | Ubuntu 24.04.5 LTS, Linux 6.8.0-147-generic, `x86_64-unknown-linux-gnu`; Rust/Cargo 1.91.1, Node.js 24.14.1, GLib 2.80.0 |

The matching `mcsr-toolkit.sha256` must be published beside that executable. Verify the exact downloaded public files after publication; local candidate verification does not establish their identity. Build and deterministic verification commands are in the [README](https://github.com/Undecember/mcsr/blob/v0.1.0-beta.2/README.md#build-and-deterministic-checks). This project is licensed under [MIT](https://github.com/Undecember/mcsr/blob/v0.1.0-beta.2/LICENSE).

The Cargo package version remains `0.1.0`, following beta.1's convention. This prerelease is identified by tag `v0.1.0-beta.2` and the exact executable SHA256 above.
