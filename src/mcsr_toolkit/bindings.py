"""Compose keyd's emitted keys with Minecraft's per-instance controls."""

from __future__ import annotations

from dataclasses import dataclass, field

from .models import Profile
from .options import OptionsDocument

# Minecraft 1.16.1's official en_us key identifiers, paired with keyd's
# keycode table. A keyd key is not necessarily a GLFW/Minecraft keyboard key.
KEYD_TO_MINECRAFT = {
    **{key: key for key in "abcdefghijklmnopqrstuvwxyz0123456789"},
    **{f"f{number}": f"f{number}" for number in range(1, 25)},
    **{f"kp{number}": f"keypad.{number}" for number in range(10)},
    **{key: key for key in (
        "apostrophe", "backslash", "backspace", "comma", "delete", "end", "enter",
        "equal", "home", "insert", "minus", "pause", "semicolon", "slash", "space",
        "tab", "up", "down", "left", "right", "menu",
    )},
    "esc": "escape",
    "escape": "escape",
    "dot": "period",
    "leftbrace": "left.bracket",
    "rightbrace": "right.bracket",
    "grave": "grave.accent",
    "capslock": "caps.lock",
    "numlock": "num.lock",
    "scrolllock": "scroll.lock",
    "pageup": "page.up",
    "pagedown": "page.down",
    "leftshift": "left.shift",
    "rightshift": "right.shift",
    "leftcontrol": "left.control",
    "rightcontrol": "right.control",
    "leftalt": "left.alt",
    "rightalt": "right.alt",
    "leftmeta": "left.win",
    "rightmeta": "right.win",
    "sysrq": "print.screen",
    "kpplus": "keypad.add",
    "kpdot": "keypad.decimal",
    "kpenter": "keypad.enter",
    "kpequal": "keypad.equal",
    "kpasterisk": "keypad.multiply",
    "kpslash": "keypad.divide",
    "kpminus": "keypad.subtract",
}
MINECRAFT_TO_KEYD = {value: key for key, value in KEYD_TO_MINECRAFT.items()}
MINECRAFT_TO_KEYD["escape"] = "esc"
CHAT_CONTROLS = {"key_key.chat": "open_chat", "key_key.command": "open_command"}


@dataclass(frozen=True)
class ManagedState:
    original: dict[str, str] = field(default_factory=dict)
    last_emitted: dict[str, str] = field(default_factory=dict)


@dataclass(frozen=True)
class ControlPlan:
    updates: dict[str, str]
    preview: list[tuple[str, str, str]]
    conflicts: list[str]
    next_state: ManagedState


def _decode_binding(binding: str) -> str | None:
    prefix = "key.keyboard."
    if not binding.startswith(prefix):
        return None
    suffix = binding[len(prefix) :]
    return MINECRAFT_TO_KEYD.get(suffix)


def _encode_binding(key: str) -> str:
    if key not in KEYD_TO_MINECRAFT:
        raise ValueError(f"unsupported Minecraft key: {key}")
    return "key.keyboard." + KEYD_TO_MINECRAFT[key]


def plan_controls(
    document: OptionsDocument, profile: Profile, prior: ManagedState | None
) -> ControlPlan:
    prior = prior or ManagedState()
    conflicts: list[str] = []
    updates: dict[str, str] = {}
    preview: list[tuple[str, str, str]] = []
    original: dict[str, str] = {}
    last_emitted: dict[str, str] = {}
    effective: dict[str, str] = {}
    affected: set[str] = set()

    for key in profile.controls.values():
        _encode_binding(key)
    for key in (profile.chat.open_chat, profile.chat.open_command):
        _encode_binding(key)
    for action in profile.controls:
        if action not in document.bindings:
            conflicts.append(f"unknown Minecraft control: {action}")

    # Missing default chat entries use Minecraft's built-in defaults. Custom
    # entries must be explicit, including in newly created minimal options files.
    bindings = dict(document.bindings)
    for action, field_name in CHAT_CONTROLS.items():
        key = getattr(profile.chat, field_name)
        default = "t" if field_name == "open_chat" else "slash"
        if action not in bindings and key != default:
            bindings[action] = _encode_binding(default)

    for action, current in bindings.items():
        observed = _decode_binding(current)
        if observed is None and current != "key.keyboard.unknown" and not (
            current in {"key.mouse.left", "key.mouse.right", "key.mouse.middle"}
            or current in {f"key.mouse.{number}" for number in range(4, 9)}
        ):
            raise ValueError(f"unsupported Minecraft binding for {action}: {current}")
        physical: str | None
        if prior.last_emitted.get(action) == current and action in prior.original:
            physical = prior.original[action]
        else:
            physical = observed
        physical = profile.controls.get(action, physical)
        if action in CHAT_CONTROLS:
            chat_physical = getattr(profile.chat, CHAT_CONTROLS[action])
            if action in profile.controls and physical != chat_physical:
                conflicts.append(f"Minecraft chat control differs from chat key: {action}")
            physical = chat_physical
        if physical is None:
            effective[action] = current
            continue
        emitted = profile.crafting.get(physical, physical)
        target = _encode_binding(emitted)
        effective[action] = target
        if target != current or action not in document.bindings:
            updates[action] = target
        if action in updates or action in prior.original or action in profile.controls:
            affected.add(action)
            preview.append((physical, emitted, action))
            original[action] = physical
            last_emitted[action] = target
        if physical in profile.ninjabrain:
            conflicts.append(f"Ninjabrain shortcut uses Minecraft control key: {action}")

    by_key: dict[str, list[str]] = {}
    for action, field_name in CHAT_CONTROLS.items():
        effective.setdefault(action, _encode_binding(getattr(profile.chat, field_name)))
        affected.add(action)
    for action, output in effective.items():
        by_key.setdefault(output, []).append(action)
    for output, actions in by_key.items():
        if len(actions) > 1 and any(action in affected for action in actions):
            conflicts.append(f"emitted {output} conflicts: {', '.join(actions)}")

    return ControlPlan(updates, preview, conflicts, ManagedState(original, last_emitted))
