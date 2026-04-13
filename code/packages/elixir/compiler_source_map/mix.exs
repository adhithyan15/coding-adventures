defmodule CodingAdventures.CompilerSourceMap.MixProject do
  use Mix.Project

  # ──────────────────────────────────────────────────────────────────────────────
  # Project definition
  #
  # This is the Elixir port of the Go `compiler-source-map` package.
  # It provides the source-map chain that flows as a sidecar through every
  # stage of the AOT compiler pipeline:
  #
  #   Segment 1: SourceToAst       — source text position  → AST node ID
  #   Segment 2: AstToIr           — AST node ID           → IR instruction IDs
  #   Segment 3: IrToIr            — IR ID → optimised IR IDs (one per pass)
  #   Segment 4: IrToMachineCode   — IR ID → machine code byte offset + length
  #
  # No dependencies — source maps are consumed by frontends and backends
  # but don't depend on either.
  # ──────────────────────────────────────────────────────────────────────────────

  def project do
    [
      app: :coding_adventures_compiler_source_map,
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
    # No dependencies — source maps are the foundation layer.
    []
  end
end
