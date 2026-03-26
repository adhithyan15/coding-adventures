defmodule CodingAdventures.BranchPredictor.MixProject do
  use Mix.Project

  def project do
    [
      app: :coding_adventures_branch_predictor,
      version: "0.1.0",
      elixir: "~> 1.14",
      start_permanent: Mix.env() == :prod,
      deps: deps(),
      test_coverage: [summary: [threshold: 80], ignore_modules: [~r/.*Tokens$/, ~r/.*Grammar$/]]
    ]
  end

  def application do
    [
      extra_applications: [:logger]
    ]
  end

  defp deps do
    [
      {:coding_adventures_directed_graph, path: "../directed_graph"},
      {:coding_adventures_state_machine, path: "../state_machine"}
    ]
  end
end
