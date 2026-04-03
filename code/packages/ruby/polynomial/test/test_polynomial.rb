# frozen_string_literal: true

require "minitest/autorun"
require_relative "../lib/polynomial"

class TestPolynomial < Minitest::Test
  include Polynomial

  DELTA = 1e-9

  def poly_equal(a, b)
    return false unless a.length == b.length
    a.zip(b).all? { |x, y| (x - y).abs <= DELTA }
  end

  # -------------------------------------------------------------------------
  # normalize
  # -------------------------------------------------------------------------

  def test_normalize_strips_trailing_zeros
    assert_equal [1], Polynomial.normalize([1, 0, 0])
  end

  def test_normalize_all_zeros_is_empty
    assert_equal [], Polynomial.normalize([0])
    assert_equal [], Polynomial.normalize([0, 0, 0])
  end

  def test_normalize_empty_is_empty
    assert_equal [], Polynomial.normalize([])
  end

  def test_normalize_already_normalized
    assert_equal [1, 2, 3], Polynomial.normalize([1, 2, 3])
  end

  def test_normalize_preserves_internal_zeros
    assert_equal [1, 0, 2], Polynomial.normalize([1, 0, 2])
  end

  # -------------------------------------------------------------------------
  # degree
  # -------------------------------------------------------------------------

  def test_degree_zero_polynomial
    assert_equal(-1, Polynomial.degree([]))
    assert_equal(-1, Polynomial.degree([0]))
    assert_equal(-1, Polynomial.degree([0, 0]))
  end

  def test_degree_constant
    assert_equal 0, Polynomial.degree([7])
    assert_equal 0, Polynomial.degree([1])
  end

  def test_degree_linear
    assert_equal 1, Polynomial.degree([1, 2])
  end

  def test_degree_quadratic
    assert_equal 2, Polynomial.degree([1, 2, 3])
    assert_equal 2, Polynomial.degree([3, 0, 2])
  end

  def test_degree_ignores_trailing_zeros
    assert_equal 0, Polynomial.degree([3, 0, 0])
    assert_equal 1, Polynomial.degree([1, 2, 0])
  end

  # -------------------------------------------------------------------------
  # zero and one
  # -------------------------------------------------------------------------

  def test_zero_is_empty
    assert_equal [], Polynomial.zero
  end

  def test_zero_is_additive_identity
    p = [1, 2, 3]
    assert_equal p, Polynomial.add(Polynomial.zero, p)
    assert_equal p, Polynomial.add(p, Polynomial.zero)
  end

  def test_one_is_singleton
    assert_equal [1], Polynomial.one
  end

  def test_one_is_multiplicative_identity
    p = [1, 2, 3]
    assert_equal p, Polynomial.multiply(Polynomial.one, p)
    assert_equal p, Polynomial.multiply(p, Polynomial.one)
  end

  # -------------------------------------------------------------------------
  # add
  # -------------------------------------------------------------------------

  def test_add_same_length
    assert_equal [5, 7, 9], Polynomial.add([1, 2, 3], [4, 5, 6])
  end

  def test_add_different_lengths
    assert_equal [5, 7, 3], Polynomial.add([1, 2, 3], [4, 5])
    assert_equal [5, 7, 3], Polynomial.add([4, 5], [1, 2, 3])
  end

  def test_add_cancellation
    assert_equal [], Polynomial.add([1, 2, 3], [-1, -2, -3])
  end

  def test_add_zero_is_identity
    assert_equal [1, 2], Polynomial.add([], [1, 2])
    assert_equal [1, 2], Polynomial.add([1, 2], [])
  end

  def test_add_commutative
    a = [1, 2, 3]
    b = [4, 5, 6, 7]
    assert_equal Polynomial.add(a, b), Polynomial.add(b, a)
  end

  # -------------------------------------------------------------------------
  # subtract
  # -------------------------------------------------------------------------

  def test_subtract_basic
    assert_equal [4, 5], Polynomial.subtract([5, 7, 3], [1, 2, 3])
  end

  def test_subtract_self_is_zero
    assert_equal [], Polynomial.subtract([1, 2, 3], [1, 2, 3])
  end

  def test_subtract_zero_is_identity
    assert_equal [1, 2, 3], Polynomial.subtract([1, 2, 3], [])
  end

  def test_subtract_round_trip
    p = [3, 1, 4]
    q = [1, 5, 9]
    assert_equal p, Polynomial.add(Polynomial.subtract(p, q), q)
  end

  # -------------------------------------------------------------------------
  # multiply
  # -------------------------------------------------------------------------

  def test_multiply_two_linears
    # (1+2x)(3+4x) = 3 + 10x + 8x²
    assert_equal [3, 10, 8], Polynomial.multiply([1, 2], [3, 4])
  end

  def test_multiply_by_zero
    assert_equal [], Polynomial.multiply([1, 2, 3], [])
    assert_equal [], Polynomial.multiply([], [1, 2, 3])
  end

  def test_multiply_by_one
    assert_equal [1, 2, 3], Polynomial.multiply([1, 2, 3], [1])
    assert_equal [1, 2, 3], Polynomial.multiply([1], [1, 2, 3])
  end

  def test_multiply_commutative
    a = [1, 2, 3]
    b = [4, 5]
    assert poly_equal(Polynomial.multiply(a, b), Polynomial.multiply(b, a))
  end

  def test_multiply_result_degree
    a = [1, 2, 3]  # degree 2
    b = [4, 5, 6]  # degree 2
    result = Polynomial.multiply(a, b)
    assert_equal 4, Polynomial.degree(result)
  end

  # -------------------------------------------------------------------------
  # divmod_poly
  # -------------------------------------------------------------------------

  def test_divmod_raises_for_zero_divisor
    assert_raises(ArgumentError) { Polynomial.divmod_poly([1, 2, 3], []) }
    assert_raises(ArgumentError) { Polynomial.divmod_poly([1, 2, 3], [0]) }
  end

  def test_divmod_low_degree_dividend
    q, r = Polynomial.divmod_poly([1, 2], [0, 0, 1])
    assert_equal [], q
    assert_equal [1, 2], r
  end

  def test_divmod_zero_remainder
    product = Polynomial.multiply([1, 1], [1, 1])
    q, r = Polynomial.divmod_poly(product, [1, 1])
    assert poly_equal(q, [1, 1])
    assert_equal [], r
  end

  def test_divmod_satisfies_a_equals_bq_plus_r
    a = [5, 1, 3, 2]
    b = [2, 1]
    q, r = Polynomial.divmod_poly(a, b)
    reconstructed = Polynomial.add(Polynomial.multiply(b, q), r)
    assert poly_equal(reconstructed, Polynomial.normalize(a))
  end

  def test_divmod_spec_example
    a = [5, 1, 3, 2]
    b = [2, 1]
    q, r = Polynomial.divmod_poly(a, b)
    assert poly_equal(q, [3, -1, 2])
    assert poly_equal(r, [-1])
  end

  def test_divmod_constant_divisor
    q, r = Polynomial.divmod_poly([4, 6, 8], [2])
    assert poly_equal(q, [2, 3, 4])
    assert_equal [], r
  end

  # -------------------------------------------------------------------------
  # divide and mod
  # -------------------------------------------------------------------------

  def test_divide_raises_for_zero_divisor
    assert_raises(ArgumentError) { Polynomial.divide([1, 2], []) }
  end

  def test_mod_exact_is_zero
    p = Polynomial.multiply([1, 1], [2, 1])
    assert_equal [], Polynomial.mod(p, [1, 1])
  end

  def test_mod_raises_for_zero_divisor
    assert_raises(ArgumentError) { Polynomial.mod([1, 2], []) }
  end

  # -------------------------------------------------------------------------
  # evaluate
  # -------------------------------------------------------------------------

  def test_evaluate_zero_polynomial
    assert_equal 0, Polynomial.evaluate([], 5)
    assert_equal 0, Polynomial.evaluate([], 0)
  end

  def test_evaluate_constant
    assert_equal 7, Polynomial.evaluate([7], 100)
  end

  def test_evaluate_linear_at_zero
    assert_equal 3, Polynomial.evaluate([3, 2], 0)
  end

  def test_evaluate_spec_example
    # 3 + x + 2x² at x=4 → 39
    assert_in_delta 39, Polynomial.evaluate([3, 1, 2], 4), DELTA
  end

  def test_evaluate_matches_naive
    p = [1, -3, 2]
    x = 3
    naive = p[0] + p[1] * x + p[2] * x * x
    assert_in_delta naive, Polynomial.evaluate(p, x), DELTA
  end

  # -------------------------------------------------------------------------
  # gcd
  # -------------------------------------------------------------------------

  def test_gcd_with_zero_returns_other
    p = [1, 2, 3]
    assert poly_equal(Polynomial.gcd(p, []), Polynomial.normalize(p))
    assert poly_equal(Polynomial.gcd([], p), Polynomial.normalize(p))
  end

  def test_gcd_coprime_is_constant
    a = [1, 1]  # 1 + x
    b = [2, 1]  # 2 + x
    g = Polynomial.gcd(a, b)
    assert_equal 0, Polynomial.degree(g)
  end

  def test_gcd_common_factor
    f1 = Polynomial.multiply([1, 1], [2, 1])
    f2 = Polynomial.multiply([1, 1], [3, 1])
    g = Polynomial.gcd(f1, f2)
    assert_equal 1, Polynomial.degree(g)
    assert_equal [], Polynomial.mod(f1, g)
    assert_equal [], Polynomial.mod(f2, g)
  end
end
