defmodule CodingAdventuresMarkovChain.MixProject do
  use Mix.Project

  def project do
    [
      app: :coding_adventures_markov_chain,
      version: "0.1.0",
      elixir: "~> 1.15",
      start_permanent: Mix.env() == :prod,
      deps: deps(),
      test_coverage: [tool: ExCoveralls]
    ]
  end

  def cli do
    [preferred_envs: [coveralls: :test]]
  end

  def application do
    [
      extra_applications: [:logger]
    ]
  end

  defp deps do
    [
      {:coding_adventures_directed_graph, path: "../../elixir/directed_graph"},
      {:excoveralls, "~> 0.18", only: :test}
    ]
  end
end
