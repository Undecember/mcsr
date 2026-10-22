"""Fail closed when GNOME focus messages stop or are malformed."""

from __future__ import annotations

import json

from mcsr_toolkit.focus import FocusController, FocusEvent


def _parse_event(message: str) -> FocusEvent:
    if len(message) > 4096:
        raise ValueError("invalid focus event size")
    try:
        value = json.loads(message)
    except json.JSONDecodeError as error:
        raise ValueError("invalid focus event JSON") from error
    if not isinstance(value, dict) or set(value) != {"class", "title", "modal"}:
        raise ValueError("invalid focus event fields")
    window_class, title, modal = value["class"], value["title"], value["modal"]
    if (
        not isinstance(window_class, str)
        or len(window_class) > 512
        or not isinstance(title, str)
        or len(title) > 1024
        or not isinstance(modal, bool)
    ):
        raise ValueError("invalid focus event values")
    return FocusEvent(window_class, title, modal)


class FocusWatchdog:
    def __init__(self, controller: FocusController, timeout: float = 2.0) -> None:
        self.controller = controller
        self.timeout = timeout
        self.last_seen: float | None = None

    def start(self, *, clear_state: bool = True) -> None:
        # The prior process may have died while game bindings were active.
        if clear_state:
            self.controller.keyd.clear_state()
        self.controller.game_active = False
        self.last_seen = None

    def observe(self, message: str, now: float) -> None:
        try:
            event = _parse_event(message)
        except ValueError:
            self.controller.on_disconnect()
            raise
        self.controller.on_focus(event)
        self.last_seen = now

    def tick(self, now: float) -> None:
        if self.last_seen is not None and now - self.last_seen >= self.timeout:
            self.controller.on_disconnect()
            self.last_seen = None
