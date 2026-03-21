defmodule CodingAdventures.GradientDescentTest do
  use ExUnit.Case
  alias CodingAdventures.GradientDescent, as: GD

  defp almost_equal?(val, exp) do
    abs(val - exp) <= 1.0e-6
  end

  test "sgd calculates correctly" do
    weights = [1.0, -0.5, 2.0]
    gradients = [0.1, -0.2, 0.0]
    lr = 0.1

    res = GD.sgd(weights, gradients, lr)
    assert almost_equal?(Enum.at(res, 0), 0.99)
    assert almost_equal?(Enum.at(res, 1), -0.48)
    assert almost_equal?(Enum.at(res, 2), 2.0)
  end

  test "errors on mismatch lengths" do
    assert GD.sgd([1.0], [], 0.1) == {:error, :length_mismatch}
  end

  test "errors on empty arrays" do
    assert GD.sgd([], [], 0.1) == {:error, :length_mismatch}
  end
end
