# Borderless full-size toggle: local implementation validation

Status: **reviewed candidate; subsequent scoped live results are recorded in [live acceptance](2026-10-30-borderless-toggle-live.md)**.

## Source and provenance

The user approved the design and implementation plan and selected Parallel SDD.
The integration worktree is
`/mnt/data/ubuntu/Documents/mcsr/.worktrees/feat-borderless-full-size-toggle`,
branch `feat/borderless-full-size-toggle`, based on dev `5ae4c41` plus the
reviewed, uncommitted startup fixes. Baseline and task patch digests are recorded
in [baseline provenance](2026-10-30-borderless-toggle-baseline.json) and
[task patch provenance](2026-10-30-borderless-toggle-task-patches.json).
The startup dependency patch remains byte-identical to its recorded digest.
No commits or merges were used to materialize these outputs.

Three isolated task worktrees implemented extension geometry, Rust bridge/status,
and windowed file preparation. Each passed deterministic checks and independent
review before assembly. Their source files remain byte-identical to the reviewed
outputs in the integration worktree. Source worktrees are retained.

## Resulting behavior

Normal resize shortcuts alternate between their observed, centered borderless
preset size and the current monitor's full bounds in logical coordinates. Both
states remain windowed. The extension checks exact content/frame geometry and
cancels on stale window, focus, monitor, or routing configuration. The decision
uses actual geometry rather than remembered keypresses. Eye and shifted eye
retain fixed-size behavior; normal transitions restore baseline sensitivity.

A typed toggle reply distinguishes preset/full-size output without weakening
fixed-size validation. Generation-bound status distinguishes full-size from an
active thin preset. The UI reports the last successful resize for the current
focus session. Rejected profile publication recovers pending sensitivity while
still holding operation ownership, before a newer eye write can run.

Profiles containing presets prepare Minecraft `fullscreen:false` and enabled
existing local Standard Settings equivalents. Preview lists affected files;
config-only changes wait for the game to close and join exact backup/Restore.
Fullscreen never enters keyboard binding/state maps. Old backups retain their
historical control-only replay checks. Missing/disabled/shared configuration
and unsafe-path rules remain intact. Removing presets does not invent a new
fullscreen preference; explicit Restore reverses managed file edits.

## Regression and independent review evidence

- Original task baselines passed before edits: extension harness, 56 Rust
  bridge/runtime tests, and 61 settings/Apply/API tests.
- Extension RED: the real harness failed on missing `ToggleResizeFocusedAsync`.
  GREEN included thin/full/thin, manual geometry, monitor changes, and stale
  request cleanup.
- Rust RED: normal preset activation lacked the typed toggle behavior. Targeted
  suites passed after implementation. A stale-eye publication/recovery test
  failed when recovery was suppressed, then passed with serialized recovery.
- Settings RED: fullscreen-only Preview and deferred Apply were incorrectly
  unchanged; new rendering/wrapper APIs were missing. GREEN covered modern and
  legacy configs, rollback/drift, no-presets behavior, and historical
  two-generation Restore with fullscreen-true bytes and resize metadata.
- UI RED: a full-size response was absent from the actual status DOM. GREEN
  displays full-size, preset, and absent-mode states correctly.
- Independent reviews of all three task outputs and final integration: PASS;
  no material defects found. The reviewer also ran extra actual-extension
  cancellation probes and verified Mutter 50 content-coordinate semantics from
  upstream source.
- An ephemeral private `dbus-run-session` probe exercised the real SessionPeer:
  valid preset/full-size `(uub)` replies, actionable UnknownMethod upgrade error,
  and rejection of malformed `(uus)` replies all passed without touching the
  desktop bus. This probe is not a persistent checked-in regression test.
- The concurrency regression uses the actual controller publication fence and
  a simulated profile generation; a full State::on_applied race fixture is not
  included. The reviewer inspected all real mutation/publication paths and found
  no material gap or lock inversion. These are explicit coverage boundaries.

## Final combined deterministic checks

- `cargo fmt --check`: passed.
- `cargo clippy --all-targets --locked -- -D warnings`: passed.
- `cargo test --locked`: passed, 405 reported tests across 23 suites.
- `python3 packaging/keyd/test.py`: passed 94 upstream event cases, 28 toolkit
  event cases, held-key reset cleanup, and actual daemon IPC reset checks.
- `node tests/extension_harness.cjs`: passed.
- `node tests/ui_harness.cjs`: passed.
- `node --test static/app.test.cjs`: 14 passed.
- `node integration/live-ui-acceptance.mjs self-test`: passed.
- `glib-compile-schemas --strict --dry-run extension/schemas`: passed.
- `cargo build --release --locked`: passed.
- `cargo test --release --locked --test rust_assets`: 6 passed.
- `graphify update .`: AST-only refresh completed, 2643 nodes and 6503 edges.
- Tracked/new-file whitespace checks passed. Pristine vendor sources/checksums
  remain unchanged.

Candidate: `target/release/mcsr-toolkit` in the integration worktree.
SHA256: `df034359d2ec42f7ab261b2fab97ed7451184210655070b334936c04f32808c0`.
This is a local reviewed artifact, not a published release.

## Desktop boundary

No installed-system changes, commits, merges, or pushes occurred during this
implementation. The installed service remains observe-only. At completion of local verification, the new extension, windowed configuration
policy, and thin/full-size transitions had not yet been deployed together. The
subsequent deployment and scoped results are recorded in the linked live record.

The previous [live test record](2026-10-29-f3-keyd/live-validation.md) still
establishes only its observed cases: B/Tab+B passed, native fullscreen recovery
failed after resize, and cleanup succeeded. The new workflow avoids native
fullscreen; it does not claim that native F11 failure fixed. Default F-key
translation and retained-record installer bootstrap remain separate limitations.
Follow the [updated resize guide](../../../integration/resize-wayland.md) for the
next supervised acceptance run and explicitly restore temporary test settings.
