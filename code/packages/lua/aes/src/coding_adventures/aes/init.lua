-- coding_adventures.aes — AES block cipher (FIPS 197)
--
-- AES (Advanced Encryption Standard) is the most widely deployed symmetric
-- cipher in the world. It is a Substitution-Permutation Network (SPN):
-- all 16 bytes of the state are transformed every round, unlike DES's Feistel
-- network which operates on only half the state per round.
--
-- Architecture
-- ─────────────
--   plaintext (16 bytes)
--        │
--   AddRoundKey(state, round_key[0])
--        │
--   ┌── Nr-1 full rounds ─────────────────────────────────────────────┐
--   │   SubBytes  — GF(2^8) inverse + affine transform (non-linear)   │
--   │   ShiftRows — cyclic row shifts (column diffusion)              │
--   │   MixColumns — GF(2^8) matrix multiply (row diffusion)          │
--   │   AddRoundKey                                                    │
--   └─────────────────────────────────────────────────────────────────┘
--        │
--   SubBytes + ShiftRows + AddRoundKey  (final round, no MixColumns)
--        │
--   ciphertext (16 bytes)
--
-- Key sizes: 128 bits (10 rounds), 192 bits (12 rounds), 256 bits (14 rounds)
-- GF(2^8) polynomial: 0x11B = x^8 + x^4 + x^3 + x + 1

local M = {}
M.VERSION = "0.1.0"

-- ─────────────────────────────────────────────────────────────────────────────
-- GF(2^8) Arithmetic with AES polynomial 0x11B
--
-- xtime(b): multiply b by 2 in GF(2^8).
--   = (b << 1) XOR 0x1B if bit 7 was set
--
-- gf_mul(a, b): Russian peasant algorithm — O(8) iterations.
-- ─────────────────────────────────────────────────────────────────────────────

local function xtime(b)
    local shifted = (b << 1) & 0xFF
    if b & 0x80 ~= 0 then
        return shifted ~ 0x1B
    end
    return shifted
end

local function gf_mul(a, b)
    local result = 0
    local aa = a
    local bb = b
    for _ = 1, 8 do
        if bb & 1 ~= 0 then result = result ~ aa end
        aa = xtime(aa)
        bb = bb >> 1
    end
    return result
end

-- ─────────────────────────────────────────────────────────────────────────────
-- AES S-box and Inverse S-box (FIPS 197, Figures 7 and 14)
-- Hardcoded for compile-time availability and O(1) lookup.
-- ─────────────────────────────────────────────────────────────────────────────

-- Forward S-box (SubBytes)
local SBOX = {
    0x63,0x7c,0x77,0x7b,0xf2,0x6b,0x6f,0xc5,0x30,0x01,0x67,0x2b,0xfe,0xd7,0xab,0x76,
    0xca,0x82,0xc9,0x7d,0xfa,0x59,0x47,0xf0,0xad,0xd4,0xa2,0xaf,0x9c,0xa4,0x72,0xc0,
    0xb7,0xfd,0x93,0x26,0x36,0x3f,0xf7,0xcc,0x34,0xa5,0xe5,0xf1,0x71,0xd8,0x31,0x15,
    0x04,0xc7,0x23,0xc3,0x18,0x96,0x05,0x9a,0x07,0x12,0x80,0xe2,0xeb,0x27,0xb2,0x75,
    0x09,0x83,0x2c,0x1a,0x1b,0x6e,0x5a,0xa0,0x52,0x3b,0xd6,0xb3,0x29,0xe3,0x2f,0x84,
    0x53,0xd1,0x00,0xed,0x20,0xfc,0xb1,0x5b,0x6a,0xcb,0xbe,0x39,0x4a,0x4c,0x58,0xcf,
    0xd0,0xef,0xaa,0xfb,0x43,0x4d,0x33,0x85,0x45,0xf9,0x02,0x7f,0x50,0x3c,0x9f,0xa8,
    0x51,0xa3,0x40,0x8f,0x92,0x9d,0x38,0xf5,0xbc,0xb6,0xda,0x21,0x10,0xff,0xf3,0xd2,
    0xcd,0x0c,0x13,0xec,0x5f,0x97,0x44,0x17,0xc4,0xa7,0x7e,0x3d,0x64,0x5d,0x19,0x73,
    0x60,0x81,0x4f,0xdc,0x22,0x2a,0x90,0x88,0x46,0xee,0xb8,0x14,0xde,0x5e,0x0b,0xdb,
    0xe0,0x32,0x3a,0x0a,0x49,0x06,0x24,0x5c,0xc2,0xd3,0xac,0x62,0x91,0x95,0xe4,0x79,
    0xe7,0xc8,0x37,0x6d,0x8d,0xd5,0x4e,0xa9,0x6c,0x56,0xf4,0xea,0x65,0x7a,0xae,0x08,
    0xba,0x78,0x25,0x2e,0x1c,0xa6,0xb4,0xc6,0xe8,0xdd,0x74,0x1f,0x4b,0xbd,0x8b,0x8a,
    0x70,0x3e,0xb5,0x66,0x48,0x03,0xf6,0x0e,0x61,0x35,0x57,0xb9,0x86,0xc1,0x1d,0x9e,
    0xe1,0xf8,0x98,0x11,0x69,0xd9,0x8e,0x94,0x9b,0x1e,0x87,0xe9,0xce,0x55,0x28,0xdf,
    0x8c,0xa1,0x89,0x0d,0xbf,0xe6,0x42,0x68,0x41,0x99,0x2d,0x0f,0xb0,0x54,0xbb,0x16
}

-- Inverse S-box (InvSubBytes)
local INV_SBOX = {
    0x52,0x09,0x6a,0xd5,0x30,0x36,0xa5,0x38,0xbf,0x40,0xa3,0x9e,0x81,0xf3,0xd7,0xfb,
    0x7c,0xe3,0x39,0x82,0x9b,0x2f,0xff,0x87,0x34,0x8e,0x43,0x44,0xc4,0xde,0xe9,0xcb,
    0x54,0x7b,0x94,0x32,0xa6,0xc2,0x23,0x3d,0xee,0x4c,0x95,0x0b,0x42,0xfa,0xc3,0x4e,
    0x08,0x2e,0xa1,0x66,0x28,0xd9,0x24,0xb2,0x76,0x5b,0xa2,0x49,0x6d,0x8b,0xd1,0x25,
    0x72,0xf8,0xf6,0x64,0x86,0x68,0x98,0x16,0xd4,0xa4,0x5c,0xcc,0x5d,0x65,0xb6,0x92,
    0x6c,0x70,0x48,0x50,0xfd,0xed,0xb9,0xda,0x5e,0x15,0x46,0x57,0xa7,0x8d,0x9d,0x84,
    0x90,0xd8,0xab,0x00,0x8c,0xbc,0xd3,0x0a,0xf7,0xe4,0x58,0x05,0xb8,0xb3,0x45,0x06,
    0xd0,0x2c,0x1e,0x8f,0xca,0x3f,0x0f,0x02,0xc1,0xaf,0xbd,0x03,0x01,0x13,0x8a,0x6b,
    0x3a,0x91,0x11,0x41,0x4f,0x67,0xdc,0xea,0x97,0xf2,0xcf,0xce,0xf0,0xb4,0xe6,0x73,
    0x96,0xac,0x74,0x22,0xe7,0xad,0x35,0x85,0xe2,0xf9,0x37,0xe8,0x1c,0x75,0xdf,0x6e,
    0x47,0xf1,0x1a,0x71,0x1d,0x29,0xc5,0x89,0x6f,0xb7,0x62,0x0e,0xaa,0x18,0xbe,0x1b,
    0xfc,0x56,0x3e,0x4b,0xc6,0xd2,0x79,0x20,0x9a,0xdb,0xc0,0xfe,0x78,0xcd,0x5a,0xf4,
    0x1f,0xdd,0xa8,0x33,0x88,0x07,0xc7,0x31,0xb1,0x12,0x10,0x59,0x27,0x80,0xec,0x5f,
    0x60,0x51,0x7f,0xa9,0x19,0xb5,0x4a,0x0d,0x2d,0xe5,0x7a,0x9f,0x93,0xc9,0x9c,0xef,
    0xa0,0xe0,0x3b,0x4d,0xae,0x2a,0xf5,0xb0,0xc8,0xeb,0xbb,0x3c,0x83,0x53,0x99,0x61,
    0x17,0x2b,0x04,0x7e,0xba,0x77,0xd6,0x26,0xe1,0x69,0x14,0x63,0x55,0x21,0x0c,0x7d
}

-- Round constants: Rcon[i] = 2^(i-1) in GF(2^8), 1-indexed
local RCON = {0x01,0x02,0x04,0x08,0x10,0x20,0x40,0x80,0x1b,0x36,0x6c,0xd8,0xab,0x4d}

-- Make S-box accessible
M.SBOX = SBOX
M.INV_SBOX = INV_SBOX

-- ─────────────────────────────────────────────────────────────────────────────
-- State Helpers
-- ─────────────────────────────────────────────────────────────────────────────

-- Convert 16-byte string to 4×4 state (column-major: state[row][col] = block[row + 4*col + 1])
-- We use 1-indexed row/col (1..4)
local function bytes_to_state(block)
    assert(#block == 16, "AES block must be 16 bytes, got " .. #block)
    local state = {}
    for row = 1, 4 do
        state[row] = {}
        for col = 1, 4 do
            state[row][col] = block:byte(row + 4 * (col - 1))
        end
    end
    return state
end

-- Convert 4×4 state back to 16-byte string
local function state_to_bytes(state)
    local bytes = {}
    for col = 1, 4 do
        for row = 1, 4 do
            bytes[#bytes + 1] = string.char(state[row][col])
        end
    end
    return table.concat(bytes)
end

-- Deep copy a 4×4 state
local function copy_state(state)
    local s = {}
    for r = 1, 4 do
        s[r] = {state[r][1], state[r][2], state[r][3], state[r][4]}
    end
    return s
end

-- ─────────────────────────────────────────────────────────────────────────────
-- Key Schedule: expand_key
-- ─────────────────────────────────────────────────────────────────────────────

--- Expand a 16, 24, or 32-byte key into (Nr+1) round keys.
-- Returns a table of (Nr+1) states (each 4×4).
function M.expand_key(key)
    local key_len = #key
    assert(key_len == 16 or key_len == 24 or key_len == 32,
        "AES key must be 16, 24, or 32 bytes; got " .. key_len)

    local nk = key_len // 4
    local nr_map = {[4]=10, [6]=12, [8]=14}
    local nr = nr_map[nk]
    local total_words = 4 * (nr + 1)

    -- W[i] is a 4-byte array (1-indexed)
    local W = {}
    for i = 1, nk do
        W[i] = {key:byte(4*(i-1)+1), key:byte(4*(i-1)+2), key:byte(4*(i-1)+3), key:byte(4*(i-1)+4)}
    end

    for i = nk + 1, total_words do
        local temp = {W[i-1][1], W[i-1][2], W[i-1][3], W[i-1][4]}
        if i % nk == 1 then
            -- RotWord
            temp = {temp[2], temp[3], temp[4], temp[1]}
            -- SubWord
            for j = 1, 4 do temp[j] = SBOX[temp[j] + 1] end
            -- XOR Rcon
            temp[1] = temp[1] ~ RCON[(i-1)//nk]
        elseif nk == 8 and i % nk == 5 then
            -- Extra SubWord for AES-256
            for j = 1, 4 do temp[j] = SBOX[temp[j] + 1] end
        end
        local prev = W[i - nk]
        W[i] = {prev[1]~temp[1], prev[2]~temp[2], prev[3]~temp[3], prev[4]~temp[4]}
    end

    -- Pack into (Nr+1) round key states
    local round_keys = {}
    for rk = 0, nr do
        local state = {}
        for row = 1, 4 do state[row] = {} end
        for col = 1, 4 do
            local word = W[4*rk + col]
            for row = 1, 4 do
                state[row][col] = word[row]
            end
        end
        round_keys[rk + 1] = state
    end
    return round_keys
end

-- ─────────────────────────────────────────────────────────────────────────────
-- AES Operations
-- ─────────────────────────────────────────────────────────────────────────────

local function add_round_key(state, rk)
    local s = copy_state(state)
    for r = 1, 4 do
        for c = 1, 4 do
            s[r][c] = s[r][c] ~ rk[r][c]
        end
    end
    return s
end

local function sub_bytes(state)
    local s = {}
    for r = 1, 4 do
        s[r] = {}
        for c = 1, 4 do
            s[r][c] = SBOX[state[r][c] + 1]
        end
    end
    return s
end

local function inv_sub_bytes(state)
    local s = {}
    for r = 1, 4 do
        s[r] = {}
        for c = 1, 4 do
            s[r][c] = INV_SBOX[state[r][c] + 1]
        end
    end
    return s
end

-- ShiftRows: shift row r left by r positions (0-based r → shift by r)
local function shift_rows(state)
    local s = {}
    for r = 1, 4 do
        s[r] = {}
        local shift = r - 1  -- row 1: 0, row 2: 1, row 3: 2, row 4: 3
        for c = 1, 4 do
            s[r][c] = state[r][((c - 1 + shift) % 4) + 1]
        end
    end
    return s
end

-- InvShiftRows: shift row r right by r positions
local function inv_shift_rows(state)
    local s = {}
    for r = 1, 4 do
        s[r] = {}
        local shift = r - 1
        for c = 1, 4 do
            s[r][c] = state[r][((c - 1 - shift + 4) % 4) + 1]
        end
    end
    return s
end

-- MixColumns: multiply each column by the AES matrix in GF(2^8)
-- Matrix: [[2,3,1,1],[1,2,3,1],[1,1,2,3],[3,1,1,2]]
local function mix_column(col)
    local s0, s1, s2, s3 = col[1], col[2], col[3], col[4]
    return {
        xtime(s0) ~ (xtime(s1)~s1) ~ s2 ~ s3,
        s0 ~ xtime(s1) ~ (xtime(s2)~s2) ~ s3,
        s0 ~ s1 ~ xtime(s2) ~ (xtime(s3)~s3),
        (xtime(s0)~s0) ~ s1 ~ s2 ~ xtime(s3)
    }
end

local function mix_columns(state)
    local s = {}
    for r = 1, 4 do s[r] = {} end
    for c = 1, 4 do
        local col = {state[1][c], state[2][c], state[3][c], state[4][c]}
        local mixed = mix_column(col)
        for r = 1, 4 do s[r][c] = mixed[r] end
    end
    return s
end

-- InvMixColumns: multiply by inverse matrix [14,11,13,9; 9,14,11,13; 13,9,14,11; 11,13,9,14]
local function inv_mix_column(col)
    local s0, s1, s2, s3 = col[1], col[2], col[3], col[4]
    return {
        gf_mul(0x0e,s0)~gf_mul(0x0b,s1)~gf_mul(0x0d,s2)~gf_mul(0x09,s3),
        gf_mul(0x09,s0)~gf_mul(0x0e,s1)~gf_mul(0x0b,s2)~gf_mul(0x0d,s3),
        gf_mul(0x0d,s0)~gf_mul(0x09,s1)~gf_mul(0x0e,s2)~gf_mul(0x0b,s3),
        gf_mul(0x0b,s0)~gf_mul(0x0d,s1)~gf_mul(0x09,s2)~gf_mul(0x0e,s3),
    }
end

local function inv_mix_columns(state)
    local s = {}
    for r = 1, 4 do s[r] = {} end
    for c = 1, 4 do
        local col = {state[1][c], state[2][c], state[3][c], state[4][c]}
        local mixed = inv_mix_column(col)
        for r = 1, 4 do s[r][c] = mixed[r] end
    end
    return s
end

-- ─────────────────────────────────────────────────────────────────────────────
-- Core Block Cipher
-- ─────────────────────────────────────────────────────────────────────────────

--- Encrypt a single 16-byte block with AES.
-- @param block  16-byte string (plaintext)
-- @param key    16, 24, or 32-byte string
-- @return       16-byte string (ciphertext)
function M.aes_encrypt_block(block, key)
    assert(#block == 16, "AES block must be 16 bytes, got " .. #block)
    local round_keys = M.expand_key(key)
    local nr = #round_keys - 1

    local state = bytes_to_state(block)
    state = add_round_key(state, round_keys[1])

    for rnd = 2, nr do
        state = sub_bytes(state)
        state = shift_rows(state)
        state = mix_columns(state)
        state = add_round_key(state, round_keys[rnd])
    end

    -- Final round: no MixColumns
    state = sub_bytes(state)
    state = shift_rows(state)
    state = add_round_key(state, round_keys[nr + 1])

    return state_to_bytes(state)
end

--- Decrypt a single 16-byte block with AES.
-- @param block  16-byte string (ciphertext)
-- @param key    16, 24, or 32-byte string
-- @return       16-byte string (plaintext)
function M.aes_decrypt_block(block, key)
    assert(#block == 16, "AES block must be 16 bytes, got " .. #block)
    local round_keys = M.expand_key(key)
    local nr = #round_keys - 1

    local state = bytes_to_state(block)
    state = add_round_key(state, round_keys[nr + 1])

    for rnd = nr, 2, -1 do
        state = inv_shift_rows(state)
        state = inv_sub_bytes(state)
        state = add_round_key(state, round_keys[rnd])
        state = inv_mix_columns(state)
    end

    -- Final inverse round
    state = inv_shift_rows(state)
    state = inv_sub_bytes(state)
    state = add_round_key(state, round_keys[1])

    return state_to_bytes(state)
end

return M
