"""Apply Minecraft control changes with recoverable per-instance backups."""

from __future__ import annotations

import hashlib
import json
import os
import re
import stat
import tempfile
import uuid
from collections.abc import Callable
from dataclasses import dataclass, field
from pathlib import Path
from typing import Any, Protocol

from mcsr_toolkit.bindings import ManagedState, plan_controls
from mcsr_toolkit.instances import Instance, ProcessInfo, discover_instances
from mcsr_toolkit.models import Profile
from mcsr_toolkit.options import parse_options, render_options


class KeydAdapter(Protocol):
    def install(self, profile: Profile) -> None: ...

    def checkpoint(self) -> list[str]: ...

    def restore_to(self, checkpoint: list[str]) -> None: ...

    def rollback(self) -> None: ...


@dataclass(frozen=True)
class InstanceResult:
    instance_id: str
    status: str
    detail: str = ""


@dataclass(frozen=True)
class ControlMapping:
    instance_id: str
    physical: str
    emitted: str
    action: str


@dataclass(frozen=True)
class ApplyPreview:
    instances: list[InstanceResult]
    mappings: list[ControlMapping] = field(default_factory=list)


@dataclass(frozen=True)
class ApplyReport:
    instances: list[InstanceResult]
    backup_id: str | None = None
    keyd_error: str | None = None
    restored_profile: Profile | None = None


@dataclass(frozen=True)
class _Prepared:
    instance: Instance
    old: str
    new: str
    state: ManagedState


def _atomic_text(path: Path, contents: str) -> None:
    fd, name = tempfile.mkstemp(prefix=f".{path.name}.", dir=path.parent)
    try:
        with os.fdopen(fd, "w", encoding="utf-8", newline="") as stream:
            if path.exists():
                os.fchmod(stream.fileno(), stat.S_IMODE(path.stat().st_mode))
            stream.write(contents)
            stream.flush()
            os.fsync(stream.fileno())
        os.replace(name, path)
        _sync_directory(path.parent)
    finally:
        if os.path.exists(name):
            os.unlink(name)


def _sync_directory(path: Path) -> None:
    fd = os.open(path, os.O_RDONLY | os.O_DIRECTORY)
    try:
        os.fsync(fd)
    finally:
        os.close(fd)


class ApplyCoordinator:
    def __init__(
        self,
        launcher_root: Path,
        state_dir: Path,
        keyd: KeydAdapter,
        processes: Callable[[], list[ProcessInfo]],
    ) -> None:
        self.launcher_root = launcher_root.resolve()
        self.state_dir = state_dir
        self.keyd = keyd
        self.processes = processes

    def _state(self) -> dict[str, ManagedState]:
        path = self.state_dir / "managed.json"
        if not path.exists():
            return {}
        data = json.loads(path.read_text(encoding="utf-8"))
        return {
            key: ManagedState(value["original"], value["last_emitted"])
            for key, value in data.items()
        }

    def _save_state(self, state: dict[str, ManagedState]) -> None:
        self.state_dir.mkdir(parents=True, exist_ok=True)
        _atomic_text(
            self.state_dir / "managed.json",
            json.dumps(
                {
                    key: {"original": value.original, "last_emitted": value.last_emitted}
                    for key, value in state.items()
                },
                sort_keys=True,
            ),
        )

    def _is_running(self, instance: Instance, processes: list[ProcessInfo]) -> bool:
        return any(process.cwd.resolve().is_relative_to(instance.path) for process in processes)

    def _prepare(
        self, profile: Profile
    ) -> tuple[list[_Prepared], list[InstanceResult], list[ControlMapping]]:
        state = self._state()
        processes = self.processes()
        prepared: list[_Prepared] = []
        results: list[InstanceResult] = []
        mappings: list[ControlMapping] = []
        for instance in discover_instances(self.launcher_root):
            if instance.problem:
                results.append(InstanceResult(instance.id, "failed", instance.problem))
                continue
            try:
                old = instance.options_path.read_bytes().decode("utf-8")
                document = parse_options(old)
                plan = plan_controls(document, profile, state.get(instance.id))
                mappings.extend(
                    ControlMapping(instance.id, physical, emitted, action)
                    for physical, emitted, action in plan.preview
                )
                if plan.conflicts:
                    raise ValueError("; ".join(plan.conflicts))
                new = render_options(document, plan)
            except (OSError, UnicodeError, ValueError) as error:
                results.append(InstanceResult(instance.id, "failed", str(error)))
                continue
            if new == old:
                results.append(InstanceResult(instance.id, "unchanged"))
                if plan.next_state != state.get(instance.id, ManagedState()):
                    prepared.append(_Prepared(instance, old, new, plan.next_state))
            elif self._is_running(instance, processes):
                results.append(InstanceResult(instance.id, "deferred"))
            else:
                prepared.append(_Prepared(instance, old, new, plan.next_state))
                results.append(InstanceResult(instance.id, "ready"))
        return prepared, results, mappings

    def preview(self, profile: Profile) -> ApplyPreview:
        _, instances, mappings = self._prepare(profile)
        return ApplyPreview(instances, mappings)

    def _record(self, path: Path) -> dict[str, Any]:
        if path.is_symlink():
            raise ValueError("unsafe recovery record")
        info = path.stat()
        if not stat.S_ISREG(info.st_mode) or info.st_uid != os.getuid() or info.st_mode & 0o077:
            raise ValueError("unsafe recovery record")
        data = json.loads(path.read_text(encoding="utf-8"))
        if not isinstance(data, dict):
            raise ValueError("invalid recovery record")
        return data

    def _history(self) -> dict[str, Any]:
        for path in (self.state_dir, self.state_dir / "backups"):
            if path.is_symlink():
                raise ValueError("unsafe recovery directory")
            if path.is_dir():
                info = path.stat()
                if info.st_uid != os.getuid():
                    raise ValueError("unsafe recovery directory")
                # Profile storage may have created this directory under a shared
                # umask. Recovery records must live in a private owned directory.
                path.chmod(0o700)
        journal = self.state_dir / "mutation.json"
        if journal.exists() or journal.is_symlink():
            raise ValueError("unfinished mutation requires manual recovery")
        path = self.state_dir / "lineage.json"
        checkpoint = self.keyd.checkpoint()
        if not path.exists() and not path.is_symlink():
            if checkpoint or (self.state_dir / "managed.json").exists():
                raise ValueError("ambiguous legacy history requires manual recovery")
            return {
                "head": None,
                "profile": None,
                "keyd": [],
                "launcher_root": str(self.launcher_root),
            }
        history = self._record(path)
        if history.get("launcher_root") != str(self.launcher_root):
            raise ValueError("history belongs to a different launcher root")
        if (
            set(history) != {"head", "profile", "keyd", "launcher_root"}
            or history["launcher_root"] != str(self.launcher_root)
            or history["keyd"] != checkpoint
            or (history["head"] is not None and not self._valid_id(history["head"]))
            or ((history["head"] is None) != (history["profile"] is None))
            or ((history["head"] is None) != (not history["keyd"]))
        ):
            raise ValueError("stale or ambiguous recovery history")
        if history["profile"] is not None:
            Profile.model_validate(history["profile"])
        return history

    @staticmethod
    def _valid_id(value: Any) -> bool:
        return isinstance(value, str) and re.fullmatch(r"[a-f0-9]{32}", value) is not None

    @staticmethod
    def _encoded_state(state: dict[str, ManagedState]) -> dict[str, Any]:
        return {
            key: {"original": value.original, "last_emitted": value.last_emitted}
            for key, value in state.items()
        }

    def _write_record(self, path: Path, data: dict[str, Any]) -> None:
        _atomic_text(path, json.dumps(data, sort_keys=True))

    def _record_failure(self, operation: str, backup_id: str | None, error: Exception) -> None:
        try:
            self._write_record(
                self.state_dir / "mutation.json",
                {
                    "operation": operation,
                    "backup_id": backup_id,
                    "error": str(error),
                },
            )
        except Exception as marker_error:
            raise ExceptionGroup(
                f"{operation} failed: {error}; recovery error could not be recorded",
                [error, marker_error],
            ) from error

    def _finish_apply(
        self,
        profile: Profile,
        backup_id: str | None,
        after_apply: Callable[[Profile], None],
    ) -> None:
        journal = self.state_dir / "mutation.json"
        try:
            after_apply(profile)
            _sync_directory(self.state_dir)
            # Unlink is last: a crash may conservatively resurrect the durable
            # marker, but cannot erase it before publication or syncing succeeds.
            journal.unlink()
        except Exception as error:
            self._record_failure("apply", backup_id, error)
            raise

    def _apply(
        self,
        profile: Profile,
        *,
        install_keyd: bool,
        after_apply: Callable[[Profile], None],
    ) -> ApplyReport:
        history = self._history()
        prepared, results, _ = self._prepare(profile)
        if any(item.status == "failed" for item in results):
            return ApplyReport(
                [
                    InstanceResult(
                        item.instance_id, "blocked", "another instance failed validation"
                    )
                    if item.status == "ready"
                    else item
                    for item in results
                ]
            )
        if any(item.status == "deferred" for item in results):
            if install_keyd:
                self.state_dir.mkdir(parents=True, exist_ok=True, mode=0o700)
                _atomic_text(self.state_dir / "pending.json", profile.model_dump_json())
            return ApplyReport(
                [
                    InstanceResult(item.instance_id, "deferred", "waiting for Minecraft to close")
                    if item.status == "ready"
                    else item
                    for item in results
                ]
            )
        profile_data = profile.model_dump(mode="json")
        install_keyd = install_keyd and profile_data != history["profile"]
        if not install_keyd and profile_data != history["profile"]:
            raise ValueError("reconciliation profile differs from installed profile")
        if not prepared and not install_keyd:
            self._write_record(
                self.state_dir / "mutation.json",
                {
                    "operation": "apply",
                    "backup_id": history["head"],
                },
            )
            (self.state_dir / "pending.json").unlink(missing_ok=True)
            self._finish_apply(profile, history["head"], after_apply)
            return ApplyReport(results)

        backup_id = uuid.uuid4().hex
        backup_dir = self.state_dir / "backups" / backup_id
        journal = self.state_dir / "mutation.json"
        prior_state: dict[str, ManagedState] = {}
        manifest: dict[str, Any] = {}
        try:
            self.state_dir.mkdir(parents=True, exist_ok=True, mode=0o700)
            backup_dir.parent.mkdir(exist_ok=True, mode=0o700)
            backup_dir.mkdir(exist_ok=False, mode=0o700)
            prior_state = self._state()
            next_state = dict(prior_state)
            changes = {item.instance.id: item for item in prepared}
            files: dict[str, str] = {}
            after_files: dict[str, str] = {}
            before_files: dict[str, str] = {}
            for instance in discover_instances(self.launcher_root):
                old = instance.options_path.read_bytes().decode("utf-8")
                item = changes.get(instance.id)
                new = item.new if item else old
                _atomic_text(backup_dir / f"{instance.id}.txt", old)
                files[instance.id] = str(instance.options_path.relative_to(self.launcher_root))
                before_files[instance.id] = hashlib.sha256(old.encode()).hexdigest()
                after_files[instance.id] = hashlib.sha256(new.encode()).hexdigest()
                if item:
                    next_state[instance.id] = item.state
            manifest = {
                "launcher_root": str(self.launcher_root),
                "files": files,
                "before_files": before_files,
                "after_files": after_files,
                "prior_state": self._encoded_state(prior_state),
                "post_state": self._encoded_state(next_state),
                "prior_history": history,
                "profile": profile_data,
                "keyd_after": None,
            }
            self._write_record(backup_dir / "manifest.json", manifest)
            _sync_directory(backup_dir.parent)
            self._write_record(journal, {"operation": "apply", "backup_id": backup_id})
        except (OSError, ValueError) as error:
            return ApplyReport(
                [
                    InstanceResult(item.instance_id, "failed", str(error))
                    if item.status in ("ready", "unchanged")
                    else item
                    for item in results
                ],
                backup_id=backup_id,
            )

        if install_keyd:
            try:
                self.keyd.install(profile)
            except Exception as error:
                # Keep the journal: an elevation timeout can occur after root-side mutation.
                self._record_failure("apply", backup_id, error)
                return ApplyReport(
                    [
                        InstanceResult(item.instance_id, "failed", str(error))
                        if item.status in ("ready", "unchanged")
                        else item
                        for item in results
                    ],
                    keyd_error=str(error),
                )

        updated: list[_Prepared] = []
        try:
            for item in prepared:
                if item.new != item.old:
                    _atomic_text(item.instance.options_path, item.new)
                    updated.append(item)
            self._save_state(next_state)
            checkpoint = self.keyd.checkpoint()
            manifest["keyd_after"] = checkpoint
            self._write_record(backup_dir / "manifest.json", manifest)
            self._write_record(
                self.state_dir / "lineage.json",
                {
                    "head": backup_id,
                    "profile": profile_data,
                    "keyd": checkpoint,
                    "launcher_root": str(self.launcher_root),
                },
            )
            (self.state_dir / "pending.json").unlink(missing_ok=True)
        except (OSError, ValueError) as error:
            self._record_failure("apply", backup_id, error)
            # Existing apply rollback remains useful, but the marker prevents trusting
            # history until all components have been inspected after a failed write.
            try:
                for item in reversed(updated):
                    _atomic_text(item.instance.options_path, item.old)
                if install_keyd:
                    self.keyd.rollback()
            except Exception as rollback_error:
                failure = RuntimeError(f"apply failed: {error}; rollback failed: {rollback_error}")
                self._record_failure("apply", backup_id, failure)
                raise failure from rollback_error
            return ApplyReport(
                [
                    InstanceResult(item.instance_id, "failed", str(error))
                    if item.status in ("ready", "unchanged")
                    else item
                    for item in results
                ],
                backup_id=backup_id,
            )
        self._finish_apply(profile, backup_id, after_apply)
        return ApplyReport(
            [
                InstanceResult(item.instance_id, "updated") if item.status == "ready" else item
                for item in results
            ],
            backup_id=backup_id,
        )

    def apply(
        self,
        profile: Profile,
        *,
        after_apply: Callable[[Profile], None] = lambda profile: None,
    ) -> ApplyReport:
        return self._apply(profile, install_keyd=True, after_apply=after_apply)

    def reconcile_new_instances(
        self,
        profile: Profile,
        *,
        after_apply: Callable[[Profile], None] = lambda profile: None,
    ) -> ApplyReport:
        pending_path = self.state_dir / "pending.json"
        if pending_path.is_file():
            profile = Profile.model_validate_json(pending_path.read_text(encoding="utf-8"))
        return self._apply(
            profile,
            install_keyd=pending_path.is_file(),
            after_apply=after_apply,
        )

    def restore(
        self,
        backup_id: str | None,
        *,
        before_restore: Callable[[], None] = lambda: None,
        after_restore: Callable[[Profile | None], None] = lambda profile: None,
    ) -> ApplyReport:
        if not self._valid_id(backup_id):
            raise ValueError("unknown backup")
        assert backup_id is not None
        history = self._history()
        backup_dir = self.state_dir / "backups" / backup_id
        manifest_path = backup_dir / "manifest.json"
        if backup_dir.is_symlink() or not manifest_path.is_file():
            raise ValueError("unknown backup")
        manifest = self._record(manifest_path)
        if manifest.get("launcher_root") != str(self.launcher_root):
            raise ValueError("backup belongs to a different launcher root")
        if history["head"] != backup_id:
            raise ValueError("stale backup is not the current recovery head")
        if (
            set(manifest)
            != {
                "launcher_root",
                "files",
                "before_files",
                "after_files",
                "prior_state",
                "post_state",
                "prior_history",
                "profile",
                "keyd_after",
            }
            or manifest["profile"] != history["profile"]
            or manifest["keyd_after"] != history["keyd"]
            or manifest["post_state"] != self._encoded_state(self._state())
        ):
            raise ValueError("changed or ambiguous backup state")
        prior = manifest["prior_history"]
        if (
            not isinstance(prior, dict)
            or set(prior) != {"head", "profile", "keyd", "launcher_root"}
            or prior["launcher_root"] != str(self.launcher_root)
            or not isinstance(prior["keyd"], list)
            or history["keyd"][: len(prior["keyd"])] != prior["keyd"]
            or (prior["head"] is not None and not self._valid_id(prior["head"]))
            or ((prior["head"] is None) != (prior["profile"] is None))
        ):
            raise ValueError("ambiguous backup lineage")
        if prior["head"] is None:
            if prior["keyd"] or manifest["prior_state"]:
                raise ValueError("ambiguous baseline lineage")
        else:
            parent_dir = self.state_dir / "backups" / prior["head"]
            if parent_dir.is_symlink():
                raise ValueError("unsafe parent backup lineage")
            parent = self._record(parent_dir / "manifest.json")
            if (
                parent.get("launcher_root") != str(self.launcher_root)
                or parent.get("profile") != prior["profile"]
                or parent.get("keyd_after") != prior["keyd"]
                or parent.get("post_state") != manifest["prior_state"]
                or not isinstance(parent.get("after_files"), dict)
                or any(
                    manifest["before_files"].get(key) != value
                    for key, value in parent["after_files"].items()
                )
            ):
                raise ValueError("changed or ambiguous parent backup lineage")
        restored_profile = (
            Profile.model_validate(prior["profile"]) if prior["profile"] is not None else None
        )
        prior_state = {
            key: ManagedState(value["original"], value["last_emitted"])
            for key, value in manifest["prior_state"].items()
        }
        instances = {item.id: item for item in discover_instances(self.launcher_root)}
        if set(instances) != set(manifest["files"]):
            raise ValueError("changed launcher instances make this backup ambiguous")
        prepared: list[tuple[str, Path, str]] = []
        processes = self.processes()
        for instance_id, relative in manifest["files"].items():
            if not isinstance(instance_id, str) or Path(instance_id).name != instance_id:
                raise ValueError("unsafe backup instance")
            instance = instances[instance_id]
            target = self.launcher_root / relative
            backup = backup_dir / f"{instance_id}.txt"
            if (
                instance.problem
                or target != instance.options_path
                or target.is_symlink()
                or not target.is_file()
                or not target.resolve().is_relative_to(self.launcher_root)
                or not backup.is_file()
                or backup.is_symlink()
            ):
                raise ValueError("unsafe backup target")
            if self._is_running(instance, processes):
                raise ValueError("cannot restore while Minecraft is running")
            contents = backup.read_bytes().decode("utf-8")
            if hashlib.sha256(target.read_bytes()).hexdigest() != manifest["after_files"].get(
                instance_id
            ) or hashlib.sha256(contents.encode()).hexdigest() != manifest["before_files"].get(
                instance_id
            ):
                raise ValueError("changed options or backup contents")
            if restored_profile is not None:
                document = parse_options(contents)
                plan = plan_controls(document, restored_profile, prior_state.get(instance_id))
                if plan.conflicts or render_options(document, plan) != contents:
                    raise ValueError("backup options do not match the prior profile")
            prepared.append((instance_id, target, contents))
        before_restore()
        journal = self.state_dir / "mutation.json"
        self._write_record(journal, {"operation": "restore", "backup_id": backup_id})
        # Authentication/helper failure must precede user-file mutation. Any later
        # failure retains the journal and focus suspension for explicit recovery.
        try:
            self.keyd.restore_to(prior["keyd"])
            for _, target, contents in prepared:
                _atomic_text(target, contents)
            self._save_state(prior_state)
            self._write_record(self.state_dir / "lineage.json", prior)
            (self.state_dir / "pending.json").unlink(missing_ok=True)
            after_restore(restored_profile)
            _sync_directory(self.state_dir)
            journal.unlink()
        except Exception as error:
            self._record_failure("restore", backup_id, error)
            raise
        return ApplyReport(
            [InstanceResult(instance_id, "restored") for instance_id, _, _ in prepared],
            backup_id=backup_id,
            restored_profile=restored_profile,
        )
