defmodule CodingAdventures.GrammarTools.MixProject do
  use Mix.Project

  def project do
    [
      app: :coding_adventures_grammar_tools,
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
    []
  end
end
