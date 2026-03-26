defmodule CodingAdventures.Intel4004Simulator.MixProject do
  use Mix.Project

  def project do
    [
      app: :coding_adventures_intel4004_simulator,
      version: "0.2.0",
      elixir: "~> 1.14",
      start_permanent: Mix.env() == :prod,
      deps: deps(),
      test_coverage: [summary: [threshold: 80], ignore_modules: [~r/.*Tokens$/, ~r/.*Grammar$/]]
    ]
  end

  def application, do: [extra_applications: [:logger]]
  defp deps, do: []
end
