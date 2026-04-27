"""
cli.py -- Command-Line Interface
=================================

This is the entry point for the build tool CLI. It ties together all the
modules: discovery, resolution, hashing, caching, execution, and reporting.

Usage::

    build-tool                         # Auto-detect root, build changed packages
    build-tool --root /path/to/repo    # Specify root explicitly
    build-tool --force                 # Rebuild everything
    build-tool --dry-run               # Show what would build without building
    build-tool --jobs 4                # Limit parallel workers
    build-tool --language python       # Only build Python packages
    build-tool --diff-base origin/main # Git ref for change detection
    build-tool --detect-languages      # Output needed language toolchains
    build-tool --emit-plan plan.json   # Write build plan and exit
    build-tool --plan-file plan.json   # Read plan, skip discovery

The flow is:
1. Discover packages (walk recursive BUILD files)
2. Evaluate Starlark BUILD files
3. Filter by language if specified
4. Resolve dependencies (parse pyproject.toml, .gemspec, go.mod, etc.)
5. Git-diff change detection (default mode)
6. Emit plan or detect languages (early exit modes)
7. Hash all packages
8. Load cache, determine what needs building
9. Execute builds (parallel by level)
10. Update and save cache
11. Print report
12. Exit with code 1 if any builds failed
"""

from __future__ import annotations

import argparse
import sys
from pathlib import Path

from build_tool.cache import BuildCache
from build_tool.ci_workflow import (
    CI_WORKFLOW_PATH,
    analyze_ci_workflow_changes,
    sorted_toolchains,
)
from build_tool.discovery import Package, discover_packages
from build_tool.executor import execute_builds
from build_tool.gitdiff import get_changed_files, map_files_to_packages
from build_tool.hasher import hash_deps, hash_package
from build_tool.plan import (
    BuildPlan,
    PackageEntry,
    read_plan,
    write_plan,
)
from build_tool.reporter import print_report
from build_tool.resolver import DirectedGraph, resolve_dependencies
from build_tool.starlark_evaluator import (
    evaluate_build_file,
    generate_commands,
    is_starlark_build,
)
from build_tool.validator import validate_build_contracts

# Optional progress bar integration. When the progress bar package is
# installed, builds get a real-time terminal UI showing which packages are
# building and overall completion. When unavailable, builds work identically
# but without the visual feedback.
try:
    from progress_bar import Tracker
except ImportError:
    Tracker = None  # type: ignore[assignment, misc]


# ALL_TOOLCHAINS is the canonical list of supported build toolchains in the
# monorepo. The order is stable and matches the order used in CI setup.
ALL_TOOLCHAINS = ["python", "ruby", "go", "typescript", "rust", "elixir", "lua", "perl", "swift", "haskell", "dotnet"]

# ALL_TOOLCHAINS is the canonical list of CI toolchains we can request.
ALL_TOOLCHAINS = [
    "python",
    "ruby",
    "go",
    "typescript",
    "rust",
    "elixir",
    "lua",
    "perl",
    "swift",
    "java",
    "kotlin",
    "haskell",
    "dotnet",
]

# SHARED_PREFIXES are repo paths that, when changed, still mean every
# toolchain needs rebuilding. ci.yml is handled separately via patch analysis
# so toolchain-scoped edits do not fan out across the whole repo.
SHARED_PREFIXES: list[str] = []


def _toolchain_for_package_language(language: str) -> str:
    if language == "wasm":
        return "rust"
    if language in {"csharp", "fsharp", "dotnet"}:
        return "dotnet"
    return language


def _toolchain_for_language(language: str) -> str:
    """Map a package language to the toolchain CI needs to install."""
    if language == "wasm":
        return "rust"
    if language in {"csharp", "fsharp", "dotnet"}:
        return "dotnet"
    return language


def _expand_affected_set_with_prereqs(
    graph: DirectedGraph,
    affected_set: set[str] | None,
) -> set[str] | None:
    """Ensure all transitive prerequisites of affected packages are also scheduled.

    This matters on fresh CI runners: some package BUILD steps materialize
    local dependency state (for example sibling TypeScript file: dependencies
    under node_modules), and dependents may fail if those prerequisite packages
    are skipped just because their own sources didn't change.

    Args:
        graph: The dependency graph.
        affected_set: Packages from git diff, or None for "all".

    Returns:
        Expanded set with all transitive prerequisites included.
    """
    if affected_set is None:
        return None

    expanded = set(affected_set)
    queue = list(affected_set)

    while queue:
        current = queue.pop(0)
        for pred in graph.predecessors(current):
            if pred not in expanded:
                expanded.add(pred)
                queue.append(pred)

    return expanded


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
        choices=[
            "python",
            "ruby",
            "go",
            "typescript",
            "rust",
            "elixir",
            "lua",
            "perl",
            "swift",
            "haskell",
            "wasm",
            "csharp",
            "fsharp",
            "dotnet",
            "all",
        ],
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
        help="Path to the build cache file (fallback for git diff)",
    )
    parser.add_argument(
        "--emit-plan",
        type=str,
        default=None,
        help="Write build plan JSON to this path and exit (used by CI detect job)",
    )
    parser.add_argument(
        "--plan-file",
        type=str,
        default=None,
        help="Read build plan JSON, skip discovery/resolution/diff",
    )
    parser.add_argument(
        "--detect-languages",
        action="store_true",
        help="Output which language toolchains are needed based on git diff, then exit",
    )
    parser.add_argument(
        "--validate-build-files",
        action="store_true",
        help="Validate BUILD/CI metadata contracts before continuing",
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

    # --plan-file mode: read a pre-computed plan and skip discovery,
    # resolution, and change detection. This is the fast path for CI
    # build jobs that received a plan from the detect job.
    if args.plan_file is not None:
        return _run_from_plan(args, root)

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

    # Step 2b: Evaluate Starlark BUILD files.
    #
    # For each discovered package, check if its BUILD file is Starlark.
    # If so, evaluate it through the Python starlark-interpreter to extract
    # declared targets (with srcs, deps, build commands). This replaces the
    # raw shell command lines with generated commands from the rule.
    starlark_count = 0
    updated_packages: list[Package] = []
    for pkg in packages:
        if is_starlark_build(pkg.build_content):
            try:
                result = evaluate_build_file(
                    str(pkg.path / "BUILD"),
                    str(pkg.path),
                    str(root),
                )
                if result.targets:
                    t = result.targets[0]
                    pkg = Package(
                        name=pkg.name,
                        path=pkg.path,
                        build_commands=generate_commands(t),
                        language=pkg.language,
                        build_content=pkg.build_content,
                        is_starlark=True,
                        declared_srcs=t.srcs,
                        declared_deps=t.deps,
                    )
                    starlark_count += 1
                else:
                    pkg = Package(
                        name=pkg.name,
                        path=pkg.path,
                        build_commands=pkg.build_commands,
                        language=pkg.language,
                        build_content=pkg.build_content,
                        is_starlark=True,
                        declared_srcs=pkg.declared_srcs,
                        declared_deps=pkg.declared_deps,
                    )
            except Exception as exc:
                print(f"Warning: Starlark eval failed for {pkg.name}: {exc}", file=sys.stderr)
        updated_packages.append(pkg)
    packages = updated_packages

    if starlark_count:
        print(f"Evaluated {starlark_count} Starlark BUILD files")

    # Step 3: Filter by language
    if args.language != "all":
        packages = [p for p in packages if p.language == args.language]
        if not packages:
            print(f"No {args.language} packages found.", file=sys.stderr)
            return 0

    if args.validate_build_files:
        validation_error = validate_build_contracts(root, packages)
        if validation_error is not None:
            print(
                "BUILD/CI validation failed:\n"
                f"  - {validation_error}\n"
                "Fix the BUILD file or CI workflow so isolated and full-build runs stay correct.",
                file=sys.stderr,
            )
            return 1

    print(f"Discovered {len(packages)} packages")

    # Step 4: Resolve dependencies
    graph = resolve_dependencies(packages)

    # Step 5: Determine which packages need building
    #
    # Default mode: git-diff based change detection.
    # Git is the source of truth -- no cache file needed.
    # Fallback: hash-based cache (for local dev when not on a branch).
    affected_set: set[str] | None = None
    ci_toolchains: frozenset[str] = frozenset()

    if not args.force:
        # Try git-diff mode first (the default)
        changed_files = get_changed_files(root, args.diff_base)
        if changed_files:
            if CI_WORKFLOW_PATH in changed_files:
                ci_change = analyze_ci_workflow_changes(root, args.diff_base)
                if ci_change.requires_full_rebuild:
                    print("Git diff: ci.yml changed in shared ways -- rebuilding everything")
                    args.force = True
                    affected_set = None
                else:
                    ci_toolchains = ci_change.toolchains
                    if ci_toolchains:
                        print(
                            "Git diff: ci.yml changed only toolchain-scoped setup for "
                            + ", ".join(sorted_toolchains(ci_toolchains))
                        )

            shared_changed = any(
                f == prefix or f.startswith(prefix + "/")
                for f in changed_files
                if f != CI_WORKFLOW_PATH
                for prefix in SHARED_PREFIXES
            )
            if shared_changed:
                print("Git diff: shared files changed -- rebuilding everything")
                args.force = True
                affected_set = None
            else:
                package_paths = {pkg.name: pkg.path for pkg in packages}
                changed_packages = map_files_to_packages(
                    changed_files, package_paths, root, packages
                )
                if changed_packages:
                    affected_set = graph.affected_nodes(changed_packages)
                    affected_set = _expand_affected_set_with_prereqs(graph, affected_set)
                    print(f"Git diff: {len(changed_packages)} packages changed, "
                          f"{len(affected_set)} affected (including dependents and prerequisites)")
                else:
                    print("Git diff: no package files changed -- nothing to build")
                    affected_set = set()
        else:
            print("Git diff unavailable -- falling back to hash-based cache")

    # --emit-plan mode: serialize the plan and exit without building.
    # This is the fast path for CI detect jobs.
    if args.emit_plan is not None:
        return _emit_plan(args, root, packages, graph, affected_set, ci_toolchains)

    # --detect-languages standalone mode: output language flags and exit.
    if args.detect_languages:
        languages_needed: dict[str, bool] = {"go": True}
        if args.force or affected_set is None:
            for lang in ALL_TOOLCHAINS:
                languages_needed[lang] = True
        else:
            for pkg in packages:
                if pkg.name in affected_set:
                    languages_needed[_toolchain_for_language(pkg.language)] = True
        _output_language_flags(languages_needed)
        return 0

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
    # Set up the progress bar tracker. We only show the progress bar during
    # real builds (not dry runs), and only when the progress bar package is
    # installed. The tracker writes to stderr so it doesn't interfere with
    # structured output on stdout.
    tracker = None
    if not args.dry_run and Tracker is not None:
        tracker = Tracker(total=len(packages), writer=sys.stderr)
        tracker.start()

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
        tracker=tracker,
    )

    if tracker:
        tracker.stop()

    # Step 10: Save cache (as secondary record, not primary mechanism)
    if not args.dry_run:
        cache.save(cache_path)

    # Step 10: Print report
    print_report(results)

    # Step 11: Exit code
    has_failures = any(r.status == "failed" for r in results.values())
    return 1 if has_failures else 0


def _emit_plan(
    args: argparse.Namespace,
    root: Path,
    packages: list[Package],
    graph: DirectedGraph,
    affected_set: set[str] | None,
    ci_toolchains: frozenset[str],
) -> int:
    """Serialize the build plan to JSON and exit.

    This is the ``--emit-plan`` code path. It constructs a ``BuildPlan``
    from the discovery, resolution, and change detection results, writes
    it to the specified path, and exits without building anything.

    If ``--detect-languages`` is also set, language flags are printed to
    stdout after writing the plan.

    Args:
        args: Parsed CLI arguments.
        root: Repository root directory.
        packages: Discovered packages.
        graph: Dependency graph.
        affected_set: Set of affected package names, or None for "all".

    Returns:
        Exit code: 0 on success, 1 on failure.
    """
    # Convert affected_set to a sorted list (or None for "all").
    affected_list: list[str] | None = None
    if affected_set is not None:
        affected_list = sorted(affected_set)

    # Build PackageEntry list from discovered packages.
    package_entries: list[PackageEntry] = []
    for pkg in packages:
        try:
            rel_path = str(pkg.path.relative_to(root)).replace("\\", "/")
        except ValueError:
            rel_path = str(pkg.path).replace("\\", "/")

        package_entries.append(
            PackageEntry(
                name=pkg.name,
                rel_path=rel_path,
                language=pkg.language,
                build_commands=pkg.build_commands,
                is_starlark=pkg.is_starlark,
                declared_srcs=pkg.declared_srcs,
                declared_deps=pkg.declared_deps,
            )
        )

    # Determine which languages are needed. A language is "needed" if
    # at least one affected package uses that language.
    languages_needed: dict[str, bool] = {}
    for pkg in packages:
        lang = _toolchain_for_language(pkg.language)
        if lang not in languages_needed:
            languages_needed[lang] = False
        if affected_set is None or pkg.name in affected_set:
            languages_needed[lang] = True

    # Build the plan.
    bp = BuildPlan(
        schema_version=1,
        diff_base=args.diff_base,
        force=args.force,
        affected_packages=affected_list,
        packages=package_entries,
        dependency_edges=graph.edges(),
        languages_needed=languages_needed,
    )

    try:
        write_plan(bp, args.emit_plan)
        print(f"Build plan written to {args.emit_plan} ({len(packages)} packages)")
    except Exception as exc:
        print(f"Error writing build plan: {exc}", file=sys.stderr)
        return 1

    # If --detect-languages was also set, output language flags.
    if args.detect_languages:
        _output_language_flags(languages_needed)

    return 0


def _run_from_plan(args: argparse.Namespace, root: Path) -> int:
    """Load a build plan from file and run the build.

    This is the ``--plan-file`` code path. It reads a pre-computed plan,
    reconstructs the packages and dependency graph, and runs the build
    without re-doing discovery or change detection.

    On any error (missing file, invalid JSON, unsupported version), it
    falls back to the normal discovery flow by returning a special sentinel
    that triggers re-execution.

    Args:
        args: Parsed CLI arguments.
        root: Repository root directory.

    Returns:
        Exit code: 0 for success, 1 if any builds failed.
    """
    try:
        bp = read_plan(args.plan_file)
    except (FileNotFoundError, ValueError) as exc:
        print(f"Warning: could not load plan ({exc}), falling back to normal flow",
              file=sys.stderr)
        # Fall back to normal flow by clearing the plan-file flag and re-running.
        args.plan_file = None
        return main(_rebuild_argv(args))

    # Reconstruct Package objects from the plan entries.
    packages: list[Package] = []
    for pe in bp.packages:
        # Convert the relative path back to an absolute path.
        pkg_path = root / pe.rel_path

        packages.append(
            Package(
                name=pe.name,
                path=pkg_path,
                build_commands=pe.build_commands,
                language=pe.language,
                is_starlark=pe.is_starlark,
                declared_srcs=pe.declared_srcs,
                declared_deps=pe.declared_deps,
            )
        )

    # Filter by language if specified.
    if args.language != "all":
        packages = [p for p in packages if p.language == args.language]
        if not packages:
            print(f"No {args.language} packages found in plan.", file=sys.stderr)
            return 0

    if args.validate_build_files:
        validation_error = validate_build_contracts(root, packages)
        if validation_error is not None:
            print(
                "BUILD/CI validation failed:\n"
                f"  - {validation_error}\n"
                "Fix the BUILD file or CI workflow so isolated and full-build runs stay correct.",
                file=sys.stderr,
            )
            return 1

    # Reconstruct the dependency graph from the plan's edges.
    graph = DirectedGraph()
    for pkg in packages:
        graph.add_node(pkg.name)
    for from_node, to_node in bp.dependency_edges:
        if graph.has_node(from_node) and graph.has_node(to_node):
            graph.add_edge(from_node, to_node)

    # Reconstruct the affected set.
    affected_set: set[str] | None = None
    if bp.affected_packages is not None:
        affected_set = set(bp.affected_packages)

    print(f"Loaded plan: {len(packages)} packages")

    # From here, the flow is identical to the normal build path:
    # hash, cache, execute, report.

    # Hash all packages
    package_hashes: dict[str, str] = {}
    deps_hashes: dict[str, str] = {}
    from build_tool.hasher import hash_deps, hash_package

    for pkg in packages:
        package_hashes[pkg.name] = hash_package(pkg)
        deps_hashes[pkg.name] = hash_deps(pkg.name, graph, package_hashes)

    # Load cache
    cache_path = args.cache_file
    if not cache_path.is_absolute():
        cache_path = root / cache_path

    cache = BuildCache()
    cache.load(cache_path)

    # Execute builds
    tracker = None
    if not args.dry_run and Tracker is not None:
        tracker = Tracker(total=len(packages), writer=sys.stderr)
        tracker.start()

    results = execute_builds(
        packages=packages,
        graph=graph,
        cache=cache,
        package_hashes=package_hashes,
        deps_hashes=deps_hashes,
        force=bp.force or args.force,
        dry_run=args.dry_run,
        max_jobs=args.jobs,
        affected_set=affected_set,
        tracker=tracker,
    )

    if tracker:
        tracker.stop()

    if not args.dry_run:
        cache.save(cache_path)

    print_report(results)

    has_failures = any(r.status == "failed" for r in results.values())
    return 1 if has_failures else 0


def _rebuild_argv(args: argparse.Namespace) -> list[str]:
    """Reconstruct argv from parsed args (for fallback re-execution).

    When --plan-file fails, we need to re-run main() without the
    --plan-file flag. This helper rebuilds the argument list.
    """
    argv: list[str] = []
    if args.root is not None:
        argv.extend(["--root", str(args.root)])
    if args.force:
        argv.append("--force")
    if args.dry_run:
        argv.append("--dry-run")
    if args.jobs is not None:
        argv.extend(["--jobs", str(args.jobs)])
    if args.language != "all":
        argv.extend(["--language", args.language])
    if args.validate_build_files:
        argv.append("--validate-build-files")
    argv.extend(["--diff-base", args.diff_base])
    argv.extend(["--cache-file", str(args.cache_file)])
    return argv


def _output_language_flags(languages_needed: dict[str, bool]) -> None:
    """Print language flags to stdout and $GITHUB_OUTPUT for CI consumption.

    Output format matches the Go build tool::

        needs_python=true
        needs_go=false
        needs_ruby=true

    Go is always needed because the build tool is written in Go.
    """
    import os

    gh_output_path = os.environ.get("GITHUB_OUTPUT", "")
    gh_file = None
    if gh_output_path:
        try:
            gh_file = open(gh_output_path, "a", encoding="utf-8")  # noqa: SIM115
        except OSError as exc:
            print(f"Warning: could not open $GITHUB_OUTPUT: {exc}", file=sys.stderr)

    # Always output all known toolchains in stable order.
    all_needed = {"go": True}  # Go is always needed
    all_needed.update(languages_needed)

    for lang in ALL_TOOLCHAINS:
        value = all_needed.get(lang, False)
        line = f"needs_{lang}={'true' if value else 'false'}"
        print(line)
        if gh_file is not None:
            gh_file.write(line + "\n")

    if gh_file is not None:
        gh_file.close()


def _compute_languages_needed(
    packages: list[Package],
    affected_set: set[str] | None,
    force: bool,
    ci_toolchains: frozenset[str],
) -> dict[str, bool]:
    languages_needed = {toolchain: False for toolchain in ALL_TOOLCHAINS}
    languages_needed["go"] = True

    if force or affected_set is None:
        for toolchain in ALL_TOOLCHAINS:
            languages_needed[toolchain] = True
        return languages_needed

    for pkg in packages:
        if pkg.name in affected_set:
            languages_needed[_toolchain_for_package_language(pkg.language)] = True

    for toolchain in ci_toolchains:
        languages_needed[toolchain] = True

    return languages_needed


if __name__ == "__main__":
    sys.exit(main())
