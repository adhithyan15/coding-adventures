# ============================================================================
# go_library.star — Build rule for Go library packages
# ============================================================================
#
# Go has a distinctive build model compared to Python or Ruby. The Go toolchain
# (go build, go test) handles compilation, linking, and dependency resolution
# natively — there's no separate "install dependencies" step like pip or
# bundler. This means the go_library rule is simpler than py_library in some
# ways, but the dependency declaration is still important for the monorepo
# build tool's change detection and build ordering.
#
# HOW GO MODULES WORK IN THIS MONOREPO
# -------------------------------------
# Each Go package has its own go.mod file (it's a Go module). Dependencies
# between packages in the monorepo use "replace" directives in go.mod to
# point to sibling directories:
#
#   require github.com/example/logic-gates v0.0.0
#   replace github.com/example/logic-gates => ../logic-gates
#
# The go_library rule doesn't manage these — go.mod handles it. What the rule
# DOES manage is telling the build tool about these relationships so it can:
#   1. Build dependencies before dependents (topological order)
#   2. Propagate changes (if logic-gates changes, rebuild everything that
#      depends on it)
#   3. Run independent packages in parallel
#
# EXAMPLE BUILD FILE
# ------------------
#   load("code/packages/starlark/library-rules/go_library.star", "go_library")
#
#   _targets = [
#       go_library(
#           name = "logic-gates",
#           srcs = ["**/*.go"],
#           deps = ["go/transistors"],
#       ),
#   ]
#
# ============================================================================

def go_library(name, srcs = [], deps = []):
    # Register a Go library target for the build system.
    #
    # Go libraries are simpler than Python libraries because Go has a single,
    # built-in test runner (go test) and a single way to manage dependencies
    # (go.mod). There's no test_runner parameter to choose between frameworks.
    #
    # The build tool will run these commands for a go_library target:
    #     go vet ./...           — static analysis (catches common mistakes)
    #     go test ./... -v -cover — run tests with verbose output and coverage
    #
    # Args:
    #     name: The package name, matching the directory under
    #           code/packages/go/. For example, "directed-graph" corresponds
    #           to code/packages/go/directed-graph/.
    #
    #     srcs: File paths or glob patterns for change detection.
    #           For Go packages, this is typically ["**/*.go"] to track all
    #           Go source files, or ["**/*.go", "go.mod", "go.sum"] to also
    #           rebuild when dependencies change.
    #
    #           If empty, the build tool tracks all files in the package
    #           directory.
    #
    #     deps: Dependencies as "language/package-name" strings.
    #           Examples:
    #               ["go/transistors"]
    #               ["go/logic-gates", "go/arithmetic"]
    #
    #           These must mirror the replace directives in go.mod. If your
    #           go.mod has a replace pointing to ../transistors, you should
    #           have "go/transistors" in deps so the build tool knows about
    #           the relationship.
    return {
        "rule": "go_library",
        "name": name,
        "srcs": srcs,
        "deps": deps,
        "commands": [
            {"type": "cmd", "program": "go", "args": ["build", "./..."]},
            {"type": "cmd", "program": "go", "args": ["test", "./...", "-v", "-cover"]},
            {"type": "cmd", "program": "go", "args": ["vet", "./..."]},
        ],
    }
