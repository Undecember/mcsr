# Unreleased installer update

Source builds now request live keyd by default. Installation offers the missing
keyd package, socket-group membership, and backed-up F13–F24 mapping; `--yes`
accepts prerequisite offers and `--observe-only` opts out of live activation.
Stale desktop service-manager groups produce a reboot-and-repair instruction.
Live-only startup failures try verified observe-only fallback and retain a
pending repair marker. Helper failures retain stderr and exit diagnostics.
`doctor` distinguishes installation safety checks from live readiness.

`repair` now recovers stale keyd uncertainty records and failed managed keyd
services before installation preflight. It stops the toolkit, authenticates a
fixed recovery operation, validates saved configuration, resets the restart
limit, and verifies daemon and IPC health before clearing the unchanged record.
Recovery invalidates older queued helper requests; failed checks retain the
record and leave the toolkit stopped. Profiles and backups remain intact.
Live startup also reuses its verified keyd clear and preset route configuration,
preventing redundant daemon restarts from exhausting systemd's start limit.
When the live service is verified healthy, `doctor` no longer describes its
expected managed keyd configuration as pending recovery.

These changes are not in the published v0.2.0 assets described below. Automated
regression checks do not establish the full focus/chat acceptance gate for the
new installer. An on-device repair and live startup check is recorded in the
[Wayland acceptance record](../integration/acceptance-wayland.md).

# MCSR Toolkit v0.2.0

Approved for stable release on 2026-10-30. MCSR Toolkit v0.2.0 targets Ubuntu
GNOME Shell 50 on x86_64 Wayland. Exact artifact identity and publication
verification are tracked in [the release checklist](rust-release-checklist.md#v020-candidate--2026-10-30).

## Changes since v0.1.0

- Embed a pinned, patched keyd daemon/client and use a toolkit-owned systemd
  override while preserving the distribution executable and service enablement.
  Installation and recovery verify daemon capabilities over IPC. Empty-config
  reset now succeeds, and startup requires sustained daemon readiness.
- Give held native F3 and remapped F3 sources such as Tab priority for F3+B,
  F3+G, repeated F3+F, Shift+F3+F and Shift+F3. Chat/command entry preserves
  ordinary typing, and resize shortcuts resume after release.
- Toggle normal resize presets between the centered borderless preset and a
  full-size borderless window using observed geometry. Both stay windowed;
  eye/shift-eye presets keep their fixed-size behavior. Status distinguishes
  the last preset resize from full-size mode.
- Prepare `fullscreen:false` in Minecraft options and enabled existing local
  Standard Settings when a profile has resize presets. Preview lists affected
  files, Apply waits for Minecraft to close, and Restore preserves exact bytes.
- Temporarily hide the stock GNOME panel for a focused, successfully resized,
  managed borderless window overlapping the panel on the same monitor. Focus,
  overview, Shell grabs/modals, session and window changes restore GNOME's
  current visibility policy. Routing reset and disable release ownership.

Existing control editing, input recording, retained backup/Restore and optional
maccel eye sensitivity remain included. Managed keyd source provenance and its
retained MIT license are documented in [the packaging guide](../packaging/keyd/README.md).

## Download, installation and upgrade

The two release assets are `mcsr-toolkit` and `mcsr-toolkit.sha256`.
After downloading the published assets, keep the pair together:

```sh
sha256sum --check mcsr-toolkit.sha256 && chmod +x ./mcsr-toolkit && ./mcsr-toolkit doctor
```

Review the report, then run `./mcsr-toolkit install`. If the extension is staged,
log out of GNOME Wayland, log back in, and run `./mcsr-toolkit repair` from the
same verified executable. The new extension must be ACTIVE before setup can
finish. The installed service starts observe-only; live keyd activation remains
an explicit local service setting. `keyd.service` must be installed and unmasked;
`maccel` is optional. XWayland decoration removal requires `xprop` from `x11-utils`.

An existing root helper must match the executable inspecting protected recovery
state. On a hash mismatch, use the matching installed
`/usr/local/bin/mcsr-toolkit doctor` first. Restore managed input and verify
recovery before uninstalling with that matching installed executable. The
supervised upgrade used guided uninstall, reviewed authenticated helper
bootstrap, extension staging, logout/login and repair. Retained root records
can require manual bootstrap review after uninstall; an automatic in-place
upgrade is not established. Do not overwrite mismatched files or delete
recovery records to bypass preflight. See [installation and recovery guidance](../README.md#repair-and-uninstall).

## Live evidence and known limitations

The supervised single-monitor XWayland checks passed repeated exact 250×1080
and 1920×1080 windowed toggles, overview/app focus recovery, all five F3
combinations with native F3 and Tab, both chat routes, standalone-release
recovery and outside typing. The panel run additionally passed hiding in both
sizes, usable input in the formerly covered top strip, overview/other-app panel
restoration and return, and Alt+F2 Run-dialog restoration. Apply/Restore returned
Minecraft options and local Standard Settings to their exact recorded hashes;
temporary keyboard settings were restored and the service returned healthy
observe-only. One focus-loss resize cancellation occurred in each recorded run.

These observations used the exact development candidates recorded in
[the toggle live record](superpowers/evidence/2026-10-30-borderless-toggle-live.md)
and [the panel live record](superpowers/evidence/2026-10-30-managed-panel-live.md).
The version-bumped release artifact has separate automated identity checks;
those earlier desktop runs do not establish a fresh install or live run of
that exact artifact.

- **Alt+Tab is blocked when Tab is remapped to F3.** Super/overview switching
  worked; this does not establish an Alt+Tab pass.
- **The test host needs temporary F13–F24 translation** (`fkeys:basic_13-24`) for
  internal resize/fence keys. Its default keyboard map does not deliver all
  required F-key symbols. This release does not automatically change XKB options;
  default-map acceptance is not established.
- **Native F11/menu fullscreen remains problematic after resize.** The supported
  alternative is windowed thin/full-size borderless toggling. Actual fullscreen
  requests are refused; this is not a fix for the native fullscreen failure.
- Native Wayland client decorations, multiple monitors/scales, eye/shift-eye
  behavior on the latest candidate, workspace gestures, panel disable/re-enable
  while hidden, and lock/unlock remain pending supervised acceptance. Automated
  harnesses cover relevant lifecycle boundaries but do not prove desktop input
  regions or those untested transitions. Replacement panels are unsupported.
- Live logout/login while remapped, crash/restart during active remapping,
  rollback and authentication-prompt counts, the latest picker/input recorder,
  and an installed-binary new-instance Restore cycle retain earlier limits.
  Focus updates remain asynchronous; zero input leakage is not established.

These are explicit release-readiness considerations, not resolved by a version
bump or a passing build. See [the resize guide](../integration/resize-wayland.md)
and [the release checklist](rust-release-checklist.md#v020-candidate--2026-10-30).

## Candidate identity and publication boundary

The development base is `d6df1a7edd5241917623da01af9e4e9914624b78`.
Release preparation changes package version to `0.2.0` and updates release
metadata/documentation; no runtime behavior change is intended. The exact
build-input manifest, executable SHA256, verification and local staged assets
are recorded in [the release checklist](rust-release-checklist.md#v020-candidate--2026-10-30).

The operator approved local release integration and annotated tag `v0.2.0` on
2026-10-30: squash `release/v0.2.0` into release-only `main`. Publication and
`main` → `dev` synchronization retain their separate approvals. Public download
identity and release-commit CI must be verified after publication; local
validation alone does not establish those results.
Previous stable release notes remain at the
[v0.1.0 tag](https://github.com/Undecember/mcsr/blob/v0.1.0/docs/rust-release-notes.md).
Report issues through the [issue tracker](https://github.com/Undecember/mcsr/issues).
