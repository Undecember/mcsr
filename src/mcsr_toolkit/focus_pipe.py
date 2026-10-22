"""Private FIFO transport from GNOME Shell to the user focus controller."""

from __future__ import annotations

import os
import select
import stat
import time
from pathlib import Path

from mcsr_toolkit.focus_watchdog import FocusWatchdog


class FocusPipe:
    def __init__(self, path: Path, watchdog: FocusWatchdog) -> None:
        self.path = path
        self.watchdog = watchdog
        self._fd: int | None = None
        self._inode: int | None = None
        self._buffer = b""
        self.errors: list[str] = []

    def start(self, *, clear_state: bool = True) -> None:
        if self._fd is not None:
            raise RuntimeError("focus FIFO is already open")
        self.path.parent.mkdir(parents=True, exist_ok=True)
        try:
            os.mkfifo(self.path, 0o600)
        except FileExistsError:
            pass
        info = self.path.lstat()
        if not stat.S_ISFIFO(info.st_mode) or info.st_uid != os.getuid() or info.st_mode & 0o077:
            raise ValueError("unsafe focus FIFO")
        fd = os.open(self.path, os.O_RDWR | os.O_NONBLOCK | os.O_CLOEXEC | os.O_NOFOLLOW)
        try:
            opened = os.fstat(fd)
            if not stat.S_ISFIFO(opened.st_mode) or opened.st_ino != info.st_ino:
                raise ValueError("unsafe focus FIFO")
            # The extension retains its writer across restarts; queued events are stale.
            while True:
                try:
                    if not os.read(fd, 8192):
                        break
                except BlockingIOError:
                    break
            self._buffer = b""
            self.watchdog.start(clear_state=clear_state)
        except Exception:
            os.close(fd)
            raise
        self._fd = fd
        self._inode = opened.st_ino

    def run_once(self, timeout: float = 0.1) -> None:
        if self._fd is None:
            raise RuntimeError("focus FIFO is closed")
        readable, _, _ = select.select([self._fd], [], [], timeout)
        if readable:
            self._buffer += os.read(self._fd, 8192)
            if len(self._buffer) > 8192:
                self._buffer = b""
                self.watchdog.controller.on_disconnect()
                self.errors.append("focus event exceeded size limit")
            while b"\n" in self._buffer:
                raw, self._buffer = self._buffer.split(b"\n", 1)
                try:
                    self.watchdog.observe(raw.decode("utf-8"), time.monotonic())
                except (UnicodeError, ValueError) as error:
                    self.watchdog.controller.on_disconnect()
                    self.errors.append(str(error))
        self.watchdog.tick(time.monotonic())

    def close(self, *, unlink: bool = True) -> None:
        if self._fd is None:
            return
        try:
            self.watchdog.controller.on_disconnect()
        finally:
            os.close(self._fd)
            self._fd = None
            if unlink:
                try:
                    if self.path.lstat().st_ino == self._inode:
                        self.path.unlink()
                except FileNotFoundError:
                    pass
            self._inode = None
