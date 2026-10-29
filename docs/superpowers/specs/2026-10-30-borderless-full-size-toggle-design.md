# Borderless full-size alternative to fullscreen

Status: approved by the user; builder check found no important unanswered questions.

## Intent and selected approach

The user chose to stop using actual Minecraft fullscreen in the managed resize
workflow. Pressing B should alternate between the thin preset and a borderless
window filling its monitor. Both states remain Minecraft windowed mode.

Use the existing GNOME borderless resize path for both sizes. Native fullscreen
and compositor-only fullscreen restoration are excluded from this workflow.
Returning to an arbitrary previous window size was considered earlier; the user
explicitly selected a full-sized borderless window instead.

This replaces further investigation of the native F11 regression for this
change. Preserve that failed acceptance evidence without claiming it fixed.

## Behavior

- A normal resize-preset activation applies that preset when the focused game
  is not already at its observed borderless size and centered position.
- Activating the same normal preset while that geometry is present expands the
  game to the current monitor's full bounds, without title bar or borders.
  The next activation returns to the preset. B therefore alternates thin/full.
- Determine the toggle direction from the current window's observed geometry,
  not a remembered keypress or the transient active-preset status. Alt+Tab,
  manual resizing, monitor changes, reconnects, and a new game window must not
  leave a stale toggle decision.
- Full-size dimensions come from the focused window's current monitor in
  Mutter's logical coordinates, rather than a saved 1920×1080 constant.
  A monitor at a nonzero origin must position the game correctly.
- Keep the existing exact frame/content size and position verification. If the
  compositor cannot provide the requested full-size geometry, report the
  observed constraint; never fall back to actual fullscreen or change desktop
  panels/global window settings to force it.
- Eye zoom and shifted eye behavior retain their existing fixed-size semantics.
  Normal preset transitions continue to restore the baseline sensitivity.
- Preserve F3 priority, chat identity routing, focus/modal gating, generation
  checks, serialization, cancellation, and keyd recovery behavior.

## Windowed game configuration

The local instance's Standard Settings currently contains `fullscreen: true`.
The managed workflow must not request true fullscreen again when the mod resets
settings. Applying the resize-enabled profile should prepare Minecraft's
`fullscreen:false` and synchronize the equivalent enabled local Standard
Settings value, including supported modern/legacy formats. Preserve unrelated
settings and existing disabled-entry/shared-config rules. Do not create a
missing mod config. Include these edits in Preview and the existing exact-file
backup/Restore transaction; do not edit running-game files.

An already running true-fullscreen game still requires the user to return it to
windowed mode or restart after the saved configuration is prepared. Do not
force that state through GNOME. Do not silently repurpose F11 or add a new
automatic launch-resize feature in this change.

## Implementation boundaries

The extension owns monitor selection, the target-size decision, preparation,
and observed result. Extend its typed bridge contract as necessary so the Rust
side can distinguish a successful preset size from successful full-size output;
the existing fixed-size reply check must not reject the full-size result.
Report full-size state accurately rather than showing the thin preset as
active. Label any affected UI preset action to match its toggle behavior.

Likely areas: `extension/extension.js`, `src/gnome.rs`, `src/runtime.rs`,
instance/Standard Settings planning in `src/apply.rs` and related modules,
corresponding Rust/extension/UI tests, README and the existing resize acceptance
guide. Keep startup fixes and their evidence intact; no commits, merges,
pushes, or system installation are authorized by design approval.

## Acceptance

Add failing regressions before implementation for thin → full → thin, observed
geometry after Alt+Tab/manual resize, multiple monitors/origins, new windows,
constrained full-size results, and true-fullscreen refusal. Retain eye zoom,
sensitivity restoration, stale generation and cancellation coverage.

Verify windowed configuration Preview/Apply/Restore with enabled modern and
legacy Standard Settings, disabled/missing/shared configs, and old backups.
Run relevant Rust/JavaScript suites, full locked tests, fmt/Clippy, schema and
release asset checks, refresh Graphify, and obtain independent review.

Supervised acceptance must demonstrate repeated B toggles, visible borderless
full-size output, Alt+Tab recovery, and F3 combinations without resizing. The
temporary F-key mapping required by the previous live test remains a separate
known environment limitation; this change does not silently alter it.
