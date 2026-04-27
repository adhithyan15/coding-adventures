-- ============================================================================
-- Ed25519: Digital Signatures on the Edwards Curve (RFC 8032)
-- ============================================================================
--
-- Ed25519 is an elliptic curve digital signature algorithm (EdDSA) designed by
-- Daniel J. Bernstein et al. It uses the twisted Edwards curve:
--
--     -x^2 + y^2 = 1 + d*x^2*y^2    (mod p)
--
-- where p = 2^255 - 19. Ed25519 provides:
--   - 32-byte public keys and 64-byte signatures
--   - 128-bit security level
--   - Deterministic signatures (no random nonce needed)
--   - Fast signing and verification
--
-- ARCHITECTURE
-- ============
-- We need three layers of arithmetic:
--
--   1. BIG INTEGER: Lua 5.4 has 64-bit integers but no arbitrary precision.
--      We represent large numbers as arrays of 30-bit "limbs" (little-endian).
--      Each limb stores a value in [0, 2^30-1]. The product of two 30-bit
--      limbs is at most 2^60, which fits in a signed 64-bit integer.
--
--   2. FIELD ARITHMETIC (mod p = 2^255 - 19): Addition, subtraction,
--      multiplication, inversion, and square root over GF(p). We reuse the
--      fast reduction from our X25519 implementation: since p = 2^255 - 19,
--      we have 2^255 = 19 (mod p), so we split at bit 255 and fold.
--
--   3. SCALAR ARITHMETIC (mod L): The group order L is a 253-bit prime.
--      We need mod-L reduction for 512-bit SHA-512 outputs. We use generic
--      big integer division for this.
--
-- EXTENDED COORDINATES
-- ====================
-- Points on the curve are represented as (X, Y, Z, T) where:
--   x = X/Z,  y = Y/Z,  T = X*Y/Z
--
-- The identity point is (0, 1, 1, 0) -- affine (0, 1).
-- This representation allows "unified" addition: the same formula works for
-- all point pairs, including doubling and adding the identity.
-- ============================================================================

local sha512 = require("coding_adventures.sha512")

local M = {}
M.VERSION = "0.1.0"

-- ============================================================================
-- PART 1: BIG INTEGER ARITHMETIC
-- ============================================================================
-- We represent integers as {limb1, limb2, ..., n=count} where each limb is
-- a 30-bit unsigned value. The number's value is:
--   limbs[1] + limbs[2]*2^30 + limbs[3]*2^60 + ...

local LIMB_BITS = 30
local LIMB_MASK = (1 << LIMB_BITS) - 1  -- 0x3FFFFFFF

-- ---------------------------------------------------------------------------
-- Constructors
-- ---------------------------------------------------------------------------

-- Create a big integer from a small Lua integer (fits in 64 bits).
local function bi_from_int(n)
    if n == 0 then
        return {0, n = 1}
    end
    local r = {n = 0}
    while n > 0 do
        r.n = r.n + 1
        r[r.n] = n & LIMB_MASK
        n = n >> LIMB_BITS
    end
    return r
end

-- Get limb i (1-indexed), returning 0 beyond stored range.
local function bi_limb(a, i)
    if i <= a.n then return a[i] else return 0 end
end

-- Remove leading zero limbs.
local function bi_normalize(a)
    while a.n > 1 and a[a.n] == 0 do
        a[a.n] = nil
        a.n = a.n - 1
    end
    return a
end

-- ---------------------------------------------------------------------------
-- Byte Conversions (Little-Endian)
-- ---------------------------------------------------------------------------

-- Decode a byte table (1-indexed, values 0-255) into a big integer.
-- The bytes are little-endian: bytes[1] is the least significant.
local function bi_from_byte_table(bytes)
    local r = {n = 0}
    local bit_acc = 0
    local bit_count = 0

    for i = 1, #bytes do
        bit_acc = bit_acc | (bytes[i] << bit_count)
        bit_count = bit_count + 8

        while bit_count >= LIMB_BITS do
            r.n = r.n + 1
            r[r.n] = bit_acc & LIMB_MASK
            bit_acc = bit_acc >> LIMB_BITS
            bit_count = bit_count - LIMB_BITS
        end
    end

    if bit_count > 0 or r.n == 0 then
        r.n = r.n + 1
        r[r.n] = bit_acc & LIMB_MASK
    end

    return bi_normalize(r)
end

-- Decode a binary string (little-endian) into a big integer.
local function bi_from_bytes(s)
    local bytes = {}
    for i = 1, #s do
        bytes[i] = string.byte(s, i)
    end
    return bi_from_byte_table(bytes)
end

-- Encode a big integer as a byte table of the given length (little-endian).
local function bi_to_byte_table(a, len)
    local bytes = {}
    local bit_acc = 0
    local bit_count = 0
    local limb_idx = 1

    for _ = 1, len do
        while bit_count < 8 do
            bit_acc = bit_acc | (bi_limb(a, limb_idx) << bit_count)
            bit_count = bit_count + LIMB_BITS
            limb_idx = limb_idx + 1
        end
        bytes[#bytes + 1] = bit_acc & 0xFF
        bit_acc = bit_acc >> 8
        bit_count = bit_count - 8
    end

    return bytes
end

-- Encode a big integer as a binary string of the given length (little-endian).
local function bi_to_bytes(a, len)
    local bytes = bi_to_byte_table(a, len)
    local chars = {}
    for i = 1, #bytes do
        chars[i] = string.char(bytes[i])
    end
    return table.concat(chars)
end

-- ---------------------------------------------------------------------------
-- Comparison
-- ---------------------------------------------------------------------------

-- Compare two big integers. Returns -1, 0, or 1.
local function bi_cmp(a, b)
    local max_n = math.max(a.n, b.n)
    for i = max_n, 1, -1 do
        local al = bi_limb(a, i)
        local bl = bi_limb(b, i)
        if al < bl then return -1 end
        if al > bl then return 1 end
    end
    return 0
end

-- Check if a big integer is zero.
local function bi_is_zero(a)
    for i = 1, a.n do
        if a[i] ~= 0 then return false end
    end
    return true
end

-- ---------------------------------------------------------------------------
-- Addition
-- ---------------------------------------------------------------------------

local function bi_add(a, b)
    local max_n = math.max(a.n, b.n)
    local r = {n = max_n}
    local carry = 0

    for i = 1, max_n do
        local sum = bi_limb(a, i) + bi_limb(b, i) + carry
        r[i] = sum & LIMB_MASK
        carry = sum >> LIMB_BITS
    end

    if carry > 0 then
        r.n = max_n + 1
        r[r.n] = carry
    end

    return r
end

-- ---------------------------------------------------------------------------
-- Subtraction (assumes a >= b)
-- ---------------------------------------------------------------------------

local function bi_sub(a, b)
    local r = {n = a.n}
    local borrow = 0

    for i = 1, a.n do
        local diff = bi_limb(a, i) - bi_limb(b, i) - borrow
        if diff < 0 then
            diff = diff + (1 << LIMB_BITS)
            borrow = 1
        else
            borrow = 0
        end
        r[i] = diff
    end

    return bi_normalize(r)
end

-- ---------------------------------------------------------------------------
-- Multiplication
-- ---------------------------------------------------------------------------

local function bi_mul(a, b)
    local rn = a.n + b.n
    local r = {n = rn}
    for i = 1, rn do r[i] = 0 end

    for i = 1, a.n do
        local carry = 0
        local ai = a[i]
        for j = 1, b.n do
            local prod = ai * b[j] + r[i + j - 1] + carry
            r[i + j - 1] = prod & LIMB_MASK
            carry = prod >> LIMB_BITS
        end
        -- Propagate carry beyond b.n
        local k = i + b.n
        while carry > 0 and k <= rn do
            local sum = r[k] + carry
            r[k] = sum & LIMB_MASK
            carry = sum >> LIMB_BITS
            k = k + 1
        end
        if carry > 0 then
            rn = rn + 1
            r.n = rn
            r[rn] = carry
        end
    end

    return bi_normalize(r)
end

-- Multiply by a small integer (fits in one limb).
local function bi_mul_small(a, n)
    local r = {n = a.n}
    local carry = 0

    for i = 1, a.n do
        local prod = a[i] * n + carry
        r[i] = prod & LIMB_MASK
        carry = prod >> LIMB_BITS
    end

    if carry > 0 then
        r.n = a.n + 1
        r[r.n] = carry
    end

    return bi_normalize(r)
end

-- ---------------------------------------------------------------------------
-- Division and Modulo (generic, for mod L)
-- ---------------------------------------------------------------------------
-- We implement schoolbook long division for arbitrary big integers.
-- This is needed to reduce 512-bit SHA-512 outputs modulo the group order L.

-- Get bit i (0-indexed) of a big integer.
local function bi_get_bit(a, i)
    local limb_idx = (i // LIMB_BITS) + 1
    local bit_idx = i % LIMB_BITS
    if limb_idx > a.n then return 0 end
    return (a[limb_idx] >> bit_idx) & 1
end

-- Left shift by one bit.
local function bi_shl1(a)
    local r = {n = a.n}
    local carry = 0
    for i = 1, a.n do
        local val = (a[i] << 1) | carry
        r[i] = val & LIMB_MASK
        carry = val >> LIMB_BITS
    end
    if carry > 0 then
        r.n = r.n + 1
        r[r.n] = carry
    end
    return r
end

-- Number of significant bits.
local function bi_bit_length(a)
    if bi_is_zero(a) then return 0 end
    local top = a[a.n]
    local bits = (a.n - 1) * LIMB_BITS
    while top > 0 do
        bits = bits + 1
        top = top >> 1
    end
    return bits
end

-- Compute a mod m using binary long division.
-- Returns remainder only (we don't need the quotient).
local function bi_mod(a, m)
    if bi_cmp(a, m) < 0 then return a end

    local r = bi_from_int(0)
    local a_bits = bi_bit_length(a)

    for i = a_bits - 1, 0, -1 do
        r = bi_shl1(r)
        -- Set bit 0 to the current bit of a
        r[1] = r[1] | bi_get_bit(a, i)
        r = bi_normalize(r)

        if bi_cmp(r, m) >= 0 then
            r = bi_sub(r, m)
        end
    end

    return r
end

-- ============================================================================
-- PART 2: FIELD ARITHMETIC (mod p = 2^255 - 19)
-- ============================================================================
-- The prime field GF(p) where p = 2^255 - 19 is the same field used in
-- X25519. We use the special structure of p for fast reduction.

local P  -- p = 2^255 - 19
do
    local two_255 = {n = 9}
    for i = 1, 8 do two_255[i] = 0 end
    two_255[9] = 1 << 15  -- bit 255
    P = bi_sub(two_255, bi_from_int(19))
end

-- Fast modular reduction for p = 2^255 - 19.
-- Since 2^255 = 19 (mod p), we split at bit 255 and fold the high bits.
local function bi_mod_p(a)
    local r = {n = a.n}
    for i = 1, a.n do r[i] = a[i] end

    for _ = 1, 4 do
        if r.n <= 9 then
            local limb9 = bi_limb(r, 9)
            if limb9 < (1 << 15) then break end
        end

        local limb9 = bi_limb(r, 9)
        local has_hi = (limb9 >> 15) > 0 or r.n > 9

        if has_hi then
            local hi = {n = 0}
            local carry_bits = limb9 >> 15

            for i = 10, r.n do
                local combined = carry_bits | ((r[i] & ((1 << 15) - 1)) << 15)
                hi.n = hi.n + 1
                hi[hi.n] = combined & LIMB_MASK
                carry_bits = r[i] >> 15
            end

            if carry_bits > 0 or hi.n == 0 then
                hi.n = hi.n + 1
                hi[hi.n] = carry_bits
            end

            hi = bi_normalize(hi)

            r[9] = limb9 & ((1 << 15) - 1)
            for i = 10, r.n do r[i] = nil end
            r.n = 9
            r = bi_normalize(r)

            r = bi_add(r, bi_mul_small(hi, 19))
        else
            break
        end
    end

    while bi_cmp(r, P) >= 0 do
        r = bi_sub(r, P)
    end

    return bi_normalize(r)
end

-- Field operations: add, subtract, multiply, square, negate.
local function field_add(a, b)
    return bi_mod_p(bi_add(a, b))
end

local function field_sub(a, b)
    if bi_cmp(a, b) < 0 then
        return bi_mod_p(bi_sub(bi_add(a, P), b))
    else
        return bi_mod_p(bi_sub(a, b))
    end
end

local function field_mul(a, b)
    return bi_mod_p(bi_mul(a, b))
end

local function field_sq(a)
    return field_mul(a, a)
end

local function field_neg(a)
    if bi_is_zero(a) then return a end
    return bi_sub(P, a)
end

-- ---------------------------------------------------------------------------
-- Field Inversion: a^(p-2) mod p (Fermat's little theorem)
-- ---------------------------------------------------------------------------
-- We use an optimized addition chain for p-2 = 2^255 - 21.

local function field_invert(a)
    local z2 = field_mul(field_sq(a), a)

    local z4 = field_sq(z2)
    z4 = field_sq(z4)
    z4 = field_mul(z4, z2)

    local z5 = field_sq(z4)
    z5 = field_mul(z5, a)

    local z10 = z5
    for _ = 1, 5 do z10 = field_sq(z10) end
    z10 = field_mul(z10, z5)

    local z20 = z10
    for _ = 1, 10 do z20 = field_sq(z20) end
    z20 = field_mul(z20, z10)

    local z40 = z20
    for _ = 1, 20 do z40 = field_sq(z40) end
    z40 = field_mul(z40, z20)

    local z50 = z40
    for _ = 1, 10 do z50 = field_sq(z50) end
    z50 = field_mul(z50, z10)

    local z100 = z50
    for _ = 1, 50 do z100 = field_sq(z100) end
    z100 = field_mul(z100, z50)

    local z200 = z100
    for _ = 1, 100 do z200 = field_sq(z200) end
    z200 = field_mul(z200, z100)

    local z250 = z200
    for _ = 1, 50 do z250 = field_sq(z250) end
    z250 = field_mul(z250, z50)

    local result = z250
    for _ = 1, 5 do result = field_sq(result) end
    local a11 = field_sq(a)
    a11 = field_sq(a11)
    a11 = field_sq(a11)
    a11 = field_mul(a11, z2)

    result = field_mul(result, a11)
    return result
end

-- ---------------------------------------------------------------------------
-- Field Square Root
-- ---------------------------------------------------------------------------
-- For p = 5 (mod 8), the square root of a is:
--   candidate = a^((p+3)/8)
-- If candidate^2 == a, return candidate.
-- If candidate^2 == -a, return candidate * sqrt(-1).
-- Otherwise, a has no square root.

-- sqrt(-1) mod p -- a constant used in square root computation.
local SQRT_M1 = bi_from_byte_table({
    0xb0, 0xa0, 0x0e, 0x4a, 0x27, 0x1b, 0xee, 0xc4,
    0x78, 0xe4, 0x2f, 0xad, 0x06, 0x18, 0x43, 0x2f,
    0xa7, 0xd7, 0xfb, 0x3d, 0x99, 0x00, 0x4d, 0x2b,
    0x0b, 0xdf, 0xc1, 0x4f, 0x80, 0x24, 0x83, 0x2b
})

-- Compute (p+3)/8 as a big integer for the exponentiation.
-- p+3 = 2^255 - 16, (p+3)/8 = 2^252 - 2
-- We compute a^(2^252 - 2) using the addition chain for field_invert
-- but stopping earlier.

local function field_pow_p38(a)
    -- We need a^((p+3)/8) = a^(2^252 - 2)
    -- Use the same addition chain approach as field_invert
    local z2 = field_mul(field_sq(a), a)        -- a^3

    local z4 = field_sq(z2)
    z4 = field_sq(z4)
    z4 = field_mul(z4, z2)                       -- a^(2^4-1) = a^15

    local z5 = field_sq(z4)
    z5 = field_mul(z5, a)                        -- a^(2^5-1) = a^31

    local z10 = z5
    for _ = 1, 5 do z10 = field_sq(z10) end
    z10 = field_mul(z10, z5)                     -- a^(2^10-1)

    local z20 = z10
    for _ = 1, 10 do z20 = field_sq(z20) end
    z20 = field_mul(z20, z10)                    -- a^(2^20-1)

    local z40 = z20
    for _ = 1, 20 do z40 = field_sq(z40) end
    z40 = field_mul(z40, z20)                    -- a^(2^40-1)

    local z50 = z40
    for _ = 1, 10 do z50 = field_sq(z50) end
    z50 = field_mul(z50, z10)                    -- a^(2^50-1)

    local z100 = z50
    for _ = 1, 50 do z100 = field_sq(z100) end
    z100 = field_mul(z100, z50)                  -- a^(2^100-1)

    local z200 = z100
    for _ = 1, 100 do z200 = field_sq(z200) end
    z200 = field_mul(z200, z100)                 -- a^(2^200-1)

    local z250 = z200
    for _ = 1, 50 do z250 = field_sq(z250) end
    z250 = field_mul(z250, z50)                  -- a^(2^250-1)

    -- a^(2^252 - 4) = z250 squared twice
    local result = z250
    result = field_sq(result)                    -- a^(2^251 - ...)
    result = field_sq(result)                    -- a^(2^252 - 4)
    -- a^(2^252 - 2) = a^(2^252-4) * a^2
    result = field_mul(result, field_sq(a))

    return result
end

local function field_sqrt(a)
    local candidate = field_pow_p38(a)

    -- Check: candidate^2 == a?
    local check = field_sq(candidate)
    if bi_cmp(check, a) == 0 then
        return candidate
    end

    -- Check: candidate^2 == -a?
    if bi_cmp(check, field_neg(a)) == 0 then
        return field_mul(candidate, SQRT_M1)
    end

    -- No square root exists
    return nil
end

-- ============================================================================
-- PART 3: CURVE CONSTANTS
-- ============================================================================

-- d = -121665/121666 mod p
-- Pre-computed value from RFC 8032.
local D = bi_from_byte_table({
    0xa3, 0x78, 0x59, 0x13, 0xca, 0x4d, 0xeb, 0x75,
    0xab, 0xd8, 0x41, 0x41, 0x4d, 0x0a, 0x70, 0x00,
    0x98, 0xe8, 0x79, 0x77, 0x79, 0x40, 0xc7, 0x8c,
    0x73, 0xfe, 0x6f, 0x2b, 0xee, 0x6c, 0x03, 0x52
})

-- Group order L = 2^252 + 27742317777372353535851937790883648493
local L = bi_from_byte_table({
    0xed, 0xd3, 0xf5, 0x5c, 0x1a, 0x63, 0x12, 0x58,
    0xd6, 0x9c, 0xf7, 0xa2, 0xde, 0xf9, 0xde, 0x14,
    0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00,
    0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x10
})

-- Base point B coordinates (from RFC 8032)
local B_Y = bi_from_byte_table({
    0x58, 0x66, 0x66, 0x66, 0x66, 0x66, 0x66, 0x66,
    0x66, 0x66, 0x66, 0x66, 0x66, 0x66, 0x66, 0x66,
    0x66, 0x66, 0x66, 0x66, 0x66, 0x66, 0x66, 0x66,
    0x66, 0x66, 0x66, 0x66, 0x66, 0x66, 0x66, 0x66
})

local B_X = bi_from_byte_table({
    0x1a, 0xd5, 0x25, 0x8f, 0x60, 0x2d, 0x56, 0xc9,
    0xb2, 0xa7, 0x25, 0x95, 0x60, 0xc7, 0x2c, 0x69,
    0x5c, 0xdc, 0xd6, 0xfd, 0x31, 0xe2, 0xa4, 0xc0,
    0xfe, 0x53, 0x6e, 0xcd, 0xd3, 0x36, 0x69, 0x21
})

-- ============================================================================
-- PART 4: POINT OPERATIONS (Extended Coordinates)
-- ============================================================================
-- A point is a table {X, Y, Z, T} where x = X/Z, y = Y/Z, T = X*Y/Z.

-- The identity point: affine (0, 1), extended (0, 1, 1, 0).
local function point_identity()
    return {
        X = bi_from_int(0),
        Y = bi_from_int(1),
        Z = bi_from_int(1),
        T = bi_from_int(0),
    }
end

-- ---------------------------------------------------------------------------
-- Point Addition (unified formula for twisted Edwards a=-1)
-- ---------------------------------------------------------------------------
-- From the Hisil-Wong-Carter-Dawson paper, this formula works for ALL
-- point pairs (including P+P, P+O, P+(-P)) without branching.
--
-- Input: P1 = (X1,Y1,Z1,T1), P2 = (X2,Y2,Z2,T2)
-- Output: P3 = (X3,Y3,Z3,T3)
--
-- A = X1*X2,  B = Y1*Y2,  C = T1*d*T2,  D = Z1*Z2
-- E = (X1+Y1)*(X2+Y2) - A - B
-- F = D - C,  G = D + C,  H = B + A    (note: H = B + A because a = -1)
-- X3 = E*F,  Y3 = G*H,  T3 = E*H,  Z3 = F*G

local function point_add(p1, p2)
    local A = field_mul(p1.X, p2.X)
    local B = field_mul(p1.Y, p2.Y)
    local C = field_mul(field_mul(p1.T, D), p2.T)
    local DD = field_mul(p1.Z, p2.Z)

    local E = field_sub(
        field_mul(field_add(p1.X, p1.Y), field_add(p2.X, p2.Y)),
        field_add(A, B)
    )
    local F = field_sub(DD, C)
    local G = field_add(DD, C)
    local H = field_add(B, A)

    return {
        X = field_mul(E, F),
        Y = field_mul(G, H),
        T = field_mul(E, H),
        Z = field_mul(F, G),
    }
end

-- ---------------------------------------------------------------------------
-- Point Doubling
-- ---------------------------------------------------------------------------
-- Doubling has a cheaper formula than generic addition:
--
-- A = X1^2,  B = Y1^2,  C = 2*Z1^2,  D = -A   (because a = -1)
-- E = (X1+Y1)^2 - A - B,  G = D + B,  F = G - C,  H = D - B
-- X3 = E*F,  Y3 = G*H,  T3 = E*H,  Z3 = F*G

local function point_double(p)
    local A = field_sq(p.X)
    local B = field_sq(p.Y)
    local C = field_mul(bi_from_int(2), field_sq(p.Z))
    local DD = field_neg(A)

    local E = field_sub(field_sq(field_add(p.X, p.Y)), field_add(A, B))
    local G = field_add(DD, B)
    local F = field_sub(G, C)
    local H = field_sub(DD, B)

    return {
        X = field_mul(E, F),
        Y = field_mul(G, H),
        T = field_mul(E, H),
        Z = field_mul(F, G),
    }
end

-- ---------------------------------------------------------------------------
-- Scalar Multiplication: double-and-add, high-to-low bit scanning
-- ---------------------------------------------------------------------------

local function scalar_mult(scalar_bi, point)
    local result = point_identity()
    local bits = bi_bit_length(scalar_bi)

    for i = bits - 1, 0, -1 do
        result = point_double(result)
        if bi_get_bit(scalar_bi, i) == 1 then
            result = point_add(result, point)
        end
    end

    return result
end

-- ============================================================================
-- PART 5: POINT ENCODING/DECODING (RFC 8032 Section 5.1.2)
-- ============================================================================

-- Encode a point as 32 bytes: y in little-endian, with the sign of x
-- stored in the high bit of byte 31.
local function point_encode(pt)
    -- Convert to affine: x = X/Z, y = Y/Z
    local z_inv = field_invert(pt.Z)
    local x = field_mul(pt.X, z_inv)
    local y = field_mul(pt.Y, z_inv)

    -- Encode y as 32 bytes LE
    local y_bytes = bi_to_byte_table(y, 32)

    -- Set the high bit of byte 31 (0-indexed) to the low bit of x.
    -- In our 1-indexed table, that's byte 32.
    y_bytes[32] = y_bytes[32] | ((bi_limb(x, 1) & 1) << 7)

    -- Convert to string
    local chars = {}
    for i = 1, 32 do chars[i] = string.char(y_bytes[i]) end
    return table.concat(chars)
end

-- Decode a 32-byte encoded point.
-- Returns the point in extended coordinates, or nil on failure.
local function point_decode(encoded)
    if #encoded ~= 32 then return nil end

    -- Extract bytes
    local bytes = {}
    for i = 1, 32 do bytes[i] = string.byte(encoded, i) end

    -- The sign bit of x is the high bit of byte 32 (1-indexed)
    local x_sign = (bytes[32] >> 7) & 1

    -- Clear the sign bit to get y
    bytes[32] = bytes[32] & 0x7F
    local y = bi_from_byte_table(bytes)

    -- Check y < p
    if bi_cmp(y, P) >= 0 then return nil end

    -- Compute x^2 = (y^2 - 1) * inv(d*y^2 + 1)
    local y2 = field_sq(y)
    local num = field_sub(y2, bi_from_int(1))
    local den = field_add(field_mul(D, y2), bi_from_int(1))
    local den_inv = field_invert(den)
    local x2 = field_mul(num, den_inv)

    -- If x^2 = 0 and the sign bit is 1, invalid
    if bi_is_zero(x2) then
        if x_sign == 1 then return nil end
        return {
            X = bi_from_int(0),
            Y = y,
            Z = bi_from_int(1),
            T = bi_from_int(0),
        }
    end

    -- Compute x = sqrt(x^2)
    local x = field_sqrt(x2)
    if x == nil then return nil end

    -- Ensure the sign matches
    if (bi_limb(x, 1) & 1) ~= x_sign then
        x = field_neg(x)
    end

    return {
        X = x,
        Y = y,
        Z = bi_from_int(1),
        T = field_mul(x, y),
    }
end

-- ============================================================================
-- PART 6: SHA-512 HELPER
-- ============================================================================
-- Our SHA-512 module returns a table of bytes. We need to convert that to
-- a binary string for concatenation, and to a big integer for arithmetic.

local function bytes_to_string(byte_table)
    local chars = {}
    for i = 1, #byte_table do
        chars[i] = string.char(byte_table[i])
    end
    return table.concat(chars)
end

local function sha512_bytes(data)
    return sha512.digest(data)
end

local function sha512_to_bigint(data)
    -- SHA-512 returns 64 bytes, big-endian conceptually, but we interpret
    -- them as a little-endian 512-bit integer for Ed25519 (per RFC 8032).
    local hash = sha512.digest(data)
    return bi_from_byte_table(hash)
end

-- ============================================================================
-- PART 7: PUBLIC API
-- ============================================================================

-- Build the base point B in extended coordinates.
local B = {
    X = B_X,
    Y = B_Y,
    Z = bi_from_int(1),
    T = field_mul(B_X, B_Y),
}

-- ---------------------------------------------------------------------------
-- generate_keypair(seed) -> public_key, secret_key
-- ---------------------------------------------------------------------------
-- Takes a 32-byte seed. Returns:
--   public_key: 32-byte encoded point (the public key A)
--   secret_key: 64-byte string (seed || public_key), for use in sign()
--
-- The secret scalar is derived by hashing the seed with SHA-512 and
-- "clamping" the first 32 bytes:
--   - Clear bits 0, 1, 2 (make divisible by cofactor 8)
--   - Clear bit 255
--   - Set bit 254

function M.generate_keypair(seed)
    assert(#seed == 32, "seed must be 32 bytes")

    local h = sha512_bytes(seed)

    -- Clamp the first 32 bytes to get the secret scalar
    h[1] = h[1] & 248       -- Clear bits 0,1,2
    h[32] = h[32] & 127     -- Clear bit 255
    h[32] = h[32] | 64      -- Set bit 254

    local scalar_bytes = {}
    for i = 1, 32 do scalar_bytes[i] = h[i] end
    local a = bi_from_byte_table(scalar_bytes)

    -- A = a * B (the public key point)
    local A = scalar_mult(a, B)
    local public_key = point_encode(A)

    -- Secret key = seed || public_key
    local secret_key = seed .. public_key

    return public_key, secret_key
end

-- ---------------------------------------------------------------------------
-- sign(message, secret_key) -> signature
-- ---------------------------------------------------------------------------
-- Creates a 64-byte deterministic signature.
--
-- Steps:
--   1. Hash the seed (first 32 bytes of secret_key) to get scalar a and
--      prefix (last 32 bytes of the hash).
--   2. r = SHA-512(prefix || message) mod L  -- deterministic nonce
--   3. R = r * B
--   4. S = (r + SHA-512(R || A || message) * a) mod L
--   5. Return encode(R) || encode(S)

function M.sign(message, secret_key)
    assert(#secret_key == 64, "secret_key must be 64 bytes")

    local seed = secret_key:sub(1, 32)
    local public_key = secret_key:sub(33, 64)

    -- Re-derive the scalar and prefix from the seed
    local h = sha512_bytes(seed)
    h[1] = h[1] & 248
    h[32] = h[32] & 127
    h[32] = h[32] | 64

    local scalar_bytes = {}
    for i = 1, 32 do scalar_bytes[i] = h[i] end
    local a = bi_from_byte_table(scalar_bytes)

    -- prefix = last 32 bytes of SHA-512(seed)
    local prefix = {}
    for i = 33, 64 do prefix[#prefix + 1] = h[i] end
    local prefix_str = bytes_to_string(prefix)

    -- r = SHA-512(prefix || message) mod L
    local r_hash = sha512_to_bigint(prefix_str .. message)
    local r = bi_mod(r_hash, L)

    -- R = r * B
    local R_point = scalar_mult(r, B)
    local R_enc = point_encode(R_point)

    -- k = SHA-512(R || A || message) mod L
    local k_hash = sha512_to_bigint(R_enc .. public_key .. message)
    local k = bi_mod(k_hash, L)

    -- S = (r + k * a) mod L
    local S = bi_mod(bi_add(r, bi_mul(k, a)), L)

    -- Encode S as 32 bytes LE
    local S_enc = bi_to_bytes(S, 32)

    return R_enc .. S_enc
end

-- ---------------------------------------------------------------------------
-- verify(message, signature, public_key) -> boolean
-- ---------------------------------------------------------------------------
-- Verifies a 64-byte signature against a message and public key.
--
-- Steps:
--   1. Decode R (first 32 bytes) and A (the public key) as curve points.
--   2. Decode S (last 32 bytes) as a scalar. Check S < L.
--   3. k = SHA-512(R || A || message) mod L
--   4. Check: S * B == R + k * A

function M.verify(message, signature, public_key)
    if #signature ~= 64 or #public_key ~= 32 then
        return false
    end

    local R_enc = signature:sub(1, 32)
    local S_enc = signature:sub(33, 64)

    -- Decode R
    local R = point_decode(R_enc)
    if R == nil then return false end

    -- Decode A (public key)
    local A = point_decode(public_key)
    if A == nil then return false end

    -- Decode S as a scalar
    local S_bytes = {}
    for i = 1, 32 do S_bytes[i] = string.byte(S_enc, i) end
    local S = bi_from_byte_table(S_bytes)

    -- Check S < L (malleability check)
    if bi_cmp(S, L) >= 0 then return false end

    -- k = SHA-512(R || A || message) mod L
    local k_hash = sha512_to_bigint(R_enc .. public_key .. message)
    local k = bi_mod(k_hash, L)

    -- Verify: S * B == R + k * A
    local lhs = scalar_mult(S, B)
    local rhs = point_add(R, scalar_mult(k, A))

    -- Compare by encoding both to 32 bytes
    local lhs_enc = point_encode(lhs)
    local rhs_enc = point_encode(rhs)

    return lhs_enc == rhs_enc
end

-- ---------------------------------------------------------------------------
-- Hex Utilities
-- ---------------------------------------------------------------------------

function M.from_hex(hex)
    return (hex:gsub('..', function(cc)
        return string.char(tonumber(cc, 16))
    end))
end

function M.to_hex(s)
    return (s:gsub('.', function(c)
        return string.format('%02x', string.byte(c))
    end))
end

return M
