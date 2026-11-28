#!/usr/bin/env python3
"""Export a Visual Studio developer environment for later GitHub Actions steps.

The Windows runner already contains Visual Studio and ``vswhere.exe``. This
script asks ``vswhere`` for the current C++ toolchain, calls ``vcvarsall.bat``,
and writes only the variables changed by that batch file to ``GITHUB_ENV``.
It replaces the unmaintained ``ilammy/msvc-dev-cmd`` JavaScript action, whose
Node 20 runtime is retired on current GitHub-hosted runners. A later setup step
can add another compiler directory to ``GITHUB_PATH``; ``--promote-vs-clang-cl``
revalidates the selected Visual Studio compiler against the active MSVC STL and
re-adds that directory without rerunning ``vcvarsall``.
"""

from __future__ import annotations

import argparse
import locale
import os
import re
import shutil
import subprocess  # nosec B404 -- fixed local runner tools only
import tempfile
from collections.abc import Callable, Iterable, Mapping
from pathlib import Path

PATH_LIKE_VARIABLES = frozenset({"PATH", "INCLUDE", "LIB", "LIBPATH"})
ENVIRONMENT_NAME = re.compile(r"^[A-Za-z_][A-Za-z0-9_()]*$")
CLANG_VERSION_RE = re.compile(r"clang version (\d+)\.(\d+)(?:\.(\d+))?")
STL_CLANG_MIN_RE = re.compile(r"expected Clang (\d+) or newer")
VS_LLVM_VARS = {
    "x64": ("llvmX64",),
    "arm64": ("llvmArm64",),
    "x86": ("llvmX86", "llvmX64"),
}


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


def append_github_path(path: Path, directory: str) -> None:
    """Prepend one verified directory for later GitHub Actions steps."""

    if not directory or "\n" in directory or "\r" in directory:
        raise ValueError("GITHUB_PATH entries must be non-empty single lines")
    with path.open("a", encoding="utf-8", newline="\n") as output:
        output.write(f"{directory}\n")


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


def developer_command_script(vcvarsall: Path, architecture: str) -> str:
    """Build a temporary batch wrapper without ``cmd /c`` quote ambiguity."""

    return (
        "@echo off\r\n"
        f'call "{vcvarsall}" {architecture}\r\n'
        "if errorlevel 1 exit /b %errorlevel%\r\n"
        "set\r\n"
    )


def capture_developer_environment(
    vcvarsall: Path, architecture: str, *, comspec: Path
) -> dict[str, str]:
    """Call vcvarsall and return the complete resulting command environment."""

    # Put the quoted vcvarsall invocation in a batch file instead of embedding
    # it in the argument after ``cmd /c``. Python's Windows argv serialization
    # and cmd.exe's first-quote rule otherwise disagree about the quotes around
    # Visual Studio's space-containing path. CALL is required here so control
    # returns to the wrapper and the resulting environment can be printed.
    with tempfile.NamedTemporaryFile(
        mode="w",
        suffix=".cmd",
        prefix="setup-msvc-",
        encoding="utf-8",
        newline="",
        delete=False,
    ) as wrapper:
        wrapper.write(developer_command_script(vcvarsall, architecture))
        wrapper_path = Path(wrapper.name)

    try:
        result = subprocess.run(  # nosec B603 -- fixed cmd.exe and reviewed batch path
            [str(comspec), "/d", "/c", wrapper_path.name],
            cwd=wrapper_path.parent,
            check=False,
            capture_output=True,
            text=True,
            encoding=locale.getpreferredencoding(False),
            errors="replace",
        )
    finally:
        wrapper_path.unlink(missing_ok=True)
    if result.returncode != 0:
        detail = result.stderr.strip() or result.stdout.strip() or "no command output"
        raise RuntimeError(
            f"vcvarsall failed with exit code {result.returncode}: {detail[-2000:]}"
        )
    return parse_environment(result.stdout.splitlines())


def parse_clang_version(output: str) -> tuple[int, int, int] | None:
    """Extract a ``(major, minor, patch)`` tuple from ``clang-cl --version``."""

    match = CLANG_VERSION_RE.search(output)
    if not match:
        return None
    major, minor, patch = match.groups()
    return (int(major), int(minor), int(patch) if patch is not None else 0)


def probe_clang_cl_version(executable: Path) -> tuple[int, int, int] | None:
    """Run ``clang-cl --version`` on an existing candidate executable."""

    result = subprocess.run(  # nosec B603 -- resolved local clang-cl executable
        [str(executable), "--version"],
        check=False,
        capture_output=True,
        text=True,
        encoding=locale.getpreferredencoding(False),
        errors="replace",
    )
    if result.returncode != 0:
        return None
    return parse_clang_version(result.stdout or result.stderr)


def _find_env_key(environment: Mapping[str, str], target: str) -> str | None:
    target_cf = target.casefold()
    for key in environment:
        if key.casefold() == target_cf:
            return key
    return None


def _default_has_clang_cl(directory: str) -> bool:
    return (Path(directory) / "clang-cl.exe").is_file()


def _default_probe_dir_clang_cl(directory: str) -> tuple[int, int, int] | None:
    return probe_clang_cl_version(Path(directory) / "clang-cl.exe")


def prefer_newest_clang_cl(
    environment: Mapping[str, str],
    architecture: str,
    *,
    has_clang_cl: Callable[[str], bool] = _default_has_clang_cl,
    probe_version: Callable[[str], tuple[int, int, int] | None] = _default_probe_dir_clang_cl,
) -> tuple[
    dict[str, str],
    list[tuple[str, tuple[int, int, int] | None]],
    tuple[str, tuple[int, int, int]] | None,
]:
    """Promote the newest ``clang-cl.exe`` directory ahead of older ones on PATH.

    GitHub Windows runners can ship a standalone ``C:\\Program Files\\LLVM\\bin``
    earlier on PATH than Visual Studio's newer bundled LLVM toolchain
    (``VC\\Tools\\Llvm\\x64\\bin``), causing MSVC STL ``STL1000`` version-mismatch
    errors when compiling C++ with ``clang-cl``. Promoting the newest available
    ``clang-cl`` directory ahead of the first ``clang-cl`` entry preserves MSVC
    ``cl.exe`` / ``link.exe`` precedence at the front of PATH while selecting the
    most recent compatible Clang driver.
    """

    updated = dict(environment)
    path_key = _find_env_key(updated, "PATH")
    if not path_key or not updated[path_key]:
        return updated, [], None

    entries = [e for e in updated[path_key].split(";") if e]
    search_dirs: list[str] = list(entries)
    seen = {e.casefold() for e in search_dirs}

    for var_name in VS_LLVM_VARS.get(architecture, ()):
        actual_key = _find_env_key(updated, var_name)
        if actual_key and updated[actual_key]:
            candidate = updated[actual_key]
            if candidate.casefold() not in seen:
                seen.add(candidate.casefold())
                search_dirs.append(candidate)

    candidates: list[tuple[str, tuple[int, int, int] | None]] = []
    first_clang_path_index: int | None = None
    for directory in search_dirs:
        if not has_clang_cl(directory):
            continue
        version = probe_version(directory)
        candidates.append((directory, version))
        if first_clang_path_index is None:
            dir_cf = directory.casefold()
            for idx, entry in enumerate(entries):
                if entry.casefold() == dir_cf:
                    first_clang_path_index = idx
                    break

    versioned = [(d, v) for d, v in candidates if v is not None]
    if not versioned:
        return updated, candidates, None

    best_dir, best_ver = versioned[0]
    for directory, version in versioned[1:]:
        if version > best_ver:
            best_dir, best_ver = directory, version

    first_dir = candidates[0][0]
    if best_dir.casefold() != first_dir.casefold():
        insert_at = first_clang_path_index if first_clang_path_index is not None else len(entries)
        new_entries = entries[:insert_at] + [best_dir] + entries[insert_at:]
        updated[path_key] = deduplicate_path_like(";".join(new_entries))

    return updated, candidates, (best_dir, best_ver)


def msvc_stl_min_clang_major(environment: Mapping[str, str]) -> int | None:
    """Read the minimum Clang major version required by MSVC's ``yvals_core.h``."""

    include_key = _find_env_key(environment, "INCLUDE")
    if not include_key or not environment[include_key]:
        return None
    for directory in environment[include_key].split(";"):
        if not directory:
            continue
        header = Path(directory) / "yvals_core.h"
        if not header.is_file():
            continue
        try:
            text = header.read_text(encoding="utf-8", errors="replace")
        except OSError:
            continue
        match = STL_CLANG_MIN_RE.search(text)
        if match:
            return int(match.group(1))
    return None


def select_compatible_vs_clang_cl(
    environment: Mapping[str, str],
    architecture: str,
    *,
    has_clang_cl: Callable[[str], bool] = _default_has_clang_cl,
    probe_version: Callable[[str], tuple[int, int, int] | None] = _default_probe_dir_clang_cl,
) -> tuple[str, tuple[int, int, int], int]:
    """Resolve the vcvars-selected Clang and prove it meets the active MSVC STL."""

    for var_name in VS_LLVM_VARS.get(architecture, ()):
        actual_key = _find_env_key(environment, var_name)
        directory = environment.get(actual_key, "") if actual_key else ""
        if not directory:
            continue
        if not has_clang_cl(directory):
            raise RuntimeError(f"Visual Studio LLVM path has no clang-cl.exe: {directory}")
        version = probe_version(directory)
        if version is None:
            raise RuntimeError(f"could not read clang-cl version from {directory}")
        minimum = msvc_stl_min_clang_major(environment)
        if minimum is None:
            raise RuntimeError("could not determine the MSVC STL minimum Clang version")
        if version[0] < minimum:
            raise RuntimeError(
                f"Visual Studio clang-cl {version[0]} is older than MSVC STL minimum {minimum}"
            )
        return directory, version, minimum
    raise RuntimeError(f"vcvarsall did not provide a Visual Studio LLVM path for {architecture}")


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


def selected_clang_cl_executable(
    selected: tuple[str, tuple[int, int, int]] | None,
) -> str | None:
    """Return the absolute executable selected by the PATH promotion policy."""

    if selected is None:
        return None
    directory, _version = selected
    return str((Path(directory) / "clang-cl.exe").resolve())


def main() -> int:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--arch", default="x64", choices=("x64", "x86", "arm64"))
    parser.add_argument(
        "--promote-vs-clang-cl",
        action="store_true",
        help="re-add the vcvars-selected compatible clang-cl directory to GITHUB_PATH without rerunning vcvarsall",
    )
    args = parser.parse_args()

    if os.name != "nt":
        raise RuntimeError("MSVC developer setup must run on Windows")
    if args.promote_vs_clang_cl:
        github_path = os.environ.get("GITHUB_PATH")
        if not github_path:
            raise RuntimeError("GITHUB_PATH must be set to promote clang-cl")
        directory, version, minimum = select_compatible_vs_clang_cl(
            os.environ, args.arch
        )
        append_github_path(Path(github_path), directory)
        print(
            f"Re-promoted Visual Studio clang-cl {'.'.join(map(str, version))} "
            f"from {directory}; MSVC STL minimum is {minimum}"
        )
        return 0

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
    after, clang_candidates, selected_clang = prefer_newest_clang_cl(after, args.arch)
    clang_override = selected_clang_cl_executable(selected_clang)
    if clang_override is not None:
        after["ISO_CLANG_CL"] = clang_override
    stl_min_clang = msvc_stl_min_clang_major(after)
    compiler, linker = require_msvc_tools(after)
    changes = changed_environment(before, after)
    append_github_environment(Path(github_env), changes)

    print(f"Configured MSVC with {vcvarsall}")
    print(f"Compiler: {compiler}")
    print(f"Linker: {linker}")
    for directory, version in clang_candidates:
        ver_str = ".".join(map(str, version)) if version is not None else "unknown"
        print(f"clang-cl candidate: {directory} ({ver_str})")
    if selected_clang is not None:
        sel_dir, sel_ver = selected_clang
        print(f"Selected clang-cl: {sel_dir} ({'.'.join(map(str, sel_ver))})")
        if stl_min_clang is not None and sel_ver[0] < stl_min_clang:
            print(
                f"WARNING: Selected clang-cl {sel_ver[0]} is older than MSVC STL "
                f"minimum Clang {stl_min_clang}"
            )
    if stl_min_clang is not None:
        print(f"MSVC STL minimum Clang major: {stl_min_clang}")
    print(f"Exported {len(changes)} changed environment variables")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
