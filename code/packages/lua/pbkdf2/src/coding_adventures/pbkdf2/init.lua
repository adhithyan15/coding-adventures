--[[
  coding_adventures.pbkdf2 -- PBKDF2 (Password-Based Key Derivation Function 2)
  RFC 8018 (formerly RFC 2898 / PKCS#5 v2.1)

  What Is PBKDF2?
  ===============
  PBKDF2 derives a cryptographic key from a password by applying a pseudorandom
  function (PRF) — typically HMAC — `c` times per output block. The iteration
  count `c` is the tunable cost: every brute-force guess requires the same `c`
  PRF calls as the original derivation.

  Real-world uses:
  - WPA2 Wi-Fi: PBKDF2-HMAC-SHA1, 4096 iterations
  - Django: PBKDF2-HMAC-SHA256, 720,000 iterations (2024)
  - macOS Keychain: PBKDF2-HMAC-SHA256

  Algorithm (RFC 8018 § 5.2)
  ===========================

    DK = T_1 || T_2 || ... (first dk_len bytes)

    T_i = U_1 XOR U_2 XOR ... XOR U_c

    U_1 = PRF(password, salt || INT_32_BE(i))
    U_j = PRF(password, U_{j-1})   for j = 2..c

  INT_32_BE(i) encodes the block counter as a 4-byte big-endian integer.
  In Lua, string.pack(">I4", i) produces this encoding.

  Security Notes
  ==============
  OWASP 2023 minimum iteration counts:
  - HMAC-SHA256: 600,000
  - HMAC-SHA1:   1,300,000

  For new systems prefer Argon2id (memory-hard, resists GPU attacks).
--]]

local hmac = require("coding_adventures.hmac")

local M = {}

-- ──────────────────────────────────────────────────────────────────────────────
-- Helpers
-- ──────────────────────────────────────────────────────────────────────────────

-- bytes_to_str: convert a table of byte values to a Lua string.
-- The HMAC functions in this monorepo return byte tables, not strings.
local function bytes_to_str(t)
  local chars = {}
  for i, b in ipairs(t) do chars[i] = string.char(b) end
  return table.concat(chars)
end

-- to_str: accept either a string or a byte table, returning a raw byte string.
local function to_str(v)
  if type(v) == "string" then return v end
  return bytes_to_str(v)
end

-- xor_strings: XOR two equal-length byte strings, returning a new string.
local function xor_strings(a, b)
  local result = {}
  for i = 1, #a do
    result[i] = string.char(
      string.byte(a, i) ~ string.byte(b, i)
    )
  end
  return table.concat(result)
end

-- to_hex: convert a byte string to a lowercase hex string.
local function to_hex(s)
  return (s:gsub(".", function(c)
    return string.format("%02x", string.byte(c))
  end))
end

-- ──────────────────────────────────────────────────────────────────────────────
-- Core loop
-- ──────────────────────────────────────────────────────────────────────────────

-- _pbkdf2: generic PBKDF2 implementation.
--
-- prf:        function(key, msg) → string of h_len bytes
-- h_len:      output byte length of prf
-- password:   secret being stretched (string, raw bytes)
-- salt:       unique random value per credential (string, raw bytes)
-- iterations: number of PRF calls per block
-- key_length: number of derived bytes
local function _pbkdf2(prf, h_len, password, salt, iterations, key_length)
  if #password == 0 then
    error("PBKDF2 password must not be empty", 2)
  end
  if type(iterations) ~= "number" or iterations <= 0 or math.floor(iterations) ~= iterations then
    error("PBKDF2 iterations must be a positive integer", 2)
  end
  if type(key_length) ~= "number" or key_length <= 0 or math.floor(key_length) ~= key_length then
    error("PBKDF2 key_length must be a positive integer", 2)
  end

  local num_blocks = math.ceil(key_length / h_len)
  local dk_parts = {}

  for i = 1, num_blocks do
    -- Seed = salt || INT_32_BE(i)
    -- string.pack(">I4", i) encodes i as a 4-byte big-endian unsigned integer.
    local seed = salt .. string.pack(">I4", i)

    -- U_1 = PRF(password, seed)
    local u = prf(password, seed)

    -- t accumulates the XOR of all U values.
    local t = u

    -- U_j = PRF(password, U_{j-1}), XOR into t.
    for _ = 2, iterations do
      u = prf(password, u)
      t = xor_strings(t, u)
    end

    dk_parts[i] = t
  end

  local dk = table.concat(dk_parts)
  return dk:sub(1, key_length)
end

-- ──────────────────────────────────────────────────────────────────────────────
-- Public API — concrete PRF variants
-- ──────────────────────────────────────────────────────────────────────────────

--- PBKDF2 with HMAC-SHA1 as the PRF.
-- hLen = 20 bytes. Used in WPA2 (4096 iterations).
-- For new systems prefer pbkdf2_hmac_sha256.
--
-- RFC 6070 test vector:
-- > require("coding_adventures.pbkdf2").pbkdf2_hmac_sha1_hex("password", "salt", 1, 20)
-- "0c60c80f961f0e71f3a9b524af6012062fe037a6"
function M.pbkdf2_hmac_sha1(password, salt, iterations, key_length)
  return _pbkdf2(
    function(key, msg) return to_str(hmac.hmac_sha1(key, msg)) end,
    20, password, salt, iterations, key_length
  )
end

--- PBKDF2 with HMAC-SHA256 as the PRF.
-- hLen = 32 bytes. Recommended for new systems (OWASP 2023: ≥ 600,000 iterations).
function M.pbkdf2_hmac_sha256(password, salt, iterations, key_length)
  return _pbkdf2(
    function(key, msg) return to_str(hmac.hmac_sha256(key, msg)) end,
    32, password, salt, iterations, key_length
  )
end

--- PBKDF2 with HMAC-SHA512 as the PRF.
-- hLen = 64 bytes. Suitable for high-security applications.
function M.pbkdf2_hmac_sha512(password, salt, iterations, key_length)
  return _pbkdf2(
    function(key, msg) return to_str(hmac.hmac_sha512(key, msg)) end,
    64, password, salt, iterations, key_length
  )
end

-- ──────────────────────────────────────────────────────────────────────────────
-- Hex variants
-- ──────────────────────────────────────────────────────────────────────────────

--- Like pbkdf2_hmac_sha1 but returns a lowercase hex string.
function M.pbkdf2_hmac_sha1_hex(password, salt, iterations, key_length)
  return to_hex(M.pbkdf2_hmac_sha1(password, salt, iterations, key_length))
end

--- Like pbkdf2_hmac_sha256 but returns a lowercase hex string.
function M.pbkdf2_hmac_sha256_hex(password, salt, iterations, key_length)
  return to_hex(M.pbkdf2_hmac_sha256(password, salt, iterations, key_length))
end

--- Like pbkdf2_hmac_sha512 but returns a lowercase hex string.
function M.pbkdf2_hmac_sha512_hex(password, salt, iterations, key_length)
  return to_hex(M.pbkdf2_hmac_sha512(password, salt, iterations, key_length))
end

return M
