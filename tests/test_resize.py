from __future__ import annotations

import subprocess

import pytest

from mcsr_toolkit.models import Profile, ResizePreset
from mcsr_toolkit.resize import PresetController, ResizeError
from mcsr_toolkit.sensitivity import SensitivityError


class FakeWindowBus:
    def __init__(self) -> None:
        self.size = (800, 600)
        self.inexact = False

    def resize_focused(self, width: int, height: int) -> tuple[int, int]:
        self.size = (width + int(self.inexact), height)
        return self.size


class FakeSensitivity:
    def __init__(self) -> None:
        self.value = "0.75"
        self.needs_restore = False
        self.calls: list[str] = []
        self.fail = False

    def eye(self) -> None:
        self.calls.append("eye")
        self.needs_restore = True
        if self.fail:
            raise SensitivityError("eye failed")
        self.value = "0.02083"

    def restore(self) -> None:
        self.calls.append("restore")
        if self.fail:
            raise SensitivityError("restore failed")
        self.value = "0.75"
        self.needs_restore = False


@pytest.fixture
def profile() -> Profile:
    return Profile(
        presets=[
            ResizePreset(id="normal", width=800, height=600, shortcut="f6"),
            ResizePreset(id="eye", width=400, height=300, shortcut="f7", eye_zoom=True),
        ]
    )


def test_eye_resize_requires_separate_activation(profile: Profile) -> None:
    adapter = FakeSensitivity()
    controller = PresetController(FakeWindowBus(), sensitivity=adapter)
    result = controller.apply_preset(profile, "eye")
    assert (result.width, result.height, result.sensitivity) == (400, 300, None)
    assert controller.active_preset is None
    assert adapter.calls == []
    result = controller.activate_eye(result)
    assert result.sensitivity == "0.02083"
    assert controller.active_preset == "eye"
    controller.apply_preset(profile, "normal")
    assert adapter.value == "0.75"
    assert adapter.calls == ["eye", "restore"]


def test_shifted_eye_and_normal_without_active_eye_need_no_helper(profile: Profile) -> None:
    adapter = FakeSensitivity()
    adapter.fail = True
    controller = PresetController(FakeWindowBus(), sensitivity=adapter)
    result = controller.apply_preset(profile, "eye", shifted=True)
    assert result.sensitivity is None
    with pytest.raises(ResizeError):
        controller.activate_eye(result)
    controller.apply_preset(profile, "normal")
    assert adapter.calls == []


def test_inexact_resize_cannot_activate_eye(profile: Profile) -> None:
    bus = FakeWindowBus()
    bus.inexact = True
    adapter = FakeSensitivity()
    controller = PresetController(bus, sensitivity=adapter)
    with pytest.raises(ResizeError, match="requested size"):
        controller.apply_preset(profile, "eye")
    assert adapter.calls == []


def test_focus_loss_restores_non_one_baseline(profile: Profile) -> None:
    adapter = FakeSensitivity()
    controller = PresetController(FakeWindowBus(), sensitivity=adapter)
    controller.activate_eye(controller.apply_preset(profile, "eye"))
    controller.focus_lost()
    assert adapter.value == "0.75"
    assert controller.active_preset is None


@pytest.mark.parametrize("operation", ["eye", "restore"])
def test_failed_sensitivity_does_not_claim_active_preset(profile: Profile, operation: str) -> None:
    adapter = FakeSensitivity()
    controller = PresetController(FakeWindowBus(), sensitivity=adapter)
    result = controller.apply_preset(profile, "eye")
    if operation == "restore":
        controller.activate_eye(result)
    adapter.fail = True
    with pytest.raises(ResizeError):
        if operation == "eye":
            controller.activate_eye(result)
        else:
            controller.apply_preset(profile, "normal")
    assert controller.active_preset is None
    assert adapter.needs_restore
    adapter.fail = False
    controller.focus_lost()
    assert adapter.value == "0.75"


def test_stale_resize_result_cannot_activate_after_normal(profile: Profile) -> None:
    adapter = FakeSensitivity()
    controller = PresetController(FakeWindowBus(), sensitivity=adapter)
    result = controller.apply_preset(profile, "eye")
    controller.apply_preset(profile, "normal")
    with pytest.raises(ResizeError):
        controller.activate_eye(result)
    assert adapter.calls == []


def test_rejects_invalid_or_unknown_before_resize(profile: Profile) -> None:
    bus = FakeWindowBus()
    controller = PresetController(bus, sensitivity=FakeSensitivity())
    with pytest.raises(ResizeError, match="unknown preset"):
        controller.apply_preset(profile, "missing")
    profile.presets[0].width = 99
    with pytest.raises(ResizeError, match="size"):
        controller.apply_preset(profile, "normal")
    assert bus.size == (800, 600)


def test_gnome_bus_calls_fixed_interface_and_parses_observation() -> None:
    from mcsr_toolkit.resize import GnomeWindowBus

    calls: list[list[str]] = []

    def run(args: list[str]) -> subprocess.CompletedProcess[str]:
        calls.append(args)
        return subprocess.CompletedProcess(args, 0, "(uint32 800, uint32 600)\n", "")

    assert GnomeWindowBus(run).resize_focused(800, 600) == (800, 600)
    assert calls == [
        [
            "gdbus",
            "call",
            "--session",
            "--dest",
            "org.gnome.Shell",
            "--object-path",
            "/org/mcsr/Toolkit",
            "--method",
            "org.mcsr.Toolkit.ResizeFocused",
            "800",
            "600",
        ]
    ]


@pytest.mark.parametrize("reply", ["(true,)", "(800, -1)", "(800, 600) garbage", "(0, 600)"])
def test_gnome_bus_rejects_invalid_reply(reply: str) -> None:
    from mcsr_toolkit.resize import GnomeWindowBus

    def run(args: list[str]) -> subprocess.CompletedProcess[str]:
        return subprocess.CompletedProcess(args, 0, reply, "")

    with pytest.raises(ResizeError, match="reply"):
        GnomeWindowBus(run).resize_focused(800, 600)


def test_profile_shortcuts_are_translated_and_ids_delivered(profile: Profile) -> None:
    import json

    from mcsr_toolkit.resize import GnomeWindowBus, preset_configuration

    profile.presets[0].shortcut = "leftcontrol+f6"
    config = preset_configuration(profile)
    assert config == [
        {"id": "normal", "accelerator": "<Control>F6", "eye_zoom": False},
        {"id": "eye", "accelerator": "F7", "eye_zoom": True},
    ]
    calls: list[list[str]] = []

    def run(args: list[str]) -> subprocess.CompletedProcess[str]:
        calls.append(args)
        return subprocess.CompletedProcess(args, 0, "()", "")

    GnomeWindowBus(run).configure_presets(profile)
    payload = calls[0][-1]
    assert json.loads(payload[1:-1]) == config
    assert calls[0][-2] == "org.mcsr.Toolkit.ConfigurePresets"


def test_extension_shortcuts_and_observed_resize() -> None:
    from pathlib import Path

    result = subprocess.run(
        ["node", "tests/extension_harness.cjs"],
        cwd=Path(__file__).resolve().parents[1],
        capture_output=True,
        text=True,
        check=False,
    )
    assert result.returncode == 0, result.stderr


def test_maccel_status_reports_unusable_binary() -> None:
    from mcsr_toolkit.resize import maccel_status

    def run(args: list[str]) -> subprocess.CompletedProcess[str]:
        return subprocess.CompletedProcess(args, 1, "", "device unavailable")

    status = maccel_status("/usr/bin/maccel", run)
    assert status.available
    assert not status.usable
    assert "device unavailable" in status.detail


def test_maccel_status_requires_valid_positive_multiplier() -> None:
    from mcsr_toolkit.resize import maccel_status

    def run(args: list[str]) -> subprocess.CompletedProcess[str]:
        return subprocess.CompletedProcess(args, 0, "nan", "")

    assert not maccel_status("maccel", run).usable


@pytest.mark.parametrize("modal", [True, False])
def test_get_focus_parses_current_gnome_modal_state(modal: bool) -> None:
    from mcsr_toolkit.focus import FocusEvent
    from mcsr_toolkit.resize import GnomeWindowBus

    commands: list[list[str]] = []

    def runner(args: list[str]) -> subprocess.CompletedProcess[str]:
        commands.append(args)
        return subprocess.CompletedProcess(
            args,
            0,
            f"('Minecraft', 'Game', uint32 800, uint32 600, {str(modal).lower()})",
            "",
        )

    assert GnomeWindowBus(runner).get_focus() == FocusEvent("Minecraft", "Game", modal)
    assert commands[0][-1] == "org.mcsr.Toolkit.GetFocus"


def test_get_focus_rejects_malformed_reply() -> None:
    from mcsr_toolkit.resize import GnomeWindowBus

    with pytest.raises(ResizeError, match="invalid GNOME focus reply"):
        GnomeWindowBus(
            lambda args: subprocess.CompletedProcess(args, 0, "('Minecraft',)", "")
        ).get_focus()


@pytest.mark.parametrize("failure", ["inexact", "exception"])
@pytest.mark.parametrize("pending", [False, True])
@pytest.mark.parametrize("restore_fails", [False, True])
def test_normal_resize_failure_still_restores_sensitivity(
    profile: Profile, failure: str, pending: bool, restore_fails: bool
) -> None:
    class FailingBus(FakeWindowBus):
        failing = False

        def resize_focused(self, width: int, height: int) -> tuple[int, int]:
            if self.failing and failure == "exception":
                raise ResizeError("bus resize failed")
            return (width + int(self.failing), height)

    adapter = FakeSensitivity()
    bus = FailingBus()
    controller = PresetController(bus, sensitivity=adapter)
    if pending:
        adapter.needs_restore = True
    else:
        controller.activate_eye(controller.apply_preset(profile, "eye"))
    adapter.calls.clear()
    adapter.fail = restore_fails
    bus.failing = True
    with pytest.raises(ResizeError) as caught:
        controller.apply_preset(profile, "normal")
    message = str(caught.value)
    assert ("requested size" if failure == "inexact" else "bus resize failed") in message
    if restore_fails:
        assert "restore failed" in message
        assert adapter.needs_restore
    else:
        assert not adapter.needs_restore
        assert adapter.value == "0.75"
    assert adapter.calls == ["restore"]
    assert controller.active_preset is None
