# frozen_string_literal: true

# =============================================================================
# polynomial — Polynomial arithmetic over real numbers.
# =============================================================================
#
# A polynomial is represented as a frozen array of numbers where the index
# equals the degree of that term:
#
#   [3, 0, 2]  →  3 + 0·x + 2·x²
#   []         →  the zero polynomial
#
# This "little-endian" convention makes addition position-aligned and
# Horner's method natural to implement.
#
# All functions return normalized polynomials — trailing zeros are stripped.
# =============================================================================

module Polynomial
  VERSION = "0.1.0"

  # ---------------------------------------------------------------------------
  # Fundamentals
  # ---------------------------------------------------------------------------

  # Remove trailing zeros from a polynomial array.
  #
  # Trailing zeros represent zero-coefficient high-degree terms. They do not
  # change the mathematical value but affect degree comparisons and division.
  #
  # @param p [Array<Numeric>] input polynomial
  # @return [Array<Numeric>] normalized polynomial
  #
  # Example:
  #   normalize([1, 0, 0]) → [1]
  #   normalize([0])       → []
  def self.normalize(p)
    result = p.dup
    result.pop while !result.empty? && result.last == 0
    result.freeze
  end

  # Return the degree of a polynomial.
  #
  # The degree is the index of the highest non-zero coefficient.
  # By convention, the zero polynomial has degree -1.
  #
  # @param p [Array<Numeric>] polynomial
  # @return [Integer] degree, or -1 for the zero polynomial
  def self.degree(p)
    normalize(p).length - 1
  end

  # Return the zero polynomial.
  # @return [Array] empty array []
  def self.zero
    [].freeze
  end

  # Return the multiplicative identity polynomial.
  # @return [Array] [1]
  def self.one
    [1].freeze
  end

  # ---------------------------------------------------------------------------
  # Addition and Subtraction
  # ---------------------------------------------------------------------------

  # Add two polynomials term-by-term.
  #
  # Shorter polynomial is implicitly zero-padded.
  #
  # @param a [Array<Numeric>] first polynomial
  # @param b [Array<Numeric>] second polynomial
  # @return [Array<Numeric>] normalized sum
  #
  # Example:
  #   add([1, 2, 3], [4, 5]) → [5, 7, 3]
  def self.add(a, b)
    len = [a.length, b.length].max
    result = Array.new(len) do |i|
      (i < a.length ? a[i] : 0) + (i < b.length ? b[i] : 0)
    end
    normalize(result)
  end

  # Subtract polynomial b from polynomial a term-by-term.
  #
  # @param a [Array<Numeric>] minuend
  # @param b [Array<Numeric>] subtrahend
  # @return [Array<Numeric>] normalized difference
  #
  # Example:
  #   subtract([5, 7, 3], [1, 2, 3]) → [4, 5]
  def self.subtract(a, b)
    len = [a.length, b.length].max
    result = Array.new(len) do |i|
      (i < a.length ? a[i] : 0) - (i < b.length ? b[i] : 0)
    end
    normalize(result)
  end

  # ---------------------------------------------------------------------------
  # Multiplication
  # ---------------------------------------------------------------------------

  # Multiply two polynomials using polynomial convolution.
  #
  # Each term a[i]·xⁱ multiplies each term b[j]·xʲ, contributing
  # a[i]·b[j] to the result at index i+j.
  #
  # @param a [Array<Numeric>] first polynomial
  # @param b [Array<Numeric>] second polynomial
  # @return [Array<Numeric>] normalized product
  #
  # Example:
  #   multiply([1, 2], [3, 4]) → [3, 10, 8]
  #   Because (1+2x)(3+4x) = 3 + 10x + 8x²
  def self.multiply(a, b)
    return zero if a.empty? || b.empty?

    result = Array.new(a.length + b.length - 1, 0)
    a.each_with_index do |ai, i|
      b.each_with_index do |bj, j|
        result[i + j] += ai * bj
      end
    end
    normalize(result)
  end

  # ---------------------------------------------------------------------------
  # Division
  # ---------------------------------------------------------------------------

  # Perform polynomial long division, returning [quotient, remainder].
  #
  # Finds q and r such that: a = b * q + r and degree(r) < degree(b).
  #
  # Named divmod_poly to match the Python package convention and avoid
  # conflicts with Ruby's Numeric#divmod.
  #
  # @param a [Array<Numeric>] dividend
  # @param b [Array<Numeric>] divisor (must not be zero polynomial)
  # @return [Array] two-element array [quotient, remainder]
  # @raise [ArgumentError] if b is the zero polynomial
  #
  # Example:
  #   divmod_poly([5, 1, 3, 2], [2, 1]) → [[3.0, -1.0, 2.0], [-1.0]]
  def self.divmod_poly(a, b)
    nb = normalize(b)
    raise ArgumentError, "polynomial division by zero" if nb.empty?

    na = normalize(a)
    deg_a = na.length - 1
    deg_b = nb.length - 1

    return [zero, na] if deg_a < deg_b

    rem = na.dup
    quot = Array.new(deg_a - deg_b + 1, 0.0)
    lead_b = nb[deg_b].to_f
    deg_rem = deg_a

    while deg_rem >= deg_b
      lead_rem = rem[deg_rem].to_f
      coeff = lead_rem / lead_b
      power = deg_rem - deg_b
      quot[power] = coeff

      (0..deg_b).each do |j|
        rem[power + j] -= coeff * nb[j]
      end

      deg_rem -= 1
      deg_rem -= 1 while deg_rem >= 0 && rem[deg_rem] == 0
    end

    [normalize(quot), normalize(rem)]
  end

  # Return the quotient of divmod_poly(a, b).
  # @raise [ArgumentError] if b is the zero polynomial
  def self.divide(a, b)
    divmod_poly(a, b)[0]
  end

  # Return the remainder of divmod_poly(a, b).
  # @raise [ArgumentError] if b is the zero polynomial
  def self.mod(a, b)
    divmod_poly(a, b)[1]
  end

  # ---------------------------------------------------------------------------
  # Evaluation
  # ---------------------------------------------------------------------------

  # Evaluate a polynomial at x using Horner's method.
  #
  # Horner's method rewrites the polynomial as:
  #   a₀ + x(a₁ + x(a₂ + ... + x·aₙ))
  #
  # This needs only n additions and n multiplications.
  #
  # @param p [Array<Numeric>] polynomial
  # @param x [Numeric] the evaluation point
  # @return [Numeric] p(x)
  #
  # Example:
  #   evaluate([3, 1, 2], 4) → 39
  #   Because 3 + 4 + 2·16 = 39
  def self.evaluate(p, x)
    n = normalize(p)
    return 0 if n.empty?

    acc = 0
    n.reverse_each { |coeff| acc = acc * x + coeff }
    acc
  end

  # ---------------------------------------------------------------------------
  # GCD
  # ---------------------------------------------------------------------------

  # Compute the GCD of two polynomials using the Euclidean algorithm.
  #
  # Repeatedly replaces (a, b) with (b, a mod b) until b is zero.
  #
  # @param a [Array<Numeric>] first polynomial
  # @param b [Array<Numeric>] second polynomial
  # @return [Array<Numeric>] normalized GCD
  def self.gcd(a, b)
    u = normalize(a)
    v = normalize(b)
    until v.empty?
      r = mod(u, v)
      u = v
      v = r
    end
    normalize(u)
  end
end
