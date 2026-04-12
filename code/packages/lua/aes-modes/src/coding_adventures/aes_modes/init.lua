-- coding_adventures.aes_modes — AES Modes of Operation
--
-- AES operates on fixed 128-bit (16-byte) blocks. To encrypt messages longer
-- than one block, you need a "mode of operation" that defines how multiple
-- block cipher calls chain together. The choice of mode critically affects
-- security:
--
--   ECB — Electronic Codebook (INSECURE, educational only)
--   CBC — Cipher Block Chaining (legacy, vulnerable to padding oracles)
--   CTR — Counter mode (modern, stream cipher from block cipher)
--   GCM — Galois/Counter Mode (authenticated encryption, gold standard)
--
-- Why do modes matter?
-- ────────────────────
-- A raw block cipher is a fixed-width permutation: 16 bytes in, 16 bytes out.
-- Real messages are longer. If you encrypt each block independently (ECB),
-- identical plaintext blocks produce identical ciphertext blocks — the famous
-- "ECB penguin" shows image structure leaking through encryption. Modes solve
-- this by introducing state that varies per block: an IV, a counter, or the
-- previous ciphertext block.
--
-- Dependencies: coding_adventures.aes (provides aes_encrypt_block, aes_decrypt_block)

local aes = require("coding_adventures.aes")

local M = {}
M.VERSION = "0.1.0"

-- ─────────────────────────────────────────────────────────────────────────────
-- Utility: XOR two equal-length binary strings
--
-- XOR is the fundamental building block of symmetric cryptography. When you
-- XOR plaintext with a random key of the same length, you get a one-time pad
-- — perfect secrecy. Modes like CTR generate pseudorandom keystream via AES
-- and XOR it with plaintext, approximating a one-time pad.
-- ─────────────────────────────────────────────────────────────────────────────

local function xor_bytes(a, b)
    assert(#a == #b, "xor_bytes: lengths must match (" .. #a .. " vs " .. #b .. ")")
    local result = {}
    for i = 1, #a do
        result[i] = string.char(a:byte(i) ~ b:byte(i))
    end
    return table.concat(result)
end

-- ─────────────────────────────────────────────────────────────────────────────
-- PKCS#7 Padding
--
-- Block ciphers need input that is an exact multiple of the block size (16
-- bytes for AES). PKCS#7 padding appends N bytes, each with value N, where
-- N = 16 - (length mod 16). If the input is already aligned, a full 16-byte
-- padding block is added (so the unpadder always has something to remove).
--
-- Example: "HELLO" (5 bytes) → "HELLO" + 11 bytes of 0x0B
-- Example: 16 bytes           → 16 bytes + 16 bytes of 0x10
--
-- Unpadding reads the last byte to learn N, then verifies and strips N bytes.
-- ─────────────────────────────────────────────────────────────────────────────

function M.pkcs7_pad(data)
    local pad_len = 16 - (#data % 16)
    return data .. string.rep(string.char(pad_len), pad_len)
end

function M.pkcs7_unpad(data)
    assert(#data > 0 and #data % 16 == 0, "pkcs7_unpad: data must be non-empty and multiple of 16")
    local pad_val = data:byte(#data)
    assert(pad_val >= 1 and pad_val <= 16, "Invalid PKCS#7 padding")
    -- Constant-time padding validation: accumulate differences with OR
    -- instead of returning early on the first mismatch (prevents timing attacks)
    local diff = 0
    for i = #data - pad_val + 1, #data do
        diff = diff | (data:byte(i) ~ pad_val)
    end
    assert(diff == 0, "Invalid PKCS#7 padding")
    return data:sub(1, #data - pad_val)
end

-- ─────────────────────────────────────────────────────────────────────────────
-- ECB — Electronic Codebook Mode (INSECURE)
--
-- The simplest mode: encrypt each 16-byte block independently.
--
--   C[i] = AES_encrypt(P[i], key)
--
-- ECB is deterministic: the same plaintext block always produces the same
-- ciphertext block. This leaks patterns. The "ECB penguin" demonstrates this
-- — encrypting a bitmap image in ECB mode preserves the image structure in
-- the ciphertext. NEVER use ECB for real data.
--
-- We include it here for educational comparison with secure modes.
-- ─────────────────────────────────────────────────────────────────────────────

function M.ecb_encrypt(plaintext, key)
    local padded = M.pkcs7_pad(plaintext)
    local blocks = {}
    for i = 1, #padded, 16 do
        blocks[#blocks + 1] = aes.aes_encrypt_block(padded:sub(i, i + 15), key)
    end
    return table.concat(blocks)
end

function M.ecb_decrypt(ciphertext, key)
    assert(#ciphertext > 0 and #ciphertext % 16 == 0,
        "ecb_decrypt: ciphertext must be non-empty multiple of 16 bytes")
    local blocks = {}
    for i = 1, #ciphertext, 16 do
        blocks[#blocks + 1] = aes.aes_decrypt_block(ciphertext:sub(i, i + 15), key)
    end
    return M.pkcs7_unpad(table.concat(blocks))
end

-- ─────────────────────────────────────────────────────────────────────────────
-- CBC — Cipher Block Chaining Mode
--
-- Each plaintext block is XOR'd with the previous ciphertext block before
-- encryption. This means identical plaintext blocks produce different
-- ciphertext (assuming a random IV).
--
--   C[0] = IV
--   C[i] = AES_encrypt(P[i] XOR C[i-1], key)
--
-- Decryption:
--   P[i] = AES_decrypt(C[i], key) XOR C[i-1]
--
-- The IV must be unpredictable (not just unique). CBC is vulnerable to
-- padding oracle attacks (POODLE, Lucky 13) if padding errors are
-- distinguishable from other errors. This is why TLS 1.3 dropped CBC
-- in favor of AEAD modes like GCM.
-- ─────────────────────────────────────────────────────────────────────────────

function M.cbc_encrypt(plaintext, key, iv)
    assert(#iv == 16, "cbc_encrypt: IV must be 16 bytes")
    local padded = M.pkcs7_pad(plaintext)
    local prev = iv
    local blocks = {}
    for i = 1, #padded, 16 do
        local block = padded:sub(i, i + 15)
        local xored = xor_bytes(block, prev)
        local encrypted = aes.aes_encrypt_block(xored, key)
        blocks[#blocks + 1] = encrypted
        prev = encrypted
    end
    return table.concat(blocks)
end

function M.cbc_decrypt(ciphertext, key, iv)
    assert(#iv == 16, "cbc_decrypt: IV must be 16 bytes")
    assert(#ciphertext > 0 and #ciphertext % 16 == 0,
        "cbc_decrypt: ciphertext must be non-empty multiple of 16 bytes")
    local prev = iv
    local blocks = {}
    for i = 1, #ciphertext, 16 do
        local block = ciphertext:sub(i, i + 15)
        local decrypted = aes.aes_decrypt_block(block, key)
        blocks[#blocks + 1] = xor_bytes(decrypted, prev)
        prev = block
    end
    return M.pkcs7_unpad(table.concat(blocks))
end

-- ─────────────────────────────────────────────────────────────────────────────
-- CTR — Counter Mode
--
-- Turns a block cipher into a stream cipher. Instead of encrypting the
-- plaintext directly, we encrypt a counter and XOR the result (keystream)
-- with the plaintext:
--
--   keystream[i] = AES_encrypt(nonce_12 || counter_4_be, key)
--   C[i] = P[i] XOR keystream[i]
--
-- The nonce is 12 bytes; the counter is a 4-byte big-endian integer starting
-- at 1 (matching GCM convention where counter 0 is reserved for the tag).
--
-- Advantages over CBC:
--   - No padding needed (XOR with exact-length keystream)
--   - Parallelizable (each block's keystream is independent)
--   - Random access (can decrypt block i without decrypting 1..i-1)
--
-- CRITICAL: Never reuse (key, nonce) pair. If you encrypt messages M1 and M2
-- with the same nonce: C1 XOR C2 = M1 XOR M2, revealing the XOR of plaintexts.
-- ─────────────────────────────────────────────────────────────────────────────

--- Build a 16-byte counter block: 12-byte nonce || 4-byte big-endian counter.
local function build_counter_block(nonce, counter)
    return nonce .. string.char(
        (counter >> 24) & 0xFF,
        (counter >> 16) & 0xFF,
        (counter >> 8) & 0xFF,
        counter & 0xFF
    )
end

function M.ctr_encrypt(plaintext, key, nonce)
    assert(#nonce == 12, "ctr_encrypt: nonce must be 12 bytes")
    local result = {}
    local counter = 1  -- Start at 1 (GCM reserves counter 0)
    for i = 1, #plaintext, 16 do
        local block = plaintext:sub(i, math.min(i + 15, #plaintext))
        local counter_block = build_counter_block(nonce, counter)
        local keystream = aes.aes_encrypt_block(counter_block, key)
        -- XOR only as many bytes as we have (handles partial last block)
        local encrypted = {}
        for j = 1, #block do
            encrypted[j] = string.char(block:byte(j) ~ keystream:byte(j))
        end
        result[#result + 1] = table.concat(encrypted)
        counter = counter + 1
    end
    return table.concat(result)
end

-- CTR decryption is identical to encryption (XOR is its own inverse)
function M.ctr_decrypt(ciphertext, key, nonce)
    return M.ctr_encrypt(ciphertext, key, nonce)
end

-- ─────────────────────────────────────────────────────────────────────────────
-- GCM — Galois/Counter Mode (Authenticated Encryption)
--
-- GCM combines CTR-mode encryption with a polynomial MAC (GHASH) over
-- GF(2^128). It provides both confidentiality AND integrity — an attacker
-- who modifies the ciphertext will be detected via the authentication tag.
--
-- Algorithm:
--   1. H = AES_encrypt(0^128, key)    — the hash subkey
--   2. J0 = IV || 0x00000001          — initial counter (12-byte IV + counter 1)
--   3. Encrypt with CTR starting at J0+1 (counter = 2, 3, ...)
--   4. GHASH over AAD and ciphertext to produce authentication tag
--   5. Tag = GHASH_result XOR AES_encrypt(J0, key)
--
-- GHASH operates in GF(2^128) with polynomial x^128 + x^7 + x^2 + x + 1.
-- This is a different field from AES's GF(2^8)! The reduction polynomial
-- is 0xE1 << 120 (i.e., 0xE1000...0 in 128-bit representation).
--
-- GF(2^128) Multiplication
-- ────────────────────────
-- For two 128-bit values X and Y:
--   Z = 0, V = Y
--   For each bit of X (MSB first, bit 0 = MSB of first byte):
--     if bit == 1: Z ^= V
--     carry = V & 1   (LSB)
--     V >>= 1
--     if carry: V ^= R  (where R = 0xE1 << 120)
--
-- We represent 128-bit values as two 64-bit integers (hi, lo) since Lua 5.4
-- has 64-bit integers.
-- ─────────────────────────────────────────────────────────────────────────────

-- The GCM reduction polynomial R = 0xE1 << 120, split into (hi, lo):
-- 0xE100000000000000 || 0x0000000000000000
local R_HI = 0xE100000000000000  -- This is a signed integer in Lua but bits are correct

--- Convert a 16-byte string to (hi, lo) pair of 64-bit integers.
local function bytes_to_u128(s)
    local hi = 0
    for i = 1, 8 do
        hi = (hi << 8) | s:byte(i)
    end
    local lo = 0
    for i = 9, 16 do
        lo = (lo << 8) | s:byte(i)
    end
    return hi, lo
end

--- Convert (hi, lo) pair back to 16-byte string.
local function u128_to_bytes(hi, lo)
    local bytes = {}
    for i = 7, 0, -1 do
        bytes[8 - i] = string.char((hi >> (i * 8)) & 0xFF)
    end
    for i = 7, 0, -1 do
        bytes[16 - i] = string.char((lo >> (i * 8)) & 0xFF)
    end
    return table.concat(bytes)
end

--- GF(2^128) multiplication using the bit-by-bit algorithm.
-- X and Y are each (hi, lo) pairs. Returns (z_hi, z_lo).
local function gf128_mul(x_hi, x_lo, y_hi, y_lo)
    local z_hi, z_lo = 0, 0
    local v_hi, v_lo = y_hi, y_lo

    -- Iterate over all 128 bits of X, MSB first.
    -- Bits 0..63 are in x_hi (bit 0 = MSB of x_hi), bits 64..127 in x_lo.
    for i = 0, 127 do
        -- Extract bit i of X (MSB-first ordering)
        local word, bit_pos
        if i < 64 then
            word = x_hi
            bit_pos = 63 - i
        else
            word = x_lo
            bit_pos = 127 - i
        end
        if (word >> bit_pos) & 1 == 1 then
            z_hi = z_hi ~ v_hi
            z_lo = z_lo ~ v_lo
        end

        -- Right-shift V by 1 and conditionally XOR with R
        local carry = v_lo & 1
        -- Logical right shift: Lua 5.4 >> is arithmetic for negative numbers,
        -- but we need logical. Use (v >> 1) & 0x7FFFFFFFFFFFFFFF to clear sign bit.
        v_lo = ((v_lo >> 1) & 0x7FFFFFFFFFFFFFFF) | ((v_hi & 1) << 63)
        v_hi = (v_hi >> 1) & 0x7FFFFFFFFFFFFFFF
        if carry == 1 then
            v_hi = v_hi ~ R_HI
            -- R_LO is 0, no XOR needed for lo
        end
    end
    return z_hi, z_lo
end

--- GHASH: polynomial hash over GF(2^128).
-- Computes: X[0] = 0; X[i] = (X[i-1] XOR data_block[i]) * H
-- where H is the hash subkey and data is zero-padded to 16-byte blocks.
local function ghash(h_hi, h_lo, data)
    local x_hi, x_lo = 0, 0
    -- Process data in 16-byte blocks
    for i = 1, #data, 16 do
        local block = data:sub(i, i + 15)
        if #block < 16 then
            block = block .. string.rep("\0", 16 - #block)
        end
        local b_hi, b_lo = bytes_to_u128(block)
        x_hi = x_hi ~ b_hi
        x_lo = x_lo ~ b_lo
        x_hi, x_lo = gf128_mul(x_hi, x_lo, h_hi, h_lo)
    end
    return x_hi, x_lo
end

--- Pad data to a multiple of 16 bytes with zero bytes.
local function gcm_pad(data)
    local remainder = #data % 16
    if remainder == 0 then return data end
    return data .. string.rep("\0", 16 - remainder)
end

--- Encode a bit length as an 8-byte big-endian integer.
local function encode_length(byte_len)
    local bit_len = byte_len * 8
    local bytes = {}
    for i = 7, 0, -1 do
        bytes[8 - i] = string.char((bit_len >> (i * 8)) & 0xFF)
    end
    return table.concat(bytes)
end

--- GCM encrypt: produces (ciphertext, 16-byte tag).
-- @param plaintext  arbitrary-length string
-- @param key        16, 24, or 32-byte AES key
-- @param iv         12-byte initialization vector
-- @param aad        additional authenticated data (not encrypted, but authenticated)
-- @return ciphertext, tag (both strings)
function M.gcm_encrypt(plaintext, key, iv, aad)
    assert(#iv == 12, "gcm_encrypt: IV must be 12 bytes")
    aad = aad or ""

    -- Step 1: Compute hash subkey H = AES_encrypt(0^128, key)
    local zero_block = string.rep("\0", 16)
    local h_bytes = aes.aes_encrypt_block(zero_block, key)
    local h_hi, h_lo = bytes_to_u128(h_bytes)

    -- Step 2: Initial counter J0 = IV || 0x00000001
    local j0 = iv .. "\0\0\0\1"

    -- Step 3: CTR encrypt starting at J0 + 1 (counter = 2)
    local ciphertext_blocks = {}
    local counter = 2
    for i = 1, #plaintext, 16 do
        local block = plaintext:sub(i, math.min(i + 15, #plaintext))
        local counter_block = build_counter_block(iv, counter)
        local keystream = aes.aes_encrypt_block(counter_block, key)
        local encrypted = {}
        for j = 1, #block do
            encrypted[j] = string.char(block:byte(j) ~ keystream:byte(j))
        end
        ciphertext_blocks[#ciphertext_blocks + 1] = table.concat(encrypted)
        counter = counter + 1
    end
    local ciphertext = table.concat(ciphertext_blocks)

    -- Step 4: Compute GHASH over AAD || pad || CT || pad || len(AAD) || len(CT)
    local ghash_input = gcm_pad(aad) .. gcm_pad(ciphertext)
        .. encode_length(#aad) .. encode_length(#ciphertext)
    local tag_hi, tag_lo = ghash(h_hi, h_lo, ghash_input)

    -- Step 5: Tag = GHASH XOR AES_encrypt(J0, key)
    local j0_enc = aes.aes_encrypt_block(j0, key)
    local j0_hi, j0_lo = bytes_to_u128(j0_enc)
    local final_tag = u128_to_bytes(tag_hi ~ j0_hi, tag_lo ~ j0_lo)

    return ciphertext, final_tag
end

--- GCM decrypt: verifies tag, then decrypts.
-- @param ciphertext  encrypted data
-- @param key         AES key
-- @param iv          12-byte IV
-- @param aad         additional authenticated data
-- @param tag         16-byte authentication tag
-- @return plaintext string, or nil + error message if tag is invalid
function M.gcm_decrypt(ciphertext, key, iv, aad, tag)
    assert(#iv == 12, "gcm_decrypt: IV must be 12 bytes")
    assert(#tag == 16, "gcm_decrypt: tag must be 16 bytes")
    aad = aad or ""

    -- Compute hash subkey
    local zero_block = string.rep("\0", 16)
    local h_bytes = aes.aes_encrypt_block(zero_block, key)
    local h_hi, h_lo = bytes_to_u128(h_bytes)

    -- Compute expected tag
    local j0 = iv .. "\0\0\0\1"
    local ghash_input = gcm_pad(aad) .. gcm_pad(ciphertext)
        .. encode_length(#aad) .. encode_length(#ciphertext)
    local tag_hi, tag_lo = ghash(h_hi, h_lo, ghash_input)

    local j0_enc = aes.aes_encrypt_block(j0, key)
    local j0_hi, j0_lo = bytes_to_u128(j0_enc)
    local expected_tag = u128_to_bytes(tag_hi ~ j0_hi, tag_lo ~ j0_lo)

    -- Constant-time comparison (well, as constant as Lua allows)
    local diff = 0
    for i = 1, 16 do
        diff = diff | (tag:byte(i) ~ expected_tag:byte(i))
    end
    if diff ~= 0 then
        return nil, "gcm_decrypt: authentication tag mismatch"
    end

    -- Decrypt using CTR starting at J0 + 1
    local plaintext_blocks = {}
    local counter = 2
    for i = 1, #ciphertext, 16 do
        local block = ciphertext:sub(i, math.min(i + 15, #ciphertext))
        local counter_block = build_counter_block(iv, counter)
        local keystream = aes.aes_encrypt_block(counter_block, key)
        local decrypted = {}
        for j = 1, #block do
            decrypted[j] = string.char(block:byte(j) ~ keystream:byte(j))
        end
        plaintext_blocks[#plaintext_blocks + 1] = table.concat(decrypted)
        counter = counter + 1
    end

    return table.concat(plaintext_blocks)
end

return M
