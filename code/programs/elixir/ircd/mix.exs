defmodule Ircd.MixProject do
  use Mix.Project

  @moduledoc false

  def project do
    [
      app: :coding_adventures_ircd,
      version: "0.1.0",
      elixir: "~> 1.14",
      escript: [main_module: CodingAdventures.Ircd],
      start_permanent: Mix.env() == :prod,
      deps: deps(),
      test_coverage: [summary: [threshold: 80]]
    ]
  end

  def application do
    [extra_applications: [:logger]]
  end

  defp deps do
    [
      {:coding_adventures_irc_proto, path: "../../../packages/elixir/irc_proto"},
      {:coding_adventures_irc_framing, path: "../../../packages/elixir/irc_framing"},
      {:coding_adventures_irc_server, path: "../../../packages/elixir/irc_server"},
      {:coding_adventures_irc_net_stdlib, path: "../../../packages/elixir/irc_net_stdlib"}
    ]
  end
end
