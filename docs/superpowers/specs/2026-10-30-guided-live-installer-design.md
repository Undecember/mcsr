# Guided installation with live keyd by default

The user approved guided prerequisite repair and default live activation after
readiness checks. Fresh Ubuntu installations must not require deciphering a
closed helper pipe, a stale group list, or an F24 routing timeout.

## Approaches

Diagnostics alone would leave manual package, permission, and layout work.
Guided repair is the selected approach: offer the changes that are needed,
explain their effects, and keep an installation resumable when a new login is
needed. Silently changing every host setting would obscure persistent changes.

## Behavior

- `install` and `repair` target live keyd by default. `--observe-only` is an
  explicit alternative; `--yes` accepts prerequisite offers for automation.
- Missing distribution keyd is offered through authenticated apt installation.
  A masked or broken service is reported for review, never silently unmasked.
- Offer membership in the keyd socket group. Distinguish persisted membership
  from effective groups of both the installer and user service manager. A stale
  manager requires reboot and `repair`; do not restart the desktop manager.
- Offer `fkeys:basic_13-24`, preserve other XKB options and input sources, and
  save original settings durably before changing them. Include a restore command.
  Verify actual routing through live startup; a settings value is not proof.
- `doctor` stays read-only and reports live readiness separately from ordinary
  installation preflight. Never call an incomplete check live-ready.
- Start with the requested mode. Verify the resolved command and API health,
  including extension connectivity, keyd availability, and recovery flags for
  live mode. Check stability, not one successful HTTP response.
- On a live-only startup failure, stop the restart loop and try a verified
  observe-only service. Keep a repair marker and report setup pending, including
  the live failure. If fallback or input recovery fails, use existing rollback;
  do not claim success or erase recovery evidence.
- Retain bounded helper stderr and exit diagnostics on protocol failures.

## Boundaries

Do not apply a profile during installation, activate bindings without focus,
change package enablement on uninstall, weaken helper validation, or overwrite
foreign recovery records. Package installation and account membership are host
prerequisites and persist across toolkit rollback. Keyboard changes are opt-in
and reversible. Current installed binaries are not replaced by this source task.
