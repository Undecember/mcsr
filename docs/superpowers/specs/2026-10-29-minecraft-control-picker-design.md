# Minecraft control picker

Replace the Minecraft control ID text field with a searchable picker so users can choose a familiar default key and action, such as `1 — Hotbar slot 1`, `W — Move forward`, or `Space — Jump`, before entering their preferred physical key.

## Approaches

- A picker shows the default key alongside the action and supports browsing and searching. The user selected this approach.
- Pressing a default key is quick but less discoverable and cannot distinguish actions sharing a key without another selection step.
- Keeping a raw ID field requires users to know Minecraft's internal names and does not address the request.

## Behavior and constraints

- Search matches default keys and action names; each entry retains its exact Minecraft options ID as its stored value.
- Use Minecraft Java 1.16 defaults, matching the project's existing options fixture. Include keyboard and mouse controls and identify unbound controls by action name.
- Labels always describe defaults, independent of an instance's current bindings.
- Keep the existing physical key field and profile/API format. Existing known IDs load as the corresponding selection.
- Provide a custom control ID option for modded or unknown controls. Preserve unknown IDs when loading and saving profiles.
- Selecting or changing a control invalidates the preview through the existing change tracking. Existing duplicate and incomplete-row validation continues to apply.
- Keep search and selection accessible by keyboard and label both picker and custom input clearly.

## Scope and validation

This is a small frontend change using the existing JavaScript and CSS. No backend migration or key-press capture is needed. Verify picker filtering and selection, known and unknown profile round trips, duplicate validation, and preview invalidation. Rebuild Graphify after implementation.

The searchable interaction, custom-ID fallback, and Java 1.16 baseline are proposed implementation choices.
