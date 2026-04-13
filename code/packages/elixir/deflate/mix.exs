defmodule CodingAdventures.Deflate.MixProject do
  use Mix.Project

  def project do
    [
      app: :coding_adventures_deflate,
      version: "0.1.0",
      elixir: "~> 1.14",
      start_permanent: Mix.env() == :prod,
      deps: deps(),
      test_coverage: [summary: [threshold: 80]]
    ]
  end

  def application do
    [extra_applications: [:logger]]
  end

  defp deps do
    [
      {:coding_adventures_huffman_tree, path: "../../elixir/huffman_tree"},
      {:coding_adventures_lzss, path: "../../elixir/lzss"}
    ]
  end
end
