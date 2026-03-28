-- md5 — Pure Lua MD5 cryptographic hash function
--
-- MD5 (Message-Digest Algorithm 5) was designed by Ron Rivest in 1991 and
-- produces a 128-bit (16-byte) hash value, typically expressed as a 32-
-- character hexadecimal string. Although MD5 is now considered cryptographically
-- broken for security purposes (collision attacks are practical), it is still
-- widely used for:
--   * Checksums and data-integrity verification
--   * Non-security hash keys (e.g. cache keys, ETags)
--   * Legacy system compatibility
--
-- HOW MD5 WORKS — A GUIDED TOUR
-- ──────────────────────────────
-- 1. PADDING
--    The message is padded so its length in bits ≡ 448 (mod 512).
--    Padding starts with a single '1' bit (byte 0x80), followed by '0' bits.
--    The original message length in bits is appended as a 64-bit little-endian
--    integer. After padding, length ≡ 0 (mod 512), i.e. a whole number of
--    512-bit (64-byte) blocks.
--
-- 2. INITIALIZATION
--    Four 32-bit words (A, B, C, D) are set to fixed "magic" constants:
--       A = 0x67452301
--       B = 0xEFCDAB89
--       C = 0x98BADCFE
--       D = 0x10325476
--    These are not random — they are the integers 0..3 written in little-endian
--    order, byte-reversed. Their purpose is purely to avoid an all-zeros start.
--
-- 3. COMPRESSION (one 64-byte block at a time)
--    Each block runs 64 rounds grouped into 4 "rounds" of 16 operations each.
--    Every round uses a different auxiliary function F, G, H, or I:
--       Round 0-15:  F(B,C,D) = (B AND C) OR (NOT B AND D)  (bit-select)
--       Round 16-31: G(B,C,D) = (B AND D) OR (C AND NOT D)
--       Round 32-47: H(B,C,D) = B XOR C XOR D
--       Round 48-63: I(B,C,D) = C XOR (B OR NOT D)
--    Per-round addition constant T[i] = floor(abs(sin(i+1)) × 2^32).
--    The message schedule uses different message word indices each round.
--
-- 4. OUTPUT
--    After all blocks, A||B||C||D are each serialized as 4 bytes little-endian
--    to produce the 16-byte digest.
--
-- LUA 5.4 BIT-ARITHMETIC NOTES
-- ─────────────────────────────
-- Lua 5.4 uses 64-bit signed integers by default. MD5 needs 32-bit unsigned
-- arithmetic with wrap-around. We achieve this by masking every addition with
-- `& 0xFFFFFFFF`. Shifts and bitwise ops are native in Lua 5.4 (no bit library
-- needed): `&`, `|`, `~` (complement), `<<`, `>>`.
--
-- Usage:
--   local md5 = require("coding_adventures.md5")
--   local hex = md5.hex("hello")      -- "5d41402abc4b2a76b9719d911017c592"
--   local raw = md5.digest("hello")   -- {0x5d, 0x41, ...}  (16 integers)
--
-- ============================================================================

local M = {}

M.VERSION = "0.1.0"

-- ---------------------------------------------------------------------------
-- Per-round T constants: T[i] = floor(abs(sin(i+1)) * 2^32)  for i = 0..63
-- Pre-computing these avoids 64 floating-point operations per hash call.
-- ---------------------------------------------------------------------------
local T = {}
do
    local TWO32 = 2^32
    for i = 0, 63 do
        -- math.sin takes radians; i+1 keeps us away from sin(0)=0
        T[i + 1] = math.floor(math.abs(math.sin(i + 1)) * TWO32) & 0xFFFFFFFF
    end
end

-- ---------------------------------------------------------------------------
-- Message-word index tables for the four rounds
-- These encode which 32-bit word of the 16-word message block is mixed in
-- at each step. Derived from the MD5 RFC.
-- ---------------------------------------------------------------------------

-- Round 1 (steps 0-15): sequential
local IDX1 = {0,1,2,3, 4,5,6,7, 8,9,10,11, 12,13,14,15}

-- Round 2 (steps 16-31): (1 + 5*i) mod 16
local IDX2 = {}
for i = 0, 15 do IDX2[i+1] = (1 + 5*i) % 16 end

-- Round 3 (steps 32-47): (5 + 3*i) mod 16
local IDX3 = {}
for i = 0, 15 do IDX3[i+1] = (5 + 3*i) % 16 end

-- Round 4 (steps 48-63): (7*i) mod 16
local IDX4 = {}
for i = 0, 15 do IDX4[i+1] = (7*i) % 16 end

-- Shift amounts per round (from the RFC — 16 values per round, repeated 4 times)
local S1 = {7,12,17,22, 7,12,17,22, 7,12,17,22, 7,12,17,22}
local S2 = {5, 9,14,20, 5, 9,14,20, 5, 9,14,20, 5, 9,14,20}
local S3 = {4,11,16,23, 4,11,16,23, 4,11,16,23, 4,11,16,23}
local S4 = {6,10,15,21, 6,10,15,21, 6,10,15,21, 6,10,15,21}

-- ---------------------------------------------------------------------------
-- rol32(x, n) — rotate x left by n bits, keeping result 32-bit
--
-- A left rotation shifts bits left and wraps the high bits back into the low
-- end. This is used extensively in MD5 to "spread" bits across the word.
-- ---------------------------------------------------------------------------
local function rol32(x, n)
    x = x & 0xFFFFFFFF
    return ((x << n) | (x >> (32 - n))) & 0xFFFFFFFF
end

-- ---------------------------------------------------------------------------
-- u32(x) — force x into the unsigned 32-bit range
-- ---------------------------------------------------------------------------
local function u32(x)
    return x & 0xFFFFFFFF
end

-- ---------------------------------------------------------------------------
-- bytes_to_words_le(bytes) → table of 32-bit little-endian words
--
-- Groups 4 bytes into one 32-bit integer, least-significant byte first.
-- MD5 is entirely little-endian: word[0] = bytes[0] | (bytes[1]<<8) | ...
-- ---------------------------------------------------------------------------
local function bytes_to_words_le(bytes)
    local words = {}
    for i = 1, #bytes, 4 do
        local b0 = bytes[i]     or 0
        local b1 = bytes[i + 1] or 0
        local b2 = bytes[i + 2] or 0
        local b3 = bytes[i + 3] or 0
        words[#words + 1] = b0 | (b1 << 8) | (b2 << 16) | (b3 << 24)
    end
    return words
end

-- ---------------------------------------------------------------------------
-- word_to_bytes_le(w) → 4 bytes, little-endian
-- ---------------------------------------------------------------------------
local function word_to_bytes_le(w)
    w = u32(w)
    return {w & 0xFF, (w >> 8) & 0xFF, (w >> 16) & 0xFF, (w >> 24) & 0xFF}
end

-- ---------------------------------------------------------------------------
-- pad_message(msg) → byte array, padded to a multiple of 64 bytes
--
-- RFC 1321 padding:
--   1. Append bit 1 (byte 0x80)
--   2. Append 0x00 bytes until length ≡ 56 (mod 64)
--   3. Append original bit-length as 64-bit little-endian
-- ---------------------------------------------------------------------------
local function pad_message(msg)
    local bytes = {}
    for i = 1, #msg do
        bytes[i] = string.byte(msg, i)
    end

    local orig_len = #msg  -- original length in bytes
    local bit_len  = orig_len * 8  -- length in bits

    -- Step 1: append 0x80
    bytes[#bytes + 1] = 0x80

    -- Step 2: append 0x00 until length ≡ 56 (mod 64)
    while #bytes % 64 ~= 56 do
        bytes[#bytes + 1] = 0x00
    end

    -- Step 3: append 64-bit little-endian length
    -- Lua integers are 64-bit, so bit_len fits without overflow for reasonable inputs.
    for shift = 0, 56, 8 do
        bytes[#bytes + 1] = (bit_len >> shift) & 0xFF
    end

    return bytes
end

-- ---------------------------------------------------------------------------
-- process_block(words, a0, b0, c0, d0) → a, b, c, d (updated state)
--
-- The core MD5 compression function. Takes 16 message words and the current
-- (A, B, C, D) chaining variables, performs 64 rounds, returns new state.
-- ---------------------------------------------------------------------------
local function process_block(M_words, a, b, c, d)
    -- Round 1: F(B,C,D) = (B AND C) OR (NOT B AND D)
    -- This is a "mux": select C if B=1, else select D.
    for i = 1, 16 do
        local F = (b & c) | ((~b) & d)
        local g = IDX1[i]  -- 0-based word index
        local temp = u32(a + F + M_words[g + 1] + T[i])
        temp = rol32(temp, S1[i])
        temp = u32(temp + b)
        a, b, c, d = d, temp, b, c
    end

    -- Round 2: G(B,C,D) = (B AND D) OR (C AND NOT D)
    -- Like F but with different operand pairings.
    for i = 1, 16 do
        local G = (b & d) | (c & (~d))
        local g = IDX2[i]
        local temp = u32(a + G + M_words[g + 1] + T[16 + i])
        temp = rol32(temp, S2[i])
        temp = u32(temp + b)
        a, b, c, d = d, temp, b, c
    end

    -- Round 3: H(B,C,D) = B XOR C XOR D
    -- Produces high avalanche effect.
    for i = 1, 16 do
        local H = b ~ c ~ d
        local g = IDX3[i]
        local temp = u32(a + H + M_words[g + 1] + T[32 + i])
        temp = rol32(temp, S3[i])
        temp = u32(temp + b)
        a, b, c, d = d, temp, b, c
    end

    -- Round 4: I(B,C,D) = C XOR (B OR NOT D)
    -- The NOT gives this round its distinctive "anti-correlation" behaviour.
    for i = 1, 16 do
        local I_val = c ~ (b | (~d))
        local g = IDX4[i]
        local temp = u32(a + I_val + M_words[g + 1] + T[48 + i])
        temp = rol32(temp, S4[i])
        temp = u32(temp + b)
        a, b, c, d = d, temp, b, c
    end

    return a, b, c, d
end

-- ---------------------------------------------------------------------------
-- digest(message) → array of 16 byte values (integers 0..255)
--
-- Computes the raw MD5 digest. Returns a 1-indexed Lua table of 16 integers.
-- ---------------------------------------------------------------------------
function M.digest(message)
    if type(message) ~= "string" then
        error("md5.digest: expected string, got " .. type(message))
    end

    -- Pad the message
    local padded = pad_message(message)

    -- Initialize the four 32-bit state words (RFC 1321 Section 3.3)
    local a0 = 0x67452301
    local b0 = 0xEFCDAB89
    local c0 = 0x98BADCFE
    local d0 = 0x10325476

    -- Process each 64-byte (512-bit) block
    for block_start = 1, #padded, 64 do
        -- Extract 16 little-endian 32-bit words from this block
        local block_bytes = {}
        for j = 0, 63 do
            block_bytes[j + 1] = padded[block_start + j]
        end
        local words = bytes_to_words_le(block_bytes)

        -- Run the compression function
        local da, db, dc, dd = process_block(words, a0, b0, c0, d0)

        -- Add compressed chunk to running state (mod 2^32)
        a0 = u32(a0 + da)
        b0 = u32(b0 + db)
        c0 = u32(c0 + dc)
        d0 = u32(d0 + dd)
    end

    -- Produce output: each of A, B, C, D as 4 little-endian bytes
    local result = {}
    for _, w in ipairs({a0, b0, c0, d0}) do
        for _, byte in ipairs(word_to_bytes_le(w)) do
            result[#result + 1] = byte
        end
    end
    return result
end

-- ---------------------------------------------------------------------------
-- hex(message) → 32-character lowercase hex string
--
-- The most common way to display an MD5 hash. Each of the 16 digest bytes is
-- formatted as exactly two lowercase hex digits.
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
