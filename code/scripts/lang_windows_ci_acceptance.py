#!/usr/bin/env python3
"""Select native LANG execution from the planner's Windows affected closure.

The closure already includes dependents: a runtime or code generator edit
selects twig-aot without duplicating its dependency graph here. Windows BUILD
overrides may differ from the Linux detector's default, so prefer that view.
"""

from __future__ import annotations

import argparse
import json
import subprocess
from pathlib import Path
from typing import Any

ACCEPTANCE_PACKAGES = frozenset({"rust/twig-aot", "rust/lang-aot"})
GATE_PATHS = (
    ".github/workflows/ci.yml",
    "code/scripts/lang_windows_ci_acceptance.py",
    "code/scripts/tests/test_lang_windows_ci_acceptance.py",
    "code/scripts/setup_msvc_dev_cmd.py",
)


def requires_lang_windows(plan: dict[str, Any], *, gate_changed: bool = False) -> bool:
    """Unknown/full plans run; empty or unrelated Windows closures do not."""
    overrides = plan.get("platform_overrides")
    if overrides is not None and not isinstance(overrides, dict):
        raise ValueError("platform_overrides must be an object or null")
    windows = (overrides or {}).get("windows")
    if windows is not None and not isinstance(windows, dict):
        raise ValueError("Windows override must be an object or null")
    affected = (plan if windows is None else windows).get("affected_packages")
    if affected is not None and (
        not isinstance(affected, list)
        or any(not isinstance(package, str) for package in affected)
    ):
        raise ValueError("affected_packages must be an array of strings or null")
    return (
        gate_changed
        or affected is None
        or bool(ACCEPTANCE_PACKAGES.intersection(affected))
    )


def gate_changed(repo_root: Path, diff_base: str) -> bool:
    """Self-test the gate when its wiring changes outside any package."""
    result = subprocess.run(
        ["git", "diff", "--quiet", f"{diff_base}...HEAD", "--", *GATE_PATHS],
        cwd=repo_root,
        check=False,
    )
    if result.returncode not in (0, 1):
        raise RuntimeError(f"git diff exited with status {result.returncode}")
    return result.returncode == 1


def main() -> int:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("plan", type=Path)
    parser.add_argument("--repo-root", type=Path)
    parser.add_argument("--diff-base")
    args = parser.parse_args()
    if (args.repo_root is None) != (args.diff_base is None):
        parser.error("--repo-root and --diff-base must be provided together")
    plan = json.loads(args.plan.read_text(encoding="utf-8"))
    if not isinstance(plan, dict):
        raise TypeError("build plan root must be an object")
    changed = args.repo_root is not None and gate_changed(
        args.repo_root, args.diff_base
    )
    required = requires_lang_windows(plan, gate_changed=changed)
    print(f"required={'true' if required else 'false'}")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
