"""Unprivileged client for the fixed, reviewed maccel recovery helper."""

from __future__ import annotations

import math
import re
import shutil
import stat
import subprocess
from collections.abc import Callable
from dataclasses import dataclass
from pathlib import Path

_HELPER = "/usr/local/libexec/mcsr-toolkit-maccel"
_SUDO = "/usr/bin/sudo"
Runner = Callable[[list[str], str | None], subprocess.CompletedProcess[str]]


class SensitivityError(RuntimeError):
    """A sensitivity operation could not be confirmed."""


def _run(args: list[str], token: str | None) -> subprocess.CompletedProcess[str]:
    return subprocess.run(args, input=token, capture_output=True, text=True, timeout=3, check=False)


def _trusted_helper() -> None:
    helper = Path(_HELPER)
    for path in [*reversed(helper.parents), helper]:
        info = path.lstat()
        expected_type = stat.S_ISREG if path == helper else stat.S_ISDIR
        if info.st_uid != 0 or info.st_mode & 0o022 or not expected_type(info.st_mode):
            raise SensitivityError(f"untrusted helper path: {path}")
        if not info.st_mode & stat.S_IXUSR:
            raise SensitivityError(f"helper path is not executable: {path}")


@dataclass(frozen=True)
class SensitivityStatus:
    available: bool
    usable: bool
    detail: str
    recovery_pending: bool


class SensitivityAdapter:
    """Preserve recovery uncertainty until the helper confirms exact restoration.

    The helper owns baseline capture and raw readback. CLI reads are only an
    availability observation and never become a guessed restoration value.
    """

    def __init__(self, *, runner: Runner = _run, maccel_binary: str | None = None) -> None:
        self._runner = runner
        self._maccel = maccel_binary if maccel_binary is not None else shutil.which("maccel")
        self._generation: str | None = None
        self.recovery_pending = False
        self.eye_active = False
        self._detail = "startup restore required"

    @property
    def needs_restore(self) -> bool:
        return self.eye_active or self.recovery_pending

    def _call(self, args: list[str], token: str | None = None) -> str:
        try:
            result = self._runner(args, token)
            if result.returncode:
                raise SensitivityError(result.stderr.strip() or "helper command failed")
            return result.stdout
        except (OSError, subprocess.SubprocessError) as error:
            raise SensitivityError(str(error)) from error

    def _check_eye_execution(self) -> None:
        # The reviewed helper rejects empty stdin before lock or sysfs access.
        # This proves eye authorization without supplying a usable generation.
        try:
            result = self._runner([_SUDO, "-n", _HELPER, "eye"], "")
        except (OSError, subprocess.SubprocessError) as error:
            raise SensitivityError(f"eye permission probe failed: {error}") from error
        if (
            result.returncode != 1
            or result.stdout != ""
            or result.stderr != "maccel helper: missing or malformed generation\n"
        ):
            raise SensitivityError(
                "eye permission unconfirmed: "
                + (result.stderr.strip() or "unexpected helper probe response")
            )

    def preflight(self) -> SensitivityStatus:
        available = False
        try:
            if self._maccel is None:
                raise SensitivityError("maccel is not installed")
            value = float(self._call([self._maccel, "get", "param", "sens-mult"]).strip())
            if not math.isfinite(value) or value <= 0:
                raise SensitivityError("invalid maccel multiplier")
            available = True
            _trusted_helper()
            for command in ("eye", "restore"):
                self._call([_SUDO, "-n", "-l", "--", _HELPER, command])
            if self.recovery_pending or self._generation is None:
                raise SensitivityError(self._detail)
            self._check_eye_execution()
        except (SensitivityError, OSError, ValueError) as error:
            return SensitivityStatus(available, False, str(error), self.recovery_pending)
        return SensitivityStatus(True, True, "eye zoom available", False)

    def startup_restore(self) -> None:
        self.restore()

    def restore(self) -> None:
        # Invalidate locally before spawning: a failure must never reuse a token.
        self._generation = None
        self.eye_active = False
        self.recovery_pending = True
        try:
            _trusted_helper()
            token = self._call([_SUDO, "-n", _HELPER, "restore"])
            if re.fullmatch(r"[0-9a-f]{64}\n?", token) is None:
                raise SensitivityError("invalid restore generation")
            self._generation = token.rstrip("\n")
        except (SensitivityError, OSError) as error:
            self._detail = f"sensitivity recovery pending: {error}"
            raise SensitivityError(self._detail) from error
        self.recovery_pending = False
        self._detail = "eye zoom available"

    def eye(self) -> None:
        if self.recovery_pending or self._generation is None:
            raise SensitivityError(self._detail)
        status = self.preflight()
        if not status.usable:
            raise SensitivityError(status.detail)
        self.recovery_pending = True
        self.eye_active = False
        try:
            self._call([_SUDO, "-n", _HELPER, "eye"], self._generation + "\n")
        except SensitivityError as error:
            self._generation = None
            self._detail = f"sensitivity recovery pending: {error}"
            raise SensitivityError(self._detail) from error
        self.recovery_pending = False
        self.eye_active = True
