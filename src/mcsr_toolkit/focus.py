"""Keep keyd's dynamic bindings aligned with GNOME's effective input focus."""

from __future__ import annotations

from dataclasses import dataclass
from threading import RLock
from typing import Protocol


@dataclass(frozen=True)
class FocusEvent:
    window_class: str
    title: str
    modal: bool = False


def is_minecraft_window(event: FocusEvent) -> bool:
    """Match Minecraft application windows while excluding browser page titles."""
    return event.window_class.casefold().startswith("minecraft")


class FocusKeyd(Protocol):
    def activate_game(self) -> None: ...

    def clear_state(self) -> None: ...


class FocusController:
    def __init__(self, keyd: FocusKeyd, lock: RLock | None = None) -> None:
        self.keyd = keyd
        self._lock = lock or RLock()
        self.game_active = False

    def on_focus(self, event: FocusEvent) -> None:
        with self._lock:
            self._on_focus(event)

    def _on_focus(self, event: FocusEvent) -> None:
        game = not event.modal and is_minecraft_window(event)
        if game and not self.game_active:
            self.keyd.activate_game()
            self.game_active = True
        elif not game and self.game_active:
            # A daemon restart is required: `bind reset` leaves toggled layers active.
            self.keyd.clear_state()
            self.game_active = False

    def on_disconnect(self) -> None:
        with self._lock:
            if self.game_active:
                self.keyd.clear_state()
                self.game_active = False
