"""Resolve and safely save official launcher and Ninjabrain Bot releases."""

from __future__ import annotations

import os
import re
import tempfile
import zipfile
from contextlib import AbstractContextManager, nullcontext
from dataclasses import dataclass
from pathlib import Path
from typing import Literal
from urllib.parse import urljoin, urlparse

import httpx

Product = Literal["launcher", "ninjabrain"]

REPOSITORIES: dict[Product, str] = {
    "launcher": "MCSRLauncher/Launcher",
    "ninjabrain": "Ninjabrain1/Ninjabrain-Bot",
}
SOURCE_URLS: dict[Product, str] = {
    "launcher": "https://mcsrlauncher.github.io/download",
    "ninjabrain": "https://github.com/Ninjabrain1/Ninjabrain-Bot/releases",
}
REDIRECT_HOSTS = frozenset({"github.com", "release-assets.githubusercontent.com"})
MAX_BYTES = 256 * 1024 * 1024


@dataclass(frozen=True)
class ReleaseAsset:
    product: Product
    version: str
    source_url: str
    asset_url: str
    filename: str


@dataclass(frozen=True)
class DownloadResult:
    version: str
    path: Path
    byte_count: int


def _client_context(client: httpx.Client | None) -> AbstractContextManager[httpx.Client]:
    if client is None:
        return httpx.Client(timeout=httpx.Timeout(20.0), headers={"User-Agent": "mcsr-toolkit"})
    return nullcontext(client)


def _https_host(url: str, allowed: frozenset[str]) -> bool:
    parsed = urlparse(url)
    return (
        parsed.scheme == "https"
        and parsed.hostname in allowed
        and parsed.username is None
        and parsed.password is None
        and parsed.port in (None, 443)
    )


def _asset_name_matches(product: Product, name: str) -> bool:
    if product == "launcher":
        return name == "MCSRLauncher.jar"
    return re.fullmatch(r"Ninjabrain-Bot-[A-Za-z0-9._-]+\.jar", name) is not None


def resolve_release(product: Product, *, client: httpx.Client | None = None) -> ReleaseAsset:
    repository = REPOSITORIES[product]
    api_url = f"https://api.github.com/repos/{repository}/releases/latest"
    try:
        with _client_context(client) as active:
            response = active.get(api_url)
            response.raise_for_status()
            data = response.json()
    except (httpx.HTTPError, ValueError) as exc:
        raise ValueError("release lookup failed") from exc
    if not isinstance(data, dict) or not isinstance(data.get("tag_name"), str):
        raise ValueError("invalid release metadata")
    if not isinstance(data.get("assets"), list):
        raise ValueError("invalid release metadata")
    prefix = f"https://github.com/{repository}/releases/download/"
    for asset in data["assets"]:
        if not isinstance(asset, dict):
            continue
        name, url = asset.get("name"), asset.get("browser_download_url")
        if not isinstance(name, str) or not _asset_name_matches(product, name):
            continue
        if (
            not isinstance(url, str)
            or not url.startswith(prefix)
            or not _https_host(url, frozenset({"github.com"}))
        ):
            raise ValueError("invalid release asset URL")
        if Path(urlparse(url).path).name != name:
            raise ValueError("invalid release asset URL")
        return ReleaseAsset(product, data["tag_name"], SOURCE_URLS[product], url, name)
    raise ValueError("invalid release metadata: expected JAR asset missing")


def _validate_jar(path: Path) -> None:
    if not zipfile.is_zipfile(path):
        raise ValueError("download is not a valid JAR")
    try:
        with zipfile.ZipFile(path) as archive:
            if "META-INF/MANIFEST.MF" not in archive.namelist() or archive.testzip() is not None:
                raise ValueError("download is not a valid JAR")
    except (OSError, zipfile.BadZipFile) as exc:
        raise ValueError("download is not a valid JAR") from exc


def download_release(
    asset: ReleaseAsset, destination: Path, *, client: httpx.Client | None = None
) -> DownloadResult:
    repository = REPOSITORIES[asset.product]
    prefix = f"https://github.com/{repository}/releases/download/"
    if not asset.asset_url.startswith(prefix) or not _https_host(
        asset.asset_url, frozenset({"github.com"})
    ):
        raise ValueError("invalid release asset URL")
    if destination.suffix.lower() != ".jar" or Path(asset.filename).name != asset.filename:
        raise ValueError("invalid JAR destination")

    destination.parent.mkdir(parents=True, exist_ok=True)
    temporary_path: Path | None = None
    current_url = asset.asset_url
    try:
        with _client_context(client) as active:
            for _ in range(6):
                with active.stream("GET", current_url, follow_redirects=False) as response:
                    if response.status_code in (301, 302, 303, 307, 308):
                        location = response.headers.get("location")
                        if not location:
                            raise ValueError("invalid release redirect")
                        current_url = urljoin(current_url, location)
                        if not _https_host(current_url, REDIRECT_HOSTS):
                            raise ValueError("unapproved release redirect")
                        continue
                    response.raise_for_status()
                    count = 0
                    with tempfile.NamedTemporaryFile(
                        mode="wb",
                        dir=destination.parent,
                        prefix=f".{destination.name}.",
                        delete=False,
                    ) as stream:
                        temporary_path = Path(stream.name)
                        for chunk in response.iter_bytes():
                            count += len(chunk)
                            if count > MAX_BYTES:
                                raise ValueError("download exceeds size limit")
                            stream.write(chunk)
                        stream.flush()
                        os.fsync(stream.fileno())
                    _validate_jar(temporary_path)
                    os.replace(temporary_path, destination)
                    return DownloadResult(asset.version, destination, count)
            raise ValueError("too many release redirects")
    except httpx.HTTPError as exc:
        raise ValueError("download failed") from exc
    finally:
        if temporary_path is not None:
            temporary_path.unlink(missing_ok=True)
