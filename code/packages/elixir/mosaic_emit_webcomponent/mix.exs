defmodule CodingAdventures.MosaicEmitWebcomponent.MixProject do
  use Mix.Project

  def project do
    [
      app: :coding_adventures_mosaic_emit_webcomponent,
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
      {:coding_adventures_mosaic_vm, path: "../mosaic_vm"},
      {:coding_adventures_mosaic_analyzer, path: "../mosaic_analyzer"},
      {:coding_adventures_mosaic_parser, path: "../mosaic_parser"},
      {:coding_adventures_mosaic_lexer, path: "../mosaic_lexer"},
      {:coding_adventures_grammar_tools, path: "../grammar_tools"},
      {:coding_adventures_lexer, path: "../lexer"},
      {:coding_adventures_directed_graph, path: "../directed_graph"},
      {:coding_adventures_parser, path: "../parser"},
      {:coding_adventures_state_machine, path: "../state_machine"}
    ]
  end
end
