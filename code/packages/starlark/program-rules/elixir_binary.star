# ============================================================================
# elixir_binary.star — Build rule for Elixir executable programs
# ============================================================================
#
# Elixir programs run on the BEAM virtual machine (Erlang's runtime). Unlike
# Go or Rust, Elixir doesn't compile to a standalone native binary by default.
# There are two ways to distribute Elixir programs:
#
#   1. Mix project: run via "mix run" or "mix <task>" (development)
#   2. Escript: compile to a self-contained executable that requires Erlang/OTP
#   3. Release: a complete, deployable package with the Erlang runtime
#
# In this monorepo, Elixir programs use Mix for development and testing.
# The entry_point is the main module/file that starts the application.
#
# ELIXIR'S EXECUTION MODEL
# ------------------------
# Elixir programs often start an OTP application (a supervision tree of
# processes). But for simple CLI programs, a plain module with a main/0
# function works:
#
#   defmodule MyApp.CLI do
#     def main do
#       IO.puts("Hello from Elixir!")
#     end
#   end
#
# The entry_point parameter points to the file containing this main module.
#
# EXAMPLE BUILD FILE
# ------------------
#   load("//rules:elixir_binary.star", "elixir_binary")
#
#   elixir_binary(
#       name = "starlark-repl",
#       srcs = ["lib/**/*.ex"],
#       deps = ["elixir/starlark-vm", "elixir/parser"],
#       entry_point = "lib/main.ex",
#   )
#
# ============================================================================

def elixir_binary(name, srcs = [], deps = [], entry_point = "lib/main.ex"):
    # Register an Elixir binary (executable program) target.
    #
    # Elixir binaries run on the BEAM VM via Mix. The build tool will:
    #     mix deps.get                       — fetch dependencies
    #     mix compile --warnings-as-errors   — compile with strict warnings
    #     mix test --cover                   — run tests if they exist
    #     mix run <entry_point>              — verify the program starts
    #
    # Args:
    #     name: The program name, matching the directory under
    #           code/programs/elixir/. For example, "starlark-repl" maps to
    #           code/programs/elixir/starlark-repl/.
    #
    #     srcs: File paths or glob patterns for change detection.
    #           Typical: ["lib/**/*.ex", "mix.exs"]
    #
    #           Track mix.exs because dependency changes should trigger
    #           a rebuild.
    #
    #     deps: Dependencies as "language/package-name" strings.
    #           Examples:
    #               ["elixir/starlark-vm"]
    #               ["elixir/parser", "elixir/lexer"]
    #
    #           Mix handles transitive dependency resolution (like Cargo),
    #           so direct deps are sufficient. But listing transitive deps
    #           explicitly helps the build tool.
    #
    #     entry_point: The Elixir file to execute when running this program.
    #           Defaults to "lib/main.ex".
    #
    #           Examples:
    #               "lib/main.ex"      — simple main module
    #               "lib/cli.ex"       — CLI entry point
    #               "lib/app.ex"       — OTP application entry point
    #
    #           Elixir doesn't have a rigid convention like Go's main() or
    #           Rust's fn main(). The entry point is configurable because
    #           Elixir programs vary: some are CLI tools, some are OTP
    #           applications, some are escripts.
    return {
        # "elixir_binary" triggers Elixir binary-specific build logic:
        #   - mix deps.get for dependencies
        #   - mix compile for compilation
        #   - mix test for testing via ExUnit
        #   - Entry point validation via mix run
        "rule": "elixir_binary",
        "name": name,
        "srcs": srcs,
        "deps": deps,
        "entry_point": entry_point,
        "commands": [
            {"type": "cmd", "program": "mix", "args": ["deps.get"]},
            {"type": "cmd", "program": "mix", "args": ["test", "--cover"]},
        ],
    }
