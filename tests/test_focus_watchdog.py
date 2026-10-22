from __future__ import annotations

import pytest

from mcsr_toolkit.focus import FocusController
from mcsr_toolkit.focus_watchdog import FocusWatchdog


class FakeKeyd:
    def __init__(self) -> None:
        self.calls: list[str] = []

    def activate_game(self) -> None:
        self.calls.append("activate")

    def clear_state(self) -> None:
        self.calls.append("clear")


def test_startup_clears_stale_mapping_and_heartbeat_keeps_game_active() -> None:
    keyd = FakeKeyd()
    watchdog = FocusWatchdog(FocusController(keyd), timeout=2)
    watchdog.start()
    watchdog.observe('{"class":"minecraft","title":"Game","modal":false}', now=10)
    watchdog.tick(now=11.9)
    assert keyd.calls == ["clear", "activate"]


def test_missing_extension_heartbeat_clears_mapping() -> None:
    keyd = FakeKeyd()
    watchdog = FocusWatchdog(FocusController(keyd), timeout=2)
    watchdog.start()
    watchdog.observe('{"class":"minecraft","title":"Game","modal":false}', now=10)
    watchdog.tick(now=12.1)
    assert keyd.calls == ["clear", "activate", "clear"]


def test_modal_message_clears_even_when_underlying_class_is_minecraft() -> None:
    keyd = FakeKeyd()
    watchdog = FocusWatchdog(FocusController(keyd), timeout=2)
    watchdog.start()
    watchdog.observe('{"class":"minecraft","title":"Game","modal":false}', now=10)
    watchdog.observe('{"class":"minecraft","title":"Game","modal":true}', now=11)
    assert keyd.calls == ["clear", "activate", "clear"]


def test_malformed_event_fails_closed() -> None:
    keyd = FakeKeyd()
    watchdog = FocusWatchdog(FocusController(keyd), timeout=2)
    watchdog.start()
    watchdog.observe('{"class":"minecraft","title":"Game","modal":false}', now=10)
    with pytest.raises(ValueError, match="focus event"):
        watchdog.observe('{"class":"minecraft","modal":"no"}', now=11)
    assert keyd.calls == ["clear", "activate", "clear"]
