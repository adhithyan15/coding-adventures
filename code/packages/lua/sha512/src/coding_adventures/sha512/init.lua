-- sha512 -- Pure Lua SHA-512 cryptographic hash function
--
-- SHA-512 (Secure Hash Algorithm 512) is the 64-bit sibling of SHA-256 in
-- the SHA-2 family, defined in FIPS PUB 180-4. It produces a 512-bit
-- (64-byte) digest, shown as a 128-character hex string. On 64-bit platforms,
-- SHA-512 is often *faster* than SHA-256 because it processes 128-byte blocks
-- and uses native 64-bit arithmetic.
--
-- HOW SHA-512 WORKS -- A GUIDED TOUR
-- -----------------------------------
--
-- 1. PADDING (same structure as SHA-256, but with a 128-bit length field)
--    Append 0x80, then zero bytes until length = 112 (mod 128).
--    Append the original bit-length as a 128-bit big-endian integer.
--    The padded message is a multiple of 1024 bits (128 bytes).
--
-- 2. INITIALIZATION (eight 64-bit words, FIPS 180-4 Section 5.3.5)
--    These are the first 64 bits of the fractional parts of the square
--    roots of the first eight primes (2, 3, 5, 7, 11, 13, 17, 19).
--
-- 3. MESSAGE SCHEDULE
--    Each 1024-bit block is expanded from 16 words (W[0..15]) to 80 words:
--      W[t] = sigma1(W[t-2]) + W[t-7] + sigma0(W[t-15]) + W[t-16]
--    Where sigma0 and sigma1 are bitwise rotation/shift functions.
--
-- 4. COMPRESSION (80 rounds per block)
--    Variables: a, b, c, d, e, f, g, h (copies of H0..H7 at block start)
--    Each round:
--      T1 = h + Sigma1(e) + Ch(e,f,g) + K[t] + W[t]
--      T2 = Sigma0(a) + Maj(a,b,c)
--      h=g, g=f, f=e, e=d+T1, d=c, c=b, b=a, a=T1+T2
--
--    Rotation/shift amounts (different from SHA-256, tuned for 64-bit):
--      Sigma0(x) = ROTR(28,x) XOR ROTR(34,x) XOR ROTR(39,x)
--      Sigma1(x) = ROTR(14,x) XOR ROTR(18,x) XOR ROTR(41,x)
--      sigma0(x) = ROTR(1,x) XOR ROTR(8,x) XOR SHR(7,x)
--      sigma1(x) = ROTR(19,x) XOR ROTR(61,x) XOR SHR(6,x)
--
-- 5. OUTPUT
--    After all blocks, serialize H0..H7 each as 8 bytes big-endian to
--    produce the 64-byte (512-bit) digest.
--
-- LUA 5.3+ 64-BIT ARITHMETIC NOTES
-- ---------------------------------
-- Lua 5.3+ has native 64-bit integers with bitwise operators (&, |, ~, >>,
-- <<). However, Lua integers are *signed* 64-bit. This matters because:
--
--   - Right shift (>>) is *arithmetic* (sign-extending) in Lua.
--   - We need *logical* right shift for SHA-512 rotations.
--   - Solution: mask with 0x7FFFFFFFFFFFFFFF after shift, or implement
--     logical right shift manually. We use a helper function ror64 that
--     handles this correctly.
--
-- The key trick: for a right rotation by n bits of a 64-bit value x:
--   ROTR(n, x) = (x >> n) | (x << (64 - n))
-- But since >> is arithmetic in Lua, the high bits from sign extension
-- get OR'd with the left-shifted part, which naturally produces the
-- correct rotation result. This is because rotation wants all 64 bits
-- to participate, and the sign-extended bits from >> occupy exactly the
-- positions that << would fill. So ROTR works correctly with arithmetic
-- shift!
--
-- For logical right shift (SHR), we mask: (x >> n) & (mask)
-- where mask = (1 << (64 - n)) - 1, clearing the sign-extended bits.
--
-- Usage:
--   local sha512 = require("coding_adventures.sha512")
--   local hex = sha512.hex("hello")
--   local raw = sha512.digest("hello")  -- {0x9b, 0x71, ...}  (64 integers)
--
-- ============================================================================

local M = {}

M.VERSION = "0.1.0"

-- ---------------------------------------------------------------------------
-- Round constants K[0..79] (FIPS 180-4 Section 4.2.3)
--
-- These are the first 64 bits of the fractional parts of the cube roots
-- of the first 80 prime numbers (2, 3, 5, 7, 11, ..., 409).
-- ---------------------------------------------------------------------------
local K = {
    [0]  = 0x428a2f98d728ae22, 0x7137449123ef65cd, 0xb5c0fbcfec4d3b2f, 0xe9b5dba58189dbbc,
    0x3956c25bf348b538, 0x59f111f1b605d019, 0x923f82a4af194f9b, 0xab1c5ed5da6d8118,
    0xd807aa98a3030242, 0x12835b0145706fbe, 0x243185be4ee4b28c, 0x550c7dc3d5ffb4e2,
    0x72be5d74f27b896f, 0x80deb1fe3b1696b1, 0x9bdc06a725c71235, 0xc19bf174cf692694,
    0xe49b69c19ef14ad2, 0xefbe4786384f25e3, 0x0fc19dc68b8cd5b5, 0x240ca1cc77ac9c65,
    0x2de92c6f592b0275, 0x4a7484aa6ea6e483, 0x5cb0a9dcbd41fbd4, 0x76f988da831153b5,
    0x983e5152ee66dfab, 0xa831c66d2db43210, 0xb00327c898fb213f, 0xbf597fc7beef0ee4,
    0xc6e00bf33da88fc2, 0xd5a79147930aa725, 0x06ca6351e003826f, 0x142929670a0e6e70,
    0x27b70a8546d22ffc, 0x2e1b21385c26c926, 0x4d2c6dfc5ac42aed, 0x53380d139d95b3df,
    0x650a73548baf63de, 0x766a0abb3c77b2a8, 0x81c2c92e47edaee6, 0x92722c851482353b,
    0xa2bfe8a14cf10364, 0xa81a664bbc423001, 0xc24b8b70d0f89791, 0xc76c51a30654be30,
    0xd192e819d6ef5218, 0xd69906245565a910, 0xf40e35855771202a, 0x106aa07032bbd1b8,
    0x19a4c116b8d2d0c8, 0x1e376c085141ab53, 0x2748774cdf8eeb99, 0x34b0bcb5e19b48a8,
    0x391c0cb3c5c95a63, 0x4ed8aa4ae3418acb, 0x5b9cca4f7763e373, 0x682e6ff3d6b2b8a3,
    0x748f82ee5defb2fc, 0x78a5636f43172f60, 0x84c87814a1f0ab72, 0x8cc702081a6439ec,
    0x90befffa23631e28, 0xa4506cebde82bde9, 0xbef9a3f7b2c67915, 0xc67178f2e372532b,
    0xca273eceea26619c, 0xd186b8c721c0c207, 0xeada7dd6cde0eb1e, 0xf57d4f7fee6ed178,
    0x06f067aa72176fba, 0x0a637dc5a2c898a6, 0x113f9804bef90dae, 0x1b710b35131c471b,
    0x28db77f523047d84, 0x32caab7b40c72493, 0x3c9ebe0a15c9bebc, 0x431d67c49c100d4c,
    0x4cc5d4becb3e42b6, 0x597f299cfc657e2a, 0x5fcb6fab3ad6faec, 0x6c44198c4a475817,
}

-- ---------------------------------------------------------------------------
-- Initial hash values (FIPS 180-4, Section 5.3.5)
--
-- First 64 bits of the fractional parts of the square roots of the first
-- 8 prime numbers (2, 3, 5, 7, 11, 13, 17, 19).
-- ---------------------------------------------------------------------------
local H_INIT = {
    0x6a09e667f3bcc908,
    0xbb67ae8584caa73b,
    0x3c6ef372fe94f82b,
    0xa54ff53a5f1d36f1,
    0x510e527fade682d1,
    0x9b05688c2b3e6c1f,
    0x1f83d9abfb41bd6b,
    0x5be0cd19137e2179,
}

-- ---------------------------------------------------------------------------
-- ror64(x, n) -- right rotate a 64-bit integer x by n bits
--
-- In Lua 5.3+, the >> operator performs arithmetic right shift (sign-
-- extending). For rotation, this is actually fine because:
--   (x >> n) | (x << (64 - n))
-- The sign-extended high bits from >> get OR'd with the wrapped-around
-- bits from <<, producing the correct 64-bit rotation.
-- ---------------------------------------------------------------------------
local function ror64(x, n)
    return (x >> n) | (x << (64 - n))
end

-- ---------------------------------------------------------------------------
-- shr64(x, n) -- logical right shift of a 64-bit integer by n bits
--
-- Unlike rotation, logical shift needs the high bits cleared.
-- We mask off the sign-extended bits by AND-ing with a mask that has
-- (64 - n) ones in the low bits.
-- ---------------------------------------------------------------------------
local function shr64(x, n)
    -- For n > 0: mask = (1 << (64 - n)) - 1, but we must avoid overflow.
    -- Lua's >> is arithmetic, so x >> n can have high bits set if x is negative.
    -- The mask clears them.
    if n >= 64 then return 0 end
    if n == 0 then return x end
    -- Use unsigned right shift trick: shift, then mask off the top n bits
    local shifted = x >> n
    -- Create mask with bottom (64-n) bits set
    -- -1 is all-ones in 64-bit, shift left by (64-n), invert
    local mask = ~(-1 << (64 - n))
    return shifted & mask
end

-- ---------------------------------------------------------------------------
-- Sigma0(x) = ROTR(28,x) XOR ROTR(34,x) XOR ROTR(39,x)
-- Used in the compression function for the 'a' variable.
-- ---------------------------------------------------------------------------
local function big_sigma0(x)
    return ror64(x, 28) ~ ror64(x, 34) ~ ror64(x, 39)
end

-- ---------------------------------------------------------------------------
-- Sigma1(x) = ROTR(14,x) XOR ROTR(18,x) XOR ROTR(41,x)
-- Used in the compression function for the 'e' variable.
-- ---------------------------------------------------------------------------
local function big_sigma1(x)
    return ror64(x, 14) ~ ror64(x, 18) ~ ror64(x, 41)
end

-- ---------------------------------------------------------------------------
-- sigma0(x) = ROTR(1,x) XOR ROTR(8,x) XOR SHR(7,x)
-- Used in the message schedule expansion.
-- ---------------------------------------------------------------------------
local function small_sigma0(x)
    return ror64(x, 1) ~ ror64(x, 8) ~ shr64(x, 7)
end

-- ---------------------------------------------------------------------------
-- sigma1(x) = ROTR(19,x) XOR ROTR(61,x) XOR SHR(6,x)
-- Used in the message schedule expansion.
-- ---------------------------------------------------------------------------
local function small_sigma1(x)
    return ror64(x, 19) ~ ror64(x, 61) ~ shr64(x, 6)
end

-- ---------------------------------------------------------------------------
-- Ch(x, y, z) = (x AND y) XOR (NOT x AND z)
-- "Choice": for each bit position, if x=1 choose y, if x=0 choose z.
-- ---------------------------------------------------------------------------
local function ch(x, y, z)
    return (x & y) ~ ((~x) & z)
end

-- ---------------------------------------------------------------------------
-- Maj(x, y, z) = (x AND y) XOR (x AND z) XOR (y AND z)
-- "Majority": output 1 if at least 2 of the 3 inputs are 1.
-- ---------------------------------------------------------------------------
local function maj(x, y, z)
    return (x & y) ~ (x & z) ~ (y & z)
end

-- ---------------------------------------------------------------------------
-- bytes_to_u64_be(b, offset) -- read 8 bytes big-endian as a 64-bit integer
--
-- SHA-512 treats the message as big-endian 64-bit words.
-- offset is 1-indexed into the byte array b.
-- ---------------------------------------------------------------------------
local function bytes_to_u64_be(b, offset)
    return (b[offset] << 56)
         | (b[offset + 1] << 48)
         | (b[offset + 2] << 40)
         | (b[offset + 3] << 32)
         | (b[offset + 4] << 24)
         | (b[offset + 5] << 16)
         | (b[offset + 6] << 8)
         |  b[offset + 7]
end

-- ---------------------------------------------------------------------------
-- u64_to_bytes_be(w) -- split a 64-bit word into 8 big-endian bytes
-- ---------------------------------------------------------------------------
local function u64_to_bytes_be(w)
    return {
        shr64(w, 56) & 0xFF,
        shr64(w, 48) & 0xFF,
        shr64(w, 40) & 0xFF,
        shr64(w, 32) & 0xFF,
        shr64(w, 24) & 0xFF,
        shr64(w, 16) & 0xFF,
        shr64(w,  8) & 0xFF,
                w     & 0xFF,
    }
end

-- ---------------------------------------------------------------------------
-- pad_message(msg) -> byte array padded to a multiple of 128 bytes
--
-- SHA-512 padding (FIPS 180-4, Section 5.1.2):
--   1. Append byte 0x80
--   2. Append 0x00 bytes until length = 112 (mod 128)
--   3. Append original bit-length as 128-bit big-endian integer
--      (we use two 64-bit words: high 64 bits then low 64 bits)
-- ---------------------------------------------------------------------------
local function pad_message(msg)
    local bytes = {}
    for i = 1, #msg do
        bytes[i] = string.byte(msg, i)
    end

    local orig_len = #msg
    local bit_len = orig_len * 8

    -- Step 1: append 0x80
    bytes[#bytes + 1] = 0x80

    -- Step 2: append 0x00 until length = 112 (mod 128)
    while #bytes % 128 ~= 112 do
        bytes[#bytes + 1] = 0x00
    end

    -- Step 3: append 128-bit big-endian length
    -- For messages < 2^64 bits, the high 64 bits are zero.
    -- High 64 bits (always 0 for messages we can handle)
    for _ = 1, 8 do
        bytes[#bytes + 1] = 0x00
    end
    -- Low 64 bits of bit_len
    for shift = 56, 0, -8 do
        bytes[#bytes + 1] = shr64(bit_len, shift) & 0xFF
    end

    return bytes
end

-- ---------------------------------------------------------------------------
-- process_block(block_bytes, state)
--   -> updated state (array of 8 values)
--
-- The SHA-512 compression function. Reads 128 bytes, builds the 80-word
-- schedule, runs 80 rounds, returns the updated eight chaining variables.
-- ---------------------------------------------------------------------------
local function process_block(block_bytes, state)
    -- Build the 80-word message schedule W
    local W = {}

    -- W[0..15]: directly from the block, 8 bytes per word, big-endian
    for i = 0, 15 do
        local base = i * 8 + 1  -- 1-indexed
        W[i] = bytes_to_u64_be(block_bytes, base)
    end

    -- W[16..79]: expanded from earlier words using sigma functions
    -- Each new word mixes 4 prior words through rotations and shifts,
    -- ensuring thorough bit diffusion across the entire schedule.
    for i = 16, 79 do
        W[i] = small_sigma1(W[i-2]) + W[i-7] + small_sigma0(W[i-15]) + W[i-16]
    end

    -- Initialize working variables to current hash values
    local a, b, c, d, e, f, g, h =
        state[1], state[2], state[3], state[4],
        state[5], state[6], state[7], state[8]

    -- 80 rounds of compression
    -- Each round mixes the working variables using the message schedule
    -- word W[t] and round constant K[t].
    for t = 0, 79 do
        local T1 = h + big_sigma1(e) + ch(e, f, g) + K[t] + W[t]
        local T2 = big_sigma0(a) + maj(a, b, c)
        h = g
        g = f
        f = e
        e = d + T1
        d = c
        c = b
        b = a
        a = T1 + T2
    end

    -- Add compressed chunk to running state (mod 2^64 -- automatic in Lua
    -- since integers are 64-bit and wrap on overflow)
    return {
        state[1] + a,
        state[2] + b,
        state[3] + c,
        state[4] + d,
        state[5] + e,
        state[6] + f,
        state[7] + g,
        state[8] + h,
    }
end

-- ---------------------------------------------------------------------------
-- digest(message) -> array of 64 byte values (integers 0..255)
--
-- Computes the raw SHA-512 digest. Returns a 1-indexed Lua table of 64
-- integers, each in the range 0-255.
-- ---------------------------------------------------------------------------
function M.digest(message)
    if type(message) ~= "string" then
        error("sha512.digest: expected string, got " .. type(message))
    end

    local padded = pad_message(message)

    -- Initialize the eight 64-bit state words
    local state = {}
    for i = 1, 8 do
        state[i] = H_INIT[i]
    end

    -- Process each 128-byte (1024-bit) block
    for block_start = 1, #padded, 128 do
        local block = {}
        for j = 0, 127 do
            block[j + 1] = padded[block_start + j]
        end
        state = process_block(block, state)
    end

    -- Serialize eight 64-bit big-endian words into 64 bytes
    local result = {}
    for i = 1, 8 do
        for _, byte_val in ipairs(u64_to_bytes_be(state[i])) do
            result[#result + 1] = byte_val
        end
    end
    return result
end

-- ---------------------------------------------------------------------------
-- hex(message) -> 128-character lowercase hex string
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
