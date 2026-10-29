# Managed panel visibility live test, 2026-10-30

Status: scoped live checks passed; cleanup verified complete.

The user requested supervised live testing of the reviewed candidate at
`.worktrees/feat-managed-panel-visibility/target/release/mcsr-toolkit`, SHA256
`11b73e7aa7715ae4abaae32789dacc227c4ab0fc8801d9c76e74a73dc2b5411f`.

Preflight: installed app/helper matched the earlier borderless-toggle candidate
`df034359d2ec42f7ab261b2fab97ed7451184210655070b334936c04f32808c0`.
The matching installed doctor reported ready. Service HTTP status was healthy
observe-only, extension ACTIVE/connected, no recovery/cleanup errors, and no
Minecraft process was found. Live keyboard activation remains off.

Saved the previous app/helper/managed-keyd, toolkit data and extension,
service definition, HTTP status and original keyboard/source/IBus settings to
`/home/undec/.local/state/mcsr-panel-live-20261030-hd1dm871` (private mode 0700).
Using the previously tested guided uninstall/bootstrap/install staging workflow.
No recovery records are removed to bypass preflight. The changed extension
requires a new GNOME session before repair. No temporary keyboard mapping has
been enabled during staging.

## Staged for login

Guided uninstall succeeded; authenticated fixed-path helper bootstrap succeeded.
Candidate install staged the new extension and returned the expected incomplete
installation message requesting logout/login. Verified `install-pending.json`
phase `shell-restart`, exact staged extension bytes, and candidate helper SHA256.
The toolkit service is inactive, distribution keyd is active, `/etc/keyd` is
empty, and XKB options remain `@as []`.

After a fresh login, resume with the exact candidate:

```sh
/mnt/data/ubuntu/Documents/mcsr/.worktrees/feat-managed-panel-visibility/target/release/mcsr-toolkit repair
```

Verify observe-only health and installed hashes before managed live preparation.
No panel live result is claimed from staging. Preserve the marker and backups.

## Post-login preparation

Exact candidate repair succeeded after the new login. Installed app/helper match
the candidate SHA256; extension ACTIVE and observe-only status clean. Saved
current keyboard baseline, then enabled the previously authorized temporary
`fkeys:basic_13-24` map with the source-refresh workaround. F13/F24 translations
verified, original Hangul-only source/MRU/current restored. Managed live service
started successfully.

The saved profile exactly matched the intended previous live profile (Tab→F3,
B thin 250×1080). Preview listed windowed preparation in options.txt and existing
local Standard Settings. Apply job `6b0921f4853e4f2da887be8e59ac85d5` succeeded, backup
`000000000000000018e30e9f7d95dc2b`, no keyd error. Verified options fullscreen:false.
Live HTTP/extension status is healthy. Temporary keyboard mapping and live mode
remain enabled for the supervised test and must be cleaned up afterward.

## First panel check

The user reported “It works!” in response to the requested B thin → full-size →
thin check, including disappearance of the top panel and usable game input in
the formerly covered top strip. This is a user-confirmed scoped pass.
App-switch/overview/modal restoration and cleanup remain pending.

The independent read-only X11 trace captured exact 250×1080 at 835,0 and
1920×1080 at 0,0; all captured game states had fullscreen false. At the first
result check, HTTP status showed one resize cancellation caused by focus loss
or Shell modal entry. Preserve this bounded cancellation rather than claiming
an error-free run. No panel-hook diagnostics appeared in the recent journal.
The service remains live with temporary translation enabled while awaiting
focus/overview and Run-dialog observations.

## Focus and modal observations

The user reported “All works” for both requested checks: Super/overview and
another app restore the panel, returning to Minecraft hides it again with B
still toggling; Alt+F2 restores it while the Run dialog is open and Esc returns
to hidden panel policy over the managed game. These supervised checks passed.
The user then confirmed Minecraft was saved and closed for exact cleanup.
Disable/re-enable while hidden, workspace gestures, lock/unlock, eye windows
and multimonitor behavior remain outside this live run.

## Verified cleanup

Restore job `2f4812fbf6a242399286aaef5a767495` succeeded for backup
`000000000000000018e30e9f7d95dc2b`, without keyd error. Independently verified
restored Minecraft options and local Standard Settings hashes against both
manifest before-hashes. Lineage head is null, `/etc/keyd` is empty, and no
mutation/uncertainty marker remains.

The live drop-in is archived in the private test backup. Original XKB options,
Hangul-only sources/MRU/current and Hangul engine were restored and checked.
Active X11 keycode 191 is XF86Tools and keycode 202 is unassigned again.
The first IBus restoration call inherited stale XWayland authorization from the
old session; rerunning with current DISPLAY/XAUTHORITY from the user manager
succeeded. No product-code change was required for that operational issue.
The read-only trace was stopped after collecting the test observations.

Final HTTP status: healthy observe-only, extension connected, no FIFO/runtime,
cleanup or recovery errors, null active preset/window mode. Candidate remains
installed; temporary mappings and live mode are disabled. Ordinary typing after
cleanup was not separately reconfirmed by the user; original keyboard settings
and empty keyd configuration were verified directly.

Accepted scope is single-monitor XWayland thin/full-size panel hiding, usable
top strip, overview/other-app restoration and return, Run-dialog restoration,
and continued B toggling. One focus-loss resize cancellation remains recorded.
Workspace gestures, disable/re-enable while hidden, lock/unlock, eye windows,
multimonitor/scale behavior and the earlier Alt+Tab/F-key/native-fullscreen
limitations retain their previous acceptance limits. No commit, merge or push
occurred.
