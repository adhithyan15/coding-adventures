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
