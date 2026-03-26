defmodule CodingAdventures.Cowsay.MixProject do
  use Mix.Project

  def project do
    [
      app: :cowsay,
      version: "0.1.0",
      elixir: "~> 1.14",
      start_permanent: Mix.env() == :prod,
      test_coverage: [summary: [threshold: 80], ignore_modules: [~r/.*Tokens$/, ~r/.*Grammar$/]],
      deps: deps()
    ]
  end

  defp deps do
    [
      {:coding_adventures_cli_builder, path: "../../../packages/elixir/cli_builder"},
      {:jason, "~> 1.4"}
    ]
  end
end
