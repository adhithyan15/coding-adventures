defmodule GrammarTools.MixProject do
  use Mix.Project

  def project do
    [
      app: :grammar_tools_program,
      version: "1.0.0",
      elixir: "~> 1.14",
      start_permanent: Mix.env() == :prod,
      escript: [main_module: GrammarTools.CLI],
      deps: deps(),
      test_coverage: [tool: ExCoveralls],
      preferred_cli_env: [coveralls: :test]
    ]
  end

  def application do
    [extra_applications: [:logger]]
  end

  defp deps do
    [
      {:coding_adventures_grammar_tools,
       path: "../../../packages/elixir/grammar_tools"},
      {:coding_adventures_cli_builder,
       path: "../../../packages/elixir/cli_builder"},
      {:jason, "~> 1.4"},
      {:excoveralls, "~> 0.18", only: :test}
    ]
  end
end
