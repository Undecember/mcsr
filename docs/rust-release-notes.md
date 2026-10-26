# Rust single-binary release notes (draft)

Status: **unreleased; Rust source cutover complete; supervised device acceptance in progress**. The planned first tag is `v0.1.0`; no tag or public artifact has been created or approved. Live logout/login, guided uninstall with rollback and prompt-count checks, and exact final published-artifact verification remain pending.

## Candidate changes

- One Linux x86_64 Rust executable contains the local service, guided `doctor`/`install`/`repair`/`uninstall` commands, browser assets, GNOME Shell 50 extension archive, and narrowly scoped root helper modes.
- The installed service starts in observe-only mode. Live keyboard remapping remains an explicit local choice; keyd and optional maccel are external components.
- The Rust profile and recovery state is separate from Python state. The installer refuses unresolved Python recovery and preserves user profiles and backups on uninstall.
- Python product source, tests, package metadata, and obsolete helper templates were removed before the live acceptance gates, at the user's direction. Python-era user data and recovery evidence remain preserved for migration and recovery.
- The extension identity is `mcsr-toolkit@undecember.github.io`; the project URL is `https://github.com/Undecember/mcsr`. The source license is MIT.

## Distribution details to fill after acceptance

| Item | Value |
| --- | --- |
| Target | Ubuntu GNOME Shell 50, Linux x86_64 Wayland |
| Release tag | `v0.1.0` (planned; not created) |
| Source commit | Pending reviewed release commit |
| Executable SHA256 | Pending checksum of exact published executable |
| Binary and checksum | `mcsr-toolkit` and `mcsr-toolkit.sha256` |
| Verified live install and repair | In progress; pending logout/login gate in [supervised checklist](rust-release-checklist.md) |
| Verified live uninstall, rollback, and prompt counts | Pending [supervised checklist](rust-release-checklist.md) |

Build from a clean checkout and verify as described in the [README](../README.md). Before installation, put the downloaded binary beside its published `mcsr-toolkit.sha256` and run `sha256sum --check mcsr-toolkit.sha256`. Follow the guided commands only after supervised Rust acceptance has passed. Earlier install and repair exercises do not close the pending logout/login, uninstall, or final artifact gates. Focus and sensitivity events are asynchronous; the [Wayland record](../integration/acceptance-wayland.md) reports observed timing and limits. Historical Python-era measurements in that record do not validate the Rust candidate.
