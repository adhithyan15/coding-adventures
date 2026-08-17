#!/usr/bin/env python3
"""Derive the complete Mosaic XAML TaskApp acceptance flag from a build plan."""

from __future__ import annotations

import argparse
import json
import subprocess
from pathlib import Path
from typing import Any


ACCEPTANCE_PACKAGES = frozenset(
    {
        "rust/mosaic-app-bindings",
        "rust/mosaic-app-capi",
        "rust/mosaic-app-conformance",
        "rust/mosaic-app-runtime",
        "rust/mosaic-compile",
        "rust/mosaic-emit-xaml",
        "rust/mosaic-package-artifact-builder",
        "rust/moslayout-compiler",
        "rust/mosmodel-compiler",
        "rust/mosstyle-compiler",
        "rust/task-mosaic-app",
        "mosaic/programs/task-app",
    }
)
ACCEPTANCE_PACKAGE_PREFIXES = ("mosaic/mosaic-pkg-",)
CI_WORKFLOW_PATH = ".github/workflows/ci.yml"
def requires_mosaic_xaml_windows(
    plan: dict[str, Any], *, workflow_changed: bool = False
) -> bool:
    """Return whether this plan must build WinUI and exercise its Rust binding."""

    if workflow_changed:
        return True
    affected = plan.get("affected_packages")
    if affected is None:
        return True
    if not isinstance(affected, list):
        raise ValueError("affected_packages must be an array or null")
    return any(
        package in ACCEPTANCE_PACKAGES
        or package.startswith(ACCEPTANCE_PACKAGE_PREFIXES)
        for package in affected
    )


def workflow_changed(repo_root: Path, diff_base: str) -> bool:
    """Return whether the main CI workflow differs from the selected base."""

    result = subprocess.run(
        [
            "git",
            "diff",
            "--quiet",
            f"{diff_base}...HEAD",
            "--",
            CI_WORKFLOW_PATH,
        ],
        cwd=repo_root,
        check=False,
    )
    if result.returncode not in (0, 1):
        raise RuntimeError(f"git diff exited with status {result.returncode}")
    return result.returncode == 1


def validate_taskapp_report(report: dict[str, Any]) -> None:
    """Require a zero-degradation native-complete XAML TaskApp."""

    if report.get("backend") != "xaml":
        raise ValueError("TaskApp degradation report backend must be xaml")
    if report.get("nativeComplete") is not True:
        raise ValueError("TaskApp XAML must be native-complete")
    degradations = report.get("degradations")
    if not isinstance(degradations, list):
        raise ValueError("TaskApp degradation report must contain a degradations array")
    if degradations:
        raise ValueError(f"TaskApp XAML must have zero degradations, observed {degradations}")


def main() -> int:
    parser = argparse.ArgumentParser()
    parser.add_argument("plan", type=Path, nargs="?")
    parser.add_argument("--repo-root", type=Path)
    parser.add_argument("--diff-base")
    parser.add_argument("--validate-taskapp-report", type=Path)
    args = parser.parse_args()

    if args.validate_taskapp_report is not None:
        if args.plan is not None or args.repo_root is not None or args.diff_base is not None:
            parser.error("--validate-taskapp-report cannot be combined with plan detection")
        with args.validate_taskapp_report.open(encoding="utf-8") as handle:
            report = json.load(handle)
        if not isinstance(report, dict):
            raise ValueError("TaskApp degradation report root must be an object")
        validate_taskapp_report(report)
        print("TaskApp XAML report is native-complete with zero degradations")
        return 0

    if args.plan is None:
        parser.error("plan is required unless --validate-taskapp-report is used")

    if (args.repo_root is None) != (args.diff_base is None):
        parser.error("--repo-root and --diff-base must be provided together")

    with args.plan.open(encoding="utf-8") as handle:
        plan = json.load(handle)
    if not isinstance(plan, dict):
        raise ValueError("build plan root must be an object")

    changed = False
    if args.repo_root is not None and args.diff_base is not None:
        changed = workflow_changed(args.repo_root, args.diff_base)

    required = requires_mosaic_xaml_windows(plan, workflow_changed=changed)
    print(f"required={'true' if required else 'false'}")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
