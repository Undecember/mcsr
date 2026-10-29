# Borderless Minecraft resize design

## Requested behavior

Applying a resize preset makes the Minecraft window borderless: no title bar or window borders. Thin and eye presets shrink the actual window, matching Jingle rather than keeping a fullscreen background. The user explicitly selected this behavior.

Preset dimensions describe the undecorated game area. Every preset change centers the window on its current monitor and verifies the resulting position. This supersedes the original position-preserving design at the user's explicit request during the 2026-10-29 supervised test. Automatic borderless conversion at game launch remains outside this change.

## Current behavior

`extension/extension.js` implements `ResizeFocusedAsync` by calling `move_resize_frame` with the requested outer frame dimensions. It does not remove decorations or leave fullscreen/maximized state. Its polling verifies only the frame dimensions. Consequently, a decorated window can satisfy the current check while its game area is smaller than the preset.

The Rust bridge and sensitivity controller already depend on observed resize results. Retain their focus gating and requirement that eye sensitivity follows a successful resize.

## Approaches

1. **Remove decorations on the actual window during preset application (recommended).** Keep the existing GNOME resize bridge, add decoration preparation, require Minecraft-owned windowed mode and clear maximization when needed, and verify the resulting window before reporting success. This matches the requested Jingle behavior.
2. **Keep a fullscreen surface and present a smaller game image on a background.** This resembles ToolScreen but requires a separate rendering/presentation feature and does not match the user's selected shrinking-window behavior.

## Platform boundary

The supported desktop remains GNOME Shell 50 on Wayland. Distinguish a Minecraft XWayland client from a native Wayland client; the desktop session type alone does not establish the game's backend.

Mutter exposes `Meta.Window:decorated` as read-only, so assigning that property is not an implementation strategy. For XWayland clients, use X11 decoration hints on the exact Minecraft client window through an unprivileged, asynchronous operation. Verify the supported GNOME 50 method of identifying that client before implementation; do not assume a general Mutter window ID is an X11 resource ID.

For native Wayland clients, preserve resizing of an already undecorated game. If client-side decorations cannot be removed through supported APIs, report that borderless preparation is unsupported with an actionable explanation. Do not claim success based only on outer frame dimensions or silently switch the game's backend.

## Resize lifecycle and failures

Validate dimensions and focused Minecraft identity before changing anything. Reserve the pending resize across preparation and observation. Recheck focus and Shell modal state before each asynchronous mutation; cancel further changes on focus loss, extension disable, or timeout.

Refuse fullscreen clients before mutation with an instruction to leave fullscreen through Minecraft's own control (usually F11); changing only compositor fullscreen state is unsafe for GLFW. Cancel if fullscreen is entered during an in-flight request. Prepare a windowed client for borderless resizing, then apply dimensions after decoration/maximization changes settle. Report success only when the window is undecorated, its observed game area matches the request, and its observed frame position matches the calculated monitor center. Missing tools, rejected decoration changes, client size constraints, and timeouts must produce useful errors. Avoid synchronous subprocesses in GNOME Shell.

Repeated presets keep the game borderless. Do not change other applications' decorations or global desktop settings. Preserve existing non-resize behavior and sensitivity restoration.

## Verification

Extend `tests/extension_harness.cjs` to cover delayed decoration removal, exact content dimensions, fullscreen refusal and safe maximized-window preparation, already borderless clients, unsupported native decorations, subprocess failure/timeout, concurrent requests, focus loss, modal entry, and disable during preparation. Run the existing Rust GNOME/sensitivity regressions and relevant packaging checks if extension assets or dependencies change.

Update `integration/resize-wayland.md` and the relevant README guidance. Record live GNOME/Minecraft acceptance separately from automated harness results, including visible absence of title bar/borders and repeated normal/thin/eye transitions. Update Graphify after code changes. Do not commit, merge, push, or install as part of design approval.

## References

- [Jingle borderless controls](https://github.com/DuncanRuns/Jingle/blob/v2/README.md)
- [ToolScreen mode system](https://github.com/jojoe77777/Toolscreen/blob/main/README.md)
- [Mutter decoration property](https://gnome.pages.gitlab.gnome.org/mutter/meta/property.Window.decorated.html)


## Corrective fullscreen boundary after live regression

The supervised test later failed Alt+Tab after compositor-only fullscreen restoration. A Minecraft-owned F11 transition recovered the same process and the user confirmed focus recovery. Therefore the original automatic compositor fullscreen conversion is withdrawn: supported resizing starts after Minecraft itself enters windowed mode. Client-integrated automatic fullscreen transitions require separate work and live focus/blur validation. Native Wayland limitations remain as recorded; this regression occurred on the confirmed XWayland target.
