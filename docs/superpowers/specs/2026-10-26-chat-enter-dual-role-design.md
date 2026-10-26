# Enter opens and sends chat

## Intent

MCSR Toolkit must preserve a Minecraft instance that binds chat to Enter and hotbar slot 7 to T. The current profile validator requires all four chat keys to differ, so it cannot represent Enter both as the key that opens chat and the key that sends a message. The first public Rust release should support that existing control layout without changing the hotbar binding.

## Approaches

- **Chosen: allow the same Enter key to open and send chat.** The game layer sends Enter and activates `mcsr_chat`; the chat layer sends Enter and clears itself. This uses keyd's existing layer-specific actions and keeps Minecraft's `key_key.chat` at Enter. The profile validator still rejects other duplicate chat-key assignments. The user selected this approach.
- Temporarily choose a different chat-entry key. That would make the current instance pass Preview but would change its preferred chat control during the test, so the user declined it.

## Behavior and constraints

The only newly valid duplicate is `open_chat=send=enter`. `open_command`, `cancel`, and every other pair remain distinct. The profile continues to reserve Enter against crafting, Ninjabrain, and movement assignments. For this configuration, `game_bindings` emits `enter = togglem(mcsr_chat, enter)` while the chat layer emits `enter = clearm(enter)`. The two lines belong to different layers; no duplicate binding is emitted within one layer. Pressing Enter in Minecraft opens chat; pressing Enter again while chat is active sends and exits. Slash still opens command chat; Escape still cancels. Focus loss and GNOME modal cleanup continue to clear the active layer.

This layer-specific use follows [keyd's documented layer stacking and `togglem`/`clearm` actions](https://github.com/rvaiya/keyd/blob/master/docs/keyd.scdoc). The live acceptance remains necessary to confirm the installed keyd 2.5 behavior on this desktop.

`plan_controls` must keep an existing `key_key.chat:key.keyboard.enter` line byte-for-byte and leave the user's `key_key.hotbar.7:key.keyboard.t` unchanged. Preview of the tested S→K profile must report only the Backward mapping and no control conflicts. Apply and Restore must change only the Backward option, preserve the original file bytes on Restore, and retain existing backup lineage rules. The browser acceptance driver must check this exact profile and these exact outputs before it can click Apply.

## Verification and rollout

Add failing Rust tests for profile acceptance and rejection of other duplicate pairs, distinct keyd game/chat actions, a real-options fixture with Enter chat and T hotbar, and exact preview/Apply/Restore behavior. Run the Rust suite, formatting, Clippy, release build/assets, GNOME/UI harnesses, and independent review. No live keyd action occurs during implementation. The changed release executable must be reviewed and receive fresh exact-hash authorization before replacing either root-owned installed copy. Then repeat the supervised live UI, chat, password, logout, and Restore checks under the already approved test scope. If any live check fails, clear keyd, return the service to observe-only, and restore the exact pre-test options snapshot.

## Limits

The normal asynchronous focus and modal interval remains; this change does not claim that remaps clear atomically before every outside-game keystroke. The design does not add arbitrary duplicate chat keys or change a user's Minecraft controls to resolve unrelated conflicts.
