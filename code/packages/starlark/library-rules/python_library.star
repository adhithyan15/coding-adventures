# ============================================================================
# python_library.star — Build rule for Python library packages
# ============================================================================
#
# WHAT IS A BUILD RULE?
# ---------------------
# In Bazel-like build systems, a "rule" is a blueprint that describes how to
# build a particular kind of target. Think of it like a recipe template:
#
#   - A py_library rule says: "Here's a Python library. Here are its source
#     files, here are its dependencies, and here's how to test it."
#
# Rules don't DO the building themselves — they DECLARE what needs to happen.
# The build tool reads these declarations and figures out the right order to
# build everything, which targets have changed and need rebuilding, and which
# can be skipped.
#
# WHY DECLARATIVE?
# ----------------
# Traditional Makefiles are imperative: "run this shell command, then that one."
# Declarative rules flip it: "here's what I am, figure out how to build me."
#
# This matters because:
#   1. The build tool can detect changes via file hashing and skip unchanged
#      targets (incremental builds).
#   2. Independent targets can be built in parallel — the tool knows the
#      dependency graph and won't start building X until all of X's deps
#      are ready.
#   3. The same rule definition works across different environments (local
#      dev, CI, different OSes) because the build tool adapts the commands.
#
# HOW THIS FILE IS USED
# ---------------------
# Each Python library package has a BUILD file that loads this rule:
#
#   load("//rules:python_library.star", "py_library")
#
#   py_library(
#       name = "logic-gates",
#       srcs = ["src/**/*.py"],
#       deps = ["python/transistors"],
#       test_runner = "pytest",
#   )
#
# The build tool's Starlark interpreter executes the BUILD file, which calls
# py_library(), which registers a target in the _targets list. After all BUILD
# files are processed, the build tool has a complete picture of every target
# and its dependencies — a directed acyclic graph (DAG).
#
# ============================================================================

# _targets is a module-level list that accumulates all registered targets.
# Each call to py_library() appends one entry. After the BUILD file finishes
# executing, the build tool reads this list to discover what targets exist.
#
# Why a list and not a dict? Because a single BUILD file might define multiple
# targets (e.g., a library and a test suite), and we want to preserve the
# order they were declared in.
_targets = []


def py_library(name, srcs = [], deps = [], test_runner = "pytest"):
    """Register a Python library target for the build system.

    This is the core rule for Python library packages in the monorepo. When
    the build tool encounters a py_library() call in a BUILD file, it records
    the target's metadata so it can later:

      1. Determine if the target needs rebuilding (by checking if any file
         matching the srcs patterns has changed since the last build).
      2. Build dependencies first (by walking the deps graph).
      3. Run the appropriate test command (based on test_runner).

    Args:
        name: The package name, matching the directory name under
              code/packages/python/. For example, "logic-gates" corresponds
              to code/packages/python/logic-gates/.

              Naming convention: lowercase with hyphens, like npm packages.
              This name is used as the unique identifier for the target in
              the dependency graph.

        srcs: A list of file paths or glob patterns that comprise this
              package's source code. The build tool uses these patterns for
              change detection — if any matching file has been modified
              (compared to the diff base, usually origin/main), this target
              is considered "dirty" and will be rebuilt.

              Examples:
                  ["src/**/*.py"]           — all Python files under src/
                  ["src/**/*.py", "*.toml"] — also track config changes

              If empty (the default), the build tool falls back to tracking
              all files in the package directory.

        deps: A list of dependency strings in "language/package-name" format.
              These tell the build tool which other targets must be built
              BEFORE this one.

              Examples:
                  ["python/transistors"]           — depends on one package
                  ["python/logic-gates",           — depends on two packages
                   "python/arithmetic"]

              The "language/" prefix is important because this monorepo
              contains the same logical package implemented in multiple
              languages (Python, Go, Ruby, etc.). The prefix disambiguates
              which implementation to depend on.

              The build tool topologically sorts all targets by their deps
              to determine build order. Circular dependencies are an error.

        test_runner: Which test framework to use. Currently supported:

              "pytest"   — (default) Runs: pytest tests/ -v
                           The standard Python test framework. Supports
                           fixtures, parametrize, and rich assertion output.

              "unittest" — Runs: python -m unittest discover tests/
                           Python's built-in test framework. No extra
                           dependencies needed, but less feature-rich.

              The build tool uses this to generate the correct test command
              when building the target.
    """
    _targets.append({
        # "rule" identifies what kind of target this is. The build tool
        # dispatches to different build logic based on this field:
        #   - "py_library" → install with uv pip, run pytest/unittest
        #   - "go_library" → run go test ./...
        #   - "rs_library" → run cargo test
        # This string must match what the build tool expects.
        "rule": "py_library",

        # "name" is the unique identifier for this target within the build
        # graph. Combined with the language prefix, it forms a fully
        # qualified target like "python/logic-gates".
        "name": name,

        # "srcs" tells the change detection system which files to watch.
        # In git-diff mode, the build tool computes:
        #   git diff --name-only origin/main...HEAD
        # and checks if any changed file matches these patterns. If so,
        # this target (and all targets that depend on it) are rebuilt.
        "srcs": srcs,

        # "deps" defines edges in the dependency graph. If A depends on B,
        # there's a directed edge from B to A. The build tool uses this
        # graph to:
        #   1. Determine build order (topological sort)
        #   2. Propagate "dirty" status (if B changed, A must rebuild too)
        #   3. Parallelize (independent targets build simultaneously)
        "deps": deps,

        # "test_runner" controls which test command is generated.
        # This is Python-specific — other rules (go_library, etc.) don't
        # need this because Go/Rust have a single built-in test runner.
        "test_runner": test_runner,
    })
