#!/usr/bin/env python3
"""Install the pinned CI Lua toolchain from verified source mirrors.

The official lua.org archive remains the primary source. Debian and Ubuntu
publish byte-identical copies of that archive, so either can keep CI moving
during a temporary lua.org outage without changing the source we compile.
Every download must match the pinned SHA-256 before extraction.
"""

from __future__ import annotations

import argparse
import hashlib
import os
import shutil
import subprocess
import sys
import tarfile
import tempfile
import urllib.request
from collections.abc import Callable, Iterable
from pathlib import Path
from typing import BinaryIO

LUA_VERSION = "5.4.7"
LUA_SHA256 = "9fbf5e28ef86c69858f6d3d34eccc32e911c1a28b4120ff3e84aaa70cfbf1e30"
LUA_SOURCE_URLS = (
    f"https://lua.org/ftp/lua-{LUA_VERSION}.tar.gz",
    f"https://deb.debian.org/debian/pool/main/l/lua5.4/lua5.4_{LUA_VERSION}.orig.tar.gz",
    f"https://archive.ubuntu.com/ubuntu/pool/main/l/lua5.4/lua5.4_{LUA_VERSION}.orig.tar.gz",
)
DOWNLOAD_TIMEOUT_SECONDS = 20

OpenUrl = Callable[..., BinaryIO]


def sha256_file(path: Path) -> str:
    digest = hashlib.sha256()
    with path.open("rb") as stream:
        for chunk in iter(lambda: stream.read(1024 * 1024), b""):
            digest.update(chunk)
    return digest.hexdigest()


def download_verified_archive(
    destination: Path,
    *,
    urls: Iterable[str] = LUA_SOURCE_URLS,
    expected_sha256: str = LUA_SHA256,
    opener: OpenUrl = urllib.request.urlopen,
) -> str:
    """Download the first byte-identical archive available and return its URL."""

    failures: list[str] = []
    request_headers = {"User-Agent": "coding-adventures-ci/1.0"}
    for url in urls:
        destination.unlink(missing_ok=True)
        try:
            print(f"Downloading Lua {LUA_VERSION} from {url}")
            request = urllib.request.Request(url, headers=request_headers)
            with (
                opener(request, timeout=DOWNLOAD_TIMEOUT_SECONDS) as response,
                destination.open("wb") as output,
            ):
                shutil.copyfileobj(response, output)

            actual_sha256 = sha256_file(destination)
            if actual_sha256 != expected_sha256:
                raise ValueError(
                    f"SHA-256 mismatch: expected {expected_sha256}, got {actual_sha256}"
                )
            print(f"Verified Lua source SHA-256: {actual_sha256}")
            return url
        except (OSError, ValueError) as error:
            failures.append(f"{url}: {error}")
            print(f"Lua source unavailable from {url}: {error}", file=sys.stderr)

    destination.unlink(missing_ok=True)
    details = "\n".join(f"  - {failure}" for failure in failures)
    raise RuntimeError(f"No verified Lua source mirror was available:\n{details}")


def extract_verified_archive(archive: Path, destination: Path) -> Path:
    """Extract a trusted archive while rejecting path traversal and links."""

    destination.mkdir(parents=True, exist_ok=True)
    destination_root = destination.resolve()
    with tarfile.open(archive, "r:gz") as source:
        members = source.getmembers()
        for member in members:
            target = (destination / member.name).resolve()
            if not target.is_relative_to(destination_root):
                raise ValueError(f"Unsafe path in Lua archive: {member.name}")
            if member.issym() or member.islnk():
                raise ValueError(f"Unexpected link in Lua archive: {member.name}")
        if sys.version_info >= (3, 12):
            source.extractall(destination, members=members, filter="data")
        else:
            source.extractall(destination, members=members)

    source_root = destination / f"lua-{LUA_VERSION}"
    if not (source_root / "src" / "lua.c").is_file():
        raise ValueError(
            f"Lua source archive is missing {source_root / 'src' / 'lua.c'}"
        )
    return source_root


def run(command: list[str], *, cwd: Path) -> None:
    print(f"+ {' '.join(command)}")
    subprocess.run(command, cwd=cwd, check=True)


def require_tool(name: str) -> Path:
    executable = shutil.which(name)
    if executable is None:
        raise RuntimeError(f"Required build tool is not on PATH: {name}")
    return Path(executable).resolve()


def install_unix(source_root: Path, prefix: Path) -> None:
    require_tool("make")
    jobs = str(max(1, os.cpu_count() or 1))
    target = "macosx" if sys.platform == "darwin" else "linux"
    run(["make", f"-j{jobs}", target], cwd=source_root)
    run(["make", f"-j{jobs}", f"INSTALL_TOP={prefix}", "install"], cwd=source_root)


def windows_source_groups(source_root: Path) -> dict[str, list[Path]]:
    groups: dict[str, list[Path]] = {"lib": [], "lua": [], "luac": []}
    for source in sorted((source_root / "src").glob("*.c")):
        if source.name == "lua.c":
            groups["lua"].append(source)
        elif source.name in {"luac.c", "print.c"}:
            groups["luac"].append(source)
        else:
            groups["lib"].append(source)
    return groups


def windows_msvc_tools() -> tuple[Path, Path]:
    """Resolve MSVC's linker next to cl.exe, never Git's Unix `link` utility."""

    compiler = require_tool("cl")
    linker = compiler.with_name("link.exe")
    if not linker.is_file():
        raise RuntimeError(f"MSVC link.exe was not found beside cl.exe: {linker}")
    return compiler, linker


def install_windows(source_root: Path, prefix: Path) -> None:
    compiler, linker = windows_msvc_tools()
    groups = windows_source_groups(source_root)
    objects: dict[str, list[str]] = {"lib": [], "lua": [], "luac": []}

    for group, sources in groups.items():
        for source in sources:
            relative_source = source.relative_to(source_root)
            command = [
                str(compiler),
                "/nologo",
                "/MD",
                "/O2",
                "/W3",
                "/c",
                "/D_CRT_SECURE_NO_DEPRECATE",
            ]
            if group == "lib":
                command.append("/DLUA_BUILD_AS_DLL")
            command.append(str(relative_source))
            run(command, cwd=source_root)
            objects[group].append(f"{source.stem}.obj")

    dll_name = "lua54.dll"
    lib_name = "lua54.lib"
    run(
        [str(linker), "/nologo", "/DLL", f"/out:{dll_name}", *objects["lib"]],
        cwd=source_root,
    )
    run(
        [
            str(linker),
            "/nologo",
            "/out:luac.exe",
            *objects["luac"],
            *objects["lib"],
        ],
        cwd=source_root,
    )
    run(
        [str(linker), "/nologo", "/out:lua.exe", *objects["lua"], lib_name],
        cwd=source_root,
    )

    bin_dir = prefix / "bin"
    lib_dir = prefix / "lib"
    include_dir = prefix / "include"
    for directory in (bin_dir, lib_dir, include_dir):
        directory.mkdir(parents=True, exist_ok=True)

    for name in ("lua.exe", "luac.exe", dll_name):
        shutil.copy2(source_root / name, bin_dir / name)
    for name in (dll_name, lib_name):
        shutil.copy2(source_root / name, lib_dir / name)
    for name in ("lua.h", "luaconf.h", "lualib.h", "lauxlib.h"):
        shutil.copy2(source_root / "src" / name, include_dir / name)
    lua_hpp = source_root / "src" / "lua.hpp"
    if not lua_hpp.is_file():
        lua_hpp = source_root / "etc" / "lua.hpp"
    shutil.copy2(lua_hpp, include_dir / "lua.hpp")


def verify_installation(prefix: Path) -> None:
    executable = prefix / "bin" / ("lua.exe" if os.name == "nt" else "lua")
    if not executable.is_file():
        raise RuntimeError(f"Lua installer did not create {executable}")
    result = subprocess.run(
        [str(executable), "-v"],
        check=True,
        capture_output=True,
        text=True,
    )
    version_output = f"{result.stdout}\n{result.stderr}".strip()
    if f"Lua {LUA_VERSION}" not in version_output:
        raise RuntimeError(
            f"Expected Lua {LUA_VERSION}, but the installed binary reported {version_output!r}"
        )
    print(version_output)


def install(prefix: Path) -> None:
    prefix = prefix.resolve()
    if prefix.exists() and any(prefix.iterdir()):
        raise RuntimeError(f"Refusing to install over non-empty prefix: {prefix}")

    with tempfile.TemporaryDirectory(prefix="coding-adventures-lua-") as directory:
        work_dir = Path(directory)
        archive = work_dir / f"lua-{LUA_VERSION}.tar.gz"
        download_verified_archive(archive)
        source_root = extract_verified_archive(archive, work_dir / "source")
        if os.name == "nt":
            install_windows(source_root, prefix)
        else:
            install_unix(source_root, prefix)
    verify_installation(prefix)


def parse_args(argv: list[str] | None = None) -> argparse.Namespace:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument(
        "--prefix",
        type=Path,
        required=True,
        help="Empty directory in which to install Lua",
    )
    return parser.parse_args(argv)


def main(argv: list[str] | None = None) -> int:
    args = parse_args(argv)
    install(args.prefix)
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
