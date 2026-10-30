# MCSR Toolkit

A local setup and keybind configurator for Minecraft speedrunning on Ubuntu GNOME Wayland. Its browser UI edits a saved profile, previews physical → emitted → Minecraft key mappings, and applies or restores instance files. A GNOME Shell extension reports focused Minecraft windows and resize shortcuts. New source builds install with live keyd enabled after readiness checks. Installation does not apply a profile; mappings require an applied profile and Minecraft focus. Use `install --observe-only` to opt out.

## Rust executable and release status

The supported product is the Rust executable. It includes the local service, browser assets, GNOME Shell 50 extension archive, guided installer, and narrowly scoped privileged helper modes. It requires Ubuntu GNOME Shell 50 on x86_64 Wayland; keyd is needed for keyboard integration, and maccel is optional for eye sensitivity. The extension UUID is `mcsr-toolkit@undecember.github.io`.

[`v0.1.0`](https://github.com/Undecember/mcsr/releases/tag/v0.1.0) is the first stable release. It includes local Standard Settings control synchronization with backup/Restore, borderless centered resize presets, the searchable control picker, keyboard/mouse input recording, and keyd uninstall recovery fixes. Candidate build and verification results are recorded in the [release checklist](docs/rust-release-checklist.md#v010-stable-candidate--2026-10-29); the release is published and both public downloads match the verified staged artifacts.

[**v0.2.0**](https://github.com/Undecember/mcsr/releases/tag/v0.2.0) is the latest stable release, adding managed keyd/F3 priority, windowed thin/full-size toggles, and conditional stock-panel hiding. Both release CI runs passed and both public downloads match the verified staged assets. Review its [release notes](docs/rust-release-notes.md) and [release checklist](docs/rust-release-checklist.md#v020-candidate--2026-10-30), including the remaining Alt+Tab, temporary F-key translation, and native-fullscreen limitations.

**v0.3.0 is in release preparation**, adding guided live prerequisites and automatic keyd recovery through `repair`. It is not yet published. See the [candidate checklist](docs/rust-release-checklist.md#v030-candidate--2026-10-30).

The stable designation does not replace live-test evidence. Earlier Rust checks passed for installation and repair, focus and remapping, recovery, and the Enter dual-role case. Live logout/login while remapped, lock/unlock and service crash/restart during active keyd remapping, guided uninstall with rollback and authentication prompt counts, and the latest picker and untested resize/lifecycle cases retain the limits disclosed in the [release notes](docs/rust-release-notes.md). Focus updates are asynchronous, so a brief interval can occur before a focus clear takes effect. Earlier Python results in the [Wayland acceptance record](integration/acceptance-wayland.md) are historical evidence and do not validate the Rust installer or service.

## Verify and install

Download `mcsr-toolkit` and its matching `mcsr-toolkit.sha256` from the same reviewed release candidate. Verify the checksum beside the executable before running it:

```sh
sha256sum --check mcsr-toolkit.sha256 && chmod +x ./mcsr-toolkit && ./mcsr-toolkit doctor
```

`doctor` reports prerequisites and unresolved recovery without changing keyboard configuration, sensitivity, or service state. Inspect its report before continuing.

A downloaded executable may be unable to verify an installed helper from a different version: the root status helper hash must match the executable running the check. If `doctor` reports a helper hash mismatch, run `/usr/local/bin/mcsr-toolkit doctor` from the matching installed version and review its recovery report. A mismatch, failed authentication, or malformed root status blocks installation and uninstall; keep the helper and policies available for recovery.

Install by running the verified executable:

```sh
./mcsr-toolkit install
```

The installer uses the executable being run as its artifact. It stages the four embedded extension files and an `install-pending.json` Shell-restart marker when GNOME does not yet recognize the new extension, then reports that setup is incomplete. Log out of GNOME Wayland, log back in, and run `./mcsr-toolkit repair` from the same verified executable. Wayland requires a new login to restart GNOME Shell; see GNOME's [extension development guide](https://gjs.guide/extensions/development/debugging.html#restarting-gnome-shell). Repeating `repair` in the same Shell session keeps installation staged.

After the new UUID is ACTIVE, `repair` tightens the toolkit data parent and Rust state directory to owner-only `0700`, preserving their contents and marker. It then disables the legacy `mcsr-toolkit@local` extension, verifies that its D-Bus interface is gone, enables the new UUID, and verifies ACTIVE. Only then does it start authenticated root setup and the user service. The installer verifies the installed executable and helper, exact sudoers policy, resolved systemd command, and local HTTP readiness before clearing the marker. A fresh host may prompt once to install the root-owned bootstrap executable and once for the fixed-target setup transaction. Staging the extension does not start root setup.

The installed executable is `/usr/local/bin/mcsr-toolkit`; the same file is copied to `/usr/local/libexec/mcsr-toolkit` for privileged helper modes. The root policies are `/etc/sudoers.d/mcsr-toolkit-rust-keyd` and `/etc/sudoers.d/mcsr-toolkit-rust-maccel`, embedded from [`packaging/mcsr-toolkit-rust-keyd.sudoers`](packaging/mcsr-toolkit-rust-keyd.sudoers) and [`packaging/mcsr-toolkit-rust-maccel.sudoers`](packaging/mcsr-toolkit-rust-maccel.sudoers). The default user unit runs `/usr/local/bin/mcsr-toolkit serve --enable-live-keyd`. `--observe-only` uses `/usr/local/bin/mcsr-toolkit serve`. Do not deploy Python helpers or create a Python-specific systemd override.

If setup is interrupted, keep `install-pending.json`, the extension lineage manifest, old extension files, and recovery records in place and run the reviewed `repair` workflow. A mismatch in an existing helper, policy, or extension file requires manual review; do not overwrite it to force installation. `repair` resumes interrupted setup and restores a stopped or disabled identical installation. The installer refuses an active legacy service or unresolved Python-era recovery. Python state, profiles, backups, and root recovery records remain available for migration and recovery; do not delete them to make preflight pass.

## Run and use the UI

Open <http://127.0.0.1:8765/> after installation. The service binds only to loopback. Live mode allows game bindings only with an applied profile and Minecraft focus. Observe-only mode reports focus and resize events without activating game bindings. The UI shows extension connection, discovered instances, keyd state, and recent bridge errors.

1. Choose **Refresh status**. If the launcher is not detected, enter its launcher root in **MCSRLauncher root override**. This is the directory containing `instances/`, not a per-instance game directory. MCSRLauncher normally stores `launcher/instances/<id>/instance.json` under its working directory.
2. Edit and save a profile. Minecraft controls come from `minecraft/options.txt`, with `.minecraft/options.txt` as a legacy fallback. The instance must have a matching `instance.json`. The editor supports Minecraft 1.16 keyboard identifiers, modifiers, mouse controls, and unbound controls while preserving other settings and line endings.
3. Choose **Preview changes** and review each physical → emitted → Minecraft action. Apply is enabled only when the saved profile has a current preview and every instance is ready or unchanged. If another tab saves a different profile, save and preview again after a `stale_profile` response.
4. Choose **Apply profile** and keep the returned backup ID. The UI also fills it into the restore field after a successful apply.
5. To undo the latest toolkit-managed file and keyd changes, enter its backup ID under **Restore a backup**. After applying A and then B, restore B first, then A. Minecraft must be closed; backups belong to their original launcher root. Wait for restore to succeed and confirm keyd health and ordinary typing before considering live activation.

Apply also synchronizes enabled control entries in existing local Standard Settings configs: `config/mcsr/standardsettings.json` (modern) and `config/standardoptions.txt` (legacy), relative to the instance's game directory. This prevents the mod from resetting toolkit controls when a new world is created. A profile with resize presets also prepares the local fullscreen setting for windowed mode, as described below. Disabled entries, blank control entries, unrelated settings, and global toggles are preserved. Missing configs stay absent; Standard Settings generates its initial config from the updated Minecraft options. Mod-only changes participate in Preview and wait for Minecraft to close before activation.

Shared configs are not modified. A `config/mcsr/standardsettings.global` reference or a legacy path redirect blocks Apply with a per-instance error because the mod would ignore local edits. To use toolkit-managed settings for such an instance, first configure it to use an independent local config. Unsafe paths and malformed configs also block Apply before any instance is changed.

Apply and Restore are serialized jobs. Keep the service running and do not edit the same instance files concurrently. New backups include local Standard Settings configs and Restore returns their exact original bytes. Restore rejects stale or already restored backups, changed options or mod configs (including added or removed config files), changed managed state or launcher instances, missing history, and inconsistent keyd snapshots. Older options-only backups remain restorable and do not change mod configs. Profiles, backups, and Python-era recovery records are retained on uninstall.

## Resize presets

Normal resize presets toggle the focused Minecraft window between the preset's centered, borderless game area and a full-size borderless window on its current monitor. For a B-bound thin preset, B goes thin → full-size → thin. The decision uses observed window geometry, so Alt+Tab or manual resizing does not leave a remembered toggle direction. Full-size follows the monitor's logical dimensions and position. The UI reports the last successful resize for the current focus session. Eye zoom and shifted eye shortcuts retain their fixed-size behavior; normal presets restore baseline sensitivity.

After a successful toolkit resize, the stock GNOME Shell 50 top panel temporarily hides while that managed borderless window is focused and overlaps it on the same monitor. This applies to thin, full-size, and fixed-size eye windows. Switching apps, opening the overview or a Shell modal, locking the session, moving out of overlap, resetting routing, or disabling the extension restores GNOME's normal panel visibility policy. An unmanaged Minecraft window does not hide the panel. Panel replacements are unsupported; unavailable Shell hooks leave panel control with GNOME and report a diagnostic. Supervised single-monitor XWayland checks passed for thin/full-size panel hiding, top-edge input, overview/app switching, and the Run dialog. Other panel lifecycle checks remain in the resize acceptance guide.

Both sizes use Minecraft's windowed mode. Applying a profile with resize presets prepares `fullscreen:false` in `options.txt` and in enabled, existing local Standard Settings entries. Preview identifies the changed files; edits wait for Minecraft to close and are covered by backup/Restore. Profiles without presets retain their previous settings behavior. Removing presets leaves the current fullscreen preference in place; use Restore to reverse the managed edits. Disabled or missing mod entries stay unchanged, missing mod configs stay absent, and shared-config redirects remain blocked.

Start or restart Minecraft after the windowed settings are applied. An already fullscreen client is refused before mutation; the toolkit does not force a fullscreen transition or repurpose F11. The supervised test found that native F11 and the game's menu could stop leaving fullscreen after a resize. The managed thin/full-size workflow avoids that transition; it does not establish a fix for the native fullscreen failure. For a windowed client, the extension clears maximization and verifies the observed borderless content/frame dimensions and target position. Compositor constraints can prevent an exact result, which is reported without entering actual fullscreen. Panel hiding begins only after a verified successful resize; it does not compensate for an incorrect size. Presets do not run automatically at launch.

For a decorated XWayland Minecraft client, borderless preparation requires `xprop` (provided by `x11-utils` on Ubuntu). Install that package if a preset reports that `xprop` is missing, then retry. Already undecorated native Wayland clients can be resized, but GNOME cannot verify the absence of client-side decorations from geometry alone; native Wayland acceptance remains unvalidated. Decorated native Wayland clients require client-side borderless support or an XWayland game. The toolkit does not switch the game backend. A client can also reject a requested size. The supervised XWayland test passed repeated 250×1080 ↔ 1920×1080 windowed toggles and focus recovery through GNOME's overview. It required temporary F13–F24 keyboard translation; Alt+Tab remained blocked by the Tab → F3 remap. See the [resize acceptance guide](integration/resize-wayland.md) for the exact scope and remaining checks. Both native/compositor fullscreen failures remain limitations.

## Managed keyd and F3 priority

Version 0.2.0 embeds a pinned, patched keyd daemon/client. Install offers to install the distribution keyd package if `keyd.service` is missing; a masked service requires review. The toolkit installs `/usr/local/libexec/mcsr-keyd` and selects it through `/etc/systemd/system/keyd.service.d/90-mcsr-toolkit.conf`; it preserves the distribution executable and service enablement. The authenticated helper records the original active state in `/var/lib/mcsr-toolkit/keyd-service.json` and restores it when the owned override is removed. Keep that record for repair or uninstall; foreign overrides and changed managed files require review.

Physical resize shortcuts now require live keyd mode and an applied profile. Saving a draft does not change active routing. Holding native F3, or a Crafting key mapped to F3 such as Tab, gives Minecraft priority for **F3+B**, **F3+G**, **F3+F**, **Shift+F3+F**, and **Shift+F3**. Ordinary resize and Shift eye zoom shortcuts resume after release. Typed command/chat input stays physical. This preserves B as a resize shortcut while allowing Tab+B to toggle hitboxes.

Internal resize signals use unused F13–F23 keys; F24 is reserved briefly as an ordered input-drain marker during configuration changes. Preview refuses an internal signal that conflicts with an observed Minecraft control. AltGr/rightalt resize shortcuts are currently rejected because their GNOME modifier translation is not established. Configuration, capability, or grab failures keep the game producer inactive and report an error. A failed consumer grab on focus regain also blocks activation until a successful retry.

The engine event tests, generated-routing tests, and sandbox installer tests do not establish live desktop acceptance. Verify all five combinations, chat, focus changes, and install/repair/uninstall recovery on the actual GNOME/Minecraft session before treating that workflow as validated. Source provenance, patch maintenance, and build details are in [the managed-keyd guide](packaging/keyd/README.md).

## Live keyd mode and recovery

New source builds enable live keyd by default after prerequisite setup and a stable live-status check. The installer offers authenticated distribution package installation, persistent membership in the privileged `keyd` socket group, and the `fkeys:basic_13-24` keyboard option. It preserves unrelated XKB options and input sources. `--yes` accepts these offers for unattended setup; authentication still uses desktop prompts. The published v0.2.0 executable predates this installer update.

If account membership changes but the desktop user service manager still has its old groups, installation runs a verified observe-only service and reports **setup pending: reboot required**. Reboot, then rerun `repair`. A failed GNOME routing handshake also tries observe-only fallback and retains a repair marker; inspect the reported error before retrying. A failed fallback uses the existing rollback and recovery workflow. Pending setup returns a nonzero exit status.

To explicitly choose observe-only mode:

```sh
./mcsr-toolkit repair --observe-only
```

This choice persists in the installed unit. A later plain `install` or `repair` requests live mode again. Existing applied profiles can resume under normal focus rules in live mode; the installer does not apply a new profile. Observe-only startup clears old live bindings and the persisted active profile; instance files and backup records remain available for Restore.

Original keyboard settings are stored once in `$XDG_DATA_HOME/mcsr-toolkit/rust-v1/keyboard-settings-before-install.json` (normally under `~/.local/share`). To restore the extended function-key setting, first complete `repair --observe-only`, then run:

```sh
./mcsr-toolkit restore-keyboard-settings
```

Restoration preserves current input sources and refuses to discard keyboard-option edits made after setup. The original backup remains available. Package installation and account group membership persist after toolkit uninstall.

`doctor` reports installation safety checks and live prerequisites separately. It reports live readiness only when a running service exposes healthy live status. Installation checks verify a stable live API with connected extension, available keyd, and clear recovery flags; these do not replace the Minecraft focus/chat acceptance procedure in the [Wayland acceptance record](integration/acceptance-wayland.md) and [focus probe](integration/keyd-focus-probe.md).

If status reports `mode: unsafe`, `unsafe_stop_failure: true`, or `recovery_clear_pending`, stop the toolkit unit and inspect keyd before continuing. For a stale `keyd-uncertain.json` record or a keyd restart-limit failure, run `mcsr-toolkit repair`. Repair stops the toolkit, takes its runtime lock, and requests administrator authentication for keyd recovery. The fixed helper validates the managed configuration and saved snapshots, resets keyd's restart limit, restarts the daemon, and verifies service and IPC health. Only after successful recovery does repair remove the unchanged uncertainty record and continue normal installation checks. Profiles and backups are preserved. A busy helper, unsafe configuration, unrelated recovery record, authentication failure, or failed health check leaves recovery blocked; do not manually delete records to bypass these checks. Ordinary focus cleanup does not reset the restart limit. This recovery requires the matching installed toolkit helper and managed keyd; mismatched or untrusted installed files still require review.

Do not claim mappings are inactive until keyd state and ordinary input are checked. If keyd cannot recover because `/etc/keyd` contains unsafe or conflicting configuration, restore it to a known safe configuration before using Restore, because Restore first resets keyd bindings and requires a working daemon. Keep the toolkit stopped if safe ordinary typing cannot be confirmed. For detailed helper recovery behavior and observed limits, see the [Wayland acceptance record](integration/acceptance-wayland.md) and [focus probe](integration/keyd-focus-probe.md).

To return to observe-only mode before repair or reinstall, first confirm the live service has no unresolved keyd or maccel recovery and that normal input works. Inspect the unit's drop-ins and resolved command:

```sh
systemctl --user show -P DropInPaths mcsr-toolkit.service
systemctl --user cat mcsr-toolkit.service
```

The Rust installer refuses every service drop-in, including a legacy live-mode override. If upgrading from a manual override, stop the unit and remove only a reviewed drop-in containing solely the toolkit `ExecStart` override; for the standard `override.conf` location:

```sh
set -e
systemctl --user stop mcsr-toolkit.service
rm -- "$HOME/.config/systemd/user/mcsr-toolkit.service.d/override.conf"
systemctl --user daemon-reload
./mcsr-toolkit repair --observe-only
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

Uninstall checks user and root recovery before removal and keeps the helper if keyd or maccel recovery is uncertain. For managed-keyd builds it removes only the verified owned binary and override, reloads the system service configuration, and restores the recorded original keyd active state. It leaves the distribution executable and service enablement intact. Restore toolkit-managed state first and verify the original keyd configuration and maccel baseline. Historical keyd snapshots and maccel lock/generation records are safe to retain after successful Restore. Uninstall preserves user profiles, backups, and Python-era recovery data. It removes the service unit but does not remove user service drop-ins. Before uninstall, record and review the drop-in paths; after successful uninstall, manually remove only the reviewed toolkit live-mode drop-in and run `systemctl --user daemon-reload`. A leftover drop-in blocks a later Rust install/repair and may refer to the removed executable. Guided uninstall with rollback and authentication prompt counts has not yet been tested on the supervised Rust device; beta users should review the [checklist](docs/rust-release-checklist.md) before trying it and report issues through the [GitHub issue tracker](https://github.com/Undecember/mcsr/issues).

## Build and deterministic checks

On native Ubuntu x86_64 with Rust, Node.js, Python 3, a C compiler, `patch`, and `glib-compile-schemas` available, build and check the candidate:

```sh
cargo fmt --check
cargo clippy --all-targets --locked -- -D warnings
cargo test --locked
python3 packaging/keyd/test.py
node tests/extension_harness.cjs
node tests/ui_harness.cjs
node integration/live-ui-acceptance.mjs self-test
glib-compile-schemas --strict --dry-run extension/schemas
cargo build --release --locked
cargo test --release --locked --test rust_assets
```

The release executable embeds the browser assets, extension archive, and patched keyd binary. Python is used for development engine tests; the installed executable has no Python runtime dependency. From the repository root, create and verify a checksum beside the local candidate:

```sh
(cd target/release && sha256sum mcsr-toolkit > mcsr-toolkit.sha256 && sha256sum --check mcsr-toolkit.sha256)
```

A local build or CI artifact is not the final published artifact; verify the exact downloaded release files separately. CI retains candidate artifacts and does not publish a release.

The project source is at [GitHub](https://github.com/Undecember/mcsr) and is licensed under [MIT](LICENSE).
