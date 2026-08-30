"""Pure, bounded extra-CI toolchain detection over supplied BUILD snapshots.

This module intentionally has no filesystem, environment, process, Git, or
network access. Callers provide every BUILD front as inert text.
"""

from __future__ import annotations

from collections.abc import Mapping, Sequence
from typing import Literal, NotRequired, TypedDict

MAX_BUILD_BYTES = 65_536
MAX_BUILD_LINES = 4_096
MAX_AGGREGATE_BUILD_BYTES = 1_048_576
DECLARATION_PREFIX = "# needs-toolchain:"

CANONICAL_TOOLCHAINS = (
    "cpp",
    "dart",
    "dotnet",
    "elixir",
    "go",
    "haskell",
    "java",
    "kotlin",
    "lua",
    "ocaml",
    "perl",
    "python",
    "ruby",
    "rust",
    "swift",
    "typescript",
)
_CANONICAL_TOOLCHAIN_SET = frozenset(CANONICAL_TOOLCHAINS)


class ToolchainDiagnostic(TypedDict):
    """Stable language-neutral toolchain diagnostic."""

    code: Literal["TOOLCHAIN_UNSUPPORTED"]
    severity: Literal["error"]
    package: NotRequired[str]


class ToolchainEvaluation(TypedDict):
    """Result returned by :func:`evaluate_snapshot`."""

    outcome: Literal["ok", "error"]
    toolchains: dict[str, bool]
    diagnostics: list[ToolchainDiagnostic]


class _PreparedPackage(TypedDict):
    name: str
    language: str
    build_files: dict[str, str]
    extra_toolchains: list[str]


def _utf8_byte_length(value: str) -> int:
    return len(value.encode("utf-8"))


def _logical_line_count(content: str) -> int:
    return content.count("\n") + 1


def parse_extra_toolchains(content: str) -> list[str]:
    """Return stable, deduplicated declarations from one bounded BUILD front."""

    if _utf8_byte_length(content) > MAX_BUILD_BYTES:
        return []
    if _logical_line_count(content) > MAX_BUILD_LINES:
        return []

    lines = content.split("\n")
    declarations: list[str] = []
    seen: set[str] = set()
    for index, raw_line in enumerate(lines):
        line = raw_line
        if index < len(lines) - 1 and line.endswith("\r"):
            line = line[:-1]
        line = line.strip(" \t")
        if not line.startswith(DECLARATION_PREFIX):
            continue

        suffix = line[len(DECLARATION_PREFIX) :]
        if not suffix or suffix[0] not in " \t":
            continue
        name = suffix.strip(" \t")
        if name not in _CANONICAL_TOOLCHAIN_SET or name in seen:
            continue
        seen.add(name)
        declarations.append(name)

    return declarations


def _copy_and_validate_packages(
    packages: Sequence[Mapping[str, object]],
) -> list[_PreparedPackage]:
    aggregate_bytes = 0
    prepared: list[_PreparedPackage] = []

    for package in packages:
        name = package.get("name")
        language = package.get("language")
        raw_build_files = package.get("build_files")
        if not isinstance(name, str) or not isinstance(language, str):
            raise TypeError("toolchain package name and language must be strings")
        if not isinstance(raw_build_files, Mapping):
            raise TypeError("toolchain package build_files must be a mapping")

        build_files: dict[str, str] = {}
        for filename, content in raw_build_files.items():
            if not isinstance(filename, str) or not isinstance(content, str):
                raise TypeError("toolchain BUILD fronts must map strings to strings")
            byte_length = _utf8_byte_length(content)
            if (
                byte_length > MAX_BUILD_BYTES
                or _logical_line_count(content) > MAX_BUILD_LINES
            ):
                raise ValueError(
                    "toolchain BUILD snapshot exceeds its per-file resource ceiling"
                )
            aggregate_bytes += byte_length
            build_files[filename] = content

        prepared.append(
            {
                "name": name,
                "language": language,
                "build_files": build_files,
                "extra_toolchains": [],
            }
        )

    if aggregate_bytes > MAX_AGGREGATE_BUILD_BYTES:
        raise ValueError(
            "toolchain BUILD snapshot exceeds its aggregate resource ceiling"
        )
    return prepared


def _build_file_candidates(platform: str) -> tuple[str, ...]:
    if platform == "darwin":
        return ("BUILD_mac", "BUILD_mac_and_linux", "BUILD")
    if platform == "linux":
        return ("BUILD_linux", "BUILD_mac_and_linux", "BUILD")
    if platform in {"windows", "win32"}:
        return ("BUILD_windows", "BUILD")
    raise ValueError(f"unsupported target platform: {platform}")


def _selected_front(build_files: Mapping[str, str], platform: str) -> str:
    for filename in _build_file_candidates(platform):
        if filename in build_files:
            return build_files[filename]
    return ""


def _toolchain_for_language(language: str) -> str | None:
    if language == "wasm":
        return "rust"
    if language in {"c", "cpp"}:
        return "cpp"
    if language in {"csharp", "fsharp", "dotnet"}:
        return "dotnet"
    if language in _CANONICAL_TOOLCHAIN_SET:
        return language
    return None


def _unsupported(package_name: str | None = None) -> ToolchainEvaluation:
    diagnostic: ToolchainDiagnostic = {
        "code": "TOOLCHAIN_UNSUPPORTED",
        "severity": "error",
    }
    if package_name is not None:
        diagnostic["package"] = package_name
    return {
        "outcome": "error",
        "toolchains": {},
        "diagnostics": [diagnostic],
    }


def evaluate_snapshot(
    platform: str,
    force_full: bool,
    packages: Sequence[Mapping[str, object]],
    scheduled_packages: Sequence[str] | None,
    forced_toolchains: Sequence[str] | None,
) -> ToolchainEvaluation:
    """Evaluate a complete caller-owned toolchain snapshot without host access."""

    prepared = _copy_and_validate_packages(packages)
    for package in prepared:
        selected_front = _selected_front(package["build_files"], platform)
        package["extra_toolchains"] = parse_extra_toolchains(selected_front)

    scheduled = None if scheduled_packages is None else frozenset(scheduled_packages)
    toolchains = {name: force_full for name in CANONICAL_TOOLCHAINS}

    for package in prepared:
        if scheduled is not None and package["name"] not in scheduled:
            continue
        toolchain = _toolchain_for_language(package["language"])
        if toolchain is None:
            return _unsupported(package["name"])
        if force_full:
            continue

        toolchains[toolchain] = True
        for extra_toolchain in package["extra_toolchains"]:
            toolchains[extra_toolchain] = True

    for forced_toolchain in forced_toolchains or ():
        if forced_toolchain not in _CANONICAL_TOOLCHAIN_SET:
            return _unsupported()
        toolchains[forced_toolchain] = True

    return {
        "outcome": "ok",
        "toolchains": toolchains,
        "diagnostics": [],
    }
