-- hmac — HMAC (Hash-based Message Authentication Code)
-- RFC 2104 / FIPS 198-1, implemented from scratch in Lua.
--
-- What Is HMAC?
-- =============
-- HMAC takes a secret key and a message and produces a fixed-size
-- authentication tag that proves two things:
--
--   1. Integrity   — the message has not been altered.
--   2. Authenticity — the sender knows the secret key.
--
-- Unlike a plain hash, an HMAC tag cannot be forged without the key.
-- HMAC is used in TLS 1.2/1.3, JWT, WPA2, TOTP/HOTP, and AWS Signature V4.
--
-- Why Not hash(key .. message)?
-- ==============================
-- Naively prepending the key is vulnerable to the **length extension attack**
-- on Merkle-Damgård hashes (MD5, SHA-1, SHA-256, SHA-512).
--
-- A Merkle-Damgård digest equals the hash function's internal state after
-- processing the last block. An attacker who knows hash(key .. msg) knows
-- that state and can resume hashing — appending arbitrary bytes — without
-- ever knowing `key`.
--
-- HMAC defeats this with two nested hash calls under different derived keys:
--
--   HMAC(K, M) = H((K' XOR opad) .. H((K' XOR ipad) .. M))
--
-- The outer hash treats the inner result as a fresh message, so an attacker
-- cannot resume it without knowing K' XOR opad (which requires knowing K).
--
-- The ipad and opad Constants
-- ============================
--   ipad = 0x36 = 0011_0110  (inner pad, XOR'd with K' for inner hash key)
--   opad = 0x5C = 0101_1100  (outer pad, XOR'd with K' for outer hash key)
--
-- They differ in 4 of 8 bits — the maximum Hamming distance for single-byte
-- values XOR'd with the same source — ensuring inner_key and outer_key are
-- as different as possible despite sharing the same K'.
--
-- The Algorithm (RFC 2104 §2)
-- ============================
--   1. Normalize K to block_size bytes:
--        len(K) > block_size → K' = H(K), zero-pad to block_size
--        len(K) ≤ block_size → zero-pad to block_size
--   2. inner_key = K' XOR (0x36 × block_size)
--   3. outer_key = K' XOR (0x5C × block_size)
--   4. inner     = H(bytes_to_str(inner_key) .. message)
--   5. return      H(bytes_to_str(outer_key) .. bytes_to_str(inner))
--
-- Block Sizes
-- ===========
--   MD5 / SHA-1 / SHA-256: 64-byte blocks
--   SHA-512:               128-byte blocks (64-bit words, 1024-bit schedule)
--
-- Note on Lua Byte Representation
-- =================================
-- The hash functions in this monorepo take a Lua `string` and return a table
-- of integers (0–255). HMAC must concatenate key-material with messages, so
-- we convert the byte-table output back to a binary string for re-input.
-- This uses `string.char(...)` on each table element.
--
-- RFC 4231 Test Vector TC1 (HMAC-SHA256)
-- ========================================
--   key = string.rep("\x0b", 20)
--   msg = "Hi There"
--   tag = "b0344c61d8db38535ca8afceaf0bf12b881dc200c9833da726e9376c2e32cff7"

local sha256_m = require("coding_adventures.sha256")
local sha512_m = require("coding_adventures.sha512")
local md5_m    = require("coding_adventures.md5")
local sha1_m   = require("coding_adventures.sha1")

local M = {}

-- ─── ipad / opad ─────────────────────────────────────────────────────────────
local IPAD = 0x36
local OPAD = 0x5C

-- ─── Private helpers ──────────────────────────────────────────────────────────

-- Convert a table of byte integers to a Lua binary string.
-- Used to pass byte-table hash output back into hash functions as input.
local function bytes_to_str(t)
    local chars = {}
    for i = 1, #t do
        chars[i] = string.char(t[i])
    end
    return table.concat(chars)
end

-- Convert a Lua string to a table of byte integers.
-- Used to split a key string into bytes for XOR operations.
local function str_to_bytes(s)
    local t = {}
    for i = 1, #s do
        t[i] = string.byte(s, i)
    end
    return t
end

-- Normalize key to exactly block_size bytes (as a Lua string).
--   - If len(key) > block_size: key = hash_fn(key) (returns byte table → str)
--   - Zero-pad or truncate to exactly block_size bytes.
local function normalize_key(hash_fn, block_size, key)
    local effective = key
    if #key > block_size then
        -- Hash the key; hash_fn returns a table, convert to string
        local hashed = hash_fn(key)
        effective = bytes_to_str(hashed)
    end
    -- Zero-pad to block_size
    if #effective < block_size then
        effective = effective .. string.rep("\x00", block_size - #effective)
    end
    return effective
end

-- XOR every byte of a string with a constant fill byte.
-- Returns a new binary string of the same length.
local function xor_fill(s, fill)
    local bytes = str_to_bytes(s)
    for i = 1, #bytes do
        bytes[i] = bytes[i] ~ fill   -- Lua 5.4 bitwise XOR
    end
    return bytes_to_str(bytes)
end

-- ─── Generic HMAC ─────────────────────────────────────────────────────────────

--- Compute HMAC using any hash function.
--
-- @param hash_fn   function(string) -> table-of-bytes
-- @param block_size integer — internal block size in bytes (64 or 128)
-- @param key       string — secret key, binary, any length
-- @param message   string — data to authenticate, binary, any length
-- @return          table-of-bytes — authentication tag
--
-- Example:
--   local sha256 = require("coding_adventures.sha256")
--   local tag = M.hmac(sha256.sha256, 64, string.rep("\x0b", 20), "Hi There")
function M.hmac(hash_fn, block_size, key, message)
    -- Step 1 — normalize key
    local key_prime = normalize_key(hash_fn, block_size, key)

    -- Step 2 — derive padded keys
    local inner_key = xor_fill(key_prime, IPAD)
    local outer_key = xor_fill(key_prime, OPAD)

    -- Step 3 — inner hash: H(inner_key || message)
    local inner_bytes = hash_fn(inner_key .. message)
    local inner_str   = bytes_to_str(inner_bytes)

    -- Step 4 — outer hash: H(outer_key || inner)
    return hash_fn(outer_key .. inner_str)
end

-- ─── Named variants ───────────────────────────────────────────────────────────

--- HMAC-MD5: returns a table of 16 bytes (RFC 2202).
function M.hmac_md5(key, message)
    return M.hmac(md5_m.digest, 64, key, message)
end

--- HMAC-SHA1: returns a table of 20 bytes (RFC 2202).
function M.hmac_sha1(key, message)
    return M.hmac(sha1_m.digest, 64, key, message)
end

--- HMAC-SHA256: returns a table of 32 bytes (RFC 4231).
function M.hmac_sha256(key, message)
    return M.hmac(sha256_m.sha256, 64, key, message)
end

--- HMAC-SHA512: returns a table of 64 bytes (RFC 4231).
-- SHA-512 uses 128-byte blocks (64-bit words), so key normalization and
-- ipad/opad arrays are 128 bytes wide.
function M.hmac_sha512(key, message)
    return M.hmac(sha512_m.digest, 128, key, message)
end

-- ─── Hex-string variants ─────────────────────────────────────────────────────

local function to_hex(bytes)
    local parts = {}
    for i = 1, #bytes do
        parts[i] = string.format("%02x", bytes[i])
    end
    return table.concat(parts)
end

--- HMAC-MD5 as a 32-character lowercase hex string.
function M.hmac_md5_hex(key, message)
    return to_hex(M.hmac_md5(key, message))
end

--- HMAC-SHA1 as a 40-character lowercase hex string.
function M.hmac_sha1_hex(key, message)
    return to_hex(M.hmac_sha1(key, message))
end

--- HMAC-SHA256 as a 64-character lowercase hex string.
function M.hmac_sha256_hex(key, message)
    return to_hex(M.hmac_sha256(key, message))
end

--- HMAC-SHA512 as a 128-character lowercase hex string.
function M.hmac_sha512_hex(key, message)
    return to_hex(M.hmac_sha512(key, message))
end

return M
