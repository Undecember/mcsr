# MCSR Toolkit Rust single-binary design

## Goal and release boundary

Replace the Python service and both Python privileged helpers with Rust so an Ubuntu user can download one executable, run a guided installer, and use the toolkit without installing Python or Rust. Keep the browser UI and GNOME Shell extension in JavaScript, embedded in the executable for distribution. GNOME must still receive extracted extension files at installation time. The first binary release targets Ubuntu GNOME Shell 50 on x86_64; other GNOME versions and CPU architectures require separate validation.

Preserve the existing toolkit behavior: the profile editor and preview, MCSRLauncher instance discovery and reconciliation, Minecraft control updates and restore, keyd focus and chat behavior, resize presets, optional maccel eye sensitivity, and official JAR downloads. The Rust service remains a local, unprivileged user service. keyd and maccel remain external dependencies for their respective features. A single downloadable executable is an installation artifact, not a promise that the application runs without GNOME, keyd, or other system facilities.

## Executable and component boundaries

One Rust executable exposes separate command modes for the local service, guided install and removal, read-only diagnostics, and narrowly scoped privileged operations. The same compiled bytes may be installed at a user-accessible service path and at a root-owned helper path. Dispatch rejects unknown commands and arguments. Privileged commands require the expected root-owned installed path, root ownership and nonwritable ancestors, and the appropriate effective identity. The root-owned mode never starts the HTTP server or interprets arbitrary browser-selected commands, paths, or values.

The service uses modules with these responsibilities:

- **Profile and controls:** validate the current profile rules and compute physical key → emitted key → Minecraft action mappings.
- **Instance store and transactions:** find all valid MCSRLauncher-managed instances, preserve unrelated `options.txt` content, and coordinate preview, Apply, backups, lineage, deferred instances, reconciliation, and Restore.
- **Focus and input:** consume the existing GNOME focus FIFO and session D-Bus contract, enforce heartbeat and modal gating, and activate or clear keyd bindings.
- **Window and sensitivity:** request exact focused-window resizing through the extension and manage optional maccel eye/restore sequencing.
- **HTTP API and jobs:** serve the existing route paths and JSON shapes on loopback, enforce host/origin/session checks, and serialize mutations and downloads.
- **Installation and assets:** serve embedded HTML/CSS/JavaScript, extract the embedded GNOME extension, provision the user unit and the reviewed helper policy, and report prerequisite failures.

Use Rust's compile-time asset embedding for the browser files and a built extension archive. The service serves web assets directly from the executable; the installer extracts the extension archive to GNOME's user extension location with its fixed UUID and schema. The build verifies that the embedded archive contains the expected `extension.js`, `metadata.json`, and compiled schema. The GNOME extension remains GJS code because GNOME loads extensions from JavaScript files. The browser UI remains JavaScript because it runs in the browser.

The HTTP API, profile JSON shape, keyd fragment grammar, focus FIFO messages, and GNOME D-Bus interface are compatibility contracts during the rewrite. The UI and extension should not need simultaneous rewrites. The new Rust recovery records may use a new format because migration of Python-created profile and backup state is outside this release.

## Privilege boundary and input safety

Keep the service under the desktop user. A root-owned copy of the executable implements the current fixed keyd install/restore/clear operations and fixed maccel eye/restore operations. The passwordless sudoers rules authorize only the exact keyd `clear` and maccel `eye`/`restore` argument forms for the selected desktop login. Installing or restoring keyd configuration still requires interactive authorization. The browser can request only the same predefined workflow operations as today; it cannot select root command lines or filesystem targets.

Port the existing keyd restrictions, root-owned backup snapshots, restart-and-health check, and stop-on-failure behavior. Port maccel's fixed sysfs target, driver fingerprint check, exact raw baseline, root-owned runtime record, generation fencing, and readback before declaring restoration complete. The helper does not grant the service user membership in the `maccel` group. Symlink, ownership, mode, unexpected file type, malformed data, and path substitution checks must fail closed. A privileged operation must never load code or configuration from a user-writable project checkout.

The guided installer may invoke an authenticated system operation to install and verify the root-owned executable and exact policy files. Its privileged setup command is unavailable through passwordless sudo. Before executing the installed helper, the installer compares its hash with the downloaded executable and checks ownership and mode. It never broadens an existing sudoers rule silently. Changes to installed helpers, sudoers, keyd configuration, or maccel state require separate review and explicit authorization before live execution on a device.

## Cutover, state, and failure handling

The installer first performs a read-only preflight for the supported desktop and architecture, required commands and permissions, running toolkit copies, prior user unit and extension, Python recovery files, keyd state, and maccel baseline record. It refuses cutover while Python-managed remaps, an unfinished mutation, or sensitivity recovery are unresolved. The user must complete Python Restore and verify ordinary input and normal sensitivity before that setup is retired. The installer does not delete or reinterpret Python backups. Rust state is stored separately so old records remain inspectable without being mistaken for new recovery records.

**2026-10-24 cutover addendum:** Before authenticated root setup, prepare the toolkit data parent and `rust-v1` state directory as real, invoking-user-owned `0700` directories using no-follow, descriptor-relative operations. Tighten existing owner-owned directories without deleting records; leave `XDG_DATA_HOME` and Python `state/` permissions intact. During GNOME handoff, disable the fixed legacy `mcsr-toolkit@local` UUID and wait boundedly for INACTIVE and removal of the `org.mcsr.Toolkit` interface at `/org/mcsr/Toolkit` on `org.gnome.Shell` before enabling the new UUID. Require new State ACTIVE before root setup. A pre-existing sticky ERROR stages one UUID for a further logout/login and records the old enabled state for rollback. Failed handoff or service startup restores the prior extension configuration where verifiable and retains the marker and recovery tools. Keep input activation disabled until its separate supervised gate.

After preflight, installation stages the user executable, root helper copy, policy, extension, and service unit. It verifies installed files and only then switches the user service. The new service starts in observe-only mode; live keyd activation remains an explicit local process setting with its safety preflight. A failed install before activation rolls back only the staged Rust changes and restores the prior user service configuration. An interrupted install leaves a durable install marker and a diagnostic repair path rather than claiming success. Installation is idempotent when the installed artifact and policy match.

Apply and Restore retain their existing all-instance safety rules. Minecraft instances must be closed before options edits. A mutation marker blocks further Apply, Restore, reconciliation, and live activation after uncertain failure. Recovery requires inspection of the recorded files and helper snapshots; the service does not erase evidence to resume automatically. Focus and modal changes still arrive asynchronously, so the short interval before cleanup is observed remains a documented limitation.

Removal first stops the Rust service, clears dynamic keyd state, verifies keyd health or a safe stopped state, restores maccel's exact baseline, and disables the extension. It removes installed executable copies, user unit, and exact toolkit policy only after these checks succeed. On cleanup failure it stops, keeps recovery tooling and records, and reports the action needed. User profiles, backups, Python-era records, and unresolved root snapshots remain until separately and explicitly removed.

## Migration and verification strategy

The rewrite proceeds in stages while the Python implementation remains the behavioral reference. Build a Rust service shell with embedded UI, API contracts, extension connectivity, and observe-only focus reporting first. Port pure profile, mapping, options, and instance logic next; then transactional Apply/Restore; then keyd and maccel helpers and runtime controllers; then the installer and removal workflow. Each stage uses automated tests, simulated system interfaces, and independent review. Defer all checks against the actual GNOME session, keyd configuration, Minecraft window, physical keyboard, and maccel device to one final supervised acceptance pass after the Rust implementation and installer are complete. At cutover, stop the Python service so both implementations cannot manage the same keyboard or instance files.

Carry representative Python tests into Rust tests and compare golden fixtures for profile validation, options preservation, API JSON and error codes, focus messages, keyd fragments, and backup behavior. Add process and fault-injection tests for interrupted transactions, service shutdown, queued stale helper calls, failed keyd restart, and failed sensitivity restoration. Validate the exact distributed executable from a clean build, including embedded assets, install/repair/uninstall, absence of Python at runtime, and read-only diagnostics. Run formatting, lint, tests, and release-build checks in CI for the supported target.

Before first public release, perform a supervised Ubuntu GNOME 50 x86_64 acceptance pass using the installed binary: first launch, browser UI Apply and Restore, automatic new-instance reconciliation, chat and movement remapping, ordinary typing after blur and Shell modal entry, exact resize, maccel restoration on normal preset/blur/modal/disconnect/shutdown/restart, logout and lock/unlock, and password prompts. Record hardware and package versions, observed timings, and any remaining limits. A public release also needs a license, unique extension UUID and project URL, installation documentation, checksums, release notes, and a release tag under the repository's agreed convention.

## Sources

- [Original toolkit design](2026-10-03-mcsr-toolkit-design.md)
- [maccel recovery design](2026-10-06-maccel-sensitivity-recovery-design.md)
- [Wayland acceptance record](../../../integration/acceptance-wayland.md)
- [GNOME extension anatomy](https://gjs.guide/extensions/overview/anatomy.html)
- [Rust compile-time byte embedding](https://doc.rust-lang.org/stable/core/macro.include_bytes.html)
- [zbus session connection](https://docs.rs/zbus/latest/zbus/connection/struct.Connection.html)
