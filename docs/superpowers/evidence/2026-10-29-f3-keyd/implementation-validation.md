# Managed keyd and F3 routing implementation verification

Implementation worktree: `/tmp/mcsr-f3-implementation`, branch `feat/f3-priority`,
based on `04febc4` from `dev`. The original checkout's local config and graph
changes were not modified by this implementation.

## Automated results

- `cargo test --locked`: 378 passed across 23 suites; zero failures.
- `cargo clippy --all-targets --locked -- -D warnings`: passed.
- `cargo fmt --check`: passed.
- `python3 packaging/keyd/test.py`: 94 upstream and 28 toolkit engine cases,
  plus held-key reset cleanup, passed.
- `node tests/extension_harness.cjs`: passed, including late grab failure,
  generation callbacks, phased input drain, abort, and watchdog expiry.
- `node tests/ui_harness.cjs`: passed.
- `node integration/live-ui-acceptance.mjs self-test`: passed fixture test.
- `glib-compile-schemas --strict --dry-run extension/schemas`: passed.
- `cargo build --release --locked`: passed.
- `cargo test --release --locked --test rust_assets`: 6 passed.
- `graphify update .`: completed; AST graph refreshed without API calls.

Release executable SHA256:
`d670753f5258ed8f4c694c56726a0b88c796ebbb3431317df8029fa22b625ba0`.
This checksum identifies the local reviewed build, not a published release.

## Review findings and regressions

An independent whole-change review identified three important issues. Each had
an observed failing reproduction and a passing regression after its fix:

1. Slash command chat opened under held F3 must retain chat identity input.
2. A consumer grab failure on focus regain must report blocked focus.
3. Unchanged reconciliation must preserve the producer's consumer acknowledgement.

Queued-signal uncertainty was also addressed: activations have unique tokens,
and an ordered F24 marker must drain the old input queue before new consumer
bindings commit. Bridge/extension tests establish phase order and safe abort;
actual compositor ordering still requires desktop acceptance.

Initial concurrent baseline test runs exposed short job-poll deadlines and a
scheduler-dependent fake restore reservation. The API tests now use a bounded
10-second polling deadline and hold the fake restore until PUT observes its
reservation. The final ordinary parallel Cargo command passed.

## Limits

No daemon, helper, system service, installed configuration, or desktop extension
was changed on this host. Installer/service tests use controlled adapters and
sandbox files. Real GNOME/Minecraft acceptance of the five combinations,
chat/focus transitions, and actual install/repair/uninstall service recovery is
pending. AltGr resize presets and cross-compilation are explicitly unsupported;
Preview refuses observed internal-signal collisions.
