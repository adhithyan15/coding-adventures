require "minitest/autorun"
require_relative "../lib/loss_functions"

class TestLossFunctions < Minitest::Test
  def setup
    @y_true = [1.0, 0.0]
    @y_pred = [0.9, 0.1]
  end

  def test_mse
    assert_in_delta 0.010, LossFunctions.mse(@y_true, @y_pred), 1e-6
  end

  def test_mae
    assert_in_delta 0.100, LossFunctions.mae(@y_true, @y_pred), 1e-6
  end

  def test_bce
    assert_in_delta 0.1053605, LossFunctions.bce(@y_true, @y_pred), 1e-6
  end

  def test_cce
    assert_in_delta 0.0526802, LossFunctions.cce(@y_true, @y_pred), 1e-6
  end

  def test_length_mismatch_error
    assert_raises(LossFunctions::LengthMismatchError) do
      LossFunctions.mse([1.0], @y_pred)
    end
    assert_raises(LossFunctions::LengthMismatchError) do
      LossFunctions.mae([1.0], @y_pred)
    end
    assert_raises(LossFunctions::LengthMismatchError) do
      LossFunctions.bce([1.0], @y_pred)
    end
    assert_raises(LossFunctions::LengthMismatchError) do
      LossFunctions.cce([1.0], @y_pred)
    end
  end

  def test_empty_arrays_error
    assert_raises(LossFunctions::LengthMismatchError) do
      LossFunctions.mse([], [])
    end
    assert_raises(LossFunctions::LengthMismatchError) do
      LossFunctions.mae([], [])
    end
    assert_raises(LossFunctions::LengthMismatchError) do
      LossFunctions.bce([], [])
    end
    assert_raises(LossFunctions::LengthMismatchError) do
      LossFunctions.cce([], [])
    end
  end

  def test_identical_arrays
    identical_true = [1.0, 0.5]
    identical_pred = [1.0, 0.5]
    assert_in_delta 0.0, LossFunctions.mse(identical_true, identical_pred), 1e-6
    assert_in_delta 0.0, LossFunctions.mae(identical_true, identical_pred), 1e-6
  end
end
