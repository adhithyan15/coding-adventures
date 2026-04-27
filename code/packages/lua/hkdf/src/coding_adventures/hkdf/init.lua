-- hkdf — HMAC-based Extract-and-Expand Key Derivation Function
-- RFC 5869, implemented from scratch in Lua.
--
-- What Is HKDF?
-- ==============
-- HKDF (HMAC-based Key Derivation Function) is a simple, well-analyzed KDF
-- built on top of HMAC. It was designed by Hugo Krawczyk and published as
-- RFC 5869 in 2010. HKDF is used in:
--
--   - TLS 1.3 (the primary key derivation mechanism)
--   - Signal Protocol (Double Ratchet key derivation)
--   - WireGuard VPN (handshake key expansion)
--   - Noise Protocol Framework
--   - IKEv2 (Internet Key Exchange)
--
-- Why Do We Need a KDF?
-- =====================
-- Raw cryptographic keys often come from sources with uneven entropy:
--
--   - Diffie-Hellman shared secrets have algebraic structure (not uniform)
--   - Passwords have low entropy concentrated in certain bits
--   - Hardware RNGs may have bias in certain bit positions
--
-- A KDF "extracts" the entropy from such sources into a uniformly random
-- pseudorandom key (PRK), then "expands" that PRK into as many output
-- bytes as needed — each cryptographically independent.
--
-- The Two-Stage Design
-- =====================
-- HKDF separates key derivation into two logically distinct stages:
--
--   1. EXTRACT: Condense input keying material into a fixed-length PRK.
--      This stage is about concentrating entropy.
--
--        PRK = HMAC-Hash(salt, IKM)
--
--      Here `salt` is the HMAC key and `IKM` (Input Keying Material) is the
--      HMAC message. The salt acts as a randomness extractor — even a non-secret
--      salt dramatically improves extraction quality.
--
--      If no salt is provided, RFC 5869 specifies using a string of HashLen
--      zero bytes. This still works because HMAC handles any key, but a
--      random salt provides stronger security guarantees.
--
--   2. EXPAND: Generate arbitrary-length output from the fixed-length PRK.
--      This stage is about stretching a short key into many derived keys.
--
--        T(0) = ""  (empty string)
--        T(1) = HMAC-Hash(PRK, T(0) || info || 0x01)
--        T(2) = HMAC-Hash(PRK, T(1) || info || 0x02)
--        ...
--        T(N) = HMAC-Hash(PRK, T(N-1) || info || 0x0N)
--        OKM = first L bytes of T(1) || T(2) || ... || T(N)
--
--      The `info` parameter provides domain separation — different `info`
--      values produce completely different output keys even from the same PRK.
--      This is how TLS 1.3 derives separate keys for client/server traffic.
--
--      The counter byte is a single octet (1-indexed, max 255), which limits
--      the maximum output length to 255 * HashLen bytes.
--
-- Security Properties
-- ====================
--   - Extract is a strong randomness extractor (under the HMAC-PRF assumption)
--   - Expand is a PRF in the `info` argument (under the same assumption)
--   - The two stages compose cleanly: extract removes source bias, expand
--     stretches the cleaned key
--
-- Hash Function Support
-- ======================
--   SHA-256: HashLen = 32 bytes, block_size = 64 bytes
--   SHA-512: HashLen = 64 bytes, block_size = 128 bytes

local hmac = require("coding_adventures.hmac")

local M = {}

-- ─── Private helpers ──────────────────────────────────────────────────────────

-- Convert a table of byte integers {0x41, 0x42} to a Lua binary string "AB".
-- Our HMAC module returns byte tables; HKDF needs binary strings for
-- concatenation in the expand loop.
local function bytes_to_str(t)
    local chars = {}
    for i = 1, #t do
        chars[i] = string.char(t[i])
    end
    return table.concat(chars)
end

-- Convert a Lua binary string to a lowercase hex string.
-- Used for debugging and hex-output convenience functions.
local function to_hex(s)
    local parts = {}
    for i = 1, #s do
        parts[i] = string.format("%02x", string.byte(s, i))
    end
    return table.concat(parts)
end

-- ─── Hash configuration ──────────────────────────────────────────────────────

-- Each supported hash algorithm needs three things:
--   1. An HMAC function (key, message) -> byte table
--   2. The hash output length (HashLen) for default salt and output limits
--
-- We store these in a lookup table keyed by algorithm name.
local HASH_CONFIG = {
    sha256 = {
        hmac_fn   = hmac.hmac_sha256,
        hash_len  = 32,
    },
    sha512 = {
        hmac_fn   = hmac.hmac_sha512,
        hash_len  = 64,
    },
}

-- Look up hash configuration, raising an error for unsupported algorithms.
local function get_config(hash_name)
    local config = HASH_CONFIG[hash_name]
    if not config then
        error("unsupported hash algorithm: " .. tostring(hash_name), 3)
    end
    return config
end

-- ─── HKDF-Extract (RFC 5869 Section 2.2) ─────────────────────────────────────

--- Extract a pseudorandom key (PRK) from input keying material.
--
-- HKDF-Extract condenses potentially non-uniform input keying material
-- into a fixed-length, uniformly distributed pseudorandom key.
--
-- The extraction uses HMAC with the salt as the key and IKM as the message:
--
--   PRK = HMAC-Hash(salt, IKM)
--
-- This is intentional — the salt acts as a randomness extractor, and HMAC's
-- key-scheduling (normalization, padding) makes extraction work even when
-- the IKM has poor entropy distribution.
--
-- @param salt   string — optional salt value (HMAC key); if empty (""),
--                        uses HashLen zero bytes as specified by RFC 5869
-- @param ikm    string — input keying material (the raw secret)
-- @param hash   string — hash algorithm name: "sha256" or "sha512" (default: "sha256")
-- @return       string — pseudorandom key (PRK), HashLen bytes
--
-- Example:
--   local prk = M.extract("\x00\x01\x02", ikm_bytes, "sha256")
--   -- prk is a 32-byte binary string
function M.extract(salt, ikm, hash)
    hash = hash or "sha256"
    local config = get_config(hash)

    -- RFC 5869 Section 2.2: "if not provided, [salt] is set to a string
    -- of HashLen zeros." An empty salt is treated as "not provided."
    local effective_salt = salt
    if effective_salt == nil or effective_salt == "" then
        effective_salt = string.rep("\x00", config.hash_len)
    end

    -- HMAC returns a byte table; convert to binary string for consistency.
    -- Note: the HMAC key-must-not-be-empty check is satisfied because
    -- effective_salt is always at least HashLen bytes when originally empty.
    local prk_bytes = config.hmac_fn(effective_salt, ikm)
    return bytes_to_str(prk_bytes)
end

-- ─── HKDF-Expand (RFC 5869 Section 2.3) ──────────────────────────────────────

--- Expand a pseudorandom key into output keying material of desired length.
--
-- HKDF-Expand generates arbitrary-length output from a fixed-length PRK
-- using an iterative HMAC construction:
--
--   T(0) = ""                                      (empty string)
--   T(i) = HMAC-Hash(PRK, T(i-1) || info || i)    (i = 1, 2, ..., N)
--   OKM  = first L bytes of T(1) || T(2) || ... || T(N)
--
-- Each T(i) feeds back into the next iteration, creating a chain where
-- every block depends on all previous blocks. The `info` parameter
-- provides domain separation — the same PRK with different `info` values
-- produces independent output keys.
--
-- The counter `i` is a single byte (0x01 through 0xFF), limiting the
-- maximum output to 255 * HashLen bytes.
--
-- @param prk    string — pseudorandom key from extract (>= HashLen bytes)
-- @param info   string — context/application-specific info (can be empty)
-- @param length integer — desired output length in bytes (1..255*HashLen)
-- @param hash   string — hash algorithm: "sha256" or "sha512" (default: "sha256")
-- @return       string — output keying material (OKM), exactly `length` bytes
function M.expand(prk, info, length, hash)
    hash = hash or "sha256"
    local config = get_config(hash)

    -- Validate output length.
    -- RFC 5869: "L <= 255*HashLen" and implicitly L >= 1 (need at least one byte).
    local max_length = 255 * config.hash_len
    if length <= 0 then
        error("HKDF expand length must be > 0", 2)
    end
    if length > max_length then
        error(string.format(
            "HKDF expand length %d exceeds maximum %d (255 * %d)",
            length, max_length, config.hash_len
        ), 2)
    end

    -- Number of HMAC iterations needed: ceil(L / HashLen).
    -- We use integer arithmetic: (L + HashLen - 1) / HashLen, truncated.
    local n = math.ceil(length / config.hash_len)

    -- Iterative expansion loop.
    -- T(0) is the empty string. Each subsequent T(i) chains the previous
    -- output with the info and a 1-byte counter.
    local t_prev = ""      -- T(0) = empty
    local okm_parts = {}   -- accumulate T(1), T(2), ..., T(N)

    for i = 1, n do
        -- T(i) = HMAC-Hash(PRK, T(i-1) || info || byte(i))
        -- The counter is a single byte, so i must be in [1, 255].
        local message = t_prev .. info .. string.char(i)
        local t_bytes = config.hmac_fn(prk, message)
        t_prev = bytes_to_str(t_bytes)
        okm_parts[#okm_parts + 1] = t_prev
    end

    -- Concatenate all T blocks and truncate to exactly `length` bytes.
    local okm = table.concat(okm_parts)
    return okm:sub(1, length)
end

-- ─── HKDF Combined (RFC 5869 Section 2.1) ────────────────────────────────────

--- Derive output keying material from input keying material in one step.
--
-- This is the standard HKDF interface that combines extract and expand:
--
--   OKM = HKDF-Expand(HKDF-Extract(salt, IKM), info, L)
--
-- Use this when you have raw keying material and want derived keys directly.
-- Use the separate extract/expand functions when you need the intermediate
-- PRK (e.g., to derive multiple independent keys from the same extraction).
--
-- @param salt   string — optional salt for extraction (empty = HashLen zeros)
-- @param ikm    string — input keying material
-- @param info   string — context info for expansion (can be empty)
-- @param length integer — desired output length (1..255*HashLen)
-- @param hash   string — "sha256" or "sha512" (default: "sha256")
-- @return       string — output keying material, `length` bytes
--
-- Example (TLS 1.3 style):
--   local traffic_key = M.hkdf(salt, shared_secret, "tls13 key", 32, "sha256")
function M.hkdf(salt, ikm, info, length, hash)
    hash = hash or "sha256"
    local prk = M.extract(salt, ikm, hash)
    return M.expand(prk, info, length, hash)
end

-- ─── Hex convenience functions ───────────────────────────────────────────────

--- HKDF-Extract returning a hex string.
function M.extract_hex(salt, ikm, hash)
    return to_hex(M.extract(salt, ikm, hash))
end

--- HKDF-Expand returning a hex string.
function M.expand_hex(prk, info, length, hash)
    return to_hex(M.expand(prk, info, length, hash))
end

--- HKDF (combined) returning a hex string.
function M.hkdf_hex(salt, ikm, info, length, hash)
    return to_hex(M.hkdf(salt, ikm, info, length, hash))
end

return M
