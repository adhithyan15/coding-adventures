defmodule CodingAdventures.Microbenchmark.MixProject do
  use Mix.Project

  def project do
    [
      app: :coding_adventures_microbenchmark,
      version: "0.1.0",
      elixir: "~> 1.18.4",
      start_permanent: Mix.env() == :prod,
      deps: deps(),
      escript: [main_module: CodingAdventures.ArrayMicrobenchmark],
      test_coverage: [summary: [threshold: 80]]
    ]
  end

  def application, do: [extra_applications: [:logger]]
  defp deps, do: [{:coding_adventures_rank_statistics, path: "../rank_statistics"}]
end
