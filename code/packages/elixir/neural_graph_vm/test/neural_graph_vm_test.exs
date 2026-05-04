defmodule NeuralGraphVMTest do
  use ExUnit.Case

  defp tiny_graph do
    NeuralNetwork.create_neural_graph("tiny")
    |> NeuralNetwork.add_input("x0")
    |> NeuralNetwork.add_input("x1")
    |> NeuralNetwork.add_constant("bias", 1.0)
    |> NeuralNetwork.add_weighted_sum("sum", [NeuralNetwork.wi("x0", 0.25, "x0_to_sum"), NeuralNetwork.wi("x1", 0.75, "x1_to_sum"), NeuralNetwork.wi("bias", -1.0, "bias_to_sum")])
    |> then(fn graph -> NeuralNetwork.add_activation(graph, "relu", "sum", "relu", %{}, "sum_to_relu") |> elem(0) end)
    |> then(fn graph -> NeuralNetwork.add_output(graph, "out", "relu", "prediction", %{}, "relu_to_out") |> elem(0) end)
  end

  test "runs tiny weighted sum" do
    outputs = tiny_graph() |> NeuralGraphVM.compile_neural_graph_to_bytecode() |> NeuralGraphVM.run_neural_bytecode_forward(%{"x0" => 4.0, "x1" => 8.0})
    assert_in_delta outputs["prediction"], 6.0, 1.0e-9
  end

  test "runs xor" do
    bytecode = NeuralNetwork.create_xor_network() |> NeuralGraphVM.compile_neural_network_to_bytecode()
    for {x0, x1, expected} <- [{0.0, 0.0, 0.0}, {0.0, 1.0, 1.0}, {1.0, 0.0, 1.0}, {1.0, 1.0, 0.0}] do
      prediction = NeuralGraphVM.run_neural_bytecode_forward(bytecode, %{"x0" => x0, "x1" => x1})["prediction"]
      if expected == 1.0, do: assert(prediction > 0.99), else: assert(prediction < 0.01)
    end
  end
end
