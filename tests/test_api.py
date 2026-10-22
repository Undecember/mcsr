from __future__ import annotations

import asyncio
import json
import time
from pathlib import Path
from threading import Event

import pytest
from fastapi.testclient import TestClient

from mcsr_toolkit.api import Dependencies, create_app, service_config
from mcsr_toolkit.apply import ApplyCoordinator
from mcsr_toolkit.models import Profile
from mcsr_toolkit.profile import save_profile


@pytest.fixture(autouse=True)
def no_installed_sensitivity_helper(monkeypatch: pytest.MonkeyPatch) -> None:
    # Runtime startup/status tests must never invoke a machine's installed sudo helper.
    from mcsr_toolkit import sensitivity

    def absent() -> None:
        raise sensitivity.SensitivityError("test helper absent")

    monkeypatch.setattr(sensitivity, "_trusted_helper", absent)


class KeydAdapter:
    def __init__(self) -> None:
        self.installs = 0
        self.restores = 0
        self.history: list[str] = []

    def install(self, profile: Profile) -> None:
        self.installs += 1
        self.history.append(f"{self.installs:032x}")

    def rollback(self) -> None:
        if self.history:
            self.history.pop()

    def checkpoint(self) -> list[str]:
        return list(self.history)

    def restore_to(self, checkpoint: list[str]) -> None:
        if self.history != checkpoint:
            self.restores += 1
            self.history = list(checkpoint)

    def restore(self) -> None:
        self.restores += 1


def instance(root: Path) -> Path:
    game = root / "instances" / "one" / "minecraft"
    game.mkdir(parents=True)
    (game.parent / "instance.json").write_text(json.dumps({"id": "one"}))
    target = game / "options.txt"
    target.write_text("key_key.back:key.keyboard.s\n")
    return target


def client(
    tmp_path: Path, *, keyd: KeydAdapter | None = None
) -> tuple[TestClient, Path, KeydAdapter]:
    root = tmp_path / "launcher"
    target = instance(root)
    adapter = keyd or KeydAdapter()
    dependencies = Dependencies(
        profile_path=tmp_path / "profile.json",
        managed_dir=tmp_path,
        state_dir=tmp_path / "state",
        coordinator_factory=lambda path: ApplyCoordinator(
            path, tmp_path / "state", adapter, lambda: []
        ),
        download=lambda product, destination: {
            "version": "v1",
            "path": str(destination),
            "byte_count": 12,
        },
        session_token="test-secret",
        live_activation_enabled=True,
    )
    return TestClient(create_app(dependencies), base_url="http://127.0.0.1:8765"), target, adapter


def write_headers() -> dict[str, str]:
    return {"origin": "http://127.0.0.1:8765", "x-mcsr-session": "test-secret"}


def job_result(client: TestClient, job_id: str) -> dict[str, object]:
    for _ in range(100):
        result = client.get(f"/api/jobs/{job_id}").json()
        if result["status"] != "running":
            return result
        time.sleep(0.01)
    raise AssertionError("job did not complete")


def test_service_config_only_binds_loopback() -> None:
    assert service_config(port=8765).host == "127.0.0.1"
    assert service_config(port=8765).port == 8765


def test_profile_preview_apply_and_restore(tmp_path: Path) -> None:
    web, target, keyd = client(tmp_path)
    profile = {"crafting": {"s": "k"}, "launcher_root": str(tmp_path / "launcher")}
    assert web.put("/api/profile", json=profile, headers=write_headers()).status_code == 200
    assert web.get("/api/profile").json()["crafting"] == {"s": "k"}
    preview = web.post("/api/preview", json=profile, headers=write_headers())
    assert preview.status_code == 200
    assert preview.json()["instances"] == [{"instance_id": "one", "status": "ready", "detail": ""}]
    accepted = web.post("/api/apply", json=web.get("/api/profile").json(), headers=write_headers())
    assert accepted.status_code == 202
    applied = job_result(web, accepted.json()["id"])
    assert applied["status"] == "succeeded"
    assert applied["result"]["instances"][0]["status"] == "updated"
    assert target.read_text() == "key_key.back:key.keyboard.k\n"
    assert keyd.installs == 1
    backup = applied["result"]["backup_id"]
    restore = web.post("/api/restore", json={"backup_id": backup}, headers=write_headers())
    assert restore.status_code == 202
    assert job_result(web, restore.json()["id"])["status"] == "succeeded"
    assert target.read_text() == "key_key.back:key.keyboard.s\n"
    assert keyd.restores == 1


def test_apply_rejects_stale_profile_from_another_tab(tmp_path: Path) -> None:
    web, _, keyd = client(tmp_path)
    root = str(tmp_path / "launcher")
    tab_a = web.put(
        "/api/profile", json={"crafting": {"s": "k"}, "launcher_root": root},
        headers=write_headers(),
    ).json()
    tab_b = web.put(
        "/api/profile", json={"crafting": {"s": "x"}, "launcher_root": root},
        headers=write_headers(),
    ).json()

    stale = web.post("/api/apply", json=tab_a, headers=write_headers())

    assert stale.status_code == 409
    assert stale.json()["detail"]["code"] == "stale_profile"
    assert web.get("/api/profile").json() == tab_b
    assert keyd.installs == 0


def test_preview_exposes_per_instance_control_mappings_without_mutating_state(
    tmp_path: Path,
) -> None:
    web, target, keyd = client(tmp_path)
    response = web.post(
        "/api/preview",
        json={"crafting": {"s": "k"}, "launcher_root": str(tmp_path / "launcher")},
        headers=write_headers(),
    )

    assert response.status_code == 200
    assert response.json()["instances"] == [
        {"instance_id": "one", "status": "ready", "detail": ""}
    ]
    assert response.json()["mappings"] == [
        {
            "instance_id": "one",
            "physical": "s",
            "emitted": "k",
            "action": "key_key.back",
        }
    ]
    assert target.read_text(encoding="utf-8") == "key_key.back:key.keyboard.s\n"
    assert not (tmp_path / "state").exists()
    assert keyd.installs == 0


def test_security_rejects_off_origin_bad_host_and_missing_token(tmp_path: Path) -> None:
    web, _, _ = client(tmp_path)
    assert (
        web.put(
            "/api/profile",
            json={},
            headers={"origin": "http://evil.test", "x-mcsr-session": "test-secret"},
        ).status_code
        == 403
    )
    assert (
        web.put("/api/profile", json={}, headers={"origin": "http://127.0.0.1:8765"}).status_code
        == 403
    )
    assert web.get("/api/status", headers={"host": "evil.test"}).status_code == 403
    assert (
        web.put(
            "/api/profile", json={}, headers={**write_headers(), "host": "evil.test"}
        ).status_code
        == 403
    )


def test_rejects_invalid_launcher_root_and_command_data(tmp_path: Path) -> None:
    web, _, _ = client(tmp_path)
    invalid = web.put(
        "/api/profile", json={"launcher_root": str(tmp_path)}, headers=write_headers()
    )
    assert invalid.status_code == 422
    command = web.put("/api/profile", json={"command": "rm -rf /"}, headers=write_headers())
    assert command.status_code == 422
    unsafe = web.put("/api/profile", json={"crafting": {"s": "k; id"}}, headers=write_headers())
    assert unsafe.status_code == 422
    assert web.post("/api/download/../../etc/passwd", headers=write_headers()).status_code in (
        404,
        422,
    )


def test_concurrent_apply_is_rejected_and_failure_is_reported(tmp_path: Path) -> None:
    entered, release = Event(), Event()
    web, _, _ = client(tmp_path)
    original = web.app.state.dependencies.coordinator_factory

    class BlockingCoordinator:
        def apply(self, profile: Profile, **kwargs: object) -> object:
            entered.set()
            release.wait(5)
            raise RuntimeError("keyd reload failed")

    web.app.state.dependencies.coordinator_factory = lambda root: BlockingCoordinator()
    assert (
        web.put(
            "/api/profile",
            json={"launcher_root": str(tmp_path / "launcher")},
            headers=write_headers(),
        ).status_code
        == 200
    )
    profile = web.get("/api/profile").json()
    first = web.post("/api/apply", json=profile, headers=write_headers())
    assert first.status_code == 202
    assert entered.wait(2)
    assert web.post("/api/apply", json=profile, headers=write_headers()).status_code == 409
    changed = {"crafting": {"s": "x"}, "launcher_root": str(tmp_path / "launcher")}
    blocked = web.put("/api/profile", json=changed, headers=write_headers())
    assert blocked.status_code == 409
    assert blocked.json()["detail"]["code"] == "job_busy"
    assert web.get("/api/profile").json() == profile
    release.set()
    failed = job_result(web, first.json()["id"])
    assert failed["status"] == "failed"
    assert "keyd reload failed" in failed["error"]
    web.app.state.dependencies.coordinator_factory = original


def test_observe_only_status_and_restore_remains_available(tmp_path: Path) -> None:
    web, _, _ = client(tmp_path)
    web.app.state.dependencies.live_activation_enabled = False
    status = web.get("/api/status").json()
    assert status["live_activation_enabled"] is False
    profile = web.get("/api/profile").json()
    assert web.post("/api/apply", json=profile, headers=write_headers()).status_code == 403
    assert (
        web.post("/api/restore", json={"backup_id": "a" * 32}, headers=write_headers()).status_code
        == 202
    )


def test_download_is_job_and_product_is_allowlisted(tmp_path: Path) -> None:
    web, _, _ = client(tmp_path)
    accepted = web.post("/api/download/ninjabrain", headers=write_headers())
    assert accepted.status_code == 202
    result = job_result(web, accepted.json()["id"])
    assert result["result"]["version"] == "v1"
    assert result["result"]["path"].endswith("Ninjabrain-Bot.jar")
    assert web.post("/api/download/evil", headers=write_headers()).status_code == 422


def test_get_session_rejects_cross_origin_bootstrap(tmp_path: Path) -> None:
    web, _, _ = client(tmp_path)
    assert web.get("/api/session", headers={"origin": "http://evil.test"}).status_code == 403
    assert web.get("/api/session").json() == {"token": "test-secret"}


def test_rejects_too_small_resize_preset(tmp_path: Path) -> None:
    web, _, _ = client(tmp_path)
    response = web.put(
        "/api/profile",
        json={"presets": [{"id": "tiny", "width": 99, "height": 600, "shortcut": "f1"}]},
        headers=write_headers(),
    )
    assert response.status_code == 422


def test_live_reconciliation_waits_for_applied_or_pending_profile(tmp_path: Path) -> None:
    from mcsr_toolkit.__main__ import ServiceRuntime

    web, _, _ = client(tmp_path)
    dependencies = web.app.state.dependencies
    calls: list[Path] = []
    original = dependencies.coordinator_factory

    def factory(root: Path) -> ApplyCoordinator:
        calls.append(root)
        return original(root)

    dependencies.coordinator_factory = factory
    runtime = ServiceRuntime(dependencies, runtime_dir=tmp_path)
    runtime.app = web.app
    runtime._reconcile()
    assert calls == []


def test_restore_preparation_fails_when_keyd_clear_fails(
    tmp_path: Path, monkeypatch: pytest.MonkeyPatch,
) -> None:
    from mcsr_toolkit.__main__ import ServiceRuntime
    from mcsr_toolkit.keyd import KeydCommands

    web, _, _ = client(tmp_path)
    runtime = ServiceRuntime(web.app.state.dependencies, runtime_dir=tmp_path)
    profile = Profile(launcher_root=tmp_path / "launcher")
    runtime._applied(profile)

    def fail_clear(self: KeydCommands) -> None:
        raise RuntimeError("keyd daemon unavailable")

    monkeypatch.setattr(KeydCommands, 'clear_state', fail_clear)
    with pytest.raises(RuntimeError, match="keyd daemon unavailable"):
        runtime._before_restore()
    assert "keyd daemon unavailable" in runtime.errors
    assert runtime.keyd.cleanup_error == "keyd daemon unavailable"
    assert runtime.keyd.profile == profile
    assert runtime.keyd.active_profile.is_file()
    assert runtime.keyd.suspended_path.is_file()


def test_reconciliation_uses_pending_profile_after_saved_profile_changes(tmp_path: Path) -> None:
    from mcsr_toolkit.__main__ import ServiceRuntime

    web, target, _ = client(tmp_path)
    dependencies = web.app.state.dependencies
    dependencies.state_dir.mkdir()
    (dependencies.state_dir / "pending.json").write_text(
        Profile(crafting={"s": "k"}, launcher_root=tmp_path / "launcher").model_dump_json()
    )
    dependencies.profile_path.write_text(
        Profile(crafting={"s": "x"}, launcher_root=tmp_path / "launcher").model_dump_json()
    )
    runtime = ServiceRuntime(dependencies, runtime_dir=tmp_path)
    runtime.app = web.app
    runtime._reconcile()
    for _ in range(100):
        if runtime.keyd.profile is not None:
            break
        time.sleep(0.01)
    assert target.read_text() == "key_key.back:key.keyboard.k\n"
    assert runtime.keyd.profile is not None
    assert runtime.keyd.profile.crafting == {"s": "k"}


def test_rejects_invalid_launcher_override_with_nul_or_symlinked_instances(tmp_path: Path) -> None:
    web, _, _ = client(tmp_path)
    nul = web.put(
        "/api/profile",
        json={"launcher_root": str(tmp_path / "launcher") + "\0"},
        headers=write_headers(),
    )
    assert nul.status_code == 422
    linked = tmp_path / "linked-root"
    linked.mkdir()
    (linked / "instances").symlink_to(tmp_path / "launcher" / "instances", target_is_directory=True)
    unsafe = web.put("/api/profile", json={"launcher_root": str(linked)}, headers=write_headers())
    assert unsafe.status_code == 422


def test_corrupt_pending_profile_does_not_stop_focus_service(tmp_path: Path) -> None:
    from mcsr_toolkit.__main__ import ServiceRuntime

    web, _, _ = client(tmp_path)
    dependencies = web.app.state.dependencies
    dependencies.state_dir.mkdir()
    (dependencies.state_dir / "pending.json").write_text("{bad json")
    runtime = ServiceRuntime(dependencies, runtime_dir=tmp_path)
    runtime.app = web.app
    runtime._reconcile()
    assert runtime.errors


def test_preview_discovers_running_launcher_working_directory(tmp_path: Path) -> None:
    from mcsr_toolkit.instances import ProcessInfo

    web, _, _ = client(tmp_path)
    dependencies = web.app.state.dependencies
    dependencies.managed_dir = tmp_path / "not-managed"
    dependencies.processes = lambda: [ProcessInfo(tmp_path)]
    response = web.post("/api/preview", json={"crafting": {"s": "k"}}, headers=write_headers())
    assert response.status_code == 200
    assert response.json()["instances"][0]["status"] == "ready"


def test_origin_must_match_exact_local_host(tmp_path: Path) -> None:
    web, _, _ = client(tmp_path)
    response = web.put(
        "/api/profile",
        json={},
        headers={
            "host": "127.0.0.1:8765",
            "origin": "http://localhost:8765",
            "x-mcsr-session": "test-secret",
        },
    )
    assert response.status_code == 403


def test_service_lock_prevents_two_fifo_consumers(tmp_path: Path) -> None:
    from mcsr_toolkit.__main__ import ServiceLock

    first = ServiceLock(tmp_path / "service.lock")
    second = ServiceLock(tmp_path / "service.lock")
    first.acquire()
    try:
        with pytest.raises(RuntimeError, match="already running"):
            second.acquire()
    finally:
        first.close()
    second.acquire()
    second.close()


def test_preset_signal_restores_zoom_if_focus_lost_during_activation(tmp_path: Path) -> None:
    import asyncio

    from mcsr_toolkit.__main__ import ServiceRuntime
    from mcsr_toolkit.resize import PresetController

    web, _, _ = client(tmp_path)
    dependencies = web.app.state.dependencies
    save_profile(
        dependencies.profile_path,
        Profile(
            presets=[{"id": "eye", "width": 800, "height": 600, "shortcut": "f7", "eye_zoom": True}]
        ),
    )
    runtime = ServiceRuntime(dependencies, runtime_dir=tmp_path)
    runtime.controller.game_active = True
    runtime.pipe.watchdog.last_seen = time.monotonic()
    commands: list[str] = []

    class Window:
        def resize_focused(self, width: int, height: int) -> tuple[int, int]:
            return width, height

    class Sensitivity:
        needs_restore = False

        def eye(self) -> None:
            commands.append("eye")
            self.needs_restore = True

        def restore(self) -> None:
            commands.append("restore")
            self.needs_restore = False

    class Bus:
        calls = 0

        async def call_get_focus(self) -> tuple[str, str, int, int, bool]:
            self.calls += 1
            if self.calls <= 2:
                return ("Minecraft", "Game", 800, 600, False)
            return ("Ptyxis", "Terminal", 800, 600, False)

    interface = Bus()
    runtime._accept_presets = True
    runtime.extension_connected = True
    runtime._preset_interface = interface
    runtime.preset = PresetController(Window(), sensitivity=Sensitivity())
    asyncio.run(runtime._handle_preset_signal(interface, "eye", False))
    assert runtime.preset.active_preset is None
    assert commands == ["eye", "restore"]


def test_preset_signal_requires_focus_bridge_to_see_minecraft(tmp_path: Path) -> None:
    import asyncio

    from mcsr_toolkit.__main__ import ServiceRuntime

    web, _, _ = client(tmp_path)
    runtime = ServiceRuntime(web.app.state.dependencies, runtime_dir=tmp_path)

    class Bus:
        async def call_get_focus(self) -> tuple[str, str, int, int, bool]:
            return ("Minecraft", "Game", 800, 600, False)

    asyncio.run(runtime._handle_preset_signal(Bus(), "unknown", False))
    assert runtime.preset.active_preset is None


def test_preset_signal_clears_state_if_postcheck_bus_fails(tmp_path: Path) -> None:
    import asyncio

    from mcsr_toolkit.__main__ import ServiceRuntime
    from mcsr_toolkit.resize import PresetController

    web, _, _ = client(tmp_path)
    dependencies = web.app.state.dependencies
    save_profile(
        dependencies.profile_path,
        Profile(presets=[{'id': 'normal', 'width': 800, 'height': 600,
                          'shortcut': 'f8'}]),
    )
    runtime = ServiceRuntime(dependencies, runtime_dir=tmp_path)
    runtime.controller.game_active = True
    runtime.pipe.watchdog.last_seen = time.monotonic()

    class Window:
        def resize_focused(self, width: int, height: int) -> tuple[int, int]:
            return width, height

    class Bus:
        calls = 0

        async def call_get_focus(self) -> tuple[str, str, int, int, bool]:
            self.calls += 1
            if self.calls == 2:
                raise RuntimeError('GNOME bus disconnected')
            return ('Minecraft', 'Game', 800, 600, False)

    interface = Bus()
    runtime._accept_presets = True
    runtime.extension_connected = True
    runtime._preset_interface = interface
    runtime.preset = PresetController(Window())
    asyncio.run(runtime._handle_preset_signal(interface, 'normal', False))
    assert runtime.preset.active_preset is None
    assert 'GNOME bus disconnected' in runtime.errors


def test_extension_status_requires_current_fifo_heartbeat(tmp_path: Path) -> None:
    from mcsr_toolkit.__main__ import ServiceRuntime

    web, _, _ = client(tmp_path)
    runtime = ServiceRuntime(web.app.state.dependencies, runtime_dir=tmp_path)
    runtime.extension_connected = True
    assert web.get('/api/status').json()['focus']['extension_connected'] is False
    runtime.pipe.watchdog.last_seen = time.monotonic()
    assert web.get('/api/status').json()['focus']['extension_connected'] is True
    runtime.pipe.watchdog.last_seen -= runtime.pipe.watchdog.timeout
    assert web.get('/api/status').json()['focus']['extension_connected'] is False


def test_extension_object_removal_reconnects_and_reconfigures(
    tmp_path: Path, monkeypatch: pytest.MonkeyPatch,
) -> None:
    import asyncio
    import inspect

    import dbus_next.aio

    from mcsr_toolkit.__main__ import ServiceRuntime

    web, _, _ = client(tmp_path)
    runtime = ServiceRuntime(web.app.state.dependencies, runtime_dir=tmp_path)
    runtime.pipe.watchdog.last_seen = time.monotonic()
    runtime.controller.game_active = True
    connections: list[object] = []
    handlers: list[object] = []
    status_on_reconnect: list[bool] = []

    class Interface:
        def __init__(self, generation: int) -> None:
            self.generation = generation
            self.calls = 0

        def on_preset_activated(self, handler: object) -> None:
            handlers.append(handler)
            assert list(inspect.signature(handler).parameters) == ['preset_id', 'shifted']

        def off_preset_activated(self, handler: object) -> None:
            assert handler in handlers
            cleanup.append('off')

        async def call_get_focus(self) -> tuple[str, str, int, int, bool]:
            self.calls += 1
            if self.generation == 1 and self.calls == 2:
                runtime._next_configure = 999999999.0
                raise RuntimeError('UnknownMethod: extension object removed')
            if self.generation == 2:
                status_on_reconnect.append(runtime.extension_connected)
                runtime.stop_event.set()
            return ('Minecraft', 'Game', 800, 600, False)

    class Bus:
        disconnected = False
        connected = True

        def __init__(self) -> None:
            self.closed = asyncio.Event()

        async def connect(self) -> Bus:
            connections.append(self)
            self.interface = Interface(len(connections))
            return self

        async def introspect(self, name: str, path: str) -> None:
            pass

        def get_proxy_object(self, name: str, path: str, introspection: object) -> Bus:
            return self

        def get_interface(self, name: str) -> Interface:
            return self.interface

        async def wait_for_disconnect(self) -> None:
            await self.closed.wait()

        async def call(self, message: object) -> object:
            from dbus_next import Message, MessageType

            cleanup.append('ping')
            return Message(message_type=MessageType.METHOD_RETURN, reply_serial=1)

        def disconnect(self) -> None:
            cleanup.append('disconnect')
            self.disconnected = True
            self.closed.set()

    monkeypatch.setattr(dbus_next.aio, 'MessageBus', Bus)
    cleanup: list[str] = []
    monkeypatch.setattr(runtime, '_extension_poll_interval', 0.001)
    monkeypatch.setattr(runtime, '_extension_retry_interval', 0.001)
    asyncio.run(runtime.watch_presets())
    assert len(connections) == 2
    assert all(bus.disconnected for bus in connections)
    assert status_on_reconnect == [False]
    assert runtime._next_configure == 0.0
    assert runtime.extension_connected is False
    assert runtime.controller.game_active is False
    assert len(handlers) == 2
    assert runtime.errors == ['UnknownMethod: extension object removed']
    assert cleanup == ['off', 'ping', 'disconnect'] * 2


def test_repeated_bridge_error_is_logged_once(
    tmp_path: Path, caplog: pytest.LogCaptureFixture,
) -> None:
    from mcsr_toolkit.__main__ import ServiceRuntime

    web, _, _ = client(tmp_path)
    runtime = ServiceRuntime(web.app.state.dependencies, runtime_dir=tmp_path)
    for _ in range(3):
        runtime._record_error(RuntimeError('extension unavailable'))
    assert runtime.errors == ['extension unavailable']
    assert caplog.text.count('extension unavailable') == 1


def test_restore_cancels_pending_reconciliation(
    tmp_path: Path, monkeypatch: pytest.MonkeyPatch,
) -> None:
    from mcsr_toolkit.__main__ import ServiceRuntime
    from mcsr_toolkit.keyd import KeydCommands

    web, _, adapter = client(tmp_path)
    dependencies = web.app.state.dependencies
    profile = Profile(crafting={'s': 'k'}, launcher_root=tmp_path / 'launcher')
    save_profile(dependencies.profile_path, profile)
    report = dependencies.coordinator_factory(tmp_path / 'launcher').apply(profile)
    runtime = ServiceRuntime(dependencies, runtime_dir=tmp_path)
    runtime.app = web.app
    runtime._applied(profile)
    monkeypatch.setattr(KeydCommands, 'clear_state', lambda self: None)
    save_profile(dependencies.state_dir / 'pending.json', profile)
    accepted = web.post(
        '/api/restore', json={'backup_id': report.backup_id}, headers=write_headers(),
    )
    assert job_result(web, accepted.json()['id'])['status'] == 'succeeded'
    assert not (dependencies.state_dir / 'pending.json').exists()
    runtime._reconcile()
    assert adapter.installs == 1


@pytest.mark.parametrize('reason', ['unknown', 'wrong_root'])
def test_rejected_restore_preserves_active_profile(tmp_path: Path, reason: str) -> None:
    from mcsr_toolkit.__main__ import ServiceRuntime

    web, _, _ = client(tmp_path)
    dependencies = web.app.state.dependencies
    profile = Profile(crafting={'s': 'k'}, launcher_root=tmp_path / 'launcher')
    save_profile(dependencies.profile_path, profile)
    runtime = ServiceRuntime(dependencies, runtime_dir=tmp_path)
    runtime._applied(profile)
    backup_id = 'unknown'
    if reason == 'wrong_root':
        report = dependencies.coordinator_factory(tmp_path / 'launcher').apply(profile)
        assert report.backup_id is not None
        backup_id = report.backup_id
        other_root = tmp_path / 'other-launcher'
        instance(other_root)
        save_profile(dependencies.profile_path, Profile(launcher_root=other_root))
    accepted = web.post('/api/restore', json={'backup_id': backup_id}, headers=write_headers())
    assert job_result(web, accepted.json()['id'])['status'] == 'failed'
    assert runtime.keyd.profile == profile
    assert runtime.keyd.active_profile.is_file()


@pytest.mark.parametrize('focus', [('Ptyxis', 'Terminal', False), ('Minecraft', 'Game', True)])
def test_live_activation_rechecks_effective_focus(
    tmp_path: Path, monkeypatch: pytest.MonkeyPatch, focus: tuple[str, str, bool],
) -> None:
    from mcsr_toolkit.__main__ import ServiceRuntime
    from mcsr_toolkit.focus import FocusEvent
    from mcsr_toolkit.keyd import KeydCommands
    from mcsr_toolkit.resize import GnomeWindowBus

    web, _, _ = client(tmp_path)
    runtime = ServiceRuntime(web.app.state.dependencies, runtime_dir=tmp_path)
    runtime._applied(Profile())
    calls: list[str] = []
    monkeypatch.setattr(GnomeWindowBus, 'get_focus', lambda self: FocusEvent(*focus))
    monkeypatch.setattr(KeydCommands, 'activate_game', lambda self: calls.append('activate'))
    with pytest.raises(RuntimeError, match='effective Minecraft focus'):
        runtime.controller.on_focus(FocusEvent('Minecraft', 'Queued game'))
    assert calls == []
    assert runtime.controller.game_active is False


@pytest.mark.parametrize('outcome', ['failed', 'deferred'])
def test_reconciliation_failure_suspends_live_activation_and_reports_status(
    tmp_path: Path, monkeypatch: pytest.MonkeyPatch, outcome: str,
) -> None:
    from mcsr_toolkit.__main__ import ServiceRuntime
    from mcsr_toolkit.apply import ApplyReport, InstanceResult
    from mcsr_toolkit.keyd import KeydCommands

    web, _, _ = client(tmp_path)
    dependencies = web.app.state.dependencies
    profile = Profile(launcher_root=tmp_path / 'launcher')
    runtime = ServiceRuntime(dependencies, runtime_dir=tmp_path)
    runtime.app = web.app
    runtime._applied(profile)
    runtime.controller.game_active = True
    clears: list[str] = []
    original_reconcile = ApplyCoordinator.reconcile_new_instances
    monkeypatch.setattr(KeydCommands, 'clear_state', lambda self: clears.append('clear'))
    monkeypatch.setattr(
        ApplyCoordinator, 'reconcile_new_instances',
        lambda self, profile, **kwargs: ApplyReport([
            InstanceResult('new', outcome, 'new instance blocked')
        ]),
    )
    runtime._reconcile()
    for _ in range(100):
        if runtime.keyd.profile is None and (dependencies.state_dir / 'pending.json').is_file():
            break
        time.sleep(0.01)
    assert runtime.keyd.profile is None
    assert runtime.controller.game_active is False
    assert clears
    assert (dependencies.state_dir / 'pending.json').is_file()
    assert not runtime.keyd.active_profile.exists()
    status = web.get('/api/status').json()['focus']
    assert status['reconciliation']['instances'][0]['status'] == outcome
    monkeypatch.setattr(ApplyCoordinator, 'reconcile_new_instances', original_reconcile)
    runtime._reconcile()
    for _ in range(100):
        if runtime.keyd.profile is not None:
            break
        time.sleep(0.01)
    assert runtime.keyd.profile == profile
    assert not (dependencies.state_dir / 'pending.json').exists()
    assert runtime.keyd.active_profile.is_file()


def test_live_activation_allows_fresh_effective_minecraft_focus(
    tmp_path: Path, monkeypatch: pytest.MonkeyPatch,
) -> None:
    from mcsr_toolkit.__main__ import ServiceRuntime
    from mcsr_toolkit.focus import FocusEvent
    from mcsr_toolkit.keyd import KeydCommands
    from mcsr_toolkit.resize import GnomeWindowBus

    web, _, _ = client(tmp_path)
    runtime = ServiceRuntime(web.app.state.dependencies, runtime_dir=tmp_path)
    runtime._applied(Profile())
    calls: list[str] = []
    monkeypatch.setattr(
        GnomeWindowBus, 'get_focus', lambda self: FocusEvent('Minecraft', 'Current game'),
    )
    monkeypatch.setattr(KeydCommands, 'activate_game', lambda self: calls.append('activate'))
    runtime.controller.on_focus(FocusEvent('Minecraft', 'FIFO game'))
    assert calls == ['activate']
    assert runtime.controller.game_active is True


def test_restore_serializes_with_pending_cleanup(
    tmp_path: Path, monkeypatch: pytest.MonkeyPatch,
) -> None:
    from threading import Thread

    from mcsr_toolkit.__main__ import ServiceRuntime
    from mcsr_toolkit.keyd import KeydCommands

    web, _, _ = client(tmp_path)
    runtime = ServiceRuntime(web.app.state.dependencies, runtime_dir=tmp_path)
    runtime._applied(Profile())
    attempted, acquired = Event(), Event()
    observed: list[Profile | None] = []

    def focus_attempt() -> None:
        attempted.set()
        with runtime._preset_lock:
            observed.append(runtime.keyd.profile)
            acquired.set()

    thread = Thread(target=focus_attempt)
    restoring = runtime.keyd.restoring
    monkeypatch.setattr(KeydCommands, 'clear_state', lambda self: None)

    def clear() -> None:
        thread.start()
        assert attempted.wait(1)
        assert not acquired.wait(0.05)
        restoring()

    runtime.keyd.restoring = clear
    runtime._before_restore()
    thread.join(timeout=1)
    assert not thread.is_alive()
    assert observed == [None]


def test_failed_clear_retries_after_service_restart(
    tmp_path: Path, monkeypatch: pytest.MonkeyPatch,
) -> None:
    from mcsr_toolkit.__main__ import ServiceRuntime
    from mcsr_toolkit.focus import FocusEvent
    from mcsr_toolkit.keyd import KeydCommands

    web, _, _ = client(tmp_path)
    dependencies = web.app.state.dependencies
    runtime = ServiceRuntime(dependencies, runtime_dir=tmp_path)
    profile = Profile()
    runtime._applied(profile)
    calls: list[str] = []
    failing = True

    def clear(self: KeydCommands) -> None:
        calls.append('clear')
        if failing:
            raise RuntimeError('keyd clear unavailable')

    monkeypatch.setattr(KeydCommands, 'clear_state', clear)
    monkeypatch.setattr(KeydCommands, 'activate_game', lambda self: calls.append('activate'))
    runtime._suspend_activation()
    assert runtime.keyd.profile == profile
    assert runtime.keyd.active_profile.is_file()
    restarted = ServiceRuntime(dependencies, runtime_dir=tmp_path)
    with pytest.raises(RuntimeError, match='suspended'):
        restarted.controller.on_focus(FocusEvent('Minecraft', 'Game'))
    assert 'activate' not in calls
    failing = False
    restarted._retry_clear()
    assert restarted.keyd.profile is None
    assert not restarted.keyd.active_profile.exists()
    assert calls == ['clear', 'clear']


def test_restore_failure_after_failed_clear_retains_recovery_profile(
    tmp_path: Path, monkeypatch: pytest.MonkeyPatch,
) -> None:
    from mcsr_toolkit.__main__ import ServiceRuntime
    from mcsr_toolkit.keyd import KeydCommands

    web, target, adapter = client(tmp_path)
    dependencies = web.app.state.dependencies
    profile = Profile(crafting={'s': 'k'}, launcher_root=tmp_path / 'launcher')
    save_profile(dependencies.profile_path, profile)
    report = dependencies.coordinator_factory(tmp_path / 'launcher').apply(profile)
    runtime = ServiceRuntime(dependencies, runtime_dir=tmp_path)
    runtime._applied(profile)

    def fail() -> None:
        raise RuntimeError('keyd unavailable')

    monkeypatch.setattr(KeydCommands, 'clear_state', lambda self: fail())
    accepted = web.post(
        '/api/restore', json={'backup_id': report.backup_id}, headers=write_headers(),
    )
    job = job_result(web, accepted.json()['id'])
    assert job['status'] == 'failed'
    assert 'keyd unavailable' in str(job['error'])
    assert target.read_text() == 'key_key.back:key.keyboard.k\n'
    assert adapter.restores == 0
    assert runtime.keyd.profile == profile
    assert runtime.keyd.active_profile.is_file()
    assert runtime.status()['activation_suspended'] is True
    assert 'keyd unavailable' in runtime.errors


def test_apply_clear_failure_precedes_options_and_keyd_install(
    tmp_path: Path, monkeypatch: pytest.MonkeyPatch,
) -> None:
    from mcsr_toolkit.__main__ import ServiceRuntime
    from mcsr_toolkit.keyd import KeydCommands

    web, target, adapter = client(tmp_path)
    dependencies = web.app.state.dependencies
    runtime = ServiceRuntime(dependencies, runtime_dir=tmp_path)
    old = Profile(crafting={'s': 'k'}, launcher_root=tmp_path / 'launcher')
    save_profile(dependencies.profile_path, old)
    dependencies.coordinator_factory(tmp_path / 'launcher').apply(old)
    runtime._applied(old)
    runtime.controller.game_active = True
    new = Profile(crafting={'s': 'x'}, launcher_root=tmp_path / 'launcher')
    save_profile(dependencies.profile_path, new)

    def fail(self: KeydCommands) -> None:
        raise RuntimeError('keyd unavailable')

    monkeypatch.setattr(KeydCommands, 'clear_state', fail)
    accepted = web.post('/api/apply', json=new.model_dump(mode='json'), headers=write_headers())
    assert job_result(web, accepted.json()['id'])['status'] == 'failed'
    assert target.read_text() == 'key_key.back:key.keyboard.k\n'
    assert adapter.installs == 1
    assert runtime.keyd.profile == old
    assert runtime.status()['activation_suspended'] is True


def test_restart_keeps_recovery_loop_alive_when_keyd_clear_fails(
    tmp_path: Path, monkeypatch: pytest.MonkeyPatch,
) -> None:
    from mcsr_toolkit.__main__ import ServiceRuntime
    from mcsr_toolkit.keyd import KeydCommands

    web, _, _ = client(tmp_path)
    dependencies = web.app.state.dependencies
    runtime = ServiceRuntime(dependencies, runtime_dir=tmp_path)
    profile = Profile(launcher_root=tmp_path / 'launcher')
    runtime._applied(profile)

    def fail(self: KeydCommands) -> None:
        raise RuntimeError('keyd temporarily unavailable')

    monkeypatch.setattr(KeydCommands, 'clear_state', fail)
    restarted = ServiceRuntime(dependencies, runtime_dir=tmp_path)
    restarted.app = web.app
    restarted.start()
    try:
        assert restarted.thread is not None and restarted.thread.is_alive()
        assert restarted.keyd.suspended is True
        assert restarted.keyd.profile == profile
        assert restarted.keyd.active_profile.is_file()
        assert restarted.keyd.suspended_path.is_file()
    finally:
        asyncio.run(restarted.close())


def test_apply_handoff_does_not_clear_old_profile_after_coordinator_mutation(
    tmp_path: Path, monkeypatch: pytest.MonkeyPatch,
) -> None:
    from mcsr_toolkit.__main__ import ServiceRuntime
    from mcsr_toolkit.keyd import KeydCommands
    from mcsr_toolkit.profile import load_profile

    web, target, adapter = client(tmp_path)
    dependencies = web.app.state.dependencies
    runtime = ServiceRuntime(dependencies, runtime_dir=tmp_path)
    old = Profile(crafting={'s': 'k'}, launcher_root=tmp_path / 'launcher')
    dependencies.coordinator_factory(tmp_path / 'launcher').apply(old)
    runtime._applied(old)
    runtime.controller.game_active = True
    new = Profile(crafting={'s': 'x'}, launcher_root=tmp_path / 'launcher')
    save_profile(dependencies.profile_path, new)
    clears: list[Profile] = []

    def clear(self: KeydCommands) -> None:
        assert adapter.installs == 1, 'clear happened after mutation'
        clears.append(self.profile)

    monkeypatch.setattr(KeydCommands, 'clear_state', clear)
    accepted = web.post('/api/apply', json=new.model_dump(mode='json'), headers=write_headers())
    assert job_result(web, accepted.json()['id'])['status'] == 'succeeded'
    assert len(clears) == 1
    assert target.read_text() == 'key_key.back:key.keyboard.x\n'
    assert runtime.keyd.profile == new
    assert load_profile(runtime.keyd.active_profile) == new
    assert runtime.keyd.suspended is False


def test_failed_apply_clear_retries_new_desired_profile_after_restart(
    tmp_path: Path, monkeypatch: pytest.MonkeyPatch,
) -> None:
    from mcsr_toolkit.__main__ import ServiceRuntime
    from mcsr_toolkit.keyd import KeydCommands
    from mcsr_toolkit.profile import load_profile

    web, target, adapter = client(tmp_path)
    dependencies = web.app.state.dependencies
    root = tmp_path / 'launcher'
    old = Profile(crafting={'s': 'k'}, launcher_root=root)
    new = Profile(crafting={'s': 'x'}, launcher_root=root)
    save_profile(dependencies.profile_path, old)
    dependencies.coordinator_factory(root).apply(old)
    runtime = ServiceRuntime(dependencies, runtime_dir=tmp_path)
    runtime._applied(old)
    save_profile(dependencies.profile_path, new)

    unavailable = True
    clears: list[Profile] = []

    def clear(self: KeydCommands) -> None:
        clears.append(self.profile)
        if unavailable:
            raise RuntimeError('keyd unavailable')

    monkeypatch.setattr(KeydCommands, 'clear_state', clear)
    accepted = web.post('/api/apply', json=new.model_dump(mode='json'), headers=write_headers())
    assert job_result(web, accepted.json()['id'])['status'] == 'failed'
    assert target.read_text() == 'key_key.back:key.keyboard.k\n'
    assert adapter.installs == 1
    assert load_profile(dependencies.state_dir / 'pending.json') == new
    assert load_profile(runtime.keyd.active_profile) == old

    unavailable = False
    restarted = ServiceRuntime(dependencies, runtime_dir=tmp_path)
    restarted.app = web.app
    restarted._retry_clear()
    assert restarted.keyd.profile is None
    restarted._reconcile()
    for _ in range(100):
        if restarted.keyd.profile == new:
            break
        time.sleep(0.01)
    assert restarted.keyd.profile == new
    assert target.read_text() == 'key_key.back:key.keyboard.x\n'
    assert adapter.installs == 2
    assert not (dependencies.state_dir / 'pending.json').exists()
    assert load_profile(restarted.keyd.active_profile) == new
    assert web.get('/api/profile').json()['crafting'] == {'s': 'x'}
    assert clears[0] == old


def test_successful_live_restart_keeps_active_profile_and_keyd_install(
    tmp_path: Path, monkeypatch: pytest.MonkeyPatch,
) -> None:
    from mcsr_toolkit.__main__ import ServiceRuntime
    from mcsr_toolkit.keyd import KeydCommands
    from mcsr_toolkit.profile import load_profile

    web, target, adapter = client(tmp_path)
    dependencies = web.app.state.dependencies
    root = tmp_path / 'launcher'
    profile = Profile(crafting={'s': 'k'}, launcher_root=root)
    save_profile(dependencies.profile_path, profile)
    dependencies.coordinator_factory(root).apply(profile)
    first = ServiceRuntime(dependencies, runtime_dir=tmp_path)
    first._applied(profile)
    clears: list[Profile] = []
    monkeypatch.setattr(KeydCommands, 'clear_state', lambda self: clears.append(self.profile))

    restarted = ServiceRuntime(dependencies, runtime_dir=tmp_path)
    restarted.app = web.app
    monkeypatch.setattr(restarted, '_loop', lambda: None)
    restarted.start()
    try:
        assert clears == [profile]
        assert restarted.keyd.profile == profile
        assert restarted.keyd.suspended is False
        assert load_profile(restarted.keyd.active_profile) == profile
        assert not restarted.keyd.suspended_path.exists()
        assert not (dependencies.state_dir / 'pending.json').exists()
        assert adapter.installs == 1

        second_game = root / 'instances' / 'two' / 'minecraft'
        second_game.mkdir(parents=True)
        (second_game.parent / 'instance.json').write_text(json.dumps({'id': 'two'}))
        second = second_game / 'options.txt'
        second.write_text('key_key.back:key.keyboard.s\n')
        restarted._reconcile()
        for _ in range(100):
            if restarted.reconciliation is not None:
                break
            time.sleep(0.01)
        assert restarted.reconciliation is not None
        assert adapter.installs == 1
        assert not (dependencies.state_dir / 'pending.json').exists()
        assert target.read_text() == 'key_key.back:key.keyboard.k\n'
        assert second.read_text() == 'key_key.back:key.keyboard.k\n'
    finally:
        asyncio.run(restarted.close())


def test_blur_persists_marker_before_reset(tmp_path: Path, monkeypatch: pytest.MonkeyPatch) -> None:
    from mcsr_toolkit.__main__ import ServiceRuntime
    from mcsr_toolkit.focus import FocusEvent
    from mcsr_toolkit.keyd import KeydCommands
    from mcsr_toolkit.resize import GnomeWindowBus

    web, _, _ = client(tmp_path)
    runtime = ServiceRuntime(web.app.state.dependencies, runtime_dir=tmp_path)
    runtime._applied(Profile())
    monkeypatch.setattr(GnomeWindowBus, 'get_focus', lambda self: FocusEvent('Minecraft', 'Game'))
    monkeypatch.setattr(KeydCommands, 'activate_game', lambda self: None)
    runtime.controller.on_focus(FocusEvent('Minecraft', 'Game'))

    def clear(self: KeydCommands) -> None:
        assert runtime.keyd.suspended_path.is_file()
        assert runtime.keyd.suspended is True

    monkeypatch.setattr(KeydCommands, 'clear_state', clear)
    runtime.controller.on_focus(FocusEvent('Ptyxis', 'Terminal'))
    assert not runtime.keyd.suspended_path.exists()
    assert runtime.keyd.profile == Profile()


def test_successful_clear_retains_profile_for_focus_reentry(
    tmp_path: Path, monkeypatch: pytest.MonkeyPatch,
) -> None:
    from mcsr_toolkit.__main__ import ServiceRuntime
    from mcsr_toolkit.focus import FocusEvent
    from mcsr_toolkit.keyd import KeydCommands
    from mcsr_toolkit.resize import GnomeWindowBus

    web, _, _ = client(tmp_path)
    runtime = ServiceRuntime(web.app.state.dependencies, runtime_dir=tmp_path)
    runtime._applied(Profile())
    calls: list[str] = []
    monkeypatch.setattr(GnomeWindowBus, 'get_focus', lambda self: FocusEvent('Minecraft', 'Game'))
    monkeypatch.setattr(KeydCommands, 'activate_game', lambda self: calls.append('activate'))
    monkeypatch.setattr(KeydCommands, 'clear_state', lambda self: calls.append('clear'))
    runtime.controller.on_focus(FocusEvent('Minecraft', 'Game'))
    runtime.controller.on_focus(FocusEvent('Ptyxis', 'Terminal'))
    runtime.controller.on_focus(FocusEvent('Minecraft', 'Game'))
    assert calls == ['activate', 'clear', 'activate']
    assert runtime.keyd.profile == Profile()
    assert runtime.status()['activation_suspended'] is False


def test_stop_failure_is_visible_and_never_reports_safe(
    tmp_path: Path, monkeypatch: pytest.MonkeyPatch,
) -> None:
    from mcsr_toolkit.__main__ import ServiceRuntime
    from mcsr_toolkit.keyd import KeydCommands

    web, _, _ = client(tmp_path)
    runtime = ServiceRuntime(web.app.state.dependencies, runtime_dir=tmp_path)
    runtime._applied(Profile())

    def fail(self: KeydCommands) -> None:
        raise RuntimeError('keyd stop failed; unsafe: daemon may remain active')

    monkeypatch.setattr(KeydCommands, 'clear_state', fail)
    runtime._suspend_activation()
    status = web.get('/api/status').json()['focus']
    assert status['recovery_clear_pending'] is True
    assert status['unsafe_stop_failure'] is True
    assert 'stop failed' in status['cleanup_error']
    assert status['mode'] != 'observe-only'


def test_keyd_crash_after_clear_suspends_activation(
    tmp_path: Path, monkeypatch: pytest.MonkeyPatch,
) -> None:
    from mcsr_toolkit.__main__ import ServiceRuntime
    from mcsr_toolkit.keyd import KeydCommands

    web, _, _ = client(tmp_path)
    runtime = ServiceRuntime(web.app.state.dependencies, runtime_dir=tmp_path)
    runtime._applied(Profile())
    monkeypatch.setattr(KeydCommands, 'clear_state', lambda self: None)
    monkeypatch.setattr(runtime, '_keyd_healthy', lambda: False)
    runtime._poll_keyd_health()
    assert runtime.keyd.suspended is True
    assert runtime.status()['activation_suspended'] is True
    assert runtime.keyd.active_profile.exists() is False


def test_heartbeat_loss_during_apply_blocks_activation(
    tmp_path: Path, monkeypatch: pytest.MonkeyPatch,
) -> None:
    from threading import Thread

    from mcsr_toolkit.__main__ import ServiceRuntime
    from mcsr_toolkit.focus import FocusEvent
    from mcsr_toolkit.keyd import KeydCommands
    from mcsr_toolkit.resize import GnomeWindowBus

    web, _, _ = client(tmp_path)
    runtime = ServiceRuntime(web.app.state.dependencies, runtime_dir=tmp_path)
    runtime._applied(Profile())
    monkeypatch.setattr(GnomeWindowBus, 'get_focus', lambda self: FocusEvent('Minecraft', 'Game'))
    activated: list[str] = []
    monkeypatch.setattr(KeydCommands, 'activate_game', lambda self: activated.append('bind'))
    runtime.controller.on_focus(FocusEvent('Minecraft', 'Game'))
    entered, release, heartbeat_started, focus_started = Event(), Event(), Event(), Event()

    def clear(self: KeydCommands) -> None:
        assert runtime.keyd.suspended_path.is_file()
        entered.set()
        assert release.wait(1)

    monkeypatch.setattr(KeydCommands, 'clear_state', clear)
    runtime.pipe.watchdog.last_seen = 10
    apply = Thread(target=lambda: runtime._before_apply(Profile(crafting={'s': 'k'})))
    apply.start()
    assert entered.wait(1)

    def heartbeat() -> None:
        heartbeat_started.set()
        runtime.pipe.watchdog.tick(12)

    def focus() -> None:
        focus_started.set()
        try:
            runtime.controller.on_focus(FocusEvent('Minecraft', 'Game'))
        except RuntimeError:
            return

    lost = Thread(target=heartbeat)
    new_focus = Thread(target=focus)
    lost.start()
    new_focus.start()
    assert heartbeat_started.wait(1) and focus_started.wait(1)
    assert activated == ['bind']
    release.set()
    for worker in (apply, lost, new_focus):
        worker.join(1)
        assert not worker.is_alive()
    assert activated == ['bind']
    assert runtime.pipe.watchdog.last_seen is None
    assert runtime.controller.game_active is False


def test_modal_during_activation_blocks_new_bind(
    tmp_path: Path, monkeypatch: pytest.MonkeyPatch,
) -> None:
    from threading import Thread

    from mcsr_toolkit.__main__ import ServiceRuntime
    from mcsr_toolkit.focus import FocusEvent
    from mcsr_toolkit.keyd import KeydCommands
    from mcsr_toolkit.resize import GnomeWindowBus

    web, _, _ = client(tmp_path)
    runtime = ServiceRuntime(web.app.state.dependencies, runtime_dir=tmp_path)
    runtime._applied(Profile())
    entered, release, modal_started = Event(), Event(), Event()
    calls: list[str] = []
    failures: list[str] = []

    def focus_reply(self: GnomeWindowBus) -> FocusEvent:
        entered.set()
        assert release.wait(1)
        return FocusEvent('Minecraft', 'Game', True)

    monkeypatch.setattr(GnomeWindowBus, 'get_focus', focus_reply)
    monkeypatch.setattr(KeydCommands, 'activate_game', lambda self: calls.append('activate'))
    monkeypatch.setattr(KeydCommands, 'clear_state', lambda self: calls.append('clear'))

    def game_focus() -> None:
        try:
            runtime.controller.on_focus(FocusEvent('Minecraft', 'Game'))
        except RuntimeError as error:
            failures.append(str(error))

    game = Thread(target=game_focus)
    game.start()
    assert entered.wait(1)
    modal_event = FocusEvent('Minecraft', 'Game', True)

    def modal_focus() -> None:
        modal_started.set()
        runtime.controller.on_focus(modal_event)

    modal = Thread(target=modal_focus)
    modal.start()
    assert modal_started.wait(1)
    assert calls == []
    release.set()
    game.join(1)
    modal.join(1)
    assert not game.is_alive() and not modal.is_alive()
    assert calls == []
    assert failures == ['live activation requires effective Minecraft focus']
    assert runtime.controller.game_active is False


def test_live_mode_requires_helper_preflight(monkeypatch: pytest.MonkeyPatch) -> None:
    import sys

    from mcsr_toolkit import __main__ as service

    monkeypatch.setattr(sys, 'argv', ['mcsr-toolkit', '--enable-live-keyd'])

    def denied() -> None:
        raise RuntimeError('helper permission missing')

    monkeypatch.setattr(service, 'preflight_keyd_clear', denied)
    with pytest.raises(RuntimeError, match='helper permission missing'):
        service.main()


def test_recovery_never_removes_marker_before_active_profile(
    tmp_path: Path, monkeypatch: pytest.MonkeyPatch,
) -> None:
    from mcsr_toolkit.__main__ import FocusKeydBridge
    from mcsr_toolkit.keyd import KeydCommands

    bridge = FocusKeydBridge(True, tmp_path / 'active-profile.json')
    bridge.applied(Profile())
    marker = bridge.suspended_path
    active = bridge.active_profile
    original_unlink = Path.unlink
    removed: list[Path] = []

    def unlink(path: Path, *args: object, **kwargs: object) -> None:
        removed.append(path)
        if path == marker:
            raise RuntimeError('simulated crash at marker removal')
        assert marker.is_file(), 'active profile removed without durable suspension'
        original_unlink(path, *args, **kwargs)

    monkeypatch.setattr(KeydCommands, 'clear_state', lambda self: None)
    monkeypatch.setattr(Path, 'unlink', unlink)
    bridge.clear_recovery()
    assert active in removed
    assert marker not in removed
    assert marker.is_file()


@pytest.mark.parametrize('method', ['clear_state', 'clear_recovery', 'restoring'])
def test_marker_write_failure_still_runs_physical_keyd_clear(
    tmp_path: Path, monkeypatch: pytest.MonkeyPatch, method: str,
) -> None:
    from mcsr_toolkit import __main__ as service
    from mcsr_toolkit.keyd import KeydCommands

    bridge = service.FocusKeydBridge(True, tmp_path / 'active-profile.json')
    bridge.applied(Profile())
    marker = bridge.suspended_path
    original_save = service.save_profile
    calls: list[str] = []

    def fail_marker(path: Path, profile: Profile) -> None:
        if path == marker:
            raise OSError('state disk full')
        original_save(path, profile)

    monkeypatch.setattr(service, 'save_profile', fail_marker)
    monkeypatch.setattr(KeydCommands, 'clear_state', lambda self: calls.append('clear'))
    with pytest.raises(OSError, match='state disk full'):
        getattr(bridge, method)()
    assert calls == ['clear']
    assert bridge.active_profile.is_file()
    assert bridge.suspended is True
    assert 'state disk full' in str(bridge.cleanup_error)


@pytest.mark.parametrize('operation', ['suspend', 'restore'])
def test_runtime_marker_failure_still_attempts_physical_clear(
    tmp_path: Path, monkeypatch: pytest.MonkeyPatch, operation: str,
) -> None:
    from mcsr_toolkit import __main__ as service
    from mcsr_toolkit.keyd import KeydCommands

    web, _, _ = client(tmp_path)
    runtime = service.ServiceRuntime(web.app.state.dependencies, runtime_dir=tmp_path)
    runtime._applied(Profile())
    marker = runtime.keyd.suspended_path
    original_save = service.save_profile
    calls: list[str] = []

    def fail_marker(path: Path, profile: Profile) -> None:
        if path == marker:
            raise OSError('state disk full')
        original_save(path, profile)

    monkeypatch.setattr(service, 'save_profile', fail_marker)
    monkeypatch.setattr(KeydCommands, 'clear_state', lambda self: calls.append('clear'))
    if operation == 'restore':
        with pytest.raises(OSError, match='state disk full'):
            runtime._before_restore()
    else:
        runtime._suspend_activation()
    assert calls == ['clear']
    assert runtime.keyd.active_profile.is_file()
    assert runtime.keyd.suspended is True


def test_failed_stop_blocks_restore_and_retains_recovery_state(
    tmp_path: Path, monkeypatch: pytest.MonkeyPatch,
) -> None:
    from mcsr_toolkit.__main__ import ServiceRuntime
    from mcsr_toolkit.keyd import KeydCommands

    web, target, adapter = client(tmp_path)
    dependencies = web.app.state.dependencies
    profile = Profile(crafting={'s': 'k'}, launcher_root=tmp_path / 'launcher')
    save_profile(dependencies.profile_path, profile)
    report = dependencies.coordinator_factory(tmp_path / 'launcher').apply(profile)
    assert report.backup_id is not None
    runtime = ServiceRuntime(dependencies, runtime_dir=tmp_path)
    runtime._applied(profile)

    def fail(self: KeydCommands) -> None:
        raise RuntimeError('keyd stop failed; unsafe')

    monkeypatch.setattr(KeydCommands, 'clear_state', fail)
    accepted = web.post(
        '/api/restore', json={'backup_id': report.backup_id}, headers=write_headers(),
    )
    assert job_result(web, accepted.json()['id'])['status'] == 'failed'
    assert target.read_text() == 'key_key.back:key.keyboard.k\n'
    assert adapter.restores == 0
    status = web.get('/api/status').json()['focus']
    assert status['unsafe_stop_failure'] is True
    assert status['recovery_clear_pending'] is True
    assert runtime.keyd.suspended_path.is_file()
    assert runtime.keyd.active_profile.is_file()


def test_restore_second_clear_failure_blocks_restore_and_keeps_retry_profile(
    tmp_path: Path, monkeypatch: pytest.MonkeyPatch,
) -> None:
    from mcsr_toolkit.__main__ import ServiceRuntime
    from mcsr_toolkit.keyd import KeydCommands

    web, target, adapter = client(tmp_path)
    dependencies = web.app.state.dependencies
    profile = Profile(crafting={'s': 'k'}, launcher_root=tmp_path / 'launcher')
    save_profile(dependencies.profile_path, profile)
    report = dependencies.coordinator_factory(tmp_path / 'launcher').apply(profile)
    assert report.backup_id is not None
    dependencies.state_dir.mkdir(exist_ok=True)
    (dependencies.state_dir / 'keyd-adapter.json').write_text('{}')
    runtime = ServiceRuntime(dependencies, runtime_dir=tmp_path)
    runtime._applied(profile)
    attempts = 0

    def clear(self: KeydCommands) -> None:
        nonlocal attempts
        attempts += 1
        if attempts == 2:
            raise RuntimeError('second clear failed')

    monkeypatch.setattr(KeydCommands, 'clear_state', clear)
    accepted = web.post(
        '/api/restore', json={'backup_id': report.backup_id}, headers=write_headers(),
    )
    assert job_result(web, accepted.json()['id'])['status'] == 'failed'
    assert attempts == 2
    assert target.read_text() == 'key_key.back:key.keyboard.k\n'
    assert adapter.restores == 0
    assert runtime.keyd.suspended_path.is_file()
    assert runtime.keyd.active_profile.is_file()
    assert runtime.keyd.profile is not None
    runtime._retry_clear()
    assert attempts == 3


def test_second_clear_and_active_profile_save_failure_keep_recovery_state(
    tmp_path: Path, monkeypatch: pytest.MonkeyPatch,
) -> None:
    from mcsr_toolkit import __main__ as service
    from mcsr_toolkit.keyd import KeydCommands

    web, target, adapter = client(tmp_path)
    dependencies = web.app.state.dependencies
    profile = Profile(crafting={'s': 'k'}, launcher_root=tmp_path / 'launcher')
    save_profile(dependencies.profile_path, profile)
    report = dependencies.coordinator_factory(tmp_path / 'launcher').apply(profile)
    assert report.backup_id is not None
    dependencies.state_dir.mkdir(exist_ok=True)
    (dependencies.state_dir / 'keyd-adapter.json').write_text('{}')
    runtime = service.ServiceRuntime(dependencies, runtime_dir=tmp_path)
    runtime._applied(profile)
    original_save = service.save_profile
    calls = 0

    def fail_second_clear(self: KeydCommands) -> None:
        nonlocal calls
        calls += 1
        if calls == 2:
            raise RuntimeError('keyd stop failed')

    def fail_active_save(path: Path, value: Profile) -> None:
        if path == runtime.keyd.active_profile:
            raise OSError('state disk full')
        original_save(path, value)

    monkeypatch.setattr(KeydCommands, 'clear_state', fail_second_clear)
    monkeypatch.setattr(service, 'save_profile', fail_active_save)
    accepted = web.post(
        '/api/restore', json={'backup_id': report.backup_id}, headers=write_headers(),
    )
    assert job_result(web, accepted.json()['id'])['status'] == 'failed'
    assert target.read_text() == 'key_key.back:key.keyboard.k\n'
    assert adapter.restores == 0
    assert runtime.status()['unsafe_stop_failure'] is True
    assert runtime.status()['recovery_clear_pending'] is True
    assert runtime.keyd.profile == profile
    assert runtime.keyd.suspended_path.is_file()
    assert not runtime.keyd.active_profile.exists()
    restarted = service.ServiceRuntime(dependencies, runtime_dir=tmp_path)
    assert restarted.keyd.profile == profile
    assert restarted.status()['recovery_clear_pending'] is True


def test_observe_only_restart_clears_persisted_live_bindings(
    tmp_path: Path, monkeypatch: pytest.MonkeyPatch,
) -> None:
    from mcsr_toolkit.__main__ import ServiceRuntime
    from mcsr_toolkit.focus import FocusEvent
    from mcsr_toolkit.keyd import KeydCommands

    web, _, _ = client(tmp_path)
    dependencies = web.app.state.dependencies
    live = ServiceRuntime(dependencies, runtime_dir=tmp_path)
    live._applied(Profile())
    dependencies.live_activation_enabled = False
    calls: list[str] = []
    monkeypatch.setattr(KeydCommands, 'clear_state', lambda self: calls.append('clear'))
    restarted = ServiceRuntime(dependencies, runtime_dir=tmp_path)
    assert restarted.keyd.profile is not None
    restarted.start()
    try:
        assert calls == ['clear']
        assert restarted.status()['mode'] == 'observe-only'
        assert restarted.keyd.profile is None
        restarted.controller.on_focus(FocusEvent('Minecraft', 'Game'))
        restarted.controller.on_focus(FocusEvent('Ptyxis', 'Terminal'))
        assert calls == ['clear']
    finally:
        asyncio.run(restarted.close())


def test_observe_only_restart_retries_persisted_failed_clear(
    tmp_path: Path, monkeypatch: pytest.MonkeyPatch,
) -> None:
    from mcsr_toolkit.__main__ import ServiceRuntime
    from mcsr_toolkit.keyd import KeydCommands

    web, _, _ = client(tmp_path)
    dependencies = web.app.state.dependencies
    live = ServiceRuntime(dependencies, runtime_dir=tmp_path)
    live._applied(Profile())
    live.keyd.suspend()
    dependencies.live_activation_enabled = False
    calls: list[str] = []
    monkeypatch.setattr(KeydCommands, 'clear_state', lambda self: calls.append('clear'))
    restarted = ServiceRuntime(dependencies, runtime_dir=tmp_path)
    assert restarted.status()['recovery_clear_pending'] is True
    restarted._retry_clear()
    assert calls == ['clear']
    assert restarted.status()['recovery_clear_pending'] is False


def test_uncertain_activation_persists_cleanup_for_retry(
    tmp_path: Path, monkeypatch: pytest.MonkeyPatch,
) -> None:
    from mcsr_toolkit.__main__ import ServiceRuntime
    from mcsr_toolkit.focus import FocusEvent
    from mcsr_toolkit.keyd import KeydCommands

    web, _, _ = client(tmp_path)
    runtime = ServiceRuntime(web.app.state.dependencies, runtime_dir=tmp_path)
    runtime._applied(Profile())
    runtime.keyd.confirm_focus = lambda: True
    calls: list[str] = []

    def uncertain_activate(self: KeydCommands) -> None:
        calls.append('activate')
        raise RuntimeError('activation response lost')

    def failed_clear(self: KeydCommands) -> None:
        calls.append('clear')
        raise RuntimeError('helper unavailable')

    monkeypatch.setattr(KeydCommands, 'activate_game', uncertain_activate)
    monkeypatch.setattr(KeydCommands, 'clear_state', failed_clear)
    with pytest.raises(RuntimeError, match='helper unavailable'):
        runtime.controller.on_focus(FocusEvent('Minecraft', 'Game'))
    assert calls == ['activate', 'clear']
    assert runtime.controller.game_active is False
    assert runtime.keyd.active_profile.is_file()
    assert runtime.keyd.suspended_path.is_file()
    assert runtime.status()['recovery_clear_pending'] is True


def test_uncertain_activation_is_cleared_before_focus_can_reactivate(
    tmp_path: Path, monkeypatch: pytest.MonkeyPatch,
) -> None:
    from mcsr_toolkit.__main__ import ServiceRuntime
    from mcsr_toolkit.focus import FocusEvent
    from mcsr_toolkit.keyd import KeydCommands

    web, _, _ = client(tmp_path)
    runtime = ServiceRuntime(web.app.state.dependencies, runtime_dir=tmp_path)
    runtime._applied(Profile())
    runtime.keyd.confirm_focus = lambda: True
    calls: list[str] = []

    def uncertain_activate(self: KeydCommands) -> None:
        calls.append('activate')
        raise RuntimeError('activation response lost')

    monkeypatch.setattr(KeydCommands, 'activate_game', uncertain_activate)
    monkeypatch.setattr(KeydCommands, 'clear_state', lambda self: calls.append('clear'))
    with pytest.raises(RuntimeError, match='activation response lost'):
        runtime.controller.on_focus(FocusEvent('Minecraft', 'Game'))
    assert calls == ['activate', 'clear']
    assert runtime.controller.game_active is False
    assert not runtime.keyd.suspended_path.exists()
    assert runtime.status()['recovery_clear_pending'] is False


def test_multi_apply_restore_publishes_prior_active_profile(tmp_path, monkeypatch):
    from mcsr_toolkit.__main__ import ServiceRuntime
    from mcsr_toolkit.keyd import KeydCommands
    from mcsr_toolkit.profile import load_profile

    web, target, _ = client(tmp_path)
    dependencies = web.app.state.dependencies
    runtime = ServiceRuntime(dependencies, runtime_dir=tmp_path)
    monkeypatch.setattr(KeydCommands, 'clear_state', lambda self: None)
    first = Profile(crafting={'s': 'k'}, launcher_root=tmp_path / 'launcher')
    second = Profile(crafting={'s': 'j'}, launcher_root=tmp_path / 'launcher')
    save_profile(dependencies.profile_path, first)
    apply_a = web.post('/api/apply', json=first.model_dump(mode='json'), headers=write_headers())
    backup_a = job_result(web, apply_a.json()['id'])['result']['backup_id']
    save_profile(dependencies.profile_path, second)
    apply_b = web.post('/api/apply', json=second.model_dump(mode='json'), headers=write_headers())
    backup_b = job_result(web, apply_b.json()['id'])['result']['backup_id']
    restore_b = web.post('/api/restore', json={'backup_id': backup_b}, headers=write_headers())
    assert job_result(web, restore_b.json()['id'])['status'] == 'succeeded'
    assert target.read_text() == 'key_key.back:key.keyboard.k\n'
    assert load_profile(dependencies.profile_path) == first
    assert load_profile(runtime.keyd.active_profile) == first
    assert runtime.keyd.profile == first
    assert runtime._desired_profile == first
    restore_a = web.post('/api/restore', json={'backup_id': backup_a}, headers=write_headers())
    assert job_result(web, restore_a.json()['id'])['status'] == 'succeeded'
    assert runtime.keyd.profile is None
    assert not runtime.keyd.active_profile.exists()
    assert runtime._desired_profile is None


def test_unfinished_mutation_blocks_runtime_activation_after_restart(tmp_path, monkeypatch):
    from mcsr_toolkit.__main__ import ServiceRuntime
    from mcsr_toolkit.focus import FocusEvent
    from mcsr_toolkit.keyd import KeydCommands
    from mcsr_toolkit.resize import GnomeWindowBus

    web, _, adapter = client(tmp_path)
    dependencies = web.app.state.dependencies
    profile = Profile(crafting={'s': 'k'}, launcher_root=tmp_path / 'launcher')
    save_profile(dependencies.profile_path, profile)
    dependencies.coordinator_factory(tmp_path / 'launcher').apply(profile)
    active = dependencies.state_dir / 'active-profile.json'
    save_profile(active, profile)
    (dependencies.state_dir / 'mutation.json').write_text('{"operation": "restore"}')
    runtime = ServiceRuntime(dependencies, runtime_dir=tmp_path)
    monkeypatch.setattr(
        GnomeWindowBus, 'get_focus', lambda self: FocusEvent('Minecraft', 'Minecraft', False)
    )
    calls = []
    monkeypatch.setattr(KeydCommands, 'activate_game', lambda self: calls.append('activate'))
    monkeypatch.setattr(KeydCommands, 'clear_state', lambda self: calls.append('clear'))
    with pytest.raises(RuntimeError, match='suspended|focus'):
        runtime.keyd.activate_game()
    runtime._retry_clear()
    runtime._schedule_reconcile()
    assert calls == ['clear']
    assert adapter.installs == 1
    assert runtime.keyd.profile == profile
    assert runtime.status()['mutation_recovery_pending'] is True


def test_restore_helper_stop_failure_is_visible_after_restart(tmp_path, monkeypatch):
    from mcsr_toolkit.__main__ import ServiceRuntime
    from mcsr_toolkit.keyd import KeydCommands

    web, target, adapter = client(tmp_path)
    dependencies = web.app.state.dependencies
    profile = Profile(crafting={'s': 'k'}, launcher_root=tmp_path / 'launcher')
    save_profile(dependencies.profile_path, profile)
    report = dependencies.coordinator_factory(tmp_path / 'launcher').apply(profile)
    runtime = ServiceRuntime(dependencies, runtime_dir=tmp_path)
    runtime._applied(profile)
    monkeypatch.setattr(KeydCommands, 'clear_state', lambda self: None)

    def fail_restore(checkpoint):
        raise RuntimeError('keyd restore failed; stop failed: denied')

    monkeypatch.setattr(adapter, 'restore_to', fail_restore)
    restore = web.post(
        '/api/restore', json={'backup_id': report.backup_id}, headers=write_headers()
    )
    assert job_result(web, restore.json()['id'])['status'] == 'failed'
    assert target.read_text() == 'key_key.back:key.keyboard.k\n'
    assert runtime.status()['unsafe_stop_failure'] is True
    restarted = ServiceRuntime(dependencies, runtime_dir=tmp_path)
    assert restarted.status()['unsafe_stop_failure'] is True
    assert restarted.status()['mutation_recovery_pending'] is True


def test_symlink_mutation_record_blocks_activation_without_reading_target(tmp_path):
    from mcsr_toolkit.__main__ import ServiceRuntime

    web, _, _ = client(tmp_path)
    dependencies = web.app.state.dependencies
    dependencies.state_dir.mkdir()
    outside = tmp_path / 'outside.json'
    outside.write_text('{"error": "unrelated target contents"}')
    (dependencies.state_dir / 'mutation.json').symlink_to(outside)
    runtime = ServiceRuntime(dependencies, runtime_dir=tmp_path)
    assert runtime.status()['mutation_recovery_pending'] is True
    assert runtime.keyd.suspended is True
    assert runtime.keyd.cleanup_error != 'unrelated target contents'


def test_apply_handoff_failure_retains_marker_and_blocks_restart(tmp_path, monkeypatch):
    from mcsr_toolkit.__main__ import ServiceRuntime
    from mcsr_toolkit.keyd import KeydCommands

    web, target, _ = client(tmp_path)
    dependencies = web.app.state.dependencies
    runtime = ServiceRuntime(dependencies, runtime_dir=tmp_path)
    profile = Profile(crafting={'s': 'k'}, launcher_root=tmp_path / 'launcher')
    save_profile(dependencies.profile_path, profile)
    monkeypatch.setattr(KeydCommands, 'clear_state', lambda self: None)

    def fail_publication(profile):
        raise OSError('active profile publication failed')

    dependencies.on_applied = fail_publication
    accepted = web.post('/api/apply', json=profile.model_dump(mode='json'), headers=write_headers())
    assert job_result(web, accepted.json()['id'])['status'] == 'failed'
    assert target.read_text() == 'key_key.back:key.keyboard.k\n'
    assert (dependencies.state_dir / 'mutation.json').is_file()
    assert runtime.status()['mutation_recovery_pending'] is True
    restarted = ServiceRuntime(dependencies, runtime_dir=tmp_path)
    assert restarted.keyd.suspended is True
    assert restarted._desired_profile is None
    with pytest.raises(RuntimeError, match='recovery'):
        restarted._before_apply(profile)


def test_apply_install_stop_failure_is_visible_after_restart(tmp_path, monkeypatch):
    from mcsr_toolkit.__main__ import ServiceRuntime

    web, target, adapter = client(tmp_path)
    dependencies = web.app.state.dependencies
    runtime = ServiceRuntime(dependencies, runtime_dir=tmp_path)
    profile = Profile(crafting={'s': 'k'}, launcher_root=tmp_path / 'launcher')
    save_profile(dependencies.profile_path, profile)

    def fail_install(profile):
        raise RuntimeError('keyd install failed; stop failed: denied')

    monkeypatch.setattr(adapter, 'install', fail_install)
    accepted = web.post('/api/apply', json=profile.model_dump(mode='json'), headers=write_headers())
    result = job_result(web, accepted.json()['id'])
    assert 'stop failed' in result['result']['keyd_error']
    assert target.read_text() == 'key_key.back:key.keyboard.s\n'
    assert runtime.status()['unsafe_stop_failure'] is True
    restarted = ServiceRuntime(dependencies, runtime_dir=tmp_path)
    assert restarted.status()['unsafe_stop_failure'] is True
    assert restarted.status()['mutation_recovery_pending'] is True


def test_repeat_apply_handoff_failure_stays_recoverable_without_new_snapshot(tmp_path, monkeypatch):
    from mcsr_toolkit.__main__ import ServiceRuntime
    from mcsr_toolkit.keyd import KeydCommands

    web, _, adapter = client(tmp_path)
    dependencies = web.app.state.dependencies
    runtime = ServiceRuntime(dependencies, runtime_dir=tmp_path)
    profile = Profile(crafting={'s': 'k'}, launcher_root=tmp_path / 'launcher')
    save_profile(dependencies.profile_path, profile)
    monkeypatch.setattr(KeydCommands, 'clear_state', lambda self: None)
    first = web.post('/api/apply', json=profile.model_dump(mode='json'), headers=write_headers())
    first_result = job_result(web, first.json()['id'])
    backup_id = first_result['result']['backup_id']

    def fail_publication(profile):
        assert (dependencies.state_dir / 'mutation.json').is_file()
        raise OSError('repeat publication failed')

    dependencies.on_applied = fail_publication
    repeated = web.post('/api/apply', json=profile.model_dump(mode='json'), headers=write_headers())
    assert job_result(web, repeated.json()['id'])['status'] == 'failed'
    marker = json.loads((dependencies.state_dir / 'mutation.json').read_text())
    assert marker['backup_id'] == backup_id
    assert adapter.installs == 1
    assert runtime.status()['mutation_recovery_pending'] is True


def test_restore_saved_profile_directory_sync_failure_keeps_marker(tmp_path, monkeypatch):
    import os
    import stat

    from mcsr_toolkit import profile as profile_module
    from mcsr_toolkit.__main__ import ServiceRuntime
    from mcsr_toolkit.keyd import KeydCommands

    web, target, _ = client(tmp_path)
    dependencies = web.app.state.dependencies
    runtime = ServiceRuntime(dependencies, runtime_dir=tmp_path)
    monkeypatch.setattr(KeydCommands, 'clear_state', lambda self: None)
    first = Profile(crafting={'s': 'k'}, launcher_root=tmp_path / 'launcher')
    second = Profile(crafting={'s': 'j'}, launcher_root=tmp_path / 'launcher')
    for profile in (first, second):
        save_profile(dependencies.profile_path, profile)
        accepted = web.post(
            '/api/apply', json=profile.model_dump(mode='json'), headers=write_headers()
        )
        result = job_result(web, accepted.json()['id'])
        assert result['status'] == 'succeeded'
    backup_id = result['result']['backup_id']
    parent_inode = dependencies.profile_path.parent.stat().st_ino
    original = os.fsync

    def fail_profile_directory_sync(fd):
        info = os.fstat(fd)
        if stat.S_ISDIR(info.st_mode) and info.st_ino == parent_inode:
            raise OSError('saved profile directory sync failed')
        original(fd)

    monkeypatch.setattr(profile_module.os, 'fsync', fail_profile_directory_sync)
    accepted = web.post('/api/restore', json={'backup_id': backup_id}, headers=write_headers())
    result = job_result(web, accepted.json()['id'])
    assert result['status'] == 'failed'
    assert 'saved profile directory sync' in result['error']
    assert target.read_text() == 'key_key.back:key.keyboard.k\n'
    assert (dependencies.state_dir / 'mutation.json').is_file()
    assert runtime.status()['mutation_recovery_pending'] is True
    assert runtime.keyd.suspended is True


def test_apply_rollback_stop_failure_is_persisted_for_restart(tmp_path, monkeypatch):
    from mcsr_toolkit import apply as apply_module
    from mcsr_toolkit.__main__ import ServiceRuntime

    web, target, adapter = client(tmp_path)
    dependencies = web.app.state.dependencies
    runtime = ServiceRuntime(dependencies, runtime_dir=tmp_path)
    profile = Profile(crafting={'s': 'k'}, launcher_root=tmp_path / 'launcher')
    save_profile(dependencies.profile_path, profile)
    original = apply_module._atomic_text

    def fail_options(path, contents):
        if path == target and contents.endswith('key.keyboard.k\n'):
            raise OSError('options disk full')
        original(path, contents)

    def fail_rollback():
        raise RuntimeError('keyd rollback failed; stop failed: denied')

    monkeypatch.setattr(apply_module, '_atomic_text', fail_options)
    monkeypatch.setattr(adapter, 'rollback', fail_rollback)
    accepted = web.post('/api/apply', json=profile.model_dump(mode='json'), headers=write_headers())
    assert job_result(web, accepted.json()['id'])['status'] == 'failed'
    assert runtime.status()['unsafe_stop_failure'] is True
    restarted = ServiceRuntime(dependencies, runtime_dir=tmp_path)
    assert restarted.status()['unsafe_stop_failure'] is True


@pytest.mark.parametrize('operation', ['apply', 'restore'])
def test_api_mutation_finalization_serializes_with_watchdog(tmp_path, monkeypatch, operation):
    from threading import Thread

    from mcsr_toolkit.__main__ import ServiceRuntime
    from mcsr_toolkit.keyd import KeydCommands
    from mcsr_toolkit.profile import load_profile

    web, target, _ = client(tmp_path)
    dependencies = web.app.state.dependencies
    runtime = ServiceRuntime(dependencies, runtime_dir=tmp_path)
    root = tmp_path / 'launcher'
    first = Profile(crafting={'s': 'k'}, launcher_root=root)
    second = Profile(crafting={'s': 'j'}, launcher_root=root)
    coordinator = dependencies.coordinator_factory(root)
    coordinator.apply(first)
    backup = coordinator.apply(second).backup_id
    save_profile(dependencies.profile_path, second)
    runtime._applied(second)
    clears = []
    monkeypatch.setattr(KeydCommands, 'clear_state', lambda self: clears.append('clear'))
    published, release, probed, retried = Event(), Event(), Event(), Event()
    lock_available = []

    def pause_publication(profile):
        if operation == 'apply':
            runtime._applied(profile)
        else:
            runtime._restored(profile)
        assert (dependencies.state_dir / 'mutation.json').is_file()
        published.set()
        assert release.wait(5)

    if operation == 'apply':
        expected = Profile(crafting={'s': 'h'}, launcher_root=root)
        save_profile(dependencies.profile_path, expected)
        dependencies.on_applied = pause_publication
        payload = expected.model_dump(mode='json')
    else:
        expected = first
        dependencies.after_restore = pause_publication
        payload = {'backup_id': backup}
    accepted = web.post(f'/api/{operation}', json=payload, headers=write_headers())
    assert accepted.status_code == 202
    assert published.wait(5)
    clears_before_watchdog = len(clears)

    def watchdog():
        available = runtime._preset_lock.acquire(blocking=False)
        lock_available.append(available)
        if available:
            runtime._preset_lock.release()
        probed.set()
        runtime._retry_clear()
        retried.set()

    worker = Thread(target=watchdog)
    worker.start()
    try:
        assert probed.wait(5)
        # Force the defective interleaving when there is no transaction lock.
        # With the fix, the watcher stays blocked until finalization completes.
        if lock_available[0]:
            assert retried.wait(5)
    finally:
        release.set()
    result = job_result(web, accepted.json()['id'])
    worker.join(5)
    assert not worker.is_alive()
    assert result['status'] == 'succeeded'
    runtime._retry_clear()
    assert runtime.keyd.profile == expected
    assert load_profile(runtime.keyd.active_profile) == expected
    assert runtime.keyd.suspended is False
    assert not (dependencies.state_dir / 'mutation.json').exists()
    assert target.read_text() == f"key_key.back:key.keyboard.{expected.crafting['s']}\n"
    assert len(clears) == clears_before_watchdog
    assert lock_available == [False]
