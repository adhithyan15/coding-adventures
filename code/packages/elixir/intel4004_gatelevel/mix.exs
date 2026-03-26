defmodule CodingAdventures.Intel4004GateLevel.MixProject do
  use Mix.Project

  def project do
    [
      app: :coding_adventures_intel4004_gatelevel,
      version: "0.1.0",
      elixir: "~> 1.14",
      start_permanent: Mix.env() == :prod,
      deps: deps(),
      test_coverage: [summary: [threshold: 80], ignore_modules: [~r/.*Tokens$/, ~r/.*Grammar$/]]
    ]
  end

  def application, do: [extra_applications: [:logger]]

  defp deps do
    [
      {:coding_adventures_logic_gates, path: "../logic_gates"},
      {:coding_adventures_arithmetic, path: "../arithmetic"}
    ]
  end
end
