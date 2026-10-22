# MCSR Toolkit Design

## Purpose and scope

Build a local web interface that helps an Ubuntu GNOME Wayland user set up MCSRLauncher, Ninjabrain Bot, and Minecraft speedrunning key bindings. The user already has `keyd` installed. The toolkit downloads the official JARs, configures keyd for focused Minecraft windows, and keeps controls in every MCSRLauncher-managed instance consistent with the emitted keys.

The first release supports one desktop user on one device. It does not install keyd, modify instances belonging to other launchers, or manage Ninjabrain Bot's own shortcut preferences. Ninjabrain actions use shortcuts that the user has already configured in the bot.

**Sensitivity update (2026-10-06):** The [maccel sensitivity recovery addendum](2026-10-06-maccel-sensitivity-recovery-design.md) supersedes the direct unprivileged `maccel set` path below. The installed CLI's persistence scripts require a separate narrow privilege boundary for safe live eye zoom.

## Platform and components

- A Python service binds only to `127.0.0.1` and serves the web UI and local API. It runs without root privileges, stores the toolkit profile under the user's application data directory, discovers MCSRLauncher instances, downloads JARs, and prepares file changes.
- An existing root-owned helper installs the toolkit-owned keyd configuration and provides a fixed `clear` command. `clear` restarts `keyd.service`, verifies its health within a bounded time, and stops it if restart or verification fails. An exact, user-specific passwordless sudoers rule authorizes only `/usr/local/libexec/mcsr-toolkit-keyd clear`; installation and restore still require authenticated elevation. The web service never runs as root. If another keyd configuration already claims the keyboard, the first release reports the conflict and leaves it untouched.
- The keyd configuration defines an inactive chat layer. A user service with `keyd` group access applies game bindings dynamically only while Minecraft has effective input focus. It uses the helper's `clear` command to clear chat state on suspension and after configuration changes. The toolkit leaves `~/.config/keyd/app.conf` untouched.
- A toolkit-provided GNOME Shell extension identifies the focused Minecraft window, reports Shell modal dialogs as a non-game input context, and resizes Minecraft to configured width × height presets. The extension and focus service use the same Minecraft window match rule.
- Optional `maccel` integration changes sensitivity for the designated eye zoom preset. The reference values are `0.02083` for eye zoom and `1` for normal sensitivity. A shifted eye zoom shortcut changes the size without changing sensitivity, as in the linked guide. The toolkit detects whether `maccel` is installed and usable before enabling this feature.

The focus-aware keyd path is an early compatibility milestone. The initial application mapper probe exposed two failures: a GNOME authentication modal did not cause a focus-window event, and `keyd bind reset` did not clear a toggled chat layer. A later keyd 2.5 probe exited with SIGSEGV after `keyd reload`, so neither routine cleanup nor install or restore uses reload. The revised service must demonstrate focus entry, focus exit, Shell modal entry and exit, chat entry and exit, and chat state clearing on blur before the UI depends on it. No production remaps activate until that proof passes.

## User workflow

1. The setup page reports the status of keyd, the toolkit focus service, the GNOME extension, MCSRLauncher discovery, and optional `maccel`.
2. The downloads page fetches MCSRLauncher from the official MCSR Ranked download source and Ninjabrain Bot from the official `Ninjabrain1/Ninjabrain-Bot` GitHub releases. It shows the resolved version and saved file location. Downloads use HTTPS, a temporary file, an expected JAR asset, and atomic replacement so a failed transfer does not masquerade as a completed download.
3. The configurator lets the user select physical keys for Minecraft controls, assign physical keys to emitted letters for left-hand crafting searches, map keys to existing Ninjabrain Bot shortcuts, choose the chat-entry and exit keys, and define window size presets and their shortcuts. Search snippets such as `KA` are typing goals shown in the UI; they are not text-expansion macros.
4. Before applying, the UI shows physical key → emitted key → Minecraft action for every affected control. For example, if physical `S` emits `K` and `S` is Backward, the resulting Minecraft binding is `K`.
5. Applying validates the profile, backs up affected files, writes the dedicated keyd configuration, updates `options.txt` in every discovered MCSRLauncher-managed instance, and activates the focus-aware mappings. Each result is shown per instance. If an instance is running or malformed, activation waits until all managed instances can be updated safely. A later run discovers and updates newly created instances. An optional launcher-data path override supports nonstandard MCSRLauncher locations while still updating every instance under that launcher root.

## Mapping rules

The normal game bindings activate only while a Minecraft window has effective input focus and no Shell modal is open. On blur, modal entry, or loss of the focus heartbeat, the service persistently suspends activation, issues `keyd bind reset` first, then invokes the helper's `clear` command to restart keyd and clear toggled layers. It serializes suspension and cleanup with activation, apply, and restore. Cleanup failures remain visible and are retried; the UI never claims mappings are safe or inactive if stopping keyd fails. Before any subsequent activation, the service rechecks effective focus and modal state. Modal signals arrive asynchronously, so a brief interval between modal appearance and suspension remains possible. The keyd mapping is resolved before Minecraft sees an input event. The configurator therefore computes each configured Minecraft control's final emitted key and writes that key to the corresponding `options.txt` entry in every managed instance.

The UI rejects ambiguous control assignments when two actions would rely on the same emitted key. It also flags reserved chat keys and incompatible shortcut combinations. It preserves unrelated `options.txt` lines, including settings from Minecraft or mods. Every Minecraft control affected by a physical-key remap is updated, including controls the user left at their original binding; unaffected controls remain unchanged.

`T` and `/` open chat or command entry and switch to a chat layer. This layer maps each customized typing key to itself. `Enter` sends and exits the layer; `Esc` cancels and exits. All four keys are configurable. The entry and exit keys emit their normal Minecraft events as they change layers. On loss of effective input focus, including a GNOME authentication modal, the suspension sequence clears chat state before activation can resume.

## Resize and sensitivity

The user defines named width × height presets and assigns a shortcut to each. The GNOME extension acts on the focused Minecraft window only, reports whether the requested size was applied, and does not resize another app. A designated eye zoom preset can set `maccel`'s sensitivity multiplier to `0.02083`; leaving that preset restores normal sensitivity, with `1` as the reference normal value. The shifted eye zoom shortcut applies the size while leaving sensitivity unchanged. A failed resize or `maccel` command is reported and must not leave the toolkit claiming the preset is active.

## Applying and recovery

The toolkit checks prerequisites and validates its restricted keyd fragment before replacing an active configuration. Ubuntu's installed keyd 2.5.0 build has no `check` command, so candidate syntax is constrained by validation and tested against the live daemon before release. It refuses a conflicting or unrecognized keyd configuration and leaves `app.conf` untouched. It makes a backup before each `options.txt` edit and writes through a temporary file with atomic replacement. If an instance is running, the entire profile activation is deferred until it closes so Minecraft cannot overwrite the edit or receive remapped keys with stale controls. Install, apply, and restore serialize with suspension and use the helper's bounded restart and health check after keyd configuration changes. If any step fails, the UI reports the affected component and leaves recoverable backups; an explicit restore action returns toolkit-managed files to their previous state. Activation remains suspended until cleanup succeeds and focus is rechecked.

The local API accepts state-changing requests only from the local UI session and validates inputs before any file or command operation. It accepts a launcher-data path override only when it resolves to an MCSRLauncher root with the expected instance structure; it never accepts arbitrary shell commands or target file paths from the browser. The root-owned, self-contained keyd helper changes only the toolkit keyd config and fragment through authenticated elevation; its fixed `clear` command is the sole passwordless keyd action. The sensitivity addendum specifies a separate helper with exact `eye` and `restore` permissions. The unprivileged focus service uses `keyd bind reset` through `keyd` group access before invoking `clear`.

## Verification and acceptance

- Automated tests cover mapping composition, key conflicts, chat transitions, Minecraft control parsing and preservation, discovery of every MCSRLauncher-managed instance, new instance detection, backup and restore, download failures, partial apply recovery, serialized suspension, cleanup failure and retry, failed keyd stop reporting, and focus rechecking before activation.
- An Ubuntu GNOME Wayland integration check repeats chat-only entry and exit cycles, focus entry and blur, modal entry, heartbeat loss, and ordinary typing in other apps. It verifies chat state is cleared and no toolkit remap survives confirmed cleanup; it also probes an empty-config rollback and runs a keyd crash soak longer than 26 seconds after the final restart. The check records any input delivered during the asynchronous modal interval rather than claiming that interval is eliminated.
- Live checks prove that `T` and `/` enter pass-through chat, `Enter` and `Esc` leave it, configured physical movement keys still perform their intended actions after control-file updates, and other applications receive ordinary keys.
- Live checks prove exact resize presets on the focused Minecraft window, eye zoom sensitivity changes when `maccel` is available, the shifted variant leaves sensitivity unchanged, and normal sensitivity is restored when eye zoom ends.
- Download checks confirm both official JAR sources, displayed versions and saved locations, and clean handling of offline or malformed responses.

## References

- [MCSR Ranked downloads](https://wiki.mcsrranked.com/install/download)
- [Ninjabrain Bot releases](https://github.com/Ninjabrain1/Ninjabrain-Bot/releases)
- [keyd application remapping and setup](https://github.com/rvaiya/keyd/blob/master/README.md)
- [keyd layer actions](https://github.com/rvaiya/keyd/blob/master/docs/keyd.scdoc)
- [GNOME Mutter window API](https://gnome.pages.gitlab.gnome.org/mutter/meta/class.Window.html)
- [Resize and DPI guide](https://docs.google.com/document/d/1WQjLTgrZRpAiOAYMzJclruCgdQFgQHHwa4hdDPaE8G4/edit?tab=t.0)
- [maccel project](https://github.com/Gnarus-G/maccel)
