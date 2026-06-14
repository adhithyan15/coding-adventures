require "minitest/autorun"
require_relative "../lib/neural_network"

class NeuralNetworkTest < Minitest::Test
  def test_builds_tiny_weighted_graph
    graph = NeuralNetwork.create_neural_graph("tiny")
    NeuralNetwork.add_input(graph, "x0")
    NeuralNetwork.add_input(graph, "x1")
    NeuralNetwork.add_constant(graph, "bias", 1.0)
    NeuralNetwork.add_weighted_sum(graph, "sum", [
      NeuralNetwork.wi("x0", 0.25, "x0_to_sum"),
      NeuralNetwork.wi("x1", 0.75, "x1_to_sum"),
      NeuralNetwork.wi("bias", -1.0, "bias_to_sum")
    ])
    NeuralNetwork.add_activation(graph, "relu", "sum", "relu", {}, "sum_to_relu")
    NeuralNetwork.add_output(graph, "out", "relu", "prediction", {}, "relu_to_out")
    assert_equal 3, graph.incoming_edges("sum").length
    assert_equal "out", graph.topological_sort.last
  end

  def test_xor_network_has_expected_edges
    network = NeuralNetwork.create_xor_network
    assert network.graph.edges.any? { |edge| edge.id == "h_or_to_out" }
  end
end
