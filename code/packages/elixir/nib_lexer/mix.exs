defmodule CodingAdventures.NibLexer.MixProject do
  use Mix.Project

  def project do
    [
      app: :coding_adventures_nib_lexer,
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
      {:coding_adventures_grammar_tools, path: "../grammar_tools"},
      {:coding_adventures_directed_graph, path: "../directed_graph"},
      {:coding_adventures_lexer, path: "../lexer"}
    ]
  end
end
