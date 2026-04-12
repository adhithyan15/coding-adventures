defmodule Ls00.MixProject do
  use Mix.Project

  def project do
    [
      app: :coding_adventures_ls00,
      version: "0.1.0",
      elixir: "~> 1.15",
      deps: deps(),
      test_coverage: [summary: [threshold: 80]]
    ]
  end

  def application do
    [extra_applications: [:logger]]
  end

  defp deps do
    [{:coding_adventures_json_rpc, path: "../json_rpc"}]
  end
end
