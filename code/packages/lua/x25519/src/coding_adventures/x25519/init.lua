-- ============================================================================
-- X25519: Elliptic Curve Diffie-Hellman on Curve25519 (RFC 7748)
-- ============================================================================
--
-- This module implements X25519 (RFC 7748), which performs Diffie-Hellman key
-- exchange on Curve25519. X25519 is widely used in TLS 1.3, Signal Protocol,
-- WireGuard, and many other modern cryptographic systems.
--
-- Since Lua 5.4+ only has 64-bit integers (no arbitrary-precision), we
-- implement big integer arithmetic using arrays of 30-bit limbs.
-- Each limb stores a value in [0, 2^30 - 1], and the full number is:
--   n = limbs[1] + limbs[2] * 2^30 + limbs[3] * 2^60 + ...
--
-- We use 30-bit limbs because the product of two 30-bit limbs is at most
-- 2^60, which fits in a 64-bit integer with room for accumulation.
-- ============================================================================

local M = {}

-- ---------------------------------------------------------------------------
-- Big Integer Representation
-- ---------------------------------------------------------------------------
-- A big integer is a table of limbs (Lua 1-indexed), stored little-endian.
-- limbs[1] is the least significant. The table also has a field `n` giving
-- the number of limbs. Limbs beyond `n` are implicitly zero.

local LIMB_BITS = 30
local LIMB_MASK = (1 << LIMB_BITS) - 1  -- 0x3FFFFFFF

-- Number of limbs needed for 256-bit numbers: ceil(256/30) = 9
-- But we may need extra during multiplication.
local NUM_LIMBS = 9

-- ---------------------------------------------------------------------------
-- Big Integer Constructors
-- ---------------------------------------------------------------------------

-- Create a big integer from a small (fits-in-one-limb) non-negative integer.
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

-- Get limb i (1-indexed), returning 0 for indices beyond the stored limbs.
local function bi_limb(a, i)
    if i <= a.n then return a[i] else return 0 end
end

-- Normalize: remove leading zero limbs.
local function bi_normalize(a)
    while a.n > 1 and a[a.n] == 0 do
        a[a.n] = nil
        a.n = a.n - 1
    end
    return a
end

-- ---------------------------------------------------------------------------
-- Big Integer from/to Bytes (Little-Endian)
-- ---------------------------------------------------------------------------

-- Decode a byte string (little-endian) into a big integer.
local function bi_from_bytes(bytes)
    -- Pack bytes into 30-bit limbs
    local r = {n = 0}
    local bit_acc = 0      -- accumulated bits
    local bit_count = 0    -- number of accumulated bits

    for i = 1, #bytes do
        local byte = string.byte(bytes, i)
        bit_acc = bit_acc | (byte << bit_count)
        bit_count = bit_count + 8

        while bit_count >= LIMB_BITS do
            r.n = r.n + 1
            r[r.n] = bit_acc & LIMB_MASK
            bit_acc = bit_acc >> LIMB_BITS
            bit_count = bit_count - LIMB_BITS
        end
    end

    -- Flush remaining bits
    if bit_count > 0 or r.n == 0 then
        r.n = r.n + 1
        r[r.n] = bit_acc & LIMB_MASK
    end

    return bi_normalize(r)
end

-- Encode a big integer as a byte string of the given length (little-endian).
local function bi_to_bytes(a, len)
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
        bytes[#bytes + 1] = string.char(bit_acc & 0xFF)
        bit_acc = bit_acc >> 8
        bit_count = bit_count - 8
    end

    return table.concat(bytes)
end

-- ---------------------------------------------------------------------------
-- Big Integer Comparison
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

-- ---------------------------------------------------------------------------
-- Big Integer Addition
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
-- Big Integer Subtraction (assumes a >= b)
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
-- Big Integer Multiplication
-- ---------------------------------------------------------------------------
-- Schoolbook O(n^2) multiplication. For our 9-limb numbers, this is 81
-- multiplications — perfectly fast enough.

local function bi_mul(a, b)
    local r = {n = a.n + b.n}
    for i = 1, r.n do r[i] = 0 end

    for i = 1, a.n do
        local carry = 0
        local ai = a[i]
        for j = 1, b.n do
            local prod = ai * b[j] + r[i + j - 1] + carry
            r[i + j - 1] = prod & LIMB_MASK
            carry = prod >> LIMB_BITS
        end
        if carry > 0 then
            r[i + b.n] = r[i + b.n] + carry
        end
    end

    return bi_normalize(r)
end

-- Multiply a big integer by a small integer (fits in one limb).
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
-- Big Integer Division and Modulo
-- ---------------------------------------------------------------------------
-- We implement modular reduction using Barrett-like reduction or simply
-- by computing a mod p = a - (a // p) * p.
--
-- For our specific use case (mod p = 2^255 - 19), we can use a much faster
-- approach: since p = 2^255 - 19, we have 2^255 ≡ 19 (mod p).
-- So for any number n = hi * 2^255 + lo (where lo < 2^255),
-- n ≡ hi * 19 + lo (mod p).
-- We can repeat this until the result is < p.

-- The prime p = 2^255 - 19 as a big integer.
local P
do
    -- Build 2^255 as a big integer: bit 255 is set
    -- 255 / 30 = 8 remainder 15, so bit 255 is in limb 9 (1-indexed), bit 15
    local two_255 = {n = 9}
    for i = 1, 8 do two_255[i] = 0 end
    two_255[9] = 1 << 15  -- bit 15 of limb 9 = bit 255 overall

    local nineteen = bi_from_int(19)
    P = bi_sub(two_255, nineteen)
end

-- Fast modular reduction for p = 2^255 - 19.
-- Split the number at bit 255: lo = lower 255 bits, hi = upper bits.
-- Then n ≡ lo + 19*hi (mod p). Repeat until result < 2^256 or so,
-- then do a final comparison with p.
--
-- The split happens within limb 9 at bit position 15 (since 8*30=240,
-- and 255-240=15). We must carefully extract hi by right-shifting the
-- upper portion by 15 bits across all limbs to maintain proper alignment.
local function bi_mod_p(a)
    -- Copy to avoid mutating
    local r = {n = a.n}
    for i = 1, a.n do r[i] = a[i] end

    -- Bit 255 is at limb 9 (1-indexed), bit 15 within that limb.
    -- (8 limbs * 30 bits = 240 bits, then bit 15 in limb 9 = bit 255)
    -- We split: lo = bits [0, 254], hi = bits [255, ...]

    -- Repeat reduction until the number fits in ~256 bits
    for _ = 1, 4 do
        if r.n <= 9 then
            local limb9 = bi_limb(r, 9)
            if limb9 < (1 << 15) then
                break  -- Already < 2^255, no reduction needed
            end
        end

        -- Extract hi = bits [255, ...] as a proper big integer.
        -- Since bit 255 falls at bit 15 of limb 9, we need to right-shift
        -- the upper portion by 15 bits. This means:
        --   hi_bit0..14 come from limb9 bits 15..29
        --   hi_bit15..44 come from limb10 bits 0..29
        --   etc.
        -- We combine adjacent limbs with a 15-bit shift.
        local limb9 = bi_limb(r, 9)
        local has_hi = (limb9 >> 15) > 0 or r.n > 9

        if has_hi then
            -- Build hi by extracting bits [255..] and packing into 30-bit limbs.
            -- The split is at bit 15 within limb 9, so we need to right-shift
            -- the upper portion by 15 bits. We do this at the limb level:
            -- each output hi limb combines the upper 15 bits of one source limb
            -- with the lower 15 bits of the next source limb.
            local hi = {n = 0}
            -- Start with the upper 15 bits of limb 9
            local carry_bits = limb9 >> 15  -- 15 bits

            for i = 10, r.n do
                -- Combine carry_bits (15 bits) with lower 15 bits of r[i]
                local combined = carry_bits | ((r[i] & ((1 << 15) - 1)) << 15)
                hi.n = hi.n + 1
                hi[hi.n] = combined & LIMB_MASK
                carry_bits = r[i] >> 15
            end

            -- Flush remaining carry bits
            if carry_bits > 0 or hi.n == 0 then
                hi.n = hi.n + 1
                hi[hi.n] = carry_bits
            end

            hi = bi_normalize(hi)

            -- Clear the high bits from r (keep only lo = bits 0-254)
            r[9] = limb9 & ((1 << 15) - 1)
            for i = 10, r.n do r[i] = nil end
            r.n = 9
            r = bi_normalize(r)

            -- r = lo + 19 * hi
            local hi_times_19 = bi_mul_small(hi, 19)
            r = bi_add(r, hi_times_19)
        else
            break
        end
    end

    -- Final subtraction: if r >= p, subtract p
    while bi_cmp(r, P) >= 0 do
        r = bi_sub(r, P)
    end

    return bi_normalize(r)
end

-- ---------------------------------------------------------------------------
-- Field Arithmetic over GF(2^255 - 19)
-- ---------------------------------------------------------------------------

local function field_add(a, b)
    return bi_mod_p(bi_add(a, b))
end

local function field_sub(a, b)
    -- Ensure non-negative by adding p if a < b
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

local function field_mul_small(a, n)
    return bi_mod_p(bi_mul_small(a, n))
end

-- ---------------------------------------------------------------------------
-- Field Inversion via Fermat's Little Theorem
-- ---------------------------------------------------------------------------
-- a^(p-2) mod p, using an optimized addition chain.
-- p - 2 = 2^255 - 21

local function field_invert(a)
    -- z2 = a^3 = a^(2^2 - 1)
    local z2 = field_mul(field_sq(a), a)

    -- z4 = a^(2^4 - 1)
    local z4 = field_sq(z2)
    z4 = field_sq(z4)
    z4 = field_mul(z4, z2)

    -- z5 = a^(2^5 - 1)
    local z5 = field_sq(z4)
    z5 = field_mul(z5, a)

    -- z10 = a^(2^10 - 1)
    local z10 = z5
    for _ = 1, 5 do z10 = field_sq(z10) end
    z10 = field_mul(z10, z5)

    -- z20 = a^(2^20 - 1)
    local z20 = z10
    for _ = 1, 10 do z20 = field_sq(z20) end
    z20 = field_mul(z20, z10)

    -- z40 = a^(2^40 - 1)
    local z40 = z20
    for _ = 1, 20 do z40 = field_sq(z40) end
    z40 = field_mul(z40, z20)

    -- z50 = a^(2^50 - 1)
    local z50 = z40
    for _ = 1, 10 do z50 = field_sq(z50) end
    z50 = field_mul(z50, z10)

    -- z100 = a^(2^100 - 1)
    local z100 = z50
    for _ = 1, 50 do z100 = field_sq(z100) end
    z100 = field_mul(z100, z50)

    -- z200 = a^(2^200 - 1)
    local z200 = z100
    for _ = 1, 100 do z200 = field_sq(z200) end
    z200 = field_mul(z200, z100)

    -- z250 = a^(2^250 - 1)
    local z250 = z200
    for _ = 1, 50 do z250 = field_sq(z250) end
    z250 = field_mul(z250, z50)

    -- result = a^(2^255 - 21)
    -- z250 is a^(2^250-1), square 5 times to get a^(2^255-32)
    local result = z250
    for _ = 1, 5 do result = field_sq(result) end
    -- Multiply by a^11 = a^8 * a^3 to get a^(2^255-21)
    local a11 = field_sq(a)       -- a^2
    a11 = field_sq(a11)           -- a^4
    a11 = field_sq(a11)           -- a^8
    a11 = field_mul(a11, z2)      -- a^8 * a^3 = a^11

    result = field_mul(result, a11)
    return result
end

-- ---------------------------------------------------------------------------
-- Conditional Swap
-- ---------------------------------------------------------------------------

local function cswap(swap, a, b)
    if swap == 1 then
        return b, a
    else
        return a, b
    end
end

-- ---------------------------------------------------------------------------
-- Scalar Clamping
-- ---------------------------------------------------------------------------
-- The private key scalar is "clamped" before use:
--   byte[0]  &= 248  — Clear bits 0,1,2 (make divisible by cofactor 8)
--   byte[31] &= 127  — Clear bit 255
--   byte[31] |= 64   — Set bit 254 (constant-time ladder)

local function clamp_scalar(scalar_bytes)
    local bytes = {}
    for i = 1, 32 do
        bytes[i] = string.byte(scalar_bytes, i)
    end
    bytes[1] = bytes[1] & 248
    bytes[32] = bytes[32] & 127
    bytes[32] = bytes[32] | 64
    return bytes
end

-- Get bit i (0-indexed) of a scalar byte array (1-indexed).
local function scalar_bit(k_bytes, i)
    local byte_idx = (i // 8) + 1
    local bit_idx = i % 8
    return (k_bytes[byte_idx] >> bit_idx) & 1
end

-- ---------------------------------------------------------------------------
-- Montgomery Ladder
-- ---------------------------------------------------------------------------
-- The Montgomery ladder computes scalar multiplication on Curve25519 using
-- only the u-coordinate (x-coordinate in Montgomery form). At each step,
-- it maintains two points (x_2, z_2) and (x_3, z_3) whose difference is
-- always the base point. This allows doubling and addition using only
-- x-coordinates.
--
-- Points are in projective coordinates: the affine x is X/Z.
-- Only one expensive inversion is needed at the end.

local function montgomery_ladder(k_bytes, u)
    local x_1 = u
    local x_2 = bi_from_int(1)
    local z_2 = bi_from_int(0)
    local x_3 = {n = u.n}
    for i = 1, u.n do x_3[i] = u[i] end
    local z_3 = bi_from_int(1)
    local swap = 0

    -- Process bits 254 down to 0
    for t = 254, 0, -1 do
        local k_t = scalar_bit(k_bytes, t)
        swap = swap ~ k_t  -- XOR

        x_2, x_3 = cswap(swap, x_2, x_3)
        z_2, z_3 = cswap(swap, z_2, z_3)
        swap = k_t

        -- Montgomery ladder step: combined doubling and differential addition
        local A  = field_add(x_2, z_2)
        local AA = field_sq(A)
        local B  = field_sub(x_2, z_2)
        local BB = field_sq(B)
        local E  = field_sub(AA, BB)
        local C  = field_add(x_3, z_3)
        local D  = field_sub(x_3, z_3)
        local DA = field_mul(D, A)
        local CB = field_mul(C, B)

        -- Differential addition
        x_3 = field_sq(field_add(DA, CB))
        z_3 = field_mul(x_1, field_sq(field_sub(DA, CB)))

        -- Doubling
        x_2 = field_mul(AA, BB)
        -- z_2 = E * (AA + a24 * E) where a24 = 121665
        z_2 = field_mul(E, field_add(AA, field_mul_small(E, 121665)))
    end

    -- Final conditional swap
    x_2, x_3 = cswap(swap, x_2, x_3)
    z_2, z_3 = cswap(swap, z_2, z_3)

    -- Convert from projective to affine: result = x_2 * z_2^(p-2) mod p
    return field_mul(x_2, field_invert(z_2))
end

-- ---------------------------------------------------------------------------
-- Public API
-- ---------------------------------------------------------------------------

--- Perform X25519 scalar multiplication.
-- @param scalar 32-byte string (private key or scalar)
-- @param u_point 32-byte string (u-coordinate of the point)
-- @return 32-byte string (result u-coordinate)
function M.x25519(scalar, u_point)
    assert(#scalar == 32, "scalar must be 32 bytes")
    assert(#u_point == 32, "u_point must be 32 bytes")

    -- Clamp the scalar
    local k = clamp_scalar(scalar)

    -- Decode u-coordinate: mask bit 255 per RFC 7748
    local u_bytes = {}
    for i = 1, 32 do u_bytes[i] = string.byte(u_point, i) end
    u_bytes[32] = u_bytes[32] & 127
    local u_str = ""
    for i = 1, 32 do u_str = u_str .. string.char(u_bytes[i]) end
    local u = bi_from_bytes(u_str)

    -- Run Montgomery ladder
    local result = montgomery_ladder(k, u)

    -- Encode as 32 bytes
    local output = bi_to_bytes(result, 32)

    -- Reject all-zeros output
    local all_zero = true
    for i = 1, 32 do
        if string.byte(output, i) ~= 0 then
            all_zero = false
            break
        end
    end
    if all_zero then
        error("X25519 produced all-zeros output (low-order input point)")
    end

    return output
end

--- Compute X25519 public key from private key (base point u=9).
-- @param scalar 32-byte string
-- @return 32-byte string
function M.x25519_base(scalar)
    local base = string.char(9) .. string.rep(string.char(0), 31)
    return M.x25519(scalar, base)
end

--- Generate a keypair from a private key.
-- @param private_key 32-byte string
-- @return private_key, public_key (both 32-byte strings)
function M.generate_keypair(private_key)
    return private_key, M.x25519_base(private_key)
end

-- ---------------------------------------------------------------------------
-- Hex Utilities
-- ---------------------------------------------------------------------------

--- Decode a hex string to a binary string.
function M.from_hex(hex)
    return (hex:gsub('..', function(cc)
        return string.char(tonumber(cc, 16))
    end))
end

--- Encode a binary string as hex.
function M.to_hex(s)
    return (s:gsub('.', function(c)
        return string.format('%02x', string.byte(c))
    end))
end

return M
