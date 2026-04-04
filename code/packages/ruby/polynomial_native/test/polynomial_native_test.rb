# frozen_string_literal: true

# --------------------------------------------------------------------------
# polynomial_native_test.rb — Tests for the Rust-backed PolynomialNative module
# --------------------------------------------------------------------------
#
# These tests exercise every public function of the native polynomial extension
# to ensure the Rust implementation is correctly exposed to Ruby.
#
# Polynomials are represented as Ruby Arrays of Floats where index = degree:
#   [3.0, 0.0, 2.0]  =>  3 + 0·x + 2·x²

require_relative "test_helper"

M = CodingAdventures::PolynomialNative

class PolynomialNativeTest < Minitest::Test
  # Floating-point equality helper
  # We use a small tolerance because f64 arithmetic accumulates rounding errors.
  TOLERANCE = 1e-9

  def assert_poly_equal(expected, actual, msg = nil)
    assert_equal expected.length, actual.length,
      "#{msg}: polynomial length mismatch. Expected #{expected}, got #{actual}"
    expected.zip(actual).each_with_index do |(e, a), i|
      assert_in_delta e, a, TOLERANCE,
        "#{msg}: coefficient at index #{i} differs. Expected #{e}, got #{a}"
    end
  end

  # ========================================================================
  # normalize tests
  # ========================================================================

  def test_normalize_strips_trailing_zeros
    # [1.0, 0.0, 0.0] = 1 + 0x + 0x² → normalize to [1.0]
    result = M.normalize([1.0, 0.0, 0.0])
    assert_poly_equal [1.0], result, "normalize strips trailing zeros"
  end

  def test_normalize_zero_polynomial_becomes_empty
    # The zero polynomial [0.0] normalizes to [] (empty)
    result = M.normalize([0.0])
    assert_poly_equal [], result, "normalize([0.0]) returns []"
  end

  def test_normalize_empty_stays_empty
    result = M.normalize([])
    assert_poly_equal [], result, "normalize([]) returns []"
  end

  def test_normalize_no_trailing_zeros_unchanged
    result = M.normalize([1.0, 2.0, 3.0])
    assert_poly_equal [1.0, 2.0, 3.0], result, "normalize preserves fully-reduced poly"
  end

  def test_normalize_multiple_trailing_zeros
    result = M.normalize([5.0, 0.0, 0.0, 0.0])
    assert_poly_equal [5.0], result, "normalize strips multiple trailing zeros"
  end

  # ========================================================================
  # degree tests
  # ========================================================================

  def test_degree_simple
    # [3.0, 0.0, 2.0] = 3 + 2x², highest term is x² → degree 2
    assert_equal 2, M.degree([3.0, 0.0, 2.0])
  end

  def test_degree_constant_polynomial
    # [7.0] = constant 7, degree 0
    assert_equal 0, M.degree([7.0])
  end

  def test_degree_zero_polynomial
    # The zero polynomial has degree 0 by convention
    assert_equal 0, M.degree([])
    assert_equal 0, M.degree([0.0])
  end

  def test_degree_with_trailing_zeros
    # [1.0, 0.0] = 1 + 0x → degree 0 after normalization
    assert_equal 0, M.degree([1.0, 0.0])
  end

  # ========================================================================
  # zero and one tests
  # ========================================================================

  def test_zero_returns_zero_polynomial
    result = M.zero
    assert_poly_equal [0.0], result, "zero() returns [0.0]"
  end

  def test_one_returns_unit_polynomial
    result = M.one
    assert_poly_equal [1.0], result, "one() returns [1.0]"
  end

  def test_zero_is_additive_identity
    # add(zero(), p) == p for any polynomial p
    p = [1.0, 2.0, 3.0]
    result = M.add(M.zero, p)
    assert_poly_equal p, result, "zero is additive identity"
  end

  def test_one_is_multiplicative_identity
    # multiply(one(), p) == p for any polynomial p
    p = [1.0, 2.0, 3.0]
    result = M.multiply(M.one, p)
    assert_poly_equal p, result, "one is multiplicative identity"
  end

  # ========================================================================
  # add tests
  # ========================================================================

  def test_add_same_length
    # (1 + 2x + 3x²) + (4 + 5x) = 5 + 7x + 3x²
    a = [1.0, 2.0, 3.0]
    b = [4.0, 5.0]
    result = M.add(a, b)
    assert_poly_equal [5.0, 7.0, 3.0], result, "add same-length polys"
  end

  def test_add_different_lengths
    a = [1.0]
    b = [0.0, 2.0, 3.0]
    result = M.add(a, b)
    assert_poly_equal [1.0, 2.0, 3.0], result, "add different-length polys"
  end

  def test_add_cancellation
    # Adding a polynomial to its negation gives zero
    a = [1.0, 2.0, 3.0]
    neg_a = [-1.0, -2.0, -3.0]
    result = M.add(a, neg_a)
    assert_poly_equal [], result, "add with negation cancels to zero"
  end

  def test_add_with_zero_polynomial
    p = [5.0, 3.0]
    result = M.add(p, [])
    assert_poly_equal p, result, "add with empty is identity"
  end

  def test_add_is_commutative
    a = [1.0, 2.0]
    b = [3.0, 4.0, 5.0]
    assert_poly_equal M.add(a, b), M.add(b, a), "add is commutative"
  end

  # ========================================================================
  # subtract tests
  # ========================================================================

  def test_subtract_basic
    # (5 + 7x + 3x²) - (1 + 2x + 3x²) = 4 + 5x
    a = [5.0, 7.0, 3.0]
    b = [1.0, 2.0, 3.0]
    result = M.subtract(a, b)
    assert_poly_equal [4.0, 5.0], result, "subtract basic case"
  end

  def test_subtract_self_is_zero
    p = [1.0, 2.0, 3.0]
    result = M.subtract(p, p)
    assert_poly_equal [], result, "subtract self gives zero"
  end

  def test_subtract_zero_is_identity
    p = [3.0, 1.0]
    result = M.subtract(p, [])
    assert_poly_equal p, result, "subtract zero is identity"
  end

  # ========================================================================
  # multiply tests
  # ========================================================================

  def test_multiply_linear_polynomials
    # (1 + 2x)(3 + 4x) = 3 + 4x + 6x + 8x² = 3 + 10x + 8x²
    a = [1.0, 2.0]
    b = [3.0, 4.0]
    result = M.multiply(a, b)
    assert_poly_equal [3.0, 10.0, 8.0], result, "multiply linear polys"
  end

  def test_multiply_by_constant
    # 2 × (1 + 2x + 3x²) = 2 + 4x + 6x²
    a = [2.0]
    b = [1.0, 2.0, 3.0]
    result = M.multiply(a, b)
    assert_poly_equal [2.0, 4.0, 6.0], result, "multiply by constant"
  end

  def test_multiply_by_zero_polynomial
    p = [1.0, 2.0, 3.0]
    result = M.multiply(p, [])
    assert_poly_equal [], result, "multiply by zero poly gives zero"
  end

  def test_multiply_is_commutative
    a = [1.0, 2.0]
    b = [3.0, 4.0, 5.0]
    assert_poly_equal M.multiply(a, b), M.multiply(b, a), "multiply is commutative"
  end

  def test_multiply_degree_sums
    # degree(a) = 2, degree(b) = 3, product has degree 5
    a = [1.0, 0.0, 1.0]   # 1 + x²
    b = [1.0, 0.0, 0.0, 1.0]  # 1 + x³
    result = M.multiply(a, b)
    assert_equal 5, M.degree(result), "product degree = sum of input degrees"
  end

  # ========================================================================
  # divmod_poly tests
  # ========================================================================

  def test_divmod_basic
    # (5 + x + 3x² + 2x³) ÷ (2 + x) = quotient [3, -1, 2], remainder [-1]
    dividend = [5.0, 1.0, 3.0, 2.0]
    divisor  = [2.0, 1.0]
    q, r = M.divmod_poly(dividend, divisor)
    assert_poly_equal [3.0, -1.0, 2.0], q, "divmod quotient"
    assert_poly_equal [-1.0], r, "divmod remainder"
  end

  def test_divmod_exact_division
    # (x² - 3x + 2) ÷ (x - 1) = (x - 2), remainder 0
    # [2, -3, 1] ÷ [- 1, 1]
    dividend = [2.0, -3.0, 1.0]
    divisor  = [-1.0, 1.0]
    q, r = M.divmod_poly(dividend, divisor)
    assert_poly_equal [-2.0, 1.0], q, "exact division quotient is (x-2)"
    assert_poly_equal [], r, "exact division remainder is zero"
  end

  def test_divmod_returns_array_of_two_arrays
    q_r = M.divmod_poly([1.0], [1.0])
    assert_instance_of Array, q_r
    assert_equal 2, q_r.length
    assert_instance_of Array, q_r[0]
    assert_instance_of Array, q_r[1]
  end

  def test_divmod_zero_divisor_raises
    assert_raises(ArgumentError) do
      M.divmod_poly([1.0, 2.0], [])
    end
  end

  def test_divmod_zero_divisor_all_zeros_raises
    assert_raises(ArgumentError) do
      M.divmod_poly([1.0, 2.0], [0.0])
    end
  end

  def test_divmod_reconstruction
    # dividend = divisor * quotient + remainder
    dividend = [1.0, 2.0, 3.0, 4.0]
    divisor  = [1.0, 1.0]
    q, r = M.divmod_poly(dividend, divisor)
    reconstructed = M.add(M.multiply(divisor, q), r)
    assert_poly_equal dividend, reconstructed, "divmod: a = b*q + r"
  end

  # ========================================================================
  # divide tests
  # ========================================================================

  def test_divide_basic
    dividend = [5.0, 1.0, 3.0, 2.0]
    divisor  = [2.0, 1.0]
    result = M.divide(dividend, divisor)
    assert_poly_equal [3.0, -1.0, 2.0], result, "divide returns quotient"
  end

  def test_divide_zero_divisor_raises
    assert_raises(ArgumentError) do
      M.divide([1.0], [0.0])
    end
  end

  def test_divide_by_self_gives_constant_one
    p = [2.0, 3.0]
    result = M.divide(p, p)
    assert_poly_equal [1.0], result, "p / p = 1"
  end

  # ========================================================================
  # modulo tests
  # ========================================================================

  def test_modulo_basic
    dividend = [5.0, 1.0, 3.0, 2.0]
    divisor  = [2.0, 1.0]
    result = M.modulo(dividend, divisor)
    assert_poly_equal [-1.0], result, "modulo returns remainder"
  end

  def test_modulo_exact_division_is_zero
    # (x² - 1) = (x - 1)(x + 1), so remainder when divided by (x - 1) is 0
    dividend = [-1.0, 0.0, 1.0]   # x² - 1
    divisor  = [-1.0, 1.0]        # x - 1
    result = M.modulo(dividend, divisor)
    assert_poly_equal [], result, "exact division has zero remainder"
  end

  def test_modulo_zero_divisor_raises
    assert_raises(ArgumentError) do
      M.modulo([1.0], [])
    end
  end

  # ========================================================================
  # evaluate tests
  # ========================================================================

  def test_evaluate_constant_polynomial
    # p(x) = 5 for all x
    assert_in_delta 5.0, M.evaluate([5.0], 3.0), TOLERANCE
    assert_in_delta 5.0, M.evaluate([5.0], 0.0), TOLERANCE
  end

  def test_evaluate_linear_polynomial
    # p(x) = 2 + 3x  →  p(4) = 2 + 12 = 14
    result = M.evaluate([2.0, 3.0], 4.0)
    assert_in_delta 14.0, result, TOLERANCE, "evaluate linear poly at 4"
  end

  def test_evaluate_quadratic
    # p(x) = 3 + 0x + 1x²  →  p(2) = 3 + 0 + 4 = 7
    result = M.evaluate([3.0, 0.0, 1.0], 2.0)
    assert_in_delta 7.0, result, TOLERANCE, "evaluate quadratic at x=2"
  end

  def test_evaluate_at_zero
    # p(0) = constant term
    result = M.evaluate([7.0, 3.0, 2.0], 0.0)
    assert_in_delta 7.0, result, TOLERANCE, "p(0) equals constant term"
  end

  def test_evaluate_zero_polynomial
    # The zero polynomial evaluates to 0 everywhere
    result = M.evaluate([], 42.0)
    assert_in_delta 0.0, result, TOLERANCE, "zero poly evaluates to 0"
  end

  def test_evaluate_root
    # (x - 2)(x - 3) = x² - 5x + 6 = [6, -5, 1]
    # p(2) should be 0 and p(3) should be 0
    poly = [6.0, -5.0, 1.0]
    assert_in_delta 0.0, M.evaluate(poly, 2.0), TOLERANCE, "p(2) = 0"
    assert_in_delta 0.0, M.evaluate(poly, 3.0), TOLERANCE, "p(3) = 0"
  end

  # ========================================================================
  # gcd tests
  # ========================================================================

  def test_gcd_coprime_polynomials
    # gcd(x², x + 1) = 1 (they share no common factor)
    a = [0.0, 0.0, 1.0]   # x²
    b = [1.0, 1.0]         # x + 1
    result = M.gcd(a, b)
    # GCD should be a constant (degree 0)
    assert_equal 0, M.degree(result), "gcd of coprime polys has degree 0"
  end

  def test_gcd_common_factor
    # (x-1)(x-2) and (x-1) share the factor (x-1)
    a = [2.0, -3.0, 1.0]   # (x-1)(x-2) = x² - 3x + 2
    b = [-1.0, 1.0]         # (x - 1)
    result = M.gcd(a, b)
    # gcd should divide (x-1), so remainder when dividing (x-1) by gcd is zero
    assert_poly_equal [], M.modulo(b, result), "gcd divides (x-1)"
    assert_poly_equal [], M.modulo(a, result), "gcd divides (x-1)(x-2)"
  end

  def test_gcd_self
    # gcd(p, p) = normalize(p)
    p = [6.0, -5.0, 1.0]
    result = M.gcd(p, p)
    # Result should be proportional to p (polynomials don't have a unique monic gcd)
    assert_equal M.degree(p), M.degree(result), "gcd(p, p) has same degree as p"
  end

  def test_gcd_with_zero
    # gcd(p, 0) = p (by Euclidean algorithm convention)
    p = [1.0, 2.0]
    result = M.gcd(p, [])
    assert_equal M.degree(p), M.degree(result), "gcd(p, 0) has same degree as p"
  end

  # ========================================================================
  # Version constant
  # ========================================================================

  def test_version_constant_exists
    assert_equal "0.1.0", CodingAdventures::PolynomialNative::VERSION
  end

  # ========================================================================
  # Round-trip and algebraic identity tests
  # ========================================================================

  def test_add_subtract_round_trip
    # (a + b) - b == a
    a = [1.0, 2.0, 3.0]
    b = [4.0, 5.0]
    result = M.subtract(M.add(a, b), b)
    assert_poly_equal a, result, "(a + b) - b = a"
  end

  def test_multiply_divide_round_trip
    # (a * b) / b == a  (when b is non-zero)
    a = [1.0, 2.0, 3.0]
    b = [1.0, 1.0]   # x + 1
    product = M.multiply(a, b)
    result = M.divide(product, b)
    assert_poly_equal a, result, "(a * b) / b = a"
  end
end
