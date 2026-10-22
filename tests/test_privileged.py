from __future__ import annotations

import subprocess
from pathlib import Path

import pytest

from mcsr_toolkit import privileged
from mcsr_toolkit.privileged import DEVICE_CONFIG, KeydInstaller

VALID_KEYS = frozenset({"s", "t", "slash", "enter", "esc"})
FRAGMENT = "[mcsr_chat]\ns = s\nt = t\nslash = slash\nenter = clearm(enter)\nesc = clearm(esc)\n"


def make_installer(tmp_path: Path, restart: object = None) -> KeydInstaller:
    config_dir = tmp_path / "keyd"
    config_dir.mkdir()
    backup_root = tmp_path / "backups"
    return KeydInstaller(config_dir, backup_root, restart or (lambda: None), VALID_KEYS)


def test_install_creates_only_managed_files_and_restore_removes_them(tmp_path: Path) -> None:
    calls: list[str] = []
    installer = make_installer(tmp_path, lambda: calls.append("restart"))
    backup_id = installer.install(FRAGMENT)
    assert (installer.config_dir / "mcsr-toolkit.conf").read_text() == DEVICE_CONFIG
    assert (installer.config_dir / "mcsr-toolkit").read_text() == FRAGMENT
    assert calls == ["restart"]

    installer.restore(backup_id)
    assert list(installer.config_dir.iterdir()) == []
    assert calls == ["restart", "restart"]


def test_existing_unrelated_device_config_is_preserved_and_rejected(tmp_path: Path) -> None:
    installer = make_installer(tmp_path)
    old = installer.config_dir / "my-keyboard.conf"
    old.write_text("[ids]\n*\n[main]\na = b\n")
    with pytest.raises(ValueError, match="existing keyd config"):
        installer.install(FRAGMENT)
    assert old.read_text() == "[ids]\n*\n[main]\na = b\n"
    assert not (installer.config_dir / "mcsr-toolkit.conf").exists()


def test_symlink_target_is_rejected(tmp_path: Path) -> None:
    installer = make_installer(tmp_path)
    outside = tmp_path / "outside"
    outside.write_text("unchanged")
    (installer.config_dir / "mcsr-toolkit").symlink_to(outside)
    with pytest.raises(ValueError, match="symlink"):
        installer.install(FRAGMENT)
    assert outside.read_text() == "unchanged"


def test_invalid_keyd_action_is_rejected_before_files_change(tmp_path: Path) -> None:
    installer = make_installer(tmp_path)
    with pytest.raises(ValueError, match="invalid"):
        installer.install("[mcsr_chat]\ns = command(echo bad)\n")
    assert list(installer.config_dir.iterdir()) == []


def test_failed_restart_restores_previous_files(tmp_path: Path) -> None:
    calls = 0

    def restart() -> None:
        nonlocal calls
        calls += 1
        if calls == 2:
            raise RuntimeError("keyd restart failed")

    installer = make_installer(tmp_path, restart)
    installer.install(FRAGMENT)
    updated = FRAGMENT.replace("s = s", "s = clearm(s)")
    with pytest.raises(RuntimeError, match="restart failed"):
        installer.install(updated)
    assert (installer.config_dir / "mcsr-toolkit").read_text() == FRAGMENT
    assert calls == 3


def test_restore_rejects_unknown_backup_id(tmp_path: Path) -> None:
    installer = make_installer(tmp_path)
    with pytest.raises(ValueError, match="backup"):
        installer.restore("../outside")


def test_backup_root_symlink_is_rejected_before_install(tmp_path: Path) -> None:
    installer = make_installer(tmp_path)
    outside = tmp_path / "outside"
    outside.mkdir()
    installer.backup_root.symlink_to(outside, target_is_directory=True)
    with pytest.raises(ValueError, match="unsafe"):
        installer.install(FRAGMENT)
    assert list(installer.config_dir.iterdir()) == []
    assert list(outside.iterdir()) == []


@pytest.mark.parametrize(
    "snapshot",
    [
        {"text": "[ids]\n*\n[main]\ns = k\n", "mode": 420},
        {"text": DEVICE_CONFIG, "mode": 0o4777},
        {"text": DEVICE_CONFIG, "mode": "644"},
    ],
)
def test_restore_rejects_corrupt_snapshot_before_mutation(tmp_path: Path, snapshot: object) -> None:
    import json

    installer = make_installer(tmp_path)
    backup_id = installer.install(FRAGMENT)
    manifest = installer.backup_root / backup_id / "manifest.json"
    manifest.write_text(json.dumps({"device": snapshot, "fragment": None}))
    with pytest.raises(ValueError, match="backup"):
        installer.restore(backup_id)
    assert (installer.config_dir / "mcsr-toolkit.conf").read_text() == DEVICE_CONFIG
    assert (installer.config_dir / "mcsr-toolkit").read_text() == FRAGMENT


def test_restore_restart_failure_keeps_installed_config(tmp_path: Path) -> None:
    calls = 0

    def restart() -> None:
        nonlocal calls
        calls += 1
        if calls == 2:
            raise RuntimeError("restore restart failed")

    installer = make_installer(tmp_path, restart)
    backup_id = installer.install(FRAGMENT)
    with pytest.raises(RuntimeError, match="restore restart failed"):
        installer.restore(backup_id)
    assert (installer.config_dir / "mcsr-toolkit.conf").read_text() == DEVICE_CONFIG
    assert (installer.config_dir / "mcsr-toolkit").read_text() == FRAGMENT
    assert calls == 3


def test_config_directory_symlink_ancestor_is_rejected(tmp_path: Path) -> None:
    actual = tmp_path / "actual"
    actual.mkdir()
    config = actual / "keyd"
    config.mkdir()
    alias = tmp_path / "alias"
    alias.symlink_to(actual, target_is_directory=True)
    installer = KeydInstaller(alias / "keyd", tmp_path / "backups", lambda: None, VALID_KEYS)
    with pytest.raises(ValueError, match="unsafe"):
        installer.install(FRAGMENT)
    assert list(config.iterdir()) == []


def test_first_install_creates_private_backup_parent(tmp_path: Path) -> None:
    installer = make_installer(tmp_path)
    installer.backup_root = tmp_path / "new-state" / "keyd"
    backup_id = installer.install(FRAGMENT)
    assert (installer.backup_root / backup_id / "manifest.json").is_file()
    assert installer.backup_root.parent.stat().st_mode & 0o777 == 0o700


@pytest.mark.parametrize("key", ["#", "=", "[main]", "a b", "a,b", "a)"])
def test_installer_rejects_config_syntax_even_in_keyd_vocabulary(tmp_path: Path, key: str) -> None:
    installer = make_installer(tmp_path)
    installer.valid_keys = frozenset({key})
    with pytest.raises(ValueError, match="invalid"):
        installer.install(f"[mcsr_chat]\n{key} = clearm({key})\n")
    assert list(installer.config_dir.iterdir()) == []


def test_deployed_helper_executes_directly_with_isolated_imports(tmp_path: Path) -> None:
    import os
    import shutil
    import subprocess

    helper = tmp_path / "mcsr-toolkit-keyd"
    source = Path(__file__).parents[1] / "src" / "mcsr_toolkit" / "privileged.py"
    shutil.copyfile(source, helper)
    helper.chmod(0o755)
    (tmp_path / "json.py").write_text('raise RuntimeError("untrusted import executed")\n')
    environment = dict(os.environ, PYTHONPATH=str(tmp_path))
    result = subprocess.run(
        [str(helper), "--help"],
        capture_output=True,
        text=True,
        env=environment,
        timeout=5,
        check=False,
    )
    assert "untrusted import executed" not in result.stderr
    if os.geteuid() == 0:
        assert result.returncode == 0
        assert "install" in result.stdout
    else:
        assert result.returncode != 0
        assert "installer must run as root" in result.stderr


class FakeClock:
    def __init__(self) -> None:
        self.now = 0.0

    def clock(self) -> float:
        return self.now

    def sleep(self, seconds: float) -> None:
        assert seconds > 0
        self.now += seconds


def test_clear_accepts_no_extra_argument(monkeypatch: pytest.MonkeyPatch) -> None:
    monkeypatch.setattr(privileged.os, "geteuid", lambda: 0)
    calls: list[str] = []
    monkeypatch.setattr(privileged, "restart_keyd_service", lambda *args: calls.append("restart"))
    assert privileged.main(["clear"]) == 0
    assert calls == ["restart"]
    with pytest.raises(SystemExit) as error:
        privileged.main(["clear", "extra"])
    assert error.value.code == 2
    assert calls == ["restart"]


def test_clear_restarts_and_checks_health(monkeypatch: pytest.MonkeyPatch) -> None:
    monkeypatch.setattr(privileged.os, "geteuid", lambda: 0)
    clock = FakeClock()
    calls: list[tuple[list[str], float]] = []

    def runner(command: list[str], timeout: float) -> subprocess.CompletedProcess[str]:
        calls.append((command, timeout))
        return subprocess.CompletedProcess(command, 0, "", "")

    monkeypatch.setattr(privileged, "_run_systemctl", runner)
    monkeypatch.setattr(privileged.time, "monotonic", clock.clock)
    monkeypatch.setattr(privileged.time, "sleep", clock.sleep)
    assert privileged.main(["clear"]) == 0
    assert calls[0] == (["/usr/bin/systemctl", "restart", "keyd.service"], 5)
    assert len(calls) >= 6
    assert all(
        command == ["/usr/bin/systemctl", "is-active", "--quiet", "keyd.service"]
        and timeout == 5
        for command, timeout in calls[1:]
    )
    assert 0.5 <= clock.now <= 0.6


def test_restart_timeout_stops_daemon() -> None:
    clock = FakeClock()
    calls: list[tuple[list[str], float]] = []

    def runner(command: list[str], timeout: float) -> subprocess.CompletedProcess[str]:
        calls.append((command, timeout))
        code = 3 if command[1] == "is-active" else 0
        return subprocess.CompletedProcess(command, code, "", "")

    with pytest.raises(RuntimeError, match="health"):
        privileged.restart_keyd_service(runner, clock.clock, clock.sleep)
    assert calls[0] == (["/usr/bin/systemctl", "restart", "keyd.service"], 5)
    assert calls[-1] == (["/usr/bin/systemctl", "stop", "keyd.service"], 5)
    assert len(calls) <= 54
    assert 5 <= clock.now <= 5.1
    assert all(timeout == 5 for _, timeout in calls)


def test_failed_stop_reports_unsafe_state() -> None:
    clock = FakeClock()
    calls: list[list[str]] = []

    def runner(command: list[str], timeout: float) -> subprocess.CompletedProcess[str]:
        calls.append(command)
        return subprocess.CompletedProcess(command, 1, "", "failure")

    with pytest.raises(RuntimeError, match="stop failed.*unsafe"):
        privileged.restart_keyd_service(runner, clock.clock, clock.sleep)
    assert calls == [
        ["/usr/bin/systemctl", "restart", "keyd.service"],
        ["/usr/bin/systemctl", "stop", "keyd.service"],
    ]


def test_health_drop_during_stability_stops_daemon() -> None:
    clock = FakeClock()
    calls: list[list[str]] = []

    def runner(command: list[str], timeout: float) -> subprocess.CompletedProcess[str]:
        calls.append(command)
        code = 3 if command[1] == "is-active" and len(calls) == 4 else 0
        return subprocess.CompletedProcess(command, code, "", "")

    with pytest.raises(RuntimeError, match="stability"):
        privileged.restart_keyd_service(runner, clock.clock, clock.sleep)
    assert calls[-1] == ["/usr/bin/systemctl", "stop", "keyd.service"]
    assert clock.now < 0.5


def test_slow_health_check_during_stability_stops_daemon() -> None:
    clock = FakeClock()
    calls: list[list[str]] = []

    def runner(command: list[str], timeout: float) -> subprocess.CompletedProcess[str]:
        calls.append(command)
        if command[1] == "is-active" and len(calls) == 3:
            clock.now += 0.6
        return subprocess.CompletedProcess(command, 0, "", "")

    with pytest.raises(RuntimeError, match="stability"):
        privileged.restart_keyd_service(runner, clock.clock, clock.sleep)
    assert calls == [
        ["/usr/bin/systemctl", "restart", "keyd.service"],
        ["/usr/bin/systemctl", "is-active", "--quiet", "keyd.service"],
        ["/usr/bin/systemctl", "is-active", "--quiet", "keyd.service"],
        ["/usr/bin/systemctl", "stop", "keyd.service"],
    ]


def test_late_first_health_success_still_stops_daemon() -> None:
    clock = FakeClock()
    calls: list[list[str]] = []

    def runner(command: list[str], timeout: float) -> subprocess.CompletedProcess[str]:
        calls.append(command)
        if command[1] == "is-active" and len(calls) == 2:
            clock.now = 5.1
        return subprocess.CompletedProcess(command, 0, "", "")

    with pytest.raises(RuntimeError, match="timed out"):
        privileged.restart_keyd_service(runner, clock.clock, clock.sleep)
    assert calls == [
        ["/usr/bin/systemctl", "restart", "keyd.service"],
        ["/usr/bin/systemctl", "is-active", "--quiet", "keyd.service"],
        ["/usr/bin/systemctl", "stop", "keyd.service"],
    ]


def test_install_failure_restores_initially_empty_config(tmp_path: Path) -> None:
    calls = 0

    def restart() -> None:
        nonlocal calls
        calls += 1
        if calls == 1:
            raise RuntimeError("restart failed")

    installer = make_installer(tmp_path, restart)
    with pytest.raises(RuntimeError, match="restart failed"):
        installer.install(FRAGMENT)
    assert not (installer.config_dir / "mcsr-toolkit.conf").exists()
    assert not (installer.config_dir / "mcsr-toolkit").exists()
    assert calls == 2


def test_restore_failure_preserves_current_snapshot(tmp_path: Path) -> None:
    calls = 0

    def restart() -> None:
        nonlocal calls
        calls += 1
        if calls == 3:
            raise RuntimeError("restore restart failed")

    installer = make_installer(tmp_path, restart)
    first_id = installer.install(FRAGMENT)
    updated = FRAGMENT.replace("s = s", "s = clearm(s)")
    installer.install(updated)
    with pytest.raises(RuntimeError, match="restore restart failed"):
        installer.restore(first_id)
    assert (installer.config_dir / "mcsr-toolkit.conf").read_text() == DEVICE_CONFIG
    assert (installer.config_dir / "mcsr-toolkit").read_text() == updated
    assert calls == 4


def test_failed_rollback_reports_both_failures(tmp_path: Path) -> None:
    calls = 0

    def restart() -> None:
        nonlocal calls
        calls += 1
        raise RuntimeError(f"restart failure {calls}")

    installer = make_installer(tmp_path, restart)
    with pytest.raises(ExceptionGroup, match="install and rollback failed") as error:
        installer.install(FRAGMENT)
    assert len(error.value.exceptions) == 2
    assert "restart failure 1" in str(error.value.exceptions[0])
    assert "restart failure 2" in str(error.value.exceptions[1])
    assert not (installer.config_dir / "mcsr-toolkit.conf").exists()
    assert not (installer.config_dir / "mcsr-toolkit").exists()


def test_helper_accepts_only_canonical_custom_chat_exit_events() -> None:
    keys = frozenset({'tab', 'backspace', 'enter', 'esc'})
    privileged._validate_fragment(
        '[mcsr_chat]\ntab = clearm(enter)\nbackspace = clearm(esc)\n', keys,
    )
    with pytest.raises(ValueError, match='invalid keyd fragment action'):
        privileged._validate_fragment('[mcsr_chat]\ntab = clearm(backspace)\n', keys)
