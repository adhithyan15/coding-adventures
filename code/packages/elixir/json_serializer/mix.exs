defmodule CodingAdventures.JsonSerializer.MixProject do
  use Mix.Project

  def project do
    [
      app: :coding_adventures_json_serializer,
      version: "0.1.0",
      elixir: "~> 1.14",
      start_permanent: Mix.env() == :prod,
      deps: deps(),
      test_coverage: [tool: ExCoveralls]
    ]
  end

  def application do
    [
      extra_applications: [:logger]
    ]
  end

  defp deps do
    [
      # --- Direct dependency ---
      {:coding_adventures_json_value, path: "../json_value"},

      # --- Transitive dependencies (json_value -> json_parser -> ...) ---
      {:coding_adventures_json_parser, path: "../json_parser"},
      {:coding_adventures_grammar_tools, path: "../grammar_tools"},
      {:coding_adventures_directed_graph, path: "../directed_graph"},
      {:coding_adventures_lexer, path: "../lexer"},
      {:coding_adventures_parser, path: "../parser"},
      {:coding_adventures_json_lexer, path: "../json_lexer"}
    ]
  end
end
