#!/usr/bin/python3 -I
"""Root-owned, fixed-target installer for the toolkit's restricted keyd fragment.

The deployment copy of this file must be root-owned and must not import code from
the user's writable project checkout when executed through pkexec.
"""

from __future__ import annotations

import argparse
import base64
import json
import os
import re
import stat
import subprocess
import tempfile
import time
import uuid
from collections.abc import Callable
from pathlib import Path
from typing import Any

DEVICE_CONFIG = "# MCSR Toolkit managed\n[ids]\n*\n\ninclude mcsr-toolkit\n"
DEVICE_NAME = "mcsr-toolkit.conf"
FRAGMENT_NAME = "mcsr-toolkit"
SYSTEMCTL = "/usr/bin/systemctl"
SERVICE = "keyd.service"


def _run_systemctl(command: list[str], timeout: float) -> subprocess.CompletedProcess[str]:
    return subprocess.run(command, capture_output=True, text=True, timeout=timeout, check=False)


def restart_keyd_service(
    runner: Callable[[list[str], float], subprocess.CompletedProcess[str]],
    clock: Callable[[], float],
    sleep: Callable[[float], None],
) -> None:
    try:
        result = runner([SYSTEMCTL, "restart", SERVICE], 5)
        if result.returncode != 0:
            raise RuntimeError(f"keyd restart failed: {result.stderr.strip()}")

        deadline = clock() + 5
        stable_since: float | None = None
        last_healthy_at: float | None = None
        while True:
            result = runner([SYSTEMCTL, "is-active", "--quiet", SERVICE], 5)
            now = clock()
            if stable_since is None and now > deadline:
                raise RuntimeError("keyd health check timed out")
            if result.returncode == 0:
                if stable_since is None:
                    stable_since = now
                elif last_healthy_at is not None and now - last_healthy_at > 0.2:
                    # Leave room for scheduling jitter while rejecting missed polls.
                    raise RuntimeError("keyd health check gap during stability window")
                last_healthy_at = now
                if now - stable_since >= 0.5:
                    return
            else:
                if stable_since is not None:
                    raise RuntimeError("keyd health check failed during stability window")
            if stable_since is None and now >= deadline:
                raise RuntimeError("keyd health check timed out")
            remaining = (deadline if stable_since is None else stable_since + 0.5) - now
            sleep(min(0.1, remaining))
    except Exception as failure:
        try:
            stopped = runner([SYSTEMCTL, "stop", SERVICE], 5)
        except Exception as stop_failure:
            raise RuntimeError(f"keyd stop failed; input state unsafe: {stop_failure}") from failure
        if stopped.returncode != 0:
            raise RuntimeError(
                f"keyd stop failed; input state unsafe: {stopped.stderr.strip()}"
            ) from failure
        raise


def _validate_fragment(fragment: str, valid_keys: frozenset[str]) -> None:
    if len(fragment.encode("utf-8")) > 16384:
        raise ValueError("invalid keyd fragment size")
    lines = fragment.splitlines()
    if not lines or lines[0] != "[mcsr_chat]":
        raise ValueError("invalid keyd fragment section")
    seen: set[str] = set()
    for line in lines[1:]:
        key, separator, action = line.partition(" = ")
        if (
            not separator
            or not re.fullmatch(r"[a-z0-9][a-z0-9_]*", key)
            or key not in valid_keys
            or key in seen
        ):
            raise ValueError(f"invalid keyd fragment binding: {line}")
        if action not in {key, f"clearm({key})", "clearm(enter)", "clearm(esc)"}:
            raise ValueError(f"invalid keyd fragment action: {line}")
        seen.add(key)


def _safe_directory(path: Path) -> None:
    # Reject symlinks in ancestors as well as the final component.
    for directory in reversed((path, *path.parents)):
        info = directory.lstat()
        if not stat.S_ISDIR(info.st_mode):
            raise ValueError("unsafe keyd directory")
        if os.geteuid() == 0 and (info.st_uid != 0 or info.st_mode & 0o022):
            raise ValueError("unsafe keyd directory ownership or permissions")


def _atomic_text(path: Path, content: str, mode: int = 0o644) -> None:
    fd, name = tempfile.mkstemp(prefix=f".{path.name}.", dir=path.parent)
    try:
        with os.fdopen(fd, "w", encoding="utf-8", newline="") as stream:
            os.fchmod(stream.fileno(), mode)
            stream.write(content)
            stream.flush()
            os.fsync(stream.fileno())
        os.replace(name, path)
    finally:
        if os.path.exists(name):
            os.unlink(name)


class KeydInstaller:
    def __init__(
        self,
        config_dir: Path,
        backup_root: Path,
        restart_keyd: Callable[[], None],
        valid_keys: frozenset[str],
    ) -> None:
        self.config_dir = config_dir
        self.backup_root = backup_root
        self.restart_keyd = restart_keyd
        self.valid_keys = valid_keys

    def _assert_directory(self) -> None:
        _safe_directory(self.config_dir)

    def _snapshot(self, path: Path) -> dict[str, Any] | None:
        try:
            info = path.lstat()
        except FileNotFoundError:
            return None
        if stat.S_ISLNK(info.st_mode):
            raise ValueError(f"keyd target is a symlink: {path.name}")
        if not stat.S_ISREG(info.st_mode):
            raise ValueError(f"keyd target is not a regular file: {path.name}")
        return {"text": path.read_text(encoding="utf-8"), "mode": stat.S_IMODE(info.st_mode)}

    def _restore_snapshot(self, path: Path, snapshot: dict[str, Any] | None) -> None:
        self._snapshot(path)  # Refuse a swapped symlink or special file.
        if snapshot is None:
            path.unlink(missing_ok=True)
        else:
            _atomic_text(path, snapshot["text"], snapshot["mode"])

    def _check_ownership(self) -> tuple[dict[str, Any] | None, dict[str, Any] | None]:
        self._assert_directory()
        for path in self.config_dir.glob("*.conf"):
            if path.name != DEVICE_NAME:
                raise ValueError(f"existing keyd config requires manual integration: {path.name}")
        device = self._snapshot(self.config_dir / DEVICE_NAME)
        fragment = self._snapshot(self.config_dir / FRAGMENT_NAME)
        if device is not None and device["text"] != DEVICE_CONFIG:
            raise ValueError("unrecognized existing toolkit device config")
        if fragment is not None:
            _validate_fragment(fragment["text"], self.valid_keys)
        if (device is None) != (fragment is None):
            raise ValueError("incomplete existing toolkit config")
        return device, fragment

    def _backup(self, device: dict[str, Any] | None, fragment: dict[str, Any] | None) -> str:
        missing: list[Path] = []
        existing = self.backup_root
        while not existing.exists() and not existing.is_symlink():
            missing.append(existing)
            existing = existing.parent
        _safe_directory(existing)
        for directory in reversed(missing):
            directory.mkdir(mode=0o700)
            _safe_directory(directory)
        _safe_directory(self.backup_root)
        backup_id = uuid.uuid4().hex
        directory = self.backup_root / backup_id
        directory.mkdir(mode=0o700)
        _atomic_text(
            directory / "manifest.json",
            json.dumps({"device": device, "fragment": fragment}, sort_keys=True),
            0o600,
        )
        return backup_id

    def install(self, fragment: str) -> str:
        _validate_fragment(fragment, self.valid_keys)
        device_before, fragment_before = self._check_ownership()
        backup_id = self._backup(device_before, fragment_before)
        try:
            _atomic_text(self.config_dir / FRAGMENT_NAME, fragment)
            _atomic_text(self.config_dir / DEVICE_NAME, DEVICE_CONFIG)
            self.restart_keyd()
        except Exception as failure:
            try:
                self._restore_snapshot(self.config_dir / DEVICE_NAME, device_before)
                self._restore_snapshot(self.config_dir / FRAGMENT_NAME, fragment_before)
                self.restart_keyd()
            except Exception as rollback_failure:
                raise ExceptionGroup(
                    "keyd install and rollback failed", [failure, rollback_failure]
                ) from failure
            raise
        return backup_id

    def restore(self, backup_id: str) -> None:
        if not re.fullmatch(r"[a-f0-9]{32}", backup_id):
            raise ValueError("unknown keyd backup")
        _safe_directory(self.backup_root)
        directory = self.backup_root / backup_id
        if directory.is_symlink() or not directory.is_dir():
            raise ValueError("unknown keyd backup")
        manifest_path = directory / "manifest.json"
        if manifest_path.is_symlink() or not manifest_path.is_file():
            raise ValueError("unknown keyd backup")
        manifest = json.loads(manifest_path.read_text(encoding="utf-8"))
        if not isinstance(manifest, dict) or set(manifest) != {"device", "fragment"}:
            raise ValueError("invalid keyd backup")
        for name, snapshot in manifest.items():
            if snapshot is None:
                continue
            if (
                not isinstance(snapshot, dict)
                or set(snapshot) != {"text", "mode"}
                or not isinstance(snapshot["text"], str)
                or type(snapshot["mode"]) is not int
                or not 0 <= snapshot["mode"] <= 0o777
            ):
                raise ValueError("invalid keyd backup snapshot")
            if name == "device" and snapshot["text"] != DEVICE_CONFIG:
                raise ValueError("invalid keyd backup device config")
            if name == "fragment":
                _validate_fragment(snapshot["text"], self.valid_keys)
        if (manifest["device"] is None) != (manifest["fragment"] is None):
            raise ValueError("incomplete keyd backup")
        device_current, fragment_current = self._check_ownership()
        try:
            self._restore_snapshot(self.config_dir / DEVICE_NAME, manifest["device"])
            self._restore_snapshot(self.config_dir / FRAGMENT_NAME, manifest["fragment"])
            self.restart_keyd()
        except Exception as failure:
            try:
                self._restore_snapshot(self.config_dir / DEVICE_NAME, device_current)
                self._restore_snapshot(self.config_dir / FRAGMENT_NAME, fragment_current)
                self.restart_keyd()
            except Exception as rollback_failure:
                raise ExceptionGroup(
                    "keyd restore and rollback failed", [failure, rollback_failure]
                ) from failure
            raise


def main(argv: list[str] | None = None) -> int:
    if os.geteuid() != 0:
        raise PermissionError("the keyd installer must run as root")
    parser = argparse.ArgumentParser(prog="mcsr-toolkit-keyd")
    subcommands = parser.add_subparsers(dest="command", required=True)
    subcommands.add_parser("clear")
    subcommands.add_parser("install").add_argument("fragment_base64")
    subcommands.add_parser("restore").add_argument("backup_id")
    arguments = parser.parse_args(argv)
    if arguments.command == "clear":
        restart_keyd_service(_run_systemctl, time.monotonic, time.sleep)
        return 0
    binary = next(
        (
            str(path)
            for path in (Path("/usr/bin/keyd.rvaiya"), Path("/usr/bin/keyd"))
            if path.is_file()
        ),
        None,
    )
    if binary is None:
        raise RuntimeError("keyd is not installed")
    result = subprocess.run([binary, "list-keys"], capture_output=True, text=True, timeout=5)
    if result.returncode != 0:
        raise RuntimeError("keyd list-keys failed")
    installer = KeydInstaller(
        Path("/etc/keyd"),
        Path("/var/lib/mcsr-toolkit/keyd"),
        lambda: restart_keyd_service(_run_systemctl, time.monotonic, time.sleep),
        frozenset(result.stdout.splitlines()),
    )
    if arguments.command == "install":
        if len(arguments.fragment_base64) > 24000:
            raise ValueError("invalid keyd fragment size")
        fragment = base64.b64decode(arguments.fragment_base64, validate=True).decode("utf-8")
        backup_id = installer.install(fragment)
        print(json.dumps({"backup_id": backup_id}))
    else:
        installer.restore(arguments.backup_id)
        print(json.dumps({"restored": arguments.backup_id}))
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
