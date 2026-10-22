"""Generate restricted keyd rules for a focus-managed Minecraft profile."""

from __future__ import annotations

import base64
import json
import os
import re
import shutil
import stat
import subprocess
import tempfile
from collections.abc import Callable
from pathlib import Path

from mcsr_toolkit.models import Profile, supported_keys

MODIFIERS = {
    "leftcontrol": "C",
    "rightcontrol": "C",
    "leftshift": "S",
    "rightshift": "S",
    "leftalt": "A",
    "rightalt": "G",
    "leftmeta": "M",
    "rightmeta": "M",
}


def _shortcut_action(shortcut: str) -> str:
    parts = shortcut.split("+")
    if not parts or parts[-1] in MODIFIERS:
        raise ValueError(f"invalid Ninjabrain shortcut: {shortcut}")
    modifier_parts = parts[:-1]
    if any(part not in MODIFIERS for part in modifier_parts):
        raise ValueError(f"invalid Ninjabrain shortcut: {shortcut}")
    modifiers = [MODIFIERS[part] for part in modifier_parts]
    if len(modifiers) != len(set(modifiers)):
        raise ValueError(f"duplicate shortcut modifier: {shortcut}")
    return "-".join([*modifiers, parts[-1]])


def game_bindings(profile: Profile) -> list[str]:
    """Bindings applied through `keyd bind reset` only for effective game focus."""
    entries = [f"{source} = {target}" for source, target in profile.crafting.items()]
    entries.extend(
        f"{source} = {_shortcut_action(shortcut)}"
        for source, shortcut in profile.ninjabrain.items()
    )
    entries.extend(
        [
            f"{profile.chat.open_chat} = togglem(mcsr_chat, {profile.chat.open_chat})",
            f"{profile.chat.open_command} = togglem(mcsr_chat, {profile.chat.open_command})",
        ]
    )
    return entries


def render_keyd_fragment(profile: Profile) -> str:
    """Only the inactive chat layer is persistent in the device config."""
    identity_keys = (
        set(profile.crafting)
        | set(profile.ninjabrain)
        | {profile.chat.open_chat, profile.chat.open_command}
    ) - {profile.chat.send, profile.chat.cancel}
    lines = ["[mcsr_chat]"]
    lines.extend(f"{key} = {key}" for key in sorted(identity_keys))
    lines.extend(
        [
            f"{profile.chat.send} = clearm(enter)",
            f"{profile.chat.cancel} = clearm(esc)",
        ]
    )
    fragment = "\n".join(lines) + "\n"
    validate_fragment(fragment)
    return fragment


def validate_fragment(fragment: str) -> None:
    """Validate the complete restricted grammar emitted by this module."""
    lines = fragment.splitlines()
    if not lines or lines[0] != "[mcsr_chat]":
        raise ValueError("invalid keyd fragment section")
    keys: set[str] = set()
    for line in lines[1:]:
        left, separator, right = line.partition(" = ")
        if (
            not separator
            or not re.fullmatch(r"[a-z0-9][a-z0-9_]*", left)
            or left not in supported_keys()
            or left in keys
        ):
            raise ValueError(f"invalid keyd fragment binding: {line}")
        if right not in {left, f"clearm({left})", "clearm(enter)", "clearm(esc)"}:
            raise ValueError(f"invalid keyd fragment action: {line}")
        keys.add(left)


def _run_keyd(args: list[str]) -> subprocess.CompletedProcess[str]:
    try:
        return subprocess.run(args, capture_output=True, text=True, timeout=3, check=False)
    except (OSError, subprocess.SubprocessError) as error:
        raise RuntimeError(f"keyd command failed: {error}") from error


def _run_clear(args: list[str]) -> subprocess.CompletedProcess[str]:
    # Restart, health polling, and failure-stop can each consume subprocess deadlines.
    try:
        return subprocess.run(args, capture_output=True, text=True, timeout=30, check=False)
    except (OSError, subprocess.SubprocessError) as error:
        raise RuntimeError(f"keyd clear failed: {error}") from error


class KeydCommands:
    """Fixed keyd CLI calls made by the user service after focus validation."""

    def __init__(
        self,
        profile: Profile,
        binary: str | None = None,
        runner: Callable[[list[str]], subprocess.CompletedProcess[str]] | None = None,
    ) -> None:
        self.profile = profile
        # Keep the clear helper reachable even if the keyd client disappears.
        self.binary = (
            binary or shutil.which("keyd.rvaiya") or shutil.which("keyd") or "/usr/bin/keyd.rvaiya"
        )
        self.runner = runner or _run_keyd
        self.clear_runner = runner or _run_clear

    def _call(self, *args: str) -> None:
        result = self.runner([self.binary, *args])
        if result.returncode != 0:
            raise RuntimeError(f"keyd command failed: {result.stderr.strip() or result.returncode}")

    def activate_game(self) -> None:
        self._call("bind", "reset", *game_bindings(self.profile))

    def clear_state(self) -> None:
        failures: list[str] = []
        for name, args, runner in (
            ("bind reset", [self.binary, "bind", "reset"], self.runner),
            ("helper clear", ["/usr/bin/sudo", "-n", HELPER_PATH, "clear"], self.clear_runner),
        ):
            try:
                result = runner(args)
                if result.returncode:
                    failures.append(f"{name} failed: {result.stderr.strip() or result.returncode}")
            except Exception as error:
                failures.append(f"{name} failed: {error}")
        if failures:
            raise RuntimeError("; ".join(failures))


HELPER_PATH = "/usr/local/libexec/mcsr-toolkit-keyd"


def _check_helper_path() -> None:
    helper = Path(HELPER_PATH)
    for path in (helper, *helper.parents):
        try:
            info = path.lstat()
        except OSError as error:
            raise RuntimeError(f"unsafe root-owned keyd helper path: {error}") from error
        if (
            info.st_uid != 0
            or info.st_mode & 0o022
            or stat.S_ISLNK(info.st_mode)
            or (path == helper and (not stat.S_ISREG(info.st_mode) or not info.st_mode & 0o100))
            or (path != helper and not stat.S_ISDIR(info.st_mode))
        ):
            raise RuntimeError("unsafe root-owned keyd helper")


def preflight_keyd_clear(
    runner: Callable[[list[str]], subprocess.CompletedProcess[str]] = _run_keyd,
) -> None:
    """Check the deployed helper and current service login's exact sudo permission."""
    _check_helper_path()
    try:
        result = runner(["/usr/bin/sudo", "-n", "-l", HELPER_PATH, "clear"])
    except Exception as error:
        raise RuntimeError(f"keyd clear permission check failed: {error}") from error
    if result.returncode:
        raise RuntimeError(
            f"keyd clear permission check failed: {result.stderr.strip() or result.returncode}"
        )


def _run_privileged(args: list[str]) -> subprocess.CompletedProcess[str]:
    _check_helper_path()
    try:
        return subprocess.run(args, capture_output=True, text=True, timeout=120, check=False)
    except (OSError, subprocess.SubprocessError) as error:
        raise RuntimeError(f"keyd elevation failed: {error}") from error


class PrivilegedKeydAdapter:
    """Authenticated install with persistent baseline restore and latest rollback.

    The root helper is deployed separately to HELPER_PATH; no user checkout is
    imported by the elevated interpreter. Installing never activates game binds.
    """

    def __init__(
        self,
        state_path: Path,
        runner: Callable[[list[str]], subprocess.CompletedProcess[str]] = _run_privileged,
    ) -> None:
        self.state_path = state_path
        self.runner = runner

    def _backups(self) -> list[str]:
        if self.state_path.is_symlink():
            raise ValueError("unsafe keyd adapter state")
        try:
            info = self.state_path.stat()
        except FileNotFoundError:
            return []
        if not stat.S_ISREG(info.st_mode) or info.st_uid != os.getuid() or info.st_mode & 0o077:
            raise ValueError("unsafe keyd adapter state")
        value = json.loads(self.state_path.read_text(encoding="utf-8"))
        if (
            not isinstance(value, dict)
            or set(value) != {"backups"}
            or not isinstance(value["backups"], list)
            or not value["backups"]
            or len(set(value["backups"])) != len(value["backups"])
            or any(
                not isinstance(item, str) or not re.fullmatch(r"[a-f0-9]{32}", item)
                for item in value["backups"]
            )
        ):
            raise ValueError("invalid keyd adapter state")
        return list(value["backups"])

    def _save(self, backups: list[str]) -> None:
        if not backups:
            self.state_path.unlink(missing_ok=True)
            return
        self.state_path.parent.mkdir(parents=True, exist_ok=True, mode=0o700)
        fd, name = tempfile.mkstemp(prefix=".keyd.", dir=self.state_path.parent)
        try:
            with os.fdopen(fd, "w", encoding="utf-8") as stream:
                os.fchmod(stream.fileno(), 0o600)
                json.dump({"backups": backups}, stream)
                stream.flush()
                os.fsync(stream.fileno())
            os.replace(name, self.state_path)
        finally:
            if os.path.exists(name):
                os.unlink(name)

    def _call(self, command: str, payload: str, response_key: str) -> str:
        result = self.runner(
            [
                "/usr/bin/pkexec",
                HELPER_PATH,
                command,
                payload,
            ]
        )
        if result.returncode:
            raise RuntimeError(
                f"keyd elevation failed: {result.stderr.strip() or result.returncode}"
            )
        try:
            value = json.loads(result.stdout)
        except (ValueError, TypeError) as error:
            raise RuntimeError("invalid keyd helper response") from error
        if (
            not isinstance(value, dict)
            or set(value) != {response_key}
            or not isinstance(value[response_key], str)
            or not re.fullmatch(r"[a-f0-9]{32}", value[response_key])
            or (command == "restore" and value[response_key] != payload)
        ):
            raise RuntimeError("invalid keyd helper response")
        return str(value[response_key])

    def install(self, profile: Profile) -> None:
        backups = self._backups()
        fragment = base64.b64encode(render_keyd_fragment(profile).encode()).decode("ascii")
        backup_id = self._call("install", fragment, "backup_id")
        try:
            self._save([*backups, backup_id])
        except Exception:
            self._call("restore", backup_id, "restored")
            raise

    def checkpoint(self) -> list[str]:
        """Return the persisted snapshot lineage without elevation."""
        return self._backups()

    def restore_to(self, checkpoint: list[str]) -> None:
        """Restore a known prefix, preserving earlier baseline snapshots."""
        backups = self._backups()
        if (
            not isinstance(checkpoint, list)
            or len(checkpoint) > len(backups)
            or backups[:len(checkpoint)] != checkpoint
        ):
            raise ValueError("stale keyd checkpoint")
        if len(checkpoint) < len(backups):
            self._call("restore", backups[len(checkpoint)], "restored")
            self._save(checkpoint)

    def rollback(self) -> None:
        backups = self._backups()
        if backups:
            self.restore_to(backups[:-1])

    def restore(self) -> None:
        self.restore_to([])
