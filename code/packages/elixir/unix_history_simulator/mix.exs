defmodule UnixHistorySimulator.MixProject do
  use Mix.Project

  def project,
    do: [
      app: :unix_history_simulator,
      version: "0.2.0",
      elixir: "~> 1.15",
      start_permanent: Mix.env() == :prod,
      deps: []
    ]

  def application, do: [extra_applications: [:logger]]
end
