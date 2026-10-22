"""Preserve Minecraft options lines while replacing selected control values."""

from __future__ import annotations

from dataclasses import dataclass
from typing import TYPE_CHECKING

if TYPE_CHECKING:
    from .bindings import ControlPlan


@dataclass(frozen=True)
class OptionsDocument:
    lines: tuple[str, ...]
    bindings: dict[str, str]


def parse_options(text: str) -> OptionsDocument:
    lines = tuple(text.splitlines(keepends=True))
    bindings: dict[str, str] = {}
    for line in lines:
        name, separator, raw_value = line.partition(":")
        if not separator or not name.startswith("key_"):
            continue
        if name in bindings:
            raise ValueError(f"duplicate control entry: {name}")
        bindings[name] = raw_value.rstrip("\r\n")
    return OptionsDocument(lines, bindings)


def render_options(document: OptionsDocument, plan: ControlPlan) -> str:
    if plan.conflicts:
        raise ValueError("cannot render conflicted controls")
    rendered: list[str] = []
    for line in document.lines:
        name, separator, _ = line.partition(":")
        if separator and name in plan.updates:
            ending = "\r\n" if line.endswith("\r\n") else "\n" if line.endswith("\n") else ""
            rendered.append(f"{name}:{plan.updates[name]}{ending}")
        else:
            rendered.append(line)
    additions = {
        name: value for name, value in plan.updates.items() if name not in document.bindings
    }
    if additions:
        ending = "\r\n" if any(line.endswith("\r\n") for line in document.lines) else "\n"
        if rendered and not rendered[-1].endswith("\n"):
            rendered.append(ending)
        rendered.extend(f"{name}:{value}{ending}" for name, value in additions.items())
    return "".join(rendered)
