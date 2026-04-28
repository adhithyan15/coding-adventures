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

  test "hidden layer teaching examples run one training step" do
    cases = [
      {"XNOR", @inputs, [[1.0], [0.0], [0.0], [1.0]], 3},
      {"absolute value", [[-1.0], [-0.5], [0.0], [0.5], [1.0]], [[1.0], [0.5], [0.0], [0.5], [1.0]], 4},
      {"piecewise pricing", [[0.1], [0.3], [0.5], [0.7], [0.9]], [[0.12], [0.25], [0.55], [0.88], [0.88]], 4},
      {"circle classifier", [[0.0, 0.0], [0.5, 0.0], [1.0, 1.0], [-0.5, 0.5], [-1.0, 0.0]], [[1.0], [1.0], [0.0], [1.0], [0.0]], 5},
      {"two moons", [[1.0, 0.0], [0.0, 0.5], [0.5, 0.85], [0.5, -0.35], [-1.0, 0.0], [2.0, 0.5]], [[0.0], [1.0], [0.0], [1.0], [0.0], [1.0]], 5},
      {"interaction features", [[0.2, 0.25, 0.0], [0.6, 0.5, 1.0], [1.0, 0.75, 1.0], [1.0, 1.0, 0.0]], [[0.08], [0.72], [0.96], [0.76]], 5}
    ]

    for {name, inputs, targets, hidden_count} <- cases do
      step = TwoLayerNetwork.train_one_epoch(inputs, targets, sample_parameters(length(hd(inputs)), hidden_count), 0.4)

      assert step.loss >= 0.0, name
      assert length(step.input_to_hidden_weight_gradients) == length(hd(inputs))
      assert length(step.hidden_to_output_weight_gradients) == hidden_count
    end
  end

  defp sample_parameters(input_count, hidden_count) do
    %{
      input_to_hidden_weights:
        for feature <- 0..(input_count - 1) do
          for hidden <- 0..(hidden_count - 1), do: 0.17 * (feature + 1) - 0.11 * (hidden + 1)
        end,
      hidden_biases: for(hidden <- 0..(hidden_count - 1), do: 0.05 * (hidden - 1)),
      hidden_to_output_weights: for(hidden <- 0..(hidden_count - 1), do: [0.13 * (hidden + 1) - 0.25]),
      output_biases: [0.02]
    }
  end
end
