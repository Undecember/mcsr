# Held-F3 feasibility spike

This directory contains a throwaway keyd engine experiment, not a shipped toolkit
change. Nothing was installed, and the system keyboard daemon was not changed.

Upstream: https://github.com/rvaiya/keyd

Revision: `f564288ac2b19d2305a5b39023c474805ff8fce5`.

## Result

The plain `tab = f3` mapping preserves held F3+B without resize mappings.
The collision is not evidence that the basic remap releases F3 early.

With resize signals configured as B→F13, G→F14, F→F15, these results compare
actual keyd engine output against ordinary F3 key-down/key-up semantics:

| Candidate | Passing event cases | Limitation |
|---|---:|---|
| Direct F3 mapping | 8/20 | Does not bypass resize mappings. |
| `overload` | 5/20 | Does not emit held F3; Shift pressed after Tab can suppress the tap entirely. |
| `layerm` | 5/20 | Emits and releases F3 before the combination key. |
| `oneshotk` | 17/20 | After a standalone F3 tap, the layer remains active for the next key. |
| Prototype `layerk` | 20/20 | Requires a keyd engine change; not available in tested stock keyd. |

The prototype adds a momentary layer action that also forwards the F3 key's
press and release, without one-shot latching. It changes three upstream source
files; see `layerk-prototype.patch`. It does not demonstrate reset, multiple
simultaneous F3 sources, chat, privileged validation, GNOME signal delivery,
or Minecraft behavior. Those remain integration requirements.

All 94 existing upstream event tests also passed after applying the prototype;
their output is in `upstream-tests.log`. Exact candidate results and traces are
in `results.json`. The `.t` files contain input events, then a blank line, then
expected output events. Each candidate/case was run in a fresh process.

## Reproduce

In a disposable checkout of the pinned upstream revision, run `make test-io`.
Before applying the patch, invoke `bin/test-io <candidate.conf> <case.t>` for
each stock candidate and each case separately. `layerk.conf` must fail to parse
on that revision before the patch.

Apply `layerk-prototype.patch` with `git apply`, then run `make test-io` again.
Invoke `bin/test-io <layerk.conf> <case.t>` separately for all twenty cases.
Paths to the supplied files can be absolute. Do not run `make install`, and do
not configure or restart a desktop daemon for this experiment.

The outcome supports a specific required primitive, not a conclusion that all
stock-keyd configurations or alternative backends are impossible. Distribution
of a compatible daemon remains a design decision.
