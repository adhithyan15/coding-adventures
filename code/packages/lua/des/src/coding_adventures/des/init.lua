-- coding_adventures.des — DES and 3DES block cipher (FIPS 46-3 / SP 800-67)
--
-- DES (Data Encryption Standard) was standardized by NIST in 1977. It is now
-- cryptographically broken (56-bit key space) but remains essential study:
--
--   1. Feistel networks — encryption and decryption share the same circuit
--      (just reverse the subkey order). The round function f never inverted.
--
--   2. S-boxes — the only non-linear step. Without them, DES is a linear
--      function solvable by Gaussian elimination over GF(2).
--
--   3. Key schedule — a single 56-bit key expands into 16 × 48-bit subkeys.
--
-- Architecture
-- ─────────────
--   plaintext (8 bytes / 64 bits)
--        │
--   IP (initial permutation)
--        │
--   ┌── 16 Feistel rounds ─────────────────────────────────────────────┐
--   │   L_i = R_{i-1}                                                   │
--   │   R_i = L_{i-1} XOR f(R_{i-1}, K_i)                             │
--   │   f: E(R) → XOR K → S-boxes → P                                  │
--   └───────────────────────────────────────────────────────────────────┘
--        │
--   FP = IP⁻¹
--        │
--   ciphertext (8 bytes)
--
-- Decryption = encryption with subkeys in reverse order.

local M = {}
M.VERSION = "0.1.0"

-- ─────────────────────────────────────────────────────────────────────────────
-- Permutation and Selection Tables (all values 1-indexed, matching the FIPS 46
-- standard; we subtract 1 at use time to convert to 0-based Lua bit indices)
-- ─────────────────────────────────────────────────────────────────────────────

-- IP — Initial Permutation (64 → 64 bits)
local IP = {
    58,50,42,34,26,18,10,2, 60,52,44,36,28,20,12,4,
    62,54,46,38,30,22,14,6, 64,56,48,40,32,24,16,8,
    57,49,41,33,25,17, 9,1, 59,51,43,35,27,19,11,3,
    61,53,45,37,29,21,13,5, 63,55,47,39,31,23,15,7
}

-- FP — Final Permutation = IP⁻¹
local FP = {
    40, 8,48,16,56,24,64,32, 39, 7,47,15,55,23,63,31,
    38, 6,46,14,54,22,62,30, 37, 5,45,13,53,21,61,29,
    36, 4,44,12,52,20,60,28, 35, 3,43,11,51,19,59,27,
    34, 2,42,10,50,18,58,26, 33, 1,41, 9,49,17,57,25
}

-- PC-1 — Permuted Choice 1 (64 → 56 bits, drops parity bits)
local PC1 = {
    57,49,41,33,25,17, 9,  1,58,50,42,34,26,18,
    10, 2,59,51,43,35,27, 19,11, 3,60,52,44,36,
    63,55,47,39,31,23,15,  7,62,54,46,38,30,22,
    14, 6,61,53,45,37,29, 21,13, 5,28,20,12, 4
}

-- PC-2 — Permuted Choice 2 (56 → 48 bits, subkey selection)
local PC2 = {
    14,17,11,24, 1, 5,  3,28,15, 6,21,10,
    23,19,12, 4,26, 8, 16, 7,27,20,13, 2,
    41,52,31,37,47,55, 30,40,51,45,33,48,
    44,49,39,56,34,53, 46,42,50,36,29,32
}

-- E — Expansion (32 → 48 bits, border bits shared between adjacent groups)
local E = {
    32, 1, 2, 3, 4, 5,  4, 5, 6, 7, 8, 9,
     8, 9,10,11,12,13, 12,13,14,15,16,17,
    16,17,18,19,20,21, 20,21,22,23,24,25,
    24,25,26,27,28,29, 28,29,30,31,32, 1
}

-- P — Post-S-box permutation (32 → 32 bits, diffusion)
local P = {
    16, 7,20,21,29,12,28,17,
     1,15,23,26, 5,18,31,10,
     2, 8,24,14,32,27, 3, 9,
    19,13,30, 6,22,11, 4,25
}

-- SHIFTS — Key schedule left-rotation amounts per round
local SHIFTS = {1,1,2,2,2,2,2,2,1,2,2,2,2,2,2,1}

-- ─────────────────────────────────────────────────────────────────────────────
-- S-Boxes (8 boxes, each 4 rows × 16 cols → 4-bit output)
-- Flat 1D arrays: index = row*16 + col + 1 (1-based Lua indexing)
-- ─────────────────────────────────────────────────────────────────────────────

local SBOXES = {
    -- S1
    {14, 4,13, 1, 2,15,11, 8, 3,10, 6,12, 5, 9, 0, 7,
      0,15, 7, 4,14, 2,13, 1,10, 6,12,11, 9, 5, 3, 8,
      4, 1,14, 8,13, 6, 2,11,15,12, 9, 7, 3,10, 5, 0,
     15,12, 8, 2, 4, 9, 1, 7, 5,11, 3,14,10, 0, 6,13},
    -- S2
    {15, 1, 8,14, 6,11, 3, 4, 9, 7, 2,13,12, 0, 5,10,
      3,13, 4, 7,15, 2, 8,14,12, 0, 1,10, 6, 9,11, 5,
      0,14, 7,11,10, 4,13, 1, 5, 8,12, 6, 9, 3, 2,15,
     13, 8,10, 1, 3,15, 4, 2,11, 6, 7,12, 0, 5,14, 9},
    -- S3
    {10, 0, 9,14, 6, 3,15, 5, 1,13,12, 7,11, 4, 2, 8,
     13, 7, 0, 9, 3, 4, 6,10, 2, 8, 5,14,12,11,15, 1,
     13, 6, 4, 9, 8,15, 3, 0,11, 1, 2,12, 5,10,14, 7,
      1,10,13, 0, 6, 9, 8, 7, 4,15,14, 3,11, 5, 2,12},
    -- S4
    { 7,13,14, 3, 0, 6, 9,10, 1, 2, 8, 5,11,12, 4,15,
     13, 8,11, 5, 6,15, 0, 3, 4, 7, 2,12, 1,10,14, 9,
     10, 6, 9, 0,12,11, 7,13,15, 1, 3,14, 5, 2, 8, 4,
      3,15, 0, 6,10, 1,13, 8, 9, 4, 5,11,12, 7, 2,14},
    -- S5
    { 2,12, 4, 1, 7,10,11, 6, 8, 5, 3,15,13, 0,14, 9,
     14,11, 2,12, 4, 7,13, 1, 5, 0,15,10, 3, 9, 8, 6,
      4, 2, 1,11,10,13, 7, 8,15, 9,12, 5, 6, 3, 0,14,
     11, 8,12, 7, 1,14, 2,13, 6,15, 0, 9,10, 4, 5, 3},
    -- S6
    {12, 1,10,15, 9, 2, 6, 8, 0,13, 3, 4,14, 7, 5,11,
     10,15, 4, 2, 7,12, 9, 5, 6, 1,13,14, 0,11, 3, 8,
      9,14,15, 5, 2, 8,12, 3, 7, 0, 4,10, 1,13,11, 6,
      4, 3, 2,12, 9, 5,15,10,11,14, 1, 7, 6, 0, 8,13},
    -- S7
    { 4,11, 2,14,15, 0, 8,13, 3,12, 9, 7, 5,10, 6, 1,
     13, 0,11, 7, 4, 9, 1,10,14, 3, 5,12, 2,15, 8, 6,
      1, 4,11,13,12, 3, 7,14,10,15, 6, 8, 0, 5, 9, 2,
      6,11,13, 8, 1, 4,10, 7, 9, 5, 0,15,14, 2, 3,12},
    -- S8
    {13, 2, 8, 4, 6,15,11, 1,10, 9, 3,14, 5, 0,12, 7,
      1,15,13, 8,10, 3, 7, 4,12, 5, 6,11, 0,14, 9, 2,
      7,11, 4, 1, 9,12,14, 2, 0, 6,10,13,15, 3, 5, 8,
      2, 1,14, 7, 4,10, 8,13,15,12, 9, 0, 3, 5, 6,11}
}

-- ─────────────────────────────────────────────────────────────────────────────
-- Bit Manipulation Helpers
-- ─────────────────────────────────────────────────────────────────────────────

-- Convert an 8-byte string to a table of 64 bits (MSB first within each byte)
local function bytes_to_bits(s)
    local bits = {}
    for i = 1, #s do
        local byte = s:byte(i)
        for b = 7, 0, -1 do
            bits[#bits + 1] = (byte >> b) & 1
        end
    end
    return bits
end

-- Convert a table of bits (MSB first) back to a byte string
local function bits_to_bytes(bits)
    local result = {}
    for i = 1, #bits, 8 do
        local byte = 0
        for j = 0, 7 do
            byte = byte * 2 + (bits[i + j] or 0)
        end
        result[#result + 1] = string.char(byte)
    end
    return table.concat(result)
end

-- Apply a permutation table (1-indexed positions) to a bit table
local function permute(bits, tbl)
    local out = {}
    for i = 1, #tbl do
        out[i] = bits[tbl[i]]
    end
    return out
end

-- Left-rotate a 28-element table (key half) by n positions
local function left_rotate(half, n)
    local out = {}
    for i = 1, 28 do
        out[i] = half[((i - 1 + n) % 28) + 1]
    end
    return out
end

-- ─────────────────────────────────────────────────────────────────────────────
-- Key Schedule: expand_key
-- ─────────────────────────────────────────────────────────────────────────────

--- Derive the 16 DES round subkeys from an 8-byte key string.
-- Returns a table of 16 strings, each 6 bytes (48 bits).
function M.expand_key(key)
    assert(#key == 8, "DES key must be exactly 8 bytes, got " .. #key)

    local key_bits = bytes_to_bits(key)
    local permuted = permute(key_bits, PC1)  -- 64 → 56 bits

    local c = {}
    local d = {}
    for i = 1, 28 do c[i] = permuted[i] end
    for i = 1, 28 do d[i] = permuted[28 + i] end

    local subkeys = {}
    for _, shift in ipairs(SHIFTS) do
        c = left_rotate(c, shift)
        d = left_rotate(d, shift)
        -- Concatenate c and d, then apply PC-2 (56 → 48 bits)
        local cd = {}
        for i = 1, 28 do cd[i] = c[i] end
        for i = 1, 28 do cd[28 + i] = d[i] end
        local sk_bits = permute(cd, PC2)
        subkeys[#subkeys + 1] = bits_to_bytes(sk_bits)
    end
    return subkeys
end

-- ─────────────────────────────────────────────────────────────────────────────
-- Round Function f(R, K)
-- ─────────────────────────────────────────────────────────────────────────────

local function feistel_f(right_bits, subkey)
    -- Step 1: Expand R from 32 → 48 bits
    local expanded = permute(right_bits, E)

    -- Step 2: XOR with 48-bit subkey
    local sk_bits = bytes_to_bits(subkey)
    local xored = {}
    for i = 1, 48 do
        xored[i] = expanded[i] ~ sk_bits[i]
    end

    -- Step 3: Apply 8 S-boxes (6 bits → 4 bits each)
    local sbox_out = {}
    for box = 0, 7 do
        local offset = box * 6
        local chunk = {xored[offset+1], xored[offset+2], xored[offset+3],
                       xored[offset+4], xored[offset+5], xored[offset+6]}
        -- Row = outer bits (b[1] and b[6]); Col = inner bits (b[2]..b[5])
        local row = chunk[1] * 2 + chunk[6]
        local col = chunk[2] * 8 + chunk[3] * 4 + chunk[4] * 2 + chunk[5]
        local val = SBOXES[box + 1][row * 16 + col + 1]
        -- Convert 4-bit value to bits (MSB first)
        local base = box * 4
        sbox_out[base + 1] = (val >> 3) & 1
        sbox_out[base + 2] = (val >> 2) & 1
        sbox_out[base + 3] = (val >> 1) & 1
        sbox_out[base + 4] = val & 1
    end

    -- Step 4: P permutation (32 → 32 bits, diffusion)
    return permute(sbox_out, P)
end

-- ─────────────────────────────────────────────────────────────────────────────
-- Core Block Cipher
-- ─────────────────────────────────────────────────────────────────────────────

-- Encrypt or decrypt a single 8-byte block with the given subkey list.
-- Encryption: subkeys in order (1..16)
-- Decryption: subkeys in reverse (16..1)
local function des_block(block, subkeys)
    assert(#block == 8, "DES block must be exactly 8 bytes, got " .. #block)

    local bits = bytes_to_bits(block)
    bits = permute(bits, IP)

    local left  = {}
    local right = {}
    for i = 1, 32 do left[i]  = bits[i]      end
    for i = 1, 32 do right[i] = bits[32 + i] end

    -- 16 Feistel rounds
    for _, sk in ipairs(subkeys) do
        local f_out = feistel_f(right, sk)
        local new_right = {}
        for i = 1, 32 do
            new_right[i] = left[i] ~ f_out[i]
        end
        left  = right
        right = new_right
    end

    -- Swap halves before final permutation
    local combined = {}
    for i = 1, 32 do combined[i]      = right[i] end
    for i = 1, 32 do combined[32 + i] = left[i]  end

    return bits_to_bytes(permute(combined, FP))
end

-- ─────────────────────────────────────────────────────────────────────────────
-- Public API
-- ─────────────────────────────────────────────────────────────────────────────

--- Encrypt a single 8-byte block with DES.
-- @param block  8-byte string (plaintext)
-- @param key    8-byte string (64-bit DES key; bits at positions 8,16,...,64 are parity)
-- @return       8-byte string (ciphertext)
function M.des_encrypt_block(block, key)
    local subkeys = M.expand_key(key)
    return des_block(block, subkeys)
end

--- Decrypt a single 8-byte block with DES.
-- @param block  8-byte string (ciphertext)
-- @param key    8-byte string (same key used for encryption)
-- @return       8-byte string (plaintext)
function M.des_decrypt_block(block, key)
    local subkeys = M.expand_key(key)
    -- Reverse subkeys for decryption (Feistel property)
    local rev = {}
    for i = 16, 1, -1 do rev[#rev + 1] = subkeys[i] end
    return des_block(block, rev)
end

-- PKCS#7 padding: append N bytes each with value N (1 ≤ N ≤ 8)
local function pkcs7_pad(data)
    local pad_len = 8 - (#data % 8)
    return data .. string.rep(string.char(pad_len), pad_len)
end

-- Remove PKCS#7 padding; errors if invalid
local function pkcs7_unpad(data)
    assert(#data > 0, "Cannot unpad empty data")
    local pad_len = data:byte(#data)
    assert(pad_len >= 1 and pad_len <= 8, "Invalid PKCS#7 padding byte: " .. pad_len)
    assert(#data >= pad_len, "Padding length exceeds data length")
    for i = #data - pad_len + 1, #data do
        assert(data:byte(i) == pad_len, "Invalid PKCS#7 padding bytes")
    end
    return data:sub(1, #data - pad_len)
end

--- Encrypt variable-length plaintext with DES in ECB mode (PKCS#7 padding).
-- WARNING: ECB mode is insecure for most purposes (identical blocks produce
-- identical ciphertext). Educational use only.
-- @param plaintext  any-length byte string
-- @param key        8-byte DES key
-- @return           ciphertext (multiple of 8 bytes)
function M.des_ecb_encrypt(plaintext, key)
    local subkeys = M.expand_key(key)
    local padded = pkcs7_pad(plaintext)
    local result = {}
    for i = 1, #padded, 8 do
        result[#result + 1] = des_block(padded:sub(i, i + 7), subkeys)
    end
    return table.concat(result)
end

--- Decrypt variable-length ciphertext with DES in ECB mode.
-- @param ciphertext  byte string (must be a multiple of 8 bytes)
-- @param key         8-byte DES key
-- @return            plaintext with padding removed
function M.des_ecb_decrypt(ciphertext, key)
    assert(#ciphertext > 0, "Cannot decrypt empty ciphertext")
    assert(#ciphertext % 8 == 0,
        "DES ECB ciphertext must be a multiple of 8 bytes, got " .. #ciphertext)
    local subkeys = M.expand_key(key)
    local rev = {}
    for i = 16, 1, -1 do rev[#rev + 1] = subkeys[i] end
    local result = {}
    for i = 1, #ciphertext, 8 do
        result[#result + 1] = des_block(ciphertext:sub(i, i + 7), rev)
    end
    return pkcs7_unpad(table.concat(result))
end

-- ─────────────────────────────────────────────────────────────────────────────
-- 3DES / TDEA
--
-- Triple DES applies DES three times with EDE (Encrypt-Decrypt-Encrypt) ordering:
--   Encrypt: C = E_K1( D_K2( E_K3(P) ) )
--   Decrypt: P = D_K3( E_K2( D_K1(C) ) )
--
-- When K1=K2=K3, reduces to single DES (backward compatibility).
-- ─────────────────────────────────────────────────────────────────────────────

--- Triple DES (TDEA) encrypt: E_K1(D_K2(E_K3(block)))
function M.tdea_encrypt_block(block, k1, k2, k3)
    local sk1 = M.expand_key(k1)
    local sk2 = M.expand_key(k2)
    local sk3 = M.expand_key(k3)
    -- E_K3
    local t = des_block(block, sk3)
    -- D_K2 (reverse subkeys)
    local rev2 = {}
    for i = 16, 1, -1 do rev2[#rev2 + 1] = sk2[i] end
    t = des_block(t, rev2)
    -- E_K1
    return des_block(t, sk1)
end

--- Triple DES (TDEA) decrypt: D_K3(E_K2(D_K1(block)))
function M.tdea_decrypt_block(block, k1, k2, k3)
    local sk1 = M.expand_key(k1)
    local sk2 = M.expand_key(k2)
    local sk3 = M.expand_key(k3)
    -- D_K1 (reverse subkeys)
    local rev1 = {}
    for i = 16, 1, -1 do rev1[#rev1 + 1] = sk1[i] end
    local t = des_block(block, rev1)
    -- E_K2
    t = des_block(t, sk2)
    -- D_K3 (reverse subkeys)
    local rev3 = {}
    for i = 16, 1, -1 do rev3[#rev3 + 1] = sk3[i] end
    return des_block(t, rev3)
end

return M
