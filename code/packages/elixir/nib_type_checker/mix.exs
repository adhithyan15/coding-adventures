defmodule CodingAdventures.NibTypeChecker.MixProject do
  use Mix.Project

  def project do
    [
      app: :coding_adventures_nib_type_checker,
      version: "0.1.0",
      elixir: "~> 1.14",
      start_permanent: Mix.env() == :prod,
      deps: deps(),
      test_coverage: [
        summary: [threshold: 80]
      ]
    ]
  end

  def application do
    [
      extra_applications: [:logger]
    ]
  end

  defp deps do
    [
      {:coding_adventures_nib_parser, path: "../nib_parser"},
      {:coding_adventures_type_checker_protocol, path: "../type_checker_protocol"}
    ]
  end
end
