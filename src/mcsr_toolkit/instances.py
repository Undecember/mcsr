"""Locate MCSRLauncher data without traversing other launchers' files."""

from __future__ import annotations

import json
from dataclasses import dataclass
from pathlib import Path


@dataclass(frozen=True)
class ProcessInfo:
    cwd: Path


@dataclass(frozen=True)
class Instance:
    id: str
    path: Path
    options_path: Path
    problem: str | None = None


def _launcher_root(candidate: Path) -> Path | None:
    root = candidate.resolve()
    if (root / "instances").is_dir():
        return root
    return None


def locate_launcher_root(
    managed_dir: Path, configured_root: Path | None, running_processes: list[ProcessInfo]
) -> Path | None:
    """The launcher puts `launcher/instances` below its working directory."""
    if configured_root is not None:
        return _launcher_root(configured_root)
    for candidate in (managed_dir / "launcher", managed_dir):
        if root := _launcher_root(candidate):
            return root
    for process in running_processes:
        if root := _launcher_root(process.cwd / "launcher"):
            return root
    return None


def discover_instances(launcher_root: Path) -> list[Instance]:
    root = _launcher_root(launcher_root)
    if root is None:
        raise ValueError("not an MCSRLauncher root")
    instances_dir = root / "instances"
    found: list[Instance] = []
    for path in sorted(instances_dir.iterdir()):
        if path.is_symlink() or not path.is_dir():
            continue
        config_path = path / "instance.json"
        if config_path.is_symlink() or not config_path.is_file():
            continue
        try:
            config = json.loads(config_path.read_text(encoding="utf-8"))
        except (OSError, json.JSONDecodeError):
            continue
        if not isinstance(config, dict) or config.get("id") != path.name:
            continue
        preferred = path / "minecraft"
        legacy = path / ".minecraft"
        game = preferred if preferred.exists() or not legacy.exists() else legacy
        options_path = game / "options.txt"
        if game.is_symlink() or options_path.is_symlink():
            problem = "unsafe game path"
        elif not options_path.is_file():
            problem = "options.txt is missing"
        else:
            problem = None
        found.append(Instance(path.name, path, options_path, problem))
    return found
