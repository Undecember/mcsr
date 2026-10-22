"""Atomic storage for the user's toolkit profile."""

from __future__ import annotations

import os
import tempfile
from pathlib import Path

from .models import Profile


def default_profile_path() -> Path:
    root = Path(os.environ.get("XDG_DATA_HOME", Path.home() / ".local" / "share"))
    return root / "mcsr-toolkit" / "profile.json"


def load_profile(path: Path) -> Profile:
    if not path.exists():
        return Profile()
    return Profile.model_validate_json(path.read_text(encoding="utf-8"))


def save_profile(path: Path, profile: Profile) -> None:
    path.parent.mkdir(parents=True, exist_ok=True)
    temporary_path: Path | None = None
    try:
        with tempfile.NamedTemporaryFile(
            mode="w", encoding="utf-8", dir=path.parent, prefix=f".{path.name}.", delete=False
        ) as stream:
            temporary_path = Path(stream.name)
            os.chmod(stream.name, 0o600)
            stream.write(profile.model_dump_json(indent=2) + "\n")
            stream.flush()
            os.fsync(stream.fileno())
        os.replace(temporary_path, path)
        parent_fd = os.open(path.parent, os.O_RDONLY | os.O_DIRECTORY)
        try:
            os.fsync(parent_fd)
        finally:
            os.close(parent_fd)
    finally:
        if temporary_path is not None:
            temporary_path.unlink(missing_ok=True)
