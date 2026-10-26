# MCSR Toolkit v0.1.0-beta.1

MCSR Toolkit is a local setup and keybind configurator for Minecraft speedrunning on Ubuntu GNOME Wayland. This beta is distributed as a single Rust executable. It includes the loopback browser service, UI assets, GNOME Shell 50 extension, guided setup and recovery commands, and narrowly scoped privileged helper modes.

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

If install stages a GNOME extension and reports that setup is incomplete, log out of GNOME Wayland, log back in, then run `./mcsr-toolkit repair` from the same verified executable. For interrupted setup or uninstall, use `doctor` and the documented recovery guidance in the [README](https://github.com/Undecember/mcsr/blob/v0.1.0-beta.1/README.md#repair-and-uninstall). Review recovery state before continuing; do not delete markers or recovery records to bypass a refusal.

To uninstall, first restore toolkit-managed keyd and maccel state and verify ordinary input, then run `./mcsr-toolkit uninstall`. Review the README's [uninstall and drop-in guidance](https://github.com/Undecember/mcsr/blob/v0.1.0-beta.1/README.md#repair-and-uninstall) before removing user service overrides.

## Beta status and known limitations

Supervised Rust checks have passed for installation and repair, browser Save/Preview/Apply/Restore, live Backward remapping, the Enter dual-role chat case, focus clearing, and recovery to the recorded keyd, sensitivity, and Minecraft options baselines. The detailed evidence and limits are in the [Wayland acceptance record](https://github.com/Undecember/mcsr/blob/v0.1.0-beta.1/integration/acceptance-wayland.md).

- A live GNOME logout/login while a key remap is active has not been tested. The operator deferred this case for beta feedback.
- Lock/unlock and service crash/restart recovery were observed for eye sensitivity in observe-only mode. They have not been tested while live keyd remapping is active; see the dated [Rust acceptance record](https://github.com/Undecember/mcsr/blob/v0.1.0-beta.1/integration/acceptance-wayland.md#2026-10-26-rust-enter-dual-role-live-acceptance).
- Guided uninstall with rollback and authentication prompt counts has not been tested on the supervised Rust device. Review the checklist before using uninstall and report problems through the [GitHub issue tracker](https://github.com/Undecember/mcsr/issues).
- Focus changes are asynchronous. A brief interval may occur before keyd bindings clear; the observed checks do not establish zero input leakage during focus transitions.
- A separate Restore cycle using the installed binary with the new-instance Restore fix was not rerun. Existing observed Restore and recovery outcomes are documented in the acceptance record.

## Artifact identity

The executable's binary source base is commit `cef7a3336afe4ef69bc334d601f3d01ccab57be2`. The release tag commit may be newer because it includes release documentation; it does not identify the binary's source. The executable SHA256 below is for the built release asset:

| Item | Value |
| --- | --- |
| Binary source base | `cef7a3336afe4ef69bc334d601f3d01ccab57be2` |
| Executable SHA256 | `28f234d99be62a27903043c3282eb248ba5ad29ff599792fdac035bf2e8d84f1` |
| Build environment | Ubuntu 26.04.1 LTS, Linux 7.0.0-38-generic, `x86_64-unknown-linux-gnu`; Rust/Cargo 1.98.1, Node.js 24.21.0, GLib 2.88.0 |

The published `mcsr-toolkit.sha256` must match that executable. Verify the downloaded release assets themselves; a local or CI build is not a substitute for checking the exact published files. Build and deterministic verification commands are in the [README](https://github.com/Undecember/mcsr/blob/v0.1.0-beta.1/README.md#build-and-deterministic-checks). This project is licensed under [MIT](https://github.com/Undecember/mcsr/blob/v0.1.0-beta.1/LICENSE).

The Cargo package version remains `0.1.0`. This prerelease is identified by tag `v0.1.0-beta.1` and the exact executable SHA256 above; the tested binary is unchanged.
