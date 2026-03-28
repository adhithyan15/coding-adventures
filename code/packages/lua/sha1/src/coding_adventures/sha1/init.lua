-- sha1 — Pure Lua SHA-1 cryptographic hash function
--
-- SHA-1 (Secure Hash Algorithm 1) was designed by the NSA and published by
-- NIST in 1995. It produces a 160-bit (20-byte) hash, typically shown as a
-- 40-character hex string. Like MD5, SHA-1 is no longer considered secure
-- for cryptographic signing (collision attacks exist), but remains ubiquitous
-- in legacy systems, Git object IDs, and non-security checksums.
--
-- HOW SHA-1 WORKS — A GUIDED TOUR
-- ─────────────────────────────────
-- 1. PADDING (identical in structure to MD5, but big-endian)
--    Append 0x80, then zero bytes until length ≡ 56 (mod 64).
--    Append original bit-length as a 64-bit **big-endian** integer.
--    (SHA is big-endian throughout; MD5 is little-endian. Key difference!)
--
-- 2. INITIALIZATION (five 32-bit words, fixed constants from FIPS 180-4)
--    H0 = 0x67452301  H1 = 0xEFCDAB89  H2 = 0x98BADCFE
--    H3 = 0x10325476  H4 = 0xC3D2E1F0
--
-- 3. MESSAGE SCHEDULE
--    Each 512-bit block is expanded from 16 words (W[0..15]) to 80 words:
--      W[t] = ROTL1(W[t-3] XOR W[t-8] XOR W[t-14] XOR W[t-16])  for t ≥ 16
--    ROTL1 means left-rotate by 1 bit. This "stretches" the message and
--    ensures each round mixes in bits from earlier and later positions.
--
-- 4. COMPRESSION (80 rounds per block, 4 groups of 20)
--    Variables: a, b, c, d, e  (copies of H0..H4 at block start)
--    Each round:
--      TEMP = ROTL5(a) + F(t,b,c,d) + e + W[t] + K(t)
--      e = d, d = c, c = ROTL30(b), b = a, a = TEMP
--
--    Round functions F and constants K:
--      t  0-19: F = Ch(b,c,d)  = (b AND c) OR (NOT b AND d)  K = 0x5A827999
--      t 20-39: F = Parity     = b XOR c XOR d               K = 0x6ED9EBA1
--      t 40-59: F = Maj(b,c,d) = (b AND c) OR (b AND d) OR (c AND d) K = 0x8F1BBCDC
--      t 60-79: F = Parity     = b XOR c XOR d               K = 0xCA62C1D6
--
--    Note: 0x5A827999 = floor(sqrt(2) × 2^30)
--          0x6ED9EBA1 = floor(sqrt(3) × 2^30)
--          0x8F1BBCDC = floor(sqrt(5) × 2^30)
--          0xCA62C1D6 = floor(sqrt(10) × 2^30)
--
-- 5. OUTPUT
--    After all blocks, serialize H0..H4 each as 4 bytes **big-endian** to
--    produce the 20-byte digest.
--
-- LUA 5.4 BIT-ARITHMETIC NOTES
-- ─────────────────────────────
-- Same as MD5: use `& 0xFFFFFFFF` to keep arithmetic in 32-bit range.
-- SHA-1 left-rotate: ((x << n) | (x >> (32 - n))) & 0xFFFFFFFF
--
-- Usage:
--   local sha1 = require("coding_adventures.sha1")
--   local hex = sha1.hex("hello")     -- "aaf4c61ddcc5e8a2dabede0f3b482cd9aea9434d"
--   local raw = sha1.digest("hello")  -- {0xaa, 0xf4, ...}  (20 integers)
--
-- ============================================================================

local M = {}

M.VERSION = "0.1.0"

-- ---------------------------------------------------------------------------
-- Round constants (derived from square roots of small primes × 2^30)
-- ---------------------------------------------------------------------------
local K1 = 0x5A827999  -- rounds  0-19
local K2 = 0x6ED9EBA1  -- rounds 20-39
local K3 = 0x8F1BBCDC  -- rounds 40-59
local K4 = 0xCA62C1D6  -- rounds 60-79

-- ---------------------------------------------------------------------------
-- Initial hash values (FIPS 180-4, Section 5.3.1)
-- ---------------------------------------------------------------------------
local H0_INIT = 0x67452301
local H1_INIT = 0xEFCDAB89
local H2_INIT = 0x98BADCFE
local H3_INIT = 0x10325476
local H4_INIT = 0xC3D2E1F0

-- ---------------------------------------------------------------------------
-- u32(x) — clamp x to unsigned 32-bit integer (mask to 32 bits)
-- ---------------------------------------------------------------------------
local function u32(x)
    return x & 0xFFFFFFFF
end

-- ---------------------------------------------------------------------------
-- rol32(x, n) — left rotate x by n bits within 32 bits
-- ---------------------------------------------------------------------------
local function rol32(x, n)
    x = x & 0xFFFFFFFF
    return ((x << n) | (x >> (32 - n))) & 0xFFFFFFFF
end

-- ---------------------------------------------------------------------------
-- bytes_to_word_be(b0, b1, b2, b3) — pack 4 bytes as big-endian 32-bit word
-- SHA-1 is big-endian: the first byte is the most significant.
-- ---------------------------------------------------------------------------
local function bytes_to_word_be(b0, b1, b2, b3)
    return ((b0 << 24) | (b1 << 16) | (b2 << 8) | b3) & 0xFFFFFFFF
end

-- ---------------------------------------------------------------------------
-- word_to_bytes_be(w) — split a 32-bit word into 4 big-endian bytes
-- ---------------------------------------------------------------------------
local function word_to_bytes_be(w)
    w = u32(w)
    return {
        (w >> 24) & 0xFF,
        (w >> 16) & 0xFF,
        (w >>  8) & 0xFF,
         w        & 0xFF,
    }
end

-- ---------------------------------------------------------------------------
-- pad_message(msg) → byte array padded to a multiple of 64 bytes
--
-- SHA-1 padding (FIPS 180-4, Section 5.1.1):
--   1. Append byte 0x80
--   2. Append 0x00 bytes until length ≡ 56 (mod 64)
--   3. Append original bit-length as 64-bit big-endian integer
-- ---------------------------------------------------------------------------
local function pad_message(msg)
    local bytes = {}
    for i = 1, #msg do
        bytes[i] = string.byte(msg, i)
    end

    local orig_len = #msg   -- bytes
    local bit_len  = orig_len * 8  -- bits (fits in Lua 64-bit integer)

    -- Step 1: append 0x80
    bytes[#bytes + 1] = 0x80

    -- Step 2: append 0x00 until length ≡ 56 (mod 64)
    while #bytes % 64 ~= 56 do
        bytes[#bytes + 1] = 0x00
    end

    -- Step 3: append 64-bit big-endian length
    -- Most significant byte first (big-endian)
    for shift = 56, 0, -8 do
        bytes[#bytes + 1] = (bit_len >> shift) & 0xFF
    end

    return bytes
end

-- ---------------------------------------------------------------------------
-- process_block(block_bytes, h0, h1, h2, h3, h4)
--   → new h0, h1, h2, h3, h4
--
-- The SHA-1 compression function. Reads 64 bytes, builds the 80-word schedule,
-- runs 80 rounds, returns the updated five chaining variables.
-- ---------------------------------------------------------------------------
local function process_block(block_bytes, h0, h1, h2, h3, h4)
    -- Build the 80-word message schedule W
    local W = {}

    -- W[0..15]: directly from the block, 4 bytes per word, big-endian
    for i = 0, 15 do
        local base = i * 4 + 1  -- 1-indexed into block_bytes
        W[i] = bytes_to_word_be(
            block_bytes[base],
            block_bytes[base + 1],
            block_bytes[base + 2],
            block_bytes[base + 3]
        )
    end

    -- W[16..79]: expanded from earlier words
    -- The XOR of four prior words rotated left by 1 introduces additional
    -- diffusion — every output bit depends on every input bit eventually.
    for i = 16, 79 do
        W[i] = rol32(W[i-3] ~ W[i-8] ~ W[i-14] ~ W[i-16], 1)
    end

    -- Initialize working variables to current hash values
    local a, b, c, d, e = h0, h1, h2, h3, h4

    -- 80 rounds
    for t = 0, 79 do
        local F, K
        if t <= 19 then
            -- Ch: choose: if b then c else d
            F = (b & c) | ((~b) & d)
            K = K1
        elseif t <= 39 then
            -- Parity: odd number of bits set
            F = b ~ c ~ d
            K = K2
        elseif t <= 59 then
            -- Majority: at least two of {b, c, d} are 1
            F = (b & c) | (b & d) | (c & d)
            K = K3
        else
            -- Parity again
            F = b ~ c ~ d
            K = K4
        end

        local temp = u32(rol32(a, 5) + F + e + W[t] + K)
        e = d
        d = c
        c = rol32(b, 30)
        b = a
        a = temp
    end

    -- Add compressed chunk to running state (mod 2^32)
    return u32(h0 + a), u32(h1 + b), u32(h2 + c), u32(h3 + d), u32(h4 + e)
end

-- ---------------------------------------------------------------------------
-- digest(message) → array of 20 byte values (integers 0..255)
--
-- Computes the raw SHA-1 digest. Returns a 1-indexed Lua table of 20 integers.
-- ---------------------------------------------------------------------------
function M.digest(message)
    if type(message) ~= "string" then
        error("sha1.digest: expected string, got " .. type(message))
    end

    local padded = pad_message(message)

    -- Initialize the five 32-bit state words
    local h0, h1, h2, h3, h4 = H0_INIT, H1_INIT, H2_INIT, H3_INIT, H4_INIT

    -- Process each 64-byte (512-bit) block
    for block_start = 1, #padded, 64 do
        local block = {}
        for j = 0, 63 do
            block[j + 1] = padded[block_start + j]
        end
        h0, h1, h2, h3, h4 = process_block(block, h0, h1, h2, h3, h4)
    end

    -- Serialize five 32-bit big-endian words into 20 bytes
    local result = {}
    for _, w in ipairs({h0, h1, h2, h3, h4}) do
        for _, byte in ipairs(word_to_bytes_be(w)) do
            result[#result + 1] = byte
        end
    end
    return result
end

-- ---------------------------------------------------------------------------
-- hex(message) → 40-character lowercase hex string
-- ---------------------------------------------------------------------------
function M.hex(message)
    local raw = M.digest(message)
    local parts = {}
    for _, b in ipairs(raw) do
        parts[#parts + 1] = string.format("%02x", b)
    end
    return table.concat(parts)
end

return M
