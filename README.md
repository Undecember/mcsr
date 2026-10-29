# MCSR Toolkit

A local setup and keybind configurator for Minecraft speedrunning on Ubuntu GNOME Wayland. Its browser UI edits a saved profile, previews physical → emitted → Minecraft key mappings, and applies or restores instance files. A GNOME Shell extension reports focused Minecraft windows and resize shortcuts. The installed service starts in observe-only mode; live keyd activation is opt-in.

## Rust executable and beta status

The supported product is the Rust executable. It includes the local service, browser assets, GNOME Shell 50 extension archive, guided installer, and narrowly scoped privileged helper modes. It requires Ubuntu GNOME Shell 50 on x86_64 Wayland; keyd is needed for keyboard integration, and maccel is optional for eye sensitivity. The extension UUID is `mcsr-toolkit@undecember.github.io`.

The Rust source cutover is complete. `v0.1.0-beta.2` is a public beta; some supervised cases remain untested and are disclosed in the [release notes](docs/rust-release-notes.md). Earlier Rust live checks passed for installation and repair, focus and remapping, recovery, and the Enter dual-role case; beta.2's control picker and input recorder have automated coverage but have not been tested on the supervised Minecraft/GNOME device. A live logout/login while remapped, lock/unlock and service crash/restart during active keyd remapping, and guided uninstall with rollback and authentication prompt counts have not been tested. Focus updates are asynchronous, so a brief interval can occur before a focus clear takes effect. Earlier Python live results in the [Wayland acceptance record](integration/acceptance-wayland.md) are historical evidence and do not validate the Rust installer or service. See the [release checklist](docs/rust-release-checklist.md) for the detailed results and deferred checks.

## Verify and install

Download `mcsr-toolkit` and its matching `mcsr-toolkit.sha256` from the same reviewed release candidate. Verify the checksum beside the executable before running it:

```sh
sha256sum --check mcsr-toolkit.sha256 && chmod +x ./mcsr-toolkit && ./mcsr-toolkit doctor
```

`doctor` reports prerequisites and unresolved recovery without changing keyboard configuration, sensitivity, or service state. Inspect its report before continuing. Install by running the verified executable:

```sh
./mcsr-toolkit install
```

The installer uses the executable being run as its artifact. It stages the four embedded extension files and an `install-pending.json` Shell-restart marker when GNOME does not yet recognize the new extension, then reports that setup is incomplete. Log out of GNOME Wayland, log back in, and run `./mcsr-toolkit repair` from the same verified executable. Wayland requires a new login to restart GNOME Shell; see GNOME's [extension development guide](https://gjs.guide/extensions/development/debugging.html#restarting-gnome-shell). Repeating `repair` in the same Shell session keeps installation staged.

After the new UUID is ACTIVE, `repair` tightens the toolkit data parent and Rust state directory to owner-only `0700`, preserving their contents and marker. It then disables the legacy `mcsr-toolkit@local` extension, verifies that its D-Bus interface is gone, enables the new UUID, and verifies ACTIVE. Only then does it start authenticated root setup and the user service. The installer verifies the installed executable and helper, exact sudoers policy, resolved systemd command, and local HTTP readiness before clearing the marker. A fresh host may prompt once to install the root-owned bootstrap executable and once for the fixed-target setup transaction. Staging the extension does not start root setup.

The installed executable is `/usr/local/bin/mcsr-toolkit`; the same file is copied to `/usr/local/libexec/mcsr-toolkit` for privileged helper modes. The root policies are `/etc/sudoers.d/mcsr-toolkit-rust-keyd` and `/etc/sudoers.d/mcsr-toolkit-rust-maccel`, embedded from [`packaging/mcsr-toolkit-rust-keyd.sudoers`](packaging/mcsr-toolkit-rust-keyd.sudoers) and [`packaging/mcsr-toolkit-rust-maccel.sudoers`](packaging/mcsr-toolkit-rust-maccel.sudoers). The user unit runs `/usr/local/bin/mcsr-toolkit serve` in observe-only mode. Do not deploy Python helpers or create a Python-specific systemd override.

If setup is interrupted, keep `install-pending.json`, the extension lineage manifest, old extension files, and recovery records in place and run the reviewed `repair` workflow. A mismatch in an existing helper, policy, or extension file requires manual review; do not overwrite it to force installation. `repair` resumes interrupted setup and restores a stopped or disabled identical installation. The installer refuses an active legacy service or unresolved Python-era recovery. Python state, profiles, backups, and root recovery records remain available for migration and recovery; do not delete them to make preflight pass.

## Run and use the UI

Open <http://127.0.0.1:8765/> after installation. The service binds only to loopback. Its observe-only default reports focus and resize events but does not activate keyd game bindings. The UI shows extension connection, discovered instances, keyd state, and recent bridge errors.

1. Choose **Refresh status**. If the launcher is not detected, enter its launcher root in **MCSRLauncher root override**. This is the directory containing `instances/`, not a per-instance game directory. MCSRLauncher normally stores `launcher/instances/<id>/instance.json` under its working directory.
2. Edit and save a profile. Minecraft controls come from `minecraft/options.txt`, with `.minecraft/options.txt` as a legacy fallback. The instance must have a matching `instance.json`. The editor supports Minecraft 1.16 keyboard identifiers, modifiers, mouse controls, and unbound controls while preserving other settings and line endings.
3. Choose **Preview changes** and review each physical → emitted → Minecraft action. Apply is enabled only when the saved profile has a current preview and every instance is ready or unchanged. If another tab saves a different profile, save and preview again after a `stale_profile` response.
4. Choose **Apply profile** and keep the returned backup ID. The UI also fills it into the restore field after a successful apply.
5. To undo the latest toolkit-managed file and keyd changes, enter its backup ID under **Restore a backup**. After applying A and then B, restore B first, then A. Minecraft must be closed; backups belong to their original launcher root. Wait for restore to succeed and confirm keyd health and ordinary typing before considering live activation.

Apply and Restore are serialized jobs. Keep the service running and do not edit the same instance files concurrently. Restore rejects stale or already restored backups, changed options or managed state, changed launcher instances, missing history, and inconsistent keyd snapshots. Profiles, backups, and Python-era recovery records are retained on uninstall.

## Live keyd mode and recovery

Live keyd activation requires the supervised focus/chat safety procedure in the [Wayland acceptance record](integration/acceptance-wayland.md) and [focus probe](integration/keyd-focus-probe.md), plus a successful helper and sudoers preflight. Do not enable live mode until those checks are accepted. When authorized, stop the managed unit before changing its mode, then use a systemd user drop-in with this exact command:

```ini
[Service]
ExecStart=
ExecStart=/usr/local/bin/mcsr-toolkit serve --enable-live-keyd
```

Reload and restart the unit, then verify its status. The flag is a local process setting and cannot be enabled through HTTP. Do not run a second service copy beside the installed unit. Restarting in observe-only mode clears old live bindings once and drops the persisted active profile without activating bindings. Restore remains available without the flag.

If status reports `mode: unsafe`, `unsafe_stop_failure: true`, or `recovery_clear_pending`, stop the toolkit unit and inspect keyd before continuing. Do not start live mode or claim mappings are inactive until keyd state and ordinary input are checked. If keyd is stopped or unhealthy, recover `/etc/keyd` to a known safe configuration before using Restore, because Restore first resets keyd bindings and requires a working daemon. Keep the toolkit stopped if safe ordinary typing cannot be confirmed. For detailed helper recovery behavior and observed limits, see the [Wayland acceptance record](integration/acceptance-wayland.md) and [focus probe](integration/keyd-focus-probe.md).

To return to observe-only mode before repair or reinstall, first confirm the live service has no unresolved keyd or maccel recovery and that normal input works. Inspect the unit's drop-ins and resolved command:

```sh
systemctl --user show -P DropInPaths mcsr-toolkit.service
systemctl --user cat mcsr-toolkit.service
```

The Rust installer refuses every service drop-in, including a live-mode override. Stop the unit. Remove only the reviewed live-mode drop-in if its file contains only the `ExecStart` override shown above; for the standard `override.conf` location:

```sh
set -e
systemctl --user stop mcsr-toolkit.service
rm -- "$HOME/.config/systemd/user/mcsr-toolkit.service.d/override.conf"
systemctl --user daemon-reload
systemctl --user start mcsr-toolkit.service
systemctl --user status mcsr-toolkit.service --no-pager
systemctl --user show -P ExecStart mcsr-toolkit.service
```

If the drop-in path differs, the file contains other settings, or another drop-in is listed, review it manually and preserve unrelated settings; do not remove the whole directory or use `systemctl revert`. After restart, confirm the resolved command is exactly `/usr/local/bin/mcsr-toolkit serve`, the UI reports observe-only mode with no recovery-clear error, keyd is healthy, and ordinary typing works. The observe-only startup clears a persisted live profile once. Only after those checks pass should you run `doctor`, `repair`, or `install`. If input or recovery state is uncertain, keep the toolkit stopped and complete keyd recovery before repair or reinstall.

## Repair and uninstall

Run the matching verified executable from its download location:

```sh
./mcsr-toolkit doctor
./mcsr-toolkit repair
./mcsr-toolkit uninstall
```

Use `repair` for an interrupted install, a retained install marker, or a stopped/disabled identical installation. Do not remove the marker, helper, policy files, extension lineage manifest, or recovery evidence to bypass a refusal. If an uninstall is interrupted, preserve its marker and helper and use the reviewed repair workflow before retrying.

Uninstall checks user and root recovery before removal and keeps the helper if keyd or maccel recovery is uncertain. Restore toolkit-managed state first and verify the original keyd configuration and maccel baseline. Historical keyd snapshots and maccel lock/generation records are safe to retain after successful Restore. Uninstall preserves user profiles, backups, and Python-era recovery data. It removes the service unit but does not remove user service drop-ins. Before uninstall, record and review the drop-in paths; after successful uninstall, manually remove only the reviewed toolkit live-mode drop-in and run `systemctl --user daemon-reload`. A leftover drop-in blocks a later Rust install/repair and may refer to the removed executable. Guided uninstall with rollback and authentication prompt counts has not yet been tested on the supervised Rust device; beta users should review the [checklist](docs/rust-release-checklist.md) before trying it and report issues through the [GitHub issue tracker](https://github.com/Undecember/mcsr/issues).

## Build and deterministic checks

On Ubuntu x86_64 with Rust, Node.js, and `glib-compile-schemas` available, build and check the candidate:

```sh
cargo fmt --check
cargo clippy --all-targets --locked -- -D warnings
cargo test --locked
node tests/extension_harness.cjs
node tests/ui_harness.cjs
node integration/live-ui-acceptance.mjs self-test
glib-compile-schemas --strict --dry-run extension/schemas
cargo build --release --locked
cargo test --release --locked --test rust_assets
```

The release executable embeds the browser assets and extension archive and has no Python runtime dependency. From the repository root, create and verify a checksum beside the local candidate:

```sh
(cd target/release && sha256sum mcsr-toolkit > mcsr-toolkit.sha256 && sha256sum --check mcsr-toolkit.sha256)
```

A local build or CI artifact is not the final published artifact; verify the exact downloaded release files separately. CI retains candidate artifacts and does not publish a release.

The project source is at [GitHub](https://github.com/Undecember/mcsr) and is licensed under [MIT](LICENSE).
