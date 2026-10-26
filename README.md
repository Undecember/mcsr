# MCSR Toolkit

A local setup and keybind configurator for Minecraft speedrunning on Ubuntu GNOME Wayland. The browser UI edits a saved profile, previews the physical → emitted → Minecraft key mapping, and applies or restores instance files. A GNOME Shell extension reports focused Minecraft windows and resize shortcuts. keyd activation is opt-in and starts disabled.

## Rust single-binary installer candidate

The Rust executable now includes `doctor`, `install`, `repair`, and `uninstall` modes. The installer targets Ubuntu GNOME Shell 50 on x86_64 Wayland. `doctor` reports prerequisites and unresolved recovery without changing keyboard configuration, sensitivity, or service state; checking existing root recovery records may request interactive authentication and recreate a volatile keyd lock after reboot. `install` uses the executable being run as its artifact. On a first install with a new GNOME extension, it writes the four embedded extension files and an `install-pending.json` Shell restart marker, then exits with an incomplete-install message. **Log out of GNOME Wayland, log back in, then run `mcsr-toolkit repair`.** Repeating `repair` in the same Shell session leaves the installation staged without starting a new root setup or user service. GNOME's [extension development guide](https://gjs.guide/extensions/development/debugging.html#restarting-gnome-shell) explains why Wayland requires logout/login to restart Shell. After the new Shell recognizes the UUID, `repair` first tightens the toolkit data parent and Rust state directory to owner-only `0700`, preserving their contents and the pending marker. It then disables an enabled legacy `mcsr-toolkit@local`, waits for it to become INACTIVE and stop exporting the `org.mcsr.Toolkit` interface at `/org/mcsr/Toolkit` on `org.gnome.Shell`, enables the new UUID, and verifies ACTIVE. Only then does it start interactive `pkexec` for root-owned copies and exact sudoers rules. It verifies the helper copy, systemd's resolved `ExecStart`, and local HTTP readiness before removing the marker and reporting success. The installed unit runs `/usr/local/bin/mcsr-toolkit serve` in observe-only mode; live keyd remains a separate opt-in setting. Legacy extension files remain on disk for recovery. Existing extension files that differ without a matching installer lineage manifest require manual review. `repair` also resumes interrupted root setup and restores a stopped or disabled identical installation. `uninstall` keeps the helper if keyd or maccel recovery is uncertain. Historical keyd snapshots and maccel's lock and generation record remain safe to retain after Restore. User profiles, backups, and Python recovery records are retained.

The handoff marker records the prior UUID configuration before `@local` is disabled, so an interrupted `repair` can restore it. If the new extension is already in Shell State ERROR, `repair` records whether `@local` was enabled, disables `@local`, and configures only the new UUID for the next login. It leaves root setup unstarted and asks for another logout/login. After the new Shell starts, run `mcsr-toolkit repair` again: it requires the new UUID to be ACTIVE before service cutover. If that check fails, it tries to restore `@local` and retains the marker. A `handoff-uncertain` marker blocks automatic repair until the conflicting UUID states are reviewed. If automatic restoration is uncertain, inspect `gnome-extensions info` for both UUIDs; disable the new UUID, enable `mcsr-toolkit@local`, and verify `@local` is ACTIVE before another attempt. Keep the marker, old extension files, and reviewed helper for recovery. Do not leave both UUIDs enabled at logout. A failed service start after a clean handoff also restores the previous extension configuration and leaves a repair marker.

The Rust installer embeds its own [keyd](packaging/mcsr-toolkit-rust-keyd.sudoers) and [maccel](packaging/mcsr-toolkit-rust-maccel.sudoers) sudoers templates. The Python setup below uses separate legacy templates and helper paths until supervised cutover.

For a guided Rust install or repair, the installer checks protected root recovery and exact installed policy state in one authenticated status call when either exists. Extension staging does not start the authenticated root setup transaction. On the later `repair`, the installer holds one authenticated, fixed-target setup transaction while the user service starts and becomes HTTP ready. A fresh system also needs one authenticated copy of the executable into `/usr/local/libexec/mcsr-toolkit`; the installer verifies that copy's root ownership, mode, path, and SHA256 before executing it. Existing helper or policy mismatches require manual review. If setup or service readiness fails, `install-pending.json` remains for `repair`; do not delete that marker, the extension lineage manifest, or historical keyd/maccel records. The setup helper removes root files created during an aborted transaction; a verified bootstrap helper may remain to support repair. The sudoers permissions remain limited to keyd `clear` and maccel `eye`/`restore`.

Extension staging records the exact prior file fingerprints before replacement and keeps the prior lineage manifest through the staging phase. Post-login setup writes the new manifest. Without a prior manifest, staging accepts any combination of absent files and files matching the embedded bytes and modes; a differing present file requires manual review. If staging is interrupted, `repair` accepts only verified prior bytes or the embedded replacement bytes at each path, then finishes staging and requests a fresh Shell restart. An unrelated file change requires manual review. The installer uses `/usr/bin/gdbus` to identify the `org.gnome.Shell` session-bus owner and verifies that process before deciding whether Shell restarted; an unavailable or changing owner blocks the transition to root setup.

Uninstall also uses one authenticated root transaction after stopping and checking the user service. The root helper checks keyd and maccel recovery again before staging fixed policy and executable targets and before committing removal; it restores staged files if user cleanup aborts. The helper remains available until commit finishes. An interrupted uninstall keeps `install-pending.json` and the helper when possible; run the reviewed repair workflow before retrying removal. A literal desktop login of `ALL` is refused because it is a sudoers keyword.

If root removal completed but its final acknowledgement or marker cleanup failed, `repair` can clear the exact uninstall marker after verifying that every fixed root target and user integration is absent. It does not require the removed helper or delete historical root recovery data. A partial uninstall keeps the marker for manual review.

The project source is at [GitHub](https://github.com/Undecember/mcsr) and is licensed under [MIT](LICENSE). The embedded extension UUID is `mcsr-toolkit@undecember.github.io`. This is a release candidate; the supervised Rust desktop acceptance in [the release checklist](docs/rust-release-checklist.md) remains open. The earlier Python live results in [the Wayland acceptance record](integration/acceptance-wayland.md) do not validate the Rust installer or service.

To reproduce the local release build on Ubuntu x86_64 with Rust, Node.js, and `glib-compile-schemas` available:

```sh
cargo fmt --check
cargo clippy --all-targets --locked -- -D warnings
cargo test --locked
node tests/extension_harness.cjs
node tests/ui_harness.cjs
glib-compile-schemas --strict --dry-run extension/schemas
cargo build --release --locked
cargo test --release --locked --test rust_assets
(cd target/release && sha256sum mcsr-toolkit > mcsr-toolkit.sha256 && sha256sum --check mcsr-toolkit.sha256)
```

`target/release/mcsr-toolkit` is the one executable to distribute with `target/release/mcsr-toolkit.sha256`. Run `sha256sum --check mcsr-toolkit.sha256` beside a downloaded binary before the separately authorized installation. The binary embeds the browser assets and extension archive; it needs no Python or Rust interpreter at runtime. GNOME Shell 50, keyd for keyboard integration, and maccel for eye sensitivity remain external system requirements. CI retains the candidate executable and checksum as workflow artifacts; it does not publish a release.

Rust `install` and `repair` have been tested on the target Ubuntu GNOME Wayland desktop; `uninstall` and the remaining supervised acceptance cases are still pending. The Python setup below remains the installation reference until that cutover is complete. Its systemd unit instructions include a Python-specific `ExecStart` override because the packaged unit now describes the Rust service. The Rust installer refuses any service drop-in, including that documented Python override; review and remove it during supervised cutover after Python recovery is complete.

## Install the Python package

Use Python 3.11 or newer. On Ubuntu, install the developer tools and keyd package (keyd is needed only for keybind integration):

```sh
sudo apt update
sudo apt install python3-venv keyd
```

From the repository root, create a virtual environment and install the package and development tools:

```sh
python3 -m venv .venv
.venv/bin/python -m pip install --upgrade pip
.venv/bin/python -m pip install -e '.[dev]'
```

If `python3 -m venv` reports that `ensurepip` is missing, install the matching Ubuntu `python3-venv` package. Keep the repository and virtual environment in a stable location; the user service needs the absolute path to `.venv/bin/mcsr-toolkit`.

## Install and enable the GNOME extension

The extension is packaged for GNOME Shell 50. Build and install the user extension from the repository root:

```sh
mkdir -p /tmp/mcsr-toolkit-extension
glib-compile-schemas --strict --dry-run extension/schemas
gnome-extensions pack --force --out-dir=/tmp/mcsr-toolkit-extension extension
gnome-extensions install --force /tmp/mcsr-toolkit-extension/mcsr-toolkit@undecember.github.io.shell-extension.zip
```

Log out and back into the GNOME Wayland session if the shell does not discover the newly installed UUID. Then enable it and check its state:

```sh
gnome-extensions enable mcsr-toolkit@undecember.github.io
gnome-extensions info mcsr-toolkit@undecember.github.io
```

The expected state is `ACTIVE`. The service creates `$XDG_RUNTIME_DIR/mcsr-toolkit-focus.fifo`; the extension retries when the service is not running and does not create a regular file in place of the FIFO.

## keyd access and helper setup

The desktop login that runs the service needs access to the keyd socket. Check group membership with `id -nG`; if `keyd` is absent, add the login to that group and log out and back in so GNOME and the user service receive the new group list:

```sh
sudo usermod -aG keyd "$USER"
```

`PrivilegedKeydAdapter` invokes `/usr/bin/pkexec /usr/local/libexec/mcsr-toolkit-keyd` for install and restore, with interactive authentication. The helper is a separately deployed, root-owned executable copy of `src/mcsr_toolkit/privileged.py`; its file and ancestors must not be writable by other users. Its `#!/usr/bin/python3 -I` shebang runs an isolated interpreter that imports only the standard library. Polkit authorization therefore identifies the helper itself. The helper uses fixed `/etc/keyd/` targets and `/var/lib/mcsr-toolkit/keyd/` backups, rejects conflicting keyboard configurations, and restarts keyd after installation or recovery. Ubuntu keyd 2.5 has no configuration `check` command, so the helper validates only the restricted chat-layer grammar.

Deploy the reviewed helper from the repository root when installation is authorized:

```sh
sudo install -d -o root -g root -m 0755 /usr/local/libexec
sudo install -o root -g root -m 0755 src/mcsr_toolkit/privileged.py /usr/local/libexec/mcsr-toolkit-keyd
sha256sum src/mcsr_toolkit/privileged.py /usr/local/libexec/mcsr-toolkit-keyd
```

Before relying on an installed helper for this source version, confirm that the hashes match. Install the [sudoers template](packaging/mcsr-toolkit-keyd.sudoers) for the exact desktop login that owns the user service. The candidate and final rule are root-owned and mode `0440`; replace the placeholder with that login name:

**Deployment status (2026-10-06):** after separate approval, the current helper source was installed with `pkexec /usr/bin/install` using an absolute source path. The source and `/usr/local/libexec/mcsr-toolkit-keyd` SHA256 values both equal `924cb9c10b14f9b9a50f4acbe062d45fa9d093c5efbe037042bd2485e93ce339`; the installed file is `root:root`, mode `0755`. Repeat the hash and ownership checks after any helper source change before relying on the installed copy.

```sh
sudo install -o root -g root -m 0440 packaging/mcsr-toolkit-keyd.sudoers /root/mcsr-toolkit-keyd.sudoers.candidate
sudoedit /root/mcsr-toolkit-keyd.sudoers.candidate
sudo visudo -cf /root/mcsr-toolkit-keyd.sudoers.candidate
sudo install -o root -g root -m 0440 /root/mcsr-toolkit-keyd.sudoers.candidate /etc/sudoers.d/mcsr-toolkit-keyd
sudo visudo -cf /etc/sudoers.d/mcsr-toolkit-keyd
sudo rm /root/mcsr-toolkit-keyd.sudoers.candidate
```

The rule must be exactly `<desktop-user> ALL=(root) NOPASSWD: /usr/local/libexec/mcsr-toolkit-keyd clear`, with no wildcard or additional command/arguments. In a terminal logged in as the desktop service owner, check the refreshed groups and permission without executing `clear`:

```sh
id -nG
sudo -n -l /usr/local/libexec/mcsr-toolkit-keyd clear
```

`preflight_keyd_clear()` also verifies that the helper is root-owned and executable and its path has no writable or symlink components. It does not restart keyd. The service clears state using `keyd bind reset` followed by `/usr/bin/sudo -n /usr/local/libexec/mcsr-toolkit-keyd clear`. That passwordless rule authorizes only the exact `clear` command; install and restore use authenticated `pkexec`. Profile keys use config-safe lowercase letters, digits, and underscores from keyd's vocabulary. Punctuation aliases such as `#` are rejected because they carry config syntax.

Do not remove `/var/lib/mcsr-toolkit/keyd/` backups or the user's `keyd-adapter.json` recovery state until the original keyd configuration has been restored and checked. To remove the passwordless permission after recovery, stop the service, remove only its dedicated sudoers file, then validate the remaining policy:

```sh
systemctl --user stop mcsr-toolkit.service
sudo rm /etc/sudoers.d/mcsr-toolkit-keyd
sudo visudo -c
```

The helper binary can be removed separately after toolkit recovery is complete:

```sh
sudo rm /usr/local/libexec/mcsr-toolkit-keyd
```

## Start the user service

The [user unit](packaging/mcsr-toolkit.service) starts in observe-only mode. Install it and set its executable path to this checkout's virtual environment. Replace `/absolute/path/to/mcsr` with the repository's actual path:

```sh
mkdir -p ~/.config/systemd/user
install -m 0644 packaging/mcsr-toolkit.service ~/.config/systemd/user/mcsr-toolkit.service
systemctl --user edit mcsr-toolkit.service
```

In the editor, add this drop-in and replace the path with the absolute checkout path:

```ini
[Service]
Environment="PATH=/absolute/path/to/mcsr/.venv/bin:%h/.local/bin:/usr/local/bin:/usr/bin"
ExecStart=
ExecStart=/absolute/path/to/mcsr/.venv/bin/mcsr-toolkit
```

Save the drop-in, then enable and start the service:

```sh
systemctl --user daemon-reload
systemctl --user enable --now mcsr-toolkit.service
systemctl --user status mcsr-toolkit.service --no-pager
```

Open <http://127.0.0.1:8765/> in a browser. The service binds only to loopback. Its default observe-only mode keeps focus and resize observation running but does not activate keyd game bindings. Check the UI status for the extension connection, discovered instances, keyd state, and recent bridge errors.

Live keyd activation requires the supervised focus/chat safety procedure recorded in [the Wayland acceptance record](integration/acceptance-wayland.md) and [the focus probe](integration/keyd-focus-probe.md). Once that proof is accepted and the helper/sudoers preflight succeeds, an operator can deliberately enable live mode with a user unit drop-in:

```ini
[Service]
ExecStart=
ExecStart=/usr/local/bin/mcsr-toolkit serve --enable-live-keyd
```

Then apply the change:

```sh
systemctl --user daemon-reload
systemctl --user restart mcsr-toolkit.service
systemctl --user status mcsr-toolkit.service --no-pager
```

The CLI flag is a local process setting and cannot be enabled through HTTP. Stop the unit before changing the mode; do not run a second service copy beside it. Recovery through Restore remains available without the flag. Observe-only restart clears persisted old live bindings once and drops the persisted profile without activating bindings.

## Use the UI

1. Open the loopback page and select **Refresh status**. If the launcher is not detected, enter its launcher root in **MCSRLauncher root override**: this is the directory containing `instances/`, not the per-instance game directory. The path is accepted only when it contains a safe MCSRLauncher `instances` directory with at least one valid instance. MCSRLauncher normally puts `launcher/instances/<id>/instance.json` below its working directory.
2. Edit the profile and save it. Minecraft controls are read from `minecraft/options.txt`, with `.minecraft/options.txt` used as a legacy fallback. Instance directories need a matching `instance.json`; missing `options.txt` is reported. The editor supports Minecraft 1.16 keyboard identifiers, modifiers, mouse controls, and unbound controls. It translates keyd names such as `dot`, `leftbrace`, and `kp1` to Minecraft’s `period`, `left.bracket`, and `keypad.1`. Unsupported keyd control assignments, legacy numeric controls, and malformed bindings fail preview for the affected instance; other settings and line endings are preserved.
3. Choose **Preview changes** and review each physical → emitted → Minecraft action row. Apply is enabled only after the saved profile has a current preview and each instance is ready or unchanged. If a second tab saves a different profile, save and preview again after a `stale_profile` response.
4. Choose **Apply profile** and wait for the job result. Keep the returned backup ID; the UI also fills it into the restore field after a successful Apply.
5. To undo the most recent toolkit-managed file and keyd changes, enter its backup ID in **Restore a backup** and choose **Restore backup**. Restore returns options, managed controls, the keyd snapshot, and the active profile to their previous state. After Apply A then Apply B, restore B first to recover A; A can then be restored to recover the original baseline. The editor refreshes from the restored saved profile; any unsaved draft is preserved and must be saved and previewed before Apply. Wait for a successful restore result and confirm keyd health and ordinary typing before any later live activation. Backups are tied to their original launcher root, and Minecraft must be closed.

Apply and Restore are serialized jobs. Keep the service running while a job completes, and do not edit the same instance files concurrently. Repeating an identical Apply does not install another keyd snapshot or create another backup. A change that affects the profile or recorded physical control intent without editing options still receives a restore ID. The next Apply uses that recorded physical key even when two profiles emit the same key.

Restore accepts only the current backup in the recorded history. It rejects an older or already restored backup, changed options or managed state, changed launcher instances, missing history, and inconsistent keyd snapshots before restoring files. A newly discovered instance can be restored to its exact pre-reconciliation options even when those options differ from the previously active profile. Restore the newer backup first; an older backup still requires the exact instance set it recorded, so additional instances must be resolved before that older Restore. Preserve all instance backups, `managed.json`, `lineage.json`, `keyd-adapter.json`, and root-owned keyd snapshots until recovery has been verified.

An interrupted mutation or uncertain failure leaves `mutation.json`. Status exposes `mutation_recovery_pending`; the service blocks further Apply, Restore, reconciliation, and live activation, including after restart. It may still clear dynamic keyd state safely. Stop the toolkit and inspect the recorded backup, current instance files, managed state, active profile, and keyd snapshots together before proceeding. Do not delete the marker merely to bypass this check: recover those components to one consistent recorded state first. Legacy backups without matching lineage also require manual recovery.

Chat and command entry keys emit the configured physical key and update Minecraft’s `key_key.chat` and `key_key.command` bindings in every instance. Missing custom entries are added; missing default entries use Minecraft’s built-in T and slash defaults. Configured Send and Cancel keys emit Enter and Escape while clearing the chat layer, because Minecraft uses those events to send or close chat. Enter may serve as both Open Chat and Send: the game layer uses Enter to open chat, and the chat layer uses Enter to send and clear itself. Every other duplicate pair of chat keys is rejected. Preview checks entry bindings for conflicts with other Minecraft controls.

The browser obtains a per-process token from `GET /api/session`; writes require the same-origin session headers.

## Resize presets and optional maccel

The extension reports preset shortcuts to the service over session D-Bus. The extension grabs preset shortcuts only while Minecraft has effective focus with no Shell modal, and releases them on blur, modal entry, and extension disable. Presets resize only a focused Minecraft window; terminals, other applications, a Shell modal, or an inexact resize are refused. Use a normal, non-maximized Minecraft window for resizing. A separately approved supervised key-remap movement and restore test passed on 2026-10-06: the user confirmed physical `S` moved Backward, focus loss cleared the mapping, and observe-only Restore returned the original options file exactly. A later observe-only F6 test measured the focused Minecraft frame changing from 854×517 to 800×600. On 2026-10-22, supervised eye testing passed 400×300/raw `89464169` (about `0.02083`), shifted sensitivity preservation, exact F6 baseline restoration, blur/modal/disconnect recovery, corrected SIGTERM cleanup, and crash/startup recovery. Two defects found during live testing—driver newline readback and shutdown signal cleanup—were fixed and retested. Final cleanup restored raw `4294967296` / CLI `1.000000000`, removed the root baseline record through verified restore, stopped test services, cleared shortcuts, and preserved empty keyd configuration and the original options hash. Details and limits are in [resize and sensitivity acceptance](integration/resize-wayland.md) and [the acceptance record](integration/acceptance-wayland.md).

Sensitivity adjustment is optional. A readable `maccel get param sens-mult` does not prove write access or eye availability. The desktop login currently cannot write the parameter directly. The service uses the fixed [maccel helper source](src/mcsr_toolkit/maccel_privileged.py) and an [exact sudoers template](packaging/mcsr-toolkit-maccel.sudoers); future helper replacement requires separate security approval after review. The service runs unprivileged, never calls `maccel set`, and does not add the user to the `maccel` group. Upstream reset scripts under `/var/opt/maccel/resets` are group writable and can be executed as root by the upstream ownership helper; do not grant the toolkit user that group or modify those scripts.

The separately approved corrected helper is installed root:root mode `0755`; its source and installed SHA256 are `4b75d58cfa14a3de4037a95e65b2b55f0a97c204b0c10b1b06883efa57192680`. It writes raw digits without a newline and accepts only the observed one-extra-newline sysfs state for recovery, while durable baseline record grammar remains strict. Service shutdown cleanup runs during application lifespan shutdown and drains queued jobs before releasing the service lock. Final source verification passed 401 tests and independent review; the dated acceptance record contains the separate live evidence.

The helper accepts only `eye` and `restore` with no additional arguments. It supports the reviewed driver fingerprint `4AFA0F5B3EBD2F98EE54E56` and signed Q32 raw values; another driver build fails closed. It saves the exact pre-eye raw value in a root-owned runtime baseline before writing `0.02083`, verifies readback, and preserves the first baseline across repeated eye calls. A successful restore verifies that exact raw value before removing the baseline; it never guesses `1` when reading fails. Runtime records are under `/run/mcsr-toolkit-maccel` (`0700`, root-owned; files `0600`). Reboot leaves initialization to the existing driver setup.

Startup recovery must succeed before eye becomes usable. Each restore rotates a generation token; eye receives it on stdin and rejects stale or missing tokens, preventing a queued old helper from writing after recovery. Preflight checks the root-owned executable and all its ancestors, exact sudo permissions for both commands, and a safe **empty-stdin** eye permission probe. That probe must exit `1`, write no stdout, and emit exactly `maccel helper: missing or malformed generation` followed by a newline; it rejects input before any runtime or sysfs access. `sudo -n -l` alone can miss mixed authentication grants. Do not supply a generation token during a permission check.

Shift+F7 resizes without changing the multiplier, including while eye is active. Normal presets restore active eye sensitivity even if their resize fails. The service requests restoration on observed blur, Shell modal entry, extension disconnect/heartbeat loss, clean shutdown, and next startup. These recovery paths passed the supervised 2026-10-22 checks. Focus signals are asynchronous, so a brief interval before a change is observed remains possible. A failed helper operation blocks further eye writes and exposes recovery pending until restoration succeeds. See [setup, manual recovery, and removal](integration/resize-wayland.md#maccel-helper-setup-and-recovery) and the [live procedure](integration/resize-wayland.md#verify-sensitivity-with-the-user-service-connected).

## Focus, resize, and recovery troubleshooting

- **No instance found:** verify the override points to the directory containing `instances/`, each instance has an `instance.json` whose `id` matches its directory, and the game options file is present under `minecraft/` or legacy `.minecraft/`.
- **Extension is missing or inactive:** check `gnome-extensions info mcsr-toolkit@undecember.github.io`, confirm the current GNOME Shell version is supported, and log out and back in after installation if needed. Check service logs with `journalctl --user -u mcsr-toolkit.service -b --no-pager`.
- **Focus is not detected:** check that the extension is `ACTIVE`, the service is running, and UI status reports an extension connection. The service and extension use `$XDG_RUNTIME_DIR/mcsr-toolkit-focus.fifo`; the service owns its creation. A heartbeat timeout clears effective focus. Restart the user service after checking the journal; do not create or replace the FIFO manually.
- **Resize is refused or has the wrong dimensions:** focus Minecraft, leave fullscreen/maximized mode, ensure no GNOME modal is open, and retry with a supported size (100–16384 pixels per axis). Mutter can refuse exact dimensions for fullscreen/maximized windows, and applications may impose smaller limits. See [the resize acceptance procedure](integration/resize-wayland.md) for safe focus/refusal checks.
- **`maccel` is unavailable or unusable:** check the read-only CLI value and service status. A readable value alone is insufficient; verify the separately approved helper, driver fingerprint, exact sudo permissions, and startup recovery. Keep eye zoom disabled while recovery is pending; follow [manual recovery](integration/resize-wayland.md#maccel-helper-setup-and-recovery).
- **Status reports `mode: unsafe`, `unsafe_stop_failure: true`, or `recovery_clear_pending`:** stop the toolkit unit and inspect keyd before continuing. Do not start live mode or claim mappings are inactive until keyd state and ordinary input are checked:

  ```sh
  systemctl --user stop mcsr-toolkit.service
  sudo systemctl status keyd.service --no-pager
  sudo journalctl -u keyd.service -b --no-pager
  ```

  If keyd remains active with uncertain bindings, stop it manually and keep the toolkit stopped while checking configuration:

  ```sh
  sudo systemctl stop keyd.service
  sudo systemctl is-active keyd.service
  ```

  If keyd is stopped or unhealthy, do not use UI Restore yet: Restore first issues `keyd bind reset`, so it requires a working daemon. Inspect and recover `/etc/keyd` to a known safe configuration before starting keyd. Then verify that the service is active and healthy and that ordinary typing works:

  ```sh
  sudo systemctl start keyd.service
  sudo systemctl is-active keyd.service
  sudo systemctl status keyd.service --no-pager
  ```

  Only after those checks pass, start the toolkit in observe-only mode and use **Restore a backup** with the relevant Apply backup ID. Restore uses authenticated `pkexec` and is available without `--enable-live-keyd`. If keyd cannot be brought up safely or ordinary typing is not stable, keep the toolkit stopped and continue keyd recovery; do not invoke UI Restore. If input becomes unusable during supervised testing, keyd's documented panic sequence is **Backspace + Escape + Enter**. Inspect and recover the keyd configuration before starting keyd or the toolkit again.

For the detailed helper recovery behavior, reviewed sudo scope, observed Wayland limitations, and acceptance status, see [the Wayland acceptance record](integration/acceptance-wayland.md) and [focus probe](integration/keyd-focus-probe.md).

## Developer checks

Run the deterministic checks from the repository root:

```sh
.venv/bin/python -m pytest -q
.venv/bin/ruff check .
.venv/bin/mypy src
node tests/extension_harness.cjs
glib-compile-schemas --strict --dry-run extension/schemas
```

These do not exercise live keyd restarts, GNOME focus, physical keyboard input, or maccel hardware. Current recorded local results and desktop acceptance evidence are in [the Wayland acceptance record](integration/acceptance-wayland.md).
