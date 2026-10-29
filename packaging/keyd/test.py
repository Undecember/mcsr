#!/usr/bin/env python3
"""Verify pinned sources and run the managed keyd engine suite without devices/root."""
import hashlib
import os
from pathlib import Path
import shutil
import subprocess
import tempfile

ROOT = Path(__file__).resolve().parents[2]
VENDOR = ROOT / "vendor/keyd"
TESTS = ROOT / "packaging/keyd/tests"
ENGINE_SOURCES = ["keyboard", "string", "macro", "config", "log", "ini", "keys", "unicode"]


def run(*args, cwd=None):
    result = subprocess.run(list(map(str, args)), cwd=cwd, capture_output=True, text=True)
    if result.returncode:
        raise RuntimeError(f"{' '.join(map(str, args))}\n{result.stdout}\n{result.stderr}")
    return result.stdout


def main():
    for line in (VENDOR / "SHA256SUMS").read_text().splitlines():
        digest, relative = line.split("  ", 1)
        actual = hashlib.sha256((VENDOR / relative).read_bytes()).hexdigest()
        assert actual == digest, f"vendored source changed: {relative}"
    with tempfile.TemporaryDirectory(prefix="mcsr-keyd-test-") as directory:
        stage = Path(directory)
        shutil.copytree(VENDOR / "src", stage / "src")
        shutil.copytree(VENDOR / "t", stage / "t")
        run("patch", "--batch", "--forward", "-p1", "-i", ROOT / "packaging/keyd/mcsr.patch", cwd=stage)
        compiler = os.environ.get("CC", "cc")
        sources = [stage / f"src/{name}.c" for name in ENGINE_SOURCES]
        harness = stage / "test-io"
        run(compiler, '-DDATA_DIR=""', stage / "t/test-io.c", *sources, "-o", harness)
        upstream = sorted((stage / "t").glob("*.t"))
        run(harness, stage / "t/test.conf", *upstream)
        cases = sorted(TESTS.glob("*.t"))
        for case in cases:
            config = TESTS / ("matchmods.conf" if case.stem.startswith("matchmods-") else "keyd.conf")
            run(harness, config, case)
        reset = stage / "reset-test"
        run(compiler, '-DDATA_DIR=""', "-iquote", stage / "src", TESTS / "reset.c", *sources, "-o", reset)
        run(reset, TESTS / "keyd.conf")
        ipc = stage / "ipc-reset-test"
        run(compiler, '-DDATA_DIR=""', '-DCONFIG_DIR=""', '-DSOCKET_PATH=""',
            '-DVERSION="test"', "-ffunction-sections", "-fdata-sections",
            "-iquote", stage / "src", TESTS / "ipc-reset.c", *sources,
            stage / "src/util.c", "-Wl,--gc-sections", "-o", ipc)
        run(ipc, TESTS / "keyd.conf")
        print(f"PASS: {len(upstream)} upstream event cases, {len(cases)} toolkit event cases, held-key reset cleanup, IPC bind reset")


if __name__ == "__main__":
    main()
