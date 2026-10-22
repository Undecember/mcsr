from __future__ import annotations

import json
import stat
from pathlib import Path

import pytest

from mcsr_toolkit.apply import ApplyCoordinator
from mcsr_toolkit.instances import ProcessInfo
from mcsr_toolkit.models import Profile


class FakeKeyd:
    def __init__(self) -> None:
        self.installs = 0
        self.restores = 0
        self.rollbacks = 0
        self.fail = False
        self.fail_restore = False
        self.current: Profile | None = None
        self.history: list[tuple[str, Profile | None]] = []

    def install(self, profile: Profile) -> None:
        self.installs += 1
        if self.fail:
            raise RuntimeError("keyd reload failed")
        self.history.append((f"{self.installs:032x}", self.current))
        self.current = profile

    def rollback(self) -> None:
        self.rollbacks += 1
        if self.history:
            _, self.current = self.history.pop()

    def restore(self) -> None:
        self.restores += 1
        self.current = None
        self.history.clear()

    def checkpoint(self) -> list[str]:
        return [item[0] for item in self.history]

    def restore_to(self, checkpoint: list[str]) -> None:
        if self.fail_restore:
            raise RuntimeError("restore denied")
        if self.checkpoint()[:len(checkpoint)] != checkpoint:
            raise ValueError("stale keyd checkpoint")
        if len(checkpoint) < len(self.history):
            self.current = self.history[len(checkpoint)][1]
            self.history = self.history[:len(checkpoint)]
            self.restores += 1


def make_instance(root: Path, name: str, options: str = "key_key.back:key.keyboard.s\n") -> Path:
    path = root / "instances" / name
    game = path / "minecraft"
    game.mkdir(parents=True)
    (path / "instance.json").write_text(json.dumps({"id": name}), encoding="utf-8")
    target = game / "options.txt"
    target.write_text(options, encoding="utf-8")
    return target


@pytest.fixture
def setup(tmp_path: Path) -> tuple[Path, Path, FakeKeyd]:
    root = tmp_path / "launcher"
    root.mkdir()
    first = make_instance(root, "one")
    return root, first, FakeKeyd()


def profile() -> Profile:
    return Profile(crafting={"s": "k"})


def test_apply_updates_all_instances_and_restores_original(
    setup: tuple[Path, Path, FakeKeyd], tmp_path: Path
) -> None:
    root, first, keyd = setup
    second = make_instance(root, "two", "fov:0.8\nkey_key.back:key.keyboard.s\n")
    coordinator = ApplyCoordinator(root, tmp_path / "state", keyd, lambda: [])

    preview = coordinator.preview(profile())
    assert {entry.instance_id for entry in preview.instances} == {"one", "two"}
    assert all(entry.status == "ready" for entry in preview.instances)
    assert [mapping.instance_id for mapping in preview.mappings] == ["one", "two"]
    assert all(
        (mapping.physical, mapping.emitted, mapping.action) == ("s", "k", "key_key.back")
        for mapping in preview.mappings
    )
    report = coordinator.apply(profile())
    assert {entry.status for entry in report.instances} == {"updated"}
    assert keyd.installs == 1
    assert first.read_text(encoding="utf-8") == "key_key.back:key.keyboard.k\n"
    assert second.read_text(encoding="utf-8") == "fov:0.8\nkey_key.back:key.keyboard.k\n"

    restored = coordinator.restore(report.backup_id)
    assert {entry.status for entry in restored.instances} == {"restored"}
    assert first.read_text(encoding="utf-8") == "key_key.back:key.keyboard.s\n"
    assert second.read_text(encoding="utf-8") == "fov:0.8\nkey_key.back:key.keyboard.s\n"
    assert keyd.restores == 1


def test_malformed_instance_blocks_activation_and_valid_instance(
    setup: tuple[Path, Path, FakeKeyd], tmp_path: Path
) -> None:
    root, first, keyd = setup
    make_instance(root, "bad", "key_key.back:key.keyboard.s\nkey_key.back:key.keyboard.s\n")
    coordinator = ApplyCoordinator(root, tmp_path / "state", keyd, lambda: [])

    report = coordinator.apply(profile())
    assert {entry.instance_id: entry.status for entry in report.instances} == {
        "bad": "failed",
        "one": "blocked",
    }
    assert first.read_text(encoding="utf-8") == "key_key.back:key.keyboard.s\n"
    assert keyd.installs == 0


def test_running_instance_is_deferred_then_reconciled(
    setup: tuple[Path, Path, FakeKeyd], tmp_path: Path
) -> None:
    root, first, keyd = setup
    processes = [ProcessInfo(first.parent)]
    coordinator = ApplyCoordinator(root, tmp_path / "state", keyd, lambda: processes)

    report = coordinator.apply(profile())
    assert report.instances[0].status == "deferred"
    assert first.read_text(encoding="utf-8") == "key_key.back:key.keyboard.s\n"
    assert keyd.installs == 0
    processes.clear()
    followup = coordinator.reconcile_new_instances(profile())
    assert followup.instances[0].status == "updated"
    assert first.read_text(encoding="utf-8") == "key_key.back:key.keyboard.k\n"
    assert keyd.installs == 1


def test_running_instance_defers_other_instances_too(
    setup: tuple[Path, Path, FakeKeyd], tmp_path: Path
) -> None:
    root, first, keyd = setup
    second = make_instance(root, "two")
    coordinator = ApplyCoordinator(
        root, tmp_path / "state", keyd, lambda: [ProcessInfo(first.parent)]
    )

    report = coordinator.apply(profile())
    assert {entry.instance_id: entry.status for entry in report.instances} == {
        "one": "deferred",
        "two": "deferred",
    }
    assert second.read_text(encoding="utf-8") == "key_key.back:key.keyboard.s\n"
    assert keyd.installs == 0


def test_new_instance_and_repeat_apply_do_not_drift(
    setup: tuple[Path, Path, FakeKeyd], tmp_path: Path
) -> None:
    root, first, keyd = setup
    coordinator = ApplyCoordinator(root, tmp_path / "state", keyd, lambda: [])
    coordinator.apply(profile())
    second = make_instance(root, "two")

    report = coordinator.reconcile_new_instances(profile())
    assert {entry.instance_id: entry.status for entry in report.instances} == {
        "one": "unchanged",
        "two": "updated",
    }
    assert first.read_text(encoding="utf-8") == "key_key.back:key.keyboard.k\n"
    assert second.read_text(encoding="utf-8") == "key_key.back:key.keyboard.k\n"


def test_failed_keyd_install_keeps_all_instance_files(
    setup: tuple[Path, Path, FakeKeyd], tmp_path: Path
) -> None:
    root, first, keyd = setup
    keyd.fail = True
    coordinator = ApplyCoordinator(root, tmp_path / "state", keyd, lambda: [])

    report = coordinator.apply(profile())
    assert report.keyd_error == "keyd reload failed"
    assert first.read_text(encoding="utf-8") == "key_key.back:key.keyboard.s\n"
    assert report.instances[0].status == "failed"


def test_options_write_failure_rolls_back_prior_instance_and_keyd(
    setup: tuple[Path, Path, FakeKeyd], tmp_path: Path, monkeypatch: pytest.MonkeyPatch
) -> None:
    from mcsr_toolkit import apply as apply_module

    root, first, keyd = setup
    second = make_instance(root, "two")
    coordinator = ApplyCoordinator(root, tmp_path / "state", keyd, lambda: [])
    original_write = apply_module._atomic_text

    def fail_second_once(path: Path, contents: str) -> None:
        if path == second and contents.endswith("key.keyboard.k\n"):
            raise OSError("disk full")
        original_write(path, contents)

    monkeypatch.setattr(apply_module, "_atomic_text", fail_second_once)
    report = coordinator.apply(profile())
    assert {entry.status for entry in report.instances} == {"failed"}
    assert first.read_text(encoding="utf-8") == "key_key.back:key.keyboard.s\n"
    assert second.read_text(encoding="utf-8") == "key_key.back:key.keyboard.s\n"
    assert keyd.rollbacks == 1
    assert keyd.restores == 0


def test_options_mode_is_preserved(setup: tuple[Path, Path, FakeKeyd], tmp_path: Path) -> None:
    root, first, keyd = setup
    first.chmod(0o640)
    coordinator = ApplyCoordinator(root, tmp_path / "state", keyd, lambda: [])
    coordinator.apply(profile())
    assert stat.S_IMODE(first.stat().st_mode) == 0o640


def test_restore_rejects_unknown_backup(setup: tuple[Path, Path, FakeKeyd], tmp_path: Path) -> None:
    root, _, keyd = setup
    coordinator = ApplyCoordinator(root, tmp_path / "state", keyd, lambda: [])
    with pytest.raises(ValueError, match="unknown backup"):
        coordinator.restore("../other")


def test_backup_directory_failure_prevents_keyd_install(
    setup: tuple[Path, Path, FakeKeyd],
    tmp_path: Path,
) -> None:
    root, first, keyd = setup
    state = tmp_path / "state"
    state.mkdir()
    (state / "backups").write_text("not a directory")
    coordinator = ApplyCoordinator(root, state, keyd, lambda: [])
    report = coordinator.apply(profile())
    assert {entry.status for entry in report.instances} == {"failed"}
    assert keyd.installs == 0
    assert first.read_text() == "key_key.back:key.keyboard.s\n"


def test_backup_write_failure_prevents_keyd_install(
    setup: tuple[Path, Path, FakeKeyd],
    tmp_path: Path,
    monkeypatch: pytest.MonkeyPatch,
) -> None:
    from mcsr_toolkit import apply as apply_module

    root, first, keyd = setup
    original = apply_module._atomic_text

    def fail_backup(path: Path, contents: str) -> None:
        if "backups" in path.parts:
            raise OSError("backup storage full")
        original(path, contents)

    monkeypatch.setattr(apply_module, "_atomic_text", fail_backup)
    report = ApplyCoordinator(root, tmp_path / "state", keyd, lambda: []).apply(profile())
    assert {entry.status for entry in report.instances} == {"failed"}
    assert keyd.installs == 0
    assert keyd.rollbacks == 0
    assert first.read_text() == "key_key.back:key.keyboard.s\n"


def test_later_state_read_failure_prevents_keyd_install(
    setup: tuple[Path, Path, FakeKeyd],
    tmp_path: Path,
    monkeypatch: pytest.MonkeyPatch,
) -> None:
    root, first, keyd = setup
    coordinator = ApplyCoordinator(root, tmp_path / "state", keyd, lambda: [])
    original = coordinator._state
    calls = 0

    def read_state() -> object:
        nonlocal calls
        calls += 1
        if calls > 1:
            raise ValueError("corrupted managed state")
        return original()

    monkeypatch.setattr(coordinator, "_state", read_state)
    report = coordinator.apply(profile())
    assert {entry.status for entry in report.instances} == {"failed"}
    assert keyd.installs == 0
    assert first.read_text() == "key_key.back:key.keyboard.s\n"


def test_backup_cannot_be_restored_into_another_launcher_root(tmp_path: Path) -> None:
    first_root = tmp_path / "first"
    second_root = tmp_path / "second"
    first = make_instance(first_root, "one")
    second = make_instance(second_root, "one", "key_key.back:key.keyboard.x\n")
    keyd = FakeKeyd()
    state_dir = tmp_path / "state"
    report = ApplyCoordinator(first_root, state_dir, keyd, lambda: []).apply(profile())
    assert report.backup_id is not None
    assert first.read_text() == "key_key.back:key.keyboard.k\n"

    with pytest.raises(ValueError, match="launcher root"):
        ApplyCoordinator(second_root, state_dir, keyd, lambda: []).restore(report.backup_id)

    assert first.read_text() == "key_key.back:key.keyboard.k\n"
    assert second.read_text() == "key_key.back:key.keyboard.x\n"
    assert keyd.restores == 0


def test_restore_tracks_prior_profile_across_multiple_applies(setup, tmp_path):
    root, target, keyd = setup
    coordinator = ApplyCoordinator(root, tmp_path / 'state', keyd, lambda: [])
    first = coordinator.apply(profile())
    second = coordinator.apply(Profile(crafting={'s': 'j'}))
    restored = coordinator.restore(second.backup_id)
    assert target.read_text() == 'key_key.back:key.keyboard.k\n'
    assert keyd.current == profile()
    assert restored.restored_profile == profile()
    coordinator.restore(first.backup_id)
    assert target.read_text() == 'key_key.back:key.keyboard.s\n'
    assert keyd.current is None


def test_repeat_apply_does_not_grow_keyd_history(setup, tmp_path):
    root, _, keyd = setup
    coordinator = ApplyCoordinator(root, tmp_path / 'state', keyd, lambda: [])
    first = coordinator.apply(profile())
    repeated = coordinator.apply(profile())
    assert repeated.backup_id is None
    assert keyd.installs == 1
    coordinator.restore(first.backup_id)
    assert keyd.current is None


def test_stale_restore_rejected_before_options_or_focus_changes(setup, tmp_path):
    root, target, keyd = setup
    coordinator = ApplyCoordinator(root, tmp_path / 'state', keyd, lambda: [])
    first = coordinator.apply(profile())
    second = coordinator.apply(Profile(crafting={'s': 'j'}))
    callbacks = []
    with pytest.raises(ValueError, match='stale'):
        coordinator.restore(first.backup_id, before_restore=lambda: callbacks.append(True))
    assert callbacks == []
    assert target.read_text() == 'key_key.back:key.keyboard.j\n'
    coordinator.restore(second.backup_id)
    with pytest.raises(ValueError, match='stale'):
        coordinator.restore(second.backup_id)


def test_keyd_restore_failure_precedes_options_mutation(setup, tmp_path):
    root, target, keyd = setup
    coordinator = ApplyCoordinator(root, tmp_path / 'state', keyd, lambda: [])
    report = coordinator.apply(profile())
    keyd.fail_restore = True
    with pytest.raises(RuntimeError, match='restore denied'):
        coordinator.restore(report.backup_id)
    assert target.read_text() == 'key_key.back:key.keyboard.k\n'
    assert keyd.current == profile()


def test_profile_only_change_creates_restorable_backup(setup, tmp_path):
    root, target, keyd = setup
    coordinator = ApplyCoordinator(root, tmp_path / 'state', keyd, lambda: [])
    coordinator.apply(profile())
    alternate = Profile(crafting={'s': 'k'}, chat={'open_chat': 'y'})
    changed = coordinator.apply(alternate)
    assert changed.backup_id is not None
    restored = coordinator.restore(changed.backup_id)
    assert restored.restored_profile == profile()
    assert keyd.current == profile()
    assert target.read_text() == 'key_key.back:key.keyboard.k\n'


def test_external_options_edit_rejects_restore_before_keyd_mutation(setup, tmp_path):
    root, target, keyd = setup
    coordinator = ApplyCoordinator(root, tmp_path / 'state', keyd, lambda: [])
    report = coordinator.apply(profile())
    target.write_text('key_key.back:key.keyboard.x\n')
    with pytest.raises(ValueError, match='changed'):
        coordinator.restore(report.backup_id)
    assert keyd.current == profile()
    assert keyd.restores == 0


def test_partial_restore_blocks_future_mutation_after_restart(setup, tmp_path, monkeypatch):
    from mcsr_toolkit import apply as apply_module

    root, target, keyd = setup
    second = make_instance(root, 'two')
    state = tmp_path / 'state'
    coordinator = ApplyCoordinator(root, state, keyd, lambda: [])
    report = coordinator.apply(profile())
    original = apply_module._atomic_text

    def fail_second(path, contents):
        if path == second and contents.endswith('key.keyboard.s\n'):
            raise OSError('restore disk full')
        original(path, contents)

    monkeypatch.setattr(apply_module, '_atomic_text', fail_second)
    with pytest.raises(OSError, match='disk full'):
        coordinator.restore(report.backup_id)
    assert (state / 'mutation.json').is_file()
    restarted = ApplyCoordinator(root, state, keyd, lambda: [])
    with pytest.raises(ValueError, match='recovery'):
        restarted.apply(profile())
    with pytest.raises(ValueError, match='recovery'):
        restarted.restore(report.backup_id)
    assert target.read_text() == 'key_key.back:key.keyboard.s\n'


def test_restore_rejects_checkpoint_that_disagrees_with_parent_backup(setup, tmp_path):
    root, target, keyd = setup
    state = tmp_path / 'state'
    coordinator = ApplyCoordinator(root, state, keyd, lambda: [])
    coordinator.apply(profile())
    second = coordinator.apply(Profile(crafting={'s': 'j'}))
    manifest_path = state / 'backups' / second.backup_id / 'manifest.json'
    manifest = json.loads(manifest_path.read_text())
    manifest['prior_history']['keyd'] = []
    manifest_path.write_text(json.dumps(manifest))
    callbacks = []
    with pytest.raises(ValueError, match='lineage'):
        coordinator.restore(second.backup_id, before_restore=lambda: callbacks.append(True))
    assert callbacks == []
    assert keyd.current == Profile(crafting={'s': 'j'})
    assert target.read_text() == 'key_key.back:key.keyboard.j\n'


def test_restore_new_instance_backup_rejects_incompatible_prior_options(setup, tmp_path):
    root, _, keyd = setup
    coordinator = ApplyCoordinator(root, tmp_path / 'state', keyd, lambda: [])
    coordinator.apply(profile())
    second = make_instance(root, 'two')
    report = coordinator.reconcile_new_instances(profile())
    with pytest.raises(ValueError, match='prior profile|lineage'):
        coordinator.restore(report.backup_id)
    assert second.read_text() == 'key_key.back:key.keyboard.k\n'
    assert keyd.current == profile()


def test_restore_running_instance_rejected_before_callback(setup, tmp_path):
    root, target, keyd = setup
    processes = []
    coordinator = ApplyCoordinator(root, tmp_path / 'state', keyd, lambda: processes)
    report = coordinator.apply(profile())
    processes.append(ProcessInfo(target.parent))
    callbacks = []
    with pytest.raises(ValueError, match='running'):
        coordinator.restore(report.backup_id, before_restore=lambda: callbacks.append(True))
    assert callbacks == []
    assert keyd.restores == 0


def test_legacy_or_missing_lineage_blocks_restore_and_apply(setup, tmp_path):
    root, target, keyd = setup
    state = tmp_path / 'state'
    coordinator = ApplyCoordinator(root, state, keyd, lambda: [])
    report = coordinator.apply(profile())
    (state / 'lineage.json').unlink()
    operations = (lambda: coordinator.apply(profile()),
                  lambda: coordinator.restore(report.backup_id))
    for operation in operations:
        with pytest.raises(ValueError, match='legacy'):
            operation()
    assert target.read_text() == 'key_key.back:key.keyboard.k\n'
    assert keyd.restores == 0


def test_runtime_publication_failure_retains_mutation_marker(setup, tmp_path):
    root, _, keyd = setup
    state = tmp_path / 'state'
    coordinator = ApplyCoordinator(root, state, keyd, lambda: [])
    report = coordinator.apply(profile())

    def fail_publication(restored_profile):
        raise OSError('active profile storage failed')

    with pytest.raises(OSError, match='active profile'):
        coordinator.restore(report.backup_id, after_restore=fail_publication)
    assert (state / 'mutation.json').is_file()
    with pytest.raises(ValueError, match='recovery'):
        coordinator.apply(profile())


def test_real_snapshot_adapter_restores_previous_config_without_system_commands(setup, tmp_path):
    import base64
    import subprocess

    from mcsr_toolkit.keyd import PrivilegedKeydAdapter, render_keyd_fragment
    from mcsr_toolkit.models import supported_keys
    from mcsr_toolkit.privileged import DEVICE_NAME, FRAGMENT_NAME, KeydInstaller

    root, target, _ = setup
    config = tmp_path / 'keyd-config'
    config.mkdir()
    installer = KeydInstaller(config, tmp_path / 'root-backups', lambda: None, supported_keys())

    def runner(args):
        command, payload = args[-2:]
        if command == 'install':
            backup = installer.install(base64.b64decode(payload).decode())
            return subprocess.CompletedProcess(args, 0, json.dumps({'backup_id': backup}))
        installer.restore(payload)
        return subprocess.CompletedProcess(args, 0, json.dumps({'restored': payload}))

    state = tmp_path / 'state'
    adapter = PrivilegedKeydAdapter(state / 'keyd-adapter.json', runner)
    coordinator = ApplyCoordinator(root, state, adapter, lambda: [])
    first = coordinator.apply(profile())
    second = coordinator.apply(Profile(crafting={'s': 'j'}, chat={'open_chat': 'y'}))
    restarted = ApplyCoordinator(
        root, state, PrivilegedKeydAdapter(state / 'keyd-adapter.json', runner), lambda: []
    )
    restarted.restore(second.backup_id)
    assert (config / FRAGMENT_NAME).read_text() == render_keyd_fragment(profile())
    assert target.read_text() == 'key_key.back:key.keyboard.k\n'
    restarted.restore(first.backup_id)
    assert not (config / DEVICE_NAME).exists()
    assert not (config / FRAGMENT_NAME).exists()
    assert target.read_text() == 'key_key.back:key.keyboard.s\n'


def test_apply_restore_preserves_changed_and_unchanged_crlf_options(setup, tmp_path):
    root, first, keyd = setup
    initial = b'fov:0.8\r\nkey_key.back:key.keyboard.s\r\n'
    first.write_bytes(initial)
    unchanged = make_instance(root, 'two', 'key_key.back:key.keyboard.x\n')
    other_initial = b'key_key.back:key.keyboard.x\r\n'
    unchanged.write_bytes(other_initial)
    coordinator = ApplyCoordinator(root, tmp_path / 'state', keyd, lambda: [])
    report = coordinator.apply(profile())
    assert first.read_bytes() == initial.replace(b'keyboard.s', b'keyboard.k')
    assert unchanged.read_bytes() == other_initial
    coordinator.restore(report.backup_id)
    assert first.read_bytes() == initial
    assert unchanged.read_bytes() == other_initial


def test_apply_final_directory_sync_failure_keeps_recovery_marker(setup, tmp_path, monkeypatch):
    from mcsr_toolkit import apply as apply_module

    root, _, keyd = setup
    state = tmp_path / 'state'
    state.mkdir(mode=0o700)
    (state / 'pending.json').write_text(profile().model_dump_json())
    coordinator = ApplyCoordinator(root, state, keyd, lambda: [])
    original = apply_module._sync_directory
    failed = False

    def fail_final_sync(path):
        nonlocal failed
        if (not failed and path == state and (state / 'lineage.json').exists()
                and not (state / 'pending.json').exists()):
            failed = True
            raise OSError('final directory sync failed')
        original(path)

    monkeypatch.setattr(apply_module, '_sync_directory', fail_final_sync)
    with pytest.raises(OSError, match='final directory sync'):
        coordinator.apply(profile())
    assert failed
    assert (state / 'mutation.json').is_file()
    with pytest.raises(ValueError, match='recovery'):
        coordinator.apply(profile())


def test_unchanged_options_preserve_new_physical_intent_and_restore_lineage(setup, tmp_path):
    root, options, keyd = setup
    coordinator = ApplyCoordinator(root, tmp_path / 'state', keyd, lambda: [])
    first = coordinator.apply(Profile(crafting={'s': 'k'}))
    second = coordinator.apply(Profile(controls={'key_key.back': 'j'}, crafting={'j': 'k'}))
    assert options.read_text() == 'key_key.back:key.keyboard.k\n'
    assert second.instances[0].status == 'unchanged'
    managed = json.loads((tmp_path / 'state' / 'managed.json').read_text())
    assert managed['one']['original']['key_key.back'] == 'j'
    third = coordinator.apply(Profile(crafting={'j': 'h'}))
    assert options.read_text() == 'key_key.back:key.keyboard.h\n'
    coordinator.restore(third.backup_id)
    coordinator.restore(second.backup_id)
    assert options.read_text() == 'key_key.back:key.keyboard.k\n'
    coordinator.restore(first.backup_id)
    assert options.read_text() == 'key_key.back:key.keyboard.s\n'


@pytest.mark.parametrize('value', ['31', 'garbage', 'key.keyboard.dot', 'key.keyboard.volumeup'])
def test_unsupported_bound_controls_fail_preview_and_block_activation(setup, tmp_path, value):
    root, options, keyd = setup
    options.write_text(f'key_key.back:{value}\n')
    coordinator = ApplyCoordinator(root, tmp_path / 'state', keyd, lambda: [])
    preview = coordinator.preview(profile())
    assert preview.instances[0].status == 'failed'
    assert 'unsupported Minecraft binding' in preview.instances[0].detail
    assert coordinator.apply(profile()).instances[0].status == 'failed'
    assert keyd.installs == 0


def test_custom_chat_entries_apply_to_every_instance_and_restore_originals(setup, tmp_path):
    root, first, keyd = setup
    original = 'key_key.back:key.keyboard.s\n'
    second = make_instance(root, 'two', 'gamma:1.0\nkey_key.back:key.keyboard.s')
    second_original = second.read_bytes()
    coordinator = ApplyCoordinator(root, tmp_path / 'state', keyd, lambda: [])
    configured = Profile(chat={
        'open_chat': 'y', 'open_command': 'p', 'send': 'tab', 'cancel': 'backspace',
    })
    report = coordinator.apply(configured)
    for target in (first, second):
        assert 'key_key.chat:key.keyboard.y\n' in target.read_text()
        assert 'key_key.command:key.keyboard.p\n' in target.read_text()
    assert coordinator.preview(configured).instances[0].status == 'unchanged'
    coordinator.restore(report.backup_id)
    assert first.read_text() == original
    assert second.read_bytes() == second_original
