defmodule ConduitHello.MixProject do
  use Mix.Project

  def project do
    [
      app: :conduit_hello,
      version: "0.1.0",
      elixir: "~> 1.14",
      start_permanent: Mix.env() == :prod,
      deps: deps(),
      escript: [main_module: ConduitHello]
    ]
  end

  def application, do: [extra_applications: [:logger]]

  defp deps do
    [
      {:coding_adventures_conduit, path: "../../../packages/elixir/conduit"}
    ]
  end
end
