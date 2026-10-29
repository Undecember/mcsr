# Managed keyd live validation, 2026-10-29

Status: **live acceptance incomplete — B/Tab+B passed; fullscreen regression reproduced; test cleaned up**.
Post-login repair succeeded; the subsequent startup failures and their local fixes
are recorded below. No Minecraft F3 routing acceptance is claimed.

## Candidate and fresh automated checks

- Source: `dev` at `5ae4c41`.
- `cargo build --release --locked`: passed.
- Candidate: `target/release/mcsr-toolkit`, SHA256
  `509430e259532c434c4f09a8c6a58d170476384fa787b5470a37b96ffdd9dd6e`.
  This is the local build for this session, not the earlier implementation build
  or a published release artifact.
- `cargo test --release --locked --test rust_assets`: 6 passed.
- `python3 packaging/keyd/test.py`: 94 upstream and 28 toolkit event cases,
  plus held-key reset cleanup, passed.
- Extension and UI JavaScript harnesses: passed.

## Initial state and recovery

The host was running GNOME Wayland. The previous executable and helper matched
SHA256 `2c55101368ca33fcd0b7099e06cef51caf49b985cbf4ab71e3f8afcf84a71c73`.
The toolkit user service had a `10-live-keyd.conf` override and was repeatedly
failing startup because `rust-v1/keyd-uncertain.json` remained from the previous
shutdown. Its journal at 21:21:58 KST recorded a failed socket connection and
failed keyd restart. The marker persisted across the subsequent login.

Stopped the toolkit retry loop. The installed privileged keyd `clear` helper
completed successfully and the distribution daemon was active. The user
confirmed ordinary typing worked in another app. State, previous executable,
and live override were backed up under the private directory
`/home/undec/.local/state/mcsr-live-20261029-hv7bycov`. After verified external
recovery, the uncertainty marker was archived there and the reviewed live-only
override was moved there. Observe-only startup then reported a connected
extension, no cleanup/recovery errors, and no live activation.

Restored the two applied backups in order through the installed service API:

1. `000000000000000018e2ff6196d155de` — succeeded, no keyd error.
2. `000000000000000018e2ff4bd067bb9e` — succeeded, no keyd error.

The instance was closed. `/etc/keyd` returned to its empty baseline and keyd
reported all keyboards ignored. Profiles and backup history were retained.

## Uninstall/reinstall finding

The matching old executable's preflight was ready and guided uninstall exited
successfully. Both installed toolkit executables were removed, the user service
was removed, and the distribution keyd daemon remained active with no drop-ins.

The new candidate's first install attempt was refused:

```text
root status verification failed: root status helper unavailable: No such file or directory (os error 2); keyd recovery pending; maccel baseline recovery pending
```

This exposes a lifecycle gap: retained root recovery paths require authenticated
inspection, but uninstall has removed the helper needed for that inspection.
Do not count this as a successful ordinary reinstall. A follow-up fix and a
regression test for reinstall with retained root records remain necessary.

For this supervised run, bootstrapped the exact new candidate to the installer's
fixed `/usr/local/libexec/mcsr-toolkit` path using authenticated `pkexec install`.
The candidate's authenticated recovery preflight then reported ready. No root
recovery records were deleted to bypass preflight.

## Pre-login checkpoint

The new candidate's installer staged the updated extension and returned the
documented logout/login requirement. `install-pending.json` has phase
`shell-restart`. The extension is currently disabled/inactive. The user service
is absent; the managed keyd binary/drop-in has not been activated. Distribution
keyd is active and `/etc/keyd` is empty. The candidate helper is installed and
matches the candidate SHA256 above. Preserve the marker and lineage records.

Log out of GNOME and log back in, then resume from the repository root:

```sh
./target/release/mcsr-toolkit repair
```

Verify the matching installed executable/helper, managed daemon and service
override, extension protocol, observe-only health, and ordinary typing before
enabling live mode. The archived saved profile contains Tab → F3 and B resize;
Restore changed the currently saved profile, so recover that intended draft
through the validated profile API/UI, Preview, and Apply rather than copying
managed state back into place.

Still pending: native F3 and Tab-based F3+B/G/F, Shift+F3+F and Shift+F3;
repeated F, release orders and immediate ordinary B resize; chat and command
identity input; focus/modal changes, restart recovery and input-drain behavior;
new managed-daemon repair/uninstall recovery. Minecraft must supply the visual
results; automated engine tests do not establish them. Authentication prompts
occurred during setup operations, but their count was not measured.

## Post-login repair and live startup failure

After the user logged back in, `target/release/mcsr-toolkit repair` exited
successfully. The extension became ACTIVE, the install marker was cleared, and
both installed toolkit copies matched the candidate SHA256. The managed keyd
override selected `/usr/local/libexec/mcsr-keyd`; both its local and running
daemon probes reported `mcsr-layerk-v1` and `mcsr-matchmods-v1`. Observe-only
HTTP status reported a connected extension and no recovery or cleanup errors.

Enabled the documented live-only service override with Minecraft closed. Live
startup failed before the saved profile could be changed or applied:

```text
keyd cleanup failed; input safety unverified: command failed: exit status: 255; stdout: ; stderr: ; helper clear succeeded
```

Direct reproduction with the stable running managed daemon:
`mcsr-keyd mcsr-daemon-capabilities` succeeded, while `mcsr-keyd bind reset`
exited 255 with empty stderr. `/etc/keyd` was empty. In
`vendor/keyd/src/daemon.c`, the IPC_BIND handler initializes success to zero
and only marks it successful after evaluating a loaded config; with none,
reset is rejected. The toolkit's initial live routing setup requires a clear,
blocking the first Apply from this baseline.

Automatic retries additionally reported a failed daemon capability connection
immediately after the helper restarted keyd. This suggests a separate readiness
race: systemd active state alone does not establish IPC readiness. A fix must
verify this with a bounded readiness check and regression coverage rather than
assuming the empty-config reset issue explains every failure.

Stopped the restart loop and archived the new live override as
`10-live-keyd.post-login-disabled.conf` in the private evidence directory above.
The service is now active with exactly `/usr/local/bin/mcsr-toolkit serve` and
no drop-ins. Final HTTP status reports observe-only mode, connected extension,
no recovery pending, and no cleanup errors. Managed keyd is active, `/etc/keyd`
is empty, and no user uncertainty marker exists. The intended profile was not
saved or applied during this attempt. Startup fixes are required before the
Minecraft combination checks can proceed.


## Startup fixes: reviewed local candidate

The user approved the focused design and implementation plan, then selected
Parallel SDD. Two independent worktrees based on `5ae4c41` implemented the
empty-config IPC reset and bounded readiness fixes. Both outputs passed
independent review before patch assembly in
`.worktrees/fix-keyd-live-startup` on branch `fix/keyd-live-startup`.
No commits or merges were used to assemble the output; both source worktrees
remain available.

Regression evidence:

- Empty-config reset IPC test failed before the patch (expected IPC_SUCCESS,
  observed IPC_FAIL), then passed. The device-free harness invokes the actual
  patched daemon dispatcher over a socketpair, including exact reset, ordinary
  and invalid binds, trailing bytes/NUL suffix, and populated configuration.
- The readiness regression first failed with `IPC connection unavailable`,
  then passed with bounded initial retries and the required stability interval.
  Diagnostic-retention and exhausted-budget regressions also failed before
  their fixes and passed afterward.
- Final isolated helper tests: 24 passed. Affected keyd library tests: 12 passed.
  The untouched integration baseline helper/runtime suites passed 40 tests.

Final combined verification in the integration worktree:

- `cargo fmt --check`: passed.
- `cargo clippy --all-targets --locked -- -D warnings`: passed.
- `cargo test --locked`: all suites passed.
- `python3 packaging/keyd/test.py`: 94 upstream + 28 toolkit event cases,
  held-key reset cleanup, and daemon IPC reset checks passed.
- Extension/UI harnesses and live-UI fixture self-test: passed.
- Strict schema compilation dry-run: passed.
- `cargo build --release --locked`: passed.
- `cargo test --release --locked --test rust_assets`: 6 passed.
- `graphify update .`: AST-only refresh completed (2571 nodes, 6315 edges).
- Independent per-task and combined integration review: PASS, no material defects.

Local candidate executable:
`.worktrees/fix-keyd-live-startup/target/release/mcsr-toolkit`.
SHA256: `765c78b46bdd2c1fe34ed120f38d3b531f79ee7157cb1f0f6981e9cf189268a0`.
This identifies a locally built, reviewed artifact, not a published release.

The two fixes are ready for supervised deployment and empty-baseline startup
retesting. The distribution keyd files, authentication scope, pristine vendored
sources/checksums, and existing cleanup failure handling are unchanged. The
separate uninstall/reinstall helper-bootstrap issue remains outside this fix.
The installed service was not changed during implementation and remains in
observe-only mode. Minecraft F3/chat/focus acceptance remains pending.


## Fixed-candidate deployment and input-fence blocker, 2026-10-30

User authorized the next live test. Verified and installed the reviewed candidate
SHA256 `765c78b46bdd2c1fe34ed120f38d3b531f79ee7157cb1f0f6981e9cf189268a0`.
Private pre-test backups are at
`/home/undec/.local/state/mcsr-live-20261030-t5wzu2zg`. Guided uninstall succeeded;
used the already documented authenticated fixed-path helper bootstrap to work
around the separate retained-record reinstall gap. Guided install succeeded
without another logout/login because extension contents were unchanged. Both
installed toolkit copies match the candidate hash and the extension is ACTIVE.

Live daemon capability probes succeeded. `mcsr-keyd bind reset` with empty
`/etc/keyd` now returned `Success`: the empty-config reset fix passed on this
host. Live startup's helper clear/restart operations completed, but the service
failed later with `GNOME extension call timed out`. Shell journal entries
identified repeated `ConfigurePresets: routing input drain pending` errors.
Stopped the retry loop before applying any profile.

The session keyboard source is IBus Hangul and `xkb-options` was empty. The
current XWayland keymap (using the current user-manager XAUTHORITY, not the
stale path inherited across logout/login) reports:

```text
191 XF86Tools           192 XF86Launch5
193 XF86Launch6         194 XF86Launch7
195 XF86Launch8         196 XF86Launch9
197 no symbol          198 XF86AudioMicMute
199 XF86TouchpadToggle  200 XF86TouchpadOn
201 XF86TouchpadOff     202 no symbol
```

These are the evdev F13–F24 codes plus XKB's eight-code offset. In particular,
F24 has no symbol, while the extension grabs the logical accelerator `F24`.
The installed XKB symbols provide `fkeys(basic_13-24)` as an optional override.
This supports a keyboard-layout mismatch diagnosis; the default session has
not completed the drain marker, and default-layout acceptance has failed.

Returned the toolkit to observe-only mode and archived the exact reviewed live
service override as `10-live-keyd.fence-timeout.conf` in the private backup.
No intended profile was saved or applied and `/etc/keyd` remains empty. Asked
the user whether to temporarily enable `fkeys:basic_13-24` for supervised tests
and restore the setting afterward, or stop and fix default-layout support.
No keyboard-layout setting has been changed at this checkpoint.


## Temporary F-key mapping and successful live startup, 2026-10-30

The user explicitly approved temporary `fkeys:basic_13-24` mapping. Saved the
original empty options in the private backup as `xkb-options-before-test.json`.
The initial option update did not change the active keymap and the fence still
failed. Inspection of the installed GNOME Shell `keyboardManager.js` resource
showed `reapply()` passing its current layout object to `_doApply(id)`, which
looks up a layout by ID and returns if absent. This supports a host Shell
option-reapplication defect rather than a failed settings write.

Refreshed the input source list by briefly appending an XKB US source, then
restored the original Hangul-only list and MRU value; saved both original values
as `input-sources-before-refresh.json`. Hangul remained selected. Verified the
active XWayland map now reports F13 through F24 at keycodes 191–202 (F23 retains
its existing shifted XF86Assistant alternate). With this mapping, live startup
completed with connected extension and no cleanup, recovery, or FIFO errors.
This establishes that the F24 handshake succeeds under the temporary mapping;
default-layout support remains a separate failed acceptance condition.

With Minecraft/Java closed, saved the intended archived profile (Tab → F3,
B thin resize, W → O, S → K, Q → N). Preview reported the instance ready with
the expected mappings. Apply job `d98ed0fe3435442c8449da9cfc3e1505` succeeded,
updated MCSR_Ranked, and returned backup `000000000000000018e308d79393c981`
without a keyd error. The private backup contains `apply-result.json`.
HTTP status then reported mode live, extension connected, no recovery/cleanup
errors, and unchanged reconciliation. Minecraft visual results are still pending.

The temporary XKB option is still enabled for the supervised test. After tests,
restore this backup and ordinary input, stop live mode, restore original XKB
options, refresh the keymap if necessary, and verify the original source list
and Hangul engine before declaring cleanup complete.


## First Minecraft observations and fullscreen recovery failure

With temporary F-key mapping and the intended live profile applied, the user
confirmed both B resize and Tab+B hitbox toggling work correctly. This establishes
those two visual checks only; the remaining F3 combinations, release cases,
chat, and recovery checks remain pending.

The user then reported that entering fullscreen with F11 prevents further
resizing, and that pressing F11 again does not reliably return to windowed mode.
Pressing B a second time currently reapplies the thin preset; restoring normal
window size on a second press is not implemented. A toggle/restore behavior is
an additional requirement to clarify after recovery.

The service remained live with connected extension and no keyd recovery/cleanup
error. It received repeated PresetActivated signals. Recorded resize errors
include the intended fullscreen refusal and one focus-loss cancellation. X11
inspection of Minecraft client `0x140000b` confirmed FULLSCREEN at 1920×1080,
with `_MOTIF_WM_HINTS` decorations value 1. No compositor-only fullscreen
mutation was performed. Asked the user to recover using Minecraft Video
Settings Fullscreen OFF, then stopped live bindings to isolate the game/window
failure from remapping. Applied options and backups remain intact; no Restore
was attempted while Minecraft was running.

Temporary F-key mapping remains enabled during this investigation and still
requires restoration at test cleanup. The applied backup to restore after
Minecraft closes remains `000000000000000018e308d79393c981`.


Follow-up isolation: the user could not reproduce while B was disabled, so the
live service was re-enabled and reported mode live, connected extension, and no
cleanup/recovery errors. With B enabled, the user again confirmed that the
second F11 does not leave fullscreen. After a fresh Minecraft launch, the user
confirmed that F11 works before using B or Tab. This narrows the failure to the
resize/fullscreen interaction, but does not establish its implementation cause.

A read-only X11/runtime trace is retained privately as `fullscreen-trace.jsonl`.
The game process was restarted during capture (54852 → 56556); an intermediate
BadWindow/empty-geometry sample reflects that client disappearing, not a
successful fullscreen exit. The XID was reused by the subsequent process.
The instance's local Standard Settings includes `fullscreen: true` and F11 as
its fullscreen binding; those fields were not changed, and causation has not
been established.

Asked the user to distinguish desired second-B behavior (borderless full-screen
window, previous geometry, or Minecraft-owned fullscreen) and whether the game
menu Fullscreen OFF can recover the failed state. No compositor fullscreen
mutation or new code change was made. B/live mode and the temporary F-key mapping
are currently enabled for the supervised investigation.


## Confirmed outcome and completed test cleanup

The user chose the new second-B requirement: **fill the screen as a borderless
window**. This is windowed geometry, not a request for Minecraft-owned true
fullscreen. No toggle implementation was added during this acceptance run.
The user also confirmed that Minecraft's Video Settings Fullscreen OFF fails
in the affected post-resize state. They saved and closed Minecraft normally.
The cause of that post-resize native fullscreen regression remains unproven;
F11 was confirmed to work on a fresh launch before B/Tab usage.

Stopped the live unit, archived its exact live-only drop-in in the private
backup as `10-live-keyd.after-f11-test.conf`, and started observe-only mode.
Restored original `xkb-options` (`@as []`), original Hangul-only sources and MRU,
and refreshed the active keymap. Verified keycode 191 is XF86Tools again and
202 is unassigned, matching the initial map; IBus engine is Hangul.

Initial Restore job `60628289608a45a6be8b87325545455d` safely refused with
`changed options or backup contents`. Preserved the game-written options as
`options-after-game.txt` in the private backup. Byte-level review established
that the sole drift from the exact Apply image was `fullscreen:true` becoming
`fullscreen:false`; changing only that field reproduced the recorded after-hash.
Reconciled that reviewed field for the normal Restore transaction, without
editing backup manifests or recovery records. Restore job
`d24f48d7b1d9457daabebaf6464f5991` succeeded for backup
`000000000000000018e308d79393c981`, with no keyd error. Retained the game's
final `fullscreen:false` preference afterward; all test-managed controls and
local Standard Settings were restored through the application's backup path.

Final verified state: lineage head null, `/etc/keyd` empty, no uncertainty
marker, toolkit observe-only, extension connected, no cleanup/recovery errors.
The test D-Bus monitor was stopped. Candidate remains installed; no commits,
merges, or pushes occurred. Remaining acceptance cases have not passed by
implication: F3+G/F, Shift combinations, native F3 comparison, chat/focus/restart,
and the resize/fullscreen lifecycle still require their own completed checks.
