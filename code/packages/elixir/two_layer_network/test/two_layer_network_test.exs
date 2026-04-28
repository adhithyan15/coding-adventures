defmodule CodingAdventures.TwoLayerNetworkTest do
  use ExUnit.Case

  alias CodingAdventures.TwoLayerNetwork

  @inputs [[0.0, 0.0], [0.0, 1.0], [1.0, 0.0], [1.0, 1.0]]
  @targets [[0.0], [1.0], [1.0], [0.0]]

  test "forward pass exposes hidden activations" do
    pass = TwoLayerNetwork.forward(@inputs, TwoLayerNetwork.xor_warm_start_parameters())

    assert length(pass.hidden_activations) == 4
    assert length(hd(pass.hidden_activations)) == 2
    assert pass.predictions |> Enum.at(1) |> hd() > 0.7
    assert pass.predictions |> hd() |> hd() < 0.3
  end

  test "training step exposes both layer gradients" do
    step = TwoLayerNetwork.train_one_epoch(@inputs, @targets, TwoLayerNetwork.xor_warm_start_parameters(), 0.5)

    assert length(step.input_to_hidden_weight_gradients) == 2
    assert length(hd(step.input_to_hidden_weight_gradients)) == 2
    assert length(step.hidden_to_output_weight_gradients) == 2
    assert length(hd(step.hidden_to_output_weight_gradients)) == 1
  end
end
