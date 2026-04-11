defmodule CodingAdventures.CorrelationVector.MixProject do
  use Mix.Project

  def project do
    [
      app: :coding_adventures_correlation_vector,
      version: "0.1.0",
      elixir: "~> 1.14",
      start_permanent: Mix.env() == :prod,
      deps: deps(),
      test_coverage: [summary: [threshold: 95]]
    ]
  end

  def application do
    [
      extra_applications: [:logger]
    ]
  end

  defp deps do
    [
      # SHA-256 for ID generation (no :crypto needed — pure Elixir impl)
      {:coding_adventures_sha256, path: "../sha256"},

      # JSON serialization: direct dep
      {:coding_adventures_json_serializer, path: "../json_serializer"},

      # JSON serializer transitive deps (leaf-to-root order per lessons.md):
      {:coding_adventures_json_value, path: "../json_value"},
      {:coding_adventures_json_parser, path: "../json_parser"},
      {:coding_adventures_grammar_tools, path: "../grammar_tools"},
      {:coding_adventures_directed_graph, path: "../directed_graph"},
      {:coding_adventures_lexer, path: "../lexer"},
      {:coding_adventures_parser, path: "../parser"},
      {:coding_adventures_json_lexer, path: "../json_lexer"}
    ]
  end
end
