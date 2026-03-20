defmodule CodingAdventures.LossFunctionsTest do
  use ExUnit.Case
  alias CodingAdventures.LossFunctions, as: LF

  @y_true [1.0, 0.0]
  @y_pred [0.9, 0.1]

  defp almost_equal?(val, exp) do
    abs(val - exp) <= 1.0e-6
  end

  test "mse calculates correctly" do
    assert almost_equal?(LF.mse(@y_true, @y_pred), 0.010)
  end

  test "mae calculates correctly" do
    assert almost_equal?(LF.mae(@y_true, @y_pred), 0.100)
  end

  test "bce calculates correctly" do
    assert almost_equal?(LF.bce(@y_true, @y_pred), 0.1053605)
  end

  test "cce calculates correctly" do
    assert almost_equal?(LF.cce(@y_true, @y_pred), 0.0526802)
  end

  test "errors on mismatch lengths" do
    assert LF.mse([1.0], @y_pred) == {:error, :length_mismatch}
    assert LF.mae([1.0], @y_pred) == {:error, :length_mismatch}
    assert LF.bce([1.0], @y_pred) == {:error, :length_mismatch}
    assert LF.cce([1.0], @y_pred) == {:error, :length_mismatch}
  end

  test "errors on empty arrays" do
    assert LF.mse([], []) == {:error, :length_mismatch}
    assert LF.mae([], []) == {:error, :length_mismatch}
    assert LF.bce([], []) == {:error, :length_mismatch}
    assert LF.cce([], []) == {:error, :length_mismatch}
  end

  test "identical slices return 0" do
    assert almost_equal?(LF.mse([1.0, 0.5], [1.0, 0.5]), 0.0)
    assert almost_equal?(LF.mae([1.0, 0.5], [1.0, 0.5]), 0.0)
  end

  test "mse_derivative" do
    y_true = [1.0, 0.0]
    y_pred = [0.8, 0.2]
    res = LF.mse_derivative(y_true, y_pred)
    assert almost_equal?(Enum.at(res, 0), -0.2)
    assert almost_equal?(Enum.at(res, 1), 0.2)
  end

  test "mae_derivative" do
    y_true = [1.0, 0.0, 0.5]
    y_pred = [0.8, 0.2, 0.5]
    res = LF.mae_derivative(y_true, y_pred)
    assert almost_equal?(Enum.at(res, 0), -1.0 / 3.0)
    assert almost_equal?(Enum.at(res, 1), 1.0 / 3.0)
    assert almost_equal?(Enum.at(res, 2), 0.0)
  end

  test "bce_derivative" do
    y_true = [1.0, 0.0]
    y_pred = [0.8, 0.2]
    res = LF.bce_derivative(y_true, y_pred)
    assert almost_equal?(Enum.at(res, 0), -0.625)
    assert almost_equal?(Enum.at(res, 1), 0.625)
  end

  test "cce_derivative" do
    y_true = [1.0, 0.0]
    y_pred = [0.8, 0.2]
    res = LF.cce_derivative(y_true, y_pred)
    assert almost_equal?(Enum.at(res, 0), -0.625)
    assert almost_equal?(Enum.at(res, 1), 0.0)
  end
end
