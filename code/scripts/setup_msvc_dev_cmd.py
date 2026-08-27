#!/usr/bin/env python3
"""Export a Visual Studio developer environment for later GitHub Actions steps.

The Windows runner already contains Visual Studio and ``vswhere.exe``. This
script asks ``vswhere`` for the current C++ toolchain, calls ``vcvarsall.bat``,
and writes only the variables changed by that batch file to ``GITHUB_ENV``.
It replaces the unmaintained ``ilammy/msvc-dev-cmd`` JavaScript action, whose
Node 20 runtime is retired on current GitHub-hosted runners.
"""

from __future__ import annotations

import argparse
import locale
import os
import re
import shutil
import subprocess  # nosec B404 -- fixed local runner tools only
from collections.abc import Iterable, Mapping
from pathlib import Path

PATH_LIKE_VARIABLES = frozenset({"PATH", "INCLUDE", "LIB", "LIBPATH"})
ENVIRONMENT_NAME = re.compile(r"^[A-Za-z_][A-Za-z0-9_()]*$")


def parse_environment(lines: Iterable[str]) -> dict[str, str]:
    """Parse ``cmd.exe set`` output, ignoring drive aliases and fluff."""

    environment: dict[str, str] = {}
    for raw_line in lines:
        line = raw_line.rstrip("\r\n")
        if "=" not in line:
            continue
        name, value = line.split("=", 1)
        if name.startswith("=") or not ENVIRONMENT_NAME.fullmatch(name):
            continue
        environment[name] = value
    return environment


def deduplicate_path_like(value: str) -> str:
    """Preserve path precedence while removing case-insensitive duplicates."""

    result: list[str] = []
    seen: set[str] = set()
    for entry in value.split(";"):
        key = entry.casefold()
        if key in seen:
            continue
        seen.add(key)
        result.append(entry)
    return ";".join(result)


def changed_environment(
    before: Mapping[str, str], after: Mapping[str, str]
) -> dict[str, str]:
    """Return the variables vcvars added or changed, with stable key order."""

    before_casefolded = {name.casefold(): value for name, value in before.items()}
    changes: dict[str, str] = {}
    for name in sorted(after, key=str.casefold):
        value = after[name]
        if name.upper() in PATH_LIKE_VARIABLES:
            value = deduplicate_path_like(value)
        if before_casefolded.get(name.casefold()) != value:
            changes[name] = value
    return changes


def append_github_environment(path: Path, changes: Mapping[str, str]) -> None:
    """Append changes using the GitHub environment-file protocol."""

    with path.open("a", encoding="utf-8", newline="\n") as output:
        for index, (name, value) in enumerate(changes.items()):
            if "\n" not in value and "\r" not in value:
                output.write(f"{name}={value}\n")
                continue
            delimiter = f"MSVC_ENV_{index}"
            while delimiter in value:
                delimiter += "_X"
            output.write(f"{name}<<{delimiter}\n{value}\n{delimiter}\n")


def find_vcvarsall(program_files_x86: Path) -> Path:
    """Resolve the latest installed VS C++ environment through vswhere."""

    vswhere = (
        program_files_x86 / "Microsoft Visual Studio" / "Installer" / "vswhere.exe"
    )
    if not vswhere.is_file():
        raise RuntimeError(f"vswhere.exe was not found at {vswhere}")

    result = subprocess.run(  # nosec B603 -- resolved fixed executable
        [
            str(vswhere),
            "-latest",
            "-products",
            "*",
            "-requires",
            "Microsoft.VisualStudio.Component.VC.Tools.x86.x64",
            "-property",
            "installationPath",
        ],
        check=True,
        capture_output=True,
        text=True,
        encoding=locale.getpreferredencoding(False),
        errors="replace",
    )
    installations = [
        line.strip() for line in result.stdout.splitlines() if line.strip()
    ]
    if len(installations) != 1:
        raise RuntimeError(
            f"vswhere returned {len(installations)} Visual Studio installations"
        )

    vcvarsall = Path(installations[0]) / "VC" / "Auxiliary" / "Build" / "vcvarsall.bat"
    if not vcvarsall.is_file():
        raise RuntimeError(f"vcvarsall.bat was not found at {vcvarsall}")
    return vcvarsall


def capture_developer_environment(
    vcvarsall: Path, architecture: str, *, comspec: Path
) -> dict[str, str]:
    """Call vcvarsall and return the complete resulting command environment."""

    # Invoke the batch file the same way the upstream action did: as a command
    # in cmd.exe, not through CALL. On the Windows 2025 / Visual Studio 2026
    # runner, CALL incorrectly propagates exit status 1 even though the same
    # vcvarsall command is valid. The harmless leading SET also keeps the
    # command from beginning with a quoted path, avoiding cmd.exe /c's special
    # first-quote parsing rule.
    command = f'set >nul && "{vcvarsall}" {architecture} >nul && set'
    result = subprocess.run(  # nosec B603 -- fixed cmd.exe and reviewed batch path
        [str(comspec), "/d", "/c", command],
        check=False,
        capture_output=True,
        text=True,
        encoding=locale.getpreferredencoding(False),
        errors="replace",
    )
    if result.returncode != 0:
        detail = result.stderr.strip() or result.stdout.strip() or "no command output"
        raise RuntimeError(
            f"vcvarsall failed with exit code {result.returncode}: {detail[-2000:]}"
        )
    return parse_environment(result.stdout.splitlines())


def require_msvc_tools(environment: Mapping[str, str]) -> tuple[Path, Path]:
    """Prove that the captured PATH resolves the compiler and MSVC linker."""

    path = environment.get("Path") or environment.get("PATH")
    if not path:
        raise RuntimeError("vcvarsall did not produce PATH")
    compiler = shutil.which("cl.exe", path=path)
    linker = shutil.which("link.exe", path=path)
    if compiler is None or linker is None:
        raise RuntimeError("vcvarsall PATH does not resolve cl.exe and link.exe")
    return Path(compiler), Path(linker)


def main() -> int:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--arch", default="x64", choices=("x64", "x86", "arm64"))
    args = parser.parse_args()

    if os.name != "nt":
        raise RuntimeError("MSVC developer setup must run on Windows")
    program_files = os.environ.get("ProgramFiles(x86)")
    comspec = os.environ.get("ComSpec")
    github_env = os.environ.get("GITHUB_ENV")
    if not program_files or not comspec or not github_env:
        raise RuntimeError(
            "ProgramFiles(x86), ComSpec, and GITHUB_ENV must be set on the runner"
        )

    before = dict(os.environ)
    vcvarsall = find_vcvarsall(Path(program_files))
    after = capture_developer_environment(vcvarsall, args.arch, comspec=Path(comspec))
    compiler, linker = require_msvc_tools(after)
    changes = changed_environment(before, after)
    append_github_environment(Path(github_env), changes)

    print(f"Configured MSVC with {vcvarsall}")
    print(f"Compiler: {compiler}")
    print(f"Linker: {linker}")
    print(f"Exported {len(changes)} changed environment variables")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
