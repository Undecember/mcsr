# Key capture for binding fields

Make key bindings easier to enter by adding an explicit “Record input…” button beside each keystroke field. Preserve manual entry and existing profile serialization.

## Scope

- Capture physical keys in Minecraft mappings, both crafting mapping fields, Ninjabrain physical keys and shortcuts, chat keys, and resize preset shortcuts.
- Keep Minecraft action selection, control search, custom control IDs, preset names, dimensions, and snippet text as their existing inputs.
- Capture one keyboard key or supported mouse button for single-input fields. Capture modifiers plus a final non-modifier key for shortcut fields, preserving left/right modifier distinctions and existing backend syntax.

## Approaches

- Explicit capture button plus manual entry is recommended: users deliberately enter listening mode and can still enter desktop-intercepted keys manually.
- Capturing whenever an input receives focus makes text editing and keyboard navigation unpredictable.
- A key-name dropdown avoids typing names but is slower to browse and cumbersome for shortcuts.

## Behavior

- Listen only after activating capture and completing its initiating click or key release. Show an accessible “Press a key or mouse button… 5s” countdown. Do not show a Cancel button.
- Escape and Tab are bindable during capture. Expire after five seconds without a supported complete binding, retaining the previous value. Loss of window focus or removal of the row also stops recording without changing the value. During recording, input is reserved for capture rather than other page actions.
- Use physical key codes to translate supported keys into backend names, including Backspace, function keys, navigation keys, and left/right modifiers. Unsupported or composing events leave the value unchanged and explain how to proceed without extending the deadline.
- Suppress default browser behavior for captured events where possible, ignore repeats, and stop listening after a successful capture. Capture left, middle, right, back, and forward mouse buttons where browser delivery and runtime support permit. Suppress context menus, auxiliary clicks, and back/forward navigation where cancelable, including follow-up events from the recorded gesture. Browser delivery of extra buttons is not universal; do not promise every mouse event. Desktop-reserved combinations remain available through manual entry.
- Update the existing input and invalidate previews through existing change tracking only when a value changes.
- Extend backend validation and Minecraft key translation as needed for supported mouse buttons; the current Minecraft encoder accepts keyboard keys only. Verify actual runtime support for mouse sources, emitted buttons, and shortcut destinations before enabling capture for each field. Do not save a mouse binding that its destination cannot execute. Wheel scrolling, pointer movement, and extra buttons beyond browser/runtime support are outside the initial scope.

## Validation

Test key-name translation, modifier shortcuts, capture across every binding field, unchanged Minecraft action inputs, timeout and focus loss, initiating-click exclusion, mouse buttons and follow-up default actions, unsupported events, profile round trips, and preview invalidation. Run the frontend and Rust test suites and update Graphify after code changes.

The user confirmed scope across all keystroke fields while excluding Minecraft control items and requested mouse capture with timeout instead of an explicit Cancel button. The five-second duration, supported mouse-button scope, physical-code mapping, and manual fallback are proposed implementation choices.

## Verified runtime constraints

The managed keyd `[ids] *` configuration matches keyboards, not physical mice. Mouse recording is enabled for native Minecraft control bindings and emitted crafting/Ninjabrain bindings. Crafting/Ninjabrain physical sources and chat fields record keyboard inputs; mouse triggers require explicit device enrollment and remain available through manual configuration. Resize presets use GNOME keyboard accelerators and record keyboard shortcuts only. Supporting device selection/enrollment is outside this change.

Keyd canonical mouse names are `leftmouse`, `middlemouse`, `rightmouse`, `mouse1`, and `mouse2`; Minecraft targets are respectively `key.mouse.left`, `key.mouse.middle`, `key.mouse.right`, `key.mouse.4`, and `key.mouse.5`. Browser event button codes 0–4 map to those names.
