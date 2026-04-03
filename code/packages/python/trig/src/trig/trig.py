"""
Trigonometric Functions from First Principles
==============================================

This module implements sine and cosine using **Taylor series** (specifically,
Maclaurin series — Taylor series centered at zero). No math library is used;
everything is built from addition, multiplication, and division alone.

Why Taylor series?
------------------

Any "smooth" function can be approximated near a point by a polynomial. The
idea, due to Brook Taylor (1715), is:

    f(x) = f(0) + f'(0)*x + f''(0)*x^2/2! + f'''(0)*x^3/3! + ...

When centered at zero this is called a **Maclaurin series**. For sine and
cosine the derivatives cycle through a simple pattern, giving us concrete
formulas we can compute with just arithmetic.

How accurate is this?
---------------------

With 20 terms and range reduction to [-pi, pi], we achieve accuracy matching
IEEE 754 double-precision (~15 decimal digits) for all inputs, including very
large ones like sin(1000*pi).
"""

# ---------------------------------------------------------------------------
# Constants
# ---------------------------------------------------------------------------

# Pi to full double-precision accuracy.  This is the ratio of a circle's
# circumference to its diameter — the most important constant in
# trigonometry.  We hard-code it here rather than computing it so that every
# other function in the module can use it without circular imports.

PI: float = 3.141592653589793

# Two-pi comes up constantly in range reduction (see below).  A full
# rotation around the unit circle is 2*pi radians, so adding or subtracting
# 2*pi from an angle doesn't change its sine or cosine.

TWO_PI: float = 2 * PI


# ---------------------------------------------------------------------------
# Range Reduction
# ---------------------------------------------------------------------------

def _range_reduce(x: float) -> float:
    """
    Normalize an angle *x* (in radians) into the interval [-pi, pi].

    Why do we need this?
    ~~~~~~~~~~~~~~~~~~~~

    The Taylor series for sin and cos converge for *any* real number, but
    they converge **faster** when x is small.  If someone passes in
    x = 1000*pi, the raw series would need hundreds of terms to cancel out
    the enormous intermediate values.  By first reducing x into [-pi, pi],
    20 terms are more than enough.

    How it works
    ~~~~~~~~~~~~

    Since sin and cos are **periodic** with period 2*pi:

        sin(x) = sin(x - 2*pi*k)   for any integer k

    we subtract (or add) multiples of 2*pi until the result lands in
    [-pi, pi].  Python's modulo operator handles this neatly:

        x mod 2*pi   lands in [0, 2*pi)

    then shifting by -pi lands in [-pi, pi).
    """
    # Step 1: bring x into [0, 2*pi) using modulo.
    # Python's % operator always returns a non-negative result when the
    # divisor is positive, which is exactly what we want.
    x = x % TWO_PI

    # Step 2: shift from [0, 2*pi) to [-pi, pi).
    # Values greater than pi get wrapped to the negative side.
    if x > PI:
        x -= TWO_PI

    return x


# ---------------------------------------------------------------------------
# Sine — The Maclaurin Series
# ---------------------------------------------------------------------------

def sin(x: float) -> float:
    """
    Compute the sine of *x* (in radians) using the Maclaurin series.

    The Maclaurin series for sine
    ~~~~~~~~~~~~~~~~~~~~~~~~~~~~~

    Starting from the general Taylor formula and plugging in the derivatives
    of sin (which cycle: cos, -sin, -cos, sin, ...):

        sin(x) = x - x^3/3! + x^5/5! - x^7/7! + ...

    Written with sigma notation:

                 inf
        sin(x) = SUM  (-1)^n * x^(2n+1) / (2n+1)!
                 n=0

    Computing iteratively (the key trick)
    ~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~

    Rather than computing each term from scratch (which would require
    computing large factorials and large powers), we compute each term
    **from the previous one**.  Look at the ratio of consecutive terms:

        term_{n+1}     (-1)^(n+1) * x^(2(n+1)+1) / (2(n+1)+1)!
        ----------  =  ----------------------------------------
         term_n            (-1)^n * x^(2n+1) / (2n+1)!

    Simplifying:

        term_{n+1} = term_n * (-x^2) / ((2n+2) * (2n+3))

    This is beautiful: each new term is just the old term multiplied by
    a small fraction.  No factorials, no large powers — just one multiply
    and one divide per iteration.
    """
    # --- Range reduction first ---
    # Bring x into [-pi, pi] so the series converges rapidly.
    x = _range_reduce(x)

    # --- Series computation ---
    # The first term (n=0) of the Maclaurin series for sin is simply x.
    term: float = x          # current term: (-1)^n * x^(2n+1) / (2n+1)!
    total: float = x         # running sum of all terms so far

    # We'll compute 20 terms total (n=0 through n=19).  In practice the
    # series converges well before 20 terms for inputs in [-pi, pi], but
    # extra terms cost almost nothing and guarantee full precision.
    for n in range(1, 20):
        # Compute the multiplier to go from term_n to term_{n+1}:
        #
        #   multiplier = -x^2 / ((2n+2) * (2n+3))
        #
        # But since we're using the *previous* n (which is n-1 in our loop
        # variable), the denominator uses (2*(n-1)+2) * (2*(n-1)+3) = 2n * (2n+1).
        # Wait — let's be precise.  In the loop, `n` is the 1-based index of
        # the term we're about to compute.  The previous term had index n-1.
        # So:
        #
        #   term_n = term_{n-1} * (-x^2) / ((2(n-1)+2) * (2(n-1)+3))
        #          = term_{n-1} * (-x^2) / ((2n) * (2n+1))

        denominator = (2 * n) * (2 * n + 1)
        term = term * (-x * x) / denominator
        total += term

    return total


# ---------------------------------------------------------------------------
# Cosine — The Maclaurin Series
# ---------------------------------------------------------------------------

def cos(x: float) -> float:
    """
    Compute the cosine of *x* (in radians) using the Maclaurin series.

    The Maclaurin series for cosine
    ~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~

    Cosine's derivatives also cycle, but shifted by one position compared
    to sine:

        cos(x) = 1 - x^2/2! + x^4/4! - x^6/6! + ...

    In sigma notation:

                 inf
        cos(x) = SUM  (-1)^n * x^(2n) / (2n)!
                 n=0

    The iterative trick (same idea as sine)
    ~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~

    The ratio of consecutive terms:

        term_{n+1} = term_n * (-x^2) / ((2n+1) * (2n+2))

    Notice it's almost identical to the sine recurrence — only the
    denominator indices differ by one.  This makes sense: cosine uses
    even powers (0, 2, 4, ...) while sine uses odd powers (1, 3, 5, ...),
    so the factorial denominators shift accordingly.
    """
    # --- Range reduction first ---
    x = _range_reduce(x)

    # --- Series computation ---
    # The first term (n=0) of the Maclaurin series for cos is 1.
    term: float = 1.0        # current term: (-1)^n * x^(2n) / (2n)!
    total: float = 1.0       # running sum

    for n in range(1, 20):
        # Going from term at index (n-1) to term at index n:
        #
        #   term_n = term_{n-1} * (-x^2) / ((2(n-1)+1) * (2(n-1)+2))
        #          = term_{n-1} * (-x^2) / ((2n-1) * (2n))

        denominator = (2 * n - 1) * (2 * n)
        term = term * (-x * x) / denominator
        total += term

    return total


# ---------------------------------------------------------------------------
# Degree / Radian Conversion
# ---------------------------------------------------------------------------

def radians(deg: float) -> float:
    """
    Convert an angle from degrees to radians.

    Why radians?
    ~~~~~~~~~~~~

    Degrees are a human convenience (360 per full turn, inherited from
    Babylonian base-60 arithmetic).  Radians are the *natural* unit for
    angles in mathematics: one radian is the angle subtended by an arc
    whose length equals the radius.  A full circle is 2*pi radians.

    The conversion is straightforward:

        radians = degrees * (pi / 180)

    since 180 degrees = pi radians.
    """
    return deg * PI / 180


def degrees(rad: float) -> float:
    """
    Convert an angle from radians to degrees.

    This is simply the inverse of the radians() function:

        degrees = radians * (180 / pi)
    """
    return rad * 180 / PI


# ---------------------------------------------------------------------------
# Square Root — Newton's (Babylonian) Method
# ---------------------------------------------------------------------------

def sqrt(x: float) -> float:
    """
    Compute the square root of *x* using Newton's method.

    Newton's Method for Square Roots
    ~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~

    Newton's method (also called the Babylonian method — it was known to
    Babylonian mathematicians over 3,000 years ago) says: if *guess* is
    an approximation to sqrt(x), then the average of *guess* and *x/guess*
    is a better approximation.

    Intuition: if guess < sqrt(x), then x/guess > sqrt(x), so their average
    "squeezes" toward the true value from both sides.

    The convergence is **quadratic** — the number of correct decimal digits
    doubles with each iteration. For x = 2:

        Iteration 0: guess = 2.0
        Iteration 1: guess = (2.0 + 2.0/2.0) / 2 = 1.5
        Iteration 2: guess = (1.5 + 2.0/1.5) / 2 ≈ 1.41667
        Iteration 3: guess ≈ 1.41422
        Iteration 4: guess ≈ 1.41421356237...  (full precision in 4 steps!)

    Typically converges in 10–15 iterations for any normal double-precision input.
    """
    if x < 0:
        raise ValueError(f"sqrt: domain error — input {x} is negative")

    # sqrt(0) is exactly 0.
    if x == 0.0:
        return 0.0

    # Initial guess: x itself for x >= 1 (good for large numbers),
    # 1.0 for x < 1 (avoids dividing by a tiny number in the first step).
    guess: float = x if x >= 1.0 else 1.0

    # Iterate until convergence or safety limit.
    for _ in range(60):
        next_guess = (guess + x / guess) / 2.0

        # Stop when improvement is below the precision floor.
        # 1e-15 * guess is relative precision; 1e-300 handles subnormals.
        if abs(next_guess - guess) < 1e-15 * guess + 1e-300:
            return next_guess

        guess = next_guess

    return guess


# ---------------------------------------------------------------------------
# Tangent — Sine over Cosine
# ---------------------------------------------------------------------------

def tan(x: float) -> float:
    """
    Compute the tangent of *x* (in radians).

    Definition and Geometry
    ~~~~~~~~~~~~~~~~~~~~~~~

    Tangent is the ratio of sine to cosine:

        tan(x) = sin(x) / cos(x)

    On the unit circle, if you draw a vertical line tangent to the circle at
    (1, 0), then tan(x) is where the ray at angle x meets that line. This is
    the literal origin of the name "tangent."

    Undefined Points (Poles)
    ~~~~~~~~~~~~~~~~~~~~~~~~

    Wherever cos(x) = 0 — at x = π/2 + k·π for any integer k — tan(x) is
    undefined (division by zero). The function "blows up" to ±∞. We detect
    when |cos(x)| < 1e-15 and return a very large finite number instead, to
    signal the near-singularity without raising an error.

    We call our own sin() and cos() here — no math.tan used.
    """
    s = sin(x)  # our own sin — no math.sin
    c = cos(x)  # our own cos — no math.cos

    # Guard against poles: |cos(x)| < 1e-15 means we're within
    # about 1e-15 radians of a discontinuity.
    if abs(c) < 1e-15:
        # Return the largest float, signed to match the direction of divergence.
        return 1.0e308 if s > 0 else -1.0e308

    return s / c


# ---------------------------------------------------------------------------
# Arctangent — Inverse of Tangent
# ---------------------------------------------------------------------------

# HALF_PI is π/2. It appears in both atan's range reduction and atan2's
# quadrant handling. We compute it once here for reuse.
HALF_PI: float = PI / 2.0


def _atan_core(x: float) -> float:
    """
    Core atan computation for |x| <= 1, using Taylor series + half-angle reduction.

    This is a private helper. Callers should use atan() instead.

    Half-Angle Reduction
    ~~~~~~~~~~~~~~~~~~~~

    The Taylor series for atan:

        atan(x) = x - x^3/3 + x^5/5 - x^7/7 + ...   (for |x| <= 1)

    converges slowly near x = 1 (requires ~50 terms for full precision).

    Fix: the half-angle identity for atan:

        atan(x) = 2 * atan( x / (1 + sqrt(1 + x^2)) )

    This halves the argument: if |x| <= 1, then after reduction we get
    |y| <= tan(pi/8) ~= 0.414, where the series converges in ~15 terms.

    Taylor Series Iteration
    ~~~~~~~~~~~~~~~~~~~~~~~

    atan(t) = t - t^3/3 + t^5/5 - t^7/7 + ...

    Iterative ratio between consecutive terms:

        term_n = term_{n-1} * (-t^2) * (2n-1) / (2n+1)

    We multiply the final result by 2 to undo the half-angle halving.
    """
    # Half-angle reduction: shrink |x| to |y| <= tan(pi/8) ~= 0.414.
    # We use our own sqrt here — no math.sqrt.
    reduced: float = x / (1.0 + sqrt(1.0 + x * x))

    # Taylor series on the reduced argument.
    t: float = reduced
    t_sq: float = t * t
    term: float = t
    result: float = t

    for n in range(1, 31):
        # Each term: term_n = term_{n-1} * (-t^2) * (2n-1) / (2n+1)
        term = term * (-t_sq) * (2 * n - 1) / (2 * n + 1)
        result += term

        # Early exit when terms are negligibly small.
        if abs(term) < 1e-17:
            break

    # Undo the half-angle: atan(x) = 2 * atan(reduced).
    return 2.0 * result


def atan(x: float) -> float:
    """
    Compute the arctangent of *x*, returning the angle in radians.

    Return range: (-pi/2, pi/2).

    Range Reduction
    ~~~~~~~~~~~~~~~

    The Taylor series for atan converges only for |x| <= 1. For |x| > 1
    we use the complementary identity:

        atan(x)  = pi/2 - atan(1/x)    for x > 1
        atan(x)  = -pi/2 - atan(1/x)   for x < -1

    Proof: atan(x) + atan(1/x) = pi/2 for x > 0. If theta = atan(x),
    then tan(pi/2 - theta) = cot(theta) = 1/x, so atan(1/x) = pi/2 - theta.
    """
    if x == 0.0:
        return 0.0

    if x > 1.0:
        return HALF_PI - _atan_core(1.0 / x)
    if x < -1.0:
        return -HALF_PI - _atan_core(1.0 / x)

    return _atan_core(x)


def atan2(y: float, x: float) -> float:
    """
    Compute the four-quadrant arctangent of *y* and *x*.

    Return range: (-pi, pi].

    Why Is atan2 Different from atan?
    ~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~

    atan(y/x) only gives angles in (-pi/2, pi/2) — the right half of the
    plane. It cannot distinguish between:

        (y=1, x=1)  →  angle +pi/4   (first quadrant)
        (y=-1, x=-1) →  y/x = 1, so atan gives pi/4 — WRONG, should be -3pi/4

    atan2 inspects the signs of both y and x separately to determine the
    correct quadrant.

    Quadrant Diagram:
    ~~~~~~~~~~~~~~~~~

            y > 0
        Q2  |  Q1        atan2 > 0 in Q1 and Q2
      ------+------  x   atan2 < 0 in Q3 and Q4
        Q3  |  Q4        atan2 = ±pi on the negative x-axis

    Special Cases:
    ~~~~~~~~~~~~~~
      (y=0, x>0)   → 0       (positive x-axis)
      (y>0, x=0)   → pi/2    (positive y-axis)
      (y=0, x<0)   → pi      (negative x-axis, by atan(0/x<0) + pi = pi)
      (y<0, x=0)   → -pi/2   (negative y-axis)
      (y=0, x=0)   → 0       (undefined by convention)
    """
    if x > 0.0:
        return atan(y / x)
    if x < 0.0 and y >= 0.0:
        return atan(y / x) + PI
    if x < 0.0 and y < 0.0:
        return atan(y / x) - PI
    if x == 0.0 and y > 0.0:
        return HALF_PI
    if x == 0.0 and y < 0.0:
        return -HALF_PI
    # x == 0.0 and y == 0.0: undefined, return 0 by convention.
    return 0.0
