defmodule CodingAdventures.BloomFilter.MixProject do
  use Mix.Project

  def project do
    [
      app: :coding_adventures_bloom_filter,
      version: "0.1.0",
      elixir: "~> 1.14",
      start_permanent: Mix.env() == :prod,
      deps: [],
      test_coverage: [summary: [threshold: 80]]
    ]
  end

  def application, do: [extra_applications: [:logger]]
end
