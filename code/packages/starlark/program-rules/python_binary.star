# ============================================================================
# python_binary.star — Build rule for Python executable programs
# ============================================================================
#
# LIBRARY vs BINARY: WHAT'S THE DIFFERENCE?
# ------------------------------------------
# A library is code that other code imports. It doesn't run on its own — it
# provides functions, classes, and modules for other packages to use.
#
# A binary (or program) is code that runs directly. It has an entry point —
# a "main" file that you execute: python main.py
#
# In Bazel terminology:
#   - py_library = a package of reusable Python code
#   - py_binary  = a runnable Python program
#
# The key differences in build behavior:
#   1. A binary has an entry_point (the file you execute)
#   2. A binary typically depends on libraries (via deps)
#   3. Testing a binary often means running it and checking output, rather
#      than importing it and calling functions
#   4. A binary might be "installed" by creating a CLI entry point
#
# WHERE BINARIES LIVE
# -------------------
# Libraries go under code/packages/python/<name>/
# Binaries go under code/programs/python/<name>/
#
# This mirrors the Bazel convention of separating reusable code (libraries)
# from runnable programs (binaries).
#
# EXAMPLE BUILD FILE
# ------------------
#   load("//rules:python_binary.star", "py_binary")
#
#   py_binary(
#       name = "build-tool",
#       srcs = ["src/**/*.py"],
#       deps = ["python/directed-graph", "python/starlark-vm"],
#       entry_point = "main.py",
#   )
#
# ============================================================================

_targets = []


def py_binary(name, srcs = [], deps = [], entry_point = "main.py"):
    """Register a Python binary (executable program) target.

    Unlike py_library, a binary has an entry point that can be executed
    directly. The build tool will:
        1. Install dependencies (same as py_library)
        2. Run tests if they exist
        3. Verify the entry point is executable: python <entry_point>

    Args:
        name: The program name, matching the directory under
              code/programs/python/. For example, "build-tool" maps to
              code/programs/python/build-tool/.

        srcs: File paths or glob patterns for change detection.
              Same as py_library. Typical: ["src/**/*.py", "*.py"]

        deps: Dependencies as "language/package-name" strings.
              A binary typically depends on library packages:
                  ["python/directed-graph", "python/starlark-vm"]

              The build tool ensures all deps are built before the binary.

        entry_point: The Python file to execute when running this program.
              Defaults to "main.py" — the conventional entry point.

              Examples:
                  "main.py"          — simple script in package root
                  "src/cli.py"       — CLI entry point in src directory
                  "src/__main__.py"  — Python package entry point

              The build tool uses this to verify the program starts
              successfully (e.g., python main.py --help should exit 0).
    """
    install_cmd = {"type": "cmd", "program": "uv", "args": ["pip", "install", "--system", "-e", ".[dev]"]}
    test_cmd = {"type": "cmd", "program": "python", "args": ["-m", "pytest", "--cov", "--cov-report=term-missing"]}

    _targets.append({
        "rule": "py_binary",
        "name": name,
        "srcs": srcs,
        "deps": deps,
        "entry_point": entry_point,
        "commands": [install_cmd, test_cmd],
    })
