#!/usr/bin/python3 -I
"""Standalone root helper for the reviewed maccel Q32 driver.

Only SENS_MULT is written outside the private runtime directory. HelperPaths is
an internal test seam; the installed entry point always constructs fixed paths.
"""

from __future__ import annotations

import fcntl
import os
import re
import secrets
import stat
import sys
import time
from collections.abc import Iterator
from contextlib import contextmanager
from dataclasses import dataclass
from decimal import ROUND_HALF_UP, Decimal, InvalidOperation, localcontext
from pathlib import Path
from typing import TextIO

SUPPORTED_SRCVERSION = "4AFA0F5B3EBD2F98EE54E56"
MAX_RAW = (1 << 63) - 1
SCALE = 1 << 32
LOCK_TIMEOUT = 3.0
_TOKEN = re.compile(r"[0-9a-f]{64}\n?\Z")
_RAW = re.compile(r"[1-9][0-9]{0,18}\n?\Z")


@dataclass(frozen=True)
class HelperPaths:
    parameter: Path = Path("/sys/module/maccel/parameters/SENS_MULT")
    runtime: Path = Path("/run/mcsr-toolkit-maccel")
    fingerprint: Path = Path("/sys/module/maccel/srcversion")
    owner_uid: int = 0
    anchor: Path = Path("/")


def to_raw(value: str) -> int:
    try:
        with localcontext() as context:
            context.prec = 80
            decimal = Decimal(value)
            if not decimal.is_finite() or decimal <= 0 or decimal > Decimal(MAX_RAW) / SCALE:
                raise ValueError("invalid fixed-point multiplier")
            raw = int((decimal * SCALE).to_integral_value(rounding=ROUND_HALF_UP))
    except InvalidOperation as error:
        raise ValueError("invalid fixed-point multiplier") from error
    if not 0 < raw <= MAX_RAW:
        raise ValueError("invalid fixed-point multiplier")
    return raw


def _raw(text: str) -> int:
    if not _RAW.fullmatch(text) or not 0 < int(text) <= MAX_RAW:
        raise ValueError("unsupported signed Q32 value")
    return int(text)


def _parameter_raw(text: str) -> int:
    # The supported driver retains a submitted newline and appends its own.
    # Accept that legacy state for recovery without relaxing record grammar.
    if text.endswith("\n\n"):
        text = text[:-1]
    return _raw(text)


def _token(text: str) -> str:
    if not _TOKEN.fullmatch(text):
        raise ValueError("missing or malformed generation")
    return text.rstrip("\n")


def _check_directory(fd: int, paths: HelperPaths, *, private: bool = False) -> None:
    info = os.fstat(fd)
    if (
        not stat.S_ISDIR(info.st_mode)
        or info.st_uid != paths.owner_uid
        or (stat.S_IMODE(info.st_mode) != 0o700 if private else bool(info.st_mode & 0o022))
    ):
        raise ValueError("unsafe directory ownership or mode")


@contextmanager
def _directory(path: Path, paths: HelperPaths) -> Iterator[int]:
    # Walk with directory descriptors: no symlink or writable ancestor is trusted.
    relative = path.relative_to(paths.anchor)
    if ".." in relative.parts:
        raise ValueError("unexpected path")
    fd = os.open(paths.anchor, os.O_RDONLY | os.O_DIRECTORY | os.O_NOFOLLOW)
    try:
        _check_directory(fd, paths)
        for component in relative.parts:
            child = os.open(component, os.O_RDONLY | os.O_DIRECTORY | os.O_NOFOLLOW, dir_fd=fd)
            os.close(fd)
            fd = child
            _check_directory(fd, paths)
        yield fd
    finally:
        os.close(fd)


def _open_file(
    parent: int, name: str, paths: HelperPaths, kind: str, flags: int = os.O_RDONLY
) -> int:
    fd = os.open(name, flags | os.O_NOFOLLOW | os.O_NONBLOCK, 0o600, dir_fd=parent)
    try:
        info = os.fstat(fd)
        mode = stat.S_IMODE(info.st_mode)
        allowed = (
            {0o644, 0o664}
            if kind == "parameter"
            else {0o444, 0o644}
            if kind == "fingerprint"
            else {0o600}
        )
        if (
            not stat.S_ISREG(info.st_mode)
            or info.st_uid != paths.owner_uid
            or info.st_nlink != 1
            or mode not in allowed
        ):
            raise ValueError("unsafe file ownership, mode, or type")
        return fd
    except BaseException:
        os.close(fd)
        raise


def _read(fd: int) -> str:
    os.lseek(fd, 0, os.SEEK_SET)
    data = os.read(fd, 257)
    if len(data) > 256:
        raise ValueError("oversized record")
    return data.decode("ascii")


def _record(parent: int, name: str, paths: HelperPaths) -> str | None:
    try:
        fd = _open_file(parent, name, paths, "record")
    except FileNotFoundError:
        return None
    try:
        return _read(fd)
    finally:
        os.close(fd)


def _store(parent: int, name: str, text: str, paths: HelperPaths) -> None:
    # Exclusive creation avoids following or truncating an existing unexpected file.
    fd = _open_file(parent, name, paths, "record", os.O_WRONLY | os.O_CREAT | os.O_EXCL)
    try:
        data = text.encode("ascii")
        if os.write(fd, data) != len(data):
            raise OSError("partial record write")
        os.fsync(fd)
    finally:
        os.close(fd)
    os.fsync(parent)


def _revoke(parent: int) -> None:
    try:
        os.unlink("generation", dir_fd=parent)
    except FileNotFoundError:
        pass
    os.fsync(parent)


def _write_parameter(fd: int, raw: int) -> None:
    os.lseek(fd, 0, os.SEEK_SET)
    data = str(raw).encode("ascii")
    if os.write(fd, data) != len(data):
        raise OSError("partial parameter write")
    if _parameter_raw(_read(fd)) != raw:
        raise OSError("parameter readback mismatch")


@contextmanager
def _locked(runtime: int, paths: HelperPaths) -> Iterator[None]:
    fd = _open_file(runtime, "lock", paths, "record", os.O_RDWR | os.O_CREAT)
    try:
        deadline = time.monotonic() + LOCK_TIMEOUT
        while True:
            try:
                fcntl.flock(fd, fcntl.LOCK_EX | fcntl.LOCK_NB)
                break
            except BlockingIOError:
                if time.monotonic() >= deadline:
                    raise TimeoutError("helper lock timed out") from None
                time.sleep(0.02)
        yield
    finally:
        os.close(fd)


def _validated_records(runtime: int, paths: HelperPaths) -> tuple[str | None, int | None]:
    if set(os.listdir(runtime)) - {"lock", "generation", "baseline"}:
        raise ValueError("unexpected runtime entry")
    generation = _record(runtime, "generation", paths)
    if generation is not None:
        generation = _token(generation)
    baseline_text = _record(runtime, "baseline", paths)
    baseline = _raw(baseline_text) if baseline_text is not None else None
    return generation, baseline


def _transition(
    command: str, token: str | None, runtime: int, parameter: int, paths: HelperPaths
) -> str:
    generation, baseline = _validated_records(runtime, paths)
    current = _parameter_raw(_read(parameter))
    if command == "eye":
        if token is None or generation is None or not secrets.compare_digest(token, generation):
            raise ValueError("stale generation")
        try:
            if baseline is None:
                _store(runtime, "baseline", f"{current}\n", paths)
            else:
                # A prior helper may have died between creation and fsync.
                baseline_fd = _open_file(runtime, "baseline", paths, "record")
                try:
                    os.fsync(baseline_fd)
                finally:
                    os.close(baseline_fd)
                os.fsync(runtime)
            _write_parameter(parameter, to_raw("0.02083"))
        except BaseException:
            # A failed write leaves the baseline and requires a successful restore.
            _revoke(runtime)
            raise
        with localcontext() as context:
            context.prec = 80
            return str(Decimal(to_raw("0.02083")) / SCALE)
    # Revoke the old token before attempting recovery, including failure paths.
    _revoke(runtime)
    if baseline is not None:
        _write_parameter(parameter, baseline)
        os.unlink("baseline", dir_fd=runtime)
        os.fsync(runtime)
    new_generation = secrets.token_hex(32)
    _store(runtime, "generation", new_generation + "\n", paths)
    return new_generation


@contextmanager
def _parameter(paths: HelperPaths) -> Iterator[int]:
    with _directory(paths.fingerprint.parent, paths) as driver_parent:
        fingerprint = _open_file(driver_parent, paths.fingerprint.name, paths, "fingerprint")
        try:
            if _read(fingerprint) != SUPPORTED_SRCVERSION + "\n":
                raise ValueError("unsupported maccel driver fingerprint")
        finally:
            os.close(fingerprint)
    with _directory(paths.parameter.parent, paths) as parameter_parent:
        parameter = _open_file(
            parameter_parent, paths.parameter.name, paths, "parameter", os.O_RDWR
        )
        try:
            _parameter_raw(_read(parameter))
            yield parameter
        finally:
            os.close(parameter)


@contextmanager
def _runtime(paths: HelperPaths, *, create: bool) -> Iterator[int | None]:
    with _directory(paths.runtime.parent, paths) as runtime_parent:
        if create:
            try:
                os.mkdir(paths.runtime.name, mode=0o700, dir_fd=runtime_parent)
                os.fsync(runtime_parent)
            except FileExistsError:
                pass
        try:
            runtime = os.open(
                paths.runtime.name,
                os.O_RDONLY | os.O_DIRECTORY | os.O_NOFOLLOW,
                dir_fd=runtime_parent,
            )
        except FileNotFoundError:
            if create:
                raise
            yield None
            return
        try:
            _check_directory(runtime, paths, private=True)
            yield runtime
        finally:
            os.close(runtime)


def run(command: str, stream: TextIO, paths: HelperPaths) -> str:
    if command not in {"eye", "restore"}:
        raise ValueError("expected eye or restore")
    # Read before lock acquisition, so a queued process cannot adopt a new token.
    token = _token(stream.read(66)) if command == "eye" else None
    with _runtime(paths, create=False) as existing_runtime:
        if existing_runtime is not None:
            with _locked(existing_runtime, paths):
                if command == "restore":
                    # Existing generations must be invalidated even when module
                    # preflight fails transiently. Unsafe evidence is preserved.
                    _validated_records(existing_runtime, paths)
                    _revoke(existing_runtime)
                with _parameter(paths) as parameter:
                    return _transition(command, token, existing_runtime, parameter, paths)
        # Do not create records for a missing or unsupported module. There is no
        # existing generation to revoke when the private directory is absent.
        with _parameter(paths) as parameter:
            with _runtime(paths, create=True) as runtime:
                assert runtime is not None
                with _locked(runtime, paths):
                    return _transition(command, token, runtime, parameter, paths)


def main(argv: list[str] | None = None) -> int:
    arguments = sys.argv[1:] if argv is None else argv
    if len(arguments) != 1 or arguments[0] not in {"eye", "restore"}:
        print("usage: mcsr-toolkit-maccel {eye|restore}", file=sys.stderr)
        return 2
    if os.geteuid() != 0:
        print("maccel helper must run as root", file=sys.stderr)
        return 1
    try:
        result = run(arguments[0], sys.stdin, HelperPaths())
    except (OSError, ValueError, InvalidOperation) as error:
        print(f"maccel helper: {error}", file=sys.stderr)
        return 1
    print(result)
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
