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

  # ---------------------------------------------------------------------------
  # Square Root — Newton's (Babylonian) Method
  # ---------------------------------------------------------------------------
  #
  # Newton's method for sqrt is one of the oldest numerical algorithms
  # (Babylonian clay tablets, ~1700 BCE). The recurrence is:
  #
  #   next_guess = (guess + x / guess) / 2.0
  #
  # This has *quadratic convergence* — the number of correct decimal digits
  # doubles each iteration. For x = 2:
  #
  #   Iteration 0: guess = 2.0
  #   Iteration 1: guess = 1.5
  #   Iteration 2: guess ≈ 1.41667
  #   Iteration 3: guess ≈ 1.41422
  #   Iteration 4: guess ≈ 1.41421356237...  (full double precision!)
  #
  # @param x [Numeric] the radicand (must be >= 0)
  # @return [Float] the square root of x
  # @raise [ArgumentError] if x < 0

  def self.sqrt(x)
    x = x.to_f
    raise ArgumentError, "sqrt: domain error — input #{x} is negative" if x < 0

    # sqrt(0) is exactly 0.
    return 0.0 if x == 0.0

    # Initial guess: x itself works for x >= 1 (large values converge faster
    # from x than from 1). For x < 1, start at 1.0 to avoid tiny denominators.
    guess = x >= 1.0 ? x : 1.0

    # Iterate to convergence (or up to 60 steps as a safety cap).
    60.times do
      next_guess = (guess + x / guess) / 2.0

      # Stop when improvement is below the precision floor.
      # 1e-15 * guess is relative precision; 1e-300 handles subnormals.
      return next_guess if (next_guess - guess).abs < 1e-15 * guess + 1e-300

      guess = next_guess
    end

    guess
  end

  # ---------------------------------------------------------------------------
  # Tangent — Sine / Cosine
  # ---------------------------------------------------------------------------
  #
  # Tangent is defined as sin(x) / cos(x). On the unit circle, it is the
  # y-coordinate of where the angle's ray meets the vertical tangent line
  # at x = 1 — literally "the tangent."
  #
  # Poles: tan is undefined at x = π/2 + k·π where cos(x) = 0. We detect
  # |cos(x)| < 1e-15 and return a very large finite float to signal
  # near-singularity without raising a ZeroDivisionError.
  #
  # We call our own sin and cos here — no Math::sin or Math::cos.
  #
  # @param x [Numeric] angle in radians
  # @return [Float] tangent of x

  def self.tan(x)
    s = sin(x)  # our own sin
    c = cos(x)  # our own cos

    # Guard: at poles, cos is effectively zero.
    if c.abs < 1e-15
      return s > 0 ? 1.0e308 : -1.0e308
    end

    s / c
  end

  # ---------------------------------------------------------------------------
  # Private helper: range_reduce_for_atan — half-angle + Taylor series
  # ---------------------------------------------------------------------------

  # HALF_PI is π/2. Used in atan range reduction and atan2 quadrant cases.
  HALF_PI = PI / 2.0

  # atan_core computes atan(x) for |x| <= 1 using half-angle reduction
  # followed by the Taylor series. This is a private helper.
  #
  # Half-angle identity:
  #   atan(x) = 2·atan( x / (1 + sqrt(1 + x²)) )
  #
  # After reduction, |reduced| <= tan(π/8) ≈ 0.414, and the Taylor series
  # atan(t) = t - t³/3 + t⁵/5 - ... converges in ~15 terms.

  def self.atan_core(x)
    # Half-angle reduction: shrink |x| to |y| <= ~0.414.
    # We use our own sqrt — no Math::sqrt.
    reduced = x / (1.0 + sqrt(1.0 + x * x))

    # Taylor series: atan(t) = t - t³/3 + t⁵/5 - ...
    # Iterative form: term_n = term_{n-1} * (-t²) * (2n-1) / (2n+1)
    t = reduced
    t_sq = t * t
    term = t
    result = t

    1.upto(30) do |n|
      term = term * (-t_sq) * (2 * n - 1) / (2 * n + 1)
      result += term
      break if term.abs < 1e-17
    end

    # Undo the half-angle: atan(x) = 2·atan(reduced).
    2.0 * result
  end

  private_class_method :atan_core

  # ---------------------------------------------------------------------------
  # Arctangent — Inverse Tangent
  # ---------------------------------------------------------------------------
  #
  # atan(x) returns the angle θ ∈ (-π/2, π/2) such that tan(θ) = x.
  #
  # Range reduction for |x| > 1:
  #   atan(x)  = π/2 - atan(1/x)    for x > 1
  #   atan(x)  = -π/2 - atan(1/x)   for x < -1
  #
  # This identity holds because tan(π/2 - θ) = cot(θ) = 1/tan(θ).
  # So if tan(θ) = x, then tan(π/2 - θ) = 1/x, meaning atan(1/x) = π/2 - θ.
  #
  # @param x [Numeric] the value whose arctangent to compute
  # @return [Float] angle in radians, in (-π/2, π/2)

  def self.atan(x)
    x = x.to_f
    return 0.0 if x == 0.0

    if x > 1.0
      return HALF_PI - atan_core(1.0 / x)
    end
    if x < -1.0
      return -HALF_PI - atan_core(1.0 / x)
    end

    atan_core(x)
  end

  # ---------------------------------------------------------------------------
  # Two-Argument Arctangent — Four-Quadrant Inverse Tangent
  # ---------------------------------------------------------------------------
  #
  # atan2(y, x) returns the angle in (-π, π] that the vector (x, y) makes
  # with the positive x-axis. Unlike atan(y/x), it correctly handles all
  # four quadrants by inspecting the signs of x and y separately.
  #
  # Why atan(y/x) is insufficient:
  #   atan(-1 / 1) = -π/4   (Q4 — correct)
  #   atan(-1 / -1) = atan(1) = π/4   (but the point (-1, -1) is in Q3 — WRONG)
  #
  # Quadrant map:
  #
  #         y > 0
  #     Q2  |  Q1       returns ∈ (0,    π]  for y >= 0
  #   ------+------  x  returns ∈ (-π,   0)  for y <  0
  #     Q3  |  Q4
  #         y < 0
  #
  # @param y [Numeric] the y-coordinate (numerator)
  # @param x [Numeric] the x-coordinate (denominator)
  # @return [Float] angle in radians, in (-π, π]

  def self.atan2(y, x)
    y = y.to_f
    x = x.to_f

    if x > 0.0
      return atan(y / x)
    end
    if x < 0.0 && y >= 0.0
      return atan(y / x) + PI
    end
    if x < 0.0 && y < 0.0
      return atan(y / x) - PI
    end
    if x == 0.0 && y > 0.0
      return HALF_PI
    end
    if x == 0.0 && y < 0.0
      return -HALF_PI
    end
    # Both zero: undefined by convention, return 0.
    0.0
  end
end
