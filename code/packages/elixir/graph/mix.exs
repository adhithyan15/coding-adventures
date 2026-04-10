defmodule CodingAdventures.Graph.MixProject do
  use Mix.Project

  def project do
    [
      app: :coding_adventures_graph,
      version: "0.1.0",
      elixir: "~> 1.14",
      start_permanent: Mix.env() == :prod,
      deps: [],
      test_coverage: [summary: [threshold: 75]]
    ]
  end

  def application do
    [extra_applications: [:logger]]
  end
end
