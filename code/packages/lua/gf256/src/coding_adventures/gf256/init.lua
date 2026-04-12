-- gf256 — Galois Field GF(2^8) arithmetic
--
-- # What Is GF(2^8)?
--
-- GF(2^8) — pronounced "GF of 2 to the 8th" or "GF of 256" — is a finite
-- field with exactly 256 elements: the integers 0 through 255. The elements
-- are bytes. The arithmetic, however, is very different from ordinary integer
-- arithmetic.
--
-- # Why Does This Exist?
--
-- Three important algorithms in modern computing rely on GF(256):
--
--   1. Reed-Solomon error correction — Used in QR codes, CDs, DVDs, and hard
--      drives. RS codes treat data bytes as field elements and perform polynomial
--      arithmetic over GF(256) to add redundancy that can detect and correct
--      burst errors.
--
--   2. QR codes — The error correction codewords in a QR code are a Reed-Solomon
--      code over GF(256). A QR code can survive up to 30% damage because of this.
--
--   3. AES encryption — The SubBytes step (the S-box) and the MixColumns step
--      use arithmetic in GF(2^8).
--
-- # The Primitive Polynomial
--
-- The elements of GF(2^8) are polynomials over GF(2) of degree ≤ 7:
--
--   a₇x⁷ + a₆x⁶ + a₅x⁵ + a₄x⁴ + a₃x³ + a₂x² + a₁x + a₀
--
-- where each aᵢ ∈ {0, 1} (one bit). This gives 2^8 = 256 elements.
--
-- To multiply two such polynomials, the product can have degree up to 14. We
-- reduce it modulo an irreducible polynomial of degree 8, just as integers
-- modulo a prime give a finite field.
--
-- We use the Reed-Solomon primitive polynomial:
--
--   p(x) = x^8 + x^4 + x^3 + x^2 + 1  =  0x11D  =  285
--
-- In binary: 1_0001_1101 = bit8, bit4, bit3, bit2, bit0.
--
-- # Characteristic 2: Add = XOR = Subtract
--
-- In characteristic-2 fields, 1 + 1 = 0. Concretely, for bytes:
--
--   add(a, b) = a XOR b
--   subtract(a, b) = a XOR b      ← the same operation!
--
-- Each bit of a byte is an element of GF(2) = {0, 1}, and GF(2) addition is
-- 1+1=0 mod 2, which is XOR. There is no carry: XOR is exact.
--
-- # Log/Antilog Tables: Fast Multiplication
--
-- The 255 non-zero elements of GF(256) form a cyclic group under multiplication.
-- The generator g = 2 satisfies:
--
--   g^0=1, g^1=2, g^2=4, ..., g^254, g^255=1  (cycles back)
--
-- Every non-zero element appears exactly once in this list.
--
-- This means we can represent multiplication using logarithms:
--
--   a × b = g^(log_g(a) + log_g(b))
--
-- which turns a multiplication into two table lookups and an addition mod 255.
--
-- # Log/Antilog Table Construction
--
-- ALOG[i] = g^i mod p(x)   (antilogarithm: exponent → field element)
-- LOG[x]  = i such that g^i = x  (logarithm: field element → exponent)
--
-- Algorithm:
--   Start with val = 1. Each step: multiply by 2 (left-shift 1 bit).
--   If the result overflows a byte (bit 8 is set), XOR with 0x11D to reduce
--   modulo the primitive polynomial.
--
--   ALOG[0] = 1
--   for i = 1 to 254:
--       val = ALOG[i-1] << 1
--       if val >= 256:
--           val = val XOR 0x11D
--       ALOG[i] = val
--       LOG[val] = i
--
-- First 10 ALOG entries (powers of 2 mod 0x11D):
--   ALOG[0]  = 1     (2^0)
--   ALOG[1]  = 2     (2^1)
--   ALOG[2]  = 4     (2^2)
--   ALOG[3]  = 8     (2^3)
--   ALOG[4]  = 16    (2^4)
--   ALOG[5]  = 32    (2^5)
--   ALOG[6]  = 64    (2^6)
--   ALOG[7]  = 128   (2^7 = 0x80)
--   ALOG[8]  = 29    (256 XOR 0x11D = 0x100 XOR 0x11D = 0x01D = 29)
--   ALOG[9]  = 58    (29 * 2 = 58, no overflow)
--
-- This is Layer MA01 of the coding-adventures math stack.
-- It depends on MA00 (polynomial) for the conceptual foundation.
--
-- # Quick Start
--
--   local gf = require("coding_adventures.gf256")
--
--   local a = 0x53
--   local b = 0x8C
--   print(gf.multiply(a, b))   -- 1 (they are multiplicative inverses)
--   print(gf.add(a, a))        -- 0 (every element is its own additive inverse)

local M = {}

M.VERSION    = "0.1.0"
M.ZERO       = 0
M.ONE        = 1

-- The primitive (irreducible) polynomial used for modular reduction.
-- p(x) = x^8 + x^4 + x^3 + x^2 + 1
-- Binary: 1_0001_1101 = 0x11D = 285
M.PRIMITIVE_POLYNOMIAL = 0x11D

-- ============================================================================
-- Build the LOG and ALOG tables at module load time.
-- ============================================================================
--
-- We use local variables (not stored in M) because the tables are an
-- implementation detail. They are read-only once built.
--
-- ALOG has 256 entries: indices 1..255 are standard (g^0 through g^254);
-- index 256 holds 1 (ALOG[255] = 1 conceptually, stored at Lua index 256
-- because Lua arrays are 1-indexed).
--
-- Note on Lua indexing: we use 1-based tables throughout, so:
--   ALOG[i+1] corresponds to g^i  (exponent i stored at Lua index i+1)
--   LOG[x+1]  corresponds to LOG[x] (field element x stored at Lua index x+1)
--
-- This way we avoid the need for 0-indexed access, which Lua does not support
-- natively. Callers do not interact with these tables directly.

local LOG  = {}   -- LOG[x+1]  = log_g(x), for x in 0..255
local ALOG = {}   -- ALOG[i+1] = g^i,      for i in 0..255

do
    -- Initialize to 0 (LOG[0] is never used for valid inputs).
    for i = 1, 256 do
        LOG[i]  = 0
        ALOG[i] = 0
    end

    local val = 1
    for i = 0, 254 do
        -- Store g^i at ALOG[i+1].
        ALOG[i + 1] = val
        -- Store the inverse mapping: LOG[val+1] = i.
        LOG[val + 1] = i

        -- Multiply val by 2 (the generator g = x, represented as bit 1 set).
        -- In GF(2^8), multiplying by x shifts all polynomial coefficients
        -- up by one degree. If the degree-8 coefficient becomes 1 (bit 8 set,
        -- meaning val >= 256), we reduce modulo p(x) by XOR-ing with 0x11D.
        val = val << 1
        if val >= 256 then
            val = val ~ M.PRIMITIVE_POLYNOMIAL   -- XOR in Lua 5.4 is ~
        end
    end

    -- ALOG[255+1] = ALOG[256] = 1: the multiplicative group has order 255,
    -- so g^255 = g^0 = 1. This entry is used by inverse(1):
    --   inverse(1) = ALOG[255 - LOG[1] + 1] = ALOG[255 - 0 + 1] = ALOG[256] = 1  ✓
    ALOG[256] = 1
end

-- ============================================================================
-- add(a, b) — GF(256) addition
-- ============================================================================
--
-- In a characteristic-2 field, addition is XOR. Each bit represents a GF(2)
-- coefficient, and GF(2) addition satisfies 1+1=0 mod 2, which is XOR.
--
-- No overflow, no carry, no tables needed.
--
-- Truth table for a single bit:
--   0 + 0 = 0    (0 XOR 0 = 0)
--   0 + 1 = 1    (0 XOR 1 = 1)
--   1 + 0 = 1    (1 XOR 0 = 1)
--   1 + 1 = 0    (1 XOR 1 = 0)   ← characteristic 2: 1+1=0
--
-- Examples:
--   add(0x53, 0xCA) = 0x53 XOR 0xCA = 0x99
--   add(x, x) = 0 for all x  (every element is its own additive inverse)
--
function M.add(a, b)
    return a ~ b   -- Lua 5.4 bitwise XOR operator
end

-- ============================================================================
-- subtract(a, b) — GF(256) subtraction
-- ============================================================================
--
-- In characteristic 2, subtraction equals addition (since -1 = 1 in GF(2)).
-- subtract(a, b) = add(a, b) = a XOR b.
--
-- This simplifies error-correction algorithms: syndrome computation via
-- subtraction uses the same hardware as addition — just XOR.
--
function M.subtract(a, b)
    return a ~ b
end

-- ============================================================================
-- multiply(a, b) — GF(256) multiplication via log/antilog tables
-- ============================================================================
--
-- The mathematical identity: a × b = g^(log_g(a) + log_g(b))
-- where g = 2 is our generator.
--
-- Why log/antilog tables?
--   Direct polynomial multiplication (carry-less multiplication mod p(x)) is
--   correct but requires more operations. The table approach turns the entire
--   multiplication into two lookups and one addition modulo 255 — O(1) time.
--
-- Special case: if either operand is 0, the result is 0.
--   Zero has no logarithm (there is no power of g that equals 0). So we handle
--   it explicitly before looking up LOG.
--
-- Example: multiply(3, 7)
--   LOG[3+1] = LOG[4] = some exponent i such that 2^i = 3
--   LOG[7+1] = LOG[8] = some exponent j such that 2^j = 7
--   result = ALOG[(i + j) mod 255 + 1]
--
function M.multiply(a, b)
    if a == 0 or b == 0 then return 0 end
    local exp = (LOG[a + 1] + LOG[b + 1]) % 255
    return ALOG[exp + 1]
end

-- ============================================================================
-- divide(a, b) — GF(256) division
-- ============================================================================
--
-- a / b = g^(log_g(a) - log_g(b)) = ALOG[(LOG[a] - LOG[b] + 255) mod 255]
--
-- The `+ 255` before the modulo ensures the result is non-negative when
-- LOG[a] < LOG[b]. Without it, Lua's `%` operator could return a negative
-- number for negative operands.
--
-- Special case: a = 0 → result is 0 (0 / anything = 0 in any field).
--
-- @error if b == 0: division by zero is undefined in any field.
--
function M.divide(a, b)
    if b == 0 then
        error("GF256: division by zero")
    end
    if a == 0 then return 0 end
    local exp = (LOG[a + 1] - LOG[b + 1] + 255) % 255
    return ALOG[exp + 1]
end

-- ============================================================================
-- power(base, exp) — GF(256) exponentiation
-- ============================================================================
--
-- Raise a GF(256) element to a non-negative integer power.
--
-- Uses the logarithm table:
--   base^exp = ALOG[(LOG[base] * exp) mod 255]
--
-- The modulo 255 reflects the order of the multiplicative group: every non-zero
-- element satisfies g^255 = 1 (Fermat's little theorem for finite fields).
--
-- Special cases:
--   0^0 = 1  by convention (consistent with most numeric libraries)
--   0^n = 0  for n > 0
--   x^0 = 1  for any non-zero x
--
-- Note: The `(... % 255 + 255) % 255` pattern handles negative intermediate
-- results from Lua's `%` operator gracefully.
--
function M.power(base, exp)
    if base == 0 then
        if exp == 0 then return 1 end
        return 0
    end
    if exp == 0 then return 1 end
    local e = ((LOG[base + 1] * exp) % 255 + 255) % 255
    return ALOG[e + 1]
end

-- ============================================================================
-- inverse(a) — multiplicative inverse
-- ============================================================================
--
-- The multiplicative inverse of a satisfies: a × inverse(a) = 1.
--
-- By the cyclic group property:
--   a × a^(-1) = 1 = g^0 = g^255
--   So log(a) + log(a^(-1)) ≡ 0 (mod 255)
--   Therefore log(a^(-1)) = 255 - log(a)
--   And a^(-1) = ALOG[255 - LOG[a]]
--
-- This operation is fundamental to Reed-Solomon decoding and AES SubBytes.
--
-- @error if a == 0: zero has no multiplicative inverse in any field.
--
function M.inverse(a)
    if a == 0 then
        error("GF256: zero has no multiplicative inverse")
    end
    return ALOG[255 - LOG[a + 1] + 1]
end

-- ============================================================================
-- new_field(polynomial) — parameterizable field factory
-- ============================================================================
--
-- The functions above are fixed to the Reed-Solomon polynomial 0x11D.
-- AES uses 0x11B. new_field(poly) builds an independent field table for any
-- primitive polynomial, returning a table with the same API as this module.
--
-- Usage:
--   local gf = require("coding_adventures.gf256")
--   local aes = gf.new_field(0x11B)
--   print(aes.multiply(0x53, 0xCA))   -- 1 (AES GF(2^8) inverses)
--   print(aes.multiply(0x57, 0x83))   -- 0xC1 (FIPS 197 Appendix B)
--
-- Note on Lua 1-based indexing: the field tables use the same i+1 convention
-- as the module-level LOG and ALOG tables above.
--
function M.new_field(polynomial)
    -- Russian peasant (shift-and-XOR) multiplication for GF(2^8).
    --
    -- Log/antilog tables require a primitive generator g such that g^1..g^255
    -- visits all 255 non-zero field elements. g=2 works for 0x11D but is NOT
    -- primitive for 0x11B — AES uses g=0x03 (x+1) per FIPS 197 §4.1. Building
    -- tables with g=2 and 0x11B leaves most log entries at 0, giving wrong
    -- results. Russian peasant needs no generator assumption.
    --
    -- reduce = low byte of the polynomial, used for the overflow reduction step.
    local reduce = polynomial & 0xFF

    -- gf_mul(a, b): multiply a and b in GF(2^8) via Russian peasant.
    local function gf_mul(a, b)
        local result = 0
        local aa = a
        local bb = b
        for _ = 1, 8 do
            if bb & 1 ~= 0 then result = result ~ aa end
            local hi = aa & 0x80
            aa = (aa << 1) & 0xFF
            if hi ~= 0 then aa = aa ~ reduce end
            bb = bb >> 1
        end
        return result
    end

    -- gf_pow(base, exp): repeated squaring.
    -- inverse(a) = gf_pow(a, 254) since a^255 = 1 in GF(2^8).
    local function gf_pow(base, exp)
        if base == 0 then
            if exp == 0 then return 1 end
            return 0
        end
        if exp == 0 then return 1 end
        local result = 1
        local b = base
        local e = exp
        while e > 0 do
            if e & 1 ~= 0 then result = gf_mul(result, b) end
            b = gf_mul(b, b)
            e = e >> 1
        end
        return result
    end

    -- Return a table with the same API as the module.
    local F = {}

    -- add and subtract are polynomial-independent (always XOR).
    function F.add(a, b) return a ~ b end
    function F.subtract(a, b) return a ~ b end

    function F.multiply(a, b)
        return gf_mul(a, b)
    end

    function F.divide(a, b)
        if b == 0 then error("GF256Field: division by zero") end
        return gf_mul(a, gf_pow(b, 254))
    end

    function F.power(base, exp)
        return gf_pow(base, exp)
    end

    function F.inverse(a)
        if a == 0 then error("GF256Field: zero has no multiplicative inverse") end
        return gf_pow(a, 254)
    end

    F.polynomial = polynomial

    return F
end

return M
