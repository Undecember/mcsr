# Managed stock-panel visibility validation

Status: implementation, automated checks, lifecycle review and final documentation/artifact review passed. Subsequent scoped live results are recorded in [the live record](2026-10-30-managed-panel-live.md).

## Provenance and execution

Worktree: `.worktrees/feat-managed-panel-visibility`.
Branch: `feat/managed-panel-visibility`, based on dev `5ae4c41c7f6660aac6f171252f017e34ad1778d3`.
The reviewed, uncommitted startup and borderless-toggle prerequisites were copied
from `.worktrees/feat-borderless-full-size-toggle`. Exact seeded files, SHA256s,
and tracked patch SHA256 are recorded in
[the baseline manifest](2026-10-30-managed-panel-baseline.json).
No build directories or untracked Graphify caches were copied.

The approved plan uses these dependency waves:

| Task | Depends on | Wave | Ownership | State |
| --- | --- | --- | --- | --- |
| P1 panel lifecycle and regression tests | seeded prerequisites | 1 | extension/extension.js, tests/extension_harness.cjs; panel_implementation | complete; independent re-review PASS |
| P2 documentation and combined validation | P1 verified and reviewed | 2 | README.md, integration/resize-wayland.md, evidence, Graphify; root | pending |
| R final independent lifecycle review | assembled P2 candidate | 3 | read-only review | complete; PASS |

One coupled implementation task avoids concurrent writes to the visibility
state machine and actual-extension harness. All work remains local and
uncommitted. Installation and merging are separate actions.

## Baseline

- `node tests/extension_harness.cjs`: passed before edits.
- `cargo test --locked --test rust_gnome --test rust_runtime --test rust_sensitivity`: 56 passed across three suites.

## Regression-first development

The new actual-extension regression failed before implementation:
`verified overlapping thin window hides panel` expected `visible=false`, observed
`visible=true` (`node tests/extension_harness.cjs`, exit 1). This establishes the
missing behavior; see [the retained failure output](2026-10-30-managed-panel-red.txt).

Unchanged surrounding checks passed: `cargo fmt --check`, 94 upstream and
28 toolkit keyd event cases plus held-key reset/IPC checks, 14 static UI tests,
and the live-UI result-fixture self-test.

## Frozen Task 1 candidate

- Extension SHA256: `6301f6ea24cc62d976ed972f050a3e537e7adef89288bea29674ffbe8278510e`.
- Harness SHA256: `08b2f5878369dc13e23a7bb0e339ecbf00820a3bd81ea7e3800414337aa35608`.
- Root reran actual-extension harness: passed.
- UI harness, strict schema dry-run and diff whitespace check: passed.
- Independent lifecycle review found a modal-event gap, subsequently repaired.
  A workspace gesture calls `Main.pushModal(global.stage)` with keyboard focus
  already null, so change-sensitive `notify::key-focus` does not fire. The panel
  stayed hidden until the heartbeat. The harness previously emitted that signal
  unconditionally; the repaired harness uses a source-ordered regression.

## Modal review repair

A second RED run reproduced a stage grab with unchanged null keyboard focus;
see [the retained failure](2026-10-30-managed-panel-modal-red.txt). The repair
observes `notify::is-grabbed` and treats active Shell grabs as ineligible. Tests
follow installed Shell ordering (grab before modalCount increments, dismiss
after modalCount decrements), including nested grabs. The actual-extension
harness, UI harness, schema dry run and whitespace checks passed afterward.
Independent re-review: PASS. No further material defect found. The reviewer
verified the installed Clutter 18 `Stage.is-grabbed` boolean property with GJS
and checked installed Shell source ordering independently.

## Combined checks

All commands completed successfully in this worktree. Rust/Clippy, extension,
UI and schema checks were rerun after the modal repair.

| Command/check | Result |
| --- | --- |
| `cargo fmt --check` | passed |
| `cargo clippy --all-targets --locked -- -D warnings` | no issues |
| `cargo test --locked` | 405 passed, 62 filtered, 23 suites; no failures |
| `python3 packaging/keyd/test.py` | 94 upstream and 28 toolkit event cases, held-key reset and IPC checks passed |
| `node tests/extension_harness.cjs` | passed |
| `node tests/ui_harness.cjs` | passed |
| `node --test static/app.test.cjs` | 14 passed |
| `node integration/live-ui-acceptance.mjs self-test` | passed |
| `glib-compile-schemas --strict --dry-run extension/schemas` | passed |
| `cargo build --release --locked` | passed after modal repair |
| `cargo test --release --locked --test rust_assets` | 6 passed |
| `graphify update .` | AST-only; 2698 nodes, 6582 edges, 131 communities |
| `git diff --check` | passed |

Candidate: `.worktrees/feat-managed-panel-visibility/target/release/mcsr-toolkit`.
SHA256: `11b73e7aa7715ae4abaae32789dacc227c4ab0fc8801d9c76e74a73dc2b5411f`.
The entire release extension archive was found byte-for-byte inside the
executable, and its `extension.js` exactly matched the frozen source above.
See [the artifact manifest](2026-10-30-managed-panel-artifact.json).

The 41 seeded file digests were checked; prerequisite source/evidence outside
the intentionally changed extension, harness, docs and generated graph remained
unchanged. Source worktrees and all uncommitted changes are preserved.

## API evidence

Implementation and review consulted GNOME Shell through Context7 and the exact
installed GNOME 50 resources in `/usr/lib/gnome-shell/libshell-18.so`:
`ui/layout.js` (stock tracked panel, visibility recomputation), `ui/main.js`
(modal/grab ordering), `ui/overview.js`, and `ui/sessionMode.js`.
`/usr/share/gnome-shell/modes/ubuntu.json` inherits `user` mode. The installed
Clutter 18 typelib exposes `Stage.is-grabbed`; native grab notifications cover
workspace gestures that do not change stage keyboard focus. Missing required
hooks disable this feature while retaining resize and focus delivery.

## Remaining live acceptance

The candidate was subsequently installed and scoped desktop checks passed;
see [the panel live record](2026-10-30-managed-panel-live.md).
Existing borderless-toggle live results and limitations remain in
[the earlier live record](2026-10-30-borderless-toggle-live.md).

Supervised checks are in [the resize guide](../../../integration/resize-wayland.md#managed-stock-panel-visibility-candidate-2026-10-30).
Final independent review found no material documentation/evidence inconsistency
and independently verified the binary checksum, archive embedding and frozen
extension bytes. Relative documentation links, new-file whitespace and
prerequisite digests passed final checks.

Top-edge input delivery passed the subsequent live check. Lock/unlock,
workspace gestures, disable/re-enable while hidden, eye windows and multimonitor
behavior remain live acceptance items. Alt+Tab with Tab mapped to F3, temporary F13–F24 translation,
and native F11/menu fullscreen problems retain their earlier limitations.
