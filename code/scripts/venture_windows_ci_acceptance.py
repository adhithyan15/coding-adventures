#!/usr/bin/env python3
"""Derive the Venture native-Windows acceptance flag from a build plan."""

from __future__ import annotations

import argparse
import json
import subprocess
from pathlib import Path
from typing import Any


ACCEPTANCE_PACKAGES = frozenset(
    {
        "rust/venture-browser-windows",
        "unknown/programs/venture-browser",
    }
)


CI_WORKFLOW_PATH = ".github/workflows/ci.yml"


def requires_venture_windows(
    plan: dict[str, Any], *, workflow_changed: bool = False
) -> bool:
    """Return whether this plan must exercise Venture's native Windows gate."""

    if workflow_changed:
        return True
    affected = plan.get("affected_packages")
    if affected is None:
        return True
    if not isinstance(affected, list):
        raise ValueError("affected_packages must be an array or null")
    return bool(ACCEPTANCE_PACKAGES.intersection(affected))


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


def main() -> int:
    parser = argparse.ArgumentParser()
    parser.add_argument("plan", type=Path)
    parser.add_argument("--repo-root", type=Path)
    parser.add_argument("--diff-base")
    args = parser.parse_args()

    if (args.repo_root is None) != (args.diff_base is None):
        parser.error("--repo-root and --diff-base must be provided together")

    with args.plan.open(encoding="utf-8") as handle:
        plan = json.load(handle)
    if not isinstance(plan, dict):
        raise ValueError("build plan root must be an object")

    changed = False
    if args.repo_root is not None and args.diff_base is not None:
        changed = workflow_changed(args.repo_root, args.diff_base)

    required = requires_venture_windows(plan, workflow_changed=changed)
    print(f"required={'true' if required else 'false'}")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
