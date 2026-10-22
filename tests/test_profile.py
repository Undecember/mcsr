from pathlib import Path

import pytest
from pydantic import ValidationError

from mcsr_toolkit.models import Profile, ResizePreset
from mcsr_toolkit.profile import load_profile, save_profile


def test_default_chat_keys_open_and_close_chat() -> None:
    profile = Profile()
    assert profile.chat.open_chat == "t"
    assert profile.chat.open_command == "slash"
    assert profile.chat.send == "enter"
    assert profile.chat.cancel == "esc"


def test_crafting_remap_can_share_a_movement_key() -> None:
    profile = Profile(controls={"key_key.back": "s"}, crafting={"s": "k"})
    assert profile.controls["key_key.back"] == "s"
    assert profile.crafting["s"] == "k"


def test_duplicate_physical_control_keys_are_rejected() -> None:
    with pytest.raises(ValidationError, match="physical control"):
        Profile(controls={"key_key.back": "s", "key_key.left": "s"})


def test_unknown_physical_key_is_rejected() -> None:
    with pytest.raises(ValidationError, match="key"):
        Profile(crafting={"not_a_real_key": "k"})


def test_duplicate_preset_shortcuts_are_rejected() -> None:
    with pytest.raises(ValidationError, match="shortcut"):
        Profile(
            presets=[
                ResizePreset(id="normal", width=1280, height=720, shortcut="f8"),
                ResizePreset(id="thin", width=400, height=720, shortcut="f8"),
            ]
        )


def test_nonpositive_preset_dimension_is_rejected() -> None:
    with pytest.raises(ValidationError, match="width"):
        ResizePreset(id="bad", width=0, height=720, shortcut="f8")


def test_more_than_eight_presets_is_rejected() -> None:
    with pytest.raises(ValidationError, match="eight presets"):
        Profile(
            presets=[
                ResizePreset(id=f"size{index}", width=800, height=600, shortcut=f"f{index}")
                for index in range(1, 10)
            ]
        )


def test_profile_round_trips_with_snippet_goals(tmp_path: Path) -> None:
    path = tmp_path / "state" / "profile.json"
    profile = Profile(
        controls={"key_key.back": "s"},
        crafting={"s": "k"},
        snippets=["KA", "ON"],
        presets=[ResizePreset(id="normal", width=1280, height=720, shortcut="f8")],
    )

    save_profile(path, profile)

    assert load_profile(path) == profile
    assert path.read_text().endswith("\n")


def test_missing_profile_loads_defaults(tmp_path: Path) -> None:
    assert load_profile(tmp_path / "missing.json") == Profile()


def test_chat_entry_key_cannot_also_be_crafting_source() -> None:
    with pytest.raises(ValidationError, match="reserved chat key"):
        Profile(crafting={"t": "k"})


def test_crafting_and_ninjabrain_cannot_use_same_physical_key() -> None:
    with pytest.raises(ValidationError, match="physical key"):
        Profile(crafting={"s": "k"}, ninjabrain={"s": "leftcontrol+n"})


def test_shortcut_requires_modifiers_before_one_nonmodifier_key() -> None:
    with pytest.raises(ValidationError, match="shortcut"):
        Profile(ninjabrain={"r": "a+b"})
    with pytest.raises(ValidationError, match="shortcut"):
        Profile(ninjabrain={"r": "leftcontrol+leftcontrol+n"})


def test_profile_syncs_renamed_parent_before_returning(tmp_path, monkeypatch):
    import os
    import stat

    from mcsr_toolkit import profile as profile_module

    path = tmp_path / 'state' / 'profile.json'
    events = []
    original_sync = os.fsync
    original_replace = os.replace

    def record_sync(fd):
        events.append('directory-sync' if stat.S_ISDIR(os.fstat(fd).st_mode) else 'file-sync')
        original_sync(fd)

    def record_replace(source, target):
        events.append('rename')
        original_replace(source, target)

    monkeypatch.setattr(profile_module.os, 'fsync', record_sync)
    monkeypatch.setattr(profile_module.os, 'replace', record_replace)
    save_profile(path, Profile(crafting={'s': 'k'}))
    assert events == ['file-sync', 'rename', 'directory-sync']
