#!/usr/bin/env python3
"""Classify Python uv BUILD fronts that cannot be repeated in place."""

from __future__ import annotations

import argparse
import json
import re
import subprocess
from collections import Counter
from pathlib import Path
from typing import Any


SCHEMA_VERSION = 1
PYTHON_ROOT = Path("code/packages/python")
VENV_COMMAND = re.compile(r"^uv\s+venv\b.*(?:^|\s)\.venv(?:\s|$)")
PYTHON_PIN = re.compile(r"(?:^|\s)--python(?:=|\s+)([^\s]+)")
LOCAL_DEPENDENCY = re.compile(r"\.\./([A-Za-z0-9_-]+)")
QUOTED_EDITABLE = re.compile(r"(?:^|\s)-e\s+['\"]\.\[dev\]['\"]")
REQUIRES_PYTHON = re.compile(
    r'^requires-python\s*=\s*(["\'])(?P<value>[^"\']+)\1\s*(?:#.*)?$',
    re.MULTILINE,
)


class AuditError(ValueError):
    """Raised when a package front cannot be classified without guessing."""


def active_commands(text: str) -> list[str]:
    """Return the commands that the line-oriented BUILD executor sees."""

    return [
        line.strip()
        for line in text.splitlines()
        if line.strip() and not line.lstrip().startswith("#")
    ]


def parse_front(text: str, *, platform: str) -> dict[str, Any]:
    """Classify one canonical or Windows Python BUILD front."""

    if platform not in {"canonical", "windows"}:
        raise AuditError(f"unsupported platform: {platform}")

    commands = active_commands(text)
    venv_commands = [command for command in commands if VENV_COMMAND.search(command)]
    if len(venv_commands) != 1:
        raise AuditError(
            f"{platform} front must have exactly one uv venv command targeting .venv"
        )

    venv_command = venv_commands[0]
    pin = PYTHON_PIN.search(venv_command)
    pip_commands = [
        command for command in commands if re.match(r"^uv\s+pip\s+", command)
    ]
    pytest_command = next(
        (
            command
            for command in commands
            if re.search(r"(?:^|\s)pytest(?:\s|$)", command)
        ),
        "",
    )

    if platform == "windows":
        explicit = re.match(
            r"^\.venv[\\/]Scripts[\\/]python(?:\.exe)?(?:\s|$)", pytest_command
        )
    else:
        explicit = re.match(r"^\.venv/bin/python(?:3)?(?:\s|$)", pytest_command)

    if explicit:
        test_interpreter = "explicit-venv"
    elif re.match(r"^uv\s+run(?:\s|$)", pytest_command):
        test_interpreter = "uv-run"
    else:
        test_interpreter = "other"

    dependencies: list[str] = []
    for command in pip_commands:
        for dependency in LOCAL_DEPENDENCY.findall(command):
            if dependency not in dependencies:
                dependencies.append(dependency)

    return {
        "venv_command": venv_command,
        "has_clear": bool(re.search(r"(?:^|\s)--clear(?:\s|$)", venv_command)),
        "has_no_project": bool(
            re.search(r"(?:^|\s)--no-project(?:\s|$)", venv_command)
        ),
        "python_pin": pin.group(1) if pin else None,
        "test_interpreter": test_interpreter,
        "all_pip_commands_use_named_venv": bool(pip_commands)
        and all(
            re.search(r"(?:^|\s)--python(?:=|\s+)\.venv(?:\s|$)", command)
            for command in pip_commands
        ),
        "quoted_editable": any(QUOTED_EDITABLE.search(command) for command in commands),
        "local_dependencies": dependencies,
    }


def git_visible_paths(root: Path) -> list[str]:
    """Return tracked plus untracked, non-ignored repository paths."""

    result = subprocess.run(
        [
            "git",
            "ls-files",
            "--cached",
            "--others",
            "--exclude-standard",
            "-z",
            "--",
            PYTHON_ROOT.as_posix(),
        ],
        cwd=root,
        check=True,
        capture_output=True,
    )
    return sorted(
        raw.decode("utf-8", errors="surrogateescape").replace("\\", "/")
        for raw in result.stdout.split(b"\0")
        if raw
    )


def _package_paths(paths: list[str], filename: str) -> dict[str, str]:
    prefix = f"{PYTHON_ROOT.as_posix()}/"
    result: dict[str, str] = {}
    for path in paths:
        if not path.startswith(prefix):
            continue
        parts = path.split("/")
        if len(parts) == 5 and parts[-1] == filename:
            result[parts[-2]] = path
    return result


def _requires_python(text: str, package: str) -> str:
    match = REQUIRES_PYTHON.search(text)
    if match is None:
        raise AuditError(f"{package}: pyproject.toml lacks requires-python")
    return match.group("value")


def _issues(canonical: dict[str, Any], windows: dict[str, Any]) -> list[str]:
    issues: list[str] = []
    for name, front in (("canonical", canonical), ("windows", windows)):
        if not front["has_clear"]:
            issues.append(f"{name}-missing-clear")
        if front["python_pin"] is None:
            issues.append(f"{name}-missing-python-pin")
        if not front["has_no_project"]:
            issues.append(f"{name}-missing-no-project")
        if front["test_interpreter"] != "explicit-venv":
            issues.append(f"{name}-implicit-test-interpreter")
        if not front["all_pip_commands_use_named_venv"]:
            issues.append(f"{name}-pip-without-named-venv")
    if windows["quoted_editable"]:
        issues.append("windows-quoted-editable")
    if canonical["local_dependencies"] != windows["local_dependencies"]:
        issues.append("local-dependency-order-mismatch")
    return sorted(issues)


def _dependency_components(fronts: list[dict[str, Any]]) -> dict[str, list[str]]:
    packages = {front["package"] for front in fronts}
    adjacency: dict[str, set[str]] = {package: set() for package in packages}
    for front in fronts:
        package = front["package"]
        dependencies = set(front["canonical"]["local_dependencies"])
        dependencies.update(front["windows"]["local_dependencies"])
        for dependency in dependencies & packages:
            adjacency[package].add(dependency)
            adjacency[dependency].add(package)

    by_package: dict[str, list[str]] = {}
    remaining = set(packages)
    while remaining:
        start = min(remaining)
        pending = [start]
        component: set[str] = set()
        while pending:
            package = pending.pop()
            if package in component:
                continue
            component.add(package)
            pending.extend(sorted(adjacency[package] - component, reverse=True))
        members = sorted(component)
        for package in members:
            by_package[package] = members
        remaining -= component
    return by_package


def build_report(root: Path) -> dict[str, Any]:
    """Build the deterministic version-1 audit report."""

    root = root.resolve()
    paths = git_visible_paths(root)
    canonical_paths = _package_paths(paths, "BUILD")
    windows_paths = _package_paths(paths, "BUILD_windows")
    pyproject_paths = _package_paths(paths, "pyproject.toml")
    fronts: list[dict[str, Any]] = []

    for package in sorted(windows_paths):
        windows_text = (root / windows_paths[package]).read_text(encoding="utf-8")
        venv_commands = [
            command
            for command in active_commands(windows_text)
            if VENV_COMMAND.search(command)
        ]
        if not venv_commands or all(
            re.search(r"(?:^|\s)--clear(?:\s|$)", command) for command in venv_commands
        ):
            continue
        if package not in canonical_paths or package not in pyproject_paths:
            raise AuditError(f"{package}: missing BUILD or pyproject.toml companion")

        canonical = parse_front(
            (root / canonical_paths[package]).read_text(encoding="utf-8"),
            platform="canonical",
        )
        windows = parse_front(windows_text, platform="windows")
        requires_python = _requires_python(
            (root / pyproject_paths[package]).read_text(encoding="utf-8"), package
        )
        fronts.append(
            {
                "package": package,
                "requires_python": requires_python,
                "canonical": canonical,
                "windows": windows,
                "local_dependency_symmetric": canonical["local_dependencies"]
                == windows["local_dependencies"],
                "issues": _issues(canonical, windows),
            }
        )

    components = _dependency_components(fronts)
    for front in fronts:
        front["dependency_component"] = components[front["package"]]

    requires = Counter(front["requires_python"] for front in fronts)
    component_count = len({tuple(members) for members in components.values()})
    return {
        "schema_version": SCHEMA_VERSION,
        "python_package_count": len(canonical_paths),
        "summary": {
            "dependency_components": component_count,
            "fronts_missing_canonical_clear": sum(
                not front["canonical"]["has_clear"] for front in fronts
            ),
            "fronts_missing_canonical_python_pin": sum(
                front["canonical"]["python_pin"] is None for front in fronts
            ),
            "fronts_missing_windows_clear": sum(
                not front["windows"]["has_clear"] for front in fronts
            ),
            "fronts_missing_windows_python_pin": sum(
                front["windows"]["python_pin"] is None for front in fronts
            ),
            "fronts_with_local_dependencies": sum(
                bool(front["windows"]["local_dependencies"]) for front in fronts
            ),
            "non_idempotent_fronts": len(fronts),
            "requires_python": dict(sorted(requires.items())),
        },
        "fronts": fronts,
    }


def render_markdown(report: dict[str, Any]) -> str:
    """Render a human-readable view without adding unreported facts."""

    summary = report["summary"]
    lines = [
        "# Python uv BUILD-front idempotence audit",
        "",
        "## Summary",
        "",
        "| Measure | Count |",
        "|---|---:|",
        f"| Python packages | {report['python_package_count']} |",
        f"| Non-idempotent fronts | {summary['non_idempotent_fronts']} |",
        f"| Fronts with local dependencies | {summary['fronts_with_local_dependencies']} |",
        f"| Dependency components | {summary['dependency_components']} |",
        "",
        "## Fronts",
        "",
        "| Package | Requires Python | Local dependencies | Component | Issues |",
        "|---|---|---|---|---|",
    ]
    for front in report["fronts"]:
        dependencies = ", ".join(front["windows"]["local_dependencies"]) or "—"
        component = ", ".join(front["dependency_component"])
        issues = ", ".join(f"`{issue}`" for issue in front["issues"])
        lines.append(
            f"| `{front['package']}` | `{front['requires_python']}` | "
            f"{dependencies} | {component} | {issues} |"
        )
    return "\n".join(lines) + "\n"


def main() -> int:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--root", type=Path, default=Path.cwd())
    parser.add_argument("--format", choices=("json", "markdown"), default="markdown")
    args = parser.parse_args()

    report = build_report(args.root)
    if args.format == "json":
        print(json.dumps(report, indent=2, sort_keys=True))
    else:
        print(render_markdown(report), end="")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
