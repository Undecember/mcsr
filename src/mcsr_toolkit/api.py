"""Loopback API for the unprivileged toolkit user service."""

from __future__ import annotations

import secrets
import threading
import uuid
from collections.abc import Callable
from concurrent.futures import ThreadPoolExecutor
from contextlib import AbstractAsyncContextManager, AbstractContextManager, nullcontext
from dataclasses import asdict, dataclass, field
from pathlib import Path
from typing import Any, Literal
from urllib.parse import urlsplit

from fastapi import FastAPI, HTTPException, Request
from fastapi.responses import JSONResponse
from fastapi.staticfiles import StaticFiles
from pydantic import BaseModel, ConfigDict

from mcsr_toolkit.apply import ApplyCoordinator
from mcsr_toolkit.downloads import DownloadResult, Product, download_release, resolve_release
from mcsr_toolkit.instances import ProcessInfo, discover_instances, locate_launcher_root
from mcsr_toolkit.models import Profile
from mcsr_toolkit.profile import load_profile, save_profile


@dataclass(frozen=True)
class ServiceConfig:
    host: str
    port: int


def service_config(*, port: int = 8765) -> ServiceConfig:
    if not 1 <= port <= 65535:
        raise ValueError("invalid local service port")
    return ServiceConfig("127.0.0.1", port)


def _download(product: Product, destination: Path) -> dict[str, Any]:
    asset = resolve_release(product)
    result: DownloadResult = download_release(asset, destination)
    return asdict(result)


@dataclass
class Dependencies:
    profile_path: Path
    managed_dir: Path
    state_dir: Path
    coordinator_factory: Callable[[Path], ApplyCoordinator]
    download: Callable[[Product, Path], dict[str, Any]] = _download
    session_token: str = field(default_factory=lambda: secrets.token_urlsafe(32))
    live_activation_enabled: bool = False
    port: int = 8765
    static_dir: Path | None = None
    focus_status: Callable[[], dict[str, Any]] = lambda: {"mode": "observe-only"}
    on_applied: Callable[[Profile], None] = lambda profile: None
    before_apply: Callable[[Profile], None] = lambda profile: None
    before_restore: Callable[[], None] = lambda: None
    after_restore: Callable[[Profile | None], None] = lambda profile: None
    mutation_failed: Callable[[Exception], None] = lambda error: None
    mutation_guard: Callable[[], AbstractContextManager[object]] = nullcontext
    processes: Callable[[], list[ProcessInfo]] = lambda: []


class RestoreRequest(BaseModel):
    model_config = ConfigDict(extra="forbid")
    backup_id: str


class JobQueue:
    def __init__(self) -> None:
        self._executor = ThreadPoolExecutor(max_workers=1, thread_name_prefix="mcsr-job")
        self._lock = threading.Lock()
        self._active: str | None = None
        self._jobs: dict[str, dict[str, Any]] = {}

    def submit(self, operation: Callable[[], Any]) -> dict[str, Any]:
        with self._lock:
            if self._active is not None:
                raise HTTPException(
                    409, detail={"code": "job_busy", "message": "another operation is running"}
                )
            job_id = uuid.uuid4().hex
            job: dict[str, Any] = {"id": job_id, "status": "running", "result": None, "error": None}
            self._jobs[job_id] = job
            self._active = job_id
            if len(self._jobs) > 128:
                oldest = next(iter(self._jobs))
                if oldest != job_id:
                    del self._jobs[oldest]

        def run() -> None:
            try:
                result = operation()
                if hasattr(result, "instances"):
                    result = asdict(result)
                with self._lock:
                    job["status"] = "succeeded"
                    job["result"] = result
            except Exception as error:
                with self._lock:
                    job["status"] = "failed"
                    job["error"] = str(error)
            finally:
                with self._lock:
                    self._active = None

        self._executor.submit(run)
        return {"id": job_id, "status": "running"}

    def require_idle(self) -> None:
        with self._lock:
            if self._active is not None:
                raise HTTPException(
                    409, detail={"code": "job_busy", "message": "another operation is running"}
                )

    def get(self, job_id: str) -> dict[str, Any]:
        with self._lock:
            if job_id not in self._jobs:
                raise HTTPException(404, detail={"code": "unknown_job"})
            return dict(self._jobs[job_id])

    def close(self) -> None:
        self._executor.shutdown(wait=True, cancel_futures=False)


def _valid_host(host: str, port: int) -> bool:
    return host in (f"127.0.0.1:{port}", f"localhost:{port}")


def _valid_origin(origin: str, host: str, port: int) -> bool:
    try:
        parsed = urlsplit(origin)
        return (
            parsed.scheme == "http"
            and parsed.hostname in ("127.0.0.1", "localhost")
            and parsed.port == port
            and parsed.netloc == host
            and not parsed.username
            and not parsed.password
            and not parsed.path
            and not parsed.query
            and not parsed.fragment
        )
    except ValueError:
        return False


def _launcher_root(dependencies: Dependencies, profile: Profile) -> Path:
    try:
        root = locate_launcher_root(
            dependencies.managed_dir, profile.launcher_root, dependencies.processes()
        )
    except (OSError, ValueError, RuntimeError):
        root = None
    if root is None:
        raise HTTPException(
            422, detail={"code": "launcher_root", "message": "MCSRLauncher root not found"}
        )
    if (root / "instances").is_symlink() or not discover_instances(root):
        raise HTTPException(
            422, detail={"code": "launcher_root", "message": "no safe MCSRLauncher instances"}
        )
    return root


def create_app(
    dependencies: Dependencies,
    *,
    lifespan: Callable[[FastAPI], AbstractAsyncContextManager[None]] | None = None,
) -> FastAPI:
    app = FastAPI(
        title="MCSR Toolkit", docs_url=None, redoc_url=None, openapi_url=None, lifespan=lifespan,
    )
    app.state.dependencies = dependencies
    queue = JobQueue()
    app.state.jobs = queue
    profile_lock = threading.Lock()

    @app.middleware("http")
    async def local_session(request: Request, call_next: Any) -> Any:
        host = request.headers.get("host", "")
        if not _valid_host(host, dependencies.port):
            return JSONResponse({"detail": {"code": "bad_host"}}, status_code=403)
        origin = request.headers.get("origin", "")
        if origin and not _valid_origin(origin, host, dependencies.port):
            return JSONResponse({"detail": {"code": "bad_origin"}}, status_code=403)
        if request.method not in ("GET", "HEAD", "OPTIONS"):
            token = request.headers.get("x-mcsr-session", "")
            if not _valid_origin(origin, host, dependencies.port) or not secrets.compare_digest(
                token, dependencies.session_token
            ):
                return JSONResponse({"detail": {"code": "bad_session"}}, status_code=403)
        return await call_next(request)

    @app.get("/api/session")
    def session() -> dict[str, str]:
        return {"token": dependencies.session_token}

    @app.get("/api/status")
    def status() -> dict[str, Any]:
        profile = load_profile(dependencies.profile_path)
        root = locate_launcher_root(
            dependencies.managed_dir, profile.launcher_root, dependencies.processes()
        )
        instances = []
        if root is not None:
            instances = [asdict(item) for item in discover_instances(root)]
        return {
            "live_activation_enabled": dependencies.live_activation_enabled,
            "launcher_root": str(root) if root else None,
            "instances": instances,
            "focus": dependencies.focus_status(),
        }

    @app.get("/api/profile")
    def get_profile() -> dict[str, Any]:
        return load_profile(dependencies.profile_path).model_dump(mode="json")

    @app.put("/api/profile")
    def put_profile(profile: Profile) -> dict[str, Any]:
        if profile.launcher_root is not None:
            _launcher_root(dependencies, profile)
        with profile_lock:
            queue.require_idle()
            save_profile(dependencies.profile_path, profile)
        return profile.model_dump(mode="json")

    @app.post("/api/preview")
    def preview(profile: Profile) -> dict[str, Any]:
        root = _launcher_root(dependencies, profile)
        return asdict(dependencies.coordinator_factory(root).preview(profile))

    @app.post("/api/apply", status_code=202)
    def apply(expected_profile: Profile) -> dict[str, Any]:
        if not dependencies.live_activation_enabled:
            raise HTTPException(
                403, detail={"code": "safety_gate", "message": "live keyd activation is disabled"}
            )
        with profile_lock:
            profile = load_profile(dependencies.profile_path)
            if profile != expected_profile:
                raise HTTPException(
                    409,
                    detail={
                        "code": "stale_profile",
                        "message": "profile changed; save and preview again",
                    },
                )
            root = _launcher_root(dependencies, profile)

            def operation() -> Any:
                with dependencies.mutation_guard():
                    try:
                        dependencies.before_apply(profile)
                        report = dependencies.coordinator_factory(root).apply(
                            profile, after_apply=dependencies.on_applied,
                        )
                        marker = dependencies.state_dir / "mutation.json"
                        if marker.exists() or marker.is_symlink():
                            error = report.keyd_error or "; ".join(
                                item.detail for item in report.instances if item.status == "failed"
                            ) or "apply requires manual recovery"
                            dependencies.mutation_failed(RuntimeError(error))
                        return report
                    except Exception as error:
                        marker = dependencies.state_dir / "mutation.json"
                        if marker.exists() or marker.is_symlink():
                            dependencies.mutation_failed(error)
                        raise

            return queue.submit(operation)

    @app.post("/api/restore", status_code=202)
    def restore(request: RestoreRequest) -> dict[str, Any]:
        profile = load_profile(dependencies.profile_path)
        root = _launcher_root(dependencies, profile)

        def operation() -> Any:
            def publish(restored_profile: Profile | None) -> None:
                if restored_profile is not None:
                    save_profile(dependencies.profile_path, restored_profile)
                dependencies.after_restore(restored_profile)

            with dependencies.mutation_guard():
                try:
                    return dependencies.coordinator_factory(root).restore(
                        request.backup_id, before_restore=dependencies.before_restore,
                        after_restore=publish,
                    )
                except Exception as error:
                    marker = dependencies.state_dir / "mutation.json"
                    if marker.exists() or marker.is_symlink():
                        dependencies.mutation_failed(error)
                    raise

        return queue.submit(operation)

    @app.post("/api/download/{product}", status_code=202)
    def download(product: Literal["launcher", "ninjabrain"]) -> dict[str, Any]:
        name = "MCSRLauncher.jar" if product == "launcher" else "Ninjabrain-Bot.jar"
        destination = dependencies.managed_dir / "downloads" / name
        return queue.submit(lambda: dependencies.download(product, destination))

    @app.get("/api/jobs/{job_id}")
    def get_job(job_id: str) -> dict[str, Any]:
        return queue.get(job_id)

    if dependencies.static_dir is not None and dependencies.static_dir.is_dir():
        app.mount("/", StaticFiles(directory=dependencies.static_dir, html=True), name="static")
    return app
