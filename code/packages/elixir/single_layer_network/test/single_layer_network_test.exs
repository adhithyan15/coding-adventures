defmodule CodingAdventures.SingleLayerNetworkTest do
  use ExUnit.Case

  alias CodingAdventures.SingleLayerNetwork

  defp near(actual, expected), do: assert(abs(actual - expected) <= 1.0e-6)

  test "one epoch exposes matrix gradients" do
    step =
      SingleLayerNetwork.train_one_epoch_with_matrices(
        [[1.0, 2.0]],
        [[3.0, 5.0]],
        [[0.0, 0.0], [0.0, 0.0]],
        [0.0, 0.0],
        0.1
      )

    assert step.predictions == [[0.0, 0.0]]
    assert step.errors == [[-3.0, -5.0]]
    assert step.weight_gradients == [[-3.0, -5.0], [-6.0, -10.0]]
    near(Enum.at(Enum.at(step.next_weights, 0), 0), 0.3)
    near(Enum.at(Enum.at(step.next_weights, 1), 1), 1.0)
  end

  test "fit learns m inputs to n outputs" do
    {model, history} =
      SingleLayerNetwork.new(3, 2)
      |> SingleLayerNetwork.fit(
        [[0.0, 0.0, 1.0], [1.0, 2.0, 1.0], [2.0, 1.0, 1.0]],
        [[1.0, -1.0], [3.0, 2.0], [4.0, 1.0]],
        0.05,
        500
      )

    assert List.last(history).loss < hd(history).loss
    assert length(hd(SingleLayerNetwork.predict(model, [[1.0, 1.0, 1.0]]))) == 2
  end
end
