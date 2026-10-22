# GNOME Wayland focus probe

Status: window identification, focus delivery, exact resize, observe-only heartbeat, and supervised chat/focus transitions passed. The 2026-10-04 failed probe remains historical. On 2026-10-05, supervised password entry in a real GNOME authentication dialog completed successfully while the live profile was active; the service clear counter advanced during the transition. The asynchronous focus delivery leaves a brief possible interval, and a direct `modal=true` sample was not captured during that dialog.

## Initial environment at first inspection (2026-10-03)

- Ubuntu 26.04.1 LTS, GNOME Shell 50.1, Wayland.
- Ubuntu `keyd` package 2.5.0-5 provided `/usr/bin/keyd.rvaiya`; its daemon was active at that inspection.
- This build does not expose upstream `keyd check`.
- `keyd-application-mapper` was a separate package and was not installed at that inspection; it was installed later.
- The packaged keyd GNOME extension metadata lists Shell 45 and 46, so it cannot be enabled as packaged on Shell 50.
- The login at that inspection was not in the `keyd` group and could not access `/run/keyd.socket`; group membership was added later.
- No Minecraft window was running during the initial inspection.
- MCSRLauncher started a managed `MCSR_Ranked` instance, but Minecraft crashed before creating `minecraft/options.txt`: `mcsrfairplay-public`/JNativeHook could not load `libxkbcommon-x11.so.0`. Ubuntu's `libxkbcommon-x11-0` package was absent then and was installed later.
- The temporary GNOME 50 probe extension was installed in the user extension directory, but the Shell session at that inspection had not registered its UUID. `gnome-extensions enable mcsr-toolkit-probe@local` reported that it did not exist; a fresh login later registered it.

## Observed after GNOME extension activation

- `gnome-extensions info mcsr-toolkit-probe@local` reports `State: ACTIVE` on GNOME Shell 50.1.
- Focused Minecraft reports class and title `Minecraft* 1.16.1 - MCSR Ranked`. The application mapper normalizes that class to `minecraft-1-16-1-mcsr-ranked`; the `minecraft*` class filter matches it.
- The probe observed a frame of 854×517, requested 800×600 using `ResizeFocused`, and then observed exactly 800×600 from `GetFocus`.
- The same resize call returned `false` with a focused `org.gnome.Ptyxis` terminal; terminal frame remained 1920×1048.
- `keyd-application-mapper` was installed by the time of this check. Although group membership was recorded in `/etc/group`, the GNOME and user-manager processes at mapper-probe time still lacked group 973. A temporary authenticated process dropped to the user identity with refreshed groups for the mapper probe; normal production startup required a refreshed user session.
- With the probe extension feeding `keyd.fifo`, the application mapper logged terminal → Minecraft → terminal → Minecraft and normalized the Minecraft class to `minecraft-1-16-1-mcsr-ranked`.

## Failed chat safety test and cleanup

- A temporary `/etc/keyd/mcsr-probe.conf` and user `app.conf` mapped `s = k` for Minecraft and used `togglem(mcsr_chat, t)`/`togglem(mcsr_chat, slash)` for chat. The daemon parsed the config without warnings and matched the built-in keyboard.
- A GNOME authentication dialog appeared while Minecraft was focused. The probe extension only listened for `notify::focus-window`; the dialog did not cause the mapper to reset. The user's password typing was affected by the remap, so the live test was stopped immediately. The user confirmed normal typing after the mapper reloaded an empty `app.conf` section.
- The temporary root config was removed, `keyd.rvaiya reload` returned to the prior no-config state, the mapper and layer listener were stopped, the temporary user `app.conf`/lock file were removed, and the probe extension was disabled. Journal output shows the keyboard is ignored again by keyd.
- The package's GNOME extension listens for `system-modal-opened`, which the temporary probe omitted. A production focus observer must treat Shell modals as non-Minecraft input context, including restoration when they close. `bind reset` still cannot clear a toggled chat layer, so a separate state-clear mechanism is required. Do not activate another remap until both transitions are verified without remapping authentication input.

## Revised modal probe, 2026-10-04

- After a fresh GNOME login, the revised probe's `GetInputContext` returned `('org.gnome.Ptyxis', false)` with no Shell modal.
- Opening GNOME's Alt+F2 Run dialog returned `('google-chrome', true)`: the underlying focus window remained visible to GNOME, while `Main.modalCount` identified the modal context.
- Closing the Run dialog with Escape returned `modal=false`. No keyd configuration or dynamic remap was active during this check.
- A production observer can use `system-modal-opened` for immediate blur and a short heartbeat to detect modal close. The supervised authentication check below observed focus clearing during password entry, but asynchronous delivery can leave a brief interval and did not capture the auth dialog's modal state directly.

## Production observe-only service, 2026-10-04

- The production `mcsr-toolkit@local` extension loaded on GNOME Shell 50.1. A first service run rejected malformed FIFO bytes from the extension's asynchronous writer; no keyd calls were made. After replacing the writer with refcounted `GLib.Bytes`, a standalone native GJS test delivered 20 exact JSON messages, including across a reader restart. The corrected extension was installed and loaded after a new GNOME login.
- The corrected extension and service then reported valid focus events with no FIFO errors. With Minecraft focused, service status showed `minecraft_focused=true` and `mode=observe-only`. Opening GNOME's Alt+F2 Run dialog changed the effective state to false; closing it and focusing Minecraft returned it to true. Disabling the extension while Minecraft was focused caused the heartbeat watchdog to clear the state after two seconds.
- The service was run without `--enable-live-keyd`; `/etc/keyd` remained empty. These results establish the focus signal path and fail-closed watchdog, but they do not prove that live dynamic bindings and chat state reset before authentication input. The stale extension-availability status after disabling the extension was fixed and live retested.

## Failed supervised chat-only probe, 2026-10-04

- A root-owned helper was installed for the supervised probe. The temporary keyd configuration contained only the Minecraft chat layer and dynamic `T` and `/` bindings.
- At 19:31:47 KST the daemon parsed the configuration; at 19:32:26 KST it matched the built-in keyboard. At 19:32:52 KST the daemon logged a segmentation fault, before keyd activation had been observed.
- The user reported that `T` and `/` opened chat, but this was vanilla Minecraft behavior and does not demonstrate that the dynamic bindings worked.
- The service was stopped, the empty `/etc/keyd` state was restored, and keyd was restarted. It is active and ignoring the keyboard. The user probe state was removed. No remaps are active.
- The probe failed. Keep the UI gated until a supervised test establishes that keyd activation and the chat/focus transitions work safely.

## Restart-helper recovery check, 2026-10-05

- The reviewed helper source and deployed `/usr/local/libexec/mcsr-toolkit-keyd` had matching SHA256 `6e167ad804f62b955fd63f2ef9b631e9b92a53375b17fb1ba734e7836ca4478a`. The deployed helper was `root:root` mode `0755`.
- The installed sudoers rule was exactly `undec ALL=(root) NOPASSWD: /usr/local/libexec/mcsr-toolkit-keyd clear`, owned by `root:root` mode `0440`. Both the candidate and installed rule parsed successfully with `visudo`.
- An initially empty `/etc/keyd` baseline was checked. A chat-only configuration was installed through the authenticated helper. The daemon restarted and parsed the configuration, remained active, and showed no SIGSEGV for more than 35 seconds.
- The fixed reset and restart sequence succeeded: `keyd bind reset`, followed by `/usr/bin/sudo -n /usr/local/libexec/mcsr-toolkit-keyd clear`. The helper's `clear` action performs a bounded restart and health check.
- After toggling the GNOME extension, its heartbeat reconnected to the observe-only service. No dynamic game bindings were tested. Minecraft live proof remains pending; these recovery observations do not establish focus/chat safety or pass the UI gate.

## Supervised live focus and chat check, 2026-10-05

- Environment: Ubuntu 26.04.1 LTS, GNOME Shell 50.1 on Wayland, keyd 2.5.0-5, Minecraft 1.16.1 (`MCSR_Ranked`). The chat-only profile was active through the validated root helper and sudoers setup recorded above.
- With Minecraft focused, the live service reported mode `live`, extension connected, keyd active, and no errors. Dynamic activation counters advanced during the focus and recovery checks.
- `keyd listen` showed `+mcsr_chat` then `-mcsr_chat` for both `T`/`Esc` and `/`/`Esc`. The user confirmed both chat routes worked.
- Switching away while chat was open showed `+mcsr_chat` followed by `-mcsr_chat`. The user confirmed `t` and `/` typed normally outside Minecraft both before and after this transition.
- Opening GNOME Alt+F2 over Minecraft reported `modal=true` and cleared effective Minecraft focus without remap errors. Closing it reported `modal=false`; refocusing Minecraft reactivated the profile.
- Disabling the production extension caused heartbeat loss: status changed to `focused=false`, `connected=false`, and the clear counter advanced while keyd remained active. Re-enabling the extension restored `focused=true`, `connected=true`, and advanced the activation counter.
- Transient D-Bus object/interface errors appeared only during intentional extension disable; the service recovered after re-enable. During normal shutdown, `PrivilegedKeydAdapter.restore()` succeeded, `/etc/keyd` returned to empty, and keyd was active and ignoring the keyboard. No SIGSEGV was seen; systemd status 15 during restart was expected. After restoration, the user confirmed exact `t` and `s` input appeared normally.
- No authentication dialog was opened during this first live run. The Alt+F2 Run dialog verifies Shell modal handling, but does not prove that password entry is unaffected by remaps.

## Second supervised session, 2026-10-05

This earlier observation was superseded by the later supervised authentication input check below.

- The user confirmed `T` opened Minecraft chat and `Enter` closed it; `keyd listen` showed `+mcsr_chat` followed by `-mcsr_chat`.
- A real GNOME password dialog appeared over focused Minecraft. The user said it did not affect typing, but the operator had asked them not to type; whether a password was entered or the dialog was cancelled is awaiting clarification.
- The toolkit was queried after the prompt had closed, so no `modal=true` state was captured directly. The focus clear counter advanced during the request, then the profile reactivated after focus returned.
- Normal service shutdown and `PrivilegedKeydAdapter.restore()` succeeded again. `/etc/keyd` was empty afterward. Following the final restart at 01:55:14, keyd remained active for 56 seconds with no SIGSEGV or new restart loop.

## Supervised authentication input check, 2026-10-05

- The initial `/etc/keyd` configuration was empty. The helper hash and exact sudo scope had already been validated as recorded above.
- A chat-only configuration was installed, `MCSR_Ranked` was made visible, and the service was started in live mode. The GNOME extension needed a disable/enable cycle to reconnect. Before authentication, status showed live mode, Minecraft focused, the extension connected, no errors, and keyd active.
- From a terminal the operator ran `pkcheck --revoke-temp; sleep 5; pkexec /usr/bin/true`, then switched to Minecraft. A real GNOME password dialog appeared. The user entered their password, and the command completed without error.
- The service clear counter advanced during the transition: activate 3 / clear 3 before authentication, then activate 4 / clear 5 afterward. The dialog could not be held open while messaging, so no direct `modal=true` sample was captured for this authentication dialog. Earlier Alt+F2 testing did capture `modal=true` directly.
- The live service was stopped and `PrivilegedKeydAdapter.restore()` succeeded. `/etc/keyd` was empty, keyd was ignoring the keyboard, the probe active profile was removed, and the user confirmed literal `t` and `s` typed normally.
- The final keyd restart at 17:44:29 KST remained active through 17:45:26 (57 seconds) with no SIGSEGV or new restart loop. systemd's prior-process status 15 on restart was expected. No adapter, active, pending, or marker state files remained.
- This passes the Task 4 safety gate. Asynchronous focus delivery can leave a brief interval, and direct auth-modal state was not sampled; successful password entry and focus clearing were observed, so this is not a claim of zero input leakage.

## Chat focus risk

Inspection of keyd's `kbd_eval("reset")` shows it copies the original config without clearing active toggled layers. The supervised check observed the active chat layer clear on focus loss, Shell modal entry, and extension heartbeat loss. The approved clear sequence first sends `keyd bind reset`, then invokes `/usr/bin/sudo -n /usr/local/libexec/mcsr-toolkit-keyd clear`; the root-owned helper restarts `keyd.service` and checks its health. This restart affects every keyd device. Ordinary typing was confirmed in other apps and after restoring the empty baseline; supervised authentication password entry completed successfully, with the measured asynchronous focus-delivery limitation recorded above.

The focus, chat, Alt+F2 modal, extension heartbeat loss/recovery, empty-config restore, ordinary typing, and supervised authentication input checks above were observed on 2026-10-05. Keep the physical Backspace + Escape + Enter keyd panic sequence available during supervised live testing. The acceptance result and measured limitation are recorded in [acceptance-wayland.md](acceptance-wayland.md).

The earlier mapper and user `app.conf` probe have been removed and must not be reinstated for this procedure.
