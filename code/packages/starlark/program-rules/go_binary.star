# ============================================================================
# go_binary.star — Build rule for Go executable programs
# ============================================================================
#
# Go binaries are programs with a main package and a main() function. When
# you run "go build" in a directory with package main, Go produces a single
# static binary — no runtime dependencies, no virtual environment, just one
# executable file.
#
# This makes Go particularly well-suited for build tools, CLI programs, and
# infrastructure. In fact, this monorepo's own build tool is a Go binary
# (code/programs/go/build-tool/).
#
# GO BINARY vs GO LIBRARY
# -----------------------
# In Go, the distinction is simple:
#   - Library: package <something> (importable, no main function)
#   - Binary:  package main with func main() (executable)
#
# The go_binary rule doesn't need an entry_point parameter because Go's
# convention is rigid: the main() function in the main package IS the entry
# point. There's no choice to make.
#
# EXAMPLE BUILD FILE
# ------------------
#   load("//rules:go_binary.star", "go_binary")
#
#   go_binary(
#       name = "build-tool",
#       srcs = ["**/*.go"],
#       deps = ["go/directed-graph"],
#   )
#
# ============================================================================

def go_binary(name, srcs = [], deps = []):
    # Register a Go binary (executable program) target.
    #
    # Go binaries compile to a single static executable. The build tool runs:
    #     go vet ./...            — static analysis
    #     go build -o <name> .    — compile to binary
    #     go test ./... -v -cover — run tests (if any)
    #
    # The compiled binary is placed in the package directory with the target
    # name. For a target named "build-tool", the output is ./build-tool.
    #
    # Args:
    #     name: The program name, matching the directory under
    #           code/programs/go/. For example, "build-tool" maps to
    #           code/programs/go/build-tool/.
    #
    #           This is also the name of the output binary.
    #
    #     srcs: File paths or glob patterns for change detection.
    #           Typical: ["**/*.go", "go.mod", "go.sum"]
    #
    #           Tracking go.mod and go.sum ensures the binary is rebuilt
    #           when dependencies change, even if no .go files changed.
    #
    #     deps: Dependencies as "language/package-name" strings.
    #           Examples:
    #               ["go/directed-graph"]
    #               ["go/starlark-vm", "go/directed-graph"]
    #
    #           Go binaries often import library packages. These deps ensure
    #           the build tool compiles libraries before the binary that
    #           uses them (though Go's module system handles the actual
    #           linking).
    #
    # Note: No entry_point parameter needed. In Go, the entry point is always
    # the main() function in package main. The Go compiler enforces this.
    return {
        # "go_binary" triggers binary-specific build logic:
        #   - go build to produce the executable
        #   - go vet for linting
        #   - go test if test files exist
        #   - The output binary name matches the target name
        "rule": "go_binary",
        "name": name,
        "srcs": srcs,
        "deps": deps,
        "commands": [
            {"type": "cmd", "program": "go", "args": ["build", "./..."]},
            {"type": "cmd", "program": "go", "args": ["test", "./...", "-v", "-cover"]},
            {"type": "cmd", "program": "go", "args": ["vet", "./..."]},
        ],
    }
