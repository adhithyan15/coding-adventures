defmodule CodingAdventures.AlgolParser.MixProject do
  use Mix.Project

  def project do
    [
      app: :coding_adventures_algol_parser,
      version: "0.1.0",
      elixir: "~> 1.14",
      start_permanent: Mix.env() == :prod,
      deps: deps()
    ]
  end

  def application do
    [
      extra_applications: [:logger]
    ]
  end

  defp deps do
    [
      {:coding_adventures_grammar_tools, path: "../grammar_tools"},
      {:coding_adventures_directed_graph, path: "../directed_graph"},
      {:coding_adventures_lexer, path: "../lexer"},
      {:coding_adventures_parser, path: "../parser"},
      {:coding_adventures_algol_lexer, path: "../algol_lexer"}
    ]
  end
end
