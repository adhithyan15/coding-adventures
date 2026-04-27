defmodule GradientDescentTest do
  use ExUnit.Case
  test "sgd" do
    res = GradientDescent.sgd([1.0, 2.0], [0.1, 0.2], 0.5)
    assert_in_delta Enum.at(res, 0), 0.95, 0.0001
    assert_in_delta Enum.at(res, 1), 1.9, 0.0001
  end
end
