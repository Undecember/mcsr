"""Boundary tests never execute sudo or write to the actual driver."""

from __future__ import annotations

import stat
import subprocess
from pathlib import Path
from types import SimpleNamespace

import pytest

from mcsr_toolkit import sensitivity


class Commands:
    def __init__(self) -> None:
        self.calls: list[tuple[list[str], str | None]] = []
        self.failure: str | None = None
        self.token = "a" * 64
        self.timeout = False

    def __call__(self, args: list[str], token: str | None) -> subprocess.CompletedProcess[str]:
        self.calls.append((args, token))
        if args[-1] == "eye" and token == "":
            return subprocess.CompletedProcess(
                args, 1, "", "maccel helper: missing or malformed generation\n"
            )
        if self.timeout and "-l" not in args and args[-1] in ("eye", "restore"):
            raise subprocess.TimeoutExpired(args, 3)
        if args[-1] == self.failure and "-l" not in args:
            return subprocess.CompletedProcess(args, 1, "", "denied")
        output = self.token + "\n" if args[-1] == "restore" else ""
        if args[1:4] == ["get", "param", "sens-mult"]:
            output = "0.75\n"
        return subprocess.CompletedProcess(args, 0, output, "")


@pytest.fixture
def adapter(monkeypatch: pytest.MonkeyPatch) -> tuple[sensitivity.SensitivityAdapter, Commands]:
    monkeypatch.setattr(sensitivity, "_trusted_helper", lambda: None)
    commands = Commands()
    return sensitivity.SensitivityAdapter(
        runner=commands, maccel_binary="/usr/bin/maccel"
    ), commands


def test_preflight_checks_exact_permissions_without_executing_helper(adapter) -> None:
    subject, commands = adapter
    status = subject.preflight()
    assert status.available
    assert not status.usable  # Startup recovery has not happened.
    assert commands.calls == [
        (["/usr/bin/maccel", "get", "param", "sens-mult"], None),
        (
            ["/usr/bin/sudo", "-n", "-l", "--", "/usr/local/libexec/mcsr-toolkit-maccel", "eye"],
            None,
        ),
        (
            [
                "/usr/bin/sudo",
                "-n",
                "-l",
                "--",
                "/usr/local/libexec/mcsr-toolkit-maccel",
                "restore",
            ],
            None,
        ),
    ]


def test_startup_restore_rotates_token_and_eye_supplies_only_stdin(adapter) -> None:
    subject, commands = adapter
    subject.startup_restore()
    assert subject.preflight().usable
    subject.eye()
    commands.token = "b" * 64
    subject.restore()
    subject.eye()
    mutations = [
        call
        for call in commands.calls
        if call[0][1:3] == ["-n", "/usr/local/libexec/mcsr-toolkit-maccel"] and call[1] != ""
    ]
    assert mutations == [
        (["/usr/bin/sudo", "-n", "/usr/local/libexec/mcsr-toolkit-maccel", "restore"], None),
        (["/usr/bin/sudo", "-n", "/usr/local/libexec/mcsr-toolkit-maccel", "eye"], "a" * 64 + "\n"),
        (["/usr/bin/sudo", "-n", "/usr/local/libexec/mcsr-toolkit-maccel", "restore"], None),
        (["/usr/bin/sudo", "-n", "/usr/local/libexec/mcsr-toolkit-maccel", "eye"], "b" * 64 + "\n"),
    ]


@pytest.mark.parametrize("operation", ["eye", "restore"])
@pytest.mark.parametrize("timeout", [False, True])
def test_failed_mutation_blocks_eye_until_successful_restore(adapter, operation, timeout) -> None:
    subject, commands = adapter
    subject.startup_restore()
    commands.failure = operation
    commands.timeout = timeout
    with pytest.raises(sensitivity.SensitivityError):
        getattr(subject, operation)()
    assert subject.recovery_pending
    before = len(commands.calls)
    with pytest.raises(sensitivity.SensitivityError):
        subject.eye()
    assert len(commands.calls) == before
    commands.failure = None
    commands.timeout = False
    assert not subject.preflight().usable
    subject.restore()
    assert not subject.recovery_pending
    subject.eye()


@pytest.mark.parametrize("token", ["", "1", "a" * 63, "a" * 64 + "\nextra"])
def test_malformed_restore_token_retains_recovery(adapter, token) -> None:
    subject, commands = adapter
    commands.token = token
    with pytest.raises(sensitivity.SensitivityError):
        subject.startup_restore()
    assert subject.recovery_pending
    assert not subject.preflight().usable


def test_readable_maccel_does_not_imply_helper_available(adapter, monkeypatch) -> None:
    subject, commands = adapter

    def absent() -> None:
        raise OSError("helper absent")

    monkeypatch.setattr(sensitivity, "_trusted_helper", absent)
    status = subject.preflight()
    assert status.available and not status.usable
    assert "helper absent" in status.detail
    assert len(commands.calls) == 1


def test_missing_sudo_permission_disables_eye(adapter) -> None:
    subject, commands = adapter

    def denied(args, token):
        return subprocess.CompletedProcess(args, 1 if "-l" in args else 0, "0.75", "denied")

    subject._runner = denied
    status = subject.preflight()
    assert not status.usable
    assert "denied" in status.detail


@pytest.mark.parametrize(
    "bad_path",
    ["/", "/usr", "/usr/local", "/usr/local/libexec", "/usr/local/libexec/mcsr-toolkit-maccel"],
)
@pytest.mark.parametrize("defect", ["owner", "writable", "symlink"])
def test_helper_and_every_parent_must_be_root_owned_and_nonwritable(
    monkeypatch, bad_path, defect
) -> None:
    def info(path):
        mode = (
            stat.S_IFREG | 0o755
            if str(path).endswith("mcsr-toolkit-maccel")
            else stat.S_IFDIR | 0o755
        )
        uid = 0
        if str(path) == bad_path:
            if defect == "owner":
                uid = 1000
            elif defect == "writable":
                mode |= 0o020
            else:
                mode = stat.S_IFLNK | 0o777
        return SimpleNamespace(st_uid=uid, st_mode=mode)

    monkeypatch.setattr(Path, "lstat", info)
    with pytest.raises(sensitivity.SensitivityError):
        sensitivity._trusted_helper()


def test_subprocess_boundary_is_bounded_and_has_no_shell(monkeypatch) -> None:
    calls = []

    def run(args, **kwargs):
        calls.append((args, kwargs))
        return subprocess.CompletedProcess(args, 0, "", "")

    monkeypatch.setattr(subprocess, "run", run)
    sensitivity._run(
        ["/usr/bin/sudo", "-n", "/usr/local/libexec/mcsr-toolkit-maccel", "eye"],
        "a" * 64 + "\n",
    )
    assert calls == [
        (
            ["/usr/bin/sudo", "-n", "/usr/local/libexec/mcsr-toolkit-maccel", "eye"],
            {
                "input": "a" * 64 + "\n",
                "capture_output": True,
                "text": True,
                "timeout": 3,
                "check": False,
            },
        )
    ]


@pytest.mark.parametrize("reply", ["nan", "0", "-1", "garbage", "inf"])
def test_unreadable_multiplier_cannot_enable_eye(adapter, reply) -> None:
    subject, commands = adapter
    subject.startup_restore()
    before = len(commands.calls)

    def invalid(args, token):
        commands.calls.append((args, token))
        return subprocess.CompletedProcess(args, 0, reply, "")

    subject._runner = invalid
    assert not subject.preflight().usable
    assert len(commands.calls) == before + 1
    with pytest.raises(sensitivity.SensitivityError):
        subject.eye()
    assert all(call[0][-1] != "eye" for call in commands.calls[before:])


def test_missing_helper_during_startup_retains_visible_recovery(adapter, monkeypatch) -> None:
    subject, commands = adapter

    def absent():
        raise OSError("helper missing")

    monkeypatch.setattr(sensitivity, "_trusted_helper", absent)
    with pytest.raises(sensitivity.SensitivityError, match="helper missing"):
        subject.startup_restore()
    assert subject.needs_restore
    assert subject.preflight().recovery_pending
    assert all(call[0][-1] != "restore" for call in commands.calls)


def test_allowed_passwd_listing_does_not_prove_noninteractive_execution(adapter) -> None:
    subject, commands = adapter

    def password_required(args, token):
        commands.calls.append((args, token))
        if "-l" in args:
            return subprocess.CompletedProcess(args, 0, f"(root) PASSWD: {args[-2]} {args[-1]}", "")
        if args[-1] == "restore":
            return subprocess.CompletedProcess(args, 1, "", "sudo: a password is required")
        return subprocess.CompletedProcess(args, 0, "0.75", "")

    subject._runner = password_required
    assert not subject.preflight().usable
    with pytest.raises(sensitivity.SensitivityError, match="password is required"):
        subject.startup_restore()
    status = subject.preflight()
    assert status.available and not status.usable and status.recovery_pending
    before = len(commands.calls)
    with pytest.raises(sensitivity.SensitivityError):
        subject.eye()
    assert len(commands.calls) == before


def test_mixed_grants_cannot_report_eye_usable(adapter) -> None:
    subject, commands = adapter
    subject.startup_restore()

    def mixed_grants(args, token):
        commands.calls.append((args, token))
        if "-l" in args:
            return subprocess.CompletedProcess(args, 0, "allowed", "")
        if args[-1] == "eye":
            return subprocess.CompletedProcess(args, 1, "", "sudo: a password is required")
        return subprocess.CompletedProcess(args, 0, "0.75", "")

    subject._runner = mixed_grants
    status = subject.preflight()
    assert not status.usable
    assert "password is required" in status.detail
    assert commands.calls[-1] == (
        ["/usr/bin/sudo", "-n", "/usr/local/libexec/mcsr-toolkit-maccel", "eye"],
        "",
    )
    with pytest.raises(sensitivity.SensitivityError):
        subject.eye()
    assert all(token in (None, "") for _, token in commands.calls)


@pytest.mark.parametrize(
    "code,stdout,stderr",
    [
        (0, "", ""),
        (1, "", "maccel helper must run as root\n"),
        (1, "unexpected", "maccel helper: missing or malformed generation\n"),
        (1, "", "maccel helper: stale generation\n"),
    ],
)
def test_eye_permission_probe_requires_exact_safe_helper_rejection(adapter, code, stdout, stderr):
    subject, commands = adapter
    subject.startup_restore()
    original = subject._runner

    def probe_result(args, token):
        if args[-1] == "eye" and "-l" not in args:
            return subprocess.CompletedProcess(args, code, stdout, stderr)
        return original(args, token)

    subject._runner = probe_result
    assert not subject.preflight().usable


def test_empty_token_probe_rejection_precedes_helper_filesystem_access(monkeypatch, capsys):
    import io

    from mcsr_toolkit import maccel_privileged as helper

    def forbidden(*args, **kwargs):
        raise AssertionError("empty token probe must not touch runtime or driver")

    monkeypatch.setattr(helper.os, "geteuid", lambda: 0)
    monkeypatch.setattr(helper.sys, "stdin", io.StringIO(""))
    monkeypatch.setattr(helper, "_runtime", forbidden)
    monkeypatch.setattr(helper, "_parameter", forbidden)
    assert helper.main(["eye"]) == 1
    captured = capsys.readouterr()
    assert captured.out == ""
    assert captured.err == "maccel helper: missing or malformed generation\n"


@pytest.mark.parametrize("error", [OSError("probe missing"), subprocess.TimeoutExpired("probe", 3)])
def test_eye_probe_transport_failure_disables_eye(adapter, error):
    subject, commands = adapter
    subject.startup_restore()
    original = subject._runner

    def failed_probe(args, token):
        if args[-1] == "eye" and token == "":
            raise error
        return original(args, token)

    subject._runner = failed_probe
    status = subject.preflight()
    assert not status.usable
    assert "eye permission probe failed" in status.detail


class RuntimeCommands(Commands):
    """Model the helper's generation fence and durable baseline at its process boundary."""

    def __init__(self) -> None:
        super().__init__()
        self.raw = 3221225472  # A non-1 baseline: 0.75 in Q32.
        self.baseline: int | None = None
        self.generation = 1
        self.token = f"{self.generation:064x}"
        self.after_eye = lambda: None
        self.writes: list[str] = []

    def __call__(self, args, token):
        if args[1:3] != ["-n", "/usr/local/libexec/mcsr-toolkit-maccel"] or token == "":
            return super().__call__(args, token)
        self.calls.append((args, token))
        if self.failure == args[-1]:
            return subprocess.CompletedProcess(args, 1, "", "restore denied")
        if args[-1] == "restore":
            if self.baseline is not None:
                self.raw = self.baseline
                self.baseline = None
            self.generation += 1
            self.token = f"{self.generation:064x}"
            self.writes.append("restore")
            return subprocess.CompletedProcess(args, 0, self.token + "\n", "")
        if token != self.token + "\n":
            return subprocess.CompletedProcess(args, 1, "", "stale generation")
        if self.baseline is None:
            self.baseline = self.raw
        self.raw = 89464169
        self.writes.append("eye")
        self.after_eye()
        return subprocess.CompletedProcess(args, 0, "", "")


@pytest.fixture
def runtime_case(tmp_path, monkeypatch):
    return make_runtime_case(tmp_path, monkeypatch)


def make_runtime_case(tmp_path, monkeypatch):
    from mcsr_toolkit.__main__ import ServiceRuntime
    from mcsr_toolkit.api import Dependencies
    from mcsr_toolkit.models import Profile
    from mcsr_toolkit.profile import save_profile

    commands = RuntimeCommands()
    monkeypatch.setattr(sensitivity, "_trusted_helper", lambda: None)
    subject = sensitivity.SensitivityAdapter(runner=commands, maccel_binary="/usr/bin/maccel")
    dependencies = Dependencies(
        profile_path=tmp_path / "profile.json", managed_dir=tmp_path,
        state_dir=tmp_path / "state", coordinator_factory=lambda root: None,
    )
    save_profile(dependencies.profile_path, Profile(presets=[
        {"id": "normal", "width": 800, "height": 600, "shortcut": "f6"},
        {"id": "eye", "width": 400, "height": 300, "shortcut": "f7", "eye_zoom": True},
    ]))
    runtime = ServiceRuntime(dependencies, runtime_dir=tmp_path)
    runtime.sensitivity = subject
    runtime.preset.sensitivity = subject
    # Use real FIFO startup and recovery, but keep its worker thread deterministic.
    monkeypatch.setattr(runtime, "_loop", lambda: None)
    dimensions = []

    class Window:
        def resize_focused(self, width, height):
            dimensions.append((width, height))
            return width, height

    runtime.preset.window_bus = Window()
    return runtime, commands, dimensions


class RuntimeFocus:
    def __init__(self):
        self.focused = True
        self.calls = 0

    async def call_get_focus(self):
        self.calls += 1
        return ("Minecraft" if self.focused else "Ptyxis", "Game", 400, 300, False)


def connect_runtime(runtime, interface):
    import time

    runtime.extension_connected = True
    runtime._preset_interface = interface
    runtime.controller.game_active = True
    runtime.pipe.watchdog.last_seen = time.monotonic()


def test_runtime_startup_recovers_orphan_before_accepting_signals(runtime_case, monkeypatch):
    import asyncio

    runtime, commands, _ = runtime_case
    old_token = commands.token
    commands.baseline = commands.raw
    commands.raw = 89464169
    original = runtime.pipe.start

    def start_pipe(**kwargs):
        assert commands.raw == 3221225472
        assert commands.baseline is None
        original(**kwargs)

    monkeypatch.setattr(runtime.pipe, "start", start_pipe)
    runtime.start()
    stale = commands(["/usr/bin/sudo", "-n", "/usr/local/libexec/mcsr-toolkit-maccel", "eye"],
                     old_token + "\n")
    assert stale.returncode == 1
    assert runtime.status()["maccel"]["usable"] is True
    asyncio.run(runtime.close())


@pytest.mark.parametrize("when", ["resize", "helper"])
def test_runtime_focus_loss_around_eye_restores_exact_baseline(runtime_case, when):
    import asyncio

    runtime, commands, dimensions = runtime_case
    runtime.start()
    interface = RuntimeFocus()
    connect_runtime(runtime, interface)
    if when == "resize":
        def resize(width, height):
            dimensions.append((width, height))
            interface.focused = False
            return width, height
        runtime.preset.window_bus.resize_focused = resize
    else:
        commands.after_eye = lambda: setattr(interface, "focused", False)

    async def scenario():
        await runtime._handle_preset_signal(interface, "eye", False)
        assert runtime.preset.active_preset is None
        assert commands.raw == 3221225472
        assert commands.baseline is None
        assert commands.writes == (["restore"] if when == "resize" else
                                   ["restore", "eye", "restore"])
        await runtime.close()

    asyncio.run(scenario())


@pytest.mark.parametrize("reason", ["blur", "modal", "heartbeat", "disconnect", "shutdown"])
def test_runtime_cleanup_paths_restore_eye(runtime_case, reason):
    import asyncio
    import time

    runtime, commands, _ = runtime_case
    runtime.start()
    interface = RuntimeFocus()
    connect_runtime(runtime, interface)

    async def scenario():
        await runtime._handle_preset_signal(interface, "eye", False)
        assert commands.raw == 89464169
        if reason in ("blur", "modal"):
            window_class = "Ptyxis" if reason == "blur" else "Minecraft"
            modal = "true" if reason == "modal" else "false"
            runtime.pipe.watchdog.observe(
                f'{{"class":"{window_class}","title":"Game","modal":{modal}}}',
                time.monotonic(),
            )
        elif reason == "heartbeat":
            runtime.pipe.watchdog.tick(time.monotonic() + 3)
        elif reason == "disconnect":
            await runtime._disconnect_presets()
        else:
            await runtime.close()
        assert commands.raw == 3221225472
        assert commands.baseline is None
        assert runtime.preset.active_preset is None
        if reason != "shutdown":
            await runtime.close()

    asyncio.run(scenario())


def test_runtime_failed_restore_status_and_safe_retry(runtime_case):
    import asyncio

    from fastapi.testclient import TestClient

    from mcsr_toolkit.api import create_app

    runtime, commands, _ = runtime_case
    runtime.start()
    interface = RuntimeFocus()
    connect_runtime(runtime, interface)

    async def scenario():
        await runtime._handle_preset_signal(interface, "eye", False)
        commands.failure = "restore"
        runtime._focus_lost()
        web = TestClient(create_app(runtime.dependencies), base_url="http://127.0.0.1:8765")
        state = web.get("/api/status").json()["focus"]
        assert state["active_preset"] is None
        assert state["maccel"]["recovery_pending"] is True
        assert state["maccel"]["usable"] is False
        assert commands.baseline == 3221225472
        commands.failure = None
        runtime._retry_clear()
        assert commands.raw == 3221225472
        assert runtime.status()["maccel"]["recovery_pending"] is False
        await runtime.close()

    asyncio.run(scenario())


def test_runtime_normal_resize_works_when_helper_absent(runtime_case, monkeypatch):
    import asyncio

    runtime, commands, dimensions = runtime_case

    def absent():
        raise OSError("helper absent")

    monkeypatch.setattr(sensitivity, "_trusted_helper", absent)
    runtime.start()
    interface = RuntimeFocus()
    connect_runtime(runtime, interface)

    async def scenario():
        await runtime._handle_preset_signal(interface, "normal", False)
        assert dimensions == [(800, 600)]
        assert commands.writes == []
        assert runtime.status()["maccel"]["usable"] is False
        await runtime.close()
        assert runtime.errors

    asyncio.run(scenario())


@pytest.mark.parametrize("later", ["normal", "disconnect", "shutdown", "blur_reentry"])
def test_runtime_pending_eye_cannot_write_after_later_action(runtime_case, later):
    import asyncio
    import time

    runtime, commands, dimensions = runtime_case
    runtime.start()

    async def scenario():
        paused, release = asyncio.Event(), asyncio.Event()

        class PausedFocus(RuntimeFocus):
            async def call_get_focus(self):
                if self.calls == 1:
                    paused.set()
                    await release.wait()
                return await super().call_get_focus()

        interface = PausedFocus()
        connect_runtime(runtime, interface)
        eye = runtime._submit_preset_signal(interface, "eye", False)
        await asyncio.wait_for(paused.wait(), 1)
        if later == "normal":
            normal = runtime._submit_preset_signal(interface, "normal", False)
            release.set()
            await asyncio.gather(eye, normal)
            assert dimensions[-1] == (800, 600)
        elif later == "blur_reentry":
            runtime.pipe.watchdog.observe(
                '{"class":"Ptyxis","title":"Terminal","modal":false}', time.monotonic())
            runtime.pipe.watchdog.observe(
                '{"class":"Minecraft","title":"Game","modal":false}', time.monotonic())
            release.set()
            await eye
        else:
            cleanup = runtime.close() if later == "shutdown" else runtime._disconnect_presets()
            await asyncio.wait_for(cleanup, 1)
            release.set()
            assert eye.done()
        assert "eye" not in commands.writes
        assert commands.raw == 3221225472
        assert commands.baseline is None
        if later != "shutdown":
            await runtime.close()

    asyncio.run(scenario())


@pytest.mark.parametrize("later", ["normal", "disconnect", "shutdown"])
def test_runtime_inflight_helper_is_drained_before_later_cleanup(runtime_case, later):
    import asyncio
    import threading
    import time

    runtime, commands, dimensions = runtime_case
    runtime.start()
    entered, release = threading.Event(), threading.Event()

    def pause_helper():
        entered.set()
        assert release.wait(2)

    commands.after_eye = pause_helper

    async def scenario():
        interface = RuntimeFocus()
        connect_runtime(runtime, interface)
        eye = runtime._submit_preset_signal(interface, "eye", False)
        assert await asyncio.to_thread(entered.wait, 1)
        try:
            started = time.monotonic()
            if later == "normal":
                subsequent = runtime._submit_preset_signal(interface, "normal", False)
            else:
                subsequent = asyncio.create_task(
                    runtime.close() if later == "shutdown" else runtime._disconnect_presets()
                )
                await asyncio.sleep(0)
            # Epoch invalidation and cancellation must not block the loop on the helper lock.
            assert time.monotonic() - started < 0.5
            assert not subsequent.done()
            assert commands.baseline == 3221225472
        finally:
            release.set()
        await asyncio.gather(eye, subsequent, return_exceptions=True)
        assert commands.raw == 3221225472
        assert commands.baseline is None
        if later == "normal":
            assert dimensions[-1] == (800, 600)
            assert runtime.preset.active_preset == "normal"
        if later != "shutdown":
            await runtime.close()

    asyncio.run(scenario())


def test_runtime_shutdown_restore_failure_retains_recovery_record(runtime_case):
    import asyncio

    runtime, commands, _ = runtime_case
    runtime.start()
    interface = RuntimeFocus()
    connect_runtime(runtime, interface)

    async def scenario():
        await runtime._handle_preset_signal(interface, "eye", False)
        commands.failure = "restore"
        await runtime.close()
        state = runtime.status()
        assert commands.baseline == 3221225472
        assert commands.raw == 89464169
        assert state["maccel"]["recovery_pending"] is True
        assert state["maccel"]["usable"] is False
        assert state["active_preset"] is None
        assert any("restore denied" in message for message in state["errors"])
        assert not runtime._preset_tasks
        assert runtime.stop_event.is_set()

    asyncio.run(scenario())


def test_runtime_eye_is_rejected_before_startup_recovery(runtime_case):
    import asyncio

    runtime, commands, dimensions = runtime_case
    interface = RuntimeFocus()
    connect_runtime(runtime, interface)
    asyncio.run(runtime._handle_preset_signal(interface, "eye", False))
    assert commands.writes == []
    assert dimensions == []


def test_runtime_shutdown_holds_service_lock_through_final_restore(runtime_case, monkeypatch):
    import asyncio

    runtime, commands, _ = runtime_case
    runtime.start()
    original = runtime.sensitivity.restore
    locked_during_restore = []

    def restore():
        locked_during_restore.append(runtime.lock.fd is not None)
        original()

    monkeypatch.setattr(runtime.sensitivity, "restore", restore)
    asyncio.run(runtime.close())
    assert locked_during_restore == [True]
    assert runtime.lock.fd is None


@pytest.mark.parametrize("focus_call", [0, 1, 2], ids=["initial", "prewrite", "postwrite"])
@pytest.mark.parametrize("reply", ["negative", "exception"])
def test_runtime_stale_eye_focus_reply_preserves_newer_normal(runtime_case, focus_call, reply):
    import asyncio

    runtime, commands, dimensions = runtime_case
    runtime.start()

    async def scenario():
        paused, release = asyncio.Event(), asyncio.Event()

        class DelayedFocus(RuntimeFocus):
            async def call_get_focus(self):
                if self.calls == focus_call:
                    self.calls += 1
                    paused.set()
                    await release.wait()
                    if reply == "exception":
                        raise RuntimeError("older focus request failed")
                    return ("Ptyxis", "Terminal", 400, 300, False)
                return await super().call_get_focus()

        interface = DelayedFocus()
        connect_runtime(runtime, interface)
        eye = runtime._submit_preset_signal(interface, "eye", False)
        await asyncio.wait_for(paused.wait(), 1)
        normal = runtime._submit_preset_signal(interface, "normal", False)
        release.set()
        try:
            await asyncio.gather(eye, normal)
            assert (800, 600) in dimensions
            assert dimensions[-1] == (800, 600)
            assert runtime.preset.active_preset == "normal"
            assert commands.raw == 3221225472
            assert commands.baseline is None
        finally:
            await runtime.close()

    asyncio.run(scenario())


def run_signal_shutdown_probe(tmp_path):
    import asyncio
    import json

    from mcsr_toolkit import __main__ as service
    from mcsr_toolkit.api import ServiceConfig

    monkeypatch = pytest.MonkeyPatch()
    runtime, commands, _ = make_runtime_case(tmp_path, monkeypatch)
    ready = tmp_path / "ready"
    state = tmp_path / "driver.json"
    original_runner = runtime.sensitivity._runner

    def runner(args, token):
        result = original_runner(args, token)
        state.write_text(json.dumps({"raw": commands.raw, "baseline": commands.baseline}))
        return result

    async def watch():
        await asyncio.Event().wait()

    class Server(service.uvicorn.Server):
        async def startup(self, sockets=None):
            await super().startup(sockets)
            interface = RuntimeFocus()
            connect_runtime(runtime, interface)
            await runtime._handle_preset_signal(interface, "eye", False)
            assert commands.raw == 89464169
            ready.touch()

    # Only external boundaries are replaced: real Uvicorn lifespan and signal
    # capture must run, including its fatal SIGTERM replay after shutdown.
    runtime.sensitivity._runner = runner
    monkeypatch.setattr(runtime, "watch_presets", watch)
    monkeypatch.setattr(service, "ServiceRuntime", lambda *args, **kwargs: runtime)
    monkeypatch.setattr(
        service, "default_profile_path", lambda: runtime.dependencies.profile_path,
    )
    monkeypatch.setattr(
        service, "service_config", lambda **kwargs: ServiceConfig("127.0.0.1", 0),
    )
    monkeypatch.setattr(service.uvicorn, "Server", Server)
    monkeypatch.setattr("sys.argv", ["mcsr-toolkit"])
    service.main()


def test_sigterm_restores_eye_before_uvicorn_replays_signal(tmp_path):
    import json
    import multiprocessing
    import os
    import signal
    import time

    ready = tmp_path / "ready"
    state = tmp_path / "driver.json"
    process = multiprocessing.get_context("spawn").Process(
        target=run_signal_shutdown_probe, args=(tmp_path,),
    )
    process.start()
    try:
        deadline = time.monotonic() + 10
        while not ready.exists() and process.is_alive() and time.monotonic() < deadline:
            time.sleep(0.01)
        assert ready.exists(), f"service did not activate fake eye: {process.exitcode}"
        os.kill(process.pid, signal.SIGTERM)
        process.join(10)
        assert process.exitcode == -signal.SIGTERM
        assert json.loads(state.read_text()) == {"raw": 3221225472, "baseline": None}
    finally:
        if process.is_alive():
            process.kill()
        process.join(5)
        process.close()


def test_serve_shutdown_drains_jobs_before_final_restore_and_lock_release(
    runtime_case, monkeypatch,
):
    import asyncio
    import threading

    from mcsr_toolkit import __main__ as service

    runtime, commands, _ = runtime_case
    entered = threading.Event()
    release = threading.Event()
    events = []
    original_restore = runtime.sensitivity.restore
    original_release = runtime.lock.close

    def restore():
        events.append(("restore", runtime.lock.fd is not None))
        return original_restore()

    def release_lock():
        events.append(("release", commands.raw))
        original_release()

    def operation():
        entered.set()
        assert release.wait(5)
        # A queued mutation callback can restore after authorization recovers.
        commands.failure = None
        runtime._focus_lost()
        events.append(("job_done", True))

    async def watch():
        await asyncio.Event().wait()

    class Server:
        def __init__(self, config):
            pass

        async def serve(self):
            interface = RuntimeFocus()
            connect_runtime(runtime, interface)
            await runtime._handle_preset_signal(interface, "eye", False)
            assert commands.raw == 89464169
            monkeypatch.setattr(runtime.sensitivity, "restore", restore)
            commands.failure = "restore"
            queue = runtime.app.state.jobs
            original_close = queue.close

            def close_jobs():
                release.set()
                original_close()

            monkeypatch.setattr(queue, "close", close_jobs)
            queue.submit(operation)
            assert await asyncio.to_thread(entered.wait, 1)

    monkeypatch.setattr(runtime, "watch_presets", watch)
    monkeypatch.setattr(runtime.lock, "close", release_lock)
    monkeypatch.setattr(service, "ServiceRuntime", lambda *args, **kwargs: runtime)
    monkeypatch.setattr(service, "default_profile_path", lambda: runtime.dependencies.profile_path)
    monkeypatch.setattr(service.uvicorn, "Server", Server)
    monkeypatch.setattr("sys.argv", ["mcsr-toolkit"])
    service.main()
    assert all(held for kind, held in events if kind == "restore")
    assert events[-2:] == [("restore", True), ("release", 3221225472)]
    assert events.index(("job_done", True)) < len(events) - 2
    assert commands.baseline is None


def test_complete_serve_shutdown_finishes_watcher_before_releasing_lock(runtime_case, monkeypatch):
    import asyncio
    import time

    import dbus_next.aio

    from mcsr_toolkit import __main__ as service

    runtime, commands, _ = runtime_case
    operations_held_lock = []
    original_restore = runtime.sensitivity.restore

    def restore():
        operations_held_lock.append(runtime.lock.fd is not None)
        return original_restore()

    original_release = runtime.lock.close

    def release_lock():
        original_release()
        # Model authorization recovering after the failed shutdown restore.
        commands.failure = None

    monkeypatch.setattr(runtime.lock, "close", release_lock)

    class Interface(RuntimeFocus):
        def on_preset_activated(self, handler):
            pass

        def off_preset_activated(self, handler):
            pass

    class Bus:
        connected = True

        def __init__(self):
            self.closed = asyncio.Event()
            self.interface = Interface()

        async def connect(self):
            return self

        async def introspect(self, name, path):
            return None

        def get_proxy_object(self, name, path, introspection):
            return self

        def get_interface(self, name):
            return self.interface

        async def wait_for_disconnect(self):
            await self.closed.wait()

        async def call(self, message):
            from dbus_next import Message, MessageType

            return Message(message_type=MessageType.METHOD_RETURN, reply_serial=1)

        def disconnect(self):
            self.connected = False
            self.closed.set()

    class Server:
        def __init__(self, config):
            pass

        async def serve(self):
            while not runtime.extension_connected:
                await asyncio.sleep(0)
            runtime.controller.game_active = True
            runtime.pipe.watchdog.last_seen = time.monotonic()
            await runtime._handle_preset_signal(runtime._preset_interface, "eye", False)
            assert commands.baseline == 3221225472
            monkeypatch.setattr(runtime.sensitivity, "restore", restore)
            commands.failure = "restore"

    monkeypatch.setattr(dbus_next.aio, "MessageBus", Bus)
    monkeypatch.setattr(service, "ServiceRuntime", lambda *args, **kwargs: runtime)
    monkeypatch.setattr(service, "default_profile_path", lambda: runtime.dependencies.profile_path)
    monkeypatch.setattr(service.uvicorn, "Server", Server)
    monkeypatch.setattr("sys.argv", ["mcsr-toolkit"])
    monkeypatch.setenv("XDG_RUNTIME_DIR", str(runtime.pipe.path.parent))
    service.main()
    assert operations_held_lock
    assert all(operations_held_lock)
    assert runtime.lock.fd is None
    assert commands.baseline == 3221225472
    assert runtime.sensitivity.recovery_pending


@pytest.mark.parametrize("boundary", ["configure", "health_poll"])
def test_shutdown_drains_focus_thread_before_final_restore(runtime_case, monkeypatch, boundary):
    import asyncio
    import threading

    from mcsr_toolkit import __main__ as service

    runtime, commands, _ = runtime_case
    entered, release = threading.Event(), threading.Event()
    late_steps = []

    def blocked(*args):
        entered.set()
        assert release.wait(2)

    def pipe_step(timeout):
        if runtime.stop_event.is_set():
            late_steps.append("pipe")

    def reconcile():
        if runtime.stop_event.is_set():
            late_steps.append("reconcile")

    monkeypatch.setattr(runtime, "_loop", lambda: service.ServiceRuntime._loop(runtime))
    monkeypatch.setattr(runtime.pipe, "run_once", pipe_step)
    monkeypatch.setattr(runtime, "_reconcile", reconcile)
    if boundary == "configure":
        runtime.extension_connected = True
        monkeypatch.setattr(service.GnomeWindowBus, "configure_presets", blocked)
    else:
        monkeypatch.setattr(runtime, "_poll_keyd_health", blocked)
    runtime.start()
    assert entered.wait(1)
    original_join = runtime.thread.join

    def join(timeout=None):
        # Deterministically model the current short deadline expiring at this boundary.
        original_join(0 if timeout == 3 else timeout)

    monkeypatch.setattr(runtime.thread, "join", join)

    async def scenario():
        closing = asyncio.create_task(runtime.close())
        try:
            await asyncio.sleep(0.02)
            assert not closing.done()
            assert runtime.lock.fd is not None
        finally:
            release.set()
            await closing
            await asyncio.to_thread(original_join)
        assert not runtime.thread.is_alive()
        assert late_steps == []
        assert runtime.lock.fd is None

    asyncio.run(scenario())
