-- sha256 — Pure Lua SHA-256 cryptographic hash function
--
-- SHA-256 is a member of the SHA-2 family, designed by the NSA and published
-- by NIST in 2001 as FIPS 180-2 (updated in FIPS 180-4). It produces a
-- 256-bit (32-byte) digest, typically displayed as a 64-character hex string.
--
-- Unlike MD5 (broken 2004) and SHA-1 (broken 2017), SHA-256 remains secure
-- with no known practical attacks. The birthday bound is 2^128, making
-- collision search computationally infeasible.
--
-- HOW SHA-256 WORKS — A GUIDED TOUR
-- ──────────────────────────────────
--
-- 1. PADDING (Merkle-Damgård, identical structure to SHA-1)
--    Append 0x80, then zero bytes until length ≡ 56 (mod 64).
--    Append original bit-length as a 64-bit **big-endian** integer.
--    The padded message is a multiple of 512 bits (64 bytes).
--
-- 2. INITIALIZATION (eight 32-bit words)
--    H0..H7 = first 32 bits of the fractional parts of the square roots
--    of the first 8 primes (2, 3, 5, 7, 11, 13, 17, 19).
--    These are "nothing up my sleeve" numbers — derived from an obvious
--    mathematical formula so no backdoor can be hidden.
--
-- 3. ROUND CONSTANTS (64 values)
--    K0..K63 = first 32 bits of the fractional parts of the cube roots
--    of the first 64 primes (2, 3, 5, ..., 311).
--
-- 4. MESSAGE SCHEDULE (per 512-bit block)
--    Each block is parsed as 16 big-endian 32-bit words W[0..15].
--    Extend to 64 words:
--      W[t] = σ1(W[t-2]) + W[t-7] + σ0(W[t-15]) + W[t-16]
--    where σ0 and σ1 are "small sigma" functions (defined below).
--
-- 5. COMPRESSION (64 rounds per block)
--    Working variables: a, b, c, d, e, f, g, h (copies of H0..H7)
--    Each round:
--      T1 = h + Σ1(e) + Ch(e,f,g) + K[t] + W[t]
--      T2 = Σ0(a) + Maj(a,b,c)
--      h=g, g=f, f=e, e=d+T1, d=c, c=b, b=a, a=T1+T2
--
-- 6. AUXILIARY FUNCTIONS
--    Ch(x,y,z)  = (x AND y) XOR (NOT x AND z)       — "choice"
--    Maj(x,y,z) = (x AND y) XOR (x AND z) XOR (y AND z) — "majority"
--    Σ0(x) = ROTR(2,x) XOR ROTR(13,x) XOR ROTR(22,x)   — "big sigma 0"
--    Σ1(x) = ROTR(6,x) XOR ROTR(11,x) XOR ROTR(25,x)   — "big sigma 1"
--    σ0(x) = ROTR(7,x) XOR ROTR(18,x) XOR SHR(3,x)     — "small sigma 0"
--    σ1(x) = ROTR(17,x) XOR ROTR(19,x) XOR SHR(10,x)   — "small sigma 1"
--
-- 7. OUTPUT
--    After all blocks, serialize H0..H7 each as 4 bytes **big-endian**
--    to produce the 32-byte digest.
--
-- LUA 5.4 BIT-ARITHMETIC NOTES
-- ─────────────────────────────
-- Lua 5.3+ has native 64-bit integers and bitwise operators: &, |, ~, <<, >>.
-- Use `& 0xFFFFFFFF` to keep arithmetic in the 32-bit range.
-- Right-rotate: ((x >> n) | (x << (32 - n))) & 0xFFFFFFFF
--
-- Usage:
--   local sha256 = require("coding_adventures.sha256")
--   local hex = sha256.sha256_hex("hello")
--   local raw = sha256.sha256("hello")      -- table of 32 integers (0..255)
--
--   -- Streaming:
--   local h = sha256.new()
--   h:update("ab")
--   h:update("c")
--   print(h:hex_digest())  -- same as sha256.sha256_hex("abc")
--
-- ============================================================================

local M = {}

M.VERSION = "0.1.0"

-- ---------------------------------------------------------------------------
-- Initial hash values H0..H7 (FIPS 180-4, Section 5.3.3)
--
-- First 32 bits of the fractional parts of the square roots of the first
-- 8 primes: 2, 3, 5, 7, 11, 13, 17, 19.
--
-- Example derivation for H0:
--   sqrt(2) = 1.41421356... → fractional part = 0.41421356...
--   0.41421356... × 2^32 = 1779033703.95... → floor = 0x6A09E667
-- ---------------------------------------------------------------------------
local H_INIT = {
    0x6A09E667,  -- H0 — sqrt(2)
    0xBB67AE85,  -- H1 — sqrt(3)
    0x3C6EF372,  -- H2 — sqrt(5)
    0xA54FF53A,  -- H3 — sqrt(7)
    0x510E527F,  -- H4 — sqrt(11)
    0x9B05688C,  -- H5 — sqrt(13)
    0x1F83D9AB,  -- H6 — sqrt(17)
    0x5BE0CD19,  -- H7 — sqrt(19)
}

-- ---------------------------------------------------------------------------
-- Round constants K0..K63 (FIPS 180-4, Section 4.2.2)
--
-- First 32 bits of the fractional parts of the cube roots of the first
-- 64 primes (2, 3, 5, 7, 11, 13, 17, 19, 23, 29, 31, 37, ..., 311).
-- ---------------------------------------------------------------------------
local K = {
    0x428A2F98, 0x71374491, 0xB5C0FBCF, 0xE9B5DBA5,
    0x3956C25B, 0x59F111F1, 0x923F82A4, 0xAB1C5ED5,
    0xD807AA98, 0x12835B01, 0x243185BE, 0x550C7DC3,
    0x72BE5D74, 0x80DEB1FE, 0x9BDC06A7, 0xC19BF174,
    0xE49B69C1, 0xEFBE4786, 0x0FC19DC6, 0x240CA1CC,
    0x2DE92C6F, 0x4A7484AA, 0x5CB0A9DC, 0x76F988DA,
    0x983E5152, 0xA831C66D, 0xB00327C8, 0xBF597FC7,
    0xC6E00BF3, 0xD5A79147, 0x06CA6351, 0x14292967,
    0x27B70A85, 0x2E1B2138, 0x4D2C6DFC, 0x53380D13,
    0x650A7354, 0x766A0ABB, 0x81C2C92E, 0x92722C85,
    0xA2BFE8A1, 0xA81A664B, 0xC24B8B70, 0xC76C51A3,
    0xD192E819, 0xD6990624, 0xF40E3585, 0x106AA070,
    0x19A4C116, 0x1E376C08, 0x2748774C, 0x34B0BCB5,
    0x391C0CB3, 0x4ED8AA4A, 0x5B9CCA4F, 0x682E6FF3,
    0x748F82EE, 0x78A5636F, 0x84C87814, 0x8CC70208,
    0x90BEFFFA, 0xA4506CEB, 0xBEF9A3F7, 0xC67178F2,
}

-- ---------------------------------------------------------------------------
-- u32(x) — clamp x to unsigned 32-bit integer
--
-- Lua 5.4 integers are 64-bit signed. Masking with 0xFFFFFFFF ensures we
-- stay in the unsigned 32-bit range, which is essential for SHA-256's
-- modular arithmetic.
-- ---------------------------------------------------------------------------
local function u32(x)
    return x & 0xFFFFFFFF
end

-- ---------------------------------------------------------------------------
-- rotr32(x, n) — right rotate x by n bits within 32 bits
--
-- SHA-256 uses right rotations (unlike SHA-1 which uses left rotations).
-- The left-shift component recovers the bits that "fall off" the right end
-- and wraps them to the left.
--
-- Example (8-bit, n=3):
--   x = 0b10110100
--   Right shift 3: 0b00010110  (leading bits become 0)
--   Left shift 5:  0b10000000  (the 3 bits that fell off, now at the top)
--   OR together:   0b10010110
-- ---------------------------------------------------------------------------
local function rotr32(x, n)
    x = x & 0xFFFFFFFF
    return ((x >> n) | (x << (32 - n))) & 0xFFFFFFFF
end

-- ---------------------------------------------------------------------------
-- SHA-256 Auxiliary Functions (FIPS 180-4, Section 4.1.2)
--
-- These six functions provide the non-linear mixing that makes SHA-256
-- a one-way function. Without non-linearity, the hash would be a simple
-- linear transformation — easily invertible.
-- ---------------------------------------------------------------------------

-- Ch(x,y,z) — "Choice": for each bit position, if x=1 choose y, else z
-- Truth table:
--   x=0, y=0, z=0 → 0    x=1, y=0, z=0 → 0
--   x=0, y=0, z=1 → 1    x=1, y=0, z=1 → 0
--   x=0, y=1, z=0 → 0    x=1, y=1, z=0 → 1
--   x=0, y=1, z=1 → 1    x=1, y=1, z=1 → 1
local function ch(x, y, z)
    return (x & y) ~ ((~x) & z)
end

-- Maj(x,y,z) — "Majority": output is the majority vote of the 3 inputs
-- If at least 2 of {x, y, z} have a 1, output 1; otherwise 0.
local function maj(x, y, z)
    return (x & y) ~ (x & z) ~ (y & z)
end

-- Σ0(x) — "Big Sigma 0": used in the compression rounds on variable 'a'
-- Mixes bits from three different rotation positions for maximum diffusion.
local function big_sigma0(x)
    return rotr32(x, 2) ~ rotr32(x, 13) ~ rotr32(x, 22)
end

-- Σ1(x) — "Big Sigma 1": used in the compression rounds on variable 'e'
local function big_sigma1(x)
    return rotr32(x, 6) ~ rotr32(x, 11) ~ rotr32(x, 25)
end

-- σ0(x) — "Small sigma 0": used in the message schedule expansion
-- Note: the third term is a right SHIFT (not rotate) — bits fall off permanently.
local function small_sigma0(x)
    return rotr32(x, 7) ~ rotr32(x, 18) ~ ((x & 0xFFFFFFFF) >> 3)
end

-- σ1(x) — "Small sigma 1": used in the message schedule expansion
local function small_sigma1(x)
    return rotr32(x, 17) ~ rotr32(x, 19) ~ ((x & 0xFFFFFFFF) >> 10)
end

-- ---------------------------------------------------------------------------
-- bytes_to_word_be(b0, b1, b2, b3) — pack 4 bytes as big-endian 32-bit word
-- SHA-256 is big-endian: the first byte is the most significant.
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
-- process_block(block_bytes, state)
--   → updated state (8-element table)
--
-- The SHA-256 compression function. Reads 64 bytes, builds the 64-word
-- message schedule, runs 64 rounds, and updates the eight chaining variables.
-- ---------------------------------------------------------------------------
local function process_block(block_bytes, state)
    -- Build the 64-word message schedule W
    local W = {}

    -- W[0..15]: directly from the block, 4 bytes per word, big-endian
    -- (Using 0-indexed internally to match the FIPS spec notation)
    for i = 0, 15 do
        local base = i * 4 + 1  -- 1-indexed into block_bytes
        W[i] = bytes_to_word_be(
            block_bytes[base],
            block_bytes[base + 1],
            block_bytes[base + 2],
            block_bytes[base + 3]
        )
    end

    -- W[16..63]: message schedule extension
    -- Each new word mixes four prior words through two "small sigma" functions.
    -- σ1 looks back 2 positions, plain addition looks back 7, σ0 looks back 15,
    -- and the final term looks back 16. This ensures that every input word
    -- influences many schedule entries.
    for i = 16, 63 do
        W[i] = u32(small_sigma1(W[i-2]) + W[i-7] + small_sigma0(W[i-15]) + W[i-16])
    end

    -- Initialize working variables from current hash state
    local a, b, c, d, e, f, g, h =
        state[1], state[2], state[3], state[4],
        state[5], state[6], state[7], state[8]

    -- 64 rounds of compression
    -- Each round mixes the message schedule word W[t] and round constant K[t]
    -- into the working variables through non-linear functions.
    for t = 0, 63 do
        -- T1 combines: current h, non-linear function of (e,f,g),
        --              round constant K[t], and schedule word W[t]
        local T1 = u32(h + big_sigma1(e) + ch(e, f, g) + K[t + 1] + W[t])

        -- T2 combines: non-linear function of (a,b,c)
        local T2 = u32(big_sigma0(a) + maj(a, b, c))

        -- Rotate working variables and insert T1 + T2
        h = g
        g = f
        f = e
        e = u32(d + T1)  -- d gets "promoted" with T1 mixed in
        d = c
        c = b
        b = a
        a = u32(T1 + T2)
    end

    -- Davies-Meyer feed-forward: add compressed output back to running state
    -- This makes the compression function one-way even if the block cipher
    -- (the round function) could be inverted.
    return {
        u32(state[1] + a), u32(state[2] + b),
        u32(state[3] + c), u32(state[4] + d),
        u32(state[5] + e), u32(state[6] + f),
        u32(state[7] + g), u32(state[8] + h),
    }
end

-- ---------------------------------------------------------------------------
-- sha256(message) → array of 32 byte values (integers 0..255)
--
-- Computes the raw SHA-256 digest. Returns a 1-indexed Lua table of 32
-- integers, each in range [0, 255].
-- ---------------------------------------------------------------------------
function M.sha256(message)
    if type(message) ~= "string" then
        error("sha256: expected string, got " .. type(message))
    end

    -- Step 1: Convert string to byte array
    local bytes = {}
    for i = 1, #message do
        bytes[i] = string.byte(message, i)
    end

    -- Step 2: Padding (Merkle-Damgård)
    local orig_len = #message
    local bit_len  = orig_len * 8

    -- Append 0x80
    bytes[#bytes + 1] = 0x80

    -- Append zeros until length ≡ 56 (mod 64)
    while #bytes % 64 ~= 56 do
        bytes[#bytes + 1] = 0x00
    end

    -- Append 64-bit big-endian length
    for shift = 56, 0, -8 do
        bytes[#bytes + 1] = (bit_len >> shift) & 0xFF
    end

    -- Step 3: Initialize state
    local state = {}
    for i = 1, 8 do
        state[i] = H_INIT[i]
    end

    -- Step 4: Process each 64-byte block
    for block_start = 1, #bytes, 64 do
        local block = {}
        for j = 0, 63 do
            block[j + 1] = bytes[block_start + j]
        end
        state = process_block(block, state)
    end

    -- Step 5: Serialize eight 32-bit big-endian words into 32 bytes
    local result = {}
    for i = 1, 8 do
        for _, byte_val in ipairs(word_to_bytes_be(state[i])) do
            result[#result + 1] = byte_val
        end
    end
    return result
end

-- ---------------------------------------------------------------------------
-- sha256_hex(message) → 64-character lowercase hex string
-- ---------------------------------------------------------------------------
function M.sha256_hex(message)
    local raw = M.sha256(message)
    local parts = {}
    for _, b in ipairs(raw) do
        parts[#parts + 1] = string.format("%02x", b)
    end
    return table.concat(parts)
end

-- ============================================================================
-- Streaming Hasher
-- ============================================================================
--
-- When the full message is not available at once — for example, reading a
-- large file in chunks — the streaming API allows incremental updates.
--
-- Internally we track:
--   - state: the eight-word running hash
--   - buffer: bytes not yet forming a complete 64-byte block
--   - total_len: total bytes fed so far (needed for the padding length)
--
-- Usage:
--   local h = sha256.new()
--   h:update("hello ")
--   h:update("world")
--   print(h:hex_digest())  -- same as sha256.sha256_hex("hello world")
-- ============================================================================

local Hasher = {}
Hasher.__index = Hasher

-- ---------------------------------------------------------------------------
-- new() → Hasher instance
-- ---------------------------------------------------------------------------
function M.new()
    local self = setmetatable({}, Hasher)
    self._state = {}
    for i = 1, 8 do
        self._state[i] = H_INIT[i]
    end
    self._buffer = {}
    self._total_len = 0
    return self
end

-- ---------------------------------------------------------------------------
-- update(data) → self (chainable)
--
-- Feed more bytes into the hasher. Processes complete 64-byte blocks
-- immediately and buffers any remainder.
-- ---------------------------------------------------------------------------
function Hasher:update(data)
    if type(data) ~= "string" then
        error("sha256 update: expected string, got " .. type(data))
    end

    -- Append new bytes to the buffer
    for i = 1, #data do
        self._buffer[#self._buffer + 1] = string.byte(data, i)
    end
    self._total_len = self._total_len + #data

    -- Process complete 64-byte blocks
    while #self._buffer >= 64 do
        local block = {}
        for j = 1, 64 do
            block[j] = self._buffer[j]
        end
        self._state = process_block(block, self._state)

        -- Remove processed bytes from buffer
        local remaining = {}
        for j = 65, #self._buffer do
            remaining[#remaining + 1] = self._buffer[j]
        end
        self._buffer = remaining
    end

    return self  -- chainable
end

-- ---------------------------------------------------------------------------
-- digest() → array of 32 byte values (non-destructive)
--
-- Returns the SHA-256 digest of all data fed so far. Does NOT modify the
-- internal state, so you can continue calling update() afterwards.
-- ---------------------------------------------------------------------------
function Hasher:digest()
    -- Work on a copy of the buffer so we don't modify internal state
    local buf = {}
    for i = 1, #self._buffer do
        buf[i] = self._buffer[i]
    end

    local bit_len = self._total_len * 8

    -- Padding
    buf[#buf + 1] = 0x80
    while #buf % 64 ~= 56 do
        buf[#buf + 1] = 0x00
    end
    for shift = 56, 0, -8 do
        buf[#buf + 1] = (bit_len >> shift) & 0xFF
    end

    -- Process remaining blocks with a copy of the state
    local state = {}
    for i = 1, 8 do
        state[i] = self._state[i]
    end

    for block_start = 1, #buf, 64 do
        local block = {}
        for j = 0, 63 do
            block[j + 1] = buf[block_start + j]
        end
        state = process_block(block, state)
    end

    -- Serialize
    local result = {}
    for i = 1, 8 do
        for _, byte_val in ipairs(word_to_bytes_be(state[i])) do
            result[#result + 1] = byte_val
        end
    end
    return result
end

-- ---------------------------------------------------------------------------
-- hex_digest() → 64-character lowercase hex string (non-destructive)
-- ---------------------------------------------------------------------------
function Hasher:hex_digest()
    local raw = self:digest()
    local parts = {}
    for _, b in ipairs(raw) do
        parts[#parts + 1] = string.format("%02x", b)
    end
    return table.concat(parts)
end

-- ---------------------------------------------------------------------------
-- copy() → independent deep copy of the hasher
--
-- Useful for computing multiple digests from a common prefix without
-- re-hashing the shared data.
-- ---------------------------------------------------------------------------
function Hasher:copy()
    local other = M.new()
    for i = 1, 8 do
        other._state[i] = self._state[i]
    end
    other._buffer = {}
    for i = 1, #self._buffer do
        other._buffer[i] = self._buffer[i]
    end
    other._total_len = self._total_len
    return other
end

return M
