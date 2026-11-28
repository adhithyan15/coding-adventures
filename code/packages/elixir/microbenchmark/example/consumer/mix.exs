defmodule MicrobenchmarkConsumer.MixProject do
  use Mix.Project

  def project,
    do: [app: :microbenchmark_consumer_smoke, version: "0.1.0", elixir: "~> 1.18.4", deps: deps()]

  def application, do: [extra_applications: [:logger]]

  defp deps,
    do: [
      {:coding_adventures_microbenchmark, path: "../.."},
      {:coding_adventures_rank_statistics, path: "../../../rank_statistics"}
    ]
end
