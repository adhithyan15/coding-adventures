-- polynomial — Coefficient-array polynomial arithmetic over real numbers
--
-- # What Is a Polynomial?
--
-- A polynomial is a mathematical expression built from a variable x and a
-- list of constant coefficients. For example:
--
--   3 + 0x + 2x²
--
-- has three terms: a constant (3), a degree-1 term (0·x), and a degree-2
-- term (2·x²). The number attached to each power of x is its "coefficient".
--
-- # How We Store Polynomials
--
-- We represent a polynomial as a Lua array (table) of numbers, where the
-- array INDEX equals the DEGREE of that term's coefficient. Because Lua
-- arrays are 1-indexed, we shift by one: index k+1 holds the coefficient
-- of x^k.
--
--   {3, 0, 2}      means  3 + 0·x + 2·x²
--    ↑  ↑  ↑
--    |  |  └── index 3 = coefficient of x²  (degree 2)
--    |  └───── index 2 = coefficient of x¹  (degree 1)
--    └──────── index 1 = coefficient of x⁰  (degree 0, the constant)
--
-- This "little-endian" layout (lowest degree first) makes addition trivial
-- (just add corresponding slots) and keeps Horner's method easy to read.
--
-- # The Zero Polynomial
--
-- The zero polynomial has no non-zero terms. We represent it as {0} (a
-- single-element array with value 0). The normalize() function always
-- ensures at least one element remains, so we never return an empty table.
--
-- # Why This Package Matters
--
-- Polynomial arithmetic is the mathematical foundation for:
--   1. GF(2^8) — Galois field arithmetic used in AES and Reed-Solomon.
--   2. Reed-Solomon error correction — Used in QR codes, CDs, hard drives.
--   3. CRC checksums — A CRC is the remainder after polynomial division.
--
-- This is Layer MA00 of the coding-adventures math stack.
--
-- # Quick Start
--
--   local poly = require("coding_adventures.polynomial")
--
--   local a = {1, 2, 3}   -- 1 + 2x + 3x²
--   local b = {4, 5}      -- 4 + 5x
--
--   local sum = poly.add(a, b)           -- {5, 7, 3}
--   local deg = poly.degree(a)           -- 2
--   local val = poly.evaluate(a, 2)      -- 1 + 4 + 12 = 17

local M = {}

M.VERSION = "0.1.0"

-- ============================================================================
-- normalize(poly)
-- ============================================================================
--
-- Strip trailing near-zero coefficients so that the result is in canonical
-- form: the highest-index entry is non-zero (or the result is {0}).
--
-- "Near-zero" means |coeff| < 1e-10, which handles floating-point rounding
-- errors in division and multiplication. For example, 1e-16 is treated as 0.
--
-- Why normalize?
--   Without normalization, {1, 0, 0} and {1} would look different even
--   though they represent the same constant polynomial 1. The degree() and
--   divmod() functions rely on an accurate degree, so we must strip trailing
--   zeros.
--
-- Examples:
--   normalize({1, 0, 0})  →  {1}
--   normalize({0})        →  {0}
--   normalize({3, 2, 1})  →  {3, 2, 1}  (already normalized)
--
local EPSILON = 1e-10

function M.normalize(poly)
    -- Find the last index with a non-near-zero coefficient.
    local last = #poly
    while last > 1 and math.abs(poly[last]) < EPSILON do
        last = last - 1
    end

    -- If everything is zero, return {0} (never an empty table in Lua).
    if last == 1 and math.abs(poly[1]) < EPSILON then
        return {0}
    end

    -- Copy only the meaningful prefix into a new table.
    local result = {}
    for i = 1, last do
        result[i] = poly[i]
    end
    return result
end

-- ============================================================================
-- degree(poly)
-- ============================================================================
--
-- Return the degree of a polynomial — the index of the highest non-zero
-- coefficient minus 1 (because Lua arrays are 1-indexed, degree = index - 1).
--
-- Degree conventions:
--   degree({3, 0, 2}) = 2    (highest non-zero is at index 3 → degree 2)
--   degree({7})       = 0    (constant polynomial; only a degree-0 term)
--   degree({0})       = -1   (zero polynomial; degree is -1 by convention)
--
-- Why -1 for the zero polynomial?
--   The polynomial long division loop stops when degree(remainder) < degree(divisor).
--   Using -1 for the zero polynomial ensures the loop terminates cleanly,
--   because any valid divisor has degree ≥ 0.
--
function M.degree(poly)
    local n = M.normalize(poly)
    -- If the only element is (near-)zero, it's the zero polynomial.
    if #n == 1 and math.abs(n[1]) < EPSILON then
        return -1
    end
    -- The degree equals the last index minus 1 (Lua is 1-indexed).
    return #n - 1
end

-- ============================================================================
-- zero() and one()
-- ============================================================================
--
-- Convenience constructors for the additive identity (zero) and the
-- multiplicative identity (one).
--
-- zero() is the additive identity: add(zero(), p) = p for any p.
-- one()  is the multiplicative identity: multiply(one(), p) = p for any p.
--
function M.zero()
    return {0}
end

function M.one()
    return {1}
end

-- ============================================================================
-- add(a, b)
-- ============================================================================
--
-- Add two polynomials term-by-term: add coefficients at matching indices,
-- extending the shorter polynomial with implicit zeros.
--
-- Visual example:
--   {1, 2, 3}   =  1 + 2x + 3x²
-- + {4, 5}      =  4 + 5x
-- ─────────────────────────────
--   {5, 7, 3}   =  5 + 7x + 3x²
--
-- Step-by-step:
--   index 1: 1 + 4 = 5
--   index 2: 2 + 5 = 7
--   index 3: 3 + 0 = 3  (b has no index-3 entry → treat as 0)
--
function M.add(a, b)
    local len = math.max(#a, #b)
    local result = {}
    for i = 1, len do
        local ai = a[i] or 0
        local bi = b[i] or 0
        result[i] = ai + bi
    end
    return M.normalize(result)
end

-- ============================================================================
-- subtract(a, b)
-- ============================================================================
--
-- Subtract polynomial b from polynomial a term-by-term.
--
-- This is equivalent to add(a, negate(b)), implemented directly to avoid
-- allocating an intermediate negated polynomial.
--
-- Visual example:
--   {5, 7, 3}   =  5 + 7x + 3x²
-- - {1, 2, 3}   =  1 + 2x + 3x²
-- ─────────────────────────────
--   {4, 5, 0}   →  normalize  →  {4, 5}
--
-- Note: 3x² - 3x² = 0; normalize strips that trailing zero.
--
function M.subtract(a, b)
    local len = math.max(#a, #b)
    local result = {}
    for i = 1, len do
        local ai = a[i] or 0
        local bi = b[i] or 0
        result[i] = ai - bi
    end
    return M.normalize(result)
end

-- ============================================================================
-- multiply(a, b)
-- ============================================================================
--
-- Multiply two polynomials using polynomial convolution.
--
-- Each term a[i]·x^(i-1) multiplies each term b[j]·x^(j-1), contributing
-- a[i]·b[j] to the coefficient of x^(i+j-2), which is stored at index i+j-1.
--
-- If a has degree m and b has degree n, the result has degree m+n.
-- Length: a_len + b_len - 1.
--
-- Visual example:
--   {1, 2}  =  1 + 2x
-- × {3, 4}  =  3 + 4x
-- ────────────────────────────────────────────
-- result = {0, 0, 0}  (length = 2 + 2 - 1 = 3)
--   i=1, j=1: result[1+1-1=1] += 1·3 = 3   → {3, 0, 0}
--   i=1, j=2: result[1+2-1=2] += 1·4 = 4   → {3, 4, 0}
--   i=2, j=1: result[2+1-1=2] += 2·3 = 6   → {3, 10, 0}
--   i=2, j=2: result[2+2-1=3] += 2·4 = 8   → {3, 10, 8}
--
-- Result: {3, 10, 8}  =  3 + 10x + 8x²
-- Verify: (1+2x)(3+4x) = 3+4x+6x+8x² = 3+10x+8x²  ✓
--
function M.multiply(a, b)
    -- Multiplying by the zero polynomial yields the zero polynomial.
    if M.degree(a) == -1 or M.degree(b) == -1 then
        return {0}
    end

    local result_len = #a + #b - 1
    local result = {}
    for k = 1, result_len do
        result[k] = 0
    end

    for i = 1, #a do
        for j = 1, #b do
            result[i + j - 1] = result[i + j - 1] + a[i] * b[j]
        end
    end

    return M.normalize(result)
end

-- ============================================================================
-- divmod(dividend, divisor)
-- ============================================================================
--
-- Perform polynomial long division: given a and b (b ≠ zero), find q and r
-- such that:
--   a = b × q + r     and     degree(r) < degree(b)
--
-- This is the same algorithm as integer long division, adapted for polynomials.
--
-- Step-by-step example: divide {5, 1, 3, 2} = 5 + x + 3x² + 2x³  by  {2, 1} = 2 + x
--
--   We work with a mutable copy of the dividend as our "remainder".
--
--   Step 1: remainder = {5, 1, 3, 2}, deg=3.  Divisor deg=1.  3 ≥ 1 → proceed.
--           leading rem  = 2 (at index 4, degree 3)
--           leading div  = 1 (at index 2, degree 1)
--           quotient term: coeff = 2/1 = 2, degree = 3-1 = 2  → q[3] = 2
--           Subtract 2·x² · (2+x) = {0,0,4,2} from remainder:
--           {5, 1, 3-4, 2-2} = {5, 1, -1, 0} → normalize → {5, 1, -1}
--
--   Step 2: remainder = {5, 1, -1}, deg=2.  2 ≥ 1 → proceed.
--           leading rem  = -1 (index 3, degree 2)
--           quotient term: coeff = -1/1 = -1, degree = 2-1 = 1 → q[2] = -1
--           Subtract -x · (2+x) = {0,-2,-1}:
--           {5, 1-(-2), -1-(-1)} = {5, 3, 0} → {5, 3}
--
--   Step 3: remainder = {5, 3}, deg=1.  1 ≥ 1 → proceed.
--           leading rem  = 3 (index 2, degree 1)
--           quotient term: coeff = 3/1 = 3, degree = 1-1 = 0 → q[1] = 3
--           Subtract 3 · (2+x) = {6, 3}:
--           {5-6, 3-3} = {-1, 0} → {-1}
--
--   Step 4: remainder = {-1}, deg=0 < 1 = deg(b). STOP.
--   Result: q = {3, -1, 2}  (3 - x + 2x²)
--           r = {-1}        (-1)
--   Verify: (x+2)(3-x+2x²) + (-1) = 3x-x²+2x³+6-2x+4x² - 1 = 5+x+3x²+2x³  ✓
--
function M.divmod(dividend, divisor)
    local nb = M.normalize(divisor)
    if M.degree(nb) == -1 then
        error("polynomial division by zero")
    end

    local na = M.normalize(dividend)
    local deg_a = M.degree(na)
    local deg_b = M.degree(nb)

    -- If dividend has lower degree than divisor, quotient = 0, remainder = dividend.
    if deg_a < deg_b then
        return {0}, na
    end

    -- Work on a mutable copy of the remainder.
    local rem = {}
    for i = 1, #na do
        rem[i] = na[i]
    end

    -- Allocate quotient with the correct length.
    local quot_len = deg_a - deg_b + 1
    local quot = {}
    for i = 1, quot_len do
        quot[i] = 0
    end

    -- Leading coefficient of the divisor (used to compute each quotient term).
    local lead_b = nb[deg_b + 1]

    -- Current effective degree of the remainder.
    local deg_rem = deg_a

    while deg_rem >= deg_b do
        -- Leading coefficient of the current remainder.
        local lead_rem = rem[deg_rem + 1]

        -- Coefficient and degree of the next quotient term.
        local coeff = lead_rem / lead_b
        local power = deg_rem - deg_b

        -- Store in quotient array (power+1 because Lua is 1-indexed).
        quot[power + 1] = coeff

        -- Subtract coeff·x^power·b from the remainder.
        for j = 1, #nb do
            local idx = power + j  -- Lua index for degree (power + j - 1)
            rem[idx] = rem[idx] - coeff * nb[j]
        end

        -- Walk deg_rem backwards past any new trailing zeros.
        deg_rem = deg_rem - 1
        while deg_rem >= 0 and math.abs(rem[deg_rem + 1]) < EPSILON do
            deg_rem = deg_rem - 1
        end
    end

    return M.normalize(quot), M.normalize(rem)
end

-- ============================================================================
-- divide(a, b) and modulo(a, b)
-- ============================================================================
--
-- Convenience wrappers that return only the quotient or only the remainder.
--
function M.divide(a, b)
    local q, _ = M.divmod(a, b)
    return q
end

function M.modulo(a, b)
    local _, r = M.divmod(a, b)
    return r
end

-- ============================================================================
-- evaluate(poly, x)
-- ============================================================================
--
-- Evaluate a polynomial at a given point using Horner's method.
--
-- Naïve evaluation of a₀ + a₁x + a₂x² + ... + aₙxⁿ needs n multiplications
-- and n additions, PLUS n exponentiations (x², x³, ..., xⁿ).
--
-- Horner's method eliminates the exponentiations by rewriting in nested form:
--   a₀ + x(a₁ + x(a₂ + ... + x·aₙ))
--
-- This uses only n additions and n multiplications — a significant speedup
-- for high-degree polynomials.
--
-- Algorithm (reading coefficients from HIGH degree DOWN to 0):
--   acc = 0
--   for i from degree(p) downto 0:
--       acc = acc * x + p[i]   (p[i] = p[i+1] in Lua 1-indexed terms)
--   return acc
--
-- Example: evaluate {3, 1, 2} = 3 + x + 2x² at x = 4:
--   Start: acc = 0
--   i=2 (index 3): acc = 0·4 + 2 = 2
--   i=1 (index 2): acc = 2·4 + 1 = 9
--   i=0 (index 1): acc = 9·4 + 3 = 39
--   Verify: 3 + 4 + 2·16 = 3 + 4 + 32 = 39  ✓
--
function M.evaluate(poly, x)
    local n = M.normalize(poly)
    -- Zero polynomial evaluates to 0 everywhere.
    if M.degree(n) == -1 then
        return 0
    end

    local acc = 0
    -- Iterate from the high-degree term down to the constant term.
    for i = #n, 1, -1 do
        acc = acc * x + n[i]
    end
    return acc
end

-- ============================================================================
-- gcd(a, b)
-- ============================================================================
--
-- Compute the greatest common divisor (GCD) of two polynomials.
--
-- The GCD is the highest-degree monic polynomial that divides both a and b
-- with zero remainder. "Monic" means the leading coefficient is 1.
--
-- We use the Euclidean algorithm, which is exactly the same as for integers,
-- with polynomial mod (remainder after division) in place of integer mod:
--
--   gcd(a, b):
--       while b ≠ zero:
--           a, b = b, a mod b
--       return a (normalized)
--
-- Why does this work?
--   The key insight is: gcd(a, b) = gcd(b, a mod b). This is because any
--   polynomial that divides both a and b also divides a - b·q = r. So the
--   common divisors of (a, b) are exactly the common divisors of (b, r).
--
-- Example: gcd({6, 7, 1} = 6+7x+x², {6, 5, 1} = 6+5x+x²)
--   Round 1: r = {6,7,1} mod {6,5,1} = {2}  (a constant)
--   Round 2: r = {6,5,1} mod {2} = {0}  (any poly is divisible by a constant)
--   Stop: return normalize({2}) = {2}
--   Meaning: the two polynomials share no common factor other than constants.
--
-- The result is normalized (but may not be monic in real-number arithmetic,
-- since we don't divide by the leading coefficient here).
--
function M.gcd(a, b)
    local u = M.normalize(a)
    local v = M.normalize(b)

    while M.degree(v) >= 0 do
        local r = M.modulo(u, v)
        u = v
        v = r
    end

    return M.normalize(u)
end

return M
