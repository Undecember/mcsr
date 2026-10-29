# Managed Minecraft panel visibility

Status: approved by the user. Small extension-focused change.

## Intent and selected approach

The user requested implementation of the recommended conditional panel hiding:
the GNOME clock/battery panel must not cover the focused toolkit-managed
borderless Minecraft window. Keep Minecraft windowed and retain thin/full-size
toggling. Both sizes can overlap the top panel on this host.

Temporarily hide the overlapping stock GNOME panel. Ordinary always-on-top
stacking does not raise an application above Shell chrome. Reparenting/raising
the game above Shell UI was considered but rejected as a more fragile change.
No native/compositor fullscreen transition is part of this approach.

## Behavior and ownership

- Eligibility begins only after this extension has successfully verified a
  managed borderless resize for that game window. Matching a Minecraft class
  alone is insufficient; do not hide the panel for unrelated/unmanaged games.
- Hide only while that same window is focused, windowed, undecorated, and its
  observed rectangle actually overlaps the panel on the panel's monitor.
  A game on another monitor must not hide the primary monitor's panel.
- Restore normal panel behavior immediately on focus loss, overview entry,
  Shell modal/lock/session change, minimization, close, loss of overlap,
  decoration/fullscreen change, routing reset, or extension disable. Reevaluate
  on returning to a still-managed eligible window.
- Respect GNOME's current visibility policy when restoring, including another
  application's actual fullscreen state. Do not blindly show a panel that GNOME
  currently hides. Track this feature's own visibility intervention and preserve
  an already hidden baseline; do not assume ownership of another extension's
  panel customization.
- GNOME visibility refreshes while the window remains eligible must not leave
  the bar covering the game. Avoid a self-triggering visibility-signal loop.
- Observe window geometry/ownership changes; retain the existing heartbeat as
  a fallback without using polling as the sole focus/overview restoration path.
- Drop managed ownership on routing replacement/reset and disable; disconnect
  all added handlers and restore panel policy during cleanup and failures.

## Scope and constraints

Keep the existing extension's focus delivery, resize geometry verification,
F3/chat routing, and recovery semantics. No profile fields, persistent desktop
settings, keyd changes, privileged helpers, window reparenting, or replacement
panel implementation. The supported target is the stock GNOME Shell 50 panel;
compatibility with third-party panel replacements is not promised.

This is an explicit, temporary panel-visibility feature selected by the user.
It does not authorize changing panel layout/struts or hiding panels as a
fallback when exact window dimensions cannot be achieved. If required Shell
hooks are unavailable, leave the panel under normal Shell control and report a
useful diagnostic rather than force unrelated UI changes.

Build on the reviewed startup and borderless-toggle prerequisite changes in
the current isolated checkout, preserving their uncommitted source/evidence.
Do not commit, merge, push, or deploy as part of design approval.

## Verification

Before implementation, add failing actual-extension harness cases for eligible
thin/full-size overlap, unmanaged/decorated/fullscreen clients, another monitor,
focus/overview/modal/session transitions, geometry changes, repeated visibility
updates, routing reset, close, disable/re-enable, and a panel already hidden by
normal Shell policy. Check that cleanup respects current Shell policy and
removes only this feature's handlers. Keep existing resize and input tests green.

Run extension/schema checks, the full locked Rust suite, fmt/Clippy, web harnesses,
engine tests, release build/assets, Graphify refresh, and independent lifecycle
review. Update README and resize acceptance guidance.

A later supervised deployment must verify that the panel stops covering both
sizes, returns on app switching/overview, and survives repeated toggles and
disable/re-enable. No live acceptance is inferred from the harness.
