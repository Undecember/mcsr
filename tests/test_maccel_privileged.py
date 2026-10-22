from __future__ import annotations

import io
import os
from multiprocessing.connection import Connection
from multiprocessing.synchronize import Event
from pathlib import Path

import pytest

from mcsr_toolkit import maccel_privileged as helper


@pytest.fixture
def paths(tmp_path: Path, monkeypatch: pytest.MonkeyPatch) -> helper.HelperPaths:
    parameter = tmp_path / "SENS_MULT"
    parameter.write_text("4294967296\n")
    parameter.chmod(0o664)
    fingerprint = tmp_path / "srcversion"
    fingerprint.write_text("4AFA0F5B3EBD2F98EE54E56\n")
    fingerprint.chmod(0o444)
    # This driver retains the submitted text and appends its own newline on read.
    original_write = os.write
    parameter_inode = parameter.stat().st_ino

    def sysfs_write(fd: int, data: bytes) -> int:
        if os.fstat(fd).st_ino == parameter_inode:
            os.ftruncate(fd, 0)
            return original_write(fd, data + b"\n") - 1
        return original_write(fd, data)

    monkeypatch.setattr(helper.os, "write", sysfs_write)
    return helper.HelperPaths(parameter, tmp_path / "runtime", fingerprint, os.geteuid(), tmp_path)


def invoke(paths: helper.HelperPaths, command: str, token: str = "") -> str:
    return helper.run(command, io.StringIO(token), paths)


def test_exact_q32_conversion() -> None:
    assert helper.to_raw("1") == 4294967296
    assert helper.to_raw("0.02083") == 89464169


@pytest.mark.parametrize("value", ["0", "-1", "NaN", "inf", "1e100"])
def test_conversion_rejects_invalid_values(value: str) -> None:
    with pytest.raises(ValueError):
        helper.to_raw(value)


@pytest.mark.parametrize(
    "arguments", [[], ["set"], ["eye", "1"], ["restore", "/tmp/x"], ["--help"]]
)
def test_main_rejects_arguments_before_opening_paths(arguments: list[str]) -> None:
    assert helper.main(arguments) != 0


def test_main_constructs_fixed_paths_ignoring_environment(monkeypatch: pytest.MonkeyPatch) -> None:
    captured: list[helper.HelperPaths] = []
    monkeypatch.setenv("MCSR_MACCEL_PARAMETER", "/tmp/evil")
    monkeypatch.setattr(helper.os, "geteuid", lambda: 0)

    def capture(command: str, stream: object, selected: helper.HelperPaths) -> str:
        captured.append(selected)
        return "a" * 64

    monkeypatch.setattr(helper, "run", capture)
    assert helper.main(["restore"]) == 0
    assert captured == [helper.HelperPaths()]
    assert captured[0].parameter == Path("/sys/module/maccel/parameters/SENS_MULT")
    assert captured[0].runtime == Path("/run/mcsr-toolkit-maccel")


def test_restore_rotates_generation_and_eye_restores_nonunit_baseline(
    paths: helper.HelperPaths,
) -> None:
    paths.parameter.write_text("6442450944\n")
    token = invoke(paths, "restore")
    assert len(token) == 64
    assert invoke(paths, "eye", token + "\n") == "0.02083000005222856998443603515625"
    assert paths.parameter.read_text() == "89464169\n"
    assert (paths.runtime / "baseline").read_text() == "6442450944\n"
    invoke(paths, "eye", token + "\n")
    assert (paths.runtime / "baseline").read_text() == "6442450944\n"
    new_token = invoke(paths, "restore")
    assert token != new_token
    assert paths.parameter.read_text() == "6442450944\n"
    assert not (paths.runtime / "baseline").exists()
    assert invoke(paths, "restore") != new_token
    assert paths.runtime.stat().st_mode & 0o777 == 0o700
    for file in paths.runtime.iterdir():
        assert file.stat().st_mode & 0o777 == 0o600


@pytest.mark.parametrize("token", ["", "x", "a" * 64, "a" * 65, "a" * 64 + "\nextra"])
def test_bad_or_stale_token_cannot_write(paths: helper.HelperPaths, token: str) -> None:
    invoke(paths, "restore")
    with pytest.raises(ValueError):
        invoke(paths, "eye", token)
    assert paths.parameter.read_text() == "4294967296\n"
    assert not (paths.runtime / "baseline").exists()


@pytest.mark.parametrize("raw", ["1.000000000\n", "nan\n", "0\n", "-1\n", "9223372036854775808\n"])
def test_unsupported_parameter_is_rejected(paths: helper.HelperPaths, raw: str) -> None:
    paths.parameter.write_text(raw)
    with pytest.raises(ValueError):
        invoke(paths, "restore")
    assert paths.parameter.read_text() == raw


@pytest.mark.parametrize("fingerprint", ["", "DEADBEEF\n"])
def test_unknown_driver_is_rejected(paths: helper.HelperPaths, fingerprint: str) -> None:
    paths.fingerprint.chmod(0o644)
    paths.fingerprint.write_text(fingerprint)
    paths.fingerprint.chmod(0o444)
    with pytest.raises(ValueError):
        invoke(paths, "restore")
    assert not paths.runtime.exists()


@pytest.mark.parametrize(
    "target", ["runtime", "parameter", "fingerprint", "lock", "generation", "baseline"]
)
def test_symlinks_are_rejected(paths: helper.HelperPaths, target: str, tmp_path: Path) -> None:
    token = invoke(paths, "restore")
    outside = tmp_path / "outside"
    outside.write_text("unchanged")
    selected = (
        getattr(paths, target)
        if target in {"runtime", "parameter", "fingerprint"}
        else paths.runtime / target
    )
    if selected.is_dir():
        for child in selected.iterdir():
            child.unlink()
        selected.rmdir()
    else:
        selected.unlink(missing_ok=True)
    selected.symlink_to(outside)
    with pytest.raises((ValueError, OSError)):
        invoke(paths, "eye", token + "\n")
    assert outside.read_text() == "unchanged"


@pytest.mark.parametrize(
    "target,mode",
    [
        ("runtime", 0o755),
        ("parameter", 0o666),
        ("lock", 0o644),
        ("generation", 0o644),
        ("baseline", 0o644),
    ],
)
def test_unsafe_modes_are_rejected(paths: helper.HelperPaths, target: str, mode: int) -> None:
    token = invoke(paths, "restore")
    selected = (
        getattr(paths, target) if target in {"runtime", "parameter"} else paths.runtime / target
    )
    if not selected.exists():
        selected.write_text("4294967296\n")
    selected.chmod(mode)
    with pytest.raises(ValueError):
        invoke(paths, "eye", token + "\n")
    assert paths.parameter.read_text() == "4294967296\n"


def test_unexpected_owner_rejected(paths: helper.HelperPaths) -> None:
    wrong_owner = helper.HelperPaths(
        paths.parameter, paths.runtime, paths.fingerprint, os.geteuid() + 1, paths.anchor
    )
    with pytest.raises(ValueError):
        invoke(wrong_owner, "restore")
    assert not paths.runtime.exists()


@pytest.mark.parametrize("record,text", [("generation", "garbage\n"), ("baseline", "nan\n")])
def test_malformed_record_rejected(paths: helper.HelperPaths, record: str, text: str) -> None:
    token = invoke(paths, "restore")
    file = paths.runtime / record
    file.write_text(text)
    file.chmod(0o600)
    with pytest.raises(ValueError):
        invoke(paths, "eye", token + "\n")
    assert paths.parameter.read_text() == "4294967296\n"


def test_missing_module_fails_without_runtime_records(paths: helper.HelperPaths) -> None:
    paths.parameter.unlink()
    with pytest.raises(FileNotFoundError):
        invoke(paths, "restore")
    assert not paths.runtime.exists()


@pytest.mark.parametrize("repeated", [False, True])
def test_baseline_is_synced_before_parameter_write(
    paths: helper.HelperPaths, monkeypatch: pytest.MonkeyPatch, repeated: bool
) -> None:
    token = invoke(paths, "restore")
    if repeated:
        invoke(paths, "eye", token + "\n")
    synced: set[int] = set()
    original_sync = os.fsync
    original_write = os.write

    def sync(fd: int) -> None:
        synced.add(os.fstat(fd).st_ino)
        original_sync(fd)

    def write(fd: int, data: bytes) -> int:
        if os.fstat(fd).st_ino == paths.parameter.stat().st_ino:
            baseline = paths.runtime / "baseline"
            assert baseline.read_text() == "4294967296\n"
            assert baseline.stat().st_ino in synced
            assert paths.runtime.stat().st_ino in synced
        return original_write(fd, data)

    monkeypatch.setattr(helper.os, "fsync", sync)
    monkeypatch.setattr(helper.os, "write", write)
    invoke(paths, "eye", token + "\n")


@pytest.mark.parametrize("failure", ["partial", "write", "readback"])
def test_eye_failure_retains_baseline_and_requires_restore(
    paths: helper.HelperPaths, monkeypatch: pytest.MonkeyPatch, failure: str
) -> None:
    token = invoke(paths, "restore")
    original_write = os.write

    def failing_write(fd: int, data: bytes) -> int:
        if os.fstat(fd).st_ino != paths.parameter.stat().st_ino:
            return original_write(fd, data)
        if failure == "write":
            raise OSError("injected write failure")
        if failure == "partial":
            return original_write(fd, data[:3])
        original_write(fd, b"123\n")
        return len(data)

    with monkeypatch.context() as patch:
        patch.setattr(helper.os, "write", failing_write)
        with pytest.raises(OSError):
            invoke(paths, "eye", token + "\n")
    assert (paths.runtime / "baseline").read_text() == "4294967296\n"
    assert not (paths.runtime / "generation").exists()
    with pytest.raises(ValueError):
        invoke(paths, "eye", token + "\n")
    invoke(paths, "restore")
    assert paths.parameter.read_text() == "4294967296\n"
    assert not (paths.runtime / "baseline").exists()


def test_failed_restore_revokes_generation_retains_baseline(
    paths: helper.HelperPaths, monkeypatch: pytest.MonkeyPatch
) -> None:
    token = invoke(paths, "restore")
    invoke(paths, "eye", token + "\n")
    original_write = os.write

    def failing_write(fd: int, data: bytes) -> int:
        if os.fstat(fd).st_ino == paths.parameter.stat().st_ino:
            raise OSError("injected restore failure")
        return original_write(fd, data)

    with monkeypatch.context() as patch:
        patch.setattr(helper.os, "write", failing_write)
        with pytest.raises(OSError):
            invoke(paths, "restore")
    assert (paths.runtime / "baseline").read_text() == "4294967296\n"
    assert not (paths.runtime / "generation").exists()
    with pytest.raises(ValueError):
        invoke(paths, "eye", token + "\n")
    invoke(paths, "restore")
    assert paths.parameter.read_text() == "4294967296\n"


def test_lock_wait_has_deadline(paths: helper.HelperPaths, monkeypatch: pytest.MonkeyPatch) -> None:
    import fcntl

    invoke(paths, "restore")
    with (paths.runtime / "lock").open("r") as held:
        fcntl.flock(held, fcntl.LOCK_EX)
        monkeypatch.setattr(helper, "LOCK_TIMEOUT", 0.01)
        with pytest.raises(TimeoutError):
            invoke(paths, "restore")


def _stale_eye_process(
    paths: helper.HelperPaths, token: str, waiting: Event, resume: Event, result: Connection
) -> None:
    original_sleep = helper.time.sleep

    def pause_after_blocked(seconds: float) -> None:
        waiting.set()
        if not resume.wait(5):
            raise TimeoutError("test restore never completed")
        original_sleep(seconds)

    helper.time.sleep = pause_after_blocked
    try:
        invoke(paths, "eye", token + "\n")
    except ValueError as error:
        result.send(str(error))
    except BaseException as error:
        result.send(repr(error))
    else:
        result.send("unexpected success")
    finally:
        result.close()


@pytest.mark.parametrize("preflight_fails", [False, True])
def test_stale_process_reads_token_before_wait_and_rejects_after_restore(
    paths: helper.HelperPaths, monkeypatch: pytest.MonkeyPatch, preflight_fails: bool
) -> None:
    import fcntl
    import multiprocessing

    token = invoke(paths, "restore")
    context = multiprocessing.get_context("spawn")
    waiting = context.Event()
    resume = context.Event()
    result, child_result = context.Pipe(duplex=False)
    held = (paths.runtime / "lock").open("r")
    fcntl.flock(held, fcntl.LOCK_EX)

    child = context.Process(
        target=_stale_eye_process, args=(paths, token, waiting, resume, child_result)
    )
    child.start()
    child_result.close()
    try:
        assert waiting.wait(5), "child never reached the held lock"
        held.close()
        if preflight_fails:
            original_open = helper._open_file

            def fail_driver_open(
                parent: int,
                name: str,
                selected: helper.HelperPaths,
                kind: str,
                flags: int = os.O_RDONLY,
            ) -> int:
                if kind == "fingerprint":
                    raise OSError("transient preflight failure")
                return original_open(parent, name, selected, kind, flags)

            with monkeypatch.context() as patch:
                patch.setattr(helper, "_open_file", fail_driver_open)
                with pytest.raises(OSError, match="transient preflight"):
                    invoke(paths, "restore")
        else:
            new_token = invoke(paths, "restore")
            assert new_token != token
        resume.set()
        assert result.poll(5), "child never returned a result"
        assert result.recv() == "stale generation"
        child.join(5)
        assert child.exitcode == 0
        assert paths.parameter.read_text() == "4294967296\n"
        assert not (paths.runtime / "baseline").exists()
    finally:
        held.close()
        resume.set()
        if child.is_alive():
            child.terminate()
        child.join(5)
        result.close()


def test_external_paths_and_commands_are_never_used(
    paths: helper.HelperPaths, monkeypatch: pytest.MonkeyPatch
) -> None:
    import subprocess

    original_open = os.open

    def restricted_open(
        path: object, flags: int, mode: int = 0o777, *, dir_fd: int | None = None
    ) -> int:
        selected = os.fspath(path)
        if os.path.isabs(selected):
            assert Path(selected).is_relative_to(paths.anchor)
        return original_open(selected, flags, mode, dir_fd=dir_fd)

    def forbidden(*args: object, **kwargs: object) -> None:
        pytest.fail("helper attempted to execute an external command")

    monkeypatch.setattr(helper.os, "open", restricted_open)
    monkeypatch.setattr(subprocess, "Popen", forbidden)
    monkeypatch.setattr(os, "system", forbidden)
    token = invoke(paths, "restore")
    invoke(paths, "eye", token + "\n")
    invoke(paths, "restore")


def test_unrecognized_runtime_entry_is_rejected(paths: helper.HelperPaths) -> None:
    token = invoke(paths, "restore")
    (paths.runtime / "unexpected").write_text("untouched")
    with pytest.raises(ValueError, match="unexpected runtime"):
        invoke(paths, "eye", token + "\n")
    assert paths.parameter.read_text() == "4294967296\n"


@pytest.mark.parametrize(
    "target", ["parameter", "fingerprint", "runtime", "lock", "generation", "baseline"]
)
def test_each_wrong_owner_is_rejected(
    paths: helper.HelperPaths, monkeypatch: pytest.MonkeyPatch, target: str
) -> None:
    token = invoke(paths, "restore")
    selected = (
        getattr(paths, target)
        if target in {"parameter", "fingerprint", "runtime"}
        else paths.runtime / target
    )
    if target == "baseline":
        selected.write_text("4294967296\n")
        selected.chmod(0o600)
    selected_inode = selected.stat().st_ino
    original_fstat = os.fstat

    def wrong_owner(fd: int) -> os.stat_result:
        info = original_fstat(fd)
        if info.st_ino == selected_inode:
            fields = list(info)
            fields[4] = os.geteuid() + 1
            return os.stat_result(fields)
        return info

    monkeypatch.setattr(helper.os, "fstat", wrong_owner)
    with pytest.raises(ValueError):
        invoke(paths, "eye", token + "\n")
    assert paths.parameter.read_text() == "4294967296\n"


@pytest.mark.parametrize("target", ["parameter", "lock", "generation", "baseline"])
def test_hardlinked_file_is_rejected(
    paths: helper.HelperPaths, tmp_path: Path, target: str
) -> None:
    token = invoke(paths, "restore")
    selected = paths.parameter if target == "parameter" else paths.runtime / target
    if target == "baseline":
        selected.write_text("4294967296\n")
        selected.chmod(0o600)
    os.link(selected, tmp_path / "alias")
    with pytest.raises(ValueError):
        invoke(paths, "eye", token + "\n")
    assert paths.parameter.read_text() == "4294967296\n"


def test_writable_parameter_ancestor_rejected(paths: helper.HelperPaths) -> None:
    subdirectory = paths.anchor / "writable"
    subdirectory.mkdir(mode=0o777)
    subdirectory.chmod(0o777)
    selected = subdirectory / "SENS_MULT"
    selected.write_text("4294967296\n")
    altered = helper.HelperPaths(
        selected, paths.runtime, paths.fingerprint, paths.owner_uid, paths.anchor
    )
    with pytest.raises(ValueError):
        invoke(altered, "restore")
    assert selected.read_text() == "4294967296\n"
    assert not paths.runtime.exists()


def test_special_runtime_file_is_rejected_without_blocking(paths: helper.HelperPaths) -> None:
    token = invoke(paths, "restore")
    os.mkfifo(paths.runtime / "baseline", 0o600)
    with pytest.raises(ValueError):
        invoke(paths, "eye", token + "\n")
    assert paths.parameter.read_text() == "4294967296\n"


def test_failed_baseline_sync_blocks_eye_until_restore(
    paths: helper.HelperPaths, monkeypatch: pytest.MonkeyPatch
) -> None:
    token = invoke(paths, "restore")
    original_sync = os.fsync

    def fail_baseline_sync(fd: int) -> None:
        baseline = paths.runtime / "baseline"
        if baseline.exists() and os.fstat(fd).st_ino == baseline.stat().st_ino:
            raise OSError("injected baseline fsync failure")
        original_sync(fd)

    with monkeypatch.context() as patch:
        patch.setattr(helper.os, "fsync", fail_baseline_sync)
        with pytest.raises(OSError):
            invoke(paths, "eye", token + "\n")
    assert paths.parameter.read_text() == "4294967296\n"
    with pytest.raises(ValueError):
        invoke(paths, "eye", token + "\n")
    invoke(paths, "restore")
    assert paths.parameter.read_text() == "4294967296\n"


@pytest.mark.parametrize("target", ["fingerprint", "parameter"])
def test_failed_restore_preflight_revokes_old_generation(
    paths: helper.HelperPaths, monkeypatch: pytest.MonkeyPatch, target: str
) -> None:
    token = invoke(paths, "restore")
    original_open = helper._open_file

    def fail_driver_open(
        parent: int,
        name: str,
        selected: helper.HelperPaths,
        kind: str,
        flags: int = os.O_RDONLY,
    ) -> int:
        if kind == target:
            raise OSError("transient preflight failure")
        return original_open(parent, name, selected, kind, flags)

    with monkeypatch.context() as patch:
        patch.setattr(helper, "_open_file", fail_driver_open)
        with pytest.raises(OSError, match="transient preflight"):
            invoke(paths, "restore")
    assert not (paths.runtime / "generation").exists()
    with pytest.raises(ValueError, match="stale generation"):
        invoke(paths, "eye", token + "\n")
    assert paths.parameter.read_text() == "4294967296\n"
    assert invoke(paths, "restore") != token


@pytest.mark.parametrize("unsafe", ["directory", "baseline", "generation"])
def test_failed_restore_preserves_unsafe_runtime_evidence(
    paths: helper.HelperPaths, unsafe: str
) -> None:
    token = invoke(paths, "restore")
    if unsafe == "directory":
        paths.runtime.chmod(0o755)
    elif unsafe == "baseline":
        (paths.runtime / "baseline").symlink_to(paths.parameter)
    else:
        (paths.runtime / "generation").chmod(0o644)
    with pytest.raises((OSError, ValueError)):
        invoke(paths, "restore")
    assert (paths.runtime / "generation").read_text() == token + "\n"
    assert paths.parameter.read_text() == "4294967296\n"


def test_driver_appended_newline_eye_and_exact_nonunit_restore(
    paths: helper.HelperPaths,
) -> None:
    paths.parameter.write_text("6442450944\n")
    token = invoke(paths, "restore")
    assert invoke(paths, "eye", token + "\n") == "0.02083000005222856998443603515625"
    assert paths.parameter.read_bytes() == b"89464169\n"
    assert (paths.runtime / "baseline").read_bytes() == b"6442450944\n"
    invoke(paths, "eye", token + "\n")
    assert invoke(paths, "restore") != token
    assert paths.parameter.read_bytes() == b"6442450944\n"
    assert not (paths.runtime / "baseline").exists()


def test_restore_recovers_existing_driver_double_newline(
    paths: helper.HelperPaths,
) -> None:
    token = invoke(paths, "restore")
    paths.parameter.write_bytes(b"89464169\n\n")
    baseline = paths.runtime / "baseline"
    baseline.write_bytes(b"6442450944\n")
    baseline.chmod(0o600)
    assert invoke(paths, "restore") != token
    assert paths.parameter.read_bytes() == b"6442450944\n"
    assert not baseline.exists()


def test_double_newline_initial_parameter_is_normalized_in_baseline(
    paths: helper.HelperPaths,
) -> None:
    paths.parameter.write_bytes(b"6442450944\n\n")
    token = invoke(paths, "restore")
    invoke(paths, "eye", token + "\n")
    assert (paths.runtime / "baseline").read_bytes() == b"6442450944\n"
    invoke(paths, "restore")
    assert paths.parameter.read_bytes() == b"6442450944\n"


@pytest.mark.parametrize(
    "raw", ["4294967296\n\n\n", "4294967296 ", " 4294967296\n", "4294967296\r\n", "4294967296\n "]
)
def test_parameter_rejects_unobserved_whitespace(
    paths: helper.HelperPaths,
    raw: str,
) -> None:
    paths.parameter.write_text(raw)
    with pytest.raises(ValueError):
        invoke(paths, "restore")
    assert paths.parameter.read_bytes() == raw.encode("ascii")


def test_double_newline_baseline_record_remains_invalid(
    paths: helper.HelperPaths,
) -> None:
    token = invoke(paths, "restore")
    baseline = paths.runtime / "baseline"
    baseline.write_bytes(b"6442450944\n\n")
    baseline.chmod(0o600)
    with pytest.raises(ValueError):
        invoke(paths, "restore")
    assert (paths.runtime / "generation").read_text() == token + "\n"
    assert baseline.read_bytes() == b"6442450944\n\n"
    assert paths.parameter.read_bytes() == b"4294967296\n"
