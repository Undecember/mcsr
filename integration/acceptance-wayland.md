# Wayland integration acceptance

## 2026-10-05 browser UI acceptance (Task 9)

- Ran the built-in API and static page in headless Google Chrome 154.0.8037.97 against a disposable MCSRLauncher root under `/tmp/mcsr-ui-acceptance-*`. The fixture used a fake keyd adapter and fake download responses; it did not access `/etc/keyd` or the real launcher instances.
- Keyboard Tab moved focus from the visible skip link to **Refresh status**. The browser discovered the fixture instance, saved a profile with crafting `s → k`, then rendered the API-provided preview row `one | s | k | key_key.back`.
- Apply became available only after Save and Preview. Its job reported the instance as `updated`, returned a backup ID, and changed the fixture `options.txt` from `key_key.back:key.keyboard.s` to `.k`. Restore with that ID reported `restored` and returned the file to `.s`.
- Both fake download jobs displayed their versions and destinations in separate panels; the first remained visible after the second finished. A second-tab profile save invalidated the first tab's preview: its Apply request received `stale_profile`, the UI disabled Apply, and the fixture file stayed unchanged. Changing the crafting source to reserved chat key `t` produced a validation error and left Apply disabled.
- Browser check result: **PASS** for navigation and the save → preview → apply → restore flow. This fixture validates the UI and API path; the separate live keyd and GNOME checks above cover desktop behavior.

Status: **PASS (Task 4 safety gate)**. Supervised focus/chat transitions, extension heartbeat recovery, empty-config restoration, ordinary typing, authentication password entry, and a 57-second post-restart soak were observed on 2026-10-05. Asynchronous focus delivery can leave a brief interval, and no direct `modal=true` sample was captured while the authentication dialog was open; successful password entry and focus clearing were observed, without a zero-leakage claim. This gate allowed Task 9 UI work to proceed; its browser evidence is recorded above.

## 2026-10-05 keyd helper recovery check

- Helper source and deployed `/usr/local/libexec/mcsr-toolkit-keyd` SHA256 matched: `6e167ad804f62b955fd63f2ef9b631e9b92a53375b17fb1ba734e7836ca4478a`.
- Deployed helper ownership and mode: `root:root`, `0755`.
- Installed sudoers rule: `undec ALL=(root) NOPASSWD: /usr/local/libexec/mcsr-toolkit-keyd clear`; ownership and mode: `root:root`, `0440`. Both the candidate and installed file parsed with `visudo`.
- The starting `/etc/keyd` configuration was empty. A chat-only configuration was installed through the authenticated helper. keyd restarted, parsed the configuration, remained active, and showed no SIGSEGV for more than 35 seconds.
- The fixed clear sequence succeeded: `keyd bind reset`, then `/usr/bin/sudo -n /usr/local/libexec/mcsr-toolkit-keyd clear`. The helper restarts `keyd.service` and checks its health.
- After toggling the GNOME extension, its heartbeat reconnected to the observe-only service. No dynamic game bindings were tested.

## 2026-10-05 supervised live focus and chat check

The earlier authentication note in this section was superseded by the later supervised input check below.

- Environment recorded in the probe log: Ubuntu 26.04.1 LTS, GNOME Shell 50.1 on Wayland, keyd 2.5.0-5, Minecraft 1.16.1 (`MCSR_Ranked`). The service reported live mode, Minecraft focused, extension connected, keyd active, and no errors. Dynamic activation counters advanced.
- `keyd listen` showed `+mcsr_chat`/`-mcsr_chat` for `T`/`Esc` and `/`/`Esc`; the user confirmed both routes. Blurring during open chat showed `+mcsr_chat` then `-mcsr_chat`. The user confirmed `t` and `/` typed normally outside Minecraft before and after.
- GNOME Alt+F2 produced `modal=true` and cleared effective Minecraft focus without remap errors. Closing it returned `modal=false`; refocusing Minecraft reactivated the profile.
- Disabling the production extension caused heartbeat loss and status `focused=false`, `connected=false`; the clear counter advanced while keyd remained active. Re-enable restored `focused=true`, `connected=true`; the activation counter advanced. Transient D-Bus object/interface errors occurred only during intentional disable, and the service recovered.
- Normal shutdown followed by `PrivilegedKeydAdapter.restore()` succeeded. `/etc/keyd` was empty afterward; keyd was active and ignoring the keyboard. No SIGSEGV was seen. Systemd status 15 during restart was expected. After baseline restoration, the user confirmed that typing the literal `t/s` produced those characters normally.
- A second supervised session confirmed `T` opened chat and `Enter` closed it; `keyd listen` showed `+mcsr_chat` then `-mcsr_chat`.
- A real GNOME password dialog appeared over focused Minecraft. The user said it did not affect typing, but the operator had asked them not to type; whether they entered a password or cancelled is awaiting clarification. The toolkit was queried after the prompt closed, so no `modal=true` was captured directly. The focus clear counter advanced during the request, then the profile reactivated.
- Normal shutdown and `PrivilegedKeydAdapter.restore()` succeeded again; `/etc/keyd` was empty afterward. After the final restart at 01:55:14, keyd remained active for 56 seconds with no SIGSEGV or new restart loop.

## 2026-10-05 supervised authentication input check

- The initial `/etc/keyd` configuration was empty. The helper hash and exact sudo scope had already been validated in the recovery check above.
- A chat-only configuration was installed. After `MCSR_Ranked` was visible, the service was started in live mode; the GNOME extension needed a disable/enable cycle to reconnect. Before authentication, status showed live mode, Minecraft focused, extension connected, no errors, and keyd active.
- The operator ran `pkcheck --revoke-temp; sleep 5; pkexec /usr/bin/true` in a terminal, then switched to Minecraft. A real GNOME password dialog appeared; the user entered their password and the command completed without error.
- The service counters changed from activate 3 / clear 3 before authentication to activate 4 / clear 5 afterward. Since the dialog could not be held open while messaging, no direct `modal=true` sample was captured for it. Earlier Alt+F2 testing captured `modal=true` directly.
- The live service was stopped and `PrivilegedKeydAdapter.restore()` succeeded. `/etc/keyd` was empty, keyd was ignoring the keyboard, the probe active profile was removed, and the user confirmed literal `t` and `s` typed normally.
- The final restart at 17:44:29 KST remained active through 17:45:26 (57 seconds), with no SIGSEGV or new restart loop. systemd prior-process status 15 on restart was expected. No adapter, active, pending, or marker state files remained.
- Limitation: asynchronous focus delivery can leave a brief interval. The observed successful password entry and focus clear do not establish zero input leakage.

## Acceptance result

Task 4 safety gate: **PASS** with the measured limitation above. The live authentication input and cleanup observations are recorded. Task 9 browser evidence is recorded at the top of this document.

## Task 10 documentation and final verification, updated 2026-10-06

Task 9 browser evidence is recorded at the top of this document. The following safe deterministic checks ran from the feature worktree root on the target Ubuntu GNOME Wayland device after the final mapping fixes and review. The README's privileged installation and user-service commands were not freshly executed or verified in Task 10.

| Check | Tool version | Command | Result |
| --- | --- | --- | --- |
| Python test suite | Python 3.14.4, pytest 9.1.1 | `.venv/bin/python -m pytest -q` | **PASS** — 246 passed; one Starlette deprecation warning about using `httpx` with `starlette.testclient` |
| Ruff | 0.16.10 | `.venv/bin/ruff check .` | **PASS** — all checks passed |
| mypy | 1.20.2 | `.venv/bin/mypy src` | **PASS** — no issues in 16 source files |
| GNOME extension harness | Node.js v24.21.0 | `node tests/extension_harness.cjs` | **PASS** — extension behavior checks passed |
| UI harness | Node.js v24.21.0 | `node tests/ui_harness.cjs` | **PASS** — restore refreshes the canonical profile, preserves an unsaved draft, and preserves edits made during the in-flight refresh |
| Schema validation | GLib 2.88.0 | `glib-compile-schemas --strict --dry-run extension/schemas` | **PASS** — no diagnostics |
| Whitespace check | Git | `git diff --check` | **PASS** |

The restricted keyd fragment generation and validation using `render_keyd_fragment(Profile())` and `validate_fragment()` passed during the initial Task 10 checks (Python 3.14.4); this validator was not rerun with the latest suite. The broad independent review passed the five final mapping fixes.

The latest full suite includes the restore-lineage and mapping-fix cases: restoring B then A returns through the saved states in order; stale, legacy, inconsistent, externally changed, and incompatible lineage cases are rejected before mutation. The README documents this current-head restore history and backup preservation guidance.

No live keyd daemon candidate check or user-service change was run for Task 10. The GNOME session was temporarily locked while the extension was inactive, so a fresh locked-session check was not completed; the session is now unlocked and `mcsr-toolkit@local` reports `ACTIVE`. This current extension state does not establish locked-session behavior. An earlier observe-only F6 observation that day had no direct frame sample; the later measured F6 run is recorded below and in [resize-wayland.md](resize-wayland.md). The real options file and `/etc/keyd` were unchanged during the earlier observation, and its service was stopped. The separately dated supervised live focus/chat/authentication observations above and browser fixture evidence at the top are prior evidence, not Task 10 reruns. The `pytest` run includes controlled extension boundary tests, not real GNOME focus, physical input, resize, or maccel behavior.

## 2026-10-06 approved helper deployment and current state

- After separate approval, the one-line privileged validator update was installed with `pkexec /usr/bin/install` using an absolute source path. The current source and `/usr/local/libexec/mcsr-toolkit-keyd` SHA256 values both equal `924cb9c10b14f9b9a50f4acbe062d45fa9d093c5efbe037042bd2485e93ce339`; the installed file is `root:root`, mode `0755`.
- After deployment, `/etc/keyd` remained empty, keyd was active, and the toolkit user service was inactive. The exact passwordless rule remained `undec ALL=(root) NOPASSWD: /usr/local/libexec/mcsr-toolkit-keyd clear`.
- The production GNOME extension's installed source hash matched the updated source. After unlocking the session it was enabled and reported `ACTIVE`.
- This helper deployment did not test or enable live key mappings. The later supervised movement test is recorded below; `maccel` remains absent, so sensitivity acceptance is still pending.

## 2026-10-06 approved movement-test attempt

- The user separately approved a supervised physical `s` → emitted `k` → Minecraft Backward test with restoration afterward. Before activation, the only discovered instance was `MCSR_Ranked`, Minecraft was closed, `/etc/keyd` was empty, the toolkit user service was inactive, the production GNOME extension was `ACTIVE`, and the reviewed helper passed `preflight_keyd_clear()`.
- A private probe data directory kept the normal toolkit profile and state untouched. An independent byte snapshot of the real `options.txt` had SHA256 `2a9f9cfcd9ffea1c112348393115c70da78d313083c84ec18cf877b908c8ec48`. Preview reported one ready instance and exactly `s` → `k` → `key_key.back`, with no control conflicts.
- The live service applied the profile successfully (backup `81a6e06e2d744741b0aec43ef291dc46`). The real Backward binding became `key.keyboard.k`; the dedicated keyd config was installed, keyd remained active past the earlier crash interval, and the bridge was connected with no reported errors. The service observed no Minecraft focus event or dynamic activation while waiting for the user to open the game. **Physical Backward movement was not tested in this attempt.**
- To avoid leaving a temporary live setup in place, the live service was stopped and the same backup was restored through the observe-only API. Restore reported success. The real `options.txt` again matched the independent baseline SHA256 exactly, Backward was `key.keyboard.s`, `/etc/keyd` was empty, the keyd adapter state was absent, and both toolkit services were stopped. keyd remained active with zero restarts for at least 55 seconds after that restoration. Ordinary typing was confirmed during the second attempt below.

## 2026-10-06 supervised movement test, second attempt

- The first attempt above was restored to its exact baseline after Minecraft focus and dynamic activation were not observed. For the second attempt, a private `XDG_DATA_HOME` isolated toolkit state and `MCSR_Ranked` was the sole discovered instance. Preview showed physical `s` → emitted `k` → `key_key.back`; Apply succeeded with backup `2e4dff7e52094aedb84ce9fcf81df85e`.
- The real options file showed Backward=`k`. Live focus status had `activate=2`, `clear=2`, and no errors. The user confirmed physical `S` moved Backward in Minecraft. After Minecraft blurred, the clear count advanced to 3; the user confirmed `s` and `t` typed literally outside the game.
- After the game closed, the options file hash matched the expected post-Apply hash, with only the Backward line changed. The live service was stopped and observe-only Restore succeeded. It restored the original options SHA256 exactly: `2a9f9cfcd9ffea1c112348393115c70da78d313083c84ec18cf877b908c8ec48`; Backward was `s` afterward.
- Cleanup left `/etc/keyd` empty; adapter, active, mutation, and pending state files absent; and both toolkit services stopped. The user confirmed `s` and `t` typed literally after Restore. keyd remained active with `NRestarts=0` for more than 60 seconds after restart.
- Result: **PASS** for the supervised `s` → `k` Minecraft Backward movement, focus-loss clear, and exact options/keyd restoration. This does not complete live window-resize acceptance or maccel sensitivity acceptance; `maccel` remains unavailable.

## 2026-10-06 exact observe-only F6 resize acceptance

- A fresh `XDG_DATA_HOME` isolated an observe-only toolkit service on port 8877. The temporary profile configured `normal` on F6 for 800×600 with `eye_zoom=false`. Status reported `live_activation_enabled=false`, `extension_connected=true`, no service or FIFO errors, and `/etc/keyd` remained empty.
- With `MCSR_Ranked` focused, GNOME `GetFocus` reported 854×517 and `modal=false`. The F6 action emitted `PresetActivated ('normal', false)`. Frame samples showed 854×517, then 800×600 within about 0.55 seconds of the signal; multiple subsequent samples remained 800×600 while the class still identified Minecraft and `modal=false`. The user confirmed the visible resize.
- `active_preset` reported `normal` during initial 800×600 samples, then later became `null` while the frame remained 800×600 and no errors were reported. The cause is unknown; this status discrepancy is retained without attributing a cause.
- The service was stopped and `ConfigurePresets '[]'` returned `()`. The service did not apply a profile or edit Minecraft controls. After Minecraft closed, Backward remained `key.keyboard.s`; the only difference from the pre-resize options snapshot was Minecraft's saved `soundCategory_music:1.0` → `0.0`, which was preserved. Result: **PASS** for the measured normal F6 800×600 focused-window resize with keyd activation disabled. Eye-zoom sensitivity remains untested because `maccel` is absent. The locked-session check also remains unperformed.

## Earlier automated recovery verification, 2026-10-05 (203-test run)

- The full automated test suite passed: 203 tests. Ruff, mypy, the Node extension harness, GLib schema validation, and `git diff --check` also passed. These verify code and packaging behavior, separate from the live desktop observations above.
- With keyd clear failure during Restore, the Restore job fails before writing instance files or invoking authenticated `pkexec`. The recovery marker and profile remain available for a later clear attempt.
- An independent Sol high review passed for the recovery paths after fixes.
- On observe-only service restart, persisted old live bindings are cleared once and the persisted profile is dropped; startup does not activate bindings.
- This regression result is separate from the live authentication input observation and the Task 4 safety gate result above.

The observed clear sequence was `keyd bind reset` followed by `/usr/bin/sudo -n /usr/local/libexec/mcsr-toolkit-keyd clear`. Configuration installation and `PrivilegedKeydAdapter.restore()` used the authenticated helper path; the passwordless sudo rule permits only `clear`. Keep the physical Backspace + Escape + Enter keyd panic sequence available during any remaining supervised testing.

Resize and sensitivity acceptance is tracked separately in [resize-wayland.md](resize-wayland.md).

## 2026-10-22 maccel recovery preparation — live acceptance pending

- Read-only inspection now finds `/usr/local/bin/maccel`, CLI version `maccel-cli 0.6.0`, multiplier `1.000000000`, raw `/sys/module/maccel/parameters/SENS_MULT` value `4294967296`, and driver fingerprint `4AFA0F5B3EBD2F98EE54E56`. The parameter is `root:maccel` mode `0664`; the current `undec` login groups do not include `maccel`. `/var/opt/maccel/resets` is `root:maccel` mode `0775`. A successful CLI read does not establish write permission.
- The earlier statements that maccel was absent describe the October 6 observations. They remain historical evidence; the movement and measured F6 results above are unchanged.
- The new fixed helper and exact two-command sudoers template are implemented, but `/usr/local/libexec/mcsr-toolkit-maccel` and `/etc/sudoers.d/mcsr-toolkit-maccel` are absent. No helper installation, sudoers installation, permission probe, or live eye write was performed in this documentation preparation. No keyd configuration was changed.
- **PENDING:** separate security approval after independent source/deployment review; helper deployment and verification; focused F7 400×300 with about `0.02083`; both shifted cases; exact F6 baseline recovery; separate blur, Shell modal, disconnect, shutdown, and startup recovery cycles. Use the [updated setup and live procedure](resize-wayland.md#maccel-helper-setup-and-recovery). No authentication-field input is part of this probe.
- Cleanup acceptance must confirm the original raw and CLI values, root baseline absent, shortcuts cleared, toolkit services stopped, and `/etc/keyd` unchanged. These are future live acceptance criteria, not new observed results.

## 2026-10-22 approved live eye attempt — recovery failed

- After separate security approval, helper SHA256 `78f74b7c6c621be292c65312a9a084d1743a203abb148087023f2310c7837077` and the exact `undec` two-command sudoers rule were installed and verified for hash, root ownership, and `visudo`. The empty-token permission probe and initial restore succeeded without changing baseline raw `4294967296` / CLI `1.000000000`.
- A fresh isolated observe-only service used F6 normal 800×600 and F7 eye 400×300, with a background raw/CLI/frame/focus sampler. F6 reached 800×600 at raw `4294967296`; Shift+F7 reached 400×300 with that baseline unchanged. F7 reached 400×300/raw `89464169` (about `0.02083`). No Apply or live keyd activation was used.
- **FAIL for helper readback/recovery:** the old helper wrote `raw + LF`, while this driver retained that LF and appended another on reads. Eye was physically written but rejected by strict readback; restoration likewise failed. Direct separately approved recovery restored numeric raw `4294967296` / CLI `1.000000000`. This does not establish successful helper restoration.
- Toolkit service and sampler were stopped, GNOME shortcuts cleared, and `/etc/keyd` left empty. The root baseline record remains because the installed old helper still fails restore. Keep the service and eye disabled and preserve that record; cleanup acceptance is incomplete.
- Corrected source SHA256 `4b75d58cfa14a3de4037a95e65b2b55f0a97c204b0c10b1b06883efa57192680` writes raw digits without LF and accepts the observed legacy double-LF parameter state for recovery, without relaxing durable record parsing. Its 399-test result and independent security review are source verification, not live replacement acceptance. The installed helper remains the old hash. Fresh explicit approval is required to replace this exact source, then successfully consume the retained baseline record through verified restore.
- **PENDING:** approved replacement, exact helper recovery and absent root baseline, then a fresh full eye/restoration matrix including Shift+F7 during eye and separate F6, blur, Shell modal, extension disconnect, shutdown, and startup recovery cycles. The earlier movement/F6 evidence remains unchanged. See [replacement and recovery](resize-wayland.md#maccel-helper-setup-and-recovery).

## 2026-10-22 corrected-helper acceptance and shutdown repair

Result: **PASS for the supervised resize, sensitivity, and recovery cases below after both fixes**. The initial newline failure and first failed SIGTERM remain failed attempts; later successful tests supersede them for the corrected code. This acceptance covers Ubuntu GNOME Wayland, Minecraft 1.16.1/MCSR Ranked, the reviewed driver fingerprint, and the recorded baseline. It does not establish zero sensitivity leakage or acceptance on other drivers/sessions.

After fresh explicit replacement approval, the corrected helper was installed root:root mode `0755`, with source/installed SHA256 `4b75d58cfa14a3de4037a95e65b2b55f0a97c204b0c10b1b06883efa57192680`. Exact helper restore succeeded and authenticated root inspection confirmed its baseline record absent before the new run. The exact `undec` eye/restore sudoers rule was retained. A fresh isolated `XDG_DATA_HOME` used an observe-only service on port 8877, F6 normal 800×600, and F7 eye 400×300. No Apply or live keyd activation was used.

The sampler contains 5,345 records from 12:42:54.202 to 13:04:10.729 UTC. It reads raw sensitivity, CLI multiplier, and GNOME focus/frame in sequence, with API status every fourth sample and a 0.2-second sleep between iterations. Times below identify sample starts, not atomic snapshots or exact driver-change times. Shortcut/action identities and deliberate signal times come from the supervised operator sequence.

| Case | Observed evidence (UTC, 2026-10-22) | Result |
| --- | --- | --- |
| Normal F6 and shifted eye from baseline | Initial approved run measured 800×600 on F6 and 400×300 on Shift+F7, both at raw `4294967296`; retained above. Corrected-helper run also sampled focused 400×300 at baseline before F7 and 800×600 at baseline after F6. | PASS for baseline sizing/unchanged sensitivity |
| F7, Shift+F7 during eye, then F6 | Eye at 400×300/raw `89464169`, CLI `0.020830000`, from 12:43:56.552. The supervised shifted action preserved eye sensitivity. F6 reached 800×600/raw `4294967296`, CLI `1.000000000`, at 12:44:09.835. | PASS |
| Blur in a separate eye cycle | At 12:54:25.962, raw still eye while the later focus read identified the terminal; raw/CLI baseline was sampled at 12:54:26.380. | PASS, with observable asynchronous interval |
| Shell modal in a separate eye cycle | Eye sampled from 12:47:52.119; at 12:47:56.069, Minecraft frame stayed 400×300, `modal=true`, raw/CLI baseline restored. | PASS |
| Extension disconnect in a separate eye cycle | Operator disabled extension at 12:55:25.650 while keeping Minecraft focused. Raw/CLI baseline sampled at 12:55:25.791. Exported focus object was unavailable as expected; API status at 12:55:26.424 reported `extension_connected=false`, active preset cleared, and no sensitivity recovery pending. | PASS |
| SIGTERM after shutdown fix | Signal sent 13:02:49.020. Eye sampled through 13:02:49.133; baseline sampled at 13:02:49.345 while Minecraft remained focused at 400×300. Service stopped and its log completed application shutdown. | PASS on retest |
| SIGKILL followed by startup recovery | Kill at 13:03:43.970, replacement service spawned 13:03:44.070. Sampler still saw eye at 13:03:44.588, then exact raw/CLI baseline at 13:03:44.800 with Minecraft focused at 400×300. Operator independently observed baseline at about 13:03:44.775. API at 13:03:45.013 reported usable sensitivity, no recovery pending, and no errors. | PASS |

The **first SIGTERM attempt failed** before the shutdown fix: around 12:56:11 the server had stopped, but raw remained `89464169`, including while Minecraft stayed focused. It returned to baseline only after the operator invoked the exact helper `restore` (first sampler baseline at 12:56:31.000). Uvicorn replayed its termination signal before cleanup in the outer `finally` could finish. Cleanup now runs through application lifespan shutdown; queued jobs drain before the service lock is released, and early-startup failure cleanup remains covered. The orchestrator reports 401 automated tests passing and independent review PASS for this fix. `service-retest.log` and `service-postcrash.log` record completed application shutdown; `service-crash.log` ends at the intentional SIGKILL. These source checks complement the successful live retests.

Final operator checks: `ConfigurePresets '[]'` returned `()`; every acceptance service and the sampler stopped; port 8877 closed; raw `4294967296` and CLI `1.000000000` matched the pre-test baseline; authenticated `pkexec /usr/bin/test ! -e /run/mcsr-toolkit-maccel/baseline` exited 0; `/etc/keyd` remained empty. The real Minecraft options SHA256 matched its starting value exactly: `e578e653d787df964641e43c1af68d125b4c8e14ce1f10fdb284229c9a789642`. The corrected helper and exact rule remain installed. No authentication-field input was used in the focused live probe; deployment/root inspection used separate authenticated operations.

Evidence: `/tmp/mcsr-maccel-repaired-live-20261022-t403465o/samples.jsonl`, `baseline.json`, `service.log`, `service-retest.log`, `service-crash.log`, `service-postcrash.log`, and `.superpowers/sdd/2026-10-22-maccel-sensitivity-recovery/live-preparation-repaired.json`. The metadata records initial readiness, not final cleanup; the final checks above are operator-supplied evidence. Temporary logs may be removed by normal `/tmp` cleanup, so this dated record preserves the relevant samples and outcomes.

GNOME focus/modal delivery and the sampler are asynchronous. The blur observation explicitly shows eye sensitivity and terminal focus in one sequential sample before the next baseline sample. These results establish eventual restoration for the tested transitions, with a brief observed interval; they do not prove zero leakage, atomic event timing, or locked-session behavior. Earlier movement/F6 observations and the outstanding locked-session limitation remain unchanged.
