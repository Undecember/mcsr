# Borderless full-size toggle live acceptance, 2026-10-30

Status: **scoped live checks passed; Alt+Tab routing failed; cleanup verified complete**.

The user authorized supervised testing of the reviewed candidate at
`.worktrees/feat-borderless-full-size-toggle/target/release/mcsr-toolkit`, SHA256
`df034359d2ec42f7ab261b2fab97ed7451184210655070b334936c04f32808c0`.

Preflight: previous installed app/helper matched reviewed startup candidate
`765c78b46bdd2c1fe34ed120f38d3b531f79ee7157cb1f0f6981e9cf189268a0`.
Toolkit reported healthy observe-only mode, extension connected, no recovery or
cleanup errors. No Minecraft/Java process was running. Lineage head was null,
`/etc/keyd` empty, and original XKB options remained `@as []`.

Private state and previous executable/managed-keyd backups were saved at
`/home/undec/.local/state/mcsr-borderless-live-20261030-f6bjilpm`.
The matching installed executable's doctor reported ready and guided uninstall
succeeded. Used the previously reviewed authenticated fixed-path helper bootstrap
for the retained-root-record reinstall limitation, then ran candidate install.
No recovery records were deleted to bypass preflight.

Install staged the changed extension and requested a new Shell session.
`install-pending.json` has phase `shell-restart`. The new helper matches the
candidate hash. The user app/service is not yet reinstalled; distribution keyd
is active with no toolkit override and empty configuration. No temporary XKB
option was enabled during staging.

After logging out and back in, resume using this exact executable:

```sh
/mnt/data/ubuntu/Documents/mcsr/.worktrees/feat-borderless-full-size-toggle/target/release/mcsr-toolkit repair
```

Verify installed hashes and new extension method, then use the already
authorized temporary F-key mapping (preserve/restore original XKB settings and
Hangul source). Preview and Apply the intended Tab → F3/B thin profile while
Minecraft is closed, verifying `fullscreen:false` in options and enabled local
Standard Settings before launch. Keep the resulting backup ID.

Pending supervised checks: repeated B thin → full-size borderless → thin with
no actual fullscreen, Alt+Tab away/back, accurate status, Tab+B and remaining F3
combinations without resize, then exact Restore/input/keyboard-map cleanup.
Do not infer these results from the earlier candidate's B/Tab+B success or from
automated tests. Preserve staging marker and lineage for repair.


## Post-login repair and preparation

The user returned after logout/login. Repair from the exact candidate exited
successfully. Both installed toolkit hashes match
`df034359d2ec42f7ab261b2fab97ed7451184210655070b334936c04f32808c0`, the
extension is ACTIVE, the staging marker is gone, and live introspection exposes
`ToggleResizeFocused(u,u)->(u,u,b)` alongside the unchanged fixed-size method.
Managed daemon capability probes succeeded. Observe-only HTTP status was clear.

Saved original XKB/source/MRU/current/IBus state as `keyboard-baseline.json` in
the private backup. Original XKB options were empty and source/engine Hangul.
Enabled the previously authorized temporary `fkeys:basic_13-24` option and
refreshed the source list to work around the host Shell reapply issue, restoring
its original Hangul-only list and MRU. Verified active keycodes 191/202 map to
F13/F24 and Hangul remains selected. Live startup completed without errors.

With Minecraft closed, saved and previewed the intended archived profile
(Tab → F3, B thin 250×1080). Preview correctly listed both `options.txt` and
`config/mcsr/standardsettings.json` for windowed preparation. Apply job
`dab79db5f5ca45b5bfa9b7a48dcded35` succeeded and returned backup
`000000000000000018e30c3f14907795`, with no keyd error. Verified
`fullscreen:false` in Minecraft options and boolean false in local Standard
Settings. HTTP mode is live, extension connected, no recovery/cleanup errors.

Asked the user to launch and perform B thin → full-size → thin, then Alt+Tab
away/back and B again, without F11. A read-only X11/status trace records only
Minecraft geometry/fullscreen state and runtime mode/errors in private
`toggle-trace.jsonl`. Temporary keyboard mapping and live mode remain enabled
for this supervised check; cleanup is still required afterward.


## Observed windowed transitions and focus recovery

The user confirmed repeated thin/full-size toggles work. The independent trace
captured several exact 250×1080 windows at 835,0 alternating with 1920×1080 at
0,0. Every captured game state had X11 fullscreen false. Successful replies were
published as `window_mode: preset` with active thin, or `full_size` with no
active thin preset. The mode clears on focus-generation changes as designed.

One early request was cancelled with `Minecraft focus lost or Shell modal opened
during resize`; subsequent repeated toggles succeeded. Preserve this observation
rather than claiming an error-free initial sequence.

Alt+Tab could not perform the focus switch because the active Tab → F3 remap
also consumes Tab in the Alt chord. Source routing currently assigns F3 sources
unconditionally to `layerk(mcsr_f3, f3)`. This is a separate desktop-shortcut
routing limitation, not a successful Alt+Tab recovery test. No routing code was
changed during acceptance.

Using GNOME's Super-key overview to choose another app and return, the user
confirmed Minecraft regains focus and B still toggles. This establishes focus
recovery through that route. The native fullscreen regression remains excluded
from the managed windowed workflow, and Alt+Tab requires its own fix/test.

Asked the user to check all five required F3 combinations with both remapped Tab
and native F3, including repeated F, without resize. Those results remain pending.
Live mode and the temporary keyboard map remain active for supervised testing.


## F3 priority results

The user confirmed all five required combinations work without resize, both
with Tab remapped to F3 and with native F3: B hitboxes, G chunk borders, repeated
F render-distance increase, Shift+F decrease, and Shift+F3 profiler pie. This is
visual GNOME/Minecraft evidence for those combinations on this candidate,
under the temporary F-key mapping. It does not resolve the Alt+Tab limitation.

Final chat checks now requested: Enter chat and command chat opened while Tab
is held must preserve b/g/f/s/w/q identity typing and cause no resize. Also
requested tap/release Tab followed by ordinary B, and unchanged s/w/q typing
outside Minecraft via the overview. Results and cleanup remain pending.


## Chat, release, and ordinary-input results

The user confirmed both Enter chat and command chat opened while Tab/F3 is held
preserve b/g/f/s/w/q typing without resize. After releasing a standalone Tab tap,
B resumes normal toggling. Switching out through the overview, s/w/q type
unchanged outside Minecraft. These checks passed on the actual desktop.

The supervised run therefore establishes repeated windowed thin/full-size
transitions, overview focus recovery, all five required combinations with both
F3 sources, both chat routes, release-resume behavior, and ordinary outside
input. Alt+Tab remains a failed acceptance case because remapping Tab to F3
also affects that desktop chord. This run does not establish eye behavior (no
eye preset configured), multi-monitor/scale cases, service crash/lock/login
while remapped, or a fix for native fullscreen/default F-key translation.

Requested normal game closure for exact backup restoration. Live remaps have
been stopped and the reviewed live-only override archived as
`10-live-keyd.test-complete.conf`. Original XKB/source/MRU/current/IBus state is
being restored from the private keyboard baseline before observe-only restart.
Profile backup Restore and final verification remain pending.


## Verified cleanup and final scope

The user saved and closed Minecraft normally. Game options and local Standard
Settings still matched their exact post-Apply hashes; no manual reconciliation
was required. Restore job `9dbc401923c04788a8a61caed07c8099` succeeded for backup
`000000000000000018e30c3f14907795`, without keyd error. Verified all restored
options/mod files against the recorded before-hashes. This also restored the
original local Standard Settings fullscreen preference, as an exact Restore
should; windowed preparation will be applied again before future live use.

Original XKB options (`@as []`), sources, MRU, current source, and Hangul engine
were restored and checked. Active X11 keycode 191 is XF86Tools and 202 is
unassigned again, matching the saved baseline. The live drop-in is archived and
no longer active. Final HTTP status is observe-only, extension connected, no
mutation/recovery/cleanup errors, active preset/window mode null, and no FIFO
errors. Lineage head is null, `/etc/keyd` is empty, and no uncertainty marker
exists. The bounded read-only trace completed.

Candidate remains installed but live mappings are disabled after this test.
No source commit, merge, push, or release occurred. Confirmed live scope is the
single-monitor XWayland thin/full-size path, overview focus recovery, both F3
sources/all five combinations, chat identity, standalone-release recovery, and
outside typing under the explicitly temporary F-key map. Alt+Tab and default
F-key translation still need fixes before unqualified desktop acceptance.
