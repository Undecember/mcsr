# GNOME Wayland resize and sensitivity acceptance

## Borderless full-size toggle candidate (2026-10-30)

The current candidate uses windowed thin/full-size toggling as the fullscreen
alternative selected after the latest live regression. Normal shortcuts toggle
between their centered preset and the current monitor's full bounds. Both are
borderless windows; no fullscreen state mutation or F11 synthesis is used.
`ResizeFocused` remains the fixed-size operation for eye presets, while the new
typed `ToggleResizeFocused` reply includes the verified size and full-size flag.
An older extension that lacks the method must report an upgrade error.

Applying a profile with presets prepares Minecraft `fullscreen:false` and the
enabled equivalent in existing local Standard Settings. Check Preview's file
details, close Minecraft for Apply, and retain its backup ID. Disabled/missing
mod entries and missing files remain unchanged; shared configs are refused.
Restore must return the exact pre-Apply bytes, including an original fullscreen
preference. Older backups remain valid under their historical control checks.

The supervised single-monitor XWayland run passed repeated thin/full-size
toggles with exact 250×1080 at 835,0 and 1920×1080 at 0,0, always with the X11
fullscreen flag off. Overview-based focus recovery, all five F3 combinations
with both sources, chat identity, release-resume behavior, and exact cleanup
passed. One early focus-loss cancellation was observed before successful
repeated toggles. The default F-key map required a temporary override, and
Alt+Tab failed because Tab → F3 also remaps its chord. See the
[full live record](../docs/superpowers/evidence/2026-10-30-borderless-toggle-live.md).

Use this checklist to extend acceptance beyond that recorded scope:

1. Start the prepared instance windowed. Use B repeatedly to observe thin →
   full-size → thin without the game or compositor becoming fullscreen.
2. Verify no title bar/borders, exact game dimensions, and monitor position.
   Test a monitor at a nonzero origin when available. Compositor constraints
   must produce an error rather than a false success or fullscreen fallback.
3. Alt+Tab away/back between toggles. Manually resize/move the window and verify
   the next B uses observed geometry. Close/reopen the instance and repeat.
4. Check full-size status without the thin preset falsely reported active;
   test eye/shift-eye sizes and sensitivity restoration if configured.
5. Verify Tab/native F3 debug combinations and chat input do not activate resize.
   Stop/restore the profile and confirm ordinary input afterward.

Multiple monitors/scales, manual geometry changes, repeated relaunch, and eye
presets remain unvalidated live. Alt+Tab requires a routing fix; the overview
check does not stand in for a passing Alt+Tab shortcut. The preceding native
F11 and menu fullscreen failure is recorded in
[the live evidence](../docs/superpowers/evidence/2026-10-29-f3-keyd/live-validation.md).
This workflow avoids native fullscreen; that failure is not claimed fixed.
The separate temporary F-key mapping requirement on the test host also remains
an acceptance limitation. Do not alter the user's keyboard map implicitly.

The fixed-size primitive checks below use observe-only mode. Managed keyboard shortcuts require separately authorized live keyd mode and an applied profile. Run only the guided Rust installation and its managed service; do not start a second copy. Earlier primitive checks confirmed dimensions and centering but exposed the fullscreen/focus failures recorded below. The new thin/full-size workflow still requires live acceptance. A client may impose stricter size limits than Mutter.

## Managed stock-panel visibility candidate (2026-10-30)

After a verified successful resize, the extension temporarily hides the stock
GNOME Shell 50 panel when the same managed, focused, windowed, undecorated game
window overlaps it on the panel's monitor. This includes thin, full-size, and
fixed-size eye windows. Merely launching Minecraft does not claim panel control.
Focus loss, overview or Shell-grab/modal entry, lock/session changes,
minimization, close, lost overlap, decoration/fullscreen changes, routing reset,
and disable restore normal Shell visibility policy. Returning to an eligible
managed window reevaluates hiding. A panel hidden by normal Shell policy is not
unconditionally shown on release.

This uses the stock panel actor and GNOME's current visibility policy; panel
layout, struts, desktop settings, and fullscreen state remain unchanged.
Replacement panels are outside the supported scope. Missing required hooks
leave the panel under Shell control with a bounded diagnostic. Automated
regressions and review are recorded in
[the panel validation record](../docs/superpowers/evidence/2026-10-30-managed-panel-validation.md).
The installed candidate passed supervised single-monitor XWayland checks for
thin/full-size panel hiding and usable top-edge input, overview/other-app
restoration and return, Run-dialog restoration, and continued B toggling. Exact
backup/keyboard cleanup was verified. One focus-loss cancellation was observed.
See [the live record](../docs/superpowers/evidence/2026-10-30-managed-panel-live.md).
Workspace gestures, disable/re-enable while hidden, lock/unlock, eye windows,
and multiple monitors/scales remain pending live checks.

Use this checklist for subsequent supervised runs; retain the remaining
unvalidated cases above when reporting the accepted scope:

1. Before any toolkit resize, focus Minecraft and verify ordinary panel policy.
   Trigger a verified top-overlapping thin resize, then full-size and thin
   again. Inspect the entire top edge: clock/battery chrome must not cover it.
   Verify usable game input in the strip formerly occupied by the panel;
   matching geometry or a hidden actor alone does not establish input delivery.
2. Switch to another app through Super/overview, then return to Minecraft.
   Check normal panel visibility while away and hiding on return. Open/close
   the overview and a Shell dialog, then perform a workspace gesture without
   opening the overview. Verify immediate policy restoration through each
   transition, including a stage grab with unchanged keyboard focus.
3. Move the managed game below the panel, minimize/restore it, and close/reopen
   it. Check normal policy when there is no overlap or no managed window.
   If another monitor is available, repeat there and at a nonzero origin;
   the primary panel must remain normal for a game on another monitor.
4. Reset routing, then resize again. Disable the extension while it owns a
   hide and verify panel restoration; re-enable and verify a fresh successful
   resize is required to claim the window again. Also check focus delivery
   and B toggling after re-enabling.
5. Record panel behavior when another app is actually fullscreen, and run
   lock/unlock only in a supervised session. Restoration must follow current
   GNOME policy. Finish with the normal backup/Restore and input cleanup checks.

Record observed outcomes, session/extension versions, binary SHA256, game
backend, and monitor geometry. Stage/window harness doubles do not establish
live input regions, lock/unlock, or monitor behavior. The earlier Alt+Tab,
temporary F-key mapping, and native-fullscreen limitations remain as recorded
above; this panel change does not resolve them.

## Current Rust observation procedure

Confirm the installed extension and service are available:

```sh
gnome-extensions info mcsr-toolkit@undecember.github.io
systemctl --user status mcsr-toolkit.service --no-pager
```

The extension state must be ACTIVE, and the service must report observe-only mode. If extension installation is staged, follow the logout/login and `mcsr-toolkit repair` steps in the [README](../README.md); do not test with both `mcsr-toolkit@local` and the new UUID enabled. Open the local UI at <http://127.0.0.1:8765/>, configure normal, thin, and eye presets in the saved profile, and confirm the extension is connected. Do not enable `--enable-live-keyd` for resize observation.

Run this block in a terminal, then focus Minecraft during the three-second delay:

```sh
sleep 3
gdbus call --session --dest org.gnome.Shell --object-path /org/mcsr/Toolkit \
  --method org.mcsr.Toolkit.GetFocus
gdbus call --session --dest org.gnome.Shell --object-path /org/mcsr/Toolkit \
  --method org.mcsr.Toolkit.ResizeFocused 800 600
```

Record the Minecraft client backend (XWayland or native Wayland), the actual method reply, and the observed content and frame dimensions. Preset dimensions describe the game area. An exact `(uint32 800, uint32 600)` reply, matching content and frame, and visible absence of the title bar and borders are expected for a supported client. A request starting in fullscreen must refuse before any window mutation and explain that Minecraft must leave fullscreen through its own control (usually F11). Use that native control, then retry in windowed mode. Test maximized windowed clients separately and verify maximization clears before sizing. Never use a compositor-only fullscreen atom change as a substitute for the game's transition. The actual window should shrink and remain centered on the monitor containing Minecraft. On a 1920×1080 monitor at 0,0, a 280×1000 preset must land at 820,40. Verify the observed frame position as well as dimensions. Automatic conversion at launch is outside this behavior.

For decorated XWayland clients, `xprop` from Ubuntu's `x11-utils` package is required. Verify a useful error when the tool is unavailable; install it before retrying. Native Wayland acceptance remains unvalidated: the geometry API cannot prove the absence of client-side decorations, even when content and frame dimensions match. Already undecorated native clients retain the resize path, but matching geometry alone is not native borderless acceptance. A decorated native client must report that borderless preparation is unsupported; use client-side borderless support or an XWayland game. Record errors for clients that reject dimensions rather than treating the request itself as a pass.

Repeat with the terminal focused, no game focused, a Shell modal open, and invalid dimensions such as `99 600` or `16385 600`; these cases must refuse the resize and must not resize the terminal. During preparation, change focus to another window or open a Shell modal and confirm cancellation prevents further changes. Focus must remain on the original Minecraft window until the result arrives. Confirm unrelated windows retain their decorations and dimensions.

Observe extension signals in another terminal:

```sh
gdbus monitor --session --dest org.gnome.Shell --object-path /org/mcsr/Toolkit
```

After separately authorizing live mode and applying a profile, test repeated thin/full-size toggles and any configured eye/shift-eye shortcuts. Confirm each game area and the continued absence of borders. Alt+Tab away and back after transitions; exact geometry alone is not a pass. Expected `PresetActivated` IDs include the current activation token, for example `('<activation>:thin', false)` or `('<activation>:eye', true)`. Repeat with a terminal focused and a Shell modal open; no activation should be emitted. Resolve grab conflicts before accepting delivery. Signals alone do not establish that the service applied a size or sensitivity; verify the observed result separately. A failed eye resize must not write eye sensitivity; a normal preset still attempts baseline restoration if resizing fails. Fullscreen refusal/cancellation remains covered by the harness; the native fullscreen regression is not part of the accepted windowed workflow. The service consumes typed D-Bus signals rather than monitor output.

The automated extension harness covers shortcut callbacks, preset IDs, shifted eye zoom, modal refusal, delayed dimensions, wrong focus, invalid sizes, pending focus loss, reconfiguration, and missing/regular FIFO refusal. It uses controlled GNOME boundary doubles and does not establish desktop acceptance. Run it with `node tests/extension_harness.cjs`.

## Supervised borderless primitive check (2026-10-29)

On the user's primary Ubuntu 26.04.1 / GNOME Shell 50.1 Wayland environment, the running `Minecraft* 1.16.1 - MCSR Ranked` window was confirmed as an XWayland client. Jingle v1.3.0's [default resizing script](https://github.com/DuncanRuns/Jingle/blob/v1.3.0/src/main/resources/defaultscripts/Resizing.lua) sets Thin BT to **280×1000**; this is distinct from the saved local preset 403×1080. Jingle v2 removed its resizing feature.

At the user's request, a temporary live check removed fullscreen/maximized state through EWMH, set `_MOTIF_WM_HINTS` on the exact Minecraft X11 client, then used the already installed extension's `ResizeFocused 280 1000` method while the user focused Minecraft. The installed service remained unavailable, so this test did not exercise shortcut delivery. The method replied `(uint32 280, uint32 1000)`. The next `GetFocus` reported frame 280×1000; an independent X11 client query also reported 280×1000 with zero border width, parent equal to the X11 root, and no fullscreen/maximized state. The candidate's exact `xprop -f _MOTIF_WM_HINTS 32c` command was repeated successfully. The saved profile, keyboard mappings, keyd state, and sensitivity were not modified by this test. The Minecraft options SHA256 remained `db95195134a2da846e845d6c07d1188348df2bab884d7880dcb8a4a21f39e804`.

The `_NET_FRAME_EXTENTS` property remained `[0, 0, 37, 0]` after the frame was removed. An initial attempt incorrectly treated that stale property alone as evidence of a remaining border and stopped before resize. Comparing Mutter's actual frame with the X11 client geometry established the exact borderless dimensions; do not substitute this stale hint for observed geometry. Changing the hint's X11 property type alone did not clear that metadata.

The user confirmed the size and borderless appearance, then requested centering. A temporary EWMH move placed the 280×1000 client at 820,40 on the single 1920×1080 monitor: client center 960,540 exactly matched the monitor center. The user confirmed the centered, borderless layout and requested fullscreen restoration. Fullscreen 1920×1080 at 0,0 was restored, the original absent Motif hint was restored, and the options hash remained unchanged. The implementation was updated with monitor-centered positioning and regression tests after this explicit requirement change. This check validates the live border-removal primitive and existing GNOME resize bridge on this machine. The candidate extension is not installed, so its new preparation/cancellation lifecycle and shortcut-through-service behavior remain unvalidated live. The existing service's keyd startup failure was not repaired or bypassed by starting another service. Runtime snapshots are in `/tmp/mcsr-thin-live-m_jvvrw8/` for this session; these temporary files are not persistent release evidence.

## Fullscreen restoration regression and recovery (2026-10-29)

The earlier geometry-only fullscreen restoration was **not a complete restoration pass**. The user subsequently reported that Alt+Tab could not bring Minecraft forward. The same game process was alive, but the X11 client was `Iconic` and `IsUnMapped`, with the fullscreen atom still set. Removing only the fullscreen atom, mapping the window, and activation requests did not recover it.

[GLFW 3.4's X11 FocusOut handler](https://github.com/glfw/glfw/blob/3.4/src/x11_window.c#L1626-L1687) can iconify a window when its internal monitor pointer and auto-iconify setting indicate fullscreen. External `_NET_WM_STATE` notifications do not clear that pointer. The observed recovery is consistent with this client/compositor mismatch, although the live process's internal pointer and auto-iconify flag were not inspected directly.

The configured Minecraft fullscreen binding was F11. A press/release delivered to the exact game client let Minecraft perform its own transition, followed by remapping/activation. The client became mapped with normal WM state, client 854×480 and GNOME frame 854×517, and the bridge reported Minecraft focus. The user then confirmed that Alt+Tab away and back worked again. The game remains in this working windowed state. Minecraft itself saved `fullscreen:false`; the resulting options SHA256 was `9f4b3a564a5919b99495a6b79137f2d7d28c7f68eeff473ec757abfe33de264a`. No options file was edited directly during recovery, and the process/world was not terminated.

The candidate's compositor-only `unmake_fullscreen()` path was removed. It now refuses initial fullscreen requests before mutation and cancels a request if Minecraft enters fullscreen during preparation or observation. It instructs the user to leave fullscreen through Minecraft's own control before retrying. Windowed border removal, centering, exact-size verification, and safe unmaximization remain. Regression tests cover those refusal/cancellation boundaries. Do not deploy the earlier fullscreen-conversion version or claim safe fullscreen restoration from size/state atoms alone.

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
