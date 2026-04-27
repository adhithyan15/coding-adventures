defmodule CodingAdventures.CssParser.MixProject do
  use Mix.Project

  def project do
    [
      app: :coding_adventures_css_parser,
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
      {:coding_adventures_parser, path: "../parser"},
      {:coding_adventures_css_lexer, path: "../css_lexer"}
    ]
  end
end
