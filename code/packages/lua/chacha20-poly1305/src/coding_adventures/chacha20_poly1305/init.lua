-- ============================================================================
-- coding_adventures.chacha20_poly1305
-- ============================================================================
--
-- ChaCha20-Poly1305 authenticated encryption (RFC 8439).
--
-- This module implements the three components of the ChaCha20-Poly1305 AEAD:
--
--   1. **ChaCha20** — a stream cipher using only ARX operations (Add, Rotate,
--      XOR). Designed by Daniel J. Bernstein as an improvement over Salsa20.
--
--   2. **Poly1305** — a one-time message authentication code (MAC) that
--      produces a 16-byte tag. Also by Bernstein.
--
--   3. **AEAD** — the combined construction from RFC 8439 Section 2.8 that
--      provides both confidentiality and authenticity.
--
-- Why ChaCha20 over AES?
-- ~~~~~~~~~~~~~~~~~~~~~~
-- AES relies on lookup tables (S-boxes) and is only fast when the CPU has
-- dedicated AES-NI instructions. On CPUs without AES-NI (common on mobile),
-- AES is slow and vulnerable to cache-timing side-channel attacks. ChaCha20
-- uses only addition, rotation, and XOR — operations that run in constant
-- time on every CPU.
--
-- Where is ChaCha20-Poly1305 used?
-- ~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~
-- TLS 1.3, WireGuard, SSH (chacha20-poly1305@openssh.com), Chrome/Android
-- HTTPS, and many other protocols.
--
-- IMPORTANT: This implementation is for educational purposes. Do not use
-- it for real cryptography — use a vetted library like libsodium instead.
--
-- Requires Lua 5.4 for native 64-bit integers and bitwise operators.
-- ============================================================================

local M = {}

-- ============================================================================
-- Utility: 32-bit Wrapping Arithmetic
-- ============================================================================
--
-- ChaCha20 operates on 32-bit unsigned integers. Lua 5.4 integers are 64-bit,
-- so we must mask results to 32 bits after every addition. The bitwise AND
-- with 0xFFFFFFFF keeps only the lower 32 bits, simulating unsigned overflow.
-- ============================================================================

local MASK32 = 0xFFFFFFFF

--- Add two 32-bit values with wrapping.
local function add32(a, b)
    return (a + b) & MASK32
end

--- Left-rotate a 32-bit value by n bits.
--
-- A left rotation moves bits toward the most-significant position, with bits
-- that "fall off" the top wrapping around to the bottom:
--
--   rotl(0x80000001, 1) = 0x00000003
--
-- We achieve this by shifting left and OR-ing with the bits shifted right
-- from the other end, then masking to 32 bits.
local function rotl32(x, n)
    return ((x << n) | (x >> (32 - n))) & MASK32
end

-- ============================================================================
-- ChaCha20 Quarter Round
-- ============================================================================
--
-- The quarter round is the core mixing function of ChaCha20. It operates on
-- four 32-bit words (a, b, c, d) using the ARX pattern:
--
--   a += b; d ^= a; d <<<= 16    -- Addition, XOR, Rotation
--   c += d; b ^= c; b <<<= 12
--   a += b; d ^= a; d <<<= 8
--   c += d; b ^= c; b <<<= 7
--
-- The rotation amounts (16, 12, 8, 7) were chosen by Bernstein to maximize
-- diffusion — after a few rounds, every input bit affects every output bit.
-- ============================================================================

--- Perform one quarter round on four words in the state array.
-- @param s  State array (16 elements, 1-indexed)
-- @param a  Index of first word
-- @param b  Index of second word
-- @param c  Index of third word
-- @param d  Index of fourth word
local function quarter_round(s, a, b, c, d)
    s[a] = add32(s[a], s[b]); s[d] = rotl32(s[d] ~ s[a], 16)
    s[c] = add32(s[c], s[d]); s[b] = rotl32(s[b] ~ s[c], 12)
    s[a] = add32(s[a], s[b]); s[d] = rotl32(s[d] ~ s[a],  8)
    s[c] = add32(s[c], s[d]); s[b] = rotl32(s[b] ~ s[c],  7)
end

-- ============================================================================
-- ChaCha20 Block Function
-- ============================================================================
--
-- The ChaCha20 state is a 4x4 matrix of 32-bit words laid out as:
--
--    0  1  2  3     ←  Constants ("expand 32-byte k")
--    4  5  6  7     ←  Key (first half)
--    8  9 10 11     ←  Key (second half)
--   12 13 14 15     ←  Counter + Nonce
--
-- The magic constants spell "expand 32-byte k" in ASCII when read as
-- little-endian 32-bit words:
--
--   0x61707865  →  "expa"
--   0x3320646e  →  "nd 3"
--   0x79622d32  →  "2-by"   (bytes reversed: "2-by" → 0x79622d32)
--   0x6b206574  →  "te k"
--
-- After 20 rounds (10 column rounds + 10 diagonal rounds), the original
-- state is added back to the mixed state. This "feed-forward" prevents an
-- attacker from inverting the rounds.
-- ============================================================================

local CONSTANTS = { 0x61707865, 0x3320646e, 0x79622d32, 0x6b206574 }

--- Read a little-endian 32-bit unsigned integer from a byte string.
-- @param s    Byte string
-- @param pos  1-based offset
-- @return     32-bit integer
local function le32(s, pos)
    local b0, b1, b2, b3 = string.byte(s, pos, pos + 3)
    return b0 | (b1 << 8) | (b2 << 16) | (b3 << 24)
end

--- Encode a 32-bit integer as a 4-byte little-endian string.
local function to_le32(n)
    return string.char(
        n & 0xFF,
        (n >> 8) & 0xFF,
        (n >> 16) & 0xFF,
        (n >> 24) & 0xFF
    )
end

--- Encode a 64-bit integer as an 8-byte little-endian string.
local function to_le64(n)
    return string.char(
        n & 0xFF,
        (n >>  8) & 0xFF,
        (n >> 16) & 0xFF,
        (n >> 24) & 0xFF,
        (n >> 32) & 0xFF,
        (n >> 40) & 0xFF,
        (n >> 48) & 0xFF,
        (n >> 56) & 0xFF
    )
end

--- Generate one 64-byte keystream block.
-- @param key      32-byte key string
-- @param nonce    12-byte nonce string
-- @param counter  32-bit block counter
-- @return         64-byte keystream string
local function chacha20_block(key, nonce, counter)
    -- Initialize the 16-word state (1-indexed in Lua).
    local state = {
        CONSTANTS[1], CONSTANTS[2], CONSTANTS[3], CONSTANTS[4],
        le32(key, 1),  le32(key, 5),  le32(key, 9),  le32(key, 13),
        le32(key, 17), le32(key, 21), le32(key, 25), le32(key, 29),
        counter & MASK32,
        le32(nonce, 1), le32(nonce, 5), le32(nonce, 9),
    }

    -- Copy the initial state for the feed-forward at the end.
    local initial = {}
    for i = 1, 16 do initial[i] = state[i] end

    -- 20 rounds = 10 iterations of (column round + diagonal round).
    --
    -- Column rounds operate on the columns of the 4x4 matrix:
    --   QR(0,4, 8,12)  QR(1,5, 9,13)  QR(2,6,10,14)  QR(3,7,11,15)
    --
    -- Diagonal rounds operate on the diagonals:
    --   QR(0,5,10,15)  QR(1,6,11,12)  QR(2,7, 8,13)  QR(3,4, 9,14)
    --
    -- (Note: Lua arrays are 1-indexed, so we add 1 to all indices.)
    for _ = 1, 10 do
        -- Column round
        quarter_round(state, 1, 5,  9, 13)
        quarter_round(state, 2, 6, 10, 14)
        quarter_round(state, 3, 7, 11, 15)
        quarter_round(state, 4, 8, 12, 16)
        -- Diagonal round
        quarter_round(state, 1, 6, 11, 16)
        quarter_round(state, 2, 7, 12, 13)
        quarter_round(state, 3, 8,  9, 14)
        quarter_round(state, 4, 5, 10, 15)
    end

    -- Feed-forward: add the initial state to prevent inversion.
    local parts = {}
    for i = 1, 16 do
        parts[i] = to_le32(add32(state[i], initial[i]))
    end

    return table.concat(parts)
end

-- ============================================================================
-- ChaCha20 Stream Cipher
-- ============================================================================
--
-- ChaCha20 encrypts by XOR-ing plaintext with a keystream. The keystream is
-- generated 64 bytes at a time, incrementing the block counter for each block.
-- Since XOR is its own inverse, encryption and decryption are the same
-- operation — just XOR the data with the keystream again.
-- ============================================================================

--- Encrypt (or decrypt) data using ChaCha20.
-- @param data     Input bytes (plaintext or ciphertext)
-- @param key      32-byte key
-- @param nonce    12-byte nonce
-- @param counter  Initial block counter (usually 0 or 1)
-- @return         Output bytes (same length as input)
function M.chacha20_encrypt(data, key, nonce, counter)
    assert(#key == 32, "Key must be 32 bytes")
    assert(#nonce == 12, "Nonce must be 12 bytes")
    counter = counter or 0

    local result = {}
    local data_len = #data

    for offset = 0, data_len - 1, 64 do
        local block = chacha20_block(key, nonce, counter)
        counter = counter + 1

        local chunk_len = math.min(64, data_len - offset)
        for i = 1, chunk_len do
            local p = string.byte(data, offset + i)
            local k = string.byte(block, i)
            result[#result + 1] = string.char(p ~ k)
        end
    end

    return table.concat(result)
end

-- ============================================================================
-- Poly1305 Message Authentication Code
-- ============================================================================
--
-- Poly1305 is a one-time MAC based on polynomial evaluation modulo a prime
-- near 2^130. It was designed by Bernstein for speed and provable security.
--
-- How it works:
-- ~~~~~~~~~~~~
-- 1. Split the 32-byte key into two 16-byte halves:
--    - r: the "clamped" multiplier (certain bits forced to 0)
--    - s: the final additive constant
--
-- 2. Break the message into 16-byte blocks. For each block:
--    a. Read the block as a little-endian integer
--    b. Append a 0x01 byte (mathematically: add 2^(8*block_len))
--    c. Add it to the accumulator
--    d. Multiply by r, reduce modulo 2^130 - 5
--
-- 3. After all blocks: tag = (accumulator + s) mod 2^128
--
-- Why clamping?
-- ~~~~~~~~~~~~
-- Clamping forces certain bits of r to 0, ensuring that the polynomial
-- evaluation stays well-behaved. Without clamping, there are subtle attacks
-- possible. The clamped bits are chosen so that r has at most 124 bits of
-- "entropy" and certain alignment properties hold.
--
-- Big integer arithmetic:
-- ~~~~~~~~~~~~~~~~~~~~~~
-- Poly1305 needs 130-bit arithmetic. Lua 5.4/5.5 integers are 64-bit signed,
-- so we represent big numbers as tables of 5 "limbs" of 26 bits each.
-- 5 * 26 = 130 bits, exactly what Poly1305 needs.
--
-- The product of two 26-bit numbers is at most 52 bits, which fits
-- comfortably in a 63-bit signed Lua integer. This avoids all overflow
-- concerns during multiplication.
-- ============================================================================

-- ---------------------------------------------------------------------------
-- 5-limb, 26-bit representation for Poly1305
--
-- A number N is represented as {h0, h1, h2, h3, h4} where:
--   N = h0 + h1*2^26 + h2*2^52 + h3*2^78 + h4*2^104
--
-- Each limb holds 26 bits (values 0..0x3FFFFFF) when normalized.
-- During intermediate computations limbs may temporarily exceed 26 bits.
-- ---------------------------------------------------------------------------

local LIMB26 = 26
local MASK26 = (1 << LIMB26) - 1  -- 0x3FFFFFF

--- Read a 16-byte (or shorter) little-endian byte string into 5 limbs of 26 bits.
-- If set_hibit is true, a 0x01 sentinel is logically appended after the
-- last byte (at bit position 8*len). This is the standard Poly1305 block
-- encoding.
--
-- Strategy: pad the input to 17 bytes, read four little-endian 32-bit words,
-- then split at the 26-bit boundaries. This avoids the error-prone bit
-- straddling logic entirely.
local function bytes_to_5limbs(bytes, set_hibit)
    -- Pad to exactly 17 bytes with zeros (the extra byte is for the hibit).
    local n = #bytes
    local padded = {}
    for i = 1, n do
        padded[i] = string.byte(bytes, i)
    end
    -- Set the hibit sentinel: a 1-byte at position n+1 (0-indexed byte n).
    if set_hibit then
        padded[n + 1] = 0x01
    end
    -- Fill rest with zeros up to 17 bytes.
    for i = #padded + 1, 17 do
        padded[i] = 0
    end

    -- Read four 32-bit little-endian words + one byte for the top.
    local function u32(off)
        return padded[off] | (padded[off+1] << 8) | (padded[off+2] << 16) | (padded[off+3] << 24)
    end

    local t0 = u32(1)   -- bits 0..31
    local t1 = u32(5)   -- bits 32..63
    local t2 = u32(9)   -- bits 64..95
    local t3 = u32(13)  -- bits 96..127
    local t4 = padded[17]  -- bits 128..135

    -- Now split into 5 limbs of 26 bits each:
    -- h0 = bits  0..25  = t0 & 0x3FFFFFF
    -- h1 = bits 26..51  = (t0 >> 26) | (t1 << 6)  & 0x3FFFFFF
    -- h2 = bits 52..77  = (t1 >> 20) | (t2 << 12) & 0x3FFFFFF
    -- h3 = bits 78..103 = (t2 >> 14) | (t3 << 18) & 0x3FFFFFF
    -- h4 = bits 104..129= (t3 >> 8)  | (t4 << 24) & 0x3FFFFFF
    local h = {
        t0 & MASK26,
        ((t0 >> 26) | (t1 << 6)) & MASK26,
        ((t1 >> 20) | (t2 << 12)) & MASK26,
        ((t2 >> 14) | (t3 << 18)) & MASK26,
        ((t3 >> 8)  | (t4 << 24)) & MASK26,
    }

    return h
end

--- Convert 5 limbs back to a 16-byte little-endian string.
-- Only the lower 128 bits are output (mod 2^128 is implicit).
--
-- Strategy: reconstruct four 32-bit words from the limbs, then write
-- them as little-endian bytes. This mirrors bytes_to_5limbs exactly.
local function limbs5_to_bytes16(h)
    -- First, normalize the limbs (carry propagation).
    for i = 1, 4 do
        h[i + 1] = h[i + 1] + (h[i] >> LIMB26)
        h[i] = h[i] & MASK26
    end
    -- h[5] may exceed 26 bits; we only take 128 bits total.

    -- Reconstruct 32-bit words:
    -- t0 = h0 | (h1 << 26)  → bits 0..51, but we only want bits 0..31
    -- t1 = (h1 >> 6) | (h2 << 20) → bits 32..63
    -- t2 = (h2 >> 12) | (h3 << 14) → bits 64..95
    -- t3 = (h3 >> 18) | (h4 << 8) → bits 96..127
    local h0, h1, h2, h3, h4 = h[1], h[2], h[3], h[4], h[5]

    local t0 = (h0 | (h1 << 26)) & MASK32
    local t1 = ((h1 >> 6) | (h2 << 20)) & MASK32
    local t2 = ((h2 >> 12) | (h3 << 14)) & MASK32
    local t3 = ((h3 >> 18) | (h4 << 8)) & MASK32

    return to_le32(t0) .. to_le32(t1) .. to_le32(t2) .. to_le32(t3)
end

--- Add two 5-limb numbers (no reduction, just plain add + carry).
local function limbs5_add(a, b)
    local c = {
        a[1] + b[1],
        a[2] + b[2],
        a[3] + b[3],
        a[4] + b[4],
        a[5] + b[5],
    }
    -- Carry propagation
    for i = 1, 4 do
        c[i + 1] = c[i + 1] + (c[i] >> LIMB26)
        c[i] = c[i] & MASK26
    end
    return c
end

--- Multiply two 5-limb numbers and reduce modulo p = 2^130 - 5.
--
-- This is the heart of Poly1305. We use the standard technique:
--
-- The full product of two 130-bit numbers has up to 260 bits, spread
-- across 10 limbs (d0..d9). We reduce using the identity:
--
--   x * 2^130 ≡ x * 5 (mod p)
--
-- So any product term that would land in limb >= 5 (i.e., bits >= 130)
-- gets multiplied by 5 and folded back into the lower limbs.
--
-- Concretely, for product terms a[i]*b[j] where i+j >= 5:
--   contribution = a[i] * b[j] * 5   (folded into limb (i+j-5))
--
-- This is often written using "r5" values: r5[i] = r[i] * 5.
local function limbs5_mul_mod(a, r)
    local a0, a1, a2, a3, a4 = a[1], a[2], a[3], a[4], a[5]
    local r0, r1, r2, r3, r4 = r[1], r[2], r[3], r[4], r[5]

    -- Precompute r[i]*5 for the reduction trick.
    local s1 = r1 * 5
    local s2 = r2 * 5
    local s3 = r3 * 5
    local s4 = r4 * 5

    -- Schoolbook multiplication with reduction folded in.
    -- d[k] = sum of a[i]*r[j] where (i+j) mod 5 == k,
    --         with a factor of 5 when i+j >= 5.
    --
    -- Each product is at most 26*26 = 52 bits.
    -- Each d[k] sums at most 5 such products, so max ~55 bits. Safe in 63-bit int.
    local d0 = a0*r0 + a1*s4 + a2*s3 + a3*s2 + a4*s1
    local d1 = a0*r1 + a1*r0 + a2*s4 + a3*s3 + a4*s2
    local d2 = a0*r2 + a1*r1 + a2*r0 + a3*s4 + a4*s3
    local d3 = a0*r3 + a1*r2 + a2*r1 + a3*r0 + a4*s4
    local d4 = a0*r4 + a1*r3 + a2*r2 + a3*r1 + a4*r0

    -- Carry propagation and masking.
    local c
    c = d0 >> LIMB26; d0 = d0 & MASK26; d1 = d1 + c
    c = d1 >> LIMB26; d1 = d1 & MASK26; d2 = d2 + c
    c = d2 >> LIMB26; d2 = d2 & MASK26; d3 = d3 + c
    c = d3 >> LIMB26; d3 = d3 & MASK26; d4 = d4 + c

    -- d4 may have bits above 26. Those represent 2^130+, so fold with *5.
    c = d4 >> LIMB26; d4 = d4 & MASK26
    d0 = d0 + c * 5
    c = d0 >> LIMB26; d0 = d0 & MASK26; d1 = d1 + c

    return {d0, d1, d2, d3, d4}
end

--- Clamp the r value per RFC 8439 Section 2.5.
--
-- Clamping clears certain bits to ensure the multiplier has particular
-- algebraic properties. Specifically (0-indexed bytes):
--   r[3],r[7],r[11],r[15] &= 0x0f   (clear top 4 bits)
--   r[4],r[8],r[12]       &= 0xfc   (clear bottom 2 bits)
--
-- In 1-indexed Lua bytes:
--   bytes[4],bytes[8],bytes[12],bytes[16] &= 0x0f
--   bytes[5],bytes[9],bytes[13]           &= 0xfc
local function clamp_r(r_bytes)
    local bytes = {string.byte(r_bytes, 1, 16)}
    bytes[4]  = bytes[4]  & 0x0F
    bytes[5]  = bytes[5]  & 0xFC
    bytes[8]  = bytes[8]  & 0x0F
    bytes[9]  = bytes[9]  & 0xFC
    bytes[12] = bytes[12] & 0x0F
    bytes[13] = bytes[13] & 0xFC
    bytes[16] = bytes[16] & 0x0F
    local out = {}
    for i = 1, 16 do out[i] = string.char(bytes[i]) end
    return table.concat(out)
end

--- Compute the Poly1305 MAC of a message.
-- @param message  Byte string to authenticate
-- @param key      32-byte one-time key
-- @return         16-byte authentication tag
function M.poly1305_mac(message, key)
    assert(#key == 32, "Poly1305 key must be 32 bytes")

    -- Split key: first 16 bytes = r (clamped), last 16 bytes = s.
    local r_bytes = clamp_r(string.sub(key, 1, 16))
    local s_bytes = string.sub(key, 17, 32)

    local r = bytes_to_5limbs(r_bytes, false)
    local acc = {0, 0, 0, 0, 0}  -- accumulator starts at 0

    local msg_len = #message

    -- Process the message in 16-byte blocks.
    for i = 1, msg_len, 16 do
        local block_end = math.min(i + 15, msg_len)
        local block = string.sub(message, i, block_end)

        -- Convert block to a number and set the hibit (0x01 sentinel).
        -- This 0x01 sentinel ensures that trailing zero bytes in a block
        -- are distinguishable from a shorter message.
        local n = bytes_to_5limbs(block, true)

        -- acc = ((acc + n) * r) mod (2^130 - 5)
        acc = limbs5_add(acc, n)
        acc = limbs5_mul_mod(acc, r)
    end

    -- Final step: tag = (acc + s) mod 2^128.
    -- We need to do a full addition with s as a 128-bit number (no hibit).
    local s = bytes_to_5limbs(s_bytes, false)
    acc = limbs5_add(acc, s)

    -- Final carry propagation.
    for i = 1, 4 do
        acc[i + 1] = acc[i + 1] + (acc[i] >> LIMB26)
        acc[i] = acc[i] & MASK26
    end

    -- The mod 2^128 is implicit in converting to 16 bytes (we just take
    -- the lower 128 bits).
    return limbs5_to_bytes16(acc)
end

-- ============================================================================
-- AEAD: Authenticated Encryption with Associated Data (RFC 8439 Section 2.8)
-- ============================================================================
--
-- The AEAD construction combines ChaCha20 and Poly1305:
--
-- Encryption:
--   1. Derive the Poly1305 one-time key by encrypting 32 zero bytes with
--      ChaCha20 using counter=0. (Only the first 32 bytes are used.)
--   2. Encrypt the plaintext with ChaCha20 using counter=1.
--   3. Construct the MAC input:
--        AAD || pad16(AAD) || ciphertext || pad16(CT) ||
--        le64(len(AAD)) || le64(len(CT))
--      where pad16(x) pads x to a 16-byte boundary with zero bytes.
--   4. Compute the tag = Poly1305(poly_key, mac_input).
--
-- Decryption:
--   1. Derive the Poly1305 key (same as encryption).
--   2. Verify the tag FIRST (before decrypting — this prevents releasing
--      unauthenticated plaintext).
--   3. If the tag matches, decrypt with ChaCha20 using counter=1.
-- ============================================================================

--- Pad data to a 16-byte boundary with zero bytes.
local function pad16(data)
    local remainder = #data % 16
    if remainder == 0 then
        return ""
    end
    return string.rep("\0", 16 - remainder)
end

--- Construct the Poly1305 MAC input per RFC 8439 Section 2.8.
local function build_mac_data(aad, ciphertext)
    return aad .. pad16(aad)
        .. ciphertext .. pad16(ciphertext)
        .. to_le64(#aad) .. to_le64(#ciphertext)
end

--- Constant-time comparison of two byte strings.
-- This prevents timing side-channel attacks where an attacker measures how
-- long the comparison takes to determine how many leading bytes match.
local function constant_time_equal(a, b)
    if #a ~= #b then return false end
    local diff = 0
    for i = 1, #a do
        diff = diff | (string.byte(a, i) ~ string.byte(b, i))
    end
    return diff == 0
end

--- Encrypt and authenticate data.
-- @param plaintext  Data to encrypt
-- @param key        32-byte key
-- @param nonce      12-byte nonce (must never be reused with the same key!)
-- @param aad        Associated data (authenticated but not encrypted)
-- @return ciphertext, tag  (tag is 16 bytes)
function M.aead_encrypt(plaintext, key, nonce, aad)
    assert(#key == 32, "Key must be 32 bytes")
    assert(#nonce == 12, "Nonce must be 12 bytes")
    aad = aad or ""

    -- Step 1: Generate the Poly1305 one-time key.
    local poly_key = string.sub(
        M.chacha20_encrypt(string.rep("\0", 32), key, nonce, 0),
        1, 32
    )

    -- Step 2: Encrypt the plaintext (counter starts at 1).
    local ciphertext = M.chacha20_encrypt(plaintext, key, nonce, 1)

    -- Step 3: Build the MAC input and compute the tag.
    local mac_data = build_mac_data(aad, ciphertext)
    local tag = M.poly1305_mac(mac_data, poly_key)

    return ciphertext, tag
end

--- Decrypt and verify authenticated data.
-- @param ciphertext  Encrypted data
-- @param key         32-byte key
-- @param nonce       12-byte nonce
-- @param aad         Associated data (must match what was used for encryption)
-- @param tag         16-byte authentication tag
-- @return plaintext on success, or nil + error message on failure
function M.aead_decrypt(ciphertext, key, nonce, aad, tag)
    assert(#key == 32, "Key must be 32 bytes")
    assert(#nonce == 12, "Nonce must be 12 bytes")
    assert(#tag == 16, "Tag must be 16 bytes")
    aad = aad or ""

    -- Step 1: Generate the Poly1305 one-time key.
    local poly_key = string.sub(
        M.chacha20_encrypt(string.rep("\0", 32), key, nonce, 0),
        1, 32
    )

    -- Step 2: Verify the tag BEFORE decrypting.
    local mac_data = build_mac_data(aad, ciphertext)
    local expected_tag = M.poly1305_mac(mac_data, poly_key)

    if not constant_time_equal(tag, expected_tag) then
        return nil, "authentication failed"
    end

    -- Step 3: Decrypt (counter starts at 1).
    local plaintext = M.chacha20_encrypt(ciphertext, key, nonce, 1)
    return plaintext
end

return M
