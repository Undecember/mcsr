# Standard Settings integration

## Intent and approved scope

Apply must keep the controls in each instance's Standard Settings config consistent with its toolkit-managed Minecraft controls, so creating a world does not undo the profile. The user approved local modern JSON and legacy text support, preservation of unrelated settings, backup/Restore coverage, and running-instance deferral. Shared config files must not be modified.

## Approach

Extend the existing Apply transaction to include existing local mod config files. This preserves the current profile and UI and provides the same backup, conflict detection, and recovery behavior as `options.txt`. Disabling the mod globally would discard unrelated reset behavior; modifying shared files could affect other launchers. Neither is appropriate for the approved scope.

Use `config/mcsr/standardsettings.json` and `config/standardoptions.txt`, relative to the discovered game's directory. Synchronize enabled reset entries against the final effective `options.txt` bindings, including crafting remaps, explicit controls, and chat bindings. Preserve per-setting disabled flags, global toggles, unrelated values, and unknown metadata. Missing mod configs remain absent: the mod generates its initial config from the updated game options.

JSON control values may be primitive strings or `{ "enabled": ..., "value": ... }` objects. Legacy absent or blank settings are disabled and stay that way. Reapplying an unchanged profile must still repair differing enabled mod entries and defer if the instance is running.

The mod's shared redirect takes precedence over its local config. Detect modern `.global` references and legacy text path redirects, and report an actionable instance validation failure rather than edit a shared target or claim ineffective local setup succeeded. Redirect localization and shared-file writes are outside this change. This handling is an implementation proposal following the user's local-only preference.

## Recovery and compatibility

Include every managed existing local mod config in the same backup and mutation lifecycle as game options. Restore must validate all targets and their hashes before changing anything, restore exact original bytes, and reject unsafe paths or intervening edits. Preserve restoration of old backups that contain only game options. Preserve LIFO lineage and reconciliation of newly discovered instances.

Reject symlinks and nonregular config files, including unsafe config-directory ancestors. Keep edits bounded to the recognized per-instance config paths. Preserve unrelated working-tree changes and do not commit, merge, or push.

## Verification

Use parser and coordinator tests with upstream-shaped JSON/text fixtures. Cover effective values, disabled settings, unrelated content, absent configs, stale mod-only settings, running instances, redirects, malformed configs, symlinks, rollback, exact-byte Restore, changed-file rejection, old manifests, and multi-profile LIFO restoration. Run the full Rust suite, formatting and diff checks, and update Graphify after implementation.

## Upstream evidence

- [Modern paths and shared-file precedence](https://github.com/contariaa/StandardSettings/blob/1.16-1.16.1/README.md)
- [Primitive and enabled/value JSON encodings](https://github.com/contariaa/StandardSettings/blob/1.16-1.16.1/src/main/java/me/contaria/standardsettings/options/StandardSetting.java)
- [Legacy blank settings and path redirects](https://github.com/contariaa/StandardSettings/blob/8ac15b0/README.md)
