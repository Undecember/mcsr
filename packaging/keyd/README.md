# Toolkit-managed keyd

`vendor/keyd/` contains pristine source from upstream revision
`f564288ac2b19d2305a5b39023c474805ff8fce5`. `UPSTREAM` records provenance and
`SHA256SUMS` covers its source/test files. The upstream MIT license is retained
at `vendor/keyd/LICENSE`; the packaged executable also embeds it and prints it
with `mcsr-license`.

Cargo copies the native Linux source into `OUT_DIR`, applies `mcsr.patch`, and
compiles both the daemon/client and the event test harness. No downloads occur
during a build. Build prerequisites are a native C compiler (`CC`, default `cc`)
and the `patch` utility, in addition to the toolkit's existing requirements.
Cross-compiling the embedded native dependency is explicitly rejected.

The binary is embedded in the toolkit executable. Installation selects the
toolkit-owned `/usr/local/libexec/mcsr-keyd` through the owned
`/etc/systemd/system/keyd.service.d/90-mcsr-toolkit.conf` override. The system
package executable remains untouched. Configuration and IPC use the existing
`/etc/keyd` and `/var/run/keyd.socket` namespace: two daemons must not own the
same devices. The distribution package must supply an installed, unmasked `keyd.service`. The installer owns service transition and restoration, recording the original active state while leaving service enablement unchanged. Its ownership checks refuse foreign overrides or binaries rather than overwriting them.

## Maintained patch

- `layerk(layer, key)` forwards a plain key's real down/up events while activating
  a momentary layer. Its output reference count supports multiple F3 sources;
  there is no tap timeout or one-shot latch. Modified output keys are rejected.
- `matchmods(keysequence, fallback)` checks the exact active modifier mask at
  press. The chosen descriptor is cached for the matching release, including
  nested fallbacks, so modifier changes cannot release a different output key.
- `bind reset` releases held outputs and clears cached descriptors, active layers,
  queued events, references, and timers before restoring the original config.
- `mcsr-capabilities` reports local support. `mcsr-daemon-capabilities` probes the
  running daemon over IPC. Both return `mcsr-layerk-v1` and `mcsr-matchmods-v1`;
  probing the local binary alone does not establish running-daemon compatibility.

Run `python3 packaging/keyd/test.py` to verify pristine source hashes, apply the
patch in a temporary directory, and execute the upstream/toolkit engine suites.
These tests require no privileges or input devices. Rust asset tests additionally
execute the embedded binary's capability and config-check commands.

For updates, select an explicit upstream revision, replace the pristine vendored
files and checksum manifest, rebase the small patch, then run both engine and
complete toolkit suites. Never regenerate hashes to conceal local source edits.
Real GNOME/Minecraft acceptance remains separate from engine tests.

## Routing transition protocol

Resize relays use F13–F23; F24 is reserved for a transient input fence. After
resetting the producer, the service asks GNOME to remove the old bindings and
prepare the F24 grab. keyd then emits an ordered `do f24` marker. GNOME rejects
commit until that marker has passed through the compositor input queue; only
then may the new consumer be acknowledged and the producer enabled. Failed or
timed-out preparation/commit removes the temporary grab and leaves production
inactive. Each activation has a fresh token, even for an identical profile;
queued old D-Bus signals cannot reuse the new token. The deterministic profile
hash remains separate from that activation token.
