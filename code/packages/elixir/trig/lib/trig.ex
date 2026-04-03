defmodule Trig do
  @moduledoc """
  # Trig — Trigonometric Functions from First Principles

  This module implements sine, cosine, and angle conversion functions
  using nothing but basic arithmetic. No `:math` module, no external
  dependencies — just the Maclaurin series (Taylor series centered at 0).

  ## Why from scratch?

  Building trig functions from first principles teaches you:
  1. How Taylor/Maclaurin series approximate transcendental functions
  2. Why range reduction matters for numerical stability
  3. How computers actually compute sin/cos under the hood (CORDIC aside)

  ## The Maclaurin Series

  The Maclaurin series is a Taylor series expanded around x = 0.
  For sine and cosine:

      sin(x) = x - x^3/3! + x^5/5! - x^7/7! + ...
             = sum_{n=0}^{inf} (-1)^n * x^(2n+1) / (2n+1)!

      cos(x) = 1 - x^2/2! + x^4/4! - x^6/6! + ...
             = sum_{n=0}^{inf} (-1)^n * x^(2n) / (2n)!

  Each successive term gets smaller (for |x| < pi), so after enough
  terms, the partial sum converges to the true value.

  ## Range Reduction

  The Maclaurin series converges fastest when x is near 0. For large
  inputs (like sin(1000)), the terms start huge before cancelling out,
  causing floating-point errors. Range reduction maps any input into
  [-pi, pi] using the periodicity of sin and cos (period = 2*pi).

  ## Layer

  This is a PHY00 (physics layer 0) leaf package with no dependencies.
  """

  # ---------------------------------------------------------------------------
  # Constants
  # ---------------------------------------------------------------------------

  # Pi to double-precision accuracy (IEEE 754 float64).
  # This is the ratio of a circle's circumference to its diameter,
  # the most fundamental constant in trigonometry.
  @pi 3.141592653589793

  # Two pi — one full revolution in radians. Used for range reduction.
  @two_pi 2.0 * @pi

  # Number of terms in the Maclaurin series. 20 terms gives us
  # roughly 15-16 digits of precision for inputs in [-pi, pi],
  # which matches IEEE 754 double precision.
  @terms 20

  @doc """
  Returns pi to double-precision accuracy.

  ## Examples

      iex> Trig.pi()
      3.141592653589793
  """
  def pi, do: @pi

  # ---------------------------------------------------------------------------
  # Range Reduction (private)
  # ---------------------------------------------------------------------------

  # Range reduction normalizes an angle to the interval [-pi, pi].
  #
  # Why [-pi, pi] and not [0, 2*pi]? Because the Maclaurin series
  # converges fastest near 0. By centering our interval on 0, we
  # minimize the magnitude of x, which means:
  #   - Fewer terms needed for convergence
  #   - Less floating-point cancellation error
  #
  # The algorithm:
  #   1. Compute how many full rotations (2*pi) fit in x
  #   2. Subtract those full rotations
  #   3. If the result is still outside [-pi, pi], adjust by one more 2*pi
  #
  # Example: sin(7.0)
  #   7.0 / (2*pi) = 1.114... so subtract 1 full rotation
  #   7.0 - 2*pi = 0.717... which is in [-pi, pi]. Done!

  defp range_reduce(x) when is_number(x) do
    # Subtract multiples of 2*pi to get into roughly [-2*pi, 2*pi]
    reduced = x - Float.round(x / @two_pi, 0) * @two_pi

    # Clamp to [-pi, pi] if we overshot
    cond do
      reduced > @pi  -> reduced - @two_pi
      reduced < -@pi -> reduced + @two_pi
      true           -> reduced
    end
  end

  # ---------------------------------------------------------------------------
  # sin(x) — Maclaurin Series
  # ---------------------------------------------------------------------------

  @doc """
  Computes the sine of `x` (in radians) using the Maclaurin series.

  ## How it works

  The Maclaurin series for sine is:

      sin(x) = x - x^3/3! + x^5/5! - x^7/7! + ...

  Rather than computing each term from scratch (which would involve
  recomputing large powers and factorials), we use an iterative trick:

      term_n+1 = term_n * (-x^2) / ((2n+2) * (2n+3))

  This works because:
  - Each new term introduces two more factors of x (hence x^2)
  - The sign alternates (hence the negative)
  - The factorial grows by two new factors: (2n+2) and (2n+3)

  Starting from term_0 = x, we multiply by this ratio to get each
  successive term. This avoids overflow from large intermediate values.

  ## Examples

      iex> Trig.sin(0)
      0.0

      iex> Trig.sin(Trig.pi() / 2)  # approximately 1.0
      1.0

  """
  def sin(x) when is_number(x) do
    # Step 1: Range reduce to [-pi, pi] for best convergence
    reduced = range_reduce(x)

    # Step 2: The first term of the sin series is just x itself
    first_term = reduced / 1.0

    # Step 3: Accumulate @terms terms using Enum.reduce
    #
    # We track {current_term, running_sum} through each iteration.
    # At each step n (1-based), we compute the ratio to get the next term:
    #
    #   ratio = -x^2 / (2n * (2n + 1))
    #
    # This ratio comes from:
    #   term_{n} / term_{n-1} = (-1) * x^2 / ((2n) * (2n+1))
    #
    # where 2n and 2n+1 are the next two factorial denominators.

    {_final_term, sum} =
      Enum.reduce(1..(@terms - 1), {first_term, first_term}, fn n, {term, acc} ->
        denominator = 2 * n * (2 * n + 1)
        next_term = term * (-reduced * reduced) / denominator
        {next_term, acc + next_term}
      end)

    sum
  end

  # ---------------------------------------------------------------------------
  # cos(x) — Maclaurin Series
  # ---------------------------------------------------------------------------

  @doc """
  Computes the cosine of `x` (in radians) using the Maclaurin series.

  ## How it works

  The Maclaurin series for cosine is:

      cos(x) = 1 - x^2/2! + x^4/4! - x^6/6! + ...

  Similar to sin, we use the iterative term trick:

      term_n+1 = term_n * (-x^2) / ((2n) * (2n+1))

  But the indexing is slightly different because cosine starts with
  even powers (0, 2, 4, ...) while sine starts with odd powers (1, 3, 5, ...).

  Starting from term_0 = 1, each successive term multiplies by:

      ratio = -x^2 / ((2n-1) * (2n))

  where n goes from 1 to @terms-1.

  ## Examples

      iex> Trig.cos(0)
      1.0

      iex> Trig.cos(Trig.pi())  # approximately -1.0
      -1.0

  """
  def cos(x) when is_number(x) do
    # Step 1: Range reduce to [-pi, pi]
    reduced = range_reduce(x)

    # Step 2: The first term of the cos series is 1.0
    first_term = 1.0

    # Step 3: Accumulate terms.
    #
    # For cosine, the ratio between successive terms is:
    #
    #   term_{n} / term_{n-1} = -x^2 / ((2n-1) * 2n)
    #
    # where n starts at 1. The denominators are:
    #   n=1: 1*2 = 2   (giving -x^2/2! )
    #   n=2: 3*4 = 12  (giving +x^4/4! )
    #   n=3: 5*6 = 30  (giving -x^6/6! )
    #   ...and so on.

    {_final_term, sum} =
      Enum.reduce(1..(@terms - 1), {first_term, first_term}, fn n, {term, acc} ->
        denominator = (2 * n - 1) * (2 * n)
        next_term = term * (-reduced * reduced) / denominator
        {next_term, acc + next_term}
      end)

    sum
  end

  # ---------------------------------------------------------------------------
  # Angle Conversion
  # ---------------------------------------------------------------------------

  @doc """
  Converts degrees to radians.

  The conversion factor is pi/180, since a full circle is 360 degrees
  or 2*pi radians:

      radians = degrees * pi / 180

  ## Examples

      iex> Trig.radians(180)
      3.141592653589793

      iex> Trig.radians(90)
      1.5707963267948966

      iex> Trig.radians(0)
      0.0
  """
  def radians(deg) when is_number(deg) do
    deg * @pi / 180.0
  end

  @doc """
  Converts radians to degrees.

  The conversion factor is 180/pi, the inverse of `radians/1`:

      degrees = radians * 180 / pi

  ## Examples

      iex> Trig.degrees(3.141592653589793)
      180.0

      iex> Trig.degrees(1.5707963267948966)
      90.0

      iex> Trig.degrees(0)
      0.0
  """
  def degrees(rad) when is_number(rad) do
    rad * 180.0 / @pi
  end

  # ---------------------------------------------------------------------------
  # sqrt/1 — Newton's (Babylonian) Method
  # ---------------------------------------------------------------------------

  @doc """
  Computes the square root of `x` using Newton's iterative method.

  ## The Algorithm

  Newton's method (also called the Babylonian method — Babylonian mathematicians
  used it over 3,000 years ago) says: if `guess` approximates sqrt(x), then:

      next_guess = (guess + x / guess) / 2.0

  is a better approximation. The convergence is *quadratic*: the number of
  correct digits doubles each iteration. Convergence for sqrt(2):

      iter | guess
      -----|---------------
      0    | 2.0
      1    | 1.5
      2    | 1.41667
      3    | 1.41422
      4    | 1.41421356237...   (full precision!)

  ## Guard

  Negative inputs raise an `ArithmeticError` — the real square root is only
  defined for non-negative numbers.

  ## Examples

      iex> Trig.sqrt(0)
      0.0

      iex> Trig.sqrt(4)
      2.0

      iex> abs(Trig.sqrt(2) - 1.41421356237) < 1.0e-10
      true

  """
  def sqrt(x) when is_number(x) and x < 0 do
    raise ArithmeticError, "sqrt: domain error — input #{x} is negative"
  end

  def sqrt(x) when is_number(x) do
    x = x / 1.0  # ensure float

    # sqrt(0) is exactly 0.
    if x == 0.0 do
      0.0
    else
      # Initial guess: x itself for x >= 1, else 1.0.
      guess = if x >= 1.0, do: x, else: 1.0

      # Iterate up to 60 times — quadratic convergence means ~15 in practice.
      sqrt_iterate(x, guess, 0)
    end
  end

  # Private recursive helper for sqrt Newton iterations.
  defp sqrt_iterate(_x, guess, 60), do: guess

  defp sqrt_iterate(x, guess, iteration) do
    next_guess = (guess + x / guess) / 2.0
    improvement = abs(next_guess - guess)

    # Stop when improvement is below the precision floor.
    # 1.0e-15 * guess gives relative precision; 1.0e-300 handles subnormals.
    if improvement < 1.0e-15 * guess + 1.0e-300 do
      next_guess
    else
      sqrt_iterate(x, next_guess, iteration + 1)
    end
  end

  # ---------------------------------------------------------------------------
  # tan/1 — Tangent as Sine / Cosine
  # ---------------------------------------------------------------------------

  @doc """
  Computes the tangent of `x` (in radians).

  ## Definition

  Tangent is the ratio of sine to cosine:

      tan(x) = sin(x) / cos(x)

  On the unit circle, this is the y-coordinate where the ray at angle x
  meets the vertical line x=1 — the literal "tangent line" to the circle.

  ## Undefined Points (Poles)

  tan is undefined at x = π/2 + k·π for any integer k, where cos(x) = 0.
  When |cos(x)| < 1.0e-15 we return ±1.0e308 (the largest finite float)
  to signal near-singularity without crashing.

  We call our own `sin/1` and `cos/1` — no `:math.tan` used.

  ## Examples

      iex> abs(Trig.tan(0)) < 1.0e-10
      true

      iex> abs(Trig.tan(Trig.pi() / 4) - 1.0) < 1.0e-10
      true

  """
  def tan(x) when is_number(x) do
    s = sin(x)  # our own sin
    c = cos(x)  # our own cos

    # Guard against poles: |cos| < 1e-15 means we're near a discontinuity.
    if abs(c) < 1.0e-15 do
      if s > 0, do: 1.0e308, else: -1.0e308
    else
      s / c
    end
  end

  # ---------------------------------------------------------------------------
  # Constants for atan
  # ---------------------------------------------------------------------------

  # HALF_PI is π/2. Used in atan's range reduction and atan2's quadrant cases.
  @half_pi @pi / 2.0

  # ---------------------------------------------------------------------------
  # atan/1 — Arctangent via Taylor Series with Half-Angle Reduction
  # ---------------------------------------------------------------------------

  @doc """
  Computes the arctangent of `x` (in radians).

  Returns a value in the open interval (-π/2, π/2).

  ## Range Reduction

  The Taylor series atan(x) = x - x³/3 + x⁵/5 - ... converges only for
  |x| <= 1. For |x| > 1 we apply the complementary identity:

      atan(x)  = π/2 - atan(1/x)    for x > 1
      atan(x)  = -π/2 - atan(1/x)   for x < -1

  Inside the core computation, a half-angle reduction further halves the
  argument, ensuring fast convergence in ~15 Taylor terms.

  ## Examples

      iex> abs(Trig.atan(0)) < 1.0e-10
      true

      iex> abs(Trig.atan(1) - Trig.pi() / 4) < 1.0e-10
      true

      iex> abs(Trig.atan(-1) + Trig.pi() / 4) < 1.0e-10
      true

  """
  def atan(x) when is_number(x) do
    x = x / 1.0

    cond do
      x == 0.0  -> 0.0
      x >  1.0  -> @half_pi - atan_core(1.0 / x)
      x < -1.0  -> -@half_pi - atan_core(1.0 / x)
      true      -> atan_core(x)
    end
  end

  # Private helper: atan_core computes atan for |x| <= 1 via half-angle + Taylor.
  #
  # Half-angle identity:
  #   atan(x) = 2 * atan( x / (1 + sqrt(1 + x^2)) )
  #
  # This brings |x| <= 1 down to |y| <= tan(pi/8) ~= 0.414, where the Taylor
  # series converges rapidly.
  defp atan_core(x) do
    # Half-angle reduction. We use our own sqrt/1.
    reduced = x / (1.0 + sqrt(1.0 + x * x))

    t = reduced
    t_sq = t * t

    # Taylor series with iterative term computation.
    # term_n = term_{n-1} * (-t^2) * (2n-1) / (2n+1)
    {_term, result} =
      Enum.reduce_while(1..30, {t, t}, fn n, {term, acc} ->
        next_term = term * (-t_sq) * (2 * n - 1) / (2 * n + 1)
        next_acc = acc + next_term

        # Early exit when term is negligibly small.
        if abs(next_term) < 1.0e-17 do
          {:halt, {next_term, next_acc}}
        else
          {:cont, {next_term, next_acc}}
        end
      end)

    # Undo the half-angle halving: atan(x) = 2 * atan(reduced).
    2.0 * result
  end

  # ---------------------------------------------------------------------------
  # atan2/2 — Four-Quadrant Arctangent
  # ---------------------------------------------------------------------------

  @doc """
  Computes the four-quadrant arctangent of (`y`, `x`).

  Returns the angle in radians that the vector (x, y) makes with the positive
  x-axis, in the range (-π, π].

  ## Why Not atan(y/x)?

  `atan(y/x)` only gives angles in (-π/2, π/2). It cannot distinguish
  Q1 from Q3 or Q2 from Q4 — both pairs produce the same y/x ratio.
  `atan2` inspects the signs of y and x separately:

      Quadrant I   (x>0, y>0):  atan2 ∈ (0,   π/2)
      Quadrant II  (x<0, y≥0):  atan2 ∈ [π/2,  π ]
      Quadrant III (x<0, y<0):  atan2 ∈ (-π, -π/2)
      Quadrant IV  (x>0, y<0):  atan2 ∈ (-π/2,  0)

  ## Examples

      iex> abs(Trig.atan2(0, 1)) < 1.0e-10      # positive x-axis
      true

      iex> abs(Trig.atan2(1, 0) - Trig.pi() / 2) < 1.0e-10   # positive y-axis
      true

      iex> abs(Trig.atan2(0, -1) - Trig.pi()) < 1.0e-10      # negative x-axis
      true

  """
  def atan2(y, x) when is_number(y) and is_number(x) do
    y = y / 1.0
    x = x / 1.0

    cond do
      x > 0.0                  -> atan(y / x)
      x < 0.0 and y >= 0.0    -> atan(y / x) + @pi
      x < 0.0 and y < 0.0     -> atan(y / x) - @pi
      x == 0.0 and y > 0.0    -> @half_pi
      x == 0.0 and y < 0.0    -> -@half_pi
      true                     -> 0.0   # both zero: undefined, return 0
    end
  end
end
