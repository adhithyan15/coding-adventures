"""Build and replay native fixture adapters using Windows package toolchains."""

from __future__ import annotations

import os
import shutil
import subprocess
import sys
import tempfile
from pathlib import Path


FIXTURE_ROOT = Path(__file__).resolve().parent
PACKAGE_ROOT = Path.cwd()


def run(*args: str | Path) -> None:
    subprocess.run([str(arg) for arg in args], cwd=PACKAGE_ROOT, check=True)


def replay(*args: str | Path) -> None:
    run(sys.executable, FIXTURE_ROOT / "replay_cli.py", *args)


def build_c(temp_dir: Path) -> None:
    common = (
        "cl", "/nologo", "/std:c11", "/W4", "/WX",
        "/D_CRT_SECURE_NO_WARNINGS", "/Iinclude",
    )
    sources = ("src/filesystem.c", "src/descriptors.c", "src/kernel.c")
    object_dir = f"/Fo{temp_dir}{os.sep}"
    model = temp_dir / "model-test.exe"
    adapter = temp_dir / "fixture-cli.exe"
    run(*common, *sources, "tests/model_test.c", object_dir, f"/Fe{model}")
    run(model)
    run(
        *common,
        *sources,
        FIXTURE_ROOT / "test-adapters/c/fixture_cli.c",
        object_dir,
        f"/Fe{adapter}",
    )
    replay(adapter)


def build_cpp(temp_dir: Path) -> None:
    common = ("cl", "/nologo", "/std:c++17", "/EHsc", "/W4", "/WX", "/Iinclude")
    model = temp_dir / "model-test.exe"
    adapter = temp_dir / "fixture-cli.exe"
    object_dir = f"/Fo{temp_dir}{os.sep}"
    run(*common, "src/unix_history.cpp", "tests/model_test.cpp", object_dir, f"/Fe{model}")
    run(model)
    run(
        *common,
        "src/unix_history.cpp",
        FIXTURE_ROOT / "test-adapters/cpp/fixture_cli.cpp",
        object_dir,
        f"/Fe{adapter}",
    )
    replay(adapter)


def build_java(temp_dir: Path) -> None:
    classes = temp_dir / "classes"
    classes.mkdir()
    run(
        "javac", "-Xlint:all", "-Werror", "-d", classes,
        "src/codingadventures/unixhistory/UnixHistory.java",
        "test/codingadventures/unixhistory/ModelTest.java",
        FIXTURE_ROOT / "test-adapters/java/FixtureCli.java",
    )
    run("java", "-cp", classes, "codingadventures.unixhistory.ModelTest")
    replay("java", "-cp", classes, "codingadventures.unixhistory.FixtureCli")


def build_lua(temp_dir: Path) -> None:
    run("lua", "-e", "assert(_VERSION >= 'Lua 5.3', 'Lua 5.3 or newer required')")
    run("lua", "test.lua")
    adapter = temp_dir / "fixture_cli.lua"
    shutil.copyfile(FIXTURE_ROOT / "test-adapters/lua/fixture_cli.lua", adapter)
    replay("lua", adapter)


BUILDERS = {"c": build_c, "cpp": build_cpp, "java": build_java, "lua": build_lua}


def main() -> int:
    if len(sys.argv) != 2 or sys.argv[1] not in BUILDERS:
        print("usage: build_windows.py c|cpp|java|lua", file=sys.stderr)
        return 2
    with tempfile.TemporaryDirectory(prefix=f"hos-{sys.argv[1]}-unix-history-") as temp:
        BUILDERS[sys.argv[1]](Path(temp))
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
