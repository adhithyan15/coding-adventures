defmodule CodingAdventures.SingleLayerNetwork.MixProject do
  use Mix.Project

  def project do
    [
      app: :coding_adventures_single_layer_network,
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
