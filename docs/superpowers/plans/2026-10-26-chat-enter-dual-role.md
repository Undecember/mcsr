# Enter Opens and Sends Chat Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Let Enter open Minecraft chat and send from the chat layer while preserving an instance with hotbar slot 7 on T.

**Architecture:** Keep the existing two keyd layers and mapping pipeline. Relax only the profile duplicate-key rule for `open_chat=send=enter`; the renderer already places opening and sending actions in different layers. Update the supervised browser driver to accept only this profile and its exact options/keyd effects. Keep all real-desktop operations until after code checks and review.

**Tech Stack:** Rust, keyd 2.5, Node.js CDP acceptance driver, Ubuntu GNOME Shell 50 Wayland.

**Spec:** [Enter opens and sends chat design](../specs/2026-10-26-chat-enter-dual-role-design.md).

## Global Constraints

- Preserve the real `key_key.chat:key.keyboard.enter` and `key_key.hotbar.7:key.keyboard.t` bindings. The physical `s` → emitted `k` Backward test may change only Backward before Restore.
- Permit only `open_chat=send=enter` as the newly valid duplicate; reject all other duplicate chat-key pairs and reserved-key assignments.
- Do not install helpers, change `/etc/keyd`, or edit the real Minecraft options during implementation. The current installed binary stays observe-only until a separately approved exact-hash replacement.
- Do not commit, merge, tag, or push without the user's separate authorization. The existing Subagent-Driven method applies after plan approval.

## Review Focus

- Enter bound to both roles must create one `togglem` game binding and one `clearm` chat binding, with no duplicate within either layer; Task 1 tests both strings.
- A different duplicate such as chat-open=command-open or send=cancel must remain invalid; Task 1 tests each pair.
- Existing Enter chat and T hotbar must Preview without conflict and leave both lines byte-identical; Task 1 tests a minimal options document.
- A successful Apply must modify only Backward, and Restore must return exact baseline bytes and empty lineage; Task 1 tests this with the fake keyd adapter.
- The browser driver must refuse an extra mapping, a changed instance, an unexpected keyd fragment, and any live Restore; Task 2 self-tests exact expected outputs and retains existing service/lineage gates.

---

### Task 1: Profile and mapping contract

**Files:** Modify `src/profile.rs`; test in `tests/rust_profile.rs` and `tests/rust_apply.rs`.

**Interfaces:** Preserve `parse_profile(&[u8], &KeySet) -> Result<Profile, ProfileError>`, `game_bindings(&Profile) -> Vec<String>`, `render_keyd_fragment(&Profile) -> String`, and `plan_controls(&OptionsDocument, &Profile, Option<&ManagedState>) -> Result<ControlPlan, MappingError>`.

- [ ] Add `enter_opens_and_sends_chat` in `tests/rust_profile.rs`: parse `chat.open_chat=enter, chat.send=enter`, assert game `enter = togglem(mcsr_chat, enter)`, chat `enter = clearm(enter)`, and no duplicate binding within each layer. Run `cargo test --locked --test rust_profile enter_opens_and_sends_chat`; expect failure at the current distinct-key validation.
- [ ] Add `other_chat_duplicates_still_fail` with open-chat=open-command, open-command=send, open-chat=cancel, send=cancel, and open-chat=send other than Enter. Add the real-layout mapping case with Backward S, Chat Enter, Command Slash, hotbar 7 T; assert no conflicts, only Backward S→K update, and exactly one preview mapping. Run the focused tests; expect the Enter case to fail and all rejection cases to pass.
- [ ] Change only the chat duplicate validation in `src/profile.rs` to accept the exact Enter pair and preserve all other reserved-key checks. Keep `game_bindings`, `render_keyd_fragment`, and mapping behavior unless a failing test proves a correction is needed. Run the focused tests; expect pass.
- [ ] Add an Apply/Restore regression in `tests/rust_apply.rs` using that real-layout fixture and fake keyd adapter: Apply changes only the Backward line, and Restore returns exact baseline bytes with no active lineage. Run `cargo test --locked --test rust_apply enter_chat_preserves_hotbar`; expect pass after the validator fix.
- [ ] Run `cargo fmt --check`, `cargo clippy --all-targets --locked -- -D warnings`, and `cargo test --locked`; require all pass. Submit the focused code and test diff for independent Sol high review because this changes live input semantics.

### Task 2: Browser acceptance contract and documentation

**Files:** Modify `integration/live-ui-acceptance.mjs`, `README.md`, and `integration/acceptance-wayland.md`.

**Interfaces:** Keep `node integration/live-ui-acceptance.mjs self-test`, `prepare`, and `restore` modes and the private state format; no new privileged command.

- [ ] Update the driver's exact allowlisted profile to `open_chat=send=enter`, expected Preview to only S→K Backward, expected post-Apply options to only Backward K, and expected keyd chat fragment to `enter = clearm(enter)` without a T entry. Extend `self-test` to reject the earlier T-chat/two-row preview and any extra mapping; run `node integration/live-ui-acceptance.mjs self-test` and require pass.
- [ ] Update README chat-key guidance to say Enter may occupy both opening and sending roles, with separate game/chat-layer behavior and all other duplicate pairs rejected. Record the earlier real Preview failure and the no-mutation recovery in the Wayland acceptance record. Run `node --check integration/live-ui-acceptance.mjs`, both Node harnesses, strict GNOME schema dry run, and `git diff --check`; require pass.
- [ ] Obtain independent review of the driver and docs against the exact Rust profile/mapping contract. Address material findings and rerun the affected deterministic checks.

### Task 3: Final supervised device check

**Files:** Update `integration/acceptance-wayland.md` and `docs/rust-release-checklist.md` with actual results.

**Interfaces:** Use the existing installed app/helper paths and guided Restore workflow. No implementation code is changed during this task.

- [ ] Build `cargo build --release --locked` and `cargo test --release --locked --test rust_assets`; record the new executable SHA256. Verify the source diff and obtain fresh exact-hash approval before replacing the root-owned helper or app. Keep `/etc/keyd` empty and the service observe-only until then.
- [ ] With Minecraft closed, snapshot its current `options.txt` bytes/hash and verify Chat Enter, hotbar 7 T, Backward S. Run the reviewed browser Save/Preview/Apply driver for the Enter profile. Require one ready instance, only S→K Backward mapping, exact expected keyd fragment, and a durable backup ID before reopening Minecraft.
- [ ] Complete the previously approved supervised chat, real password-modal, and guarded logout checks, with the root one-shot fallback timer only after its exact command/state is verified. Record observed clear timing and any limitation; do not claim zero leakage.
- [ ] Return the service to observe-only, make keyd healthy/unmasked, and use the browser Restore driver. Require the exact pre-test options hash, empty `/etc/keyd`, raw maccel baseline, clean lineage, and ordinary outside-game typing. Remove only the temporary profile, driver state, and timer after verification. Record a failure and stop release if any check cannot be recovered.

## Handoff

The user already chose Subagent-Driven implementation for this project. After approving this plan, use that method for Tasks 1–2, with independent reviews. Keep Task 3's real-desktop operations at the end and retain the separate AGENTS.md exact-hash/system authorization gate.
