#!/usr/bin/env python3
"""Validate the pinned Elixir Windows toolchain and every selected BUILD front."""

from __future__ import annotations

import argparse
import hashlib
import json
import re
import shutil

# The only child process is a fixed, argv-only, shell-free git visibility query.
import subprocess  # nosec B404
import sys
from pathlib import Path
from typing import Any

SCHEMA_VERSION = 1
MAX_TEXT_BYTES = 1_048_576
CONTRACT_PATH = Path("code/specs/fixtures/elixir-windows-build-front-v1/contract.json")
WORKFLOW_PATH = Path(".github/workflows/ci.yml")
ELIXIR_ROOTS = (Path("code/packages/elixir"), Path("code/programs/elixir"))
ROOT_FILE = re.compile(
    r"^(code/(?:packages|programs)/elixir/[A-Za-z0-9][A-Za-z0-9_-]*)/"
    r"(BUILD|BUILD_windows)$"
)
DIAGNOSTIC_CODE = re.compile(r"^[A-Z][A-Z0-9_]{2,63}$")
SETUP_ACTION = re.compile(r"erlef/setup-beam@[^\s]+")

CMD_FORBIDDEN = (
    (
        "POSIX_ENV_PREFIX",
        re.compile(
            r"(?:^|(?:&&|\|\||&)\s*)[A-Za-z_][A-Za-z0-9_]*="
            r"[^\s&|]+\s+(?:mix|elixir)\b"
        ),
    ),
    ("POSIX_DEV_NULL", re.compile(r"/dev/null")),
    ("POSIX_CD_DASH", re.compile(r"(?:^|\s)cd\s+-(?:\s|$)")),
    ("POSIX_COMMAND_SUBSTITUTION", re.compile(r"\$\(")),
    ("POSIX_SHELL_TEST", re.compile(r"(?:^|[;&|]\s*)\[\s")),
    ("POSIX_EXPORT", re.compile(r"(?:^|[;&|]\s*)export\s+")),
    ("POSIX_SOURCE", re.compile(r"(?:^|[;&|]\s*)source\s+")),
    ("POSIX_MKDIR_P", re.compile(r"(?:^|[;&|]\s*)mkdir\s+-p(?:\s|$)")),
    ("POSIX_BRACE_GROUP", re.compile(r"(?:^|[;&|]\s*)\{(?:\s|$)")),
    ("POSIX_SUBSHELL", re.compile(r"^\s*\(")),
)


class AuditError(ValueError):
    """Raised when the contract or repository cannot be classified safely."""


def _closed_mapping(value: object, *, label: str, required: set[str]) -> dict[str, Any]:
    if not isinstance(value, dict):
        raise AuditError(f"{label} must be an object")
    keys = set(value)
    if keys != required:
        missing = sorted(required - keys)
        extra = sorted(keys - required)
        raise AuditError(f"{label} keys differ: missing={missing} extra={extra}")
    return value


def parse_json_strict(text: str) -> Any:
    """Parse JSON while rejecting duplicate object keys."""

    def pairs_hook(pairs: list[tuple[str, Any]]) -> dict[str, Any]:
        result: dict[str, Any] = {}
        for key, value in pairs:
            if key in result:
                raise AuditError(f"duplicate JSON key: {key}")
            result[key] = value
        return result

    try:
        return json.loads(text, object_pairs_hook=pairs_hook)
    except json.JSONDecodeError as error:
        raise AuditError(f"invalid JSON: {error.msg}") from error


def validate_contract(value: object) -> dict[str, Any]:
    """Validate the closed process-free contract used by the live audit."""

    contract = _closed_mapping(
        value,
        label="contract",
        required={
            "schema_version",
            "workflow",
            "unsupported_protocol",
            "exceptions",
        },
    )
    if contract["schema_version"] != SCHEMA_VERSION:
        raise AuditError("unsupported contract schema_version")

    workflow = _closed_mapping(
        contract["workflow"],
        label="workflow",
        required={
            "pr_windows_runner",
            "setup_action",
            "elixir_version",
            "otp_version",
            "version_type",
            "setup_condition",
            "affected_build_condition",
        },
    )
    if workflow["pr_windows_runner"] != "windows-2025":
        raise AuditError("workflow runner must be windows-2025")
    if not re.fullmatch(r"erlef/setup-beam@[0-9a-f]{40}", workflow["setup_action"]):
        raise AuditError("workflow setup_action must use an exact commit")
    for key in (
        "elixir_version",
        "otp_version",
        "setup_condition",
        "affected_build_condition",
    ):
        if not isinstance(workflow[key], str) or not workflow[key]:
            raise AuditError(f"workflow.{key} must be a non-empty string")
    if workflow["version_type"] != "strict":
        raise AuditError("workflow.version_type must be strict")

    protocol = _closed_mapping(
        contract["unsupported_protocol"],
        label="unsupported_protocol",
        required={"directive_prefix", "command_prefix", "command_suffix"},
    )
    expected_protocol = {
        "directive_prefix": "# build-tool: unsupported=",
        "command_prefix": "echo BUILD_TOOL_UNSUPPORTED:",
        "command_suffix": " -- skipped",
    }
    if protocol != expected_protocol:
        raise AuditError("unsupported protocol does not match v1")

    exceptions = contract["exceptions"]
    if not isinstance(exceptions, list) or not exceptions:
        raise AuditError("exceptions must be a non-empty array")
    roots: list[str] = []
    for index, raw in enumerate(exceptions):
        item = _closed_mapping(
            raw,
            label=f"exceptions[{index}]",
            required={"root", "code", "class"},
        )
        root = item["root"]
        code = item["code"]
        if not isinstance(root, str) or not re.fullmatch(
            r"code/(?:packages|programs)/elixir/[A-Za-z0-9][A-Za-z0-9_-]*",
            root,
        ):
            raise AuditError(f"invalid exception root: {root!r}")
        if not isinstance(code, str) or not DIAGNOSTIC_CODE.fullmatch(code):
            raise AuditError(f"invalid exception code: {code!r}")
        if item["class"] not in {
            "nif-erts-import-library",
            "target-specific-metal",
        }:
            raise AuditError(f"invalid exception class: {item['class']!r}")
        if root in roots:
            raise AuditError(f"duplicate exception root: {root}")
        roots.append(root)
    if roots != sorted(roots):
        raise AuditError("exception roots must be sorted")
    return contract


def _read_bounded_text(root: Path, relative: Path) -> str:
    repo = root.resolve(strict=True)
    candidate = root / relative
    if candidate.is_symlink():
        raise AuditError(f"symlink is not allowed: {relative.as_posix()}")
    resolved = candidate.resolve(strict=True)
    if not resolved.is_relative_to(repo) or not resolved.is_file():
        raise AuditError(f"path escapes repository: {relative.as_posix()}")
    data = resolved.read_bytes()
    if len(data) > MAX_TEXT_BYTES:
        raise AuditError(f"file exceeds {MAX_TEXT_BYTES} bytes: {relative.as_posix()}")
    try:
        return data.decode("utf-8")
    except UnicodeDecodeError as error:
        raise AuditError(f"file is not UTF-8: {relative.as_posix()}") from error


def load_contract(root: Path) -> dict[str, Any]:
    return validate_contract(parse_json_strict(_read_bounded_text(root, CONTRACT_PATH)))


def git_visible_paths(root: Path) -> list[str]:
    """Return tracked plus untracked, non-ignored Elixir paths."""

    git = shutil.which("git")
    if git is None:
        raise AuditError("git executable is required for repository visibility")
    result = subprocess.run(
        [
            git,
            "ls-files",
            "--cached",
            "--others",
            "--exclude-standard",
            "-z",
            "--",
            *(path.as_posix() for path in ELIXIR_ROOTS),
        ],
        cwd=root,
        check=True,
        capture_output=True,
    )  # nosec B603
    visible: list[str] = []
    for item in result.stdout.split(b"\0"):
        if not item:
            continue
        relative = item.decode("utf-8", errors="surrogateescape").replace("\\", "/")
        candidate = root / relative
        # `--cached` includes an index entry that the working tree deleted.
        # Audit the candidate tree that will be committed while retaining
        # symlinks so the bounded reader can reject them explicitly.
        if candidate.exists() or candidate.is_symlink():
            visible.append(relative)
    return sorted(visible)


def active_commands(text: str) -> list[str]:
    return [
        line.strip()
        for line in text.splitlines()
        if line.strip() and not line.lstrip().startswith("#")
    ]


def is_starlark(text: str) -> bool:
    commands = active_commands(text)
    return bool(commands and commands[0].startswith("load("))


def parse_unsupported_front(text: str, contract: dict[str, Any]) -> str | None:
    """Return the exact unsupported code, or None for an ordinary front."""

    protocol = contract["unsupported_protocol"]
    directives = [
        line.strip()[len(protocol["directive_prefix"]) :]
        for line in text.splitlines()
        if line.strip().startswith(protocol["directive_prefix"])
    ]
    commands = active_commands(text)
    protocol_commands = [
        command
        for command in commands
        if command.startswith(protocol["command_prefix"])
    ]
    if not directives and not protocol_commands:
        return None
    if len(directives) != 1 or len(commands) != 1 or len(protocol_commands) != 1:
        raise AuditError(
            "unsupported front must have exactly one directive and exactly one active command"
        )

    command = protocol_commands[0]
    prefix = protocol["command_prefix"]
    suffix = protocol["command_suffix"]
    if not command.endswith(suffix):
        raise AuditError("unsupported command suffix differs from the contract")
    code = command[len(prefix) : -len(suffix)]
    if directives[0] != code:
        raise AuditError("unsupported directive and command codes differ")
    if not DIAGNOSTIC_CODE.fullmatch(code):
        raise AuditError("unsupported diagnostic code is invalid")
    return code


def cmd_syntax_issues(text: str) -> list[dict[str, Any]]:
    """Return deterministic Windows CMD portability issues."""

    issues: list[dict[str, Any]] = []
    for line_number, raw in enumerate(text.splitlines(), start=1):
        command = raw.strip()
        if not command or command.startswith("#"):
            continue
        for code, pattern in CMD_FORBIDDEN:
            if pattern.search(command):
                issues.append({"code": code, "line": line_number})
    return issues


def _workflow_report(root: Path, contract: dict[str, Any]) -> dict[str, Any]:
    workflow = contract["workflow"]
    text = _read_bounded_text(root, WORKFLOW_PATH)
    runner = workflow["pr_windows_runner"]
    matrix_entry = (
        f'{{"os": "{runner}", "label": "{runner}", '
        '"sharded": False, "shard_index": -1}'
    )
    if matrix_entry not in text:
        raise AuditError("pull-request Windows runner differs from the contract")

    action_refs = SETUP_ACTION.findall(text)
    if not action_refs or any(ref != workflow["setup_action"] for ref in action_refs):
        raise AuditError("setup-beam action identity drift")

    setup_block = re.compile(
        rf"if:\s*{re.escape(workflow['setup_condition'])}\s*\n"
        rf"\s*uses:\s*{re.escape(workflow['setup_action'])}\s*\n"
        r"\s*with:\s*\n"
        rf"\s*elixir-version:\s*'{re.escape(workflow['elixir_version'])}'\s*\n"
        rf"\s*otp-version:\s*'{re.escape(workflow['otp_version'])}'\s*\n"
        rf"\s*version-type:\s*{re.escape(workflow['version_type'])}"
    )
    windows_setup_enabled = bool(setup_block.search(text))
    if not windows_setup_enabled:
        raise AuditError("dynamic Elixir setup block differs from the contract")

    verify_line = 'if [ "${{ needs.detect.outputs.needs_elixir }}" = "true" ]; then'
    windows_verification_enabled = verify_line in text
    if not windows_verification_enabled:
        raise AuditError("Elixir verification remains disabled on Windows")

    affected_line = f"if: {workflow['affected_build_condition']}"
    windows_affected_build_enabled = affected_line in text
    if not windows_affected_build_enabled:
        raise AuditError(
            "affected-package build remains disabled for Elixir on Windows"
        )

    return {
        "path": WORKFLOW_PATH.as_posix(),
        "pr_windows_runner": runner,
        "setup_action": workflow["setup_action"],
        "setup_action_occurrences": len(action_refs),
        "elixir_version": workflow["elixir_version"],
        "otp_version": workflow["otp_version"],
        "version_type": workflow["version_type"],
        "windows_setup_enabled": windows_setup_enabled,
        "windows_verification_enabled": windows_verification_enabled,
        "windows_affected_build_enabled": windows_affected_build_enabled,
    }


def build_report(root: Path) -> dict[str, Any]:
    """Build the deterministic live repository report or fail closed."""

    root = root.resolve()
    contract = load_contract(root)
    paths = git_visible_paths(root)
    canonical: dict[str, str] = {}
    windows: dict[str, str] = {}
    for path in paths:
        match = ROOT_FILE.fullmatch(path)
        if match is None:
            continue
        target = canonical if match.group(2) == "BUILD" else windows
        target[match.group(1)] = path
    orphan_windows = sorted(set(windows) - set(canonical))
    if orphan_windows:
        raise AuditError(f"BUILD_windows lacks canonical BUILD: {orphan_windows}")

    exception_by_root = {item["root"]: item for item in contract["exceptions"]}
    missing_exception_roots = sorted(set(exception_by_root) - set(canonical))
    if missing_exception_roots:
        raise AuditError(
            f"registered exception root is absent: {missing_exception_roots}"
        )

    rows: list[dict[str, Any]] = []
    for package_root in sorted(canonical):
        selected_front = "BUILD_windows" if package_root in windows else "BUILD"
        selected_path = windows.get(package_root, canonical[package_root])
        text = _read_bounded_text(root, Path(selected_path))
        unsupported_code = parse_unsupported_front(text, contract)
        expected = exception_by_root.get(package_root)
        if expected is not None:
            if selected_front != "BUILD_windows":
                raise AuditError(f"{package_root}: exception lacks BUILD_windows")
            if unsupported_code != expected["code"]:
                raise AuditError(f"{package_root}: exception code differs from fixture")
        elif unsupported_code is not None:
            raise AuditError(f"{package_root}: unregistered unsupported front")

        declarative = is_starlark(text)
        issues = [] if declarative or unsupported_code else cmd_syntax_issues(text)
        if unsupported_code is None:
            for line_number, command in enumerate(active_commands(text), start=1):
                lower = command.lower()
                if "skip" in lower or "not supported" in lower:
                    issues.append({"code": "UNREGISTERED_SKIP", "line": line_number})
        if issues:
            codes = ",".join(issue["code"] for issue in issues)
            raise AuditError(f"{selected_path}: Windows front issues: {codes}")

        rows.append(
            {
                "root": package_root,
                "kind": "program" if "/programs/" in package_root else "package",
                "selected_front": selected_front,
                "selected_path": selected_path,
                "classification": "unsupported" if unsupported_code else "native",
                "front_type": "starlark" if declarative else "shell",
                "diagnostic_code": unsupported_code,
                "issues": issues,
            }
        )

    summary = {
        "canonical_fallbacks": sum(row["selected_front"] == "BUILD" for row in rows),
        "declarative_starlark": sum(row["front_type"] == "starlark" for row in rows),
        "native": sum(row["classification"] == "native" for row in rows),
        "package_roots": sum(row["kind"] == "package" for row in rows),
        "program_roots": sum(row["kind"] == "program" for row in rows),
        "total_roots": len(rows),
        "unsupported": sum(row["classification"] == "unsupported" for row in rows),
        "windows_overrides": sum(
            row["selected_front"] == "BUILD_windows" for row in rows
        ),
    }
    contract_bytes = _read_bounded_text(root, CONTRACT_PATH).encode("utf-8")
    return {
        "schema_version": SCHEMA_VERSION,
        "platform": "windows",
        "contract_path": CONTRACT_PATH.as_posix(),
        "contract_sha256": hashlib.sha256(contract_bytes).hexdigest(),
        "workflow": _workflow_report(root, contract),
        "summary": summary,
        "roots": rows,
    }


def render_markdown(report: dict[str, Any]) -> str:
    summary = report["summary"]
    lines = [
        "# Elixir Windows BUILD-front audit",
        "",
        "## Summary",
        "",
        "| Measure | Count |",
        "|---|---:|",
        f"| Total Elixir roots | {summary['total_roots']} |",
        f"| Native Windows fronts | {summary['native']} |",
        f"| Reviewed unsupported fronts | {summary['unsupported']} |",
        f"| Windows overrides | {summary['windows_overrides']} |",
        f"| Canonical fallbacks | {summary['canonical_fallbacks']} |",
        f"| Declarative Starlark fronts | {summary['declarative_starlark']} |",
        "",
        "## Reviewed exceptions",
        "",
        "| Root | Diagnostic code |",
        "|---|---|",
    ]
    for row in report["roots"]:
        if row["classification"] == "unsupported":
            lines.append(f"| `{row['root']}` | `{row['diagnostic_code']}` |")
    return "\n".join(lines) + "\n"


def main(argv: list[str] | None = None) -> int:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--root", type=Path, default=Path.cwd())
    parser.add_argument("--format", choices=("json", "markdown"), default="markdown")
    args = parser.parse_args(argv)

    try:
        report = build_report(args.root)
    except (AuditError, OSError, subprocess.CalledProcessError) as error:
        print(f"Elixir Windows BUILD-front audit failed: {error}", file=sys.stderr)
        return 1
    if args.format == "json":
        print(json.dumps(report, indent=2, sort_keys=True))
    else:
        print(render_markdown(report), end="")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
