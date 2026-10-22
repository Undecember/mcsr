from __future__ import annotations

import pytest

from mcsr_toolkit.focus import FocusController, FocusEvent, is_minecraft_window


class FakeKeyd:
    def __init__(self) -> None:
        self.calls: list[str] = []
        self.fail_clear = False

    def activate_game(self) -> None:
        self.calls.append("activate")

    def clear_state(self) -> None:
        self.calls.append("clear")
        if self.fail_clear:
            raise RuntimeError("reload failed")


MINECRAFT = FocusEvent("Minecraft* 1.16.1 - MCSR Ranked", "Minecraft* 1.16.1 - MCSR Ranked")
TERMINAL = FocusEvent("org.gnome.Ptyxis", "Terminal")


def test_matches_minecraft_class_not_browser_title() -> None:
    assert is_minecraft_window(MINECRAFT)
    assert is_minecraft_window(FocusEvent("minecraft", "Another instance"))
    assert not is_minecraft_window(FocusEvent("org.mozilla.firefox", "Minecraft wiki"))


def test_focus_entry_exit_and_reentry() -> None:
    keyd = FakeKeyd()
    controller = FocusController(keyd)
    controller.on_focus(TERMINAL)
    controller.on_focus(MINECRAFT)
    controller.on_focus(MINECRAFT)
    controller.on_focus(TERMINAL)
    controller.on_focus(MINECRAFT)
    assert keyd.calls == ["activate", "clear", "activate"]
    assert controller.game_active


def test_shell_modal_clears_before_password_and_restores_after_close() -> None:
    keyd = FakeKeyd()
    controller = FocusController(keyd)
    controller.on_focus(MINECRAFT)
    controller.on_focus(FocusEvent(MINECRAFT.window_class, MINECRAFT.title, modal=True))
    assert not controller.game_active
    controller.on_focus(MINECRAFT)
    assert keyd.calls == ["activate", "clear", "activate"]


def test_disconnect_clears_active_mapping() -> None:
    keyd = FakeKeyd()
    controller = FocusController(keyd)
    controller.on_focus(MINECRAFT)
    controller.on_disconnect()
    assert keyd.calls == ["activate", "clear"]
    assert not controller.game_active


def test_clear_failure_keeps_unsafely_active_state_for_retry() -> None:
    keyd = FakeKeyd()
    controller = FocusController(keyd)
    controller.on_focus(MINECRAFT)
    keyd.fail_clear = True
    with pytest.raises(RuntimeError, match="reload failed"):
        controller.on_focus(TERMINAL)
    assert controller.game_active
    keyd.fail_clear = False
    controller.on_focus(TERMINAL)
    assert not controller.game_active
    assert keyd.calls == ["activate", "clear", "clear"]
