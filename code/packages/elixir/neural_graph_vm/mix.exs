defmodule NeuralGraphVM.MixProject do
  use Mix.Project
  def project do
    [app: :neural_graph_vm, version: "0.1.0", elixir: "~> 1.14", deps: deps()]
  end
  defp deps do
    [{:neural_network, path: "../neural_network"}]
  end
end
