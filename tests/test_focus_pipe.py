from __future__ import annotations

import os
from pathlib import Path

import pytest

from mcsr_toolkit.focus import FocusController
from mcsr_toolkit.focus_pipe import FocusPipe
from mcsr_toolkit.focus_watchdog import FocusWatchdog


class FakeKeyd:
    def __init__(self) -> None:
        self.calls: list[str] = []

    def activate_game(self) -> None:
        self.calls.append("activate")

    def clear_state(self) -> None:
        self.calls.append("clear")


def test_focus_fifo_delivers_event_and_cleans_up(tmp_path: Path) -> None:
    keyd = FakeKeyd()
    path = tmp_path / "focus.fifo"
    pipe = FocusPipe(path, FocusWatchdog(FocusController(keyd)))
    pipe.start()
    assert path.exists()
    assert path.stat().st_mode & 0o777 == 0o600
    writer = os.open(path, os.O_WRONLY | os.O_NONBLOCK)
    try:
        os.write(writer, b'{"class":"minecraft","title":"Game","modal":false}\n')
        pipe.run_once(timeout=0)
        assert keyd.calls == ["clear", "activate"]
    finally:
        os.close(writer)
        pipe.close()
    assert keyd.calls == ["clear", "activate", "clear"]
    assert not path.exists()


def test_rejects_existing_regular_file(tmp_path: Path) -> None:
    path = tmp_path / "focus.fifo"
    path.write_text("not a fifo")
    pipe = FocusPipe(path, FocusWatchdog(FocusController(FakeKeyd())))
    with pytest.raises(ValueError, match="FIFO"):
        pipe.start()
    assert path.read_text() == "not a fifo"


def test_service_restart_can_reopen_existing_fifo_with_extension_writer(tmp_path: Path) -> None:
    path = tmp_path / "focus.fifo"
    first = FocusPipe(path, FocusWatchdog(FocusController(FakeKeyd())))
    first.start()
    writer = os.open(path, os.O_RDWR | os.O_NONBLOCK)
    try:
        first.close(unlink=False)
        assert path.is_fifo()
        second_keyd = FakeKeyd()
        second = FocusPipe(path, FocusWatchdog(FocusController(second_keyd)))
        second.start()
        os.write(writer, b'{"class":"minecraft","title":"Game","modal":false}\n')
        second.run_once(timeout=0)
        assert second_keyd.calls == ["clear", "activate"]
        second.close(unlink=False)
    finally:
        os.close(writer)
        path.unlink(missing_ok=True)


def test_start_discards_events_queued_during_service_restart(tmp_path: Path) -> None:
    path = tmp_path / 'focus.fifo'
    first = FocusPipe(path, FocusWatchdog(FocusController(FakeKeyd())))
    first.start()
    writer = os.open(path, os.O_RDWR | os.O_NONBLOCK)
    try:
        first.close(unlink=False)
        os.write(writer, b'{"class":"minecraft","title":"Old game","modal":false}\n')
        keyd = FakeKeyd()
        second = FocusPipe(path, FocusWatchdog(FocusController(keyd)))
        second.start()
        second.run_once(timeout=0)
        assert keyd.calls == ['clear']
        assert second.watchdog.last_seen is None
        second.close()
    finally:
        os.close(writer)
