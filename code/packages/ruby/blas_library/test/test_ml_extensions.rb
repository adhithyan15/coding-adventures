# frozen_string_literal: true

require_relative "test_helper"

# ================================================================
# Tests for ML extensions: activation functions, normalization,
# convolution, and attention.
# ================================================================

class TestMLActivations < Minitest::Test
  include CodingAdventures::BlasLibrary

  def setup
    @blas = Backends::CpuBlas.new
  end

  # --- ReLU ---

  def test_relu_positive
    x = Matrix.new(data: [1.0, 2.0, 3.0, 4.0], rows: 2, cols: 2)
    result = @blas.relu(x)
    assert_equal [1.0, 2.0, 3.0, 4.0], result.data
  end

  def test_relu_negative
    x = Matrix.new(data: [-1.0, -2.0, -3.0, -4.0], rows: 2, cols: 2)
    result = @blas.relu(x)
    assert_equal [0.0, 0.0, 0.0, 0.0], result.data
  end

  def test_relu_mixed
    x = Matrix.new(data: [-1.0, 2.0, -3.0, 4.0], rows: 2, cols: 2)
    result = @blas.relu(x)
    assert_equal [0.0, 2.0, 0.0, 4.0], result.data
  end

  def test_relu_zero
    x = Matrix.new(data: [0.0, 0.0], rows: 1, cols: 2)
    result = @blas.relu(x)
    assert_equal [0.0, 0.0], result.data
  end

  # --- GELU ---

  def test_gelu_zero
    x = Matrix.new(data: [0.0], rows: 1, cols: 1)
    result = @blas.gelu(x)
    assert_in_delta 0.0, result.data[0], 1e-5
  end

  def test_gelu_positive
    x = Matrix.new(data: [1.0], rows: 1, cols: 1)
    result = @blas.gelu(x)
    assert_in_delta 0.8413, result.data[0], 0.01
  end

  def test_gelu_negative
    x = Matrix.new(data: [-1.0], rows: 1, cols: 1)
    result = @blas.gelu(x)
    assert_in_delta(-0.1587, result.data[0], 0.01)
  end

  def test_gelu_large_positive
    x = Matrix.new(data: [3.0], rows: 1, cols: 1)
    result = @blas.gelu(x)
    assert_in_delta 3.0, result.data[0], 0.01
  end

  # --- Sigmoid ---

  def test_sigmoid_zero
    x = Matrix.new(data: [0.0], rows: 1, cols: 1)
    result = @blas.sigmoid(x)
    assert_in_delta 0.5, result.data[0], 1e-6
  end

  def test_sigmoid_positive
    x = Matrix.new(data: [100.0], rows: 1, cols: 1)
    result = @blas.sigmoid(x)
    assert_in_delta 1.0, result.data[0], 1e-6
  end

  def test_sigmoid_negative
    x = Matrix.new(data: [-100.0], rows: 1, cols: 1)
    result = @blas.sigmoid(x)
    assert_in_delta 0.0, result.data[0], 1e-6
  end

  def test_sigmoid_range
    x = Matrix.new(data: [-2.0, -1.0, 0.0, 1.0, 2.0], rows: 1, cols: 5)
    result = @blas.sigmoid(x)
    result.data.each do |v|
      assert v > 0.0 && v < 1.0, "Sigmoid output #{v} not in (0, 1)"
    end
  end

  # --- Tanh ---

  def test_tanh_zero
    x = Matrix.new(data: [0.0], rows: 1, cols: 1)
    result = @blas.tanh_activation(x)
    assert_in_delta 0.0, result.data[0], 1e-6
  end

  def test_tanh_positive
    x = Matrix.new(data: [100.0], rows: 1, cols: 1)
    result = @blas.tanh_activation(x)
    assert_in_delta 1.0, result.data[0], 1e-6
  end

  def test_tanh_negative
    x = Matrix.new(data: [-100.0], rows: 1, cols: 1)
    result = @blas.tanh_activation(x)
    assert_in_delta(-1.0, result.data[0], 1e-6)
  end

  def test_tanh_range
    x = Matrix.new(data: [-2.0, -1.0, 0.0, 1.0, 2.0], rows: 1, cols: 5)
    result = @blas.tanh_activation(x)
    result.data.each do |v|
      assert v > -1.0 && v < 1.0, "Tanh output #{v} not in (-1, 1)"
    end
  end
end

class TestMLSoftmax < Minitest::Test
  include CodingAdventures::BlasLibrary

  def setup
    @blas = Backends::CpuBlas.new
  end

  def test_softmax_row
    x = Matrix.new(data: [1.0, 2.0, 3.0, 1.0, 2.0, 3.0], rows: 2, cols: 3)
    result = @blas.softmax(x, axis: -1)
    # Each row should sum to 1
    row1_sum = result.data[0..2].sum
    row2_sum = result.data[3..5].sum
    assert_in_delta 1.0, row1_sum, 1e-6
    assert_in_delta 1.0, row2_sum, 1e-6
  end

  def test_softmax_column
    x = Matrix.new(data: [1.0, 2.0, 3.0, 4.0], rows: 2, cols: 2)
    result = @blas.softmax(x, axis: 0)
    # Each column should sum to 1
    col1_sum = result.data[0] + result.data[2]
    col2_sum = result.data[1] + result.data[3]
    assert_in_delta 1.0, col1_sum, 1e-6
    assert_in_delta 1.0, col2_sum, 1e-6
  end

  def test_softmax_all_equal
    x = Matrix.new(data: [1.0, 1.0, 1.0], rows: 1, cols: 3)
    result = @blas.softmax(x)
    result.data.each do |v|
      assert_in_delta 1.0 / 3.0, v, 1e-6
    end
  end

  def test_softmax_all_positive
    x = Matrix.new(data: [1.0, 2.0, 3.0], rows: 1, cols: 3)
    result = @blas.softmax(x)
    result.data.each { |v| assert v > 0.0 }
    assert_in_delta 1.0, result.data.sum, 1e-6
  end

  def test_softmax_single_element
    x = Matrix.new(data: [5.0], rows: 1, cols: 1)
    result = @blas.softmax(x)
    assert_in_delta 1.0, result.data[0], 1e-6
  end

  def test_softmax_numerical_stability
    x = Matrix.new(data: [1000.0, 1001.0, 1002.0], rows: 1, cols: 3)
    result = @blas.softmax(x)
    assert_in_delta 1.0, result.data.sum, 1e-6
    result.data.each { |v| refute v.nan? }
  end
end

class TestMLNormalization < Minitest::Test
  include CodingAdventures::BlasLibrary

  def setup
    @blas = Backends::CpuBlas.new
  end

  # --- Layer Norm ---

  def test_layer_norm_basic
    x = Matrix.new(data: [1.0, 2.0, 3.0, 4.0, 5.0, 6.0], rows: 2, cols: 3)
    gamma = Vector.new(data: [1.0, 1.0, 1.0], size: 3)
    beta = Vector.new(data: [0.0, 0.0, 0.0], size: 3)
    result = @blas.layer_norm(x, gamma, beta)
    # Each row should be normalized: mean=0, var~1
    row1 = result.data[0..2]
    assert_in_delta 0.0, row1.sum / 3.0, 1e-5
  end

  def test_layer_norm_with_scale_shift
    x = Matrix.new(data: [1.0, 3.0], rows: 1, cols: 2)
    gamma = Vector.new(data: [2.0, 2.0], size: 2)
    beta = Vector.new(data: [1.0, 1.0], size: 2)
    result = @blas.layer_norm(x, gamma, beta)
    # After norm: x_hat = [-1, 1], scaled: [-2+1, 2+1] = [-1, 3]
    assert_in_delta(-1.0, result.data[0], 1e-5)
    assert_in_delta 3.0, result.data[1], 1e-5
  end

  def test_layer_norm_gamma_mismatch
    x = Matrix.new(data: [1.0, 2.0], rows: 1, cols: 2)
    gamma = Vector.new(data: [1.0], size: 1)
    beta = Vector.new(data: [0.0, 0.0], size: 2)
    assert_raises(ArgumentError) { @blas.layer_norm(x, gamma, beta) }
  end

  def test_layer_norm_beta_mismatch
    x = Matrix.new(data: [1.0, 2.0], rows: 1, cols: 2)
    gamma = Vector.new(data: [1.0, 1.0], size: 2)
    beta = Vector.new(data: [0.0], size: 1)
    assert_raises(ArgumentError) { @blas.layer_norm(x, gamma, beta) }
  end

  # --- Batch Norm ---

  def test_batch_norm_training
    x = Matrix.new(data: [1.0, 2.0, 3.0, 4.0], rows: 2, cols: 2)
    gamma = Vector.new(data: [1.0, 1.0], size: 2)
    beta = Vector.new(data: [0.0, 0.0], size: 2)
    rm = Vector.new(data: [0.0, 0.0], size: 2)
    rv = Vector.new(data: [1.0, 1.0], size: 2)
    result = @blas.batch_norm(x, gamma, beta, rm, rv, training: true)
    # Each column should be normalized
    col1 = [result.data[0], result.data[2]]
    assert_in_delta 0.0, col1.sum / 2.0, 1e-5
  end

  def test_batch_norm_inference
    x = Matrix.new(data: [1.0, 2.0, 3.0, 4.0], rows: 2, cols: 2)
    gamma = Vector.new(data: [1.0, 1.0], size: 2)
    beta = Vector.new(data: [0.0, 0.0], size: 2)
    rm = Vector.new(data: [2.0, 3.0], size: 2)
    rv = Vector.new(data: [1.0, 1.0], size: 2)
    result = @blas.batch_norm(x, gamma, beta, rm, rv, training: false)
    # (1.0 - 2.0) / sqrt(1.0 + 1e-5) ~ -1.0
    assert_in_delta(-1.0, result.data[0], 1e-4)
  end

  def test_batch_norm_gamma_mismatch
    x = Matrix.new(data: [1.0, 2.0], rows: 1, cols: 2)
    gamma = Vector.new(data: [1.0], size: 1)
    beta = Vector.new(data: [0.0, 0.0], size: 2)
    rm = Vector.new(data: [0.0, 0.0], size: 2)
    rv = Vector.new(data: [1.0, 1.0], size: 2)
    assert_raises(ArgumentError) { @blas.batch_norm(x, gamma, beta, rm, rv) }
  end
end

class TestMLConv2d < Minitest::Test
  include CodingAdventures::BlasLibrary

  def setup
    @blas = Backends::CpuBlas.new
  end

  def test_conv2d_basic
    input = Matrix.new(data: [1.0, 2.0, 3.0, 4.0, 5.0, 6.0, 7.0, 8.0, 9.0], rows: 3, cols: 3)
    weight = Matrix.new(data: [1.0, 0.0, 0.0, 1.0], rows: 2, cols: 2)
    result = @blas.conv2d(input, weight)
    assert_equal 2, result.rows
    assert_equal 2, result.cols
  end

  def test_conv2d_identity_filter
    input = Matrix.new(data: [1.0, 2.0, 3.0, 4.0], rows: 2, cols: 2)
    weight = Matrix.new(data: [1.0], rows: 1, cols: 1)
    result = @blas.conv2d(input, weight)
    assert_equal [1.0, 2.0, 3.0, 4.0], result.data
  end

  def test_conv2d_with_padding
    input = Matrix.new(data: [1.0, 2.0, 3.0, 4.0], rows: 2, cols: 2)
    weight = Matrix.new(data: [1.0, 1.0, 1.0, 1.0, 1.0, 1.0, 1.0, 1.0, 1.0], rows: 3, cols: 3)
    result = @blas.conv2d(input, weight, padding: 1)
    assert_equal 2, result.rows
    assert_equal 2, result.cols
  end

  def test_conv2d_with_stride
    input = Matrix.new(
      data: [1.0, 2.0, 3.0, 4.0, 5.0, 6.0, 7.0, 8.0, 9.0, 10.0, 11.0, 12.0, 13.0, 14.0, 15.0, 16.0],
      rows: 4, cols: 4
    )
    weight = Matrix.new(data: [1.0, 0.0, 0.0, 1.0], rows: 2, cols: 2)
    result = @blas.conv2d(input, weight, stride: 2)
    assert_equal 2, result.rows
    assert_equal 2, result.cols
  end

  def test_conv2d_with_bias
    input = Matrix.new(data: [1.0, 2.0, 3.0, 4.0], rows: 2, cols: 2)
    weight = Matrix.new(data: [1.0], rows: 1, cols: 1)
    bias = Vector.new(data: [10.0], size: 1)
    result = @blas.conv2d(input, weight, bias: bias)
    assert_equal [11.0, 12.0, 13.0, 14.0], result.data
  end

  def test_conv2d_invalid_dimensions
    input = Matrix.new(data: [1.0], rows: 1, cols: 1)
    weight = Matrix.new(data: [1.0, 2.0, 3.0, 4.0], rows: 2, cols: 2)
    assert_raises(ArgumentError) { @blas.conv2d(input, weight) }
  end
end

class TestMLAttention < Minitest::Test
  include CodingAdventures::BlasLibrary

  def setup
    @blas = Backends::CpuBlas.new
  end

  def test_attention_basic
    q = Matrix.new(data: [1.0, 0.0, 0.0, 1.0], rows: 2, cols: 2)
    k = Matrix.new(data: [1.0, 0.0, 0.0, 1.0], rows: 2, cols: 2)
    v = Matrix.new(data: [1.0, 2.0, 3.0, 4.0], rows: 2, cols: 2)
    result = @blas.attention(q, k, v)
    assert_equal 2, result.rows
    assert_equal 2, result.cols
  end

  def test_attention_identity_qk
    q = Matrix.new(data: [1.0, 0.0, 0.0, 1.0], rows: 2, cols: 2)
    k = Matrix.new(data: [1.0, 0.0, 0.0, 1.0], rows: 2, cols: 2)
    v = Matrix.new(data: [10.0, 20.0, 30.0, 40.0], rows: 2, cols: 2)
    result = @blas.attention(q, k, v)
    # Each output row is a weighted average of V rows
    assert_equal 2, result.rows
    assert_equal 2, result.cols
    result.data.each { |v_val| refute v_val.nan? }
  end

  def test_attention_custom_scale
    q = Matrix.new(data: [1.0, 0.0, 0.0, 1.0], rows: 2, cols: 2)
    k = Matrix.new(data: [1.0, 0.0, 0.0, 1.0], rows: 2, cols: 2)
    v = Matrix.new(data: [1.0, 2.0, 3.0, 4.0], rows: 2, cols: 2)
    result = @blas.attention(q, k, v, scale: 1.0)
    assert_equal 2, result.rows
    assert_equal 2, result.cols
  end

  def test_attention_with_mask
    q = Matrix.new(data: [1.0, 0.0, 0.0, 1.0], rows: 2, cols: 2)
    k = Matrix.new(data: [1.0, 0.0, 0.0, 1.0], rows: 2, cols: 2)
    v = Matrix.new(data: [1.0, 2.0, 3.0, 4.0], rows: 2, cols: 2)
    mask = Matrix.new(data: [0.0, -1e9, 0.0, 0.0], rows: 2, cols: 2)
    result = @blas.attention(q, k, v, mask: mask)
    assert_equal 2, result.rows
    assert_equal 2, result.cols
  end
end
