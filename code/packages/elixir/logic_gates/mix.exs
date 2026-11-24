defmodule CodingAdventures.LogicGates.MixProject do
  use Mix.Project

  def project do
    [
      app: :coding_adventures_logic_gates,
      version: "0.2.0",
      consolidate_protocols: Mix.env() != :test,
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
      {:coding_adventures_transistors, path: "../transistors"}
    ]
  end
end
