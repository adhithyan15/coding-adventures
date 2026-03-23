# ============================================================================
# elixir_library.star — Build rule for Elixir library packages
# ============================================================================
#
# Elixir packages use Mix, Elixir's built-in build tool. Each package has:
#
#   my-package/
#     lib/
#       my_package.ex           # main module
#       my_package/
#         implementation.ex     # submodules
#     test/
#       my_package_test.exs     # ExUnit tests
#       test_helper.exs         # test setup (starts ExUnit)
#     mix.exs                   # project configuration and dependencies
#
# ELIXIR'S BUILD MODEL
# --------------------
# Mix is both a build tool and a dependency manager (like Cargo for Rust).
# It compiles .ex files to .beam bytecode (for the BEAM virtual machine),
# manages dependencies, and runs tests via ExUnit.
#
# Dependencies between monorepo packages use path references in mix.exs:
#
#   defp deps do
#     [{:transistors, path: "../transistors"}]
#   end
#
# IMPORTANT LESSONS LEARNED (see lessons.md):
#   - Elixir reserved words (after, rescue, catch, else, end, fn, do, when,
#     cond, try, receive) CANNOT be used as variable names. When porting code
#     from other languages, rename them (e.g., after -> rest).
#   - `if` blocks must capture their result: compiler = if ... do ... end
#   - GenericVM function calls need fresh execution context — save/restore
#     caller state around function calls
#
# WHY NO test_runner PARAMETER?
# ----------------------------
# Elixir has exactly one test framework: ExUnit, which is built into the
# language. Every Elixir project uses it. There's no choice to make.
# (This is similar to Go's testing package and Rust's #[test] attribute.)
#
# EXAMPLE BUILD FILE
# ------------------
#   load("//rules:elixir_library.star", "elixir_library")
#
#   elixir_library(
#       name = "logic-gates",
#       srcs = ["lib/**/*.ex"],
#       deps = ["elixir/transistors"],
#   )
#
# ============================================================================

_targets = []


def elixir_library(name, srcs = [], deps = []):
    """Register an Elixir library target for the build system.

    Elixir libraries use Mix for building and ExUnit for testing. The build
    tool will run:
        mix deps.get          — fetch dependencies
        mix compile --warnings-as-errors — compile with strict warnings
        mix test --cover      — run tests with built-in coverage

    Elixir's coverage tool is built into OTP (the Erlang runtime), so no
    extra package is needed — just pass --cover to mix test.

    Args:
        name: The package name, matching the directory under
              code/packages/elixir/. For example, "logic-gates" maps to
              code/packages/elixir/logic-gates/.

              Elixir module names use CamelCase (LogicGates), but the
              Mix project name and directory use snake_case/hyphens.

        srcs: File paths or glob patterns for change detection.
              Typical patterns:
                  ["lib/**/*.ex"]                         — source only
                  ["lib/**/*.ex", "test/**/*.exs"]        — source and tests
                  ["lib/**/*.ex", "mix.exs"]              — source and config

              Note: Elixir source files use .ex extension, test files use
              .exs (the "s" stands for "script" — these files are interpreted
              rather than compiled to .beam).

        deps: Dependencies as "language/package-name" strings.
              These must match the path references in mix.exs.
              Examples:
                  ["elixir/transistors"]
                  ["elixir/logic-gates", "elixir/arithmetic"]

              Mix handles transitive dependency resolution (like Cargo),
              so you technically only need direct deps. But listing
              transitive deps explicitly gives the build tool better
              change propagation information.
    """
    _targets.append({
        # "elixir_library" triggers Elixir-specific build logic:
        #   - mix deps.get for dependency resolution
        #   - mix compile for compilation (Elixir -> BEAM bytecode)
        #   - mix test for testing via ExUnit
        #   - --cover for built-in coverage measurement
        "rule": "elixir_library",
        "name": name,
        "srcs": srcs,
        "deps": deps,
        "commands": [
            {"type": "cmd", "program": "mix", "args": ["deps.get"]},
            {"type": "cmd", "program": "mix", "args": ["test", "--cover"]},
        ],
    })
