defmodule CodingAdventures.CompilerIr.MixProject do
  use Mix.Project

  # ──────────────────────────────────────────────────────────────────────────────
  # Project definition
  #
  # This is the Elixir port of the Go `compiler-ir` package. It provides the
  # intermediate representation (IR) type system for the AOT native compiler
  # pipeline:
  #
  #   - IrOp atom constants for all opcodes
  #   - IrRegister, IrImmediate, IrLabel operand structs
  #   - IrInstruction struct (opcode + operands + unique ID)
  #   - IrDataDecl struct (.data section declarations)
  #   - IrProgram struct (complete compiled unit)
  #   - IDGenerator for monotonic unique instruction IDs
  #   - IR printer (IrProgram → text)
  #   - IR parser (text → IrProgram, for roundtrip tests)
  #
  # No dependencies — this is the foundation that everything else builds on.
  # ──────────────────────────────────────────────────────────────────────────────

  def project do
    [
      app: :coding_adventures_compiler_ir,
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
    # No dependencies — compiler_ir is the base layer of the pipeline.
    []
  end
end
