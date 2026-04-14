defmodule ActivationFunctionsTest do
  use ExUnit.Case

  test "sigmoid" do
    assert_in_delta ActivationFunctions.sigmoid(0), 0.5, 0.0001
  end

  test "relu" do
    assert ActivationFunctions.relu(5) == 5.0
    assert ActivationFunctions.relu(-5) == 0.0
  end

  test "tanh" do
    assert_in_delta ActivationFunctions.tanh(0), 0.0, 0.0001
  end
end
