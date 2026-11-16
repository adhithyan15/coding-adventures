# frozen_string_literal: true

# =============================================================================
# Trig -- Trigonometric Functions from First Principles (TRIG-REF-1)
# =============================================================================
#
# This module computes sine, cosine, tangent and friends without calling
# Ruby's Math library.  Since TRIG-CONV (PHY00a, step TC-13) it follows the
# one reference algorithm, TRIG-REF-1, that every language port of `trig`
# shares, so that the same input produces the same 64 bits in every language.
#
# The analogy: seventeen workshops each cut a "one metre" rod from the same
# drawing.  They all look right and they differ by a hair.  TRIG-REF-1 is the
# master bar and the shared test vectors are its calibration marks.
#
# ## The arithmetic model
#
# Every value is an IEEE 754 binary64 number -- exactly what a Ruby Float is.
# Every step is ONE addition, subtraction or multiplication, rounded to
# nearest-even once, with no fused multiply-add (MRI evaluates `*` and `+` as
# separate method calls, so nothing can fuse), no reassociation and no library
# call.  sin, cos, radians and degrees use no division and no square root.
#
# Ruby-specific care: an Integer argument is converted ONCE, at entry, with
# `to_f`, so that nothing inside the algorithm is ever Integer or Rational
# arithmetic.  (`x / TWO_PI` with an Integer x would silently be fine, but
# `x * x` on a big Integer would not be a binary64 product.)  `to_f` keeps
# -0.0 as -0.0.
#
# ## The plan for sin(x) and cos(x)
#
#   1. Guard: NaN, infinities and |x| > 2^29 are outside the domain -> NaN.
#   2. |x| < 2^-27: sin(x) is x itself (the next term is below half an ulp),
#      cos(x) is 1.  This also makes sin(-0.0) = -0.0.
#   3. Reduce: find the nearest whole number k of quarter-turns, so that
#      r = x - k*(pi/2) satisfies |r| <= pi/4 (Cody-Waite, below).
#   4. Evaluate a fixed-length Horner polynomial for sin(r) or cos(r).
#   5. Use the quadrant q = k mod 4 to pick sin(r), cos(r), -sin(r) or -cos(r).
#
# =============================================================================

require_relative "trig/version"

module Trig
  # ---------------------------------------------------------------------------
  # Bit-exact constants
  # ---------------------------------------------------------------------------
  #
  # Every constant below is specified by its IEEE bit pattern (PHY00a 3.2),
  # and the test suite asserts those bits.  from_bits reinterprets a 64-bit
  # unsigned integer as a Float:
  #
  #   "Q>"  unsigned 64-bit integer, big-endian
  #   "G"   double-precision float, big-endian
  #
  # so the two are the same eight bytes read two ways -- a pure bit cast.

  def self.from_bits(bits)
    [bits].pack("Q>").unpack1("G")
  end

  private_class_method :from_bits

  # PI is the ratio of a circle's circumference to its diameter: the double
  # nearest the real pi, which is slightly LESS than pi.

  PI = from_bits(0x400921FB54442D18) # 3.141592653589793

  # TWO_PI: one full turn.  HALF_PI: one quarter-turn (both exact multiples
  # of PI by a power of two, so neither rounds).

  TWO_PI = from_bits(0x401921FB54442D18)
  HALF_PI = from_bits(0x3FF921FB54442D18)

  # INV_PIO2 is the double nearest 2/pi.  Multiplying x by it estimates how
  # many quarter-turns x contains.

  INV_PIO2 = from_bits(0x3FE45F306DC9C883) # 0.6366197723675814

  # pi/2 stored as a SUM of five pieces (PHY00a 3.2).  The first four have at
  # most 24 significant bits each, so k * PIO2_i is computed EXACTLY for any
  # whole number |k| < 2^29 (24 + 29 = 53 bits fit a double).  The fifth is
  # the remainder rounded to 53 bits.  Together they carry pi/2 to 149 bits.

  PIO2 = [
    from_bits(0x3FF921FB40000000), # 1.570796251296997
    from_bits(0x3E74442D00000000), # 7.549789415861596e-08
    from_bits(0x3CF8469880000000), # 5.390302529957765e-15
    from_bits(0x3B78CC5160000000), # 3.282003415807913e-22
    from_bits(0x39F01B839A25204A)  # 1.2706558760139879e-29
  ].freeze

  # MAGIC is 1.5 * 2^52.  Adding it to t (|t| < 2^51) pushes every fractional
  # bit of t off the end of the 53-bit significand, so (t + MAGIC) - MAGIC is t
  # rounded to the NEAREST WHOLE NUMBER, ties to even, using only an add and a
  # subtract -- no floor, no round, no Float#to_i on an unchecked value.

  MAGIC = from_bits(0x4338000000000000) # 6755399441055744.0

  # LIMIT = 2^29: beyond it k no longer fits the exact products above, and
  # recovering x mod pi/2 would need pi to a thousand bits (Payne-Hanek, a
  # later amendment).  Such arguments are OUTSIDE THE DOMAIN and give NaN.

  LIMIT = from_bits(0x41C0000000000000) # 536870912.0

  # TINY = 2^-27: below it sin(x) == x and cos(x) == 1 to the last bit.

  TINY = from_bits(0x3E40000000000000) # 7.450580596923828e-09

  # ---------------------------------------------------------------------------
  # The Maclaurin coefficients, stored (not recomputed)
  # ---------------------------------------------------------------------------
  #
  # sin(r) = r - r^3/3! + r^5/5! - ...    S[n] = (-1)^n / (2n+1)!,  n = 1..9
  # cos(r) = 1 - r^2/2! + r^4/4! - ...    C[n] = (-1)^n / (2n)!,    n = 2..9
  #
  # Each is the exact rational rounded ONCE to the nearest double.  The table
  # of factorials is visible right here, which keeps the package educational:
  #
  #   n   S[n]    value                        C[n]    value
  #   1   -1/3!   -0.16666666666666666         (-1/2!, applied as 0.5 * z)
  #   2   +1/5!    0.008333333333333333        +1/4!    0.041666666666666664
  #   3   -1/7!   -0.0001984126984126984       -1/6!   -0.001388888888888889
  #   4   +1/9!    2.7557319223985893e-06      +1/8!    2.48015873015873e-05
  #   5   -1/11!  -2.505210838544172e-08       -1/10!  -2.755731922398589e-07
  #   6   +1/13!   1.6059043836821613e-10      +1/12!   2.08767569878681e-09
  #   7   -1/15!  -7.647163731819816e-13       -1/14!  -1.1470745597729725e-11
  #   8   +1/17!   2.8114572543455206e-15      +1/16!   4.779477332387385e-14
  #   9   -1/19!  -8.22063524662433e-18        -1/18!  -1.5619206968586225e-16
  #
  # S[0] and C[0], C[1] are unused (nil): the leading terms r and 1 - z/2 are
  # added by the kernels themselves.

  S = [
    nil,
    from_bits(0xBFC5555555555555),
    from_bits(0x3F81111111111111),
    from_bits(0xBF2A01A01A01A01A),
    from_bits(0x3EC71DE3A556C734),
    from_bits(0xBE5AE64567F544E4),
    from_bits(0x3DE6124613A86D09),
    from_bits(0xBD6AE7F3E733B81F),
    from_bits(0x3CE952C77030AD4A),
    from_bits(0xBC62F49B46814157)
  ].freeze

  C = [
    nil, nil,
    from_bits(0x3FA5555555555555),
    from_bits(0xBF56C16C16C16C17),
    from_bits(0x3EFA01A01A01A01A),
    from_bits(0xBE927E4FB7789F5C),
    from_bits(0x3E21EED8EFF8D898),
    from_bits(0xBDA93974A8C07C9D),
    from_bits(0x3D2AE7F3E733B81F),
    from_bits(0xBCA6827863B97D97)
  ].freeze

  # Degree conversion constants: the doubles nearest pi/180 and 180/pi.

  RAD_PER_DEG = from_bits(0x3F91DF46A2529D39) # 0.017453292519943295
  DEG_PER_RAD = from_bits(0x404CA5DC1A63C1F8) # 57.29577951308232

  # ---------------------------------------------------------------------------
  # Range reduction: Cody-Waite with a nearest-integer quotient (private)
  # ---------------------------------------------------------------------------
  #
  # A sine wave repeats, so knowing sin on [-pi/4, pi/4] and which quarter-turn
  # we are in is enough to know it everywhere.  Reduction answers two
  # questions: how many quarter-turns fit in x (a whole number k), and what is
  # left over (r = x - k*pi/2, with |r| <= pi/4)?
  #
  # Step 1, the quotient:
  #
  #   t = x * INV_PIO2            one rounding; roughly x / (pi/2)
  #   k = (t + MAGIC) - MAGIC     nearest whole number to t (see MAGIC above)
  #
  # Step 2, the remainder.  pi/2 is irrational, so k*(pi/2) cannot be formed
  # exactly.  We store pi/2 as five short slices and subtract one slice at a
  # time; each product k*PIO2_i (i <= 4) is exact, so only the subtractions
  # round:
  #
  #   x ---------------------------------------------------------- the angle
  #   |-- k*PIO2_1 --|                                              24 bits
  #                  |-- k*PIO2_2 --|                               next 24
  #                                 |-- k*PIO2_3 --|                next 24
  #                                                |- k*PIO2_4 -|   next 24
  #                                                             |k*PIO2_5|
  #   r = what is left, |r| <= pi/4 (plus a few ulps)
  #
  # Every product is written as its own statement, and then subtracted: this
  # is the shape that must never be fused into a multiply-add (PHY00a 5).
  #
  # Returns [k, r], with k a whole-number Float and |k| < 2^29.

  def self.reduce(x)
    t = x * INV_PIO2
    t += MAGIC
    k = t - MAGIC
    r = x
    PIO2.each do |piece|
      m = k * piece
      r -= m
    end
    [k, r]
  end

  private_class_method :reduce

  # ---------------------------------------------------------------------------
  # The series kernels, by Horner's rule (private)
  # ---------------------------------------------------------------------------
  #
  # On |r| <= pi/4 write z = r*r.  Then
  #
  #   sin(r) = r + r*z*(S1 + S2*z + S3*z^2 + ... + S9*z^8)    r^1 .. r^19
  #   cos(r) = (1 - z/2) + z^2*(C2 + C3*z + ... + C9*z^7)      r^0 .. r^18
  #
  # Horner evaluates the bracket innermost-first: start with the highest
  # coefficient and repeatedly "multiply by z, add the next coefficient".
  # That adds the SMALLEST terms first (the accurate order), needs no
  # division, and cannot be grouped two ways.  The leading term is added last
  # so the big part of the answer is rounded only once.  Ten terms are far
  # more than rounding can resolve; the count is fixed so that every port adds
  # the same terms.

  def self.sin_kernel(r)
    z = r * r
    poly = S[9]
    8.downto(1) do |n|
      poly *= z
      poly += S[n]
    end
    w = r * z
    w *= poly
    r + w
  end

  def self.cos_kernel(r)
    z = r * r
    poly = C[9]
    8.downto(2) do |n|
      poly *= z
      poly += C[n]
    end
    u = 0.5 * z # exact: halving never rounds
    u = 1.0 - u
    v = z * z
    v *= poly
    u + v
  end

  private_class_method :sin_kernel, :cos_kernel

  # ---------------------------------------------------------------------------
  # Sine
  # ---------------------------------------------------------------------------
  #
  # With k the quarter-turn count and q = k mod 4, the identities
  #
  #   sin(r + k*pi/2) = { sin r, cos r, -sin r, -cos r }[q]
  #
  # give the answer.  k is a whole-number Float of magnitude below 2^29, so
  # k.to_i is exact, and `& 3` on a two's-complement Integer is the FLOOR
  # modulus 4 (k = -1 gives q = 3, k = -5 gives q = 3).
  #
  # Quadrant table (q: sin(x), cos(x)):
  #
  #   q = 0:   sin r      cos r
  #   q = 1:   cos r     -sin r
  #   q = 2:  -sin r     -cos r
  #   q = 3:  -cos r      sin r
  #
  # The domain guard is written as a NEGATED <=, and must be: NaN fails every
  # comparison, so `!(|x| <= LIMIT)` catches NaN, both infinities and every
  # finite value beyond 2^29 with one test.  (Writing `|x| > LIMIT` would let
  # a NaN through to `to_i`, which raises FloatDomainError.)
  #
  # @param x [Numeric] angle in radians
  # @return [Float] the sine of x; NaN if x is NaN, infinite or |x| > 2^29

  def self.sin(x)
    x = x.to_f
    return Float::NAN unless x.abs <= LIMIT # NaN, +-inf, |x| > 2^29
    return x if x.abs < TINY # exact; keeps sin(-0.0) = -0.0

    k, r = reduce(x)
    case k.to_i & 3
    when 0 then sin_kernel(r)
    when 1 then cos_kernel(r)
    when 2 then -sin_kernel(r)
    else -cos_kernel(r)
    end
  end

  # ---------------------------------------------------------------------------
  # Cosine
  # ---------------------------------------------------------------------------
  #
  # Same reduction and quadrant machinery as sine; see the table above.
  #
  # @param x [Numeric] angle in radians
  # @return [Float] the cosine of x; NaN if x is NaN, infinite or |x| > 2^29

  def self.cos(x)
    x = x.to_f
    return Float::NAN unless x.abs <= LIMIT
    return 1.0 if x.abs < TINY

    k, r = reduce(x)
    case k.to_i & 3
    when 0 then cos_kernel(r)
    when 1 then -sin_kernel(r)
    when 2 then -cos_kernel(r)
    else sin_kernel(r)
    end
  end

  # ---------------------------------------------------------------------------
  # Degree/Radian Conversion
  # ---------------------------------------------------------------------------
  #
  # PI radians = 180 degrees, so
  #
  #   radians = degrees * (PI / 180)      one multiply by a stored constant
  #   degrees = radians * (180 / PI)
  #
  # One multiply is one rounding (the older `deg * PI / 180` rounded twice and
  # could overflow in the intermediate product).  NaN propagates, +-inf maps
  # to +-inf and the sign of zero is kept: `x.to_f` preserves -0.0.

  # Converts degrees to radians.
  #
  # @param deg [Numeric] angle in degrees
  # @return [Float] angle in radians
  #
  # Examples:
  #   Trig.radians(0)   => 0.0
  #   Trig.radians(180) => PI

  def self.radians(deg)
    deg.to_f * RAD_PER_DEG
  end

  # Converts radians to degrees.
  #
  # @param rad [Numeric] angle in radians
  # @return [Float] angle in degrees
  #
  # Examples:
  #   Trig.degrees(0)  => 0.0
  #   Trig.degrees(PI) => 180.0

  def self.degrees(rad)
    rad.to_f * DEG_PER_RAD
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
    return x if x == 0.0
    return x if x.infinite? == 1

    scaled = x
    result_scale = 1.0
    while scaled < 0.25
      scaled *= 4.0
      result_scale *= 0.5
    end
    while scaled >= 4.0
      scaled *= 0.25
      result_scale *= 2.0
    end

    # Initial guess: x itself works for x >= 1 (large values converge faster
    # from x than from 1). For x < 1, start at 1.0 to avoid tiny denominators.
    guess = scaled >= 1.0 ? scaled : 1.0

    # Iterate to convergence (or up to 60 steps as a safety cap).
    60.times do
      next_guess = (guess + scaled / guess) / 2.0

      # Stop when improvement is below the precision floor.
      # 1e-15 * guess is relative precision; 1e-300 handles subnormals.
      if (next_guess - guess).abs < 1e-15 * guess + 1e-300
        return next_guess * result_scale
      end

      guess = next_guess
    end

    guess * result_scale
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
  # The sentinel is a PINNED LEGACY CONTRACT (PHY00 3.6), not an approximation:
  # the true tangent of the double nearest pi/2 is about 1.6e16, not 1e308.
  # TRIG-REF-1 makes NO accuracy claim for tan (it is sin / cos plus a
  # division).  Outside the domain (|x| > 2^29, NaN, infinity) sin and cos are
  # both NaN, the sentinel test is false, and NaN / NaN is NaN.  The domain
  # guard also makes cos == 0 unreachable, so no division by zero can occur.
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

  # HALF_PI (defined with the other constants above) is used in atan range
  # reduction and atan2 quadrant cases.

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
    # atan(x) rounds exactly to x here; avoid halving subnormals and retain -0.
    return x if x.abs <= 7.450580596923828e-9

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

    # Added by TRIG-REF-1 (PHY00a 3.9): a NaN argument must give NaN, not the
    # finite 0.0 that every comparison below would otherwise fall through to.
    return Float::NAN if x.nan? || y.nan?

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
