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
end
