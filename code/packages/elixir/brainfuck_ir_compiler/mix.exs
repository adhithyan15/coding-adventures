defmodule CodingAdventures.BrainfuckIrCompiler.MixProject do
  use Mix.Project

  # ──────────────────────────────────────────────────────────────────────────────
  # Project definition
  #
  # This is the Elixir port of the Go `brainfuck-ir-compiler` package.
  # It is the Brainfuck-specific frontend of the AOT compiler pipeline:
  #
  #   Brainfuck AST  →  IrProgram + SourceMapChain
  #
  # The compiler knows Brainfuck semantics (tape, cells, pointer, loops,
  # I/O) and translates them into target-independent IR instructions.
  # It does NOT know about RISC-V, ARM, ELF, or any specific backend.
  # ──────────────────────────────────────────────────────────────────────────────

  def project do
    [
      app: :coding_adventures_brainfuck_ir_compiler,
      version: "0.1.0",
      elixir: "~> 1.14",
      start_permanent: Mix.env() == :prod,
      deps: deps(),
      test_coverage: [
        summary: [threshold: 80]
      ]
    ]
  end

  def application do
    [
      extra_applications: [:logger]
    ]
  end

  defp deps do
    [
      {:coding_adventures_compiler_ir, path: "../compiler_ir"},
      {:coding_adventures_compiler_source_map, path: "../compiler_source_map"},
      {:coding_adventures_brainfuck, path: "../brainfuck"},
      {:coding_adventures_parser, path: "../parser"},
      {:coding_adventures_lexer, path: "../lexer"}
    ]
  end
end
