"""Audit the shared TypeScript compiler-path portability contract.

The repository intentionally keeps one extendable TypeScript base config. A
plain relative path in that file is anchored to the base file, not to a child
project. TypeScript 5.5's ``${configDir}`` template is the portable way to say
"the directory of the project being compiled". This module keeps that small
but high-leverage build invariant executable without requiring Node or an npm
install in the CI detection job.
"""

from __future__ import annotations

import argparse
import json
import re
import subprocess
from collections.abc import Iterable
from dataclasses import dataclass
from pathlib import Path

SHARED_BASE = Path("code/packages/typescript/tsconfig.base.json")
TYPESCRIPT_AREAS = (
    Path("code/packages/typescript"),
    Path("code/programs/typescript"),
)
PORTABLE_PATHS = {
    "rootDir": "${configDir}/src",
    "outDir": "${configDir}/dist",
}
MINIMUM_CONFIG_DIR_VERSION = (5, 5, 0)
VERSION_RE = re.compile(r"^(\d+)\.(\d+)\.(\d+)")


@dataclass(frozen=True)
class Issue:
    """One stable repository-contract diagnostic."""

    code: str
    path: str
    message: str


@dataclass(frozen=True)
class AuditSummary:
    """Counts and diagnostics emitted by one repository audit."""

    total_projects: int
    shared_projects: int
    inherited_root_dir: int
    inherited_out_dir: int
    locked_compilers: int
    issues: tuple[Issue, ...]


class PortabilityError(ValueError):
    """Raised when the shared TypeScript config contract is not portable."""


def _display_path(root: Path, path: Path) -> str:
    try:
        return path.relative_to(root).as_posix()
    except ValueError:
        return path.as_posix()


def _git_visible_files(root: Path, patterns: Iterable[str]) -> list[Path] | None:
    """Return Git-visible paths when ``root`` is a checkout.

    Unit tests use synthetic directories without Git metadata, so callers fall
    back to a bounded filesystem walk when this returns ``None``.
    """

    if not (root / ".git").exists():
        return None
    result = subprocess.run(
        ["git", "-C", str(root), "ls-files", "--", *patterns],
        check=False,
        capture_output=True,
        text=True,
        encoding="utf-8",
    )
    if result.returncode != 0:
        return None
    return [root / line for line in result.stdout.splitlines() if line]


def _area_files(root: Path, filename: str) -> list[Path]:
    patterns = [f"{area.as_posix()}/**/{filename}" for area in TYPESCRIPT_AREAS]
    tracked = _git_visible_files(root, patterns)
    if tracked is not None:
        return sorted(tracked)

    found: list[Path] = []
    for relative_area in TYPESCRIPT_AREAS:
        area = root / relative_area
        if not area.exists():
            continue
        found.extend(
            path
            for path in area.rglob(filename)
            if "node_modules" not in path.parts
        )
    return sorted(found)


def _read_json(root: Path, path: Path, issues: list[Issue]) -> object | None:
    try:
        return json.loads(path.read_text(encoding="utf-8"))
    except (OSError, UnicodeError, json.JSONDecodeError) as error:
        issues.append(
            Issue(
                "JSON_INVALID",
                _display_path(root, path),
                f"cannot read strict UTF-8 JSON: {error}",
            )
        )
        return None


def _compiler_options(document: object) -> dict[str, object]:
    if not isinstance(document, dict):
        return {}
    options = document.get("compilerOptions")
    return options if isinstance(options, dict) else {}


def _extends_shared_base(root: Path, config_path: Path, document: object) -> bool:
    if not isinstance(document, dict):
        return False
    extends = document.get("extends")
    if not isinstance(extends, str) or not extends:
        return False
    candidate = (config_path.parent / extends).resolve()
    return candidate == (root / SHARED_BASE).resolve()


def _parse_version(version: object) -> tuple[int, int, int] | None:
    if not isinstance(version, str):
        return None
    match = VERSION_RE.match(version)
    if match is None:
        return None
    return tuple(int(component) for component in match.groups())  # type: ignore[return-value]


def audit_repository(root: Path) -> AuditSummary:
    """Inspect one checkout and return its shared-config portability summary."""

    root = root.resolve()
    issues: list[Issue] = []
    base_path = root / SHARED_BASE
    base_document = _read_json(root, base_path, issues)
    base_options = _compiler_options(base_document)
    for option, expected in PORTABLE_PATHS.items():
        actual = base_options.get(option)
        if actual != expected:
            issues.append(
                Issue(
                    "SHARED_PATH_NOT_PORTABLE",
                    SHARED_BASE.as_posix(),
                    f"compilerOptions.{option} must be {expected!r}, got {actual!r}",
                )
            )

    total_projects = 0
    shared_projects = 0
    inherited_root_dir = 0
    inherited_out_dir = 0
    for manifest_path in _area_files(root, "package.json"):
        manifest = _read_json(root, manifest_path, issues)
        if not isinstance(manifest, dict):
            continue
        scripts = manifest.get("scripts")
        if not isinstance(scripts, dict) or not scripts.get("build"):
            continue
        config_path = manifest_path.with_name("tsconfig.json")
        if not config_path.is_file():
            continue
        config = _read_json(root, config_path, issues)
        if config is None:
            continue
        total_projects += 1
        if not _extends_shared_base(root, config_path, config):
            continue
        shared_projects += 1
        options = _compiler_options(config)
        if "rootDir" not in options:
            inherited_root_dir += 1
        if "outDir" not in options:
            inherited_out_dir += 1

    locked_compilers = 0
    for lock_path in _area_files(root, "package-lock.json"):
        lock = _read_json(root, lock_path, issues)
        if not isinstance(lock, dict):
            continue
        packages = lock.get("packages")
        if not isinstance(packages, dict):
            continue
        compiler = packages.get("node_modules/typescript")
        if not isinstance(compiler, dict) or "version" not in compiler:
            continue
        locked_compilers += 1
        raw_version = compiler.get("version")
        version = _parse_version(raw_version)
        if version is None:
            issues.append(
                Issue(
                    "TYPESCRIPT_VERSION_INVALID",
                    _display_path(root, lock_path),
                    f"cannot parse locked TypeScript version {raw_version!r}",
                )
            )
        elif version < MINIMUM_CONFIG_DIR_VERSION:
            issues.append(
                Issue(
                    "TYPESCRIPT_TOO_OLD",
                    _display_path(root, lock_path),
                    "${configDir} requires TypeScript 5.5 or newer; "
                    f"lock contains {raw_version}",
                )
            )

    return AuditSummary(
        total_projects=total_projects,
        shared_projects=shared_projects,
        inherited_root_dir=inherited_root_dir,
        inherited_out_dir=inherited_out_dir,
        locked_compilers=locked_compilers,
        issues=tuple(issues),
    )


def validate_repository(root: Path) -> AuditSummary:
    """Return a clean audit or raise one stable combined diagnostic."""

    summary = audit_repository(root)
    if summary.issues:
        details = "\n".join(
            f"{issue.code}: {issue.path}: {issue.message}" for issue in summary.issues
        )
        raise PortabilityError(f"TypeScript tsconfig portability failed:\n{details}")
    return summary


def main() -> int:
    parser = argparse.ArgumentParser(
        description="Validate shared TypeScript tsconfig path portability."
    )
    parser.add_argument("--root", type=Path, default=Path.cwd())
    args = parser.parse_args()

    try:
        summary = validate_repository(args.root)
    except PortabilityError as error:
        print(error)
        return 1

    print(
        "TypeScript tsconfig portability passed: "
        f"projects={summary.total_projects} "
        f"shared={summary.shared_projects} "
        f"inherited_rootDir={summary.inherited_root_dir} "
        f"inherited_outDir={summary.inherited_out_dir} "
        f"compiler_locks={summary.locked_compilers}"
    )
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
