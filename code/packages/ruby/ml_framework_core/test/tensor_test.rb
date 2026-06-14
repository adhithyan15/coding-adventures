# frozen_string_literal: true

# tensor_test.rb — exhaustive minitest coverage for the v0.1 Tensor class.
# =========================================================================
#
# Mirrors the test categories from
# code/packages/python/ml-framework-core/tests/test_tensor.py but adapted
# to v0.1's reduced scope (no broadcasting, no indexing, no reductions).
#
# Sections, in test-discovery order:
#   - Construction + shape inference
#   - Factories: zeros, ones, full, eye, arange, randn, from_array
#   - Shape ops: reshape, transpose, flatten, squeeze, unsqueeze
#   - Operator overloads (+, -, *, /, **, unary -)
#   - Equality + inspect
#   - Round-trip properties (the actually-important integration tests)

require "minitest/autorun"
require "coding_adventures/ml_framework_core"

T = CodingAdventures::MLFrameworkCore::Tensor

class TensorConstructionTest < Minitest::Test
  def test_construct_from_flat_array_with_explicit_shape
    t = T.new([1.0, 2.0, 3.0, 4.0], shape: [2, 2])
    assert_equal [2, 2], t.shape
    assert_equal 4, t.numel
    assert_equal 2, t.ndim
    assert_equal :f32, t.dtype
    assert_equal [1.0, 2.0, 3.0, 4.0], t.to_a
  end

  def test_construct_from_nested_array_infers_shape
    t = T.new([[1, 2, 3], [4, 5, 6]])
    assert_equal [2, 3], t.shape
    assert_equal [1.0, 2.0, 3.0, 4.0, 5.0, 6.0], t.to_a
  end

  def test_construct_from_deeply_nested_array
    # Three-deep nesting → rank-3 tensor.
    t = T.new([[[1, 2], [3, 4]], [[5, 6], [7, 8]]])
    assert_equal [2, 2, 2], t.shape
    assert_equal 8, t.numel
  end

  def test_construct_from_scalar
    # Wrapping a scalar gives a 1-element 1-D tensor (Python equivalent
    # for scalar inputs, since we don't have 0-d tensors in v0.1).
    t = T.new(7.0, shape: [1])
    assert_equal [1], t.shape
    assert_equal [7.0], t.to_a
  end

  def test_ragged_nested_array_raises
    assert_raises(ArgumentError) { T.new([[1, 2], [3]]) }
  end

  def test_data_length_mismatch_with_explicit_shape_raises
    assert_raises(ArgumentError) { T.new([1, 2, 3], shape: [2, 2]) }
  end

  def test_unsupported_dtype_raises
    assert_raises(ArgumentError) { T.new([1.0], dtype: :f64) }
  end

  def test_requires_grad_defaults_to_false
    assert_equal false, T.new([1.0]).requires_grad
  end

  def test_requires_grad_can_be_set
    t = T.new([1.0])
    t.requires_grad = true
    assert_equal true, t.requires_grad
  end

  def test_grad_is_nil_until_set
    assert_nil T.new([1.0]).grad
  end
end

class TensorFactoriesTest < Minitest::Test
  def test_zeros
    t = T.zeros(2, 3)
    assert_equal [2, 3], t.shape
    assert_equal [0.0] * 6, t.to_a
  end

  def test_zeros_accepts_single_array_arg
    t = T.zeros([2, 3])
    assert_equal [2, 3], t.shape
  end

  def test_ones
    t = T.ones(3)
    assert_equal [3], t.shape
    assert_equal [1.0, 1.0, 1.0], t.to_a
  end

  def test_full
    t = T.full([2, 2], 7.5)
    assert_equal [2, 2], t.shape
    assert_equal [7.5, 7.5, 7.5, 7.5], t.to_a
  end

  def test_eye_square_is_identity_matrix
    t = T.eye(3)
    assert_equal [3, 3], t.shape
    assert_equal [1.0, 0.0, 0.0,
                  0.0, 1.0, 0.0,
                  0.0, 0.0, 1.0], t.to_a
  end

  def test_eye_rectangular
    t = T.eye(2, 3)
    assert_equal [2, 3], t.shape
    # 1s on the diagonal of the smaller dim, 0s elsewhere.
    assert_equal [1.0, 0.0, 0.0,
                  0.0, 1.0, 0.0], t.to_a
  end

  def test_arange_stop_only
    t = T.arange(5)
    assert_equal [5], t.shape
    assert_equal [0.0, 1.0, 2.0, 3.0, 4.0], t.to_a
  end

  def test_arange_start_stop
    t = T.arange(2, 5)
    assert_equal [2.0, 3.0, 4.0], t.to_a
  end

  def test_arange_with_step
    t = T.arange(0, 10, 2)
    assert_equal [0.0, 2.0, 4.0, 6.0, 8.0], t.to_a
  end

  def test_arange_negative_step
    t = T.arange(5, 0, -1)
    assert_equal [5.0, 4.0, 3.0, 2.0, 1.0], t.to_a
  end

  def test_arange_zero_step_raises
    assert_raises(ArgumentError) { T.arange(0, 5, 0) }
  end

  def test_arange_wrong_arity_raises
    assert_raises(ArgumentError) { T.arange }
    assert_raises(ArgumentError) { T.arange(1, 2, 3, 4) }
  end

  def test_from_array_is_same_as_new
    a = T.from_array([[1, 2], [3, 4]])
    b = T.new([[1, 2], [3, 4]])
    assert_equal a, b
  end

  def test_randn_shape_is_respected
    t = T.randn(3, 4, seed: 42)
    assert_equal [3, 4], t.shape
    assert_equal 12, t.numel
    # Sanity: values aren't all the same (very unlikely with N(0,1)).
    assert t.to_a.uniq.length > 1
  end

  def test_randn_is_deterministic_with_seed
    a = T.randn(5, seed: 123)
    b = T.randn(5, seed: 123)
    assert_equal a.to_a, b.to_a
  end

  def test_randn_is_nondeterministic_without_seed
    # Theoretically could collide, but the probability with 5 f64
    # samples is astronomically small.
    a = T.randn(5)
    b = T.randn(5)
    refute_equal a.to_a, b.to_a
  end

  def test_randn_mean_roughly_zero
    # Box-Muller mean → 0 in the limit; with 1000 samples, |mean| < 0.2
    # is a generous bound (~6σ for N(0, 1/sqrt(1000))).
    t = T.randn(1000, seed: 7)
    mean = t.to_a.sum / t.numel
    assert mean.abs < 0.2, "mean #{mean} should be near 0 for randn"
  end
end

class TensorShapeOpsTest < Minitest::Test
  def test_reshape_preserves_data
    t = T.arange(6).reshape(2, 3)
    assert_equal [2, 3], t.shape
    assert_equal [0.0, 1.0, 2.0, 3.0, 4.0, 5.0], t.to_a
  end

  def test_reshape_accepts_single_array
    t = T.arange(6).reshape([3, 2])
    assert_equal [3, 2], t.shape
  end

  def test_reshape_numel_mismatch_raises
    assert_raises(ArgumentError) { T.arange(6).reshape(2, 4) }
  end

  def test_reshape_then_back_round_trip
    t = T.arange(12)
    assert_equal t, t.reshape(3, 4).reshape(12)
  end

  def test_reshape_to_same_shape_is_equal
    t = T.arange(6).reshape(2, 3)
    assert_equal t, t.reshape(t.shape)
  end

  def test_flatten
    t = T.new([[1, 2], [3, 4]]).flatten
    assert_equal [4], t.shape
    assert_equal [1.0, 2.0, 3.0, 4.0], t.to_a
  end

  def test_transpose_2d
    t = T.new([[1, 2, 3], [4, 5, 6]]).transpose
    assert_equal [3, 2], t.shape
    # Original was row-major [1,2,3,4,5,6]; transposed reads column-by-column.
    assert_equal [1.0, 4.0, 2.0, 5.0, 3.0, 6.0], t.to_a
  end

  def test_transpose_explicit_perm
    t = T.new([[1, 2], [3, 4]]).transpose(1, 0)
    assert_equal [[1.0, 3.0], [2.0, 4.0]], t.to_nested_a
  end

  def test_transpose_double_is_identity
    original = T.new([[1, 2, 3], [4, 5, 6]])
    assert_equal original, original.transpose.transpose
  end

  def test_transpose_rejects_invalid_perm
    t = T.new([[1, 2], [3, 4]])
    assert_raises(ArgumentError) { t.transpose(0, 0) }
    assert_raises(ArgumentError) { t.transpose(0, 1, 2) }
  end

  def test_transpose_higher_rank_not_yet_supported
    t = T.zeros(2, 2, 2)
    assert_raises(NotImplementedError) { t.transpose(2, 1, 0) }
  end

  def test_squeeze_no_arg_drops_all_size_1
    t = T.zeros(1, 3, 1, 2)
    assert_equal [3, 2], t.squeeze.shape
  end

  def test_squeeze_axis_drops_that_axis_only
    t = T.zeros(1, 3, 1)
    assert_equal [3, 1], t.squeeze(0).shape
    assert_equal [1, 3], t.squeeze(2).shape
  end

  def test_squeeze_negative_axis
    t = T.zeros(3, 1)
    assert_equal [3], t.squeeze(-1).shape
  end

  def test_squeeze_nonunit_axis_raises
    assert_raises(ArgumentError) { T.zeros(3, 2).squeeze(0) }
  end

  def test_unsqueeze_inserts_axis
    t = T.zeros(3)
    assert_equal [1, 3], t.unsqueeze(0).shape
    assert_equal [3, 1], t.unsqueeze(1).shape
    assert_equal [3, 1], t.unsqueeze(-1).shape   # equivalent to ndim
  end

  def test_unsqueeze_then_squeeze_round_trip
    t = T.arange(6).reshape(2, 3)
    assert_equal t, t.unsqueeze(0).squeeze(0)
  end
end

class TensorArithmeticTest < Minitest::Test
  def test_add_tensors
    a = T.new([1, 2, 3])
    b = T.new([10, 20, 30])
    assert_equal [11.0, 22.0, 33.0], (a + b).to_a
  end

  def test_add_scalar
    a = T.new([1, 2, 3])
    assert_equal [11.0, 12.0, 13.0], (a + 10).to_a
  end

  def test_sub
    a = T.new([5, 7, 9])
    b = T.new([1, 2, 3])
    assert_equal [4.0, 5.0, 6.0], (a - b).to_a
  end

  def test_mul
    a = T.new([2, 3, 4])
    b = T.new([5, 6, 7])
    assert_equal [10.0, 18.0, 28.0], (a * b).to_a
  end

  def test_div
    a = T.new([10, 20, 30])
    b = T.new([2, 4, 5])
    assert_equal [5.0, 5.0, 6.0], (a / b).to_a
  end

  def test_pow
    a = T.new([2, 3, 4])
    assert_equal [4.0, 9.0, 16.0], (a**2).to_a
  end

  def test_unary_neg
    a = T.new([1, -2, 3])
    assert_equal [-1.0, 2.0, -3.0], (-a).to_a
  end

  def test_shape_mismatch_raises
    assert_raises(ArgumentError) { T.zeros(2, 3) + T.zeros(3, 2) }
  end

  def test_unsupported_operand_type_raises
    assert_raises(TypeError) { T.zeros(2) + "not a number" }
  end
end

class TensorEqualityAndInspectTest < Minitest::Test
  def test_equality_compares_shape_and_data
    assert_equal T.new([1, 2, 3]), T.new([1, 2, 3])
    refute_equal T.new([1, 2, 3]), T.new([1, 2, 4])
    refute_equal T.new([1, 2, 3]), T.new([1, 2, 3], shape: [3, 1])
  end

  def test_equality_with_non_tensor_is_false
    refute_equal T.new([1]), [1.0]
  end

  def test_hash_equality_for_equal_tensors
    a = T.new([1, 2, 3])
    b = T.new([1, 2, 3])
    assert_equal a.hash, b.hash
  end

  def test_inspect_includes_shape_and_dtype
    s = T.zeros(2, 3).inspect
    assert_match(/shape=\[2, 3\]/, s)
    assert_match(/dtype=f32/, s)
  end

  def test_inspect_truncates_long_data
    s = T.arange(100).inspect
    assert_match(/\.\.\./, s)
  end
end

class TensorRoundTripTest < Minitest::Test
  def test_to_nested_a_round_trips
    original = [[1.0, 2.0, 3.0], [4.0, 5.0, 6.0]]
    assert_equal original, T.new(original).to_nested_a
  end

  def test_to_nested_a_for_3d
    original = [[[1.0, 2.0], [3.0, 4.0]], [[5.0, 6.0], [7.0, 8.0]]]
    assert_equal original, T.new(original).to_nested_a
  end

  def test_reshape_preserves_to_a
    t = T.arange(12)
    assert_equal t.to_a, t.reshape(3, 4).to_a
  end
end

class TensorVersionTest < Minitest::Test
  def test_version_constant_is_defined
    assert_kind_of String, CodingAdventures::MLFrameworkCore::VERSION
    assert_match(/\A\d+\.\d+\.\d+\z/, CodingAdventures::MLFrameworkCore::VERSION)
  end

  def test_short_alias_module_is_defined
    assert_equal CodingAdventures::MLFrameworkCore, MLFrameworkCore
  end
end
