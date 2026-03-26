defmodule GrammarToolsCli.MixProject do
  use Mix.Project

  def project do
    [
      app: :grammar_tools_cli,
      version: "0.1.0",
      elixir: "~> 1.19",
      start_permanent: Mix.env() == :prod,
      deps: deps()
    ]
  end

  # Run "mix help compile.app" to learn about applications.
  def application do
    [
      extra_applications: [:logger]
    ]
  end

  # Run "mix help deps" to learn about dependencies.
  defp deps do
    [
      {:coding_adventures_grammar_tools, path: "../../../packages/elixir/grammar_tools"},
      {:coding_adventures_cli_builder, path: "../../../packages/elixir/cli_builder"}
    ]
  end
end
