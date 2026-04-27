defmodule CodingAdventures.VerilogParser.MixProject do
  use Mix.Project

  def project do
    [
      app: :coding_adventures_verilog_parser,
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
      {:coding_adventures_parser, path: "../parser"},
      {:coding_adventures_verilog_lexer, path: "../verilog_lexer"}
    ]
  end
end
