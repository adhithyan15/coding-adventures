defmodule CodingAdventures.EcmascriptEs5Lexer.MixProject do
  use Mix.Project

  def project do
    [
      app: :coding_adventures_ecmascript_es5_lexer,
      version: "0.1.0",
      elixir: "~> 1.14",
      start_permanent: Mix.env() == :prod,
      deps: deps(),
      test_coverage: [
        summary: [threshold: 70]
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
      {:coding_adventures_lexer, path: "../lexer"}
    ]
  end
end
