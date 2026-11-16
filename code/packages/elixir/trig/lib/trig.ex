defmodule Trig do
  @moduledoc """
  # Trig: trigonometric functions from first principles (TRIG-REF-1)

  This module computes `sin`, `cos`, `tan`, the degree/radian conversions and
  (Part II) `sqrt`, `atan` and `atan2` using nothing but `+ - * /` on IEEE 754
  doubles.  No `:math` call is made anywhere.  It is the Elixir lane of the
  reference algorithm TRIG-REF-1 specified in `code/specs/PHY00a-trig-convergence.md`;
  every result is **bit for bit** the one the other sixteen lanes return, and
  the shared vectors in `code/specs/fixtures/trig-reference-v1` prove it.

  ## The idea in one picture

  A sine is a repeating wave.  Knowing `sin` on [-pi/4, pi/4] and which
  quarter-turn you are in is enough to know it everywhere.  So each call does
  three things:

      x ──► how many quarter-turns fit?  k = nearest integer to x / (pi/2)
        └─► what is left over?           r = x - k * pi/2,   |r| <= ~pi/4
        └─► a short polynomial in r, picked by q = k mod 4

  The polynomial is the Maclaurin series (Taylor series at 0):

      sin(r) = r - r^3/3! + r^5/5! - r^7/7! + ...
      cos(r) = 1 - r^2/2! + r^4/4! - r^6/6! + ...

  but with the factorials baked into a table of coefficients, evaluated by
  Horner's rule.  That needs no division, adds the smallest terms first (the
  accurate order) and cannot be regrouped by anybody.

  ## Arithmetic model (why the code is written one operation per line)

  Erlang floats are IEEE binary64 and the BEAM does not fuse a multiply and an
  add into one rounding (no FMA) nor keep extended-precision intermediates.
  So `a * b + c` rounds twice, exactly like the reference.  Each statement
  below is one IEEE operation, in the order the specification writes it.

  ## The domain, and what the BEAM does about NaN

  `sin`, `cos` and `tan` are defined for `|x| <= 2^29` (about 5.4e8).  Beyond
  that the reduction would need pi to over a thousand bits.  The reference
  returns NaN there (and for NaN and infinity inputs).  **A BEAM float cannot
  be NaN or infinite**: arithmetic that would produce one raises
  `ArithmeticError`.  So here "returns NaN" means *raises `ArithmeticError`*
  (PHY00a 3.3, 5, OQ-10), by the same single comparison the reference uses.
  The same convention covers every other non-finite result, for example
  `degrees(1.7e308)`, and `atan2` when `y / x` overflows although the true
  result is finite (the division raises).

  ## Layer

  PHY00 (physics layer 0), a leaf package with no dependencies.
  """

  import Bitwise

  # ---------------------------------------------------------------------------
  # Constants, written by BIT PATTERN (PHY00a 3.2)
  # ---------------------------------------------------------------------------
  #
  # A decimal literal is only as exact as the parser that reads it.  The spec
  # pins each constant by its 64 bits, and Erlang binaries can state those bits
  # directly: `<<f::float-64>> = <<bits::64>>` reinterprets eight bytes as a
  # double with no rounding.  `bits` below is that cast as a throwaway function
  # used at compile time, so every attribute is the exact double.

  bits = fn b ->
    <<f::float-64>> = <<b::64>>
    f
  end

  # pi, 2*pi, pi/2: the values the package has always exposed.
  @pi bits.(0x400921FB54442D18)
  @two_pi bits.(0x401921FB54442D18)
  @half_pi bits.(0x3FF921FB54442D18)

  # Reduction constants.  INV_PIO2 is the double nearest 2/pi.  PIO2_1..4 are
  # the first 96 bits of pi/2 in four chunks of at most 24 significant bits, so
  # the product k * PIO2_i is EXACT whenever |k| < 2^29 (24 + 29 = 53 bits).
  # PIO2_5 is the remainder rounded to 53 bits.  (Cody and Waite, 1980.)
  @inv_pio2 bits.(0x3FE45F306DC9C883)
  @pio2_1 bits.(0x3FF921FB40000000)
  @pio2_2 bits.(0x3E74442D00000000)
  @pio2_3 bits.(0x3CF8469880000000)
  @pio2_4 bits.(0x3B78CC5160000000)
  @pio2_5 bits.(0x39F01B839A25204A)

  # MAGIC = 1.5 * 2^52: adding it pushes the fraction bits of t off the end of
  # the significand, so (t + MAGIC) - MAGIC is t rounded to the nearest whole
  # number (ties to even) using only an add and a subtract.
  @magic bits.(0x4338000000000000)
  # LIMIT = 2^29 and TINY = 2^-27.
  @limit bits.(0x41C0000000000000)
  @tiny bits.(0x3E40000000000000)

  # Sine coefficients S[n] = (-1)^n / (2n+1)!, rounded once from the exact
  # rational.  Read the table as the Maclaurin series: -1/3!, +1/5!, ...
  @s1 bits.(0xBFC5555555555555)
  @s2 bits.(0x3F81111111111111)
  @s3 bits.(0xBF2A01A01A01A01A)
  @s4 bits.(0x3EC71DE3A556C734)
  @s5 bits.(0xBE5AE64567F544E4)
  @s6 bits.(0x3DE6124613A86D09)
  @s7 bits.(0xBD6AE7F3E733B81F)
  @s8 bits.(0x3CE952C77030AD4A)
  @s9 bits.(0xBC62F49B46814157)

  # Cosine coefficients C[n] = (-1)^n / (2n)!  (n = 1 is -1/2, applied as
  # 0.5 * z, which is exact).  Note C4 = 1/8! and S3 = 1/7! share a significand
  # (their ratio is 8), as do C8 and S7.
  @c2 bits.(0x3FA5555555555555)
  @c3 bits.(0xBF56C16C16C16C17)
  @c4 bits.(0x3EFA01A01A01A01A)
  @c5 bits.(0xBE927E4FB7789F5C)
  @c6 bits.(0x3E21EED8EFF8D898)
  @c7 bits.(0xBDA93974A8C07C9D)
  @c8 bits.(0x3D2AE7F3E733B81F)
  @c9 bits.(0xBCA6827863B97D97)

  # Degree conversion: one multiply by a stored constant (the doubles nearest
  # pi/180 and 180/pi).
  @rad_per_deg bits.(0x3F91DF46A2529D39)
  @deg_per_rad bits.(0x404CA5DC1A63C1F8)

  # tan and Part II thresholds: 1e-15, 1e308, 1e-17, 1e-300.
  @pole_eps bits.(0x3CD203AF9EE75616)
  @pole_big bits.(0x7FE1CCF385EBC8A0)
  @atan_term bits.(0x3C670EF54646D497)
  @sqrt_rel bits.(0x3CD203AF9EE75616)
  @sqrt_tiny bits.(0x01A56E1FC2F8F359)

  @doc """
  Returns pi to double-precision accuracy.

  ## Examples

      iex> Trig.pi()
      3.141592653589793
  """
  def pi, do: @pi

  @doc """
  Returns every constant of PHY00a 3.2 as a map from its specification name
  (`"PI"`, `"S1"`, `"PIO2_3"`, ...) to its float.  The conformance test
  compares the bits of each against the shared manifest (PHY00a 6.5 (e)).
  """
  def constants do
    %{
      "PI" => @pi,
      "TWO_PI" => @two_pi,
      "HALF_PI" => @half_pi,
      "INV_PIO2" => @inv_pio2,
      "PIO2_1" => @pio2_1,
      "PIO2_2" => @pio2_2,
      "PIO2_3" => @pio2_3,
      "PIO2_4" => @pio2_4,
      "PIO2_5" => @pio2_5,
      "MAGIC" => @magic,
      "LIMIT" => @limit,
      "TINY" => @tiny,
      "S1" => @s1,
      "S2" => @s2,
      "S3" => @s3,
      "S4" => @s4,
      "S5" => @s5,
      "S6" => @s6,
      "S7" => @s7,
      "S8" => @s8,
      "S9" => @s9,
      "C2" => @c2,
      "C3" => @c3,
      "C4" => @c4,
      "C5" => @c5,
      "C6" => @c6,
      "C7" => @c7,
      "C8" => @c8,
      "C9" => @c9,
      "RAD_PER_DEG" => @rad_per_deg,
      "DEG_PER_RAD" => @deg_per_rad,
      "TAN_POLE" => @pole_eps,
      "TAN_BIG" => @pole_big,
      "ATAN_TERM" => @atan_term,
      "SQRT_REL" => @sqrt_rel,
      "SQRT_TINY" => @sqrt_tiny
    }
  end

  # ---------------------------------------------------------------------------
  # Range reduction (private): x = k * (pi/2) + r
  # ---------------------------------------------------------------------------
  #
  # Step 1, the quotient.  t = x * (2/pi) is roughly "how many quarter-turns".
  # (t + MAGIC) - MAGIC rounds t to the nearest integer k, as a float, with no
  # floor/round/trunc call.  It is valid for |t| < 2^51; the domain guard has
  # already ensured |t| < 3.5e8.
  #
  # Step 2, the remainder, by Cody-Waite subtraction.  pi/2 is irrational, so
  # k * (pi/2) cannot be formed exactly.  Instead pi/2 is stored as a SUM of
  # five pieces; the first four are so short (24 bits) that k * piece is exact:
  #
  #    x ------------------------------------------------------- the angle
  #    |-- k*PIO2_1 --|                                           24 bits of k*pi/2
  #                   |-- k*PIO2_2 --|                            next 24
  #                                  |-- k*PIO2_3 --|             next 24
  #                                                 |- k*PIO2_4 -|  next 24
  #                                                              |k*PIO2_5| rounds
  #    r = what is left, |r| <= pi/4 (plus a few ulps)
  #
  # The first two subtractions are exact (Sterbenz); the last three round, each
  # by at most half an ulp of the current r.  Every line below is ONE IEEE
  # operation: the product and the difference are separate statements, because
  # a fused multiply-subtract would round differently.
  defp reduce(x) do
    t = x * @inv_pio2
    k = t + @magic
    k = k - @magic

    r = x - k * @pio2_1
    r = r - k * @pio2_2
    r = r - k * @pio2_3
    r = r - k * @pio2_4
    r = r - k * @pio2_5

    {k, r}
  end

  # Step 3, the quadrant.  k is an integer-valued float with |k| < 2^29, so
  # trunc(k) is exact, and `&&& 3` on a two's-complement integer is the floor
  # modulus 4 (k = -1 gives 3, k = -5 gives 3).  The only integer operation.
  defp quadrant(k), do: trunc(k) &&& 3

  # ---------------------------------------------------------------------------
  # The series kernels, by Horner's rule (PHY00a 3.5)
  # ---------------------------------------------------------------------------
  #
  #   sin(r) = r + r*z*(S1 + S2 z + S3 z^2 + ... + S9 z^8),        z = r*r
  #   cos(r) = (1 - z/2) + z^2*(C2 + C3 z + ... + C9 z^7)
  #
  # The leading term is added LAST and unrounded inside the polynomial, so the
  # dominant part of the answer carries one rounding.  Ten terms: the first
  # omitted one (r^21/21!) is far below half an ulp.
  #
  #   n   | sin coefficient | cos coefficient
  #   ----|-----------------|----------------
  #   1   | -1/3!           | -1/2!  (the 0.5 * z)
  #   2   | +1/5!           | +1/4!
  #   3   | -1/7!           | -1/6!
  #   ...
  #   9   | -1/19!          | -1/18!
  defp sin_kernel(r) do
    z = r * r
    p = @s9
    p = p * z
    p = p + @s8
    p = p * z
    p = p + @s7
    p = p * z
    p = p + @s6
    p = p * z
    p = p + @s5
    p = p * z
    p = p + @s4
    p = p * z
    p = p + @s3
    p = p * z
    p = p + @s2
    p = p * z
    p = p + @s1
    w = r * z
    w = w * p
    r + w
  end

  defp cos_kernel(r) do
    z = r * r
    q = @c9
    q = q * z
    q = q + @c8
    q = q * z
    q = q + @c7
    q = q * z
    q = q + @c6
    q = q * z
    q = q + @c5
    q = q * z
    q = q + @c4
    q = q * z
    q = q + @c3
    q = q * z
    q = q + @c2
    u = 0.5 * z
    u = 1.0 - u
    v = z * z
    v = v * q
    u + v
  end

  # ---------------------------------------------------------------------------
  # sin(x) and cos(x)  (PHY00a 3.6)
  # ---------------------------------------------------------------------------

  @doc """
  Computes the sine of `x` (radians).

  Integers are accepted and converted once with `x * 1.0` (never `x + 0.0`,
  which would turn `-0.0` into `+0.0`).

  ## Domain

  Defined for `|x| <= 2^29`.  Outside it the reference result is NaN, which a
  BEAM float cannot hold, so this raises `ArithmeticError`.

  ## How it works

      quadrant q = k mod 4      sin(x)
      0                          sinK(r)
      1                          cosK(r)
      2                         -sinK(r)
      3                         -cosK(r)

  and `sin(x) = x` exactly for `|x| < 2^-27` (including `-0.0` and every
  subnormal), because the next term `x^3/6` is under half an ulp of `x`.

  ## Examples

      iex> Trig.sin(0)
      0.0

      iex> Trig.sin(1.0)
      0.8414709848078965
  """
  def sin(x) when is_number(x) do
    x = x * 1.0
    check_domain(x)

    if abs(x) < @tiny do
      x
    else
      {k, r} = reduce(x)

      case quadrant(k) do
        0 -> sin_kernel(r)
        1 -> cos_kernel(r)
        2 -> -sin_kernel(r)
        3 -> -cos_kernel(r)
      end
    end
  end

  @doc """
  Computes the cosine of `x` (radians).  Same domain and conversion rules as
  `sin/1`.

      quadrant q = k mod 4      cos(x)
      0                          cosK(r)
      1                         -sinK(r)
      2                         -cosK(r)
      3                          sinK(r)

  `cos(x) = 1.0` exactly for `|x| < 2^-27`.

  ## Examples

      iex> Trig.cos(0)
      1.0

      iex> Trig.cos(Trig.pi())
      -1.0
  """
  def cos(x) when is_number(x) do
    x = x * 1.0
    check_domain(x)

    if abs(x) < @tiny do
      1.0
    else
      {k, r} = reduce(x)

      case quadrant(k) do
        0 -> cos_kernel(r)
        1 -> -sin_kernel(r)
        2 -> -cos_kernel(r)
        3 -> sin_kernel(r)
      end
    end
  end

  # The domain guard.  The reference writes it as `not (|x| <= LIMIT)`: one
  # comparison that also catches NaN and the infinities.  A BEAM float is never
  # NaN or infinite, so the negated form is not needed here to catch them, but
  # the comparison is kept in the reference's shape.  "Returns NaN" becomes
  # "raises ArithmeticError" (PHY00a 3.3, 5).
  defp check_domain(x) do
    if abs(x) <= @limit do
      :ok
    else
      raise ArithmeticError,
            "Trig: |x| > 2^29 is outside the domain of sin/cos/tan (TRIG-REF-1 returns NaN here)"
    end
  end

  # ---------------------------------------------------------------------------
  # Angle conversion (PHY00a 3.7): one multiply by a stored constant
  # ---------------------------------------------------------------------------

  @doc """
  Converts degrees to radians with a single multiply by the double nearest
  pi/180: one rounding, no overflowing `d * pi` intermediate, `-0.0` keeps its
  sign.  A product above the largest finite float would be `+-inf`, which a
  BEAM float cannot hold, so the multiply raises `ArithmeticError`.

  ## Examples

      iex> Trig.radians(180)
      3.141592653589793

      iex> Trig.radians(0)
      0.0
  """
  def radians(deg) when is_number(deg) do
    deg = deg * 1.0
    deg * @rad_per_deg
  end

  @doc """
  Converts radians to degrees with a single multiply by the double nearest
  180/pi.  `degrees(1.7e308)` would be `+inf`, so it raises `ArithmeticError`.

  ## Examples

      iex> Trig.degrees(3.141592653589793)
      180.0

      iex> Trig.degrees(0)
      0.0
  """
  def degrees(rad) when is_number(rad) do
    rad = rad * 1.0
    rad * @deg_per_rad
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
    # ensure float
    x = x / 1.0

    # sqrt(0) is exactly 0.
    if x == 0.0 do
      x
    else
      {scaled, result_scale} = normalize_sqrt(x, 1.0)
      guess = if scaled >= 1.0, do: scaled, else: 1.0

      # Iterate up to 60 times — quadratic convergence means ~15 in practice.
      sqrt_iterate(scaled, guess, 0) * result_scale
    end
  end

  defp normalize_sqrt(x, scale) when x < 0.25,
    do: normalize_sqrt(x * 4.0, scale * 0.5)

  defp normalize_sqrt(x, scale) when x >= 4.0,
    do: normalize_sqrt(x * 0.25, scale * 2.0)

  defp normalize_sqrt(x, scale), do: {x, scale}

  # Private recursive helper for sqrt Newton iterations.
  defp sqrt_iterate(_x, guess, 60), do: guess

  defp sqrt_iterate(x, guess, iteration) do
    next_guess = (guess + x / guess) / 2.0
    improvement = abs(next_guess - guess)

    # Stop when improvement is below the precision floor.
    # 1.0e-15 * guess gives relative precision; 1.0e-300 handles subnormals.
    if improvement < @sqrt_rel * guess + @sqrt_tiny do
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
  When |cos(x)| < 1.0e-15 we return the sentinel ±1.0e308 (not ±inf, which a BEAM float cannot be)
  to signal near-singularity without crashing.

  We call our own `sin/1` and `cos/1` — no `:math.tan` used.

  ## Examples

      iex> abs(Trig.tan(0)) < 1.0e-10
      true

      iex> abs(Trig.tan(Trig.pi() / 4) - 1.0) < 1.0e-10
      true

  """
  def tan(x) when is_number(x) do
    # sin/1 and cos/1 raise beyond 2^29, so the domain is tested first, as the
    # specification requires of a language that raises on NaN.
    s = sin(x)
    c = cos(x)

    # Guard against poles: |cos| < 1e-15 means we're near a discontinuity.
    # This pinned legacy sentinel is not an approximation (PHY00a 3.8).  The
    # guard makes c = 0 unreachable in the division.
    if abs(c) < @pole_eps do
      if s > 0, do: @pole_big, else: -@pole_big
    else
      s / c
    end
  end

  # ---------------------------------------------------------------------------
  # Constants for atan
  # ---------------------------------------------------------------------------

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
      # atan(x) rounds exactly to x here; avoid halving subnormals and retain -0.
      abs(x) <= @tiny -> x
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
        if abs(next_term) < @atan_term do
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
