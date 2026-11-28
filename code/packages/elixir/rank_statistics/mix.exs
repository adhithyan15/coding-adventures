defmodule CodingAdventures.RankStatistics.MixProject do
  use Mix.Project

  def project do
    [
      app: :coding_adventures_rank_statistics,
      version: "0.1.0",
      elixir: "~> 1.18.4",
      start_permanent: Mix.env() == :prod,
      deps: [],
      test_coverage: [summary: [threshold: 95]]
    ]
  end

  def application, do: [extra_applications: [:logger]]
end
