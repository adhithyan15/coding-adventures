"""
executor.py -- Parallel Build Execution
========================================

This module runs BUILD commands for packages that need rebuilding. It respects
the dependency graph by building packages in topological levels: packages in
the same level have no dependencies on each other and can run in parallel.

Execution strategy
------------------

1. Get the ``independent_groups()`` from the dependency graph -- these are the
   parallel levels.
2. For each level, run all packages in that level concurrently using a
   ``ThreadPoolExecutor``.
3. For each package, execute its BUILD commands sequentially via
   ``subprocess.run``, with ``cwd`` set to the package directory.
4. If a package fails (any command returns non-zero), mark all transitive
   dependents as "dep-skipped" -- there's no point building them.

Build results
-------------

Each package gets a ``BuildResult`` with its status, stdout/stderr output,
and wall-clock duration.
"""

from __future__ import annotations

import subprocess
import time
from concurrent.futures import ThreadPoolExecutor, as_completed
from dataclasses import dataclass, field
from pathlib import Path

from build_tool.cache import BuildCache
from build_tool.discovery import Package
from build_tool.hasher import hash_deps, hash_package
from build_tool.resolver import DirectedGraph


@dataclass
class BuildResult:
    """The result of building a single package.

    Attributes:
        package_name: The package's qualified name.
        status: One of "built", "failed", "skipped", "dep-skipped".
        duration: Wall-clock seconds spent building (0.0 for skipped).
        stdout: Combined stdout from all BUILD commands.
        stderr: Combined stderr from all BUILD commands.
        return_code: The exit code of the last failing command, or 0.
    """

    package_name: str
    status: str  # "built", "failed", "skipped", "dep-skipped"
    duration: float = 0.0
    stdout: str = ""
    stderr: str = ""
    return_code: int = 0


def _run_package_build(package: Package) -> BuildResult:
    """Execute all BUILD commands for a single package.

    Commands are run sequentially. If any command fails, we stop and
    return a "failed" result. All commands run with ``cwd`` set to the
    package directory and inherit the current environment.
    """
    start = time.monotonic()
    all_stdout: list[str] = []
    all_stderr: list[str] = []

    for command in package.build_commands:
        try:
            result = subprocess.run(
                command,
                shell=True,
                cwd=package.path,
                capture_output=True,
                text=True,
                timeout=600,  # 10-minute timeout per command
            )
        except subprocess.TimeoutExpired:
            elapsed = time.monotonic() - start
            return BuildResult(
                package_name=package.name,
                status="failed",
                duration=elapsed,
                stdout="".join(all_stdout),
                stderr="".join(all_stderr) + "\nBuild command timed out after 600s",
                return_code=124,
            )

        all_stdout.append(result.stdout)
        all_stderr.append(result.stderr)

        if result.returncode != 0:
            elapsed = time.monotonic() - start
            return BuildResult(
                package_name=package.name,
                status="failed",
                duration=elapsed,
                stdout="".join(all_stdout),
                stderr="".join(all_stderr),
                return_code=result.returncode,
            )

    elapsed = time.monotonic() - start
    return BuildResult(
        package_name=package.name,
        status="built",
        duration=elapsed,
        stdout="".join(all_stdout),
        stderr="".join(all_stderr),
        return_code=0,
    )


def execute_builds(
    packages: list[Package],
    graph: DirectedGraph,
    cache: BuildCache,
    package_hashes: dict[str, str],
    deps_hashes: dict[str, str],
    force: bool = False,
    dry_run: bool = False,
    max_jobs: int | None = None,
    affected_set: set[str] | None = None,
) -> dict[str, BuildResult]:
    """Execute BUILD commands for packages, respecting dependency order.

    Uses ``independent_groups()`` from the dependency graph to determine
    which packages can run in parallel. For each level, packages are built
    concurrently with a ThreadPoolExecutor.

    If a package fails, all its transitive dependents are marked as
    "dep-skipped".

    Args:
        packages: All discovered packages.
        graph: The dependency graph.
        cache: The build cache (for skip detection).
        package_hashes: Per-package source hashes.
        deps_hashes: Per-package dependency hashes.
        force: If True, rebuild everything regardless of cache.
        dry_run: If True, don't actually build -- just report what would build.
        max_jobs: Maximum number of parallel workers. None = CPU count.

    Returns:
        A dict mapping package names to their BuildResult.
    """
    # Build a lookup from name to Package
    pkg_by_name: dict[str, Package] = {p.name: p for p in packages}

    # Get the parallel levels
    groups = graph.independent_groups()

    results: dict[str, BuildResult] = {}
    failed_packages: set[str] = set()

    for level in groups:
        # Determine what to build in this level
        to_build: list[Package] = []

        for name in level:
            if name not in pkg_by_name:
                continue

            # Check if a dependency failed.
            # In our graph, edges go dep -> pkg, so a package's dependencies
            # are its predecessors (transitive_dependents walks backwards).
            dep_failed = False
            for dep in graph.transitive_dependents(name):
                if dep in failed_packages:
                    dep_failed = True
                    break

            if dep_failed:
                results[name] = BuildResult(
                    package_name=name,
                    status="dep-skipped",
                )
                continue

            # Check if we need to build
            # Priority: git-diff affected_set > hash-based cache
            if affected_set is not None and name not in affected_set:
                results[name] = BuildResult(
                    package_name=name,
                    status="skipped",
                )
                continue

            pkg_hash = package_hashes.get(name, "")
            dep_hash = deps_hashes.get(name, "")

            if affected_set is None and not force and not cache.needs_build(name, pkg_hash, dep_hash):
                results[name] = BuildResult(
                    package_name=name,
                    status="skipped",
                )
                continue

            if dry_run:
                results[name] = BuildResult(
                    package_name=name,
                    status="would-build",
                )
                continue

            to_build.append(pkg_by_name[name])

        if not to_build or dry_run:
            continue

        # Execute this level in parallel
        workers = max_jobs if max_jobs else min(len(to_build), 8)

        with ThreadPoolExecutor(max_workers=workers) as executor:
            futures = {
                executor.submit(_run_package_build, pkg): pkg
                for pkg in to_build
            }

            for future in as_completed(futures):
                pkg = futures[future]
                try:
                    result = future.result()
                except Exception as exc:
                    result = BuildResult(
                        package_name=pkg.name,
                        status="failed",
                        stderr=str(exc),
                        return_code=1,
                    )

                results[pkg.name] = result

                # Update cache
                if result.status == "built":
                    cache.record(
                        pkg.name,
                        package_hashes.get(pkg.name, ""),
                        deps_hashes.get(pkg.name, ""),
                        "success",
                    )
                elif result.status == "failed":
                    failed_packages.add(pkg.name)
                    cache.record(
                        pkg.name,
                        package_hashes.get(pkg.name, ""),
                        deps_hashes.get(pkg.name, ""),
                        "failed",
                    )

    return results
