# MCSR Toolkit v0.1.0

MCSR Toolkit v0.1.0 is the first stable release, distributed as a single Rust executable for Minecraft speedrunning on Ubuntu GNOME Wayland. It includes the loopback browser service, UI assets, GNOME Shell 50 extension, guided setup and recovery commands, and narrowly scoped privileged helper modes. The [stable release](https://github.com/Undecember/mcsr/releases/tag/v0.1.0) is published.

## New since beta.3

- Synchronize enabled controls in existing local Standard Settings configs, including modern JSON and legacy text files, so world creation does not undo toolkit mappings. Apply and Restore cover the mod files alongside Minecraft options, preserve disabled and unrelated settings, and defer changes while Minecraft is running. Shared configs are unchanged; redirects produce an actionable error.
- Make resize presets borderless and centered for windowed Minecraft clients, with observed size and position checks. Fullscreen requests are refused until Minecraft leaves fullscreen through its own control. XWayland decoration removal requires `xprop`; native Wayland decoration removal depends on client support.

The searchable control picker, keyboard/mouse input recorder, expanded bindings, and keyd uninstall recovery and diagnostic fixes from the beta releases remain included.

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

If install stages a GNOME extension and reports that setup is incomplete, log out of GNOME Wayland, log back in, then run `./mcsr-toolkit repair` from the same verified executable. For interrupted setup or uninstall, use `doctor` and the documented recovery guidance in the [README](https://github.com/Undecember/mcsr/blob/v0.1.0/README.md#repair-and-uninstall). Review recovery state before continuing; do not delete markers or recovery records to bypass a refusal.

A downloaded executable must match the installed root helper before it can verify protected recovery state. If `doctor` reports a helper hash mismatch, run `/usr/local/bin/mcsr-toolkit doctor` from the matching installed version to inspect recovery. This diagnostic does not update the installation or authorize overwriting the helper; a mismatched existing helper still requires manual review.

To uninstall, first restore toolkit-managed keyd and maccel state and verify ordinary input, then run the matching installed `/usr/local/bin/mcsr-toolkit uninstall`. Older installed uninstallers retain their original behavior until updated; downloading this candidate alone does not fix them. Review the README's [uninstall and drop-in guidance](https://github.com/Undecember/mcsr/blob/v0.1.0/README.md#repair-and-uninstall) before removing user service overrides.

## Known limitations and live-test coverage

Earlier supervised Rust checks passed for installation and repair, browser Save/Preview/Apply/Restore, live Backward remapping, the Enter dual-role chat case, focus clearing, and recovery to recorded keyd, sensitivity, and Minecraft options baselines. Those checks predate several current features; automated tests and the stable designation do not establish additional live acceptance. Detailed evidence is retained in the [Wayland acceptance record](https://github.com/Undecember/mcsr/blob/v0.1.0/integration/acceptance-wayland.md).

- Live GNOME logout/login while remapped, and lock/unlock or service crash/restart during active keyd remapping, remain untested. Eye sensitivity recovery was observed in observe-only mode.
- Successful guided uninstall with rollback and authentication prompt counts has not been validated on the supervised Rust device. Source fixes address the reproduced missing-keyd-client and empty-configuration reset failures. An older installed executable retains its old behavior until upgraded.
- A separate installed-binary new-instance Restore cycle, the latest picker/input recorder, and Standard Settings synchronization have not been exercised on the supervised Minecraft/GNOME device. They have automated coverage.
- The supervised XWayland resize primitive verified borderless size and centering. The corrected candidate lifecycle and shortcut-through-service path still need live acceptance; native Wayland acceptance is also pending. An earlier compositor-only fullscreen restoration stranded Minecraft until recovery through its own fullscreen control. Current code refuses fullscreen transitions. See the [resize acceptance record](https://github.com/Undecember/mcsr/blob/v0.1.0/integration/resize-wayland.md).
- Focus changes are asynchronous; a brief interval may occur before keyd bindings clear. The checks do not establish zero input leakage during focus transitions.

Report issues through the [GitHub issue tracker](https://github.com/Undecember/mcsr/issues).

## Candidate verification and artifact identity

Automated checks and local artifact identity are recorded in the [release checklist](https://github.com/Undecember/mcsr/blob/v0.1.0/docs/rust-release-checklist.md#v010-stable-candidate--2026-10-29). The annotated tag resolves to release commit `35a37b825ff34326294a89c0bdcf4f8a6e04900a`.

The candidate starts from development commit `a62271da0439a1a87eca881f4927ac98af5e2006`. Release preparation removes one redundant reference in `src/apply.rs` to satisfy Clippy; the fixture self-test also signals readiness after registering its shutdown handler to eliminate a test race. Neither adjustment changes application behavior. The release checklist records the exact source-input fingerprint so the release preparation delta is not confused with the development base commit.

| Item | Value |
| --- | --- |
| Development base | `a62271da0439a1a87eca881f4927ac98af5e2006` |
| Executable SHA256 | `2c55101368ca33fcd0b7099e06cef51caf49b985cbf4ab71e3f8afcf84a71c73` |
| Executable size | `12,817,512` bytes |
| Source-input fingerprint | `63cae1f76980cfaa85106141440ec9920c8459990c563fea8bb2705d16a03bbd` |
| Build environment | Ubuntu 26.04.1 LTS, `x86_64-unknown-linux-gnu`; Rust/Cargo 1.98.1, Node.js 24.21.0, GLib 2.88.0 |

The verified executable and matching checksum are staged under `target/release-preparation/v0.1.0/` in the primary checkout. The build-input manifest is beside that directory as `v0.1.0-source-inputs.sha256`. All dynamic libraries resolve locally; the highest required GLIBC symbol version is `2.39`. Compatibility with other Ubuntu versions has not been tested.

Publish exactly `mcsr-toolkit` and `mcsr-toolkit.sha256` as stable release assets, with GitHub's prerelease flag disabled. Verify both downloaded public files after publication; local candidate checks do not establish public download identity. Build commands are in the [README](https://github.com/Undecember/mcsr/blob/v0.1.0/README.md#build-and-deterministic-checks). This project is licensed under [MIT](https://github.com/Undecember/mcsr/blob/v0.1.0/LICENSE).

Cargo package version `0.1.0` already matches the stable tag `v0.1.0`; no beta suffix or prerelease designation applies to this version.
