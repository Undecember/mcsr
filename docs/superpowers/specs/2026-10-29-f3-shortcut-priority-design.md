# F3 combinations take priority over resize shortcuts

Status: approved for implementation, including a toolkit-managed patched keyd
build. No installed system changes are part of development verification.

## Intent

Keep B as a resize shortcut, while allowing a Crafting remap of Tab to F3
to make Tab+B trigger Minecraft's F3+B command. The user confirmed that
Tab alone works and B is a resize shortcut.

The user also requires F3+G, F3+F, Shift+F3+F, and Shift+F3. These must work with
physical F3 and with Tab mapped to F3, including when their command keys
are assigned resize shortcuts. Shift must retain its Minecraft meaning,
and repeated F presses while holding F3 must continue to work.

| Required combination | Minecraft Java 1.16 result |
|---|---|
| F3+B | Toggle hitboxes |
| F3+G | Toggle chunk borders |
| F3+F | Increase render distance |
| Shift+F3+F | Decrease render distance |
| Shift+F3 | Show the profiler pie chart |

Other debug combinations are secondary compatibility checks. Preserve ordinary
Tab/F3 behavior, both release orders, multiple combinations during one hold,
and immediate normal resize behavior after releasing F3 or tapping it alone.

## Cause

Resize shortcuts are registered directly with GNOME in
`extension/extension.js`. F3 is an ordinary held key, rather than a desktop
modifier, so GNOME's B grab consumes B even while F3 is down. Skipping the
resize callback cannot return the consumed event to Minecraft.

## Approaches

- Move resize to Alt+B: small, but changes the requested plain-B behavior.
  The user chose to keep B.
- Route resize through keyd and give F3 combinations a separate layer:
  preserves plain-B resize and prevents GNOME from grabbing Minecraft's B.
  This is the selected direction, subject to providing the held-key primitive
  described below. A tap/hold macro approximation is not the design.

## Held-key feasibility evidence and dependency decision

See [the reproducible experiment](../evidence/2026-10-29-f3-keyd/README.md), pinned
to upstream keyd revision `f564288ac2b19d2305a5b39023c474805ff8fce5`.

- Plain `tab = f3` preserves held F3 correctly without conflicting resize mappings.
- `overload` does not forward held F3; Shift interrupting Tab can suppress its tap.
- `layerm` releases its F3 macro before the combination key arrives.
- `oneshotk` passes the required combinations but latches after a standalone
  F3 tap, so the next B press wrongly bypasses resize after F3 is released.
- A throwaway `layerk(layer, key)` engine extension passes all 20 supplied event
  cases and all 94 existing upstream event tests. It emits F3 down/up alongside
  momentary layer activation/deactivation, with no one-shot latch.

This proves a candidate primitive, not a shipped dependency solution. The
prototype was built under `/tmp`; no system daemon was installed, configured,
replaced, or restarted. Its patch and traces are preserved with the experiment.
The results do not prove all stock-keyd alternatives impossible, or establish
GNOME/Minecraft integration.

The user approved maintaining and shipping the patch with this project. Pin and
vendor upstream source, retain its license, build it in CI, and embed the binary
in toolkit releases. Install at `/usr/local/libexec/mcsr-keyd`; use an owned
`keyd.service` drop-in to select it without overwriting the distribution binary.
Keep the existing config/socket namespace so only one daemon owns the devices.
Remove the owned override and reload/restart the original service on uninstall.
Refuse foreign override collisions and preserve existing service enablement.
Do not infer engine support from a version string or weaken held-key requirements.

## Proposed behavior and implementation

- The profile and editor continue to show the user's physical resize shortcuts.
  Compile one shared routing plan from the applied profile, including physical
  shortcuts, F3 sources, internal signals, and eye zoom variants. Both keyd and
  GNOME must consume this exact plan.
- Allocate unused F13–F23 keys as internal resize signals, one per preset.
  GNOME grabs these signals instead of the original shortcuts. Reject profiles
  that cannot allocate enough unused signals, rather than silently colliding
  with other configured mappings. Preserve the Shift variant for eye zoom.
  Allocate deterministically and persist the resulting plan for restart.
  Exclude configured physical/emitted keys, Ninjabrain destinations, shortcut
  terminals, and discovered Minecraft control bindings. Profile scans cannot
  guarantee freedom from external desktop grabs: a GNOME grab failure must
  prevent activation and trigger rollback.
- keyd translates the resize shortcut into its internal signal only while
  the applied profile has Minecraft focus. Existing reset and recovery paths
  remove these translations.
- Preserve F3 combination behavior for physical F3 and keys mapped to F3.
  B/G/F must reach Minecraft while F3 is active, and Shift must distinguish
  F3+F from Shift+F3+F. Resize and eye zoom shortcuts must not consume those
  combinations or Shift+F3. Forward real held-key events rather than emitting
  short F3 combination macros. Preserve existing standalone F3 behavior.
- Native F3 and every source remapped to F3 must activate the same priority
  behavior. The prototype has not yet established simultaneous-source behavior;
  production must retain priority until the last held F3 source is released.
- Continuous held combinations such as the F3+C crash countdown must not be
  claimed as supported without evidence of the required held-key behavior.
- Chat keeps its existing identity bindings and disables game remaps.
  Chat transitions while F3 is held must not leave a stuck key/layer or emit
  resize signals from typed text.
  Layer definitions accepted by the privileged helper remain narrowly
  validated; no arbitrary keyd commands or configuration text are accepted.

## Activation, failure, and recovery contract

- Bind the applied profile, compiled routing plan, keyd mappings, and GNOME grabs
  to one profile generation and a fresh per-activation token. Saving an edited profile must not activate either half of
  a different generation.
- Preflight actual engine capability and extension/helper protocol compatibility.
  Unsupported installations retain the currently applied configuration and
  report the missing capability before changing it.
- During changeover, disable the old keyd producer before changing GNOME signal
  bindings. Install and acknowledge the new GNOME consumer before enabling the
  new producer. Publish the applied generation only after both succeed. Prevent
  in-flight old signals from being interpreted as commands of the new generation.
  Reserve F24 as an ordered drain marker: PrepareRouting removes old consumers
  and installs a temporary marker grab, keyd emits F24 after producer reset,
  and GNOME refuses commit until the marker is consumed. On failure/timeout,
  abort removes the temporary grab. Fresh per-activation tokens reject queued
  D-Bus signals even when the same profile is applied again or reconnected.
- On partial failure, disable the new producer, remove its consumer, and restore
  the previous coherent generation. If rollback cannot be verified, leave managed
  shortcuts inactive and expose recovery status; never mix generations.
- Startup/reconnect begins with no active producer or consumer. Restore only the
  persisted applied plan after capability checks, both components acknowledge
  its generation, and valid Minecraft focus. Reconcile interrupted activation.
- Focus loss, modal entry, shutdown, restore, and profile replacement release
  managed held keys/layers and deactivate shortcuts. Focus regain must not invent
  presses for keys first pressed outside the active session.
- Preserve existing profiles/restoration records. An older extension or helper
  cannot silently participate in the new protocol; report the required upgrade.

## Verification

Add regression tests for F3+B, F3+G, F3+F, Shift+F3+F, and Shift+F3 with native F3
and Tab mapped to F3. Include overlapping resize assignments, repeated F
presses while F3 remains held, Shift pressed before and after F3, releasing
Shift while F3 remains held, and absence of extra debug-screen toggles.
Also cover immediate resize after a standalone F3 tap, multiple F3 sources,
chat/focus/modal transitions and profile replacement while keys are held,
internal signal allocation/collisions, modifiers, eye zoom, generation consistency,
saved versus applied profiles, incompatible components, partial failures,
rollback/restart, and narrow privileged validation. Engine event-level proof is
required; generated-configuration string assertions cannot replace it.
The prototype establishes engine output only, not chat, reset, GNOME, or
Minecraft behavior. Run the project's Rust and JavaScript
test suites, formatting checks, and `graphify update .`.

A real GNOME/Minecraft session is still needed to verify that B resizes,
Tab+B toggles hitboxes, Tab+G toggles chunk borders, Tab+F increases render
distance, Shift+Tab+F decreases it, and Shift+Tab shows the profiler pie chart,
without resizing or activating eye zoom. Repeat with physical F3 and verify
that shortcuts resume after releasing
F3/Tab. Automated tests alone cannot establish that desktop acceptance result.

## Implementation decisions and review fixes

- Exact modifier matching is provided by the maintained `matchmods` primitive;
  ordinary Shift+B remains game input unless explicitly assigned.
- AltGr/rightalt resize is rejected until its GNOME translation is established.
- Signals already used by observed Minecraft options cause Preview to refuse
  activation; allocation remains deterministic from the applied profile rather
  than silently varying across instances.
- Unchanged profile reconciliation preserves consumer acknowledgement. Late
  focus-time grab failures are exposed as blocked focus, preventing production.
- F3-layer command/custom chat openers preserve chat transitions. Known Java
  1.16 handled debug keys, such as F3+T, retain their game debug behavior.
