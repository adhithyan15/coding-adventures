defmodule Pwd.MixProject do
  use Mix.Project

  def project do
    [
      app: :pwd,
      version: "1.0.0",
      elixir: "~> 1.14",
      start_permanent: Mix.env() == :prod,
      escript: [main_module: Pwd.CLI],
      deps: deps()
    ]
  end

  def application do
    [extra_applications: [:logger]]
  end

  defp deps do
    [
      {:coding_adventures_cli_builder, path: "../../../packages/elixir/cli_builder"}
    ]
  end
end
