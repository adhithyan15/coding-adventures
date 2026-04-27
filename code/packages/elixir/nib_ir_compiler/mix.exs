defmodule CodingAdventures.NibIrCompiler.MixProject do
  use Mix.Project

  def project do
    [
      app: :coding_adventures_nib_ir_compiler,
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
      {:coding_adventures_nib_type_checker, path: "../nib_type_checker"},
      {:coding_adventures_nib_parser, path: "../nib_parser"}
    ]
  end
end
