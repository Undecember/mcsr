"""Resize preset coordination and optional maccel sensitivity management."""

from __future__ import annotations

import ast
import json
import math
import re
import shutil
import subprocess
from collections.abc import Callable
from dataclasses import dataclass
from typing import Protocol

from mcsr_toolkit.focus import FocusEvent
from mcsr_toolkit.models import Profile
from mcsr_toolkit.sensitivity import SensitivityAdapter, SensitivityError


class ResizeError(RuntimeError):
    """The requested preset could not be fully applied."""


class WindowBus(Protocol):
    def resize_focused(self, width: int, height: int) -> tuple[int, int]: ...


MaccelRunner = Callable[[list[str]], subprocess.CompletedProcess[str]]


def _run_maccel(args: list[str]) -> subprocess.CompletedProcess[str]:
    try:
        return subprocess.run(args, capture_output=True, text=True, timeout=3, check=False)
    except (OSError, subprocess.SubprocessError) as error:
        raise ResizeError(f"maccel failed: {error}") from error


@dataclass(frozen=True)
class ResizeResult:
    preset_id: str
    width: int
    height: int
    sensitivity: str | None


class SensitivityControl(Protocol):
    @property
    def needs_restore(self) -> bool: ...

    def eye(self) -> None: ...

    def restore(self) -> None: ...


class PresetController:
    """Resize first; let the runtime verify focus before activating eye sensitivity."""

    def __init__(
        self, window_bus: WindowBus, *, sensitivity: SensitivityControl | None = None
    ) -> None:
        self.window_bus = window_bus
        self.sensitivity = sensitivity if sensitivity is not None else SensitivityAdapter()
        self.active_preset: str | None = None
        self._pending_eye: ResizeResult | None = None

    def restore_sensitivity(self) -> None:
        self._pending_eye = None
        self.active_preset = None
        if self.sensitivity.needs_restore:
            try:
                self.sensitivity.restore()
            except SensitivityError as error:
                raise ResizeError(str(error)) from error

    def focus_lost(self) -> None:
        self.restore_sensitivity()

    def activate_eye(self, result: ResizeResult) -> ResizeResult:
        if result is not self._pending_eye:
            raise ResizeError("eye resize is absent or stale")
        self._pending_eye = None
        self.active_preset = None
        try:
            self.sensitivity.eye()
        except SensitivityError as error:
            raise ResizeError(str(error)) from error
        self.active_preset = result.preset_id
        return ResizeResult(result.preset_id, result.width, result.height, "0.02083")

    def apply_preset(self, profile: Profile, preset_id: str, shifted: bool = False) -> ResizeResult:
        return self.resize_preset(profile, preset_id, shifted)

    def resize_preset(
        self, profile: Profile, preset_id: str, shifted: bool = False
    ) -> ResizeResult:
        self.active_preset = None
        self._pending_eye = None
        preset = next((item for item in profile.presets if item.id == preset_id), None)
        if preset is None:
            raise ResizeError(f"unknown preset: {preset_id}")
        try:
            _validate_size(preset.width, preset.height)
            actual_width, actual_height = self.window_bus.resize_focused(
                preset.width, preset.height
            )
            if (actual_width, actual_height) != (preset.width, preset.height):
                raise ResizeError(
                    f"requested size {preset.width}x{preset.height}, "
                    f"got {actual_width}x{actual_height}"
                )
        except Exception as resize_error:
            # A normal preset ends eye sensitivity even if its window operation fails.
            if not preset.eye_zoom:
                try:
                    self.restore_sensitivity()
                except ResizeError as restore_error:
                    raise ResizeError(
                        f"{resize_error}; sensitivity recovery failed: {restore_error}"
                    ) from resize_error
            raise
        result = ResizeResult(preset.id, actual_width, actual_height, None)
        if preset.eye_zoom and not shifted:
            self._pending_eye = result
        else:
            if not preset.eye_zoom:
                self.restore_sensitivity()
            self.active_preset = preset.id
        return result


def _validate_size(width: int, height: int) -> None:
    if any(type(value) is not int or not 100 <= value <= 16384 for value in (width, height)):
        raise ResizeError("invalid Minecraft window size (100–16384 pixels per dimension)")


def preset_configuration(profile: Profile) -> list[dict[str, str | bool]]:
    """Translate keyd shortcuts to GNOME accelerators, rejecting unknown names."""
    aliases = {
        "slash": "slash",
        "enter": "Return",
        "esc": "Escape",
        "space": "space",
        "tab": "Tab",
        "backspace": "BackSpace",
        "delete": "Delete",
        "insert": "Insert",
        "home": "Home",
        "end": "End",
        "pageup": "Page_Up",
        "pagedown": "Page_Down",
        "up": "Up",
        "down": "Down",
        "left": "Left",
        "right": "Right",
        "minus": "minus",
        "equal": "equal",
        "comma": "comma",
        "dot": "period",
        "semicolon": "semicolon",
        "apostrophe": "apostrophe",
        "backslash": "backslash",
        "leftbrace": "bracketleft",
        "rightbrace": "bracketright",
        "grave": "grave",
    }
    modifiers = {"control": "Control", "shift": "Shift", "alt": "Alt", "meta": "Super"}
    configured: list[dict[str, str | bool]] = []
    accelerators: set[str] = set()
    for preset in profile.presets:
        _validate_size(preset.width, preset.height)
        parts = preset.shortcut.split("+")
        prefix = "".join(
            f"<{modifiers[part.removeprefix('left').removeprefix('right')]}>" for part in parts[:-1]
        )
        key = parts[-1]
        if re.fullmatch(r"[a-z0-9]|f(?:[1-9]|1[0-9]|2[0-4])", key):
            symbol = key.upper() if key.startswith("f") and len(key) > 1 else key
        else:
            symbol = aliases.get(key, "")
        if not symbol:
            raise ResizeError(f"unsupported GNOME shortcut key: {key}")
        accelerator = prefix + symbol
        canonical = "".join(sorted(re.findall(r"<[^>]+>", prefix))) + symbol
        if canonical in accelerators:
            raise ResizeError("duplicate GNOME preset shortcut")
        accelerators.add(canonical)
        configured.append(
            {"id": preset.id, "accelerator": accelerator, "eye_zoom": preset.eye_zoom}
        )
    eye = next((item for item in configured if item["eye_zoom"]), None)
    if eye is not None:
        accelerator = str(eye["accelerator"])
        if "<Shift>" in accelerator:
            raise ResizeError(
                "eye zoom shortcut must leave Shift available for size-only activation"
            )
        shifted = "".join(sorted(re.findall(r"<[^>]+>", "<Shift>" + accelerator)))
        shifted += re.sub(r"<[^>]+>", "", accelerator)
        if shifted in accelerators:
            raise ResizeError("shifted eye zoom conflicts with another preset")
    return configured


CommandRunner = Callable[[list[str]], subprocess.CompletedProcess[str]]


def _run_bus(args: list[str]) -> subprocess.CompletedProcess[str]:
    try:
        return subprocess.run(args, capture_output=True, text=True, timeout=5, check=False)
    except (OSError, subprocess.SubprocessError) as error:
        raise ResizeError(f"GNOME extension unavailable: {error}") from error


class GnomeWindowBus:
    """Fixed session-bus calls; replies are parsed as a restricted GVariant tuple."""

    def __init__(self, runner: CommandRunner = _run_bus) -> None:
        self.runner = runner

    def _call(self, method: str, *arguments: str) -> str:
        result = self.runner(
            [
                "gdbus",
                "call",
                "--session",
                "--dest",
                "org.gnome.Shell",
                "--object-path",
                "/org/mcsr/Toolkit",
                "--method",
                f"org.mcsr.Toolkit.{method}",
                *arguments,
            ]
        )
        if result.returncode:
            raise ResizeError(f"GNOME extension call failed: {result.stderr.strip()}")
        return result.stdout.strip()

    def resize_focused(self, width: int, height: int) -> tuple[int, int]:
        _validate_size(width, height)
        reply = self._call("ResizeFocused", str(width), str(height))
        match = re.fullmatch(r"\((?:uint32 )?(\d+), (?:uint32 )?(\d+)\)", reply)
        if match is None:
            raise ResizeError("invalid GNOME resize reply")
        actual = (int(match[1]), int(match[2]))
        if any(not 0 < value <= 2**32 - 1 for value in actual):
            raise ResizeError("invalid GNOME resize reply dimensions")
        return actual

    def configure_presets(self, profile: Profile) -> None:
        payload = json.dumps(preset_configuration(profile), separators=(",", ":"))
        # Profile IDs and translated accelerators contain no quotes or backslashes.
        reply = self._call("ConfigurePresets", "'" + payload + "'")
        if reply != "()":
            raise ResizeError("invalid GNOME configuration reply")

    def get_focus(self) -> FocusEvent:
        quoted = r"(?:'(?:\\.|[^'\\])*'|\"(?:\\.|[^\"\\])*\")"
        match = re.fullmatch(
            rf"\(({quoted}), ({quoted}), (?:uint32 )?\d+, (?:uint32 )?\d+, (true|false)\)",
            self._call("GetFocus"),
        )
        if match is None:
            raise ResizeError("invalid GNOME focus reply")
        try:
            window_class, title = ast.literal_eval(match[1]), ast.literal_eval(match[2])
        except (ValueError, SyntaxError) as error:
            raise ResizeError("invalid GNOME focus strings") from error
        return FocusEvent(window_class, title, match[3] == "true")


@dataclass(frozen=True)
class MaccelStatus:
    available: bool
    usable: bool
    detail: str


def maccel_status(binary: str | None = None, runner: MaccelRunner = _run_maccel) -> MaccelStatus:
    """Read-only check of the optional executable and accessible sensitivity device."""
    executable = binary if binary is not None else shutil.which("maccel")
    if executable is None:
        return MaccelStatus(False, False, "maccel is not installed")
    try:
        result = runner([executable, "get", "param", "sens-mult"])
        if result.returncode:
            return MaccelStatus(True, False, result.stderr.strip() or "maccel read failed")
        value = float(result.stdout.strip())
        if not math.isfinite(value) or value <= 0:
            raise ValueError("invalid sensitivity multiplier")
    except (ResizeError, OSError, subprocess.SubprocessError, ValueError) as error:
        return MaccelStatus(True, False, str(error))
    return MaccelStatus(True, True, format(value, ".15g"))
