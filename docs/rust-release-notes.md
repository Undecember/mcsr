# Rust single-binary release notes (draft)

Status: **unreleased; supervised Rust device acceptance in progress**. The planned first tag is `v0.1.0`; no tag or public artifact has been created or approved.

## Candidate changes

- One Linux x86_64 Rust executable contains the local service, guided `doctor`/`install`/`repair`/`uninstall` commands, browser assets, GNOME Shell 50 extension archive, and narrowly scoped root helper modes.
- The installed service starts in observe-only mode. Live keyboard remapping remains an explicit local choice; keyd and optional maccel are external components.
- The Rust profile and recovery state is separate from Python state. The installer refuses unresolved Python recovery and preserves user profiles and backups on uninstall.
- The extension identity is `mcsr-toolkit@undecember.github.io`; the project URL is `https://github.com/Undecember/mcsr`. The source license is MIT.

## Distribution details to fill after acceptance

| Item | Value |
| --- | --- |
| Target | Ubuntu GNOME Shell 50, Linux x86_64 Wayland |
| Release tag | `v0.1.0` (planned; not created) |
| Source commit | Pending reviewed release commit |
| Executable SHA256 | Pending checksum of exact published executable |
| Binary and checksum | `mcsr-toolkit` and `mcsr-toolkit.sha256` |
| Verified live install, repair, uninstall | Pending [supervised checklist](rust-release-checklist.md) |

Build from a clean checkout and verify as described in the [README](../README.md). Before installation, put the downloaded binary beside its published `mcsr-toolkit.sha256` and run `sha256sum --check mcsr-toolkit.sha256`. Follow the guided commands only after the supervised Rust acceptance has passed. Installation and repair have been exercised on the target desktop; uninstall and the remaining live cases are pending. Focus and sensitivity events are asynchronous; the [Wayland record](../integration/acceptance-wayland.md) reports observed timing and limits.
