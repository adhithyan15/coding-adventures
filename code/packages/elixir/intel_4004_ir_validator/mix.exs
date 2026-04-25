defmodule CodingAdventures.Intel4004IrValidator.MixProject do
  use Mix.Project

  def project do
    [
      app: :coding_adventures_intel_4004_ir_validator,
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
      {:coding_adventures_compiler_ir, path: "../compiler_ir"}
    ]
  end
end
