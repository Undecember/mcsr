# MCSR Toolkit Design

## Purpose and scope

Build a local web interface that helps an Ubuntu GNOME Wayland user set up MCSRLauncher, Ninjabrain Bot, and Minecraft speedrunning key bindings. The user already has `keyd` installed. The toolkit downloads the official JARs, configures keyd for focused Minecraft windows, and keeps controls in every MCSRLauncher-managed instance consistent with the emitted keys.

The first release supports one desktop user on one device. It does not install keyd, modify instances belonging to other launchers, or manage Ninjabrain Bot's own shortcut preferences. Ninjabrain actions use shortcuts that the user has already configured in the bot.

## Platform and components

- A Python service binds only to `127.0.0.1` and serves the web UI and local API. It runs without root privileges, stores the toolkit profile under the user's application data directory, discovers MCSRLauncher instances, downloads JARs, and prepares file changes.
- A narrowly scoped privileged operation installs a toolkit-owned keyd include fragment, adds its include directive to the user's selected active keyd device configuration, and reloads keyd. The web service itself never runs as root. It does not create a competing top-level device configuration.
- The dedicated keyd fragment defines game and chat layers. A toolkit-owned entry in the user's `~/.config/keyd/app.conf` activates the game mappings only while any Minecraft window is focused. Existing, unrelated keyd bindings and application mappings are preserved.
- A toolkit-provided GNOME Shell extension identifies the focused Minecraft window and resizes it to configured width × height presets. The extension and keyd integration use the same Minecraft window match rule.
- Optional `maccel` integration changes sensitivity for the designated eye zoom preset. The reference values are `0.02083` for eye zoom and `1` for normal sensitivity. A shifted eye zoom shortcut changes the size without changing sensitivity, as in the linked guide. The toolkit detects whether `maccel` is installed and usable before enabling this feature.

The focus-aware keyd path is an early compatibility milestone because keyd describes application remapping as experimental. The implementation must demonstrate focus entry, focus exit, chat entry, and chat exit on Ubuntu GNOME Wayland before the remaining UI work depends on it. If the installed keyd version cannot meet these transitions, implementation stops for a design revision rather than silently applying global remaps.

## User workflow

1. The setup page reports the status of keyd, its application mapper, the GNOME extension, MCSRLauncher discovery, and optional `maccel`.
2. The downloads page fetches MCSRLauncher from the official MCSR Ranked download source and Ninjabrain Bot from the official `Ninjabrain1/Ninjabrain-Bot` GitHub releases. It shows the resolved version and saved file location. Downloads use HTTPS, a temporary file, an expected JAR asset, and atomic replacement so a failed transfer does not masquerade as a completed download.
3. The configurator lets the user select physical keys for Minecraft controls, assign physical keys to emitted letters for left-hand crafting searches, map keys to existing Ninjabrain Bot shortcuts, choose the chat-entry and exit keys, and define window size presets and their shortcuts. Search snippets such as `KA` are typing goals shown in the UI; they are not text-expansion macros.
4. Before applying, the UI shows physical key → emitted key → Minecraft action for every affected control. For example, if physical `S` emits `K` and `S` is Backward, the resulting Minecraft binding is `K`.
5. Applying validates the profile, backs up affected files, writes the dedicated keyd configuration, updates `options.txt` in every discovered MCSRLauncher-managed instance, and activates the focus-aware mappings. Each result is shown per instance. A later run discovers and updates newly created instances. An optional launcher-data path override supports nonstandard MCSRLauncher locations while still updating every instance under that launcher root.

## Mapping rules

The normal game layer is active only while a Minecraft window has focus. Outside Minecraft, all toolkit mappings are inactive. The keyd mapping is resolved before Minecraft sees an input event. The configurator therefore computes each configured Minecraft control's final emitted key and writes that key to the corresponding `options.txt` entry in every managed instance.

The UI rejects ambiguous control assignments when two actions would rely on the same emitted key. It also flags reserved chat keys and incompatible shortcut combinations. It preserves unrelated `options.txt` lines, including settings from Minecraft or mods. Every Minecraft control affected by a physical-key remap is updated, including controls the user left at their original binding; unaffected controls remain unchanged.

`T` and `/` open chat or command entry and switch to a chat layer. This layer maps each customized typing key to itself. `Enter` sends and exits the layer; `Esc` cancels and exits. All four keys are configurable. The entry and exit keys emit their normal Minecraft events as they change layers. The chat state resets when Minecraft loses focus or closes, so it cannot remain latched for the next focused window.

## Resize and sensitivity

The user defines named width × height presets and assigns a shortcut to each. The GNOME extension acts on the focused Minecraft window only, reports whether the requested size was applied, and does not resize another app. A designated eye zoom preset can set `maccel`'s sensitivity multiplier to `0.02083`; leaving that preset restores normal sensitivity, with `1` as the reference normal value. The shifted eye zoom shortcut applies the size while leaving sensitivity unchanged. A failed resize or `maccel` command is reported and must not leave the toolkit claiming the preset is active.

## Applying and recovery

The toolkit checks prerequisites and validates the generated keyd file before replacing an active configuration. It never overwrites an unrecognized user file or removes unrelated sections from `app.conf`. It makes a backup before each `options.txt` edit and writes through a temporary file with atomic replacement. If an instance is running, its control edit is deferred until it closes so Minecraft cannot overwrite it on exit. If any step fails, the UI reports the affected component and leaves recoverable backups; an explicit restore action returns toolkit-managed files to their previous state.

The local API accepts state-changing requests only from the local UI session and validates inputs before any file or command operation. It accepts a launcher-data path override only when it resolves to an MCSRLauncher root with the expected instance structure; it never accepts arbitrary shell commands or target file paths from the browser. Privileged changes are restricted to the toolkit-owned keyd include fragment, its managed include directive, and keyd reload.

## Verification and acceptance

- Automated tests cover mapping composition, key conflicts, chat transitions, Minecraft control parsing and preservation, discovery of every MCSRLauncher-managed instance, new instance detection, backup and restore, download failures, and partial apply recovery.
- An Ubuntu GNOME Wayland integration check proves that mappings start only when a Minecraft window gains focus and stop on focus loss, including while chat is open.
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
