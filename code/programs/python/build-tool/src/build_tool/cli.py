"""
cli.py -- Command-Line Interface
=================================

This is the entry point for the build tool CLI. It ties together all the
modules: discovery, resolution, hashing, caching, execution, and reporting.

Usage::

    build-tool                        # Auto-detect root, build changed packages
    build-tool --root /path/to/repo   # Specify root explicitly
    build-tool --force                # Rebuild everything
    build-tool --dry-run              # Show what would build without building
    build-tool --jobs 4               # Limit parallel workers
    build-tool --language python      # Only build Python packages

The flow is:
1. Discover packages (walk DIRS/BUILD files)
2. Filter by language if specified
3. Resolve dependencies (parse pyproject.toml, .gemspec, go.mod)
4. Hash all packages
5. Load cache, determine what needs building
6. If --dry-run, print what would build and exit
7. Execute builds (parallel by level)
8. Update and save cache
9. Print report
10. Exit with code 1 if any builds failed
"""

from __future__ import annotations

import argparse
import sys
from pathlib import Path

from build_tool.cache import BuildCache
from build_tool.discovery import discover_packages
from build_tool.executor import execute_builds
from build_tool.gitdiff import get_changed_files, map_files_to_packages
from build_tool.hasher import hash_deps, hash_package
from build_tool.reporter import print_report
from build_tool.resolver import resolve_dependencies


def _find_repo_root(start: Path | None = None) -> Path | None:
    """Walk up from ``start`` (or cwd) looking for a ``.git`` directory.

    Returns the directory containing ``.git``, or None if not found.
    """
    current = start or Path.cwd()
    current = current.resolve()

    while True:
        if (current / ".git").exists():
            return current
        parent = current.parent
        if parent == current:
            # Reached filesystem root
            return None
        current = parent


def main(argv: list[str] | None = None) -> int:
    """Main entry point for the build tool CLI.

    Args:
        argv: Command-line arguments (defaults to sys.argv[1:]).

    Returns:
        Exit code: 0 for success, 1 if any builds failed.
    """
    parser = argparse.ArgumentParser(
        description="Incremental, parallel monorepo build tool"
    )
    parser.add_argument(
        "--root",
        type=Path,
        default=None,
        help="Repo root directory (auto-detect from .git if not given)",
    )
    parser.add_argument(
        "--force",
        action="store_true",
        help="Rebuild everything regardless of cache",
    )
    parser.add_argument(
        "--dry-run",
        action="store_true",
        help="Show what would build without actually building",
    )
    parser.add_argument(
        "--jobs",
        type=int,
        default=None,
        help="Maximum number of parallel build jobs",
    )
    parser.add_argument(
        "--language",
        choices=["python", "ruby", "go", "all"],
        default="all",
        help="Only build packages of this language",
    )
    parser.add_argument(
        "--diff-base",
        type=str,
        default="origin/main",
        help="Git ref to diff against for change detection (default: origin/main)",
    )
    parser.add_argument(
        "--cache-file",
        type=Path,
        default=Path(".build-cache.json"),
        help="Path to the build cache file (used as fallback when git diff unavailable)",
    )

    args = parser.parse_args(argv)

    # Step 1: Find repo root
    root = args.root
    if root is None:
        root = _find_repo_root()
        if root is None:
            print("Error: Could not find repo root (.git directory).", file=sys.stderr)
            print("Use --root to specify the repo root.", file=sys.stderr)
            return 1

    root = root.resolve()

    # The build starts from code/ directory
    code_root = root / "code"
    if not code_root.exists():
        print(f"Error: {code_root} does not exist.", file=sys.stderr)
        return 1

    # Step 2: Discover packages
    packages = discover_packages(code_root)

    if not packages:
        print("No packages found.", file=sys.stderr)
        return 0

    # Step 3: Filter by language
    if args.language != "all":
        packages = [p for p in packages if p.language == args.language]
        if not packages:
            print(f"No {args.language} packages found.", file=sys.stderr)
            return 0

    print(f"Discovered {len(packages)} packages")

    # Step 4: Resolve dependencies
    graph = resolve_dependencies(packages)

    # Step 5: Determine which packages need building
    #
    # Default mode: git-diff based change detection.
    # Git is the source of truth — no cache file needed.
    # Fallback: hash-based cache (for local dev when not on a branch).
    affected_set: set[str] | None = None

    if not args.force:
        # Try git-diff mode first (the default)
        changed_files = get_changed_files(root, args.diff_base)
        if changed_files:
            package_paths = {pkg.name: pkg.path for pkg in packages}
            changed_packages = map_files_to_packages(changed_files, package_paths, root)
            if changed_packages:
                affected_set = graph.affected_nodes(changed_packages)
                print(f"Git diff: {len(changed_packages)} packages changed, "
                      f"{len(affected_set)} affected (including dependents)")
            else:
                print("Git diff: no package files changed — nothing to build")
                affected_set = set()
        else:
            print("Git diff unavailable — falling back to hash-based cache")

    # Step 6: Hash all packages (needed for cache fallback)
    package_hashes: dict[str, str] = {}
    deps_hashes: dict[str, str] = {}

    for pkg in packages:
        package_hashes[pkg.name] = hash_package(pkg)
        deps_hashes[pkg.name] = hash_deps(pkg.name, graph, package_hashes)

    # Step 7: Load cache (fallback if git diff didn't work)
    cache_path = args.cache_file
    if not cache_path.is_absolute():
        cache_path = root / cache_path

    cache = BuildCache()
    cache.load(cache_path)

    # Steps 8-9: Execute builds
    results = execute_builds(
        packages=packages,
        graph=graph,
        cache=cache,
        package_hashes=package_hashes,
        deps_hashes=deps_hashes,
        force=args.force,
        dry_run=args.dry_run,
        max_jobs=args.jobs,
        affected_set=affected_set,
    )

    # Step 10: Save cache (as secondary record, not primary mechanism)
    if not args.dry_run:
        cache.save(cache_path)

    # Step 10: Print report
    print_report(results)

    # Step 11: Exit code
    has_failures = any(r.status == "failed" for r in results.values())
    return 1 if has_failures else 0


if __name__ == "__main__":
    sys.exit(main())
