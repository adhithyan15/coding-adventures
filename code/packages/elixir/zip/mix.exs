defmodule CodingAdventures.Zip.MixProject do
  use Mix.Project

  def project do
    [
      app: :coding_adventures_zip,
      version: "0.2.0",
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
      {:coding_adventures_lzss, path: "../lzss"},
      {:jason, "~> 1.4", only: :test}
    ]
  end
end
