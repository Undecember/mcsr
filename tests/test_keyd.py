from __future__ import annotations

import subprocess

import pytest

from mcsr_toolkit.keyd import KeydCommands, game_bindings, render_keyd_fragment, validate_fragment
from mcsr_toolkit.models import Profile


def test_chat_layer_passes_mapped_keys_and_chat_entry_keys_through() -> None:
    profile = Profile(crafting={"s": "k"}, ninjabrain={"r": "leftcontrol+n"})
    fragment = render_keyd_fragment(profile)
    assert "[mcsr_chat]" in fragment
    assert "s = s\n" in fragment
    assert "r = r\n" in fragment
    assert "t = t\n" in fragment
    assert "slash = slash\n" in fragment
    assert "enter = clearm(enter)\n" in fragment
    assert "esc = clearm(esc)\n" in fragment
    validate_fragment(fragment)


def test_game_bindings_combine_crafting_ninjabrain_and_chat() -> None:
    profile = Profile(crafting={"s": "k"}, ninjabrain={"r": "leftcontrol+leftshift+n"})
    assert game_bindings(profile) == [
        "s = k",
        "r = C-S-n",
        "t = togglem(mcsr_chat, t)",
        "slash = togglem(mcsr_chat, slash)",
    ]


def test_custom_chat_keys_preserve_requested_input() -> None:
    profile = Profile(
        chat={"open_chat": "y", "open_command": "p", "send": "tab", "cancel": "backspace"},
        crafting={"s": "k"},
    )
    assert "y = togglem(mcsr_chat, y)" in game_bindings(profile)
    assert "p = togglem(mcsr_chat, p)" in game_bindings(profile)
    fragment = render_keyd_fragment(profile)
    assert "tab = clearm(enter)" in fragment
    assert "backspace = clearm(esc)" in fragment


def test_invalid_fragment_is_rejected_before_install() -> None:
    try:
        validate_fragment("[mcsr_chat]\ns = command(echo bad)\n")
    except ValueError as error:
        assert "invalid" in str(error)
    else:
        raise AssertionError("invalid keyd action accepted")


def test_clear_resets_before_exact_sudo_command() -> None:
    calls: list[list[str]] = []

    def runner(args: list[str]) -> subprocess.CompletedProcess[str]:
        calls.append(args)
        return subprocess.CompletedProcess(args, 0, "Success\n", "")

    adapter = KeydCommands(Profile(crafting={"s": "k"}), "/usr/bin/keyd.rvaiya", runner)
    adapter.activate_game()
    adapter.clear_state()
    assert calls == [
        [
            "/usr/bin/keyd.rvaiya",
            "bind",
            "reset",
            "s = k",
            "t = togglem(mcsr_chat, t)",
            "slash = togglem(mcsr_chat, slash)",
        ],
        ["/usr/bin/keyd.rvaiya", "bind", "reset"],
        ["/usr/bin/sudo", "-n", "/usr/local/libexec/mcsr-toolkit-keyd", "clear"],
    ]


def test_clear_attempts_helper_even_if_reset_fails() -> None:
    calls: list[list[str]] = []

    def runner(args: list[str]) -> subprocess.CompletedProcess[str]:
        calls.append(args)
        error = "reset denied" if len(calls) == 1 else "restart denied"
        return subprocess.CompletedProcess(args, 1, "", error)

    adapter = KeydCommands(Profile(), "/usr/bin/keyd.rvaiya", runner)
    with pytest.raises(RuntimeError) as failure:
        adapter.clear_state()
    assert "reset denied" in str(failure.value)
    assert "restart denied" in str(failure.value)
    assert calls == [
        ["/usr/bin/keyd.rvaiya", "bind", "reset"],
        ["/usr/bin/sudo", "-n", "/usr/local/libexec/mcsr-toolkit-keyd", "clear"],
    ]


def test_clear_reports_helper_failure() -> None:
    def runner(args: list[str]) -> subprocess.CompletedProcess[str]:
        return subprocess.CompletedProcess(
            args, 0 if args[0] == "/usr/bin/keyd.rvaiya" else 1, "", "restart denied"
        )

    adapter = KeydCommands(Profile(), "/usr/bin/keyd.rvaiya", runner)
    with pytest.raises(RuntimeError, match="restart denied"):
        adapter.clear_state()


def test_missing_keyd_client_still_invokes_fixed_clear_helper(
    monkeypatch: pytest.MonkeyPatch,
) -> None:
    from mcsr_toolkit import keyd

    monkeypatch.setattr(keyd.shutil, 'which', lambda name: None)
    calls: list[list[str]] = []

    def runner(args: list[str]) -> subprocess.CompletedProcess[str]:
        calls.append(args)
        if args[0] == '/usr/bin/keyd.rvaiya':
            raise FileNotFoundError('keyd client missing')
        return subprocess.CompletedProcess(args, 0, '', '')

    adapter = KeydCommands(Profile(), runner=runner)
    with pytest.raises(RuntimeError, match='keyd client missing'):
        adapter.clear_state()
    assert calls == [
        ['/usr/bin/keyd.rvaiya', 'bind', 'reset'],
        ['/usr/bin/sudo', '-n', '/usr/local/libexec/mcsr-toolkit-keyd', 'clear'],
    ]


def test_clear_allows_bounded_helper_recovery_time(monkeypatch: pytest.MonkeyPatch) -> None:
    calls: list[tuple[list[str], float]] = []

    def fake_run(
        args: list[str],
        *,
        capture_output: bool,
        text: bool,
        timeout: float,
        check: bool,
    ) -> subprocess.CompletedProcess[str]:
        calls.append((args, timeout))
        return subprocess.CompletedProcess(args, 0, "", "")

    monkeypatch.setattr(subprocess, "run", fake_run)
    KeydCommands(Profile(), "/usr/bin/keyd.rvaiya").clear_state()
    assert calls == [
        (["/usr/bin/keyd.rvaiya", "bind", "reset"], 3),
        (["/usr/bin/sudo", "-n", "/usr/local/libexec/mcsr-toolkit-keyd", "clear"], 30),
    ]


def test_preflight_rejects_mutable_helper_path(monkeypatch: pytest.MonkeyPatch) -> None:
    from pathlib import Path
    from types import SimpleNamespace

    from mcsr_toolkit.keyd import preflight_keyd_clear

    def fake_lstat(path: Path) -> object:
        mode = 0o100755 if str(path) == "/usr/local/libexec/mcsr-toolkit-keyd" else 0o040755
        if str(path) == "/usr/local":
            mode |= 0o020
        return SimpleNamespace(st_mode=mode, st_uid=0)

    monkeypatch.setattr(Path, "lstat", fake_lstat)

    def forbidden(args: list[str]) -> subprocess.CompletedProcess[str]:
        raise AssertionError(f"unsafe path must not invoke sudo: {args}")

    with pytest.raises(RuntimeError, match="unsafe"):
        preflight_keyd_clear(runner=forbidden)


def test_preflight_rejects_missing_exact_sudo_permission(
    monkeypatch: pytest.MonkeyPatch,
) -> None:
    from pathlib import Path
    from types import SimpleNamespace

    from mcsr_toolkit.keyd import preflight_keyd_clear

    def fake_lstat(path: Path) -> object:
        mode = 0o100755 if str(path) == "/usr/local/libexec/mcsr-toolkit-keyd" else 0o040755
        return SimpleNamespace(st_mode=mode, st_uid=0)

    monkeypatch.setattr(Path, "lstat", fake_lstat)
    calls: list[list[str]] = []

    def runner(args: list[str]) -> subprocess.CompletedProcess[str]:
        calls.append(args)
        return subprocess.CompletedProcess(args, 1, "", "not allowed")

    with pytest.raises(RuntimeError, match="not allowed"):
        preflight_keyd_clear(runner=runner)
    assert calls == [
        ["/usr/bin/sudo", "-n", "-l", "/usr/local/libexec/mcsr-toolkit-keyd", "clear"]
    ]


def test_sudoers_template_grants_only_fixed_clear_command() -> None:
    from pathlib import Path

    template = Path("packaging/mcsr-toolkit-keyd.sudoers").read_text(encoding="utf-8")
    rules = [
        line.strip()
        for line in template.splitlines()
        if line.strip() and not line.startswith("#")
    ]
    assert rules == [
        "<desktop-user> ALL=(root) NOPASSWD: /usr/local/libexec/mcsr-toolkit-keyd clear"
    ]
    assert "*" not in rules[0]


def test_elevated_adapter_persists_baseline_and_rolls_back_latest(tmp_path: object) -> None:
    import base64
    from pathlib import Path

    from mcsr_toolkit.keyd import PrivilegedKeydAdapter

    assert isinstance(tmp_path, Path)
    calls: list[list[str]] = []
    backups = iter(["a" * 32, "b" * 32])

    def runner(args: list[str]) -> subprocess.CompletedProcess[str]:
        calls.append(args)
        if args[-2] == "install":
            assert base64.b64decode(args[-1]).decode() == (
                "[mcsr_chat]\nslash = slash\nt = t\nenter = clearm(enter)\nesc = clearm(esc)\n"
            )
            return subprocess.CompletedProcess(
                args, 0, '{"backup_id": "' + next(backups) + '"}', ""
            )
        return subprocess.CompletedProcess(args, 0, '{"restored": "' + args[-1] + '"}', "")

    state = tmp_path / "keyd.json"
    adapter = PrivilegedKeydAdapter(state, runner=runner)
    adapter.install(Profile())
    adapter.install(Profile())
    restarted = PrivilegedKeydAdapter(state, runner=runner)
    restarted.rollback()
    restarted.restore()
    assert calls[-2][-2:] == ["restore", "b" * 32]
    assert calls[-1][-2:] == ["restore", "a" * 32]
    assert calls[0][:-2] == [
        "/usr/bin/pkexec",
        "/usr/local/libexec/mcsr-toolkit-keyd",
    ]
    assert not state.exists()


@pytest.mark.parametrize("output", ["{}", '{"backup_id": "../escape"}', "not json"])
def test_elevated_adapter_rejects_invalid_response(tmp_path: object, output: str) -> None:
    from pathlib import Path

    from mcsr_toolkit.keyd import PrivilegedKeydAdapter

    assert isinstance(tmp_path, Path)

    def runner(args: list[str]) -> subprocess.CompletedProcess[str]:
        return subprocess.CompletedProcess(args, 0, output, "")

    state = tmp_path / "keyd.json"
    with pytest.raises(RuntimeError, match="response"):
        PrivilegedKeydAdapter(state, runner=runner).install(Profile())
    assert not state.exists()


def test_elevated_adapter_preserves_state_after_auth_denial(tmp_path: object) -> None:
    import json
    from pathlib import Path

    from mcsr_toolkit.keyd import PrivilegedKeydAdapter

    assert isinstance(tmp_path, Path)
    state = tmp_path / "keyd.json"
    original = json.dumps({"backups": ["a" * 32]})
    state.write_text(original)
    state.chmod(0o600)

    def runner(args: list[str]) -> subprocess.CompletedProcess[str]:
        return subprocess.CompletedProcess(args, 126, "", "authorization cancelled")

    with pytest.raises(RuntimeError, match="cancelled"):
        PrivilegedKeydAdapter(state, runner=runner).restore()
    assert state.read_text() == original


def test_elevated_adapter_rejects_symlink_state_before_elevation(tmp_path: object) -> None:
    from pathlib import Path

    from mcsr_toolkit.keyd import PrivilegedKeydAdapter

    assert isinstance(tmp_path, Path)
    outside = tmp_path / "outside.json"
    outside.write_text('{"backups": []}')
    state = tmp_path / "keyd.json"
    state.symlink_to(outside)

    def runner(args: list[str]) -> subprocess.CompletedProcess[str]:
        raise AssertionError("must reject unsafe state before elevation")

    with pytest.raises(ValueError, match="unsafe"):
        PrivilegedKeydAdapter(state, runner=runner).install(Profile())
    assert outside.read_text() == '{"backups": []}'


@pytest.mark.parametrize("key", ["#", "=", "[main]", "a b", "a,b", "a)"])
def test_profile_rejects_keyd_keys_with_config_syntax(
    monkeypatch: pytest.MonkeyPatch, key: str
) -> None:
    from mcsr_toolkit import models

    monkeypatch.setattr(
        models, "supported_keys", lambda: frozenset({key, "s", "t", "slash", "enter", "esc"})
    )
    with pytest.raises(ValueError):
        Profile(crafting={"s": key})
    with pytest.raises(ValueError):
        Profile(chat={"send": key})


def test_supported_keys_filters_config_syntax_from_daemon_output(
    monkeypatch: pytest.MonkeyPatch,
) -> None:
    from mcsr_toolkit import models

    models.supported_keys.cache_clear()
    monkeypatch.setattr(models.shutil, "which", lambda name: "/usr/bin/keyd.rvaiya")
    monkeypatch.setattr(
        models.subprocess,
        "run",
        lambda *args, **kwargs: subprocess.CompletedProcess(
            args, 0, "#\n=\n[main]\nkp1\nbackspace\n", ""
        ),
    )
    try:
        keys = models.supported_keys()
        assert {"kp1", "backspace"} <= keys
        assert not {"#", "=", "[main]"} & keys
    finally:
        models.supported_keys.cache_clear()


@pytest.mark.parametrize(
    "mode,owner", [(0o100644, 0), (0o100777, 0), (0o120755, 0), (0o100755, 1000)]
)
def test_elevation_refuses_unsafe_or_nonexecutable_helper(
    monkeypatch: pytest.MonkeyPatch,
    mode: int,
    owner: int,
) -> None:
    from pathlib import Path
    from types import SimpleNamespace

    from mcsr_toolkit.keyd import _run_privileged

    monkeypatch.setattr(Path, "lstat", lambda path: SimpleNamespace(st_mode=mode, st_uid=owner))

    def forbidden(*args: object, **kwargs: object) -> object:
        raise AssertionError("unsafe helper must be rejected before elevation")

    monkeypatch.setattr(subprocess, "run", forbidden)
    with pytest.raises(RuntimeError, match="unsafe"):
        _run_privileged(["/usr/bin/pkexec", "/usr/local/libexec/mcsr-toolkit-keyd", "install", "x"])


def test_restore_checkpoint_uses_snapshot_for_prior_profile(tmp_path):
    import json

    from mcsr_toolkit.keyd import PrivilegedKeydAdapter

    calls = []
    identifiers = iter(['a' * 32, 'b' * 32])

    def runner(args):
        calls.append(args)
        if args[-2] == 'install':
            return subprocess.CompletedProcess(
                args, 0, json.dumps({'backup_id': next(identifiers)})
            )
        return subprocess.CompletedProcess(args, 0, json.dumps({'restored': args[-1]}))

    state = tmp_path / 'keyd.json'
    adapter = PrivilegedKeydAdapter(state, runner)
    adapter.install(Profile(crafting={'s': 'k'}))
    checkpoint = adapter.checkpoint()
    adapter.install(Profile(crafting={'s': 'j'}))
    restarted = PrivilegedKeydAdapter(state, runner)
    restarted.restore_to(checkpoint)
    assert calls[-1][-2:] == ['restore', 'b' * 32]
    assert restarted.checkpoint() == ['a' * 32]
    restarted.restore_to([])
    assert calls[-1][-2:] == ['restore', 'a' * 32]
    assert not state.exists()


def test_restore_checkpoint_rejects_non_prefix_before_elevation(tmp_path):
    from mcsr_toolkit.keyd import PrivilegedKeydAdapter

    state = tmp_path / 'keyd.json'
    state.write_text('{"backups": ["' + 'a' * 32 + '"]}')
    state.chmod(0o600)

    def runner(args):
        raise AssertionError('stale checkpoint must not elevate')

    adapter = PrivilegedKeydAdapter(state, runner)
    with pytest.raises(ValueError, match='checkpoint'):
        adapter.restore_to(['b' * 32])
