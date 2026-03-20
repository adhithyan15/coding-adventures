require 'minitest/autorun'
require_relative '../lib/gradient_descent'

class TestGradientDescent < Minitest::Test
  def test_sgd
    weights = [1.0, -0.5, 2.0]
    gradients = [0.1, -0.2, 0.0]
    lr = 0.1

    res = GradientDescent.sgd(weights, gradients, lr)
    
    assert_in_delta 0.99, res[0], 1e-6
    assert_in_delta(-0.48, res[1], 1e-6)
    assert_in_delta 2.0, res[2], 1e-6
  end

  def test_errors
    assert_raises(LengthMismatchError) do
      GradientDescent.sgd([1.0], [], 0.1)
    end
    assert_raises(LengthMismatchError) do
      GradientDescent.sgd([], [], 0.1)
    end
  end
end
