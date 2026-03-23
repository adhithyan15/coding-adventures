# frozen_string_literal: true

# =============================================================================
# Trig — Trigonometric Functions from First Principles
# =============================================================================
#
# This module implements sine and cosine using the Maclaurin series (a special
# case of the Taylor series, expanded around x = 0). No built-in Math library
# is used — everything is computed from scratch using only addition,
# subtraction, multiplication, and division.
#
# ## Why Maclaurin Series?
#
# The Maclaurin series lets us approximate any smooth function as an infinite
# sum of polynomial terms. For sine and cosine, these series converge for ALL
# real numbers, which means we can compute them to arbitrary precision just by
# adding enough terms.
#
# ## The Key Insight: Iterative Term Computation
#
# Instead of computing each term from scratch (which would require computing
# large factorials and powers), we compute each term FROM the previous one.
# This is both faster and avoids overflow problems.
#
# For sin(x):
#   term_n = term_{n-1} * (-x^2) / ((2n)(2n+1))
#
# For cos(x):
#   term_n = term_{n-1} * (-x^2) / ((2n-1)(2n))
#
# Each new term just multiplies the old term by a small fraction, so numbers
# stay manageable.
#
# ## Range Reduction
#
# While the Maclaurin series converges for any x, it converges MUCH faster
# when x is small (close to zero). For large x, we'd need many more terms.
#
# The trick: sin and cos are periodic with period 2*PI. So sin(x) = sin(x mod 2*PI).
# We reduce x to the range [-PI, PI] before computing, which guarantees fast
# convergence with just 20 terms.
#
# =============================================================================

module Trig
  # ---------------------------------------------------------------------------
  # Constants
  # ---------------------------------------------------------------------------
  #
  # PI (pi) is the ratio of a circle's circumference to its diameter.
  # This is the same value as Math::PI in Ruby's standard library, accurate
  # to the full precision of a 64-bit IEEE 754 floating-point number
  # (about 15-16 significant decimal digits).

  PI = 3.141592653589793

  # TWO_PI is used for range reduction. Since sin and cos repeat every 2*PI
  # radians, we can always reduce our input to within one full cycle.

  TWO_PI = 2.0 * PI

  # ---------------------------------------------------------------------------
  # Range Reduction (private helper)
  # ---------------------------------------------------------------------------
  #
  # Normalizes any angle x (in radians) to the range [-PI, PI].
  #
  # How it works:
  #   1. Use modulo to bring x into [0, 2*PI)
  #   2. If the result is greater than PI, subtract 2*PI to get into [-PI, PI]
  #
  # This ensures the Maclaurin series converges quickly (within 20 terms)
  # because the input magnitude is at most PI (~3.14).
  #
  # Examples:
  #   range_reduce(0)        => 0
  #   range_reduce(PI)       => PI  (or very close, at the boundary)
  #   range_reduce(3 * PI)   => PI  (3*PI is equivalent to PI)
  #   range_reduce(-PI/2)    => -PI/2
  #   range_reduce(1000*PI)  => ~0  (even multiples of PI map to ~0 or ~PI)

  def self.range_reduce(x)
    # Step 1: Reduce to [0, TWO_PI) using floating-point modulo.
    x = x % TWO_PI

    # Step 2: Shift from [0, 2*PI) to [-PI, PI].
    # Values in [0, PI] are already in range.
    # Values in (PI, 2*PI) get shifted down by 2*PI to land in (-PI, 0).
    x -= TWO_PI if x > PI

    x
  end

  private_class_method :range_reduce

  # ---------------------------------------------------------------------------
  # Sine via Maclaurin Series
  # ---------------------------------------------------------------------------
  #
  # The Maclaurin series for sin(x) is:
  #
  #   sin(x) = x - x^3/3! + x^5/5! - x^7/7! + ...
  #
  # Written with summation notation:
  #
  #   sin(x) = SUM_{n=0}^{infinity} (-1)^n * x^(2n+1) / (2n+1)!
  #
  # The pattern of signs alternates: +, -, +, -, ...
  # The powers are odd: 1, 3, 5, 7, ...
  # The factorials are odd: 1!, 3!, 5!, 7!, ...
  #
  # ## Iterative Term Update
  #
  # Instead of computing x^(2n+1) / (2n+1)! from scratch each time, we
  # derive each term from the previous one:
  #
  #   term_0 = x
  #   term_n = term_{n-1} * (-x^2) / ((2n) * (2n + 1))
  #
  # Why does this work? Let's verify:
  #
  #   term_{n-1} = (-1)^{n-1} * x^{2(n-1)+1} / (2(n-1)+1)!
  #              = (-1)^{n-1} * x^{2n-1} / (2n-1)!
  #
  #   term_{n-1} * (-x^2) / (2n * (2n+1))
  #     = (-1)^{n-1} * (-1) * x^{2n-1} * x^2 / ((2n-1)! * 2n * (2n+1))
  #     = (-1)^n * x^{2n+1} / (2n+1)!
  #     = term_n  ✓
  #
  # @param x [Numeric] angle in radians
  # @return [Float] the sine of x

  def self.sin(x)
    x = range_reduce(x.to_f)

    # Start with the first term of the series: term_0 = x
    term = x
    sum = term

    # Compute 19 more terms (for a total of 20). Each iteration multiplies
    # the previous term by -x^2 / (2n * (2n+1)), which:
    #   - flips the sign (the -x^2 introduces a factor of -1 each time)
    #   - increases the power by 2
    #   - divides by the next two factorial terms
    1.upto(19) do |n|
      term *= -x * x / ((2 * n) * (2 * n + 1))
      sum += term
    end

    sum
  end

  # ---------------------------------------------------------------------------
  # Cosine via Maclaurin Series
  # ---------------------------------------------------------------------------
  #
  # The Maclaurin series for cos(x) is:
  #
  #   cos(x) = 1 - x^2/2! + x^4/4! - x^6/6! + ...
  #
  # Written with summation notation:
  #
  #   cos(x) = SUM_{n=0}^{infinity} (-1)^n * x^(2n) / (2n)!
  #
  # The pattern of signs alternates: +, -, +, -, ...
  # The powers are even: 0, 2, 4, 6, ...
  # The factorials are even: 0!, 2!, 4!, 6!, ...
  #
  # ## Iterative Term Update
  #
  # Similar to sine, we compute each term from the previous one:
  #
  #   term_0 = 1
  #   term_n = term_{n-1} * (-x^2) / ((2n - 1) * (2n))
  #
  # Verification:
  #
  #   term_{n-1} = (-1)^{n-1} * x^{2(n-1)} / (2(n-1))!
  #              = (-1)^{n-1} * x^{2n-2} / (2n-2)!
  #
  #   term_{n-1} * (-x^2) / ((2n-1) * 2n)
  #     = (-1)^{n-1} * (-1) * x^{2n-2} * x^2 / ((2n-2)! * (2n-1) * 2n)
  #     = (-1)^n * x^{2n} / (2n)!
  #     = term_n  ✓
  #
  # @param x [Numeric] angle in radians
  # @return [Float] the cosine of x

  def self.cos(x)
    x = range_reduce(x.to_f)

    # Start with the first term of the series: term_0 = 1
    term = 1.0
    sum = term

    # Compute 19 more terms (for a total of 20).
    1.upto(19) do |n|
      term *= -x * x / ((2 * n - 1) * (2 * n))
      sum += term
    end

    sum
  end

  # ---------------------------------------------------------------------------
  # Degree/Radian Conversion
  # ---------------------------------------------------------------------------
  #
  # Angles can be measured in degrees or radians:
  #
  #   - Degrees: a full circle is 360 degrees
  #   - Radians: a full circle is 2*PI radians
  #
  # The conversion factor is: PI radians = 180 degrees
  #
  # Therefore:
  #   radians = degrees * PI / 180
  #   degrees = radians * 180 / PI

  # Converts degrees to radians.
  #
  # @param deg [Numeric] angle in degrees
  # @return [Float] angle in radians
  #
  # Examples:
  #   Trig.radians(0)   => 0.0
  #   Trig.radians(90)  => PI/2  (~1.5708)
  #   Trig.radians(180) => PI    (~3.1416)
  #   Trig.radians(360) => 2*PI  (~6.2832)

  def self.radians(deg)
    deg * PI / 180.0
  end

  # Converts radians to degrees.
  #
  # @param rad [Numeric] angle in radians
  # @return [Float] angle in degrees
  #
  # Examples:
  #   Trig.degrees(0)      => 0.0
  #   Trig.degrees(PI/2)   => 90.0
  #   Trig.degrees(PI)     => 180.0
  #   Trig.degrees(2 * PI) => 360.0

  def self.degrees(rad)
    rad * 180.0 / PI
  end
end
