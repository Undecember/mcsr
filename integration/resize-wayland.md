# GNOME Wayland resize and sensitivity acceptance

This procedure tests focused Minecraft resizing without enabling keyd mappings. Use a GNOME 50 Wayland session with Minecraft and a terminal open. Presets use frame dimensions, in pixels, from Mutter. Supported requested dimensions are 100–16384 per axis; a client may impose stricter limits. Fullscreen or maximized windows may refuse an exact size; use a normal window for the successful case.

## Earlier evidence and pending checks (2026-10-04–06)

On 2026-10-04 the production `mcsr-toolkit@local` extension reported `State: ACTIVE` in GNOME Shell 50.1. With MCSR Ranked focused, `GetFocus` reported 854×517, `ResizeFocused 800 600` returned `(uint32 800, uint32 600)`, and the next `GetFocus` reported 800×600. The same resize method refused a focused terminal.

After registering F6 for `normal` and F7 for `eye`, the production extension emitted `PresetActivated ('normal', false)`, `('eye', false)`, and `('eye', true)` for F6, F7, and Shift+F7. With GNOME's Alt+F2 Run dialog over Minecraft, `GetFocus` reported `modal=true`, F6 emitted no preset signal, and `ResizeFocused 700 500` returned `no focused Minecraft window`. The user closed the dialog with Escape. No keyd remaps or sensitivity changes were active during these checks.

The first observe-only focus-service integration found malformed FIFO bytes from an asynchronous GJS write. A standalone native GJS test then delivered 20/20 exact JSON heartbeats with `GLib.Bytes` and `write_bytes_async`, including after a reader restart. After a GNOME login loaded that fix, the observe-only service received valid heartbeats with no FIFO errors, reported Minecraft focus, cleared it on the Run dialog, restored it after the dialog closed, and cleared it again when the extension was disabled for over two seconds. This confirms focus delivery and watchdog behavior without activating keyd. The service's extension-availability lifecycle fix was also live tested after disabling and re-enabling the extension on 2026-10-04; see the [focus probe record](keyd-focus-probe.md). These checks do not verify preset resize handling through the service or sensitivity behavior.

`maccel` was not found on PATH during these earlier checks. At initial preparation on 2026-10-22, maccel 0.6.0 was installed and readable while the helper/rule were uninstalled. The later approved attempts, two live defects, and successful corrected-code retests are recorded below.

Earlier on 2026-10-06, an observe-only toolkit service on port 8877 loaded a temporary `normal` preset (`F6`, 800×600) while the production extension was active. Its status reported `extension_connected=true`, no FIFO or bridge errors, `live_activation_enabled=false`, and `maccel` unavailable. The user reported a visible resize, but focus changed before the frame could be sampled. That first observation is superseded by the measured F6 run below. Live eye-zoom sensitivity was pending because `maccel` was absent at that time.

### 2026-10-06 measured observe-only F6 resize

A fresh `XDG_DATA_HOME` isolated the observe-only toolkit service on port 8877. The temporary profile had `normal` on F6 at 800×600 with `eye_zoom=false`. Status showed `live_activation_enabled=false`, `extension_connected=true`, empty service/FIFO errors, and `/etc/keyd` remained empty. With `MCSR_Ranked` focused, GNOME `GetFocus` first reported 854×517 and `modal=false`. Pressing F6 produced `PresetActivated ('normal', false)`. The frame was sampled at 854×517 and then at 800×600 within about 0.55 seconds of the signal; multiple later samples remained 800×600 while the window class still identified Minecraft and `modal=false`. The user also confirmed the visible resize.

The service reported `active_preset=normal` during the initial 800×600 samples. Later `active_preset` became `null` while the frame remained 800×600 and there were no reported errors; the cause was not determined. This passes the measured focused F6 resize case, with that status discrepancy recorded. The service was stopped and `ConfigurePresets '[]'` returned `()`. It did not edit Minecraft controls. After Minecraft closed, Backward remained `key.keyboard.s`; the only change from the pre-resize options snapshot was Minecraft's saved `soundCategory_music:1.0` → `0.0`, which was preserved.

The automated extension harness exercises configured shortcut callbacks, delivery of preset IDs, shifted eye zoom, modal refusal, delayed observed dimensions, wrong focus, invalid sizes, pending focus loss, reconfiguration, and missing/regular FIFO refusal. These checks use controlled GNOME boundary doubles; they do not establish desktop acceptance.

## Prepare at a convenient later login

From the project root, build a bundle and install it for the current user:

```bash
mkdir -p /tmp/mcsr-toolkit-extension
glib-compile-schemas --strict --dry-run extension/schemas
gnome-extensions pack --force --out-dir=/tmp/mcsr-toolkit-extension extension
gnome-extensions install --force /tmp/mcsr-toolkit-extension/mcsr-toolkit@local.shell-extension.zip
```

At a convenient time, log out and back into GNOME Wayland. Then enable the extension and confirm its state:

```bash
gnome-extensions enable mcsr-toolkit@local
gnome-extensions info mcsr-toolkit@local
```

The focus service owns `$XDG_RUNTIME_DIR/mcsr-toolkit-focus.fifo` creation. If the service is absent, the extension retries without creating a regular file. Extension disable/re-enable status lifecycle was live tested on 2026-10-04, as recorded in the [focus probe](keyd-focus-probe.md). The normal F6 path passed the earlier measured check above; the later 2026-10-22 sensitivity and recovery results are recorded below. Do not start keyd activation just to test resizing.

## Check observed resize and focus refusal

Run the following from a terminal, then focus Minecraft during the three-second delay:

```bash
sleep 3
gdbus call --session --dest org.gnome.Shell --object-path /org/mcsr/Toolkit \
  --method org.mcsr.Toolkit.GetFocus
gdbus call --session --dest org.gnome.Shell --object-path /org/mcsr/Toolkit \
  --method org.mcsr.Toolkit.ResizeFocused 800 600
```

Record the actual reply and Minecraft frame size. Expected resize reply: `(uint32 800, uint32 600)`. The extension waits up to one second for the observed frame size; an inexact result is returned as actual dimensions and rejected by the Python preset controller. A requested resize alone is not success.

Repeat the resize call while the terminal remains focused. Expected: a D-Bus error containing `no focused Minecraft window`, with neither terminal nor Minecraft resized. Repeat with no focused game, a Shell modal open, and dimensions `99 600` or `16385 600`: no accepted resize. Focus must remain on the original Minecraft window until the result arrives.

## Register and observe shortcuts

Configure two presets without changing keyd:

```bash
PYTHONPATH=src .venv/bin/python - <<'PY'
from mcsr_toolkit.models import Profile, ResizePreset
from mcsr_toolkit.resize import GnomeWindowBus
profile = Profile(presets=[
    ResizePreset(id="normal", width=800, height=600, shortcut="f6"),
    ResizePreset(id="eye", width=400, height=300, shortcut="f7", eye_zoom=True),
])
GnomeWindowBus().configure_presets(profile)
PY
```

For manual observation only, monitor the extension bus in another terminal:

```bash
gdbus monitor --session --dest org.gnome.Shell --object-path /org/mcsr/Toolkit
```

Focus Minecraft and press F6, F7, then Shift+F7. Expected `PresetActivated` payloads: `('normal', false)`, `('eye', false)`, `('eye', true)`. Repeat in the terminal and with a Shell modal: no activation. Conflicts with GNOME or other extensions must be resolved before accepting delivery. Shortcuts report preset IDs to the service; without its subscriber they do not apply a size. Production code must subscribe to typed D-Bus signals rather than parse this monitor output.

## Maccel helper setup and recovery

**Current deployment and acceptance, 2026-10-22:** corrected helper source/installed SHA256 `4b75d58cfa14a3de4037a95e65b2b55f0a97c204b0c10b1b06883efa57192680`, root:root `0755`, and the exact two-command sudoers rule are installed after separate approvals. Supervised resizing, eye/shifted sensitivity, baseline restoration, blur/modal/disconnect, SIGTERM after its fix, and crash/startup recovery passed. Final raw/CLI baseline is `4294967296` / `1.000000000`; root baseline record absent, services/sampler stopped, shortcuts cleared, `/etc/keyd` empty, and real options hash unchanged. See [the dated acceptance record](acceptance-wayland.md#2026-10-22-corrected-helper-acceptance-and-shutdown-repair) for sample times and limits.

The first helper (`78f74b7c6c621be292c65312a9a084d1743a203abb148087023f2310c7837077`) wrote LF into a driver that appends another LF on read, so eye readback/restore failed. Direct approved recovery restored numeric baseline while retaining its root record. The reviewed replacement writes raw digits without LF and accepts the observed legacy double-LF parameter state without relaxing durable records; after separate approval it recovered the record successfully. A later first SIGTERM test exposed cleanup after Uvicorn signal replay; application lifespan shutdown and draining queued jobs before lock release fixed it. The final source suite passed 401 tests and independent review; live SIGTERM and startup recovery were then retested successfully. Do not reinstall the known failing old helper.

### Verify the installed reviewed helper

Future helper installation/replacement requires fresh review and separate security approval of the exact source/hash and commands. The previous replacement proposal is preserved as history in the SDD report; its old-hash precondition intentionally refuses the current installation. To check the current reviewed copy without invoking eye:

```sh
(
set -eu
cd /mnt/data/ubuntu/Documents/mcsr/.worktrees/feat-mcsr-toolkit
maccel_expected=4b75d58cfa14a3de4037a95e65b2b55f0a97c204b0c10b1b06883efa57192680
printf '%s  %s\n' "$maccel_expected" src/mcsr_toolkit/maccel_privileged.py "$maccel_expected" /usr/local/libexec/mcsr-toolkit-maccel | sha256sum -c -
namei -l /usr/local/libexec/mcsr-toolkit-maccel
stat -c '%U:%G %a %n' / /usr /usr/local /usr/local/libexec /usr/local/libexec/mcsr-toolkit-maccel
sudo -n -l -- /usr/local/libexec/mcsr-toolkit-maccel eye
sudo -n -l -- /usr/local/libexec/mcsr-toolkit-maccel restore
)
```

Require a real regular root-owned mode `0755` helper and real root-owned executable directory ancestors with no group/world write bits or symlinks. Authentication prerequisite for root inspection, future approved installation, and removal: use the unlocked GNOME session with its Polkit agent and operator present for each `pkexec` prompt. The exact sudoers grant covers only eye/restore. Denied/cancelled authentication is a failed check. Perform prompts outside the focused Minecraft live-input probe; do not widen sudoers or depend on a sudo timestamp. Approved installation uses absolute source/target paths through `pkexec /usr/bin/install`, as in the reviewed deployment history.

The helper is a standalone `#!/usr/bin/python3 -I` executable with fixed paths and only argument-free `eye` and `restore` subcommands. The approved installation must copy the reviewed [source](../src/mcsr_toolkit/maccel_privileged.py) to `/usr/local/libexec/mcsr-toolkit-maccel` as `root:root` mode `0755`. Verify source and installed SHA256 match the reviewed hash before use. Use `namei -l /usr/local/libexec/mcsr-toolkit-maccel` and `stat` to confirm that the file is regular, every ancestor is a real directory, all are root-owned with no group/world write bits, and no component is a symlink. The current ancestors `/`, `/usr`, `/usr/local`, and `/usr/local/libexec` are `root:root` mode `0755`; recheck at deployment.

For desktop login `undec`, the exact [sudoers template](../packaging/mcsr-toolkit-maccel.sudoers) expands to:

```sudoers
undec ALL=(root) NOPASSWD: /usr/local/libexec/mcsr-toolkit-maccel eye, /usr/local/libexec/mcsr-toolkit-maccel restore
```

Validate a root-owned mode `0440` candidate with `pkexec /usr/sbin/visudo -cf <candidate>` before copying it to `/etc/sudoers.d/mcsr-toolkit-maccel` with the same ownership/mode. Validate the installed file and full policy afterward. No wildcards, additional arguments, or general-purpose commands are permitted. As `undec`, verify listing without executing either operation:

```sh
sudo -n -l -- /usr/local/libexec/mcsr-toolkit-maccel eye
sudo -n -l -- /usr/local/libexec/mcsr-toolkit-maccel restore
```

The adapter additionally executes `sudo -n /usr/local/libexec/mcsr-toolkit-maccel eye` with **empty stdin**, expecting exit `1`, empty stdout, and exactly `maccel helper: missing or malformed generation\n` on stderr. This safe permission probe rejects the missing token before touching runtime records or sysfs. It detects mixed sudo grants that a successful listing alone cannot rule out. Run it only after reviewed deployment; do not supply a generation token during permission checks. It does not establish a successful live eye write.

The supported driver fingerprint is `4AFA0F5B3EBD2F98EE54E56`. The helper records the exact positive signed Q32 raw value before eye writes, syncs the baseline, and checks every write by reading back. A repeated eye call preserves that baseline. `restore` rotates the generation to invalidate queued old eye calls, restores and verifies the baseline, and removes it only after success. Runtime directory `/run/mcsr-toolkit-maccel` must be root-owned mode `0700`; `baseline`, `generation`, and `lock` files must be root-owned mode `0600`. Do not edit or remove recovery evidence to bypass an error. An unsupported fingerprint or unhealthy parameter blocks writes. No guessed fallback multiplier is written.

Do not use `maccel set` for toolkit recovery, add `undec` to the `maccel` group, change the parameter permissions, or alter upstream reset scripts. `/var/opt/maccel/resets` is currently `root:maccel` mode `0775`; upstream scripts can run as root, so granting that group to the service user would cross the intended privilege boundary. The toolkit writes only its private runtime records and the fixed live kernel parameter; it does not change maccel persistence.

Recover with the reviewed corrected helper before considering removal. Do not bypass the root baseline record or use the known failing old helper. Stop every toolkit instance, including any transient acceptance process; only the normal unit stop is shown below. Set `maccel_baseline_raw` and `maccel_baseline_cli` in the current shell from the independently recorded pre-test values, not from a new reading that may still be eye sensitivity. This recovery block runs in a fail-fast subshell and performs no helper or policy removal:

```sh
(
set -eu
: "${maccel_baseline_raw:?Set to the recorded pre-test raw value}"
: "${maccel_baseline_cli:?Set to the recorded pre-test CLI value}"
systemctl --user stop mcsr-toolkit.service
# Also stop any isolated acceptance service before recovery.
sudo -n /usr/local/libexec/mcsr-toolkit-maccel restore
maccel_current_raw=$(cat /sys/module/maccel/parameters/SENS_MULT)
maccel_current_cli=$(maccel get param sens-mult)
printf 'raw=%s\nCLI=%s\n' "$maccel_current_raw" "$maccel_current_cli"
test "$maccel_current_raw" = "$maccel_baseline_raw"
test "$maccel_current_cli" = "$maccel_baseline_cli"
pkexec /usr/bin/test ! -e /run/mcsr-toolkit-maccel/baseline
gdbus call --session --dest org.gnome.Shell --object-path /org/mcsr/Toolkit \
  --method org.mcsr.Toolkit.ConfigurePresets '[]'
)
```

`pkexec /usr/bin/test` inspects the root-only directory through a GNOME authentication prompt; the two-command sudoers rule does not authorize a root `test` command. A cancelled prompt or existing baseline makes the block fail. Both readings must match the pre-test record exactly; the raw integer is the exact kernel baseline. `restore` prints a new generation token, not the restored multiplier. A failed restore, read, comparison, or baseline-absence check stops the block. Leave eye disabled and preserve the helper, permissions, and records for repair. The root baseline is authoritative after a crash; restore is idempotent when it is absent. Reboot clears `/run` and leaves module initialization to upstream settings.

Only after the recovery block succeeds, the independent exact raw/CLI comparison is confirmed, all toolkit instances are stopped, and shortcuts are cleared, use this separate removal block for the dedicated paths installed by the approved setup:

```sh
(
set -eu
: "${maccel_baseline_raw:?Set to the recorded pre-test raw value}"
: "${maccel_baseline_cli:?Set to the recorded pre-test CLI value}"
# Recheck the independently recorded baseline before removing recovery access.
maccel_current_raw=$(cat /sys/module/maccel/parameters/SENS_MULT)
maccel_current_cli=$(maccel get param sens-mult)
test "$maccel_current_raw" = "$maccel_baseline_raw"
test "$maccel_current_cli" = "$maccel_baseline_cli"
pkexec /usr/bin/test ! -e /run/mcsr-toolkit-maccel/baseline
pkexec /usr/bin/rm /etc/sudoers.d/mcsr-toolkit-maccel
pkexec /usr/sbin/visudo -c
# set -e prevents this deletion if policy validation fails.
pkexec /usr/bin/rm /usr/local/libexec/mcsr-toolkit-maccel
)
```

This block repeats the baseline comparisons and stops on any failure. If `visudo -c` fails, the helper remains installed for repair. For a partial installation, inspect the installed paths and review recovery before adapting removal. Keep runtime records; do not discard recovery evidence. Do not remove keyd's separate helper/rule or change `/etc/keyd`.

## Verify sensitivity with the user service connected

**Acceptance recorded:** the 2026-10-22 corrected-code run passed the matrix below after the newline and shutdown fixes; the earlier failed attempts remain in the dated record. Use this procedure for future reruns. Record fresh raw/CLI baseline before each run; the accepted baseline was `4294967296` / `1.000000000`, fingerprint `4AFA0F5B3EBD2F98EE54E56`. Check reset-script ownership, login groups, empty `/etc/keyd`, service inactivity, and cleared shortcuts. Do not type in authentication fields during this probe.

Use a fresh isolated `XDG_DATA_HOME` and one observe-only service, with the F6/F7 profile above. Stop the normal unit first and do not use `--enable-live-keyd` or Apply. Startup restore must succeed before eye is available. Capture a background sampler of the raw parameter, CLI value, focus/modal state, and actual frame so Minecraft stays focused during sensitivity measurement; switching to the terminal would trigger restore and hide the eye value. Read-only `maccel_status()` reports CLI availability, while service sensitivity status also reflects the privileged helper and recovery state.

| Action | Expected observed frame | Expected sensitivity |
| --- | --- | --- |
| F6 from baseline | 800×600 | exact baseline |
| Shift+F7 from baseline | 400×300 | unchanged baseline |
| F7 | 400×300 | about `0.02083` (Q32 raw `89464169`) |
| Shift+F7 during eye | 400×300 | unchanged eye value |
| F6 after eye | 800×600 | exact baseline raw and CLI values |
| Focus another app during a fresh eye cycle | game frame unchanged | exact baseline restored |
| Open Shell modal during a fresh eye cycle | game frame unchanged | exact baseline restored |
| Disable extension during a fresh eye cycle | game frame unchanged | exact baseline restored after detected disconnect |
| Clean service shutdown during a fresh eye cycle | game frame unchanged | exact baseline restored |
| Kill service during a fresh eye cycle, then restart | game frame unchanged | startup recovery restores exact baseline before new eye actions |

Keep the observer running through each restore. For the crash case, retain the root baseline and confirm recovery on the next isolated service startup. A failed/inexact normal resize must still restore active eye sensitivity. A failed helper operation clears the active-eye claim, blocks later eye writes, and surfaces recovery pending. Use the reviewed manual `restore` command if needed; retain failure evidence and do not report completion until an independent read matches the exact baseline.

Record session/Shell version, accelerators, actual frame replies, refusal results, raw/CLI baseline and samples, each restore cause, shortcut conflicts, and cleanup. End with the original raw/CLI values confirmed, root baseline absent, GNOME shortcuts cleared, all toolkit test services stopped, and `/etc/keyd` unchanged. The measured normal F6 800×600 case passed on 2026-10-06; preserve that evidence separately. These cases passed on 2026-10-22 as recorded separately; new source/driver changes require appropriate review and fresh observations. Automated tests and the safe permission probe do not establish desktop acceptance.

GNOME API references: [GJS D-Bus exports and method invocations](https://gjs.guide/guides/gio/dbus.html), [Mutter Window API](https://gnome.pages.gitlab.gnome.org/mutter/meta/class.Window.html).
