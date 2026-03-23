-- ============================================================================
-- trig -- Trigonometric functions computed from first principles
-- ============================================================================
--
-- This module provides sine, cosine, tangent, and angle-conversion functions
-- built entirely from scratch using Maclaurin series. No standard-library
-- trig functions are used -- the goal is to understand *how* these functions
-- work at the mathematical level.
--
-- ## Why Build Trig from Scratch?
--
-- Every programming language ships with sin() and cos() in its standard
-- library. So why reimplement them? Because understanding *how* these
-- functions work deepens our appreciation for mathematics and computing.
-- The techniques here -- Maclaurin series, range reduction, and iterative
-- term computation -- are the same ideas used inside real math libraries
-- (just with additional optimizations like Chebyshev polynomials and table
-- lookups).
--
-- ## The Core Idea: Maclaurin Series
--
-- A Maclaurin series expresses a function as an infinite sum of terms
-- computed from the function's derivatives evaluated at zero:
--
--     sin(x) = x - x^3/3! + x^5/5! - x^7/7! + ...
--     cos(x) = 1 - x^2/2! + x^4/4! - x^6/6! + ...
--
-- Each successive term is smaller than the last (for reasonable x), so
-- after enough terms the sum converges to the true value. We use 20 terms,
-- which gives us double-precision accuracy for inputs near zero.
--
-- ## Range Reduction
--
-- The Maclaurin series converges quickly for small x, but slowly for large
-- x. Since sin and cos are periodic (they repeat every 2*pi), we can always
-- reduce any input to the range [-pi, pi] without changing the result. This
-- is called "range reduction" and it's a critical step in every real trig
-- implementation.

local trig = {}

trig.VERSION = "0.1.0"

-- ============================================================================
-- Constants
-- ============================================================================

--- PI is the ratio of a circle's circumference to its diameter.
--
-- This is one of the most fundamental constants in mathematics. It appears
-- everywhere: geometry, trigonometry, calculus, physics, statistics, and more.
--
-- We define it to the full precision of a Lua number (IEEE 754 double, about
-- 15-16 significant digits). This matches the value in Go's math package and
-- Lua's own math.pi.
trig.PI = 3.141592653589793

--- TWO_PI is the full period of sine and cosine.
--
-- Sine and cosine are "periodic" functions -- they repeat the same pattern
-- over and over. The length of one complete cycle is 2*pi radians (360
-- degrees). We precompute this constant to avoid repeated multiplication.
trig.TWO_PI = 2 * trig.PI

-- ============================================================================
-- Range Reduction (internal)
-- ============================================================================

--- range_reduce normalises an angle x (in radians) to the range [-pi, pi].
--
-- ### Why Is This Necessary?
--
-- The Maclaurin series for sin and cos converges fastest when x is close to
-- zero. For large values of x (like 1000*pi), the series terms start out
-- enormous before cancelling, which causes floating-point precision loss.
--
-- Since sin and cos repeat every 2*pi, we can subtract multiples of 2*pi
-- to bring x into [-pi, pi] without changing the function's value:
--
--     sin(x) = sin(x - 2*pi*k)   for any integer k
--     cos(x) = cos(x - 2*pi*k)   for any integer k
--
-- ### How It Works
--
-- We follow the same approach used in the Go implementation:
--
--  1. Divide x by 2*pi to find how many full cycles it contains.
--  2. Subtract those full cycles using truncation toward zero.
--  3. If the result is still outside [-pi, pi], adjust by one more 2*pi.
--
-- The Lua function math.modf(n) returns the integer part truncated toward
-- zero, which mirrors Go's int() truncation behaviour:
--
--     math.modf( 2.7) =>  2     (truncate toward zero, not floor)
--     math.modf(-2.7) => -2     (truncate toward zero, not floor)
--
-- This distinction matters for negative inputs: math.floor(-2.7) = -3,
-- but truncation gives -2, which is what we want for symmetric reduction.
local function range_reduce(x)
    -- Step 1: Remove full rotations.
    -- Compute x mod 2*pi manually using division and truncation.
    --
    -- math.modf(x / TWO_PI) gives the number of complete cycles truncated
    -- toward zero. Subtracting those cycles leaves the remainder.
    local full_cycles = math.modf(x / trig.TWO_PI)  -- integer part, truncated
    x = x - trig.TWO_PI * full_cycles

    -- Step 2: Ensure we're in [-pi, pi].
    --
    -- After removing full rotations, x is in (-2*pi, 2*pi). We may need
    -- one more adjustment to land in [-pi, pi].
    --
    --   If x > pi:  subtract 2*pi  (e.g., 3.5 -> 3.5 - 6.28 = -2.78)
    --   If x < -pi: add 2*pi       (e.g., -3.5 -> -3.5 + 6.28 = 2.78)
    if x > trig.PI then
        x = x - trig.TWO_PI
    elseif x < -trig.PI then
        x = x + trig.TWO_PI
    end

    return x
end

-- ============================================================================
-- Sin -- The Sine Function
-- ============================================================================

--- Compute the sine of x (in radians) using a Maclaurin series.
--
-- ### The Maclaurin Series for Sine
--
-- The sine function can be expressed as an infinite polynomial:
--
--     sin(x) = x - x^3/3! + x^5/5! - x^7/7! + x^9/9! - ...
--
-- Written more compactly with summation notation:
--
--     sin(x) = sum_{n=0}^{inf} (-1)^n * x^(2n+1) / (2n+1)!
--
-- Each term uses odd powers of x (1, 3, 5, 7, ...) and alternates in sign.
--
-- ### Iterative Term Computation
--
-- A naive implementation would compute x^n and n! separately for each term.
-- This is wasteful and can overflow for large n. Instead, we compute each
-- term from the previous one:
--
--     term_0 = x
--     term_n = term_{n-1} * (-x^2) / ((2n)(2n+1))
--
-- This works because the ratio of consecutive terms is:
--
--     term_n / term_{n-1} = (-1) * x^2 / ((2n)(2n+1))
--
-- The factor (2n)(2n+1) in the denominator comes from the factorial growth:
-- (2n+1)! / (2n-1)! = (2n)(2n+1).
--
-- This "iterative term computation" trick avoids computing large factorials
-- and keeps every intermediate value small enough for floating-point.
--
-- ### Example: sin(pi/6) = 0.5
--
--     x = 0.5236 (pi/6)
--     term 0: +0.5236
--     term 1: -0.0239  (multiply by -x^2 / (2*3))
--     term 2: +0.0003  (multiply by -x^2 / (4*5))
--     ...
--     sum converges to 0.5000
--
-- @param x  angle in radians
-- @return   sine of x
function trig.sin(x)
    -- Step 1: Range reduction.
    -- Bring x into [-pi, pi] so the series converges quickly.
    x = range_reduce(x)

    -- Step 2: Initialise the series.
    -- The first term of the Maclaurin series for sin is just x itself.
    local term = x    -- current term in the series (the n=0 term)
    local sum  = term -- running total accumulating all terms

    -- Step 3: Accumulate terms.
    -- We compute 20 terms, which is more than enough for double precision.
    -- (In practice, the series converges in about 10-12 terms for |x| <= pi,
    -- but extra terms cost almost nothing and ensure accuracy.)
    for n = 1, 20 do
        -- Compute the ratio between consecutive terms:
        --   term_n = term_{n-1} * (-x^2) / ((2n)(2n+1))
        --
        -- The denominator (2n)(2n+1) grows quadratically, so each term
        -- shrinks rapidly. The minus sign creates the alternating pattern.
        local denom = (2 * n) * (2 * n + 1)
        term = term * (-x * x) / denom
        sum = sum + term
    end

    return sum
end

-- ============================================================================
-- Cos -- The Cosine Function
-- ============================================================================

--- Compute the cosine of x (in radians) using a Maclaurin series.
--
-- ### The Maclaurin Series for Cosine
--
-- The cosine function's Maclaurin series uses even powers of x:
--
--     cos(x) = 1 - x^2/2! + x^4/4! - x^6/6! + x^8/8! - ...
--
-- Written more compactly:
--
--     cos(x) = sum_{n=0}^{inf} (-1)^n * x^(2n) / (2n)!
--
-- ### Comparison with Sine
--
-- Notice the structural similarity:
--
--     sin: odd powers   (1, 3, 5, 7, ...)   starting with x
--     cos: even powers  (0, 2, 4, 6, ...)   starting with 1
--
-- The iterative term computation works the same way, but the ratio between
-- consecutive terms is:
--
--     term_n = term_{n-1} * (-x^2) / ((2n-1)(2n))
--
-- The denominator factors differ slightly because cosine uses even powers.
--
-- ### The Pythagorean Identity
--
-- One of the most important identities in trigonometry is:
--
--     sin^2(x) + cos^2(x) = 1
--
-- This holds for ALL values of x. Our test suite verifies this identity
-- as a way to check that both functions are implemented correctly.
--
-- @param x  angle in radians
-- @return   cosine of x
function trig.cos(x)
    -- Step 1: Range reduction.
    x = range_reduce(x)

    -- Step 2: Initialise the series.
    -- The first term of the Maclaurin series for cos is 1 (the constant term).
    local term = 1.0  -- current term: starts at 1 (the n=0 term)
    local sum  = term -- running total

    -- Step 3: Accumulate terms.
    -- Same strategy as sin, but with even-power denominators.
    for n = 1, 20 do
        -- The ratio between consecutive cosine terms:
        --   term_n = term_{n-1} * (-x^2) / ((2n-1)(2n))
        --
        -- Why (2n-1)(2n)? Because:
        --   (2n)! / (2(n-1))! = (2n-1)(2n)
        local denom = (2 * n - 1) * (2 * n)
        term = term * (-x * x) / denom
        sum = sum + term
    end

    return sum
end

-- ============================================================================
-- Tan -- The Tangent Function
-- ============================================================================

--- Compute the tangent of x (in radians).
--
-- ### Definition
--
-- Tangent is defined as the ratio of sine to cosine:
--
--     tan(x) = sin(x) / cos(x)
--
-- ### Where Tangent Is Undefined
--
-- Because tan(x) = sin(x) / cos(x), tangent is undefined wherever
-- cos(x) = 0. This happens at x = pi/2 + k*pi for any integer k.
-- At these points, tangent "blows up" to positive or negative infinity.
--
-- In our implementation, we do not explicitly check for division by zero.
-- Lua's IEEE 754 arithmetic will naturally produce inf or -inf when
-- dividing a non-zero number by zero, and nan when dividing zero by zero.
-- This matches the behaviour most users expect from a math library.
--
-- ### Geometric Interpretation
--
-- On the unit circle, if you draw a vertical line tangent to the circle
-- at the point (1, 0), then tan(x) is the y-coordinate where the angle's
-- ray intersects that tangent line. This is where the name "tangent"
-- comes from -- it literally refers to a tangent line.
--
-- @param x  angle in radians
-- @return   tangent of x
function trig.tan(x)
    return trig.sin(x) / trig.cos(x)
end

-- ============================================================================
-- Angle Conversion
-- ============================================================================

--- Convert an angle from degrees to radians.
--
-- ### Degrees vs Radians
--
-- Degrees and radians are two ways to measure angles:
--   - Degrees: a full circle = 360 degrees (arbitrary, from Babylonian astronomy)
--   - Radians: a full circle = 2*pi radians (natural, based on circle geometry)
--
-- The conversion formula comes from setting up a proportion:
--
--     degrees / 360 = radians / (2*pi)
--
-- Solving for radians:
--
--     radians = degrees * (2*pi / 360) = degrees * (pi / 180)
--
-- ### Examples
--
--     radians(0)   = 0
--     radians(90)  = pi/2   (a right angle)
--     radians(180) = pi     (a straight line)
--     radians(360) = 2*pi   (a full circle)
--
-- @param deg  angle in degrees
-- @return     angle in radians
function trig.radians(deg)
    return deg * trig.PI / 180.0
end

--- Convert an angle from radians to degrees.
--
-- This is the inverse of `radians`. The conversion formula is:
--
--     degrees = radians * (180 / pi)
--
-- ### Examples
--
--     degrees(0)      = 0
--     degrees(pi/2)   = 90
--     degrees(pi)     = 180
--     degrees(2*pi)   = 360
--
-- @param rad  angle in radians
-- @return     angle in degrees
function trig.degrees(rad)
    return rad * 180.0 / trig.PI
end

return trig
