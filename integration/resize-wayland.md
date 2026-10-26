# GNOME Wayland resize and sensitivity acceptance

This guide describes current observation steps for the Rust service and GNOME Shell 50 extension. It does not require enabling live keyd mappings. Run only the guided Rust installation and a service that is already installed; do not start a second service process beside the managed unit. Use a normal, focused Minecraft window: fullscreen or maximized clients may refuse exact dimensions, and a client may impose stricter size limits than Mutter.

## Current Rust observation procedure

Confirm the installed extension and service are available:

```sh
gnome-extensions info mcsr-toolkit@undecember.github.io
systemctl --user status mcsr-toolkit.service --no-pager
```

The extension state must be ACTIVE, and the service must report observe-only mode. If extension installation is staged, follow the logout/login and `mcsr-toolkit repair` steps in the [README](../README.md); do not test with both `mcsr-toolkit@local` and the new UUID enabled. Open the local UI at <http://127.0.0.1:8765/>, configure normal and eye presets in the saved profile, and confirm the extension is connected. Do not enable `--enable-live-keyd` for resize observation.

In a terminal, inspect focus and request a size while Minecraft is focused:

```sh
gdbus call --session --dest org.gnome.Shell --object-path /org/mcsr/Toolkit \
  --method org.mcsr.Toolkit.GetFocus
gdbus call --session --dest org.gnome.Shell --object-path /org/mcsr/Toolkit \
  --method org.mcsr.Toolkit.ResizeFocused 800 600
```

Record the actual method reply and observed frame size. An exact `(uint32 800, uint32 600)` reply and matching frame are expected for the supported case. Repeat with the terminal focused, no game focused, a Shell modal open, and invalid dimensions such as `99 600` or `16385 600`; these cases must refuse the resize and must not resize the terminal. Focus must remain on the original Minecraft window until the result arrives. The extension waits for the observed frame size; a requested resize by itself is not a pass.

Observe extension signals in another terminal:

```sh
gdbus monitor --session --dest org.gnome.Shell --object-path /org/mcsr/Toolkit
```

With shortcuts configured through the UI, press the normal and eye shortcuts and the shifted eye shortcut. Expected `PresetActivated` payloads are `('normal', false)`, `('eye', false)`, and `('eye', true)`. Repeat with the terminal focused and a Shell modal open; no activation should be emitted. Resolve conflicts with GNOME or other extensions before accepting delivery. Signals alone do not establish that the service applied a size or changed sensitivity; verify the observed frame and sensitivity separately. The service must use typed D-Bus signals rather than parse monitor output.

The automated extension harness covers shortcut callbacks, preset IDs, shifted eye zoom, modal refusal, delayed dimensions, wrong focus, invalid sizes, pending focus loss, reconfiguration, and missing/regular FIFO refusal. It uses controlled GNOME boundary doubles and does not establish desktop acceptance. Run it with `node tests/extension_harness.cjs`.

## Historical evidence (2026-10-04–06)

These observations predate the Rust source cutover and are historical evidence for the earlier extension/service implementation. They do not validate the current Rust installer or service.

On 2026-10-04 the production `mcsr-toolkit@local` extension reported `State: ACTIVE` in GNOME Shell 50.1. With MCSR Ranked focused, `GetFocus` reported 854×517, `ResizeFocused 800 600` returned `(uint32 800, uint32 600)`, and the next `GetFocus` reported 800×600. The same method refused a focused terminal.

After registering F6 for `normal` and F7 for `eye`, the extension emitted `PresetActivated ('normal', false)`, `('eye', false)`, and `('eye', true)` for F6, F7, and Shift+F7. With GNOME's Alt+F2 Run dialog over Minecraft, `GetFocus` reported `modal=true`, F6 emitted no preset signal, and `ResizeFocused 700 500` returned `no focused Minecraft window`. No keyd remaps or sensitivity changes were active during these checks.

The first observe-only focus-service integration found malformed FIFO bytes from an asynchronous GJS write. A standalone native GJS test then delivered 20/20 exact JSON heartbeats with `GLib.Bytes` and `write_bytes_async`, including after a reader restart. After a GNOME login loaded that fix, the observe-only service received valid heartbeats with no FIFO errors, reported Minecraft focus, cleared it on the Run dialog, restored it after the dialog closed, and cleared it again when the extension was disabled for over two seconds. These checks covered focus delivery and watchdog behavior without activating keyd. The service's extension-availability lifecycle fix was live tested after disabling and re-enabling the extension on 2026-10-04; see the [focus probe record](keyd-focus-probe.md). Preset resize handling through the service and sensitivity behavior were not covered by these checks.

On 2026-10-06, a measured observe-only run used a fresh data directory and port 8877. With MCSR_Ranked focused, Mutter reported 854×517 and `modal=false`; pressing F6 produced `PresetActivated ('normal', false)`. The frame reached 800×600 within about 0.55 seconds and remained there in later samples. The service initially reported `active_preset=normal`, later `null` while the frame stayed at 800×600 and no errors were reported; the cause was not determined. The user confirmed the visible resize. The service was stopped and `ConfigurePresets '[]'` returned `()`. It did not edit Minecraft controls. These results are historical and are superseded for Rust release acceptance by fresh observations.

## Historical sensitivity and helper acceptance (2026-10-22)

The 2026-10-22 live sensitivity run tested a Python-era helper and service. The corrected helper source and installed SHA256 was `4b75d58cfa14a3de4037a95e65b2b55f0a97c204b0c10b1b06883efa57192680`; the exact two-command sudoers rule was installed after separate approvals. Supervised resize, eye/shifted sensitivity, baseline restoration, blur/modal/disconnect, SIGTERM after its fix, and crash/startup recovery passed. Final raw/CLI baseline was `4294967296` / `1.000000000`; the root baseline record was absent, services and sampler stopped, shortcuts cleared, `/etc/keyd` empty, and the options hash unchanged. The earlier failed attempts and recovery details remain in the [dated Wayland acceptance record](acceptance-wayland.md#2026-10-22-corrected-helper-acceptance-and-shutdown-repair). This evidence does not satisfy Rust release acceptance.

The first helper wrote LF into a driver that appends another LF on read, so eye readback and restore failed. Direct approved recovery restored the numeric baseline while retaining its root record. A reviewed replacement wrote raw digits without LF and accepted the observed legacy double-LF parameter state without relaxing durable records; it recovered the record successfully. A later SIGTERM test exposed cleanup after Uvicorn signal replay; application lifespan shutdown and draining queued jobs before lock release fixed it. The final source suite passed 401 tests; live SIGTERM and startup recovery were then retested. Do not reinstall the known failing helper. Historical Python helper source and sudoers instructions have been removed from the active repository; use the Rust installer and [release checklist](../docs/rust-release-checklist.md) for current operations.

The supported driver fingerprint in that run was `4AFA0F5B3EBD2F98EE54E56`. The helper recorded the positive signed Q32 raw value before eye writes, synced the baseline, and checked writes by readback. Repeated eye calls preserved the baseline; Restore rotated the generation to invalidate queued eye calls and removed the baseline record only after verified restoration. No guessed fallback multiplier was written. The history and remaining limits are preserved in the dated acceptance record.

## Current Rust release gates

Fresh Rust live observations remain required. Record GNOME/session and software versions, binary SHA256, original Minecraft options hashes, keyd configuration, maccel raw/CLI baseline when applicable, observed frames, refusal cases, sensitivity samples, restoration causes, and cleanup. Confirm ordinary typing and restored state independently. Keep the installed helper and recovery records until restoration is verified. Do not treat automated harnesses or historical Python-era measurements as desktop acceptance. The pending logout/login, guided uninstall, and exact published-artifact checks are tracked in the [Rust release checklist](../docs/rust-release-checklist.md).

GNOME API references: [GJS D-Bus exports and method invocations](https://gjs.guide/guides/gio/dbus.html), [Mutter Window API](https://gnome.pages.gitlab.gnome.org/mutter/meta/class.Window.html).
