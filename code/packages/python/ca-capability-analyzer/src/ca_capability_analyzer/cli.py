"""Command-line interface for the capability analyzer.

Provides three commands:

    ca-capability-analyzer detect <path>
        Scan source files and report detected capabilities as JSON.

    ca-capability-analyzer check <path>
        Compare detected capabilities against the manifest and exit
        with code 0 (pass) or 1 (fail).

    ca-capability-analyzer banned <path>
        Scan source files for banned dynamic execution constructs.

## Usage in CI

The `check` command is designed for use in BUILD files and CI pipelines:

    .venv/bin/python -m ca_capability_analyzer check src/

It exits with code 0 if all detected capabilities are declared in the
manifest, and code 1 if any undeclared capabilities are found. This
makes it a drop-in addition to any BUILD file.
"""

import argparse
import contextlib
import json
import sys
from pathlib import Path

from ca_capability_analyzer.analyzer import (
    analyze_directory,
    analyze_file,
)
from ca_capability_analyzer.banned import detect_banned_constructs
from ca_capability_analyzer.manifest import (
    compare_capabilities,
    default_manifest,
    load_manifest,
)


def _find_manifest(search_dir: Path) -> Path | None:
    """Search for required_capabilities.json in the directory and parents.

    Walks up from the given directory looking for the manifest file.
    Stops at the first one found, or at the repository root.
    """
    current = search_dir.resolve()
    for _ in range(10):  # Safety limit on directory traversal depth
        manifest = current / "required_capabilities.json"
        if manifest.exists():
            return manifest
        # Stop at git root
        if (current / ".git").exists():
            break
        parent = current.parent
        if parent == current:
            break
        current = parent
    return None


def _find_package_name(search_dir: Path) -> str:
    """Infer the qualified package name from the directory path.

    Looks for a path component matching a known language, then uses
    the language and the leaf directory as the package name.
    """
    parts = search_dir.resolve().parts
    languages = {"python", "ruby", "go", "rust", "typescript"}
    for i, part in enumerate(parts):
        if part in languages and i + 1 < len(parts):
            return f"{part}/{parts[i + 1]}"
    return f"unknown/{search_dir.name}"


def cmd_detect(args: argparse.Namespace) -> int:
    """Detect capabilities in source files and output JSON."""
    path = Path(args.path)
    exclude_tests = args.exclude_tests

    if path.is_file():
        detected = analyze_file(path)
    else:
        detected = analyze_directory(path, exclude_tests=exclude_tests)

    output = [cap.as_dict() for cap in detected]
    print(json.dumps(output, indent=2))
    return 0


def cmd_check(args: argparse.Namespace) -> int:
    """Check capabilities against manifest and report pass/fail."""
    path = Path(args.path)
    exclude_tests = args.exclude_tests

    # Find and load manifest
    search_dir = path if path.is_dir() else path.parent
    manifest_path = args.manifest or _find_manifest(search_dir)

    if manifest_path:
        manifest = load_manifest(manifest_path)
    else:
        package_name = _find_package_name(search_dir)
        manifest = default_manifest(package_name)

    # Detect capabilities
    if path.is_file():
        detected = analyze_file(path)
    else:
        detected = analyze_directory(path, exclude_tests=exclude_tests)

    # Compare
    result = compare_capabilities(detected, manifest)
    print(result.summary())

    if args.json:
        output = {
            "passed": result.passed,
            "errors": [cap.as_dict() for cap in result.errors],
            "warnings": result.warnings,
            "matched": [cap.as_dict() for cap in result.matched],
        }
        print(json.dumps(output, indent=2))

    return 0 if result.passed else 1


def cmd_banned(args: argparse.Namespace) -> int:
    """Scan for banned dynamic execution constructs."""
    path = Path(args.path)

    violations: list = []
    if path.is_file():
        violations = detect_banned_constructs(path)
    else:
        skip_dirs = {
            ".venv",
            "__pycache__",
            ".git",
            "node_modules",
            ".mypy_cache",
            ".pytest_cache",
            ".ruff_cache",
        }
        for py_file in path.rglob("*.py"):
            if any(part in skip_dirs for part in py_file.parts):
                continue
            with contextlib.suppress(SyntaxError):
                violations.extend(detect_banned_constructs(py_file))

    if violations:
        print(f"FAIL — {len(violations)} banned construct(s) found:\n")
        for v in violations:
            print(f"  {v}")
        return 1
    else:
        print("PASS — no banned constructs found.")
        return 0


def main() -> None:
    """Entry point for the ca-capability-analyzer CLI."""
    parser = argparse.ArgumentParser(
        prog="ca-capability-analyzer",
        description=(
            "Static analyzer for OS capability detection in Python source code."
        ),
    )
    subparsers = parser.add_subparsers(dest="command", required=True)

    # detect command
    detect_parser = subparsers.add_parser(
        "detect",
        help="Detect capabilities in source files and output JSON.",
    )
    detect_parser.add_argument("path", help="File or directory to analyze.")
    detect_parser.add_argument(
        "--exclude-tests",
        action="store_true",
        help="Skip test directories.",
    )

    # check command
    check_parser = subparsers.add_parser(
        "check",
        help="Compare detected capabilities against manifest.",
    )
    check_parser.add_argument("path", help="File or directory to analyze.")
    check_parser.add_argument(
        "--manifest",
        help="Path to required_capabilities.json (auto-detected if omitted).",
    )
    check_parser.add_argument(
        "--exclude-tests",
        action="store_true",
        help="Skip test directories.",
    )
    check_parser.add_argument(
        "--json",
        action="store_true",
        help="Also output results as JSON.",
    )

    # banned command
    banned_parser = subparsers.add_parser(
        "banned",
        help="Scan for banned dynamic execution constructs.",
    )
    banned_parser.add_argument("path", help="File or directory to scan.")

    args = parser.parse_args()

    if args.command == "detect":
        sys.exit(cmd_detect(args))
    elif args.command == "check":
        sys.exit(cmd_check(args))
    elif args.command == "banned":
        sys.exit(cmd_banned(args))


if __name__ == "__main__":
    main()
