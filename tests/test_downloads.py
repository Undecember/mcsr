from io import BytesIO
from pathlib import Path
from zipfile import ZipFile

import httpx
import pytest

from mcsr_toolkit.downloads import ReleaseAsset, download_release, resolve_release


def jar_bytes() -> bytes:
    buffer = BytesIO()
    with ZipFile(buffer, "w") as archive:
        archive.writestr("META-INF/MANIFEST.MF", "Manifest-Version: 1.0\nMain-Class: Main\n")
    return buffer.getvalue()


def release_response(repository: str) -> dict[str, object]:
    if repository == "MCSRLauncher/Launcher":
        tag, name = "0.8.9-beta", "MCSRLauncher.jar"
    else:
        tag, name = "1.5.2", "Ninjabrain-Bot-1.5.2.jar"
    return {
        "tag_name": tag,
        "assets": [
            {
                "name": name,
                "browser_download_url": f"https://github.com/{repository}/releases/download/{tag}/{name}",
            }
        ],
    }


@pytest.mark.parametrize(
    ("product", "repository", "filename"),
    [
        ("launcher", "MCSRLauncher/Launcher", "MCSRLauncher.jar"),
        ("ninjabrain", "Ninjabrain1/Ninjabrain-Bot", "Ninjabrain-Bot-1.5.2.jar"),
    ],
)
def test_resolves_only_expected_official_jar(product: str, repository: str, filename: str) -> None:
    def handler(request: httpx.Request) -> httpx.Response:
        assert request.url.host == "api.github.com"
        return httpx.Response(200, json=release_response(repository))

    with httpx.Client(transport=httpx.MockTransport(handler)) as client:
        result = resolve_release(product, client=client)

    assert result.filename == filename
    assert result.asset_url.startswith(f"https://github.com/{repository}/releases/download/")
    assert result.version in {"0.8.9-beta", "1.5.2"}


def test_release_lookup_reports_offline_failure() -> None:
    def handler(request: httpx.Request) -> httpx.Response:
        raise httpx.ConnectError("offline", request=request)

    with httpx.Client(transport=httpx.MockTransport(handler)) as client:
        with pytest.raises(ValueError, match="release lookup failed"):
            resolve_release("launcher", client=client)


def test_malformed_release_metadata_is_rejected() -> None:
    with httpx.Client(
        transport=httpx.MockTransport(lambda _: httpx.Response(200, json={"tag_name": "x"}))
    ) as client:
        with pytest.raises(ValueError, match="release metadata"):
            resolve_release("launcher", client=client)


def test_release_asset_from_other_host_is_rejected() -> None:
    payload = release_response("MCSRLauncher/Launcher")
    payload["assets"] = [
        {"name": "MCSRLauncher.jar", "browser_download_url": "https://evil.example/file.jar"}
    ]
    with httpx.Client(
        transport=httpx.MockTransport(lambda _: httpx.Response(200, json=payload))
    ) as client:
        with pytest.raises(ValueError, match="asset URL"):
            resolve_release("launcher", client=client)


def launcher_asset() -> ReleaseAsset:
    return ReleaseAsset(
        product="launcher",
        version="0.8.9-beta",
        source_url="https://mcsrlauncher.github.io/download",
        asset_url="https://github.com/MCSRLauncher/Launcher/releases/download/0.8.9-beta/MCSRLauncher.jar",
        filename="MCSRLauncher.jar",
    )


def test_download_accepts_official_release_cdn_redirect(tmp_path: Path) -> None:
    payload = jar_bytes()

    def handler(request: httpx.Request) -> httpx.Response:
        if request.url.host == "github.com":
            return httpx.Response(
                302, headers={"location": "https://release-assets.githubusercontent.com/asset"}
            )
        assert request.url.host == "release-assets.githubusercontent.com"
        return httpx.Response(200, content=payload)

    path = tmp_path / "MCSRLauncher.jar"
    with httpx.Client(transport=httpx.MockTransport(handler)) as client:
        result = download_release(launcher_asset(), path, client=client)

    assert path.read_bytes() == payload
    assert result.path == path
    assert result.byte_count == len(payload)


def test_unapproved_redirect_keeps_previous_jar(tmp_path: Path) -> None:
    path = tmp_path / "MCSRLauncher.jar"
    path.write_bytes(b"previous")
    response = httpx.Response(302, headers={"location": "https://evil.example/file.jar"})
    with httpx.Client(transport=httpx.MockTransport(lambda _: response)) as client:
        with pytest.raises(ValueError, match="redirect"):
            download_release(launcher_asset(), path, client=client)
    assert path.read_bytes() == b"previous"


def test_non_jar_response_keeps_previous_jar(tmp_path: Path) -> None:
    path = tmp_path / "MCSRLauncher.jar"
    path.write_bytes(b"previous")
    with httpx.Client(
        transport=httpx.MockTransport(lambda _: httpx.Response(200, content=b"<html>oops</html>"))
    ) as client:
        with pytest.raises(ValueError, match="valid JAR"):
            download_release(launcher_asset(), path, client=client)
    assert path.read_bytes() == b"previous"


def test_interrupted_download_keeps_previous_jar(tmp_path: Path) -> None:
    class BrokenStream(httpx.SyncByteStream):
        def __iter__(self):
            yield jar_bytes()[:20]
            raise httpx.ReadError("connection lost")

    path = tmp_path / "MCSRLauncher.jar"
    path.write_bytes(b"previous")
    with httpx.Client(
        transport=httpx.MockTransport(lambda _: httpx.Response(200, stream=BrokenStream()))
    ) as client:
        with pytest.raises(ValueError, match="download failed"):
            download_release(launcher_asset(), path, client=client)
    assert path.read_bytes() == b"previous"
