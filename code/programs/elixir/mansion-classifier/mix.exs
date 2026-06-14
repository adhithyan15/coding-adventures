defmodule MansionClassifier.MixProject do
  use Mix.Project

  def project do
    [
      app: :mansion_classifier,
      version: "0.1.0",
      elixir: "~> 1.15",
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
    [
      {:matrix, path: "../../../packages/elixir/matrix"},
      {:coding_adventures_loss_functions, path: "../../../packages/elixir/loss_functions"},
      {:activation_functions, path: "../../../packages/elixir/activation_functions"},
      {:perceptron, path: "../../../packages/elixir/perceptron"},
      {:neural_network, path: "../../../packages/elixir/neural_network"},
      {:neural_graph_vm, path: "../../../packages/elixir/neural_graph_vm"}
    ]
  end
end
