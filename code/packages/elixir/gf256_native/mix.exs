defmodule CodingAdventures.GF256Native.MixProject do
  use Mix.Project

  def project do
    [
      app: :coding_adventures_gf256_native,
      version: "0.1.0",
      elixir: "~> 1.14",
      start_permanent: Mix.env() == :prod,
      compilers: Mix.compilers() ++ [:make],
      make_targets: ["all"],
      make_clean: ["clean"],
      make_cwd: "native/gf256_native",
      deps: deps(),
      test_coverage: [summary: [threshold: 80]]
    ]
  end

  def application do
    [extra_applications: [:logger]]
  end

  defp deps do
    [{:elixir_make, "~> 0.7", runtime: false}]
  end
end
