defmodule CodingAdventures.CliBuilder.MixProject do
  use Mix.Project

  @moduledoc false

  def project do
    [
      app: :coding_adventures_cli_builder,
      version: "1.1.0",
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
      {:coding_adventures_state_machine, path: "../state_machine"},
      {:coding_adventures_directed_graph, path: "../directed_graph"},
      {:jason, "~> 1.4"}
    ]
  end
end
