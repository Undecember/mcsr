"""Validated user choices shared by the UI and system adapters."""

from __future__ import annotations

import re
import shutil
import string
import subprocess
from functools import lru_cache
from pathlib import Path

from pydantic import BaseModel, ConfigDict, Field, field_validator, model_validator

MODIFIER_KEYS = frozenset(
    {
        "leftcontrol",
        "rightcontrol",
        "leftshift",
        "rightshift",
        "leftalt",
        "rightalt",
        "leftmeta",
        "rightmeta",
    }
)


@lru_cache(maxsize=1)
def supported_keys() -> frozenset[str]:
    """Return keyd's vocabulary, with a small fallback for offline validation."""
    fallback = (
        frozenset(string.ascii_lowercase + string.digits)
        | frozenset(
            {
                "slash",
                "enter",
                "esc",
                "tab",
                "space",
                "leftalt",
                "rightalt",
                "leftshift",
                "rightshift",
                "leftcontrol",
                "rightcontrol",
                "leftmeta",
                "rightmeta",
            }
        )
        | frozenset(f"f{n}" for n in range(1, 25))
    )
    binary = shutil.which("keyd.rvaiya") or shutil.which("keyd")
    if binary is None:
        return fallback
    try:
        result = subprocess.run(
            [binary, "list-keys"], capture_output=True, text=True, check=True, timeout=3
        )
    except (OSError, subprocess.SubprocessError):
        return fallback
    return fallback | frozenset(
        line.strip().lower()
        for line in result.stdout.splitlines()
        if re.fullmatch(r"[a-z0-9][a-z0-9_]*", line.strip().lower())
    )


def _key(value: str) -> str:
    normalized = value.strip().lower()
    if not re.fullmatch(r"[a-z0-9][a-z0-9_]*", normalized) or normalized not in supported_keys():
        raise ValueError(f"unknown key: {value}")
    return normalized


def _shortcut(value: str) -> str:
    normalized = value.strip().lower()
    parts = normalized.split("+")
    modifiers = parts[:-1]
    families = [part.removeprefix("left").removeprefix("right") for part in modifiers]
    if (
        not normalized
        or any(
            not re.fullmatch(r"[a-z0-9][a-z0-9_]*", part) or part not in supported_keys()
            for part in parts
        )
        or any(part not in MODIFIER_KEYS for part in modifiers)
        or parts[-1] in MODIFIER_KEYS
        or len(families) != len(set(families))
    ):
        raise ValueError(f"invalid shortcut: {value}")
    return normalized


class ChatKeys(BaseModel):
    model_config = ConfigDict(extra="forbid")

    open_chat: str = "t"
    open_command: str = "slash"
    send: str = "enter"
    cancel: str = "esc"

    @field_validator("open_chat", "open_command", "send", "cancel")
    @classmethod
    def validate_key(cls, value: str) -> str:
        return _key(value)

    @model_validator(mode="after")
    def distinct_keys(self) -> ChatKeys:
        values = [self.open_chat, self.open_command, self.send, self.cancel]
        if len(values) != len(set(values)):
            raise ValueError("chat keys must be distinct")
        return self


class ResizePreset(BaseModel):
    model_config = ConfigDict(extra="forbid")

    id: str
    width: int = Field(ge=100, le=16384)
    height: int = Field(ge=100, le=16384)
    shortcut: str
    eye_zoom: bool = False

    @field_validator("id")
    @classmethod
    def validate_id(cls, value: str) -> str:
        if not re.fullmatch(r"[a-z][a-z0-9_-]{0,31}", value):
            raise ValueError("preset id must be a lowercase name")
        return value

    @field_validator("shortcut")
    @classmethod
    def validate_shortcut(cls, value: str) -> str:
        return _shortcut(value)


class Profile(BaseModel):
    model_config = ConfigDict(extra="forbid")

    controls: dict[str, str] = Field(default_factory=dict)
    crafting: dict[str, str] = Field(default_factory=dict)
    snippets: list[str] = Field(default_factory=list)
    ninjabrain: dict[str, str] = Field(default_factory=dict)
    chat: ChatKeys = Field(default_factory=ChatKeys)
    presets: list[ResizePreset] = Field(default_factory=list)
    launcher_root: Path | None = None

    @field_validator("controls")
    @classmethod
    def validate_controls(cls, values: dict[str, str]) -> dict[str, str]:
        normalized = {action: _key(key) for action, key in values.items()}
        if any(not action or "\n" in action or ":" in action for action in normalized):
            raise ValueError("invalid Minecraft control id")
        if len(normalized) != len(set(normalized.values())):
            raise ValueError("duplicate physical control key")
        return normalized

    @field_validator("crafting")
    @classmethod
    def validate_crafting(cls, values: dict[str, str]) -> dict[str, str]:
        return {_key(source): _key(target) for source, target in values.items()}

    @field_validator("snippets")
    @classmethod
    def validate_snippets(cls, values: list[str]) -> list[str]:
        normalized = [value.upper() for value in values]
        if any(not re.fullmatch(r"[A-Z0-9]{1,24}", value) for value in normalized):
            raise ValueError("snippets must use 1-24 letters or digits")
        return normalized

    @field_validator("ninjabrain")
    @classmethod
    def validate_ninjabrain(cls, values: dict[str, str]) -> dict[str, str]:
        return {_key(source): _shortcut(target) for source, target in values.items()}

    @model_validator(mode="after")
    def distinct_presets(self) -> Profile:
        if len(self.presets) > 8:
            raise ValueError("no more than eight presets are supported")
        chat_keys = {
            self.chat.open_chat,
            self.chat.open_command,
            self.chat.send,
            self.chat.cancel,
        }
        if chat_keys & (
            self.crafting.keys() | self.ninjabrain.keys() | set(self.controls.values())
        ):
            raise ValueError("reserved chat key is assigned elsewhere")
        if self.crafting.keys() & self.ninjabrain.keys():
            raise ValueError("physical key is assigned to both crafting and Ninjabrain")
        ids = [preset.id for preset in self.presets]
        shortcuts = [preset.shortcut for preset in self.presets]
        if len(ids) != len(set(ids)):
            raise ValueError("duplicate preset id")
        if len(shortcuts) != len(set(shortcuts)):
            raise ValueError("duplicate preset shortcut")
        if sum(preset.eye_zoom for preset in self.presets) > 1:
            raise ValueError("only one eye zoom preset is supported")
        return self
