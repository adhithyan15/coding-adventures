-- ============================================================================
-- trig -- Trigonometric functions computed from first principles
-- ============================================================================
--
-- This module provides sine, cosine, tangent, and angle-conversion functions
-- built entirely from scratch.  No standard-library trig functions are used
-- -- the goal is to understand *how* these functions work at the
-- mathematical level.
--
-- The arithmetic is the shared reference algorithm TRIG-REF-1, specified in
-- code/specs/PHY00a-trig-convergence.md.  Every port of this package, in
-- every language, computes the SAME BITS for sin, cos, radians and degrees;
-- the conformance test (tests/test_conformance.lua) checks 62,154 shared
-- vectors bit for bit.
--
-- ## Why Build Trig from Scratch?
--
-- Every programming language ships with sin() and cos() in its standard
-- library.  So why reimplement them?  Because understanding *how* these
-- functions work deepens our appreciation for mathematics and computing.
-- The techniques here -- range reduction, Maclaurin series, Horner's rule --
-- are the same ideas used inside real math libraries (just with additional
-- optimizations like minimax polynomials and table lookups).
--
-- ## The Core Idea: Maclaurin Series
--
-- A Maclaurin series expresses a function as an infinite sum of terms
-- computed from the function's derivatives evaluated at zero:
--
--     sin(x) = x - x^3/3! + x^5/5! - x^7/7! + ...
--     cos(x) = 1 - x^2/2! + x^4/4! - x^6/6! + ...
--
-- Each successive term is smaller than the last (for small x), so after
-- enough terms the sum converges.  We keep ten terms of each series, and
-- we only ever evaluate them for |r| <= pi/4 (about 0.785), where the first
-- omitted term is far below half an ulp.
--
-- ## Range Reduction: Cody-Waite
--
-- Series converge quickly only near zero, but sin and cos repeat forever.
-- So we split any angle x into whole quarter-turns plus a small leftover:
--
--     x = k * (pi/2) + r        k an integer, |r| <= pi/4
--
-- and the quadrant (k mod 4) says which of sin(r), cos(r), -sin(r), -cos(r)
-- the answer is.  The hard part is computing r = x - k*(pi/2) without losing
-- its leading digits: pi/2 is irrational, so a double only holds it to 53
-- bits, and k * that double would carry an error as big as k * 2^-53.
--
-- The Cody-Waite trick stores pi/2 as a SUM of five short pieces.  Each of
-- the first four has at most 24 significant bits, so multiplying it by an
-- integer k < 2^29 needs at most 24 + 29 = 53 bits and is therefore EXACT.
-- Subtracting the five products one at a time peels k*pi/2 off x with about
-- 149 bits of pi/2 behind it.  See section 3.4 of PHY00a for the proof
-- sketch and the error budget (about 2.7 ulp worst case; 2.38 observed).
--
-- ## Lua 5.4 and its two number types
--
-- Lua 5.4 has an integer subtype and a float subtype.  `1` is an integer,
-- `1.0` is a float, and `7 // 2`, `%`, `-x * x` on integers are INTEGER
-- arithmetic that wraps silently on overflow.  Every value in this
-- algorithm must therefore be a float:
--
--   * every constant is written as a float (or built from its bits);
--   * an integer argument is converted once at entry with `x * 1.0`
--     (never `x + 0.0`, which would turn -0.0 into +0.0);
--   * the only integer operation is `math.tointeger(k) & 3`, on a float k
--     that holds a whole number below 2^29, which is exact.

local trig = {}

trig.VERSION = "0.3.0"

-- ============================================================================
-- Constants, built from their bit patterns
-- ============================================================================

--- from_bits reinterprets a 64-bit integer as an IEEE 754 binary64 float.
--
-- A decimal literal is only "the nearest double" if the interpreter's string
-- conversion is correctly rounded -- a property of the platform, not of this
-- code.  Building each constant from its bits removes the doubt: these are
-- exactly the eight bytes that PHY00a 3.2 pins.
--
--     string.pack("<i8", bits)    -- the integer as 8 little-endian bytes
--     string.unpack("<d", bytes)  -- the same 8 bytes read as a double
--
-- Lua hexadecimal integer literals wrap modulo 2^64, so 0xBFC5555555555555
-- (top bit set) is simply a negative integer with the same eight bytes.
local function from_bits(bits)
    return (string.unpack("<d", string.pack("<i8", bits)))
end

--- PI is the ratio of a circle's circumference to its diameter.
-- 0x400921FB54442D18 = 3.141592653589793.
trig.PI = from_bits(0x400921FB54442D18)

--- TWO_PI is the full period of sine and cosine (0x401921FB54442D18).
trig.TWO_PI = from_bits(0x401921FB54442D18)

--- HALF_PI is a quarter turn (0x3FF921FB54442D18); atan and atan2 use it.
trig.HALF_PI = from_bits(0x3FF921FB54442D18)

--- INV_PIO2 is the double nearest 2/pi (0.6366197723675814).
-- Multiplying by it turns "radians" into "quarter turns".
trig.INV_PIO2 = from_bits(0x3FE45F306DC9C883)

--- PIO2 holds pi/2 as five Cody-Waite pieces.  The first four have at most
-- 24 significant bits each (so k * piece is exact for |k| < 2^29); the fifth
-- is the remainder rounded to 53 bits.  Their sum is pi/2 to 149 bits.
trig.PIO2 = {
    from_bits(0x3FF921FB40000000), -- 1.570796251296997
    from_bits(0x3E74442D00000000), -- 7.549789415861596e-08
    from_bits(0x3CF8469880000000), -- 5.390302529957765e-15
    from_bits(0x3B78CC5160000000), -- 3.282003415807913e-22
    from_bits(0x39F01B839A25204A), -- 1.2706558760139879e-29
}

--- MAGIC is 1.5 * 2^52.  For |t| < 2^51, (t + MAGIC) - MAGIC is t rounded to
-- the nearest integer (ties to even), using only an add and a subtract:
-- adding MAGIC pushes the fractional bits off the end of the significand.
trig.MAGIC = from_bits(0x4338000000000000) -- 6755399441055744.0

--- LIMIT is 2^29, the edge of the domain.  Beyond it k would no longer fit
-- the exact products of the reduction, and sin/cos/tan return NaN.
trig.LIMIT = from_bits(0x41C0000000000000) -- 536870912.0

--- TINY is 2^-27.  Below it sin(x) = x and cos(x) = 1 exactly (the next
-- term of the series is under half an ulp).
trig.TINY = from_bits(0x3E40000000000000) -- 7.450580596923828e-09

--- S[n] = (-1)^n / (2n+1)!  for n = 1..9: the sine coefficients.
--
--     S[1] = -1/3!    S[4] = +1/9!    S[7] = -1/15!
--     S[2] = +1/5!    S[5] = -1/11!   S[8] = +1/17!
--     S[3] = -1/7!    S[6] = +1/13!   S[9] = -1/19!
--
-- Each is the exact rational rounded ONCE to the nearest double.
trig.S = {
    from_bits(0xBFC5555555555555), -- S[1] = -1/3!  = -0.16666666666666666
    from_bits(0x3F81111111111111), -- S[2] = +1/5!  =  0.008333333333333333
    from_bits(0xBF2A01A01A01A01A), -- S[3] = -1/7!  = -0.0001984126984126984
    from_bits(0x3EC71DE3A556C734), -- S[4] = +1/9!  =  2.7557319223985893e-06
    from_bits(0xBE5AE64567F544E4), -- S[5] = -1/11! = -2.505210838544172e-08
    from_bits(0x3DE6124613A86D09), -- S[6] = +1/13! =  1.6059043836821613e-10
    from_bits(0xBD6AE7F3E733B81F), -- S[7] = -1/15! = -7.647163731819816e-13
    from_bits(0x3CE952C77030AD4A), -- S[8] = +1/17! =  2.8114572543455206e-15
    from_bits(0xBC62F49B46814157), -- S[9] = -1/19! = -8.22063524662433e-18
}

--- C[n] = (-1)^n / (2n)!  for n = 2..9: the cosine coefficients.
-- (n = 1, the -1/2, is applied as 0.5 * z, which is exact.  Index 1 is left
-- empty so that C[n] lines up with the formula.)
--
--     C[2] = +1/4!    C[5] = -1/10!   C[8] = +1/16!
--     C[3] = -1/6!    C[6] = +1/12!   C[9] = -1/18!
--     C[4] = +1/8!    C[7] = -1/14!
trig.C = {
    [2] = from_bits(0x3FA5555555555555), -- +1/4!  =  0.041666666666666664
    [3] = from_bits(0xBF56C16C16C16C17), -- -1/6!  = -0.001388888888888889
    [4] = from_bits(0x3EFA01A01A01A01A), -- +1/8!  =  2.48015873015873e-05
    [5] = from_bits(0xBE927E4FB7789F5C), -- -1/10! = -2.755731922398589e-07
    [6] = from_bits(0x3E21EED8EFF8D898), -- +1/12! =  2.08767569878681e-09
    [7] = from_bits(0xBDA93974A8C07C9D), -- -1/14! = -1.1470745597729725e-11
    [8] = from_bits(0x3D2AE7F3E733B81F), -- +1/16! =  4.779477332387385e-14
    [9] = from_bits(0xBCA6827863B97D97), -- -1/18! = -1.5619206968586225e-16
}

--- RAD_PER_DEG is the double nearest pi/180 (0.017453292519943295).
trig.RAD_PER_DEG = from_bits(0x3F91DF46A2529D39)

--- DEG_PER_RAD is the double nearest 180/pi (57.29577951308232).
trig.DEG_PER_RAD = from_bits(0x404CA5DC1A63C1F8)

-- Locals for the hot path (one table lookup fewer per use).
local INV_PIO2, MAGIC, LIMIT, TINY = trig.INV_PIO2, trig.MAGIC, trig.LIMIT, trig.TINY
local PIO2, S, C = trig.PIO2, trig.S, trig.C
local abs = math.abs

-- NaN: 0/0 with float operands.  (Its sign bit is platform-dependent, which
-- is why callers compare NaN as a class, never by bits.)
local NAN = 0.0 / 0.0

-- ============================================================================
-- Range Reduction (internal)
-- ============================================================================

--- reduce splits x into k quarter-turns and a remainder r.
--
-- Step 1, the quotient.  t = x * (2/pi) is "x measured in quarter-turns".
-- We want the NEAREST whole number to it, and we get that without floor,
-- round or any float-to-integer cast:
--
--     k = (t + MAGIC) - MAGIC
--
-- MAGIC = 1.5 * 2^52 has its units digit in the very last place of the
-- significand, so the hardware's own round-to-nearest-even rounds t + MAGIC
-- to a whole number; subtracting MAGIC again is exact and hands the integer
-- back as a double.  Because k is the NEAREST integer, |r| <= pi/4 (and not
-- up to pi/2, as truncation would leave).
--
-- Step 2, the remainder, by five separate multiply-then-subtract steps:
--
--     x ------------------------------------------------------ the angle
--     |-- k*PIO2[1] --|                                          24 bits
--                     |-- k*PIO2[2] --|                         next 24
--                                     |-- k*PIO2[3] --|         next 24
--                                                     |- k*PIO2[4] -|
--                                                          |k*PIO2[5]| rounds
--     r = what is left, |r| <= pi/4 (plus a few ulps)
--
-- Each product is written on its own line, so no fused multiply-add can
-- merge it with the subtraction (PHY00a 3.1: one rounding per operation).
-- Lua never fuses, but the spec's rule is the same in every language.
--
-- @param x  a float with |x| <= 2^29
-- @return   k (a float holding a whole number), r (a float)
local function reduce(x)
    local t = x * INV_PIO2
    t = t + MAGIC
    local k = t - MAGIC
    local r = x
    for i = 1, 5 do
        local m = k * PIO2[i]
        r = r - m
    end
    return k, r
end

-- ============================================================================
-- The series kernels (internal)
-- ============================================================================

--- sin_kernel(r) = sin(r) for |r| <= pi/4, by Horner's rule.
--
--     sin(r) = r + r*z*(S1 + S2*z + S3*z^2 + ... + S9*z^8),   z = r*r
--
-- Horner evaluates the polynomial from the inside out: start with the
-- smallest coefficient, and repeat "multiply by z, add the next one".
-- That adds the small terms first (the accurate order), needs no division
-- at all (so no divide-by-zero trap and no per-term rounding error from
-- dividing), and cannot be grouped two ways.  The leading term r is added
-- LAST and unrounded, so the dominant part of the answer carries a single
-- rounding.
local function sin_kernel(r)
    local z = r * r
    local p = S[9]
    for n = 8, 1, -1 do
        p = p * z
        p = p + S[n]
    end
    local w = r * z
    w = w * p
    return r + w
end

--- cos_kernel(r) = cos(r) for |r| <= pi/4, by Horner's rule.
--
--     cos(r) = (1 - z/2) + z^2 * (C2 + C3*z + ... + C9*z^7),   z = r*r
--
-- 0.5 * z is exact (halving never rounds), so 1 - z/2 carries one rounding.
local function cos_kernel(r)
    local z = r * r
    local q = C[9]
    for n = 8, 2, -1 do
        q = q * z
        q = q + C[n]
    end
    local u = 0.5 * z
    u = 1.0 - u
    local v = z * z
    v = v * q
    return u + v
end

-- ============================================================================
-- Sin -- The Sine Function
-- ============================================================================

--- Compute the sine of x (in radians).
--
-- ### The algorithm in four lines
--
--     1. guard:  NaN, +-inf and |x| > 2^29 are outside the domain -> NaN
--     2. tiny:   |x| < 2^-27 -> x itself (exact; keeps sin(-0.0) = -0.0)
--     3. reduce: x = k*(pi/2) + r with |r| <= pi/4
--     4. pick:   the quadrant q = k mod 4 chooses the answer
--
-- ### The quadrant table
--
--     sin(r + k*pi/2) = { sin r, cos r, -sin r, -cos r }[k mod 4]
--
-- ### The guard is a NEGATED comparison, on purpose
--
-- A NaN fails every comparison, so `not (abs(x) <= LIMIT)` is true for NaN,
-- for both infinities and for every finite value beyond 2^29: one test
-- covers all three.  The tempting `abs(x) > LIMIT` would let a NaN through
-- to the reduction.
--
-- ### Why NaN beyond 2^29 rather than a number
--
-- Beyond 2^29 the quotient k no longer fits the exact products of the
-- reduction, and recovering x mod pi/2 would need pi to over a thousand
-- bits (Payne-Hanek).  A finite but wrong "sine" would be indistinguishable
-- from a right one; NaN fails closed.  Callers that accumulate a phase
-- should reduce it to a fractional cycle first (see lua/wave).
--
-- ### Integer arguments
--
-- Lua 5.4 has an integer subtype, whose `-x*x` would wrap.  An integer
-- argument is converted once, with `x * 1.0` (never `x + 0.0`, which turns
-- -0.0 into +0.0; integers have no -0 so either is safe for integers, but
-- the rule is uniform across the ports).
--
-- @param x  angle in radians
-- @return   sine of x (a float)
function trig.sin(x)
    if math.type(x) == "integer" then x = x * 1.0 end
    if not (abs(x) <= LIMIT) then return NAN end
    if abs(x) < TINY then return x end

    local k, r = reduce(x)
    -- k is a float holding a whole number with |k| < 2^29, so tointeger is
    -- exact; "& 3" on two's complement is the floor-modulus 4, so k = -1
    -- gives 3 and k = -5 gives 3.  (tointeger(-0.0) is 0.)
    local q = math.tointeger(k) & 3
    if q == 0 then
        return sin_kernel(r)
    elseif q == 1 then
        return cos_kernel(r)
    elseif q == 2 then
        return -sin_kernel(r)
    else
        return -cos_kernel(r)
    end
end

-- ============================================================================
-- Cos -- The Cosine Function
-- ============================================================================

--- Compute the cosine of x (in radians).
--
-- Same shape as sin, with the shifted quadrant table:
--
--     cos(r + k*pi/2) = { cos r, -sin r, -cos r, sin r }[k mod 4]
--
-- and `cos(x) = 1` below 2^-27.
--
-- ### The Pythagorean Identity
--
-- sin^2(x) + cos^2(x) = 1 for ALL x.  Our test suite verifies this as a
-- sanity check that both functions are implemented correctly.
--
-- @param x  angle in radians
-- @return   cosine of x (a float)
function trig.cos(x)
    if math.type(x) == "integer" then x = x * 1.0 end
    if not (abs(x) <= LIMIT) then return NAN end
    if abs(x) < TINY then return 1.0 end

    local k, r = reduce(x)
    local q = math.tointeger(k) & 3
    if q == 0 then
        return cos_kernel(r)
    elseif q == 1 then
        return -sin_kernel(r)
    elseif q == 2 then
        return -cos_kernel(r)
    else
        return sin_kernel(r)
    end
end

-- ============================================================================
-- Tan -- The Tangent Function
-- ============================================================================

--- Compute the tangent of x (in radians).
--
-- ### Definition
--
--     tan(x) = sin(x) / cos(x)
--
-- ### The pole sentinel (a pinned legacy contract)
--
-- tan blows up wherever cos(x) = 0, i.e. at pi/2 + k*pi.  No double is
-- exactly such a point, so cos is tiny but never zero; the contract
-- (PHY00 3.6, kept by PHY00a 3.8) is a documented sentinel instead of a
-- huge quotient: when |cos| < 1e-15 the result is +1e308 or -1e308 by the
-- sign of the sine.  That is NOT an approximation of the tangent (the true
-- tan of the double nearest pi/2 is about 1.6e16); it is a labelled
-- convention, and no accuracy claim is made for tan at all.
--
-- A NaN cos (NaN, infinity or a huge argument) makes `abs(c) < 1e-15` false,
-- so the quotient NaN / NaN is NaN.  The guard also makes c = 0 unreachable,
-- so the division never divides by zero.
--
-- @param x  angle in radians
-- @return   tangent of x, or +-1e308 at the poles
function trig.tan(x)
    local s = trig.sin(x)
    local c = trig.cos(x)
    if abs(c) < 1e-15 then
        if s > 0.0 then
            return 1e308
        else
            return -1e308
        end
    end
    return s / c
end

-- ============================================================================
-- Angle Conversion
-- ============================================================================

--- Convert an angle from degrees to radians: one multiply by a stored
-- constant.
--
--     radians = degrees * (pi / 180)
--
-- Why a pre-divided constant rather than `deg * PI / 180`: one rounding
-- instead of two; no intermediate `deg * PI` that overflows when radians(deg)
-- is finite; and it is measurably more accurate (0.65 ulp worst case against
-- 1.52).  It still gives radians(180) = PI exactly.
--
-- NaN propagates, +-inf maps to +-inf and +-0 keeps its sign.  An integer
-- argument is promoted by the multiply itself (integer * float is a float).
--
-- @param deg  angle in degrees
-- @return     angle in radians
function trig.radians(deg)
    return deg * trig.RAD_PER_DEG
end

--- Convert an angle from radians to degrees: one multiply by a stored
-- constant.
--
--     degrees = radians * (180 / pi)
--
-- degrees(PI) = 180 exactly.  A product above the largest double overflows
-- to +-inf (degrees(1.7e308) is +inf).
--
-- @param rad  angle in radians
-- @return     angle in degrees
function trig.degrees(rad)
    return rad * trig.DEG_PER_RAD
end

-- ============================================================================
-- sqrt -- Square Root via Newton's (Babylonian) Method
-- ============================================================================

--- Compute the square root of x using Newton's iterative method.
--
-- ### Newton's (Babylonian) Method
--
-- The Babylonian method for square roots is one of the oldest numerical
-- algorithms in human history (~1700 BCE). The key recurrence:
--
--     next_guess = (guess + x / guess) / 2.0
--
-- If guess < sqrt(x), then x/guess > sqrt(x). Their average is closer to
-- the true value. If guess > sqrt(x), the argument is symmetric. We always
-- move closer. This has **quadratic convergence** — the number of correct
-- digits doubles each iteration.
--
-- Convergence table for sqrt(2):
--
--     iter | guess               | correct digits
--     -----|---------------------|---------------
--     0    | 2.0                 | 0
--     1    | 1.5                 | 1
--     2    | 1.41667             | 2
--     3    | 1.41422             | 5
--     4    | 1.41421356237...    | 11+ (full precision)
--
-- ### Error
--
-- Raises an error for negative inputs (real square roots are undefined there).
--
-- @param x   the radicand (must be >= 0)
-- @return    square root of x
function trig.sqrt(x)
    -- Lua 5.4 integer subtype: convert once, so sqrt(0) is 0.0 and not 0.
    -- (x * 1.0 keeps -0.0; x + 0.0 would not.)
    if math.type(x) == "integer" then x = x * 1.0 end
    if x < 0 then
        error("trig.sqrt: domain error — input " .. tostring(x) .. " is negative", 2)
    end

    -- sqrt(0) is exactly 0.
    if x == 0.0 then return x end
    if x == math.huge then return x end

    local scaled = x
    local result_scale = 1.0
    while scaled < 0.25 do
        scaled = scaled * 4.0
        result_scale = result_scale * 0.5
    end
    while scaled >= 4.0 do
        scaled = scaled * 0.25
        result_scale = result_scale * 2.0
    end

    -- Initial guess: x itself for x >= 1 (better for large values),
    -- 1.0 for x in (0, 1) (avoids dividing by a tiny number).
    local guess = (scaled >= 1.0) and scaled or 1.0

    -- Iterate up to 60 times. Quadratic convergence means ~15 in practice.
    for _ = 1, 60 do
        local next_guess = (guess + scaled / guess) / 2.0

        -- Stop when improvement is below the precision floor.
        -- 1e-15 * guess is relative precision; 1e-300 is a subnormal floor.
        if math.abs(next_guess - guess) < 1e-15 * guess + 1e-300 then
            return next_guess * result_scale
        end

        guess = next_guess
    end

    return guess * result_scale
end

-- ============================================================================
-- atan -- Arctangent via Taylor Series with Half-Angle Reduction
-- ============================================================================

-- HALF_PI (pi/2, defined above from its bits) is used in atan's range
-- reduction and atan2's quadrant cases.

-- atan_core computes atan(x) for |x| <= 1 using half-angle reduction
-- followed by the Taylor series. This is an internal helper.
--
-- Half-angle identity:
--   atan(x) = 2 * atan( x / (1 + sqrt(1 + x^2)) )
--
-- After reduction, |reduced| <= tan(pi/8) ~= 0.414, and the Taylor series
-- atan(t) = t - t^3/3 + t^5/5 - ... converges in ~15 terms.
--
-- Iterative term computation:
--   term_0 = reduced
--   term_n = term_{n-1} * (-t^2) * (2n-1) / (2n+1)
local function atan_core(x)
    -- Half-angle reduction. We use our own trig.sqrt -- no math.sqrt.
    local reduced = x / (1.0 + trig.sqrt(1.0 + x * x))

    local t      = reduced
    local t_sq   = t * t
    local term   = t
    local result = t

    for n = 1, 30 do
        -- term_n = term_{n-1} * (-t^2) * (2n-1) / (2n+1)
        term   = term * (-t_sq) * (2 * n - 1) / (2 * n + 1)
        result = result + term

        -- Early exit when the term is negligibly small.
        if math.abs(term) < 1e-17 then break end
    end

    -- Undo the half-angle halving: atan(x) = 2 * atan(reduced).
    return 2.0 * result
end

--- Compute the arctangent of x (in radians).
--
-- Returns a value in the open interval (-pi/2, pi/2).
--
-- ### Range Reduction
--
-- The Taylor series for atan converges only for |x| <= 1. For |x| > 1
-- we use the complementary identity:
--
--     atan(x)  = pi/2 - atan(1/x)    for x > 1
--     atan(x)  = -pi/2 - atan(1/x)   for x < -1
--
-- Proof: atan(x) + atan(1/x) = pi/2 for x > 0.
-- If theta = atan(x), then tan(pi/2 - theta) = 1/x = cot(theta),
-- so atan(1/x) = pi/2 - theta.
--
-- ### Examples
--
--     atan(0)    = 0
--     atan(1)    = pi/4   (45 degrees)
--     atan(-1)   = -pi/4
--
-- @param x   any real number
-- @return    arctangent of x, in (-pi/2, pi/2)
function trig.atan(x)
    -- Keep the public result floating-point for integer arguments without
    -- adding to an existing -0.0, which would clear its sign bit.
    if math.type(x) == "integer" then x = x + 0.0 end
    -- atan(x) rounds exactly to x here; avoid halving subnormals and retain -0.
    if math.abs(x) <= 7.450580596923828e-9 then return x end

    if x > 1.0 then
        return trig.HALF_PI - atan_core(1.0 / x)
    elseif x < -1.0 then
        return -trig.HALF_PI - atan_core(1.0 / x)
    end

    return atan_core(x)
end

-- ============================================================================
-- atan2 -- Four-Quadrant Arctangent
-- ============================================================================

--- Compute the four-quadrant arctangent of (y, x).
--
-- Returns the angle in radians that the vector (x, y) makes with the
-- positive x-axis, in the range (-pi, pi].
--
-- ### Why Not atan(y/x)?
--
-- atan(y/x) only gives angles in (-pi/2, pi/2). It cannot distinguish
-- the second quadrant from the fourth, or the first from the third,
-- because opposite quadrants give the same y/x ratio.
--
-- atan2 inspects the signs of both y and x separately:
--
--     Quadrant I   (x>0, y>0):  atan2 in (0,    pi/2)
--     Quadrant II  (x<0, y>=0): atan2 in [pi/2, pi  ]
--     Quadrant III (x<0, y<0):  atan2 in (-pi, -pi/2)
--     Quadrant IV  (x>0, y<0):  atan2 in (-pi/2, 0  )
--
-- Special cases:
--     atan2(0,  1) =  0       (positive x-axis)
--     atan2(1,  0) =  pi/2    (positive y-axis)
--     atan2(0, -1) =  pi      (negative x-axis)
--     atan2(-1, 0) = -pi/2    (negative y-axis)
--     atan2(0,  0) =  0       (undefined, return 0 by convention)
--
-- @param y   the y-coordinate
-- @param x   the x-coordinate
-- @return    angle in radians, in (-pi, pi]
function trig.atan2(y, x)
    -- A NaN argument fails every comparison below and would fall through to
    -- the last branch, returning 0.0: a finite answer to an undefined
    -- question.  TRIG-REF-1 (PHY00a 3.9) adds this guard.
    if x ~= x or y ~= y then return 0.0 / 0.0 end

    if x > 0.0 then
        return trig.atan(y / x)
    elseif x < 0.0 and y >= 0.0 then
        return trig.atan(y / x) + trig.PI
    elseif x < 0.0 and y < 0.0 then
        return trig.atan(y / x) - trig.PI
    elseif x == 0.0 and y > 0.0 then
        return trig.HALF_PI
    elseif x == 0.0 and y < 0.0 then
        return -trig.HALF_PI
    else
        -- Both zero: undefined by convention, return 0.
        return 0.0
    end
end

return trig
