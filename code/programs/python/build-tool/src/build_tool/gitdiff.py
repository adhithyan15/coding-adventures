"""
gitdiff.py -- Git-based change detection
==========================================

This module determines which packages changed by comparing the current
branch against a base branch (typically ``origin/main``) using git.

This is the DEFAULT change detection mechanism. It replaces the cache-based
approach with a stateless one — git itself is the source of truth. No cache
file needed.

How it works::

    Step 1: Run ``git diff --name-only <base>...HEAD``
            This gives us every file that changed between the base and HEAD.

    Step 2: Map each changed file to a package by matching its path prefix
            against discovered package paths.
            e.g., "code/packages/python/logic-gates/src/gates.py"
                  → package "python/logic-gates"

    Step 3: Use the directed graph's ``affected_nodes()`` to find all
            packages that transitively depend on the changed packages.

    Step 4: Return the full set of affected packages to build.

The beauty of this approach: no state to manage, no cache file to commit,
and it works perfectly with CI (every PR naturally has a base branch to
diff against).
"""

from __future__ import annotations

import subprocess
from pathlib import Path


def get_changed_files(
    repo_root: Path,
    diff_base: str = "origin/main",
) -> list[str]:
    """Get the list of files changed between diff_base and HEAD.

    Uses ``git diff --name-only <base>...HEAD`` which shows files changed
    on the current branch since it diverged from the base. The three-dot
    syntax means "changes since the merge base", which is exactly what
    we want for PR builds.

    For pushes to main, use ``HEAD~1`` as the base to compare against
    the previous commit.

    Args:
        repo_root: The repository root directory (contains .git).
        diff_base: The git ref to compare against (default: origin/main).

    Returns:
        List of changed file paths relative to repo_root.
        Empty list if the diff command fails (e.g., no common ancestor).
    """
    try:
        result = subprocess.run(
            ["git", "diff", "--name-only", f"{diff_base}...HEAD"],
            capture_output=True,
            text=True,
            cwd=repo_root,
            timeout=30,
        )
        if result.returncode != 0:
            # Fallback: try two-dot diff (for when three-dot fails,
            # e.g., shallow clones or missing remote refs)
            result = subprocess.run(
                ["git", "diff", "--name-only", diff_base, "HEAD"],
                capture_output=True,
                text=True,
                cwd=repo_root,
                timeout=30,
            )
            if result.returncode != 0:
                return []

        return [
            line.strip()
            for line in result.stdout.strip().split("\n")
            if line.strip()
        ]
    except (subprocess.TimeoutExpired, FileNotFoundError):
        return []


def map_files_to_packages(
    changed_files: list[str],
    package_paths: dict[str, Path],
    repo_root: Path,
) -> set[str]:
    """Map changed file paths to package names.

    For each changed file, check which package directory it falls under.
    A file belongs to a package if its path starts with the package's
    directory path (relative to repo root).

    Args:
        changed_files: File paths relative to repo root.
        package_paths: Mapping of package name → absolute package directory.
        repo_root: The repository root directory.

    Returns:
        Set of package names that contain at least one changed file.

    Example::

        changed_files = ["code/packages/python/logic-gates/src/gates.py"]
        package_paths = {"python/logic-gates": Path("/repo/code/packages/python/logic-gates")}
        → {"python/logic-gates"}
    """
    changed = set()
    # Convert package paths to relative strings for prefix matching
    relative_pkg_paths: dict[str, str] = {}
    for name, abs_path in package_paths.items():
        try:
            rel = abs_path.relative_to(repo_root)
            relative_pkg_paths[name] = str(rel)
        except ValueError:
            continue

    for filepath in changed_files:
        for pkg_name, pkg_rel_path in relative_pkg_paths.items():
            if filepath.startswith(pkg_rel_path + "/") or filepath == pkg_rel_path:
                changed.add(pkg_name)
                break  # a file belongs to at most one package

    return changed
