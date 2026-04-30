require "minitest/autorun"
$LOAD_PATH.unshift File.expand_path("../../neural-network/lib", __dir__)
require "neural_network"
require_relative "../lib/neural_graph_vm"

class NeuralGraphVMTest < Minitest::Test
  def tiny_graph
    graph = NeuralNetwork.create_neural_graph("tiny")
    NeuralNetwork.add_input(graph, "x0")
    NeuralNetwork.add_input(graph, "x1")
    NeuralNetwork.add_constant(graph, "bias", 1.0)
    NeuralNetwork.add_weighted_sum(graph, "sum", [NeuralNetwork.wi("x0", 0.25, "x0_to_sum"), NeuralNetwork.wi("x1", 0.75, "x1_to_sum"), NeuralNetwork.wi("bias", -1.0, "bias_to_sum")])
    NeuralNetwork.add_activation(graph, "relu", "sum", "relu", {}, "sum_to_relu")
    NeuralNetwork.add_output(graph, "out", "relu", "prediction", {}, "relu_to_out")
    graph
  end

  def test_runs_tiny_weighted_sum
    bytecode = NeuralGraphVM.compile_neural_graph_to_bytecode(tiny_graph)
    assert_in_delta 6.0, NeuralGraphVM.run_neural_bytecode_forward(bytecode, { "x0" => 4.0, "x1" => 8.0 }).fetch("prediction"), 1e-9
  end

  def test_runs_xor
    bytecode = NeuralGraphVM.compile_neural_network_to_bytecode(NeuralNetwork.create_xor_network)
    [[0.0, 0.0, 0.0], [0.0, 1.0, 1.0], [1.0, 0.0, 1.0], [1.0, 1.0, 0.0]].each do |x0, x1, expected|
      prediction = NeuralGraphVM.run_neural_bytecode_forward(bytecode, { "x0" => x0, "x1" => x1 }).fetch("prediction")
      expected == 1.0 ? assert(prediction > 0.99) : assert(prediction < 0.01)
    end
  end
end
