defmodule CodingAdventures.IrcServer.MixProject do
  use Mix.Project

  @moduledoc false

  def project do
    [
      app: :coding_adventures_irc_server,
      version: "0.1.0",
      elixir: "~> 1.14",
      start_permanent: Mix.env() == :prod,
      deps: deps(),
      test_coverage: [summary: [threshold: 80]]
    ]
  end

  def application do
    [extra_applications: [:logger]]
  end

  defp deps do
    [{:coding_adventures_irc_proto, path: "../irc_proto"}]
  end
end
