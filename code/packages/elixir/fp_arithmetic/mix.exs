defmodule CodingAdventures.FpArithmetic.MixProject do
  use Mix.Project

  def project do
    [
      app: :coding_adventures_fp_arithmetic,
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
      {:coding_adventures_logic_gates, path: "../logic_gates"},
      {:coding_adventures_arithmetic, path: "../arithmetic"},
      {:coding_adventures_clock, path: "../clock"}
    ]
  end
end
