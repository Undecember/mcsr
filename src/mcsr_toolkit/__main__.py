"""Start the loopback API and persistent focus/resize bridge."""

from __future__ import annotations

import argparse
import asyncio
import fcntl
import json
import logging
import os
import shutil
import stat
import subprocess
import threading
import time
from collections.abc import AsyncIterator, Callable
from contextlib import asynccontextmanager
from dataclasses import asdict
from pathlib import Path
from typing import Any, TypeVar

import uvicorn
from fastapi import FastAPI

from mcsr_toolkit.api import Dependencies, create_app, service_config
from mcsr_toolkit.apply import ApplyCoordinator
from mcsr_toolkit.focus import FocusController, FocusEvent, is_minecraft_window
from mcsr_toolkit.focus_pipe import FocusPipe
from mcsr_toolkit.focus_watchdog import FocusWatchdog
from mcsr_toolkit.instances import ProcessInfo, locate_launcher_root
from mcsr_toolkit.keyd import KeydCommands, PrivilegedKeydAdapter, preflight_keyd_clear
from mcsr_toolkit.models import Profile
from mcsr_toolkit.profile import default_profile_path, load_profile, save_profile
from mcsr_toolkit.resize import GnomeWindowBus, PresetController, ResizeError
from mcsr_toolkit.sensitivity import SensitivityAdapter, SensitivityError

LOG = logging.getLogger(__name__)
T = TypeVar("T")


class ServiceLock:
    """Keep one reader attached to the GNOME focus FIFO per user session."""

    def __init__(self, path: Path) -> None:
        self.path = path
        self.fd: int | None = None

    def acquire(self) -> None:
        if self.fd is not None:
            raise RuntimeError("service lock is already held")
        fd = os.open(
            self.path,
            os.O_CREAT | os.O_RDWR | os.O_CLOEXEC | os.O_NOFOLLOW,
            0o600,
        )
        try:
            info = os.fstat(fd)
            if not stat.S_ISREG(info.st_mode) or info.st_uid != os.getuid() or info.st_mode & 0o077:
                raise ValueError("unsafe service lock")
            try:
                fcntl.flock(fd, fcntl.LOCK_EX | fcntl.LOCK_NB)
            except BlockingIOError as error:
                raise RuntimeError("service is already running") from error
        except Exception:
            os.close(fd)
            raise
        self.fd = fd

    def close(self) -> None:
        if self.fd is not None:
            os.close(self.fd)
            self.fd = None


def _processes() -> list[ProcessInfo]:
    found: list[ProcessInfo] = []
    for entry in Path("/proc").iterdir():
        if entry.name.isdecimal():
            try:
                found.append(ProcessInfo((entry / "cwd").resolve(strict=True)))
            except (OSError, RuntimeError):
                pass
    return found


class FocusKeydBridge:
    """Observe events until a reviewed profile has been applied with an operator opt-in."""

    def __init__(self, enabled: bool, active_profile: Path) -> None:
        self.enabled = enabled
        self.active_profile = active_profile
        self.suspended_path = active_profile.with_suffix(".suspended.json")
        self.suspended = self.suspended_path.is_file()
        self.profile: Profile | None = None
        self.cleanup_error: str | None = None
        self.events = {"activate": 0, "clear": 0}
        self.confirm_focus: Callable[[], bool] = lambda: False
        self.on_clear: Callable[[], None] = lambda: None
        if active_profile.is_file():
            self.profile = load_profile(active_profile)
        elif self.suspended:
            # Recovery must run even when the service restarts without live binds.
            self.profile = load_profile(self.suspended_path)

    def activate_game(self) -> None:
        if self.enabled and self.suspended:
            raise RuntimeError("live activation is suspended pending recovery")
        if self.enabled and self.profile is not None:
            if not self.confirm_focus():
                raise RuntimeError("live activation requires effective Minecraft focus")
            try:
                KeydCommands(self.profile).activate_game()
            except Exception as activation_error:
                # The daemon may have applied the bind before its client timed out.
                try:
                    self.clear_state()
                except Exception as clear_error:
                    raise RuntimeError(
                        f"activation failed: {activation_error}; cleanup failed: {clear_error}"
                    ) from clear_error
                raise
        self.events["activate"] += 1

    def clear_state(self) -> None:
        self.on_clear()
        self.events["clear"] += 1
        if self.profile is not None:
            self._clear_suspended()
            try:
                self.suspended_path.unlink(missing_ok=True)
            except Exception as error:
                self.cleanup_error = str(error)
                raise
            self.suspended = False

    def _clear_suspended(self) -> None:
        marker_error: Exception | None = None
        try:
            self.suspend()
        except Exception as error:
            marker_error = error
        clear_error: Exception | None = None
        try:
            self._clear_commands()
        except Exception as error:
            clear_error = error
        if clear_error is not None:
            if marker_error is not None:
                message = f"suspension failed: {marker_error}; cleanup failed: {clear_error}"
                self.cleanup_error = message
                raise RuntimeError(message) from clear_error
            raise clear_error
        if marker_error is not None:
            self.cleanup_error = str(marker_error)
            raise marker_error

    def _clear_commands(self) -> None:
        assert self.profile is not None
        try:
            KeydCommands(self.profile).clear_state()
        except Exception as error:
            self.cleanup_error = str(error)
            raise
        self.cleanup_error = None

    def applied(self, profile: Profile) -> None:
        if not self.enabled:
            raise RuntimeError("live activation is disabled")
        save_profile(self.active_profile, profile)
        self.profile = profile
        self.suspended_path.unlink(missing_ok=True)
        self.suspended = False
        self.cleanup_error = None

    def suspend(self) -> None:
        self.suspended = True
        save_profile(self.suspended_path, self.profile or Profile())

    def clear_recovery(self) -> None:
        if self.profile is not None:
            self.events["clear"] += 1
            self._clear_suspended()
        self.active_profile.unlink(missing_ok=True)
        self.profile = None

    def restoring(self) -> None:
        self.clear_recovery()

    def restored(self) -> None:
        if self.cleanup_error is not None:
            return
        self.active_profile.unlink(missing_ok=True)
        self.suspended_path.unlink(missing_ok=True)
        self.profile = None
        self.suspended = False
        self.cleanup_error = None

    def status(self) -> dict[str, Any]:
        mode = "observe-only"
        if self.cleanup_error and "stop failed" in self.cleanup_error:
            mode = "unsafe"
        elif self.suspended and self.profile is not None:
            mode = "cleanup-pending"
        elif self.enabled and self.profile is not None and not self.suspended:
            mode = "live"
        return {
            "mode": mode,
            "activation_suspended": self.suspended,
            "recovery_clear_pending": self.suspended and self.profile is not None,
            "cleanup_error": self.cleanup_error,
            "unsafe_stop_failure": bool(self.cleanup_error and "stop failed" in self.cleanup_error),
            "events": dict(self.events),
        }


class ServiceRuntime:
    def __init__(self, dependencies: Dependencies, *, runtime_dir: Path) -> None:
        self.dependencies = dependencies
        self.keyd = FocusKeydBridge(
            dependencies.live_activation_enabled,
            dependencies.state_dir / "active-profile.json",
        )
        self.sensitivity = SensitivityAdapter()
        self.preset = PresetController(GnomeWindowBus(), sensitivity=self.sensitivity)
        self._preset_lock = threading.RLock()
        self._transition_lock = asyncio.Lock()
        self._action_lock = threading.Lock()
        self._action_epoch = 0
        self._accept_presets = False
        self._preset_tasks: set[asyncio.Task[None]] = set()
        self.keyd.on_clear = self._focus_lost
        self.keyd.confirm_focus = self._confirm_game_focus
        self._mutation_cleared = False
        if self._mutation_pending():
            self.keyd.suspended = True
            try:
                marker = dependencies.state_dir / "mutation.json"
                if marker.is_symlink():
                    raise ValueError("unsafe mutation record")
                record = json.loads(marker.read_text(encoding="utf-8"))
                error = record.get("error")
                if isinstance(error, str):
                    self.keyd.cleanup_error = error
            except (OSError, ValueError, AttributeError):
                self.keyd.cleanup_error = "unreadable mutation record requires manual recovery"
        self._desired_profile = self.keyd.profile if not self.keyd.suspended else None
        self.reconciliation: dict[str, Any] | None = None
        self.controller = FocusController(self.keyd, self._preset_lock)
        self.pipe = FocusPipe(
            runtime_dir / "mcsr-toolkit-focus.fifo", FocusWatchdog(self.controller)
        )
        self.lock = ServiceLock(runtime_dir / "mcsr-toolkit-service.lock")
        self.stop_event = threading.Event()
        self.thread: threading.Thread | None = None
        self.errors: list[str] = []
        self.extension_connected = False
        self._extension_poll_interval = 0.5
        self._extension_retry_interval = 2.0
        self._preset_interface: Any = None
        self._next_configure = 0.0
        self.app: Any = None
        dependencies.focus_status = self.status
        dependencies.on_applied = self._applied
        dependencies.before_apply = self._before_apply
        dependencies.before_restore = self._before_restore
        dependencies.after_restore = self._restored
        dependencies.mutation_failed = self._mutation_failed
        dependencies.mutation_guard = lambda: self._preset_lock

    def status(self) -> dict[str, Any]:
        with self._preset_lock:
            active_preset = self.preset.active_preset
            last_seen = self.pipe.watchdog.last_seen
            extension_connected = (
                self.extension_connected
                and last_seen is not None
                and time.monotonic() - last_seen < self.pipe.watchdog.timeout
            )
            sensitivity_status = asdict(self.sensitivity.preflight())
        return {
            **self.keyd.status(),
            "mutation_recovery_pending": self._mutation_pending(),
            "minecraft_focused": self.controller.game_active,
            "extension_connected": extension_connected,
            "fifo_errors": self.pipe.errors[-5:],
            "errors": self.errors[-5:],
            "active_preset": active_preset,
            "reconciliation": self.reconciliation,
            "keyd_available": bool(shutil.which("keyd.rvaiya") or shutil.which("keyd")),
            "maccel": sensitivity_status,
        }

    def _record_error(self, error: Exception) -> None:
        message = str(error)
        if self.errors and self.errors[-1] == message:
            return
        self.errors.append(message)
        LOG.warning("service bridge: %s", error)

    def _applied(self, profile: Profile) -> None:
        with self._preset_lock:
            self.keyd.applied(profile)
            self._desired_profile = profile
        self._next_configure = 0.0

    def _before_apply(self, profile: Profile) -> None:
        with self._preset_lock:
            if self._mutation_pending():
                raise RuntimeError("unfinished mutation requires manual recovery")
            self._desired_profile = profile
            save_profile(self.dependencies.state_dir / "pending.json", profile)
            self._suspend_activation()
            if self.keyd.profile is not None:
                raise RuntimeError("cannot apply while keyd clearing is pending")

    def _before_restore(self) -> None:
        with self._preset_lock:
            self._prepare_restore()

    def _prepare_restore(self) -> None:
        self._desired_profile = None
        (self.dependencies.state_dir / "pending.json").unlink(missing_ok=True)
        self.controller.game_active = False
        self._focus_lost()
        try:
            self.keyd.restoring()
        except (OSError, ValueError, RuntimeError) as error:
            self._record_error(error)
            self.keyd.cleanup_error = str(error)
            raise
        if (self.dependencies.state_dir / "keyd-adapter.json").exists():
            recovery_profile = self.keyd.profile or Profile()
            try:
                recovery_profile = load_profile(self.dependencies.profile_path)
                KeydCommands(recovery_profile).clear_state()
            except (OSError, ValueError, RuntimeError) as error:
                self._record_error(error)
                self.keyd.profile = recovery_profile
                self.keyd.suspended = True
                self.keyd.cleanup_error = str(error)
                try:
                    save_profile(self.keyd.active_profile, recovery_profile)
                    self.keyd.suspend()
                except (OSError, ValueError, RuntimeError) as state_error:
                    self._record_error(state_error)
                    raise RuntimeError(
                        f"{error}; recovery state save failed: {state_error}"
                    ) from state_error
                raise

    def _mutation_failed(self, error: Exception) -> None:
        with self._preset_lock:
            self._desired_profile = None
            self.controller.game_active = False
            self.keyd.suspended = True
            self.keyd.cleanup_error = str(error)
            if self.keyd.profile is None and self.keyd.suspended_path.is_file():
                self.keyd.profile = load_profile(self.keyd.suspended_path)
            self._record_error(error)

    def _restored(self, profile: Profile | None = None) -> None:
        with self._preset_lock:
            self.keyd.restored()
            self._desired_profile = profile if self.keyd.enabled else None
            self.reconciliation = None
            if profile is not None:
                # Record the restored profile before removing the transaction marker.
                # Activation still requires a new effective-focus check.
                save_profile(self.keyd.active_profile, profile)
                self.keyd.profile = profile
                if not self.keyd.enabled:
                    self.keyd.suspend()

    def _mutation_pending(self) -> bool:
        path = self.dependencies.state_dir / "mutation.json"
        return path.exists() or path.is_symlink()

    def _confirm_game_focus(self) -> bool:
        if self._mutation_pending():
            return False
        event = GnomeWindowBus().get_focus()
        return not event.modal and is_minecraft_window(event)

    def _suspend_activation(self) -> None:
        with self._preset_lock:
            try:
                if self.keyd.profile is None:
                    self.keyd.suspend()
                else:
                    self.keyd.clear_recovery()
            except Exception as error:
                self._record_error(error)
            finally:
                self.controller.game_active = False
                self._focus_lost()
            if self._desired_profile is not None:
                save_profile(self.dependencies.state_dir / "pending.json", self._desired_profile)

    def _retry_clear(self) -> None:
        with self._preset_lock:
            if self.sensitivity.recovery_pending:
                self._focus_lost()
            if self._mutation_pending():
                self.keyd.suspended = True
                if self.keyd.profile is not None and not self._mutation_cleared:
                    try:
                        self.keyd._clear_suspended()
                        self._mutation_cleared = True
                    except Exception as error:
                        self._record_error(error)
                self.controller.game_active = False
                return
            self._mutation_cleared = False
            if self.keyd.suspended and self.keyd.profile is not None:
                try:
                    self.keyd.clear_recovery()
                    self.controller.game_active = False
                except Exception as error:
                    self._record_error(error)

    def _keyd_healthy(self) -> bool:
        try:
            result = subprocess.run(
                ["/usr/bin/systemctl", "is-active", "--quiet", "keyd.service"],
                capture_output=True, timeout=3, check=False,
            )
        except (OSError, subprocess.SubprocessError):
            return False
        return result.returncode == 0

    def _poll_keyd_health(self) -> None:
        with self._preset_lock:
            if self.keyd.enabled and self.keyd.profile is not None and not self.keyd.suspended:
                if not self._keyd_healthy():
                    self._record_error(RuntimeError("keyd service is not healthy"))
                    self._suspend_activation()

    def _disconnect_focus(self) -> None:
        with self._preset_lock:
            try:
                self.controller.on_disconnect()
            finally:
                self._focus_lost()

    def _focus_lost(self, *, epoch: int | None = None) -> None:
        # Keep epoch reservation independent of slow subprocesses under _preset_lock.
        # An older awaited reply may restore its work, but cannot cancel newer work.
        with self._action_lock:
            if epoch is None or epoch == self._action_epoch:
                self._action_epoch += 1
        self._restore_sensitivity()

    def _restore_sensitivity(self) -> None:
        try:
            with self._preset_lock:
                self.preset.focus_lost()
        except ResizeError as error:
            self._record_error(error)

    def start(self) -> None:
        self.lock.acquire()
        try:
            # Rotate the helper generation even when no local eye state survived.
            with self._preset_lock:
                try:
                    self.sensitivity.startup_restore()
                except SensitivityError as error:
                    self._record_error(error)
            clear_on_pipe_start = not self.keyd.suspended
            if self.keyd.profile is not None and not self.keyd.suspended:
                try:
                    if self.keyd.enabled:
                        self.keyd.clear_state()
                    else:
                        self.keyd.clear_recovery()
                        self._desired_profile = None
                except Exception as error:
                    self._record_error(error)
                    self._suspend_activation()
                clear_on_pipe_start = False
            self.pipe.start(clear_state=clear_on_pipe_start)
            self._accept_presets = True
        except Exception:
            self.lock.close()
            raise
        self.thread = threading.Thread(target=self._loop, name="mcsr-focus", daemon=True)
        self.thread.start()

    def _loop(self) -> None:
        next_reconcile = 0.0
        while not self.stop_event.is_set():
            self._retry_clear()
            if self.stop_event.is_set():
                break
            self._poll_keyd_health()
            if self.stop_event.is_set():
                break
            try:
                with self._preset_lock:
                    if self.stop_event.is_set():
                        break
                    self.pipe.run_once(0.2)
                    if not self.controller.game_active:
                        self._focus_lost()
            except Exception as error:
                if str(error) != "live activation is suspended pending recovery":
                    self._record_error(error)
                try:
                    self._disconnect_focus()
                except Exception as clear_error:
                    self._record_error(clear_error)
            if self.stop_event.is_set():
                break
            now = time.monotonic()
            if self.extension_connected and now >= self._next_configure:
                try:
                    GnomeWindowBus().configure_presets(load_profile(self.dependencies.profile_path))
                    self._next_configure = now + 30
                except (OSError, ValueError, ResizeError) as error:
                    self._record_error(error)
                    self._next_configure = now + 5
            if self.stop_event.is_set():
                break
            if now >= next_reconcile:
                next_reconcile = now + 5
                self._reconcile()

    def _reconcile(self) -> None:
        try:
            self._schedule_reconcile()
        except Exception as error:
            # Reconciliation errors must not kill the focus watchdog thread.
            if getattr(error, "status_code", None) != 409:
                self.reconciliation = {"error": str(error)}
                self._record_error(error)
                try:
                    self._suspend_activation()
                except Exception as suspend_error:
                    self._record_error(suspend_error)

    def _schedule_reconcile(self) -> None:
        if not self.dependencies.live_activation_enabled:
            return
        self._retry_clear()
        if self._mutation_pending():
            return
        if self.keyd.suspended and self.keyd.profile is not None:
            return
        pending = (self.dependencies.state_dir / "pending.json").is_file()
        if self._desired_profile is None and not pending:
            return
        if pending:
            profile = Profile.model_validate_json(
                (self.dependencies.state_dir / "pending.json").read_text(encoding="utf-8")
            )
        else:
            assert self._desired_profile is not None
            profile = self._desired_profile
        root = locate_launcher_root(
            self.dependencies.managed_dir, profile.launcher_root, _processes()
        )
        if root is None:
            raise ValueError("launcher root unavailable during reconciliation")
        queue = self.app.state.jobs

        def operation() -> Any:
            with self._preset_lock:
                self._desired_profile = profile
                try:
                    if pending or self.keyd.profile != profile:
                        self._before_apply(profile)
                    report = self.dependencies.coordinator_factory(root).reconcile_new_instances(
                        profile, after_apply=self._applied,
                    )
                except Exception as error:
                    self.reconciliation = {"error": str(error)}
                    if self._mutation_pending():
                        self._mutation_failed(error)
                    else:
                        self._suspend_activation()
                    raise
                self.reconciliation = asdict(report)
                if self._mutation_pending():
                    failure_detail = report.keyd_error or "; ".join(
                        item.detail for item in report.instances if item.status == "failed"
                    ) or "reconciliation requires manual recovery"
                    self._mutation_failed(RuntimeError(failure_detail))
                elif report.keyd_error is not None or any(
                    item.status not in ("updated", "unchanged") for item in report.instances
                ):
                    self._suspend_activation()
                return report

        queue.submit(operation)

    async def close(self) -> None:
        with self._action_lock:
            self._accept_presets = False
            self._action_epoch += 1
            self.stop_event.set()
        await self._disconnect_presets()
        if self.thread is not None:
            # Operations have their own subprocess deadlines. A short join timeout
            # could leave restore-capable loop work alive after the lock is released.
            await asyncio.to_thread(self.thread.join)
        try:
            if self.app is not None:
                # The focus producer has stopped. Drain queued mutations before
                # the final restore: their callbacks can still restore sensitivity.
                await asyncio.to_thread(self.app.state.jobs.close)
            with self._preset_lock:
                self.pipe.close(unlink=False)
        finally:
            try:
                with self._preset_lock:
                    self._focus_lost()
                    try:
                        # Fence any orphaned helper even when local state is inactive.
                        self.sensitivity.restore()
                    except SensitivityError as error:
                        self._record_error(error)
            finally:
                # A replacement service must not race this process's last restore.
                self.lock.close()

    async def _effective_game_focus(self, interface: Any) -> bool:
        values = await asyncio.wait_for(interface.call_get_focus(), timeout=2)
        if (
            not isinstance(values, (tuple, list))
            or len(values) != 5
            or not isinstance(values[0], str)
            or not isinstance(values[1], str)
            or type(values[4]) is not bool
        ):
            raise ResizeError("invalid GNOME focus reply")
        return not values[4] and is_minecraft_window(FocusEvent(values[0], values[1]))

    def _current_action(self, interface: Any, epoch: int) -> bool:
        with self._action_lock:
            last_seen = self.pipe.watchdog.last_seen
            return (
                self._accept_presets
                and not self.stop_event.is_set()
                and epoch == self._action_epoch
                and self.extension_connected
                and interface is self._preset_interface
                and self.controller.game_active
                and last_seen is not None
                and time.monotonic() - last_seen < self.pipe.watchdog.timeout
            )

    async def _preset_operation(
        self, interface: Any, epoch: int, operation: Callable[[], T]
    ) -> T | None:
        def guarded() -> T | None:
            with self._preset_lock:
                if not self._current_action(interface, epoch):
                    return None
                return operation()

        # Cancellation cannot stop a spawned subprocess/thread. Drain it before
        # allowing cleanup to rotate generations or a later action to proceed.
        worker = asyncio.create_task(asyncio.to_thread(guarded))
        cancelled = False
        while not worker.done():
            try:
                await asyncio.shield(worker)
            except asyncio.CancelledError:
                cancelled = True
        if cancelled:
            # Retrieve a possible failure before propagating cancellation.
            if not worker.cancelled():
                worker.exception()
            raise asyncio.CancelledError
        return worker.result()

    def _submit_preset_signal(
        self, interface: Any, preset_id: str, shifted: bool
    ) -> asyncio.Task[None]:
        # Reserve the epoch when the signal arrives, before its task is scheduled.
        with self._action_lock:
            self._action_epoch += 1
            epoch = self._action_epoch
        task = asyncio.create_task(
            self._handle_preset_signal(interface, preset_id, shifted, epoch=epoch)
        )
        self._preset_tasks.add(task)
        task.add_done_callback(self._preset_tasks.discard)
        return task

    async def _disconnect_presets(self) -> None:
        with self._action_lock:
            self.extension_connected = False
            self._preset_interface = None
            self._action_epoch += 1
        pending = list(self._preset_tasks)
        for task in pending:
            task.cancel()
        if pending:
            await asyncio.gather(*pending, return_exceptions=True)
        try:
            self._disconnect_focus()
        except Exception as error:
            self._record_error(error)

    async def _handle_preset_signal(
        self, interface: Any, preset_id: str, shifted: bool, *, epoch: int | None = None
    ) -> None:
        with self._action_lock:
            if epoch is None:
                self._action_epoch += 1
                epoch = self._action_epoch
        try:
            async with self._transition_lock:
                if not self._current_action(interface, epoch):
                    return
                if not await self._effective_game_focus(interface):
                    self._focus_lost(epoch=epoch)
                    return
                profile = load_profile(self.dependencies.profile_path)
                result = await self._preset_operation(
                    interface, epoch,
                    lambda: self.preset.resize_preset(profile, preset_id, shifted),
                )
                if result is None:
                    return
                if not self._current_action(interface, epoch):
                    return
                # Exact window sizing must precede a new effective-focus reply.
                if not await self._effective_game_focus(interface):
                    self._focus_lost(epoch=epoch)
                    return
                preset = next(item for item in profile.presets if item.id == preset_id)
                if preset.eye_zoom and not shifted:
                    activated = await self._preset_operation(
                        interface, epoch, lambda: self.preset.activate_eye(result)
                    )
                    if activated is None:
                        return
                    focused = await self._effective_game_focus(interface)
                    if not focused:
                        self._focus_lost(epoch=epoch)
                    elif not self._current_action(interface, epoch):
                        # Restore this completed eye action without invalidating the
                        # newer preset already waiting for the transition lock.
                        self._restore_sensitivity()
        except asyncio.CancelledError:
            raise
        except Exception as error:
            self._record_error(error)
            self._focus_lost(epoch=epoch)

    async def watch_presets(self) -> None:
        """Subscribe to the extension's typed (string, boolean) D-Bus signal."""
        from dbus_next.aio import MessageBus  # type: ignore[attr-defined]
        from dbus_next.constants import MessageType
        from dbus_next.message import Message

        while not self.stop_event.is_set():
            bus: Any = None
            disconnected: asyncio.Task[Any] | None = None
            interface: Any = None
            handler: Callable[[str, bool], None] | None = None
            try:
                bus = await MessageBus().connect()
                disconnected = asyncio.create_task(bus.wait_for_disconnect())
                introspection = await bus.introspect("org.gnome.Shell", "/org/mcsr/Toolkit")
                proxy = bus.get_proxy_object("org.gnome.Shell", "/org/mcsr/Toolkit", introspection)
                interface = proxy.get_interface("org.mcsr.Toolkit")

                def handler_for(source: Any) -> Callable[[str, bool], None]:
                    def activated(preset_id: str, shifted: bool) -> None:
                        if (
                            source is not self._preset_interface
                            or type(preset_id) is not str
                            or type(shifted) is not bool
                        ):
                            return
                        self._submit_preset_signal(source, preset_id, shifted)

                    return activated

                handler = handler_for(interface)
                interface.on_preset_activated(handler)
                # The Shell bus survives extension disable; probe the exported object too.
                while not self.stop_event.is_set():
                    await asyncio.wait_for(self._effective_game_focus(interface), timeout=2)
                    if not self.extension_connected:
                        self._next_configure = 0.0
                    self._preset_interface = interface
                    self.extension_connected = True
                    done, _ = await asyncio.wait(
                        {disconnected}, timeout=self._extension_poll_interval
                    )
                    if done:
                        await disconnected
                        break
            except Exception as error:
                self._record_error(error)
            finally:
                await self._disconnect_presets()
                if bus is not None:
                    try:
                        if interface is not None and handler is not None:
                            interface.off_preset_activated(handler)
                        if bus.connected:
                            # Drain asynchronous AddMatch/RemoveMatch replies before disconnect.
                            reply = await asyncio.wait_for(bus.call(Message(
                                destination="org.freedesktop.DBus",
                                path="/org/freedesktop/DBus",
                                interface="org.freedesktop.DBus.Peer", member="Ping",
                            )), timeout=2)
                            if reply is None or reply.message_type != MessageType.METHOD_RETURN:
                                raise RuntimeError("D-Bus subscription cleanup failed")
                    except Exception as error:
                        self._record_error(error)
                    bus.disconnect()
                if disconnected is not None:
                    # Let dbus-next remove its loop readers before the event loop closes.
                    await asyncio.wait({disconnected}, timeout=2)
                    disconnected.cancel()
                    await asyncio.gather(disconnected, return_exceptions=True)
            if not self.stop_event.is_set():
                await asyncio.sleep(self._extension_retry_interval)


def main() -> None:
    parser = argparse.ArgumentParser(description="MCSR Toolkit local user service")
    parser.add_argument("--port", type=int, default=8765)
    parser.add_argument(
        "--enable-live-keyd",
        action="store_true",
        help="enable keyd activation only after the supervised focus/chat safety proof",
    )
    args = parser.parse_args()
    if args.enable_live_keyd:
        preflight_keyd_clear()
    config = service_config(port=args.port)
    logging.basicConfig(level=logging.INFO)
    data_dir = default_profile_path().parent
    state_dir = data_dir / "state"
    runtime_dir = Path(os.environ.get("XDG_RUNTIME_DIR", f"/run/user/{os.getuid()}"))
    keyd_adapter = PrivilegedKeydAdapter(state_dir / "keyd-adapter.json")
    dependencies = Dependencies(
        profile_path=default_profile_path(),
        managed_dir=data_dir,
        state_dir=state_dir,
        coordinator_factory=lambda root: ApplyCoordinator(
            root, state_dir, keyd_adapter, _processes
        ),
        live_activation_enabled=args.enable_live_keyd,
        port=config.port,
        static_dir=Path(__file__).resolve().parents[2] / "static",
        processes=_processes,
    )
    runtime = ServiceRuntime(dependencies, runtime_dir=runtime_dir)

    async def serve() -> None:
        watcher: asyncio.Task[None] | None = None
        cleanup_started = False

        async def cleanup() -> None:
            nonlocal cleanup_started
            if cleanup_started:
                return
            cleanup_started = True
            # Stop acceptance before settling the watcher's restore-capable cleanup.
            # close() must perform the last restore before releasing the service lock.
            runtime.stop_event.set()
            try:
                if watcher is not None:
                    watcher.cancel()
                    await watcher
            except asyncio.CancelledError:
                pass
            finally:
                await runtime.close()

        @asynccontextmanager
        async def lifespan(app: FastAPI) -> AsyncIterator[None]:
            try:
                yield
            finally:
                # Uvicorn replays SIGTERM before serve() returns. Restore while
                # its signal handlers still protect application shutdown.
                await cleanup()

        app = create_app(dependencies, lifespan=lifespan)
        runtime.app = app
        runtime.start()
        watcher = asyncio.create_task(runtime.watch_presets())
        try:
            await uvicorn.Server(
                uvicorn.Config(app, host=config.host, port=config.port, proxy_headers=False)
            ).serve()
        finally:
            # Also cover failures before Uvicorn enters the application lifespan.
            await cleanup()

    asyncio.run(serve())


if __name__ == "__main__":
    main()
