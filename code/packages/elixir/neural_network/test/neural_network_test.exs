defmodule NeuralNetworkTest do
  use ExUnit.Case

  test "builds a tiny weighted graph" do
    graph =
      NeuralNetwork.create_neural_graph("tiny")
      |> NeuralNetwork.add_input("x0")
      |> NeuralNetwork.add_input("x1")
      |> NeuralNetwork.add_constant("bias", 1.0)
      |> NeuralNetwork.add_weighted_sum("sum", [NeuralNetwork.wi("x0", 0.25, "x0_to_sum"), NeuralNetwork.wi("x1", 0.75, "x1_to_sum"), NeuralNetwork.wi("bias", -1.0, "bias_to_sum")])
      |> then(fn graph -> NeuralNetwork.add_activation(graph, "relu", "sum", "relu", %{}, "sum_to_relu") |> elem(0) end)
      |> then(fn graph -> NeuralNetwork.add_output(graph, "out", "relu", "prediction", %{}, "relu_to_out") |> elem(0) end)

    assert length(NeuralNetwork.Graph.incoming_edges(graph, "sum")) == 3
    assert {:ok, order} = NeuralNetwork.Graph.topological_sort(graph)
    assert List.last(order) == "out"
  end

  test "xor network has hidden output edge" do
    network = NeuralNetwork.create_xor_network()
    assert Enum.any?(network.graph.edges, &(&1.id == "h_or_to_out"))
  end
end
