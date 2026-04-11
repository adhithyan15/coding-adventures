--[[
  coding_adventures.scrypt — scrypt Key Derivation Function
  RFC 7914, implemented from scratch in Lua 5.4.

  What Is scrypt?
  ===============
  scrypt (pronounced "ess-crypt") is a **memory-hard** password hashing and key
  derivation function designed by Colin Percival in 2009 and standardised in
  RFC 7914 (2016).

  Memory-hardness is the key property that distinguishes scrypt from PBKDF2:
  an attacker with specialised hardware (ASICs, FPGAs, GPUs) gains far less
  advantage because every verification must hold a large block of data in RAM.
  The cost of RAM per chip is roughly constant, so buying 10× the chips gives
  only ~10× the attack rate — far less leverage than with pure CPU-hard functions
  where silicon area can be traded for massive parallelism.

  Real-world uses:
  - Litecoin: original motivation (ASIC-resistant proof of work)
  - OpenBSD: bcrypt replacement candidate
  - 1Password and other password managers
  - Many web frameworks (e.g. Django alternative, libsodium)

  The scrypt Algorithm (RFC 7914 §5)
  =====================================

    1.  B  = PBKDF2-HMAC-SHA256(P, S, 1, p × 128 × r)
           Break B into p independent 128r-byte blocks B_0, …, B_{p-1}.
    2.  For i = 0..p-1:
           B_i = ROMix(B_i, N, r)
    3.  DK = PBKDF2-HMAC-SHA256(P, B_0 || … || B_{p-1}, 1, dkLen)

  Parameters
  ----------
    P     — password (may be empty — RFC 7914 vector 1 uses "")
    S     — salt
    N     — CPU/memory cost parameter; must be a power of 2, N ≥ 2
    r     — block size multiplier (32r 64-bit words per mixing block)
    p     — parallelisation parameter
    dkLen — desired key length in bytes

  Memory allocated: O(N × r) 64-byte blocks per ROMix call, × p.

  Key Building Blocks
  ====================

    Salsa20/8  — 8-round version of the Salsa20 stream cipher core.
                  Takes a 64-byte block, mixes it with a quarter-round ARX
                  (add-rotate-XOR) structure, and returns a new 64-byte block.
                  Used by BlockMix for diffusion.

    BlockMix   — Mixes a sequence of 2r Salsa20/8 blocks.

    ROMix      — The sequential memory-hard layer.
                  Fills a large V table of N block-sequences, then does N
                  pseudo-random lookups indexed by Integerify, XOR-ing blocks
                  before re-mixing. An attacker who cannot store V must
                  recompute entries on every lookup — multiplying the work by N.

  Salsa20/8 Internals
  ====================
  Salsa20 operates on a 4×4 matrix of 32-bit words. The quarter-round function
  QR(a, b, c, d) is:

      b ⊕= (a + d) <<< 7
      c ⊕= (b + a) <<< 9
      d ⊕= (c + b) <<< 13
      a ⊕= (d + c) <<< 18

  where ⊕ is XOR, + is 32-bit wrapping addition, and <<< n is left-rotation.

  Salsa20 applies 10 double-rounds (each double-round = 1 column + 1 row pass).
  Salsa20/8 uses only 4 double-rounds (half the security, twice the speed).

  Lua 5.4 Bit-Arithmetic Notes
  =============================
  Lua 5.4 integers are 64-bit signed. We emulate 32-bit unsigned arithmetic:

    Addition:     (a + b) & 0xFFFFFFFF   — mask to low 32 bits
    Rotation:     ((x << n) | (x >> (32 - n))) & 0xFFFFFFFF
    XOR:          a ~ b                  (tilde is XOR in Lua 5.4)
    AND/OR:       a & b  /  a | b

  All intermediate addition results are masked before rotation to prevent
  the high bits from contaminating the rotate (>> of a negative 64-bit number
  would sign-extend, producing wrong results if we did not mask first).

  Byte Order
  ===========
  scrypt uses little-endian 32-bit words throughout (same as Salsa20).
  Words are packed/unpacked with parse_le_u32 / pack_le_u32.

  Empty-Password Handling
  ========================
  RFC 7914 test vector 1 uses password = "" (empty string).
  Our `hmac_sha256` public API rejects empty keys (HMAC security policy).
  The internal PBKDF2 used by scrypt bypasses that guard by calling
  `hmac.hmac(sha256_fn, 64, password, message)` directly, which is the
  generic HMAC engine that does not enforce the non-empty-key invariant.
  This matches the RFC specification behaviour.
--]]

-- ─── Dependencies ─────────────────────────────────────────────────────────────

local hmac      = require("coding_adventures.hmac")
local sha256_m  = require("coding_adventures.sha256")

-- sha256 function: string → table of 32 byte integers
local sha256_fn = sha256_m.sha256

-- ─── Utility: 32-bit little-endian word I/O ──────────────────────────────────

-- parse_le_u32: read a 32-bit unsigned little-endian word from string s.
-- offset is 1-based (Lua convention). Four bytes starting at offset.
--
-- Example: parse_le_u32("\x01\x02\x03\x04", 1) → 0x04030201
local function parse_le_u32(s, offset)
  local a, b, c, d = string.byte(s, offset, offset + 3)
  return a | (b << 8) | (c << 16) | (d << 24)
end

-- pack_le_u32: encode a 32-bit unsigned integer as a 4-byte little-endian string.
-- The mask ensures we only look at the low 32 bits (Lua integers are 64-bit).
--
-- Example: pack_le_u32(0x04030201) → "\x01\x02\x03\x04"
local function pack_le_u32(x)
  x = x & 0xFFFFFFFF
  return string.char(
    x & 0xFF,
    (x >> 8) & 0xFF,
    (x >> 16) & 0xFF,
    (x >> 24) & 0xFF
  )
end

-- ─── Salsa20/8 ────────────────────────────────────────────────────────────────

-- rotl32: rotate x left by n bits in a 32-bit unsigned space.
-- Both << and >> operate on 64-bit Lua integers, so we mask the result.
--
-- Visual:  [b31 … b(32-n) | b(31-n) … b0]
--         becomes  [b(31-n) … b0 | b31 … b(32-n)]
local function rotl32(x, n)
  return ((x << n) | (x >> (32 - n))) & 0xFFFFFFFF
end

-- salsa20_8: apply 8 rounds (4 double-rounds) of the Salsa20 core to a
-- 64-byte string, returning a new 64-byte string.
--
-- The Salsa20 state is a 4×4 matrix of 32-bit words, laid out as indices 0-15.
-- Column indices:        Row indices:
--   0  4  8  12          0  1  2  3
--   1  5  9  13          4  5  6  7
--   2  6  10 14          8  9  10 11
--   3  7  11 15         12 13 14 15
--
-- Each double-round applies column QRs then row QRs.
-- The output is x_final + z_initial (word-wise wrapping addition).
local function salsa20_8(s)
  -- Parse 64 bytes → 16 LE words (0-indexed for algorithmic clarity)
  local x = {}
  for i = 0, 15 do
    x[i] = parse_le_u32(s, i * 4 + 1)
  end

  -- Save original values for the final addition step
  local z = {}
  for i = 0, 15 do z[i] = x[i] end

  -- Quarter-round: ARX operations on 4 words at indices a, b, c, d.
  -- Each line: target ⊕= rotl(source_sum, rotation_amount)
  local function qr(a, b, c, d)
    x[b] = (x[b] ~ rotl32((x[a] + x[d]) & 0xFFFFFFFF, 7))  & 0xFFFFFFFF
    x[c] = (x[c] ~ rotl32((x[b] + x[a]) & 0xFFFFFFFF, 9))  & 0xFFFFFFFF
    x[d] = (x[d] ~ rotl32((x[c] + x[b]) & 0xFFFFFFFF, 13)) & 0xFFFFFFFF
    x[a] = (x[a] ~ rotl32((x[d] + x[c]) & 0xFFFFFFFF, 18)) & 0xFFFFFFFF
  end

  -- 4 double-rounds = 8 total rounds = Salsa20/8
  for _ = 1, 4 do
    -- Column rounds (operate on columns of the 4×4 matrix)
    qr(0, 4, 8, 12)
    qr(5, 9, 13, 1)
    qr(10, 14, 2, 6)
    qr(15, 3, 7, 11)
    -- Row rounds (operate on rows of the 4×4 matrix)
    qr(0, 1, 2, 3)
    qr(5, 6, 7, 4)
    qr(10, 11, 8, 9)
    qr(15, 12, 13, 14)
  end

  -- Final step: add the original state back (feed-forward / ARX finalisation)
  local result = {}
  for i = 0, 15 do
    result[i + 1] = pack_le_u32((x[i] + z[i]) & 0xFFFFFFFF)
  end
  return table.concat(result)
end

-- ─── XOR helpers ──────────────────────────────────────────────────────────────

-- xor64: XOR two 64-byte strings byte-by-byte, returning a new 64-byte string.
-- Used to combine a block from the V table with the current working block.
local function xor64(a, b)
  local out = {}
  for i = 1, 64 do
    out[i] = string.char(string.byte(a, i) ~ string.byte(b, i))
  end
  return table.concat(out)
end

-- xor_blocks: XOR two block-lists element-wise.
-- Both lists have the same number of 64-byte blocks.
-- Returns a new list of XOR-combined blocks.
local function xor_blocks(a_blocks, b_blocks)
  local out = {}
  for i = 1, #a_blocks do
    out[i] = xor64(a_blocks[i], b_blocks[i])
  end
  return out
end

-- ─── BlockMix ─────────────────────────────────────────────────────────────────

-- block_mix: mix a sequence of 2r 64-byte blocks using Salsa20/8.
--
-- Input:  blocks — table of 2r 64-byte strings (1-indexed)
--         r      — block size multiplier
--
-- Algorithm (RFC 7914 §3):
--   X = blocks[2r]    (start with the last block)
--   For i = 1..2r:
--     X = Salsa20/8(X XOR blocks[i])
--     Y[i] = X
--   Return [Y[1], Y[3], …, Y[2r-1], Y[2], Y[4], …, Y[2r]]
--              (odd-indexed first, then even-indexed)
--
-- The interleaving at the end is a deliberate design: it mixes the two
-- "halves" of the block-sequence together for better diffusion.
local function block_mix(blocks, r)
  local two_r = 2 * r
  local x = blocks[two_r]   -- start from the last block
  local y = {}

  for i = 1, two_r do
    x = salsa20_8(xor64(x, blocks[i]))
    y[i] = x
  end

  -- Interleave: odd indices → first half of output, even → second half
  local out = {}
  for i = 1, r do
    out[i]     = y[2 * i - 1]  -- odd-indexed outputs (1, 3, 5, …)
    out[r + i] = y[2 * i]      -- even-indexed outputs (2, 4, 6, …)
  end
  return out
end

-- ─── Integerify ───────────────────────────────────────────────────────────────

-- integerify: interpret the first 8 bytes of the last block as a 64-bit LE
-- integer, then return it modulo N to get a pseudo-random table index.
--
-- Since N ≤ 2^20 (our limit), the low 32 bits of the 64-bit word are more than
-- sufficient. Lua integers are 64-bit, so the left-shift never overflows.
local function integerify(x)
  local last = x[#x]
  local a, b, c, d = string.byte(last, 1, 4)
  return a | (b << 8) | (c << 16) | (d << 24)
end

-- ─── ROMix ────────────────────────────────────────────────────────────────────

-- ro_mix: the sequential memory-hard mixing operation (RFC 7914 §4).
--
-- Input:  b_bytes — a 128r-byte string representing 2r 64-byte blocks
--         n       — memory cost (must be a power of 2)
--         r       — block size multiplier
-- Output: a 128r-byte string — the result after N fill-and-lookup passes.
--
-- Algorithm:
--   1. Parse b_bytes into a list x of 2r 64-byte blocks.
--   2. Build a "ROM" table V of N snapshots of x, updating x with BlockMix.
--   3. Perform N pseudo-random lookups into V:
--        j = Integerify(x) mod N
--        x = BlockMix(x XOR V[j])
--   4. Concatenate x back into a byte string.
--
-- Why is this memory-hard?
-- An adversary without enough RAM to store V must recompute the required V[j]
-- entry on each of the N lookups, costing O(N^2) work instead of O(N).
-- This makes parallelisation with limited memory extremely expensive.
local function ro_mix(b_bytes, n, r)
  local two_r = 2 * r

  -- Step 1: Parse the flat byte string into a list of 64-byte block strings
  local x = {}
  for i = 1, two_r do
    x[i] = b_bytes:sub((i - 1) * 64 + 1, i * 64)
  end

  -- Step 2: Fill the V table (N snapshots of x)
  local v = {}
  for i = 1, n do
    -- Snapshot: copy x's blocks by value (strings are immutable in Lua)
    v[i] = {}
    for k = 1, two_r do
      v[i][k] = x[k]
    end
    x = block_mix(x, r)
  end

  -- Step 3: N pseudo-random lookups
  for _ = 1, n do
    local j = (integerify(x) % n) + 1   -- 1-based table index
    x = block_mix(xor_blocks(x, v[j]), r)
  end

  -- Step 4: Concatenate 2r blocks back into a flat byte string
  return table.concat(x)
end

-- ─── Internal PBKDF2-HMAC-SHA256 ─────────────────────────────────────────────

-- pbkdf2_sha256_raw: PBKDF2 with HMAC-SHA256, supporting empty passwords.
--
-- This is the private PBKDF2 used by scrypt internally. It bypasses the
-- hmac_sha256 public API (which rejects empty keys) by calling the generic
-- hmac.hmac() engine directly with sha256_fn.
--
-- RFC 7914 §5 specifies c=1 (iterations=1) for both the initial and final
-- PBKDF2 calls within scrypt. This function supports arbitrary iterations
-- for completeness.
--
-- password   — raw byte string, may be empty
-- salt       — raw byte string
-- iterations — number of PRF applications per block
-- key_length — requested output length in bytes
--
-- Returns a raw byte string of exactly key_length bytes.
local function pbkdf2_sha256_raw(password, salt, iterations, key_length)
  local h_len = 32  -- SHA-256 digest length in bytes
  local num_blocks = math.ceil(key_length / h_len)
  local dk_parts = {}

  for i = 1, num_blocks do
    -- Append INT_32_BE(i): the block counter as a 4-byte big-endian integer.
    -- string.pack(">I4", i) is available in Lua 5.3+.
    local seed = salt .. string.pack(">I4", i)

    -- U_1 = HMAC-SHA256(password, seed)
    -- We call hmac.hmac() directly to allow empty passwords.
    local u_arr = hmac.hmac(sha256_fn, 64, password, seed)
    local u = string.char(table.unpack(u_arr))

    -- t will accumulate XOR of all U values: T_i = U_1 XOR U_2 XOR … XOR U_c
    local t = { string.byte(u, 1, h_len) }

    -- U_j = HMAC-SHA256(password, U_{j-1}), XOR into t
    for _ = 2, iterations do
      u_arr = hmac.hmac(sha256_fn, 64, password, u)
      u = string.char(table.unpack(u_arr))
      local u_bytes = { string.byte(u, 1, h_len) }
      for k = 1, h_len do
        t[k] = t[k] ~ u_bytes[k]
      end
    end

    dk_parts[i] = string.char(table.unpack(t))
  end

  local dk = table.concat(dk_parts)
  return dk:sub(1, key_length)
end

-- ─── Public API ───────────────────────────────────────────────────────────────

local M = {}

--- scrypt: derive a key from a password using the scrypt algorithm (RFC 7914).
--
-- @param password  string — the secret passphrase (may be empty)
-- @param salt      string — a unique random value per credential
-- @param n         integer — CPU/memory cost; must be a power of 2, ≥ 2, ≤ 2^20
-- @param r         integer — block size factor; must be ≥ 1
-- @param p         integer — parallelisation factor; must be ≥ 1
-- @param dk_len    integer — output key length in bytes (1 .. 2^20)
-- @return          string — raw derived key of dk_len bytes
--
-- Typical parameters for interactive login (2023 guidance):
--   N=16384 (2^14), r=8, p=1
-- For high-security offline keys:
--   N=1048576 (2^20), r=8, p=1
--
-- Example:
--   local M = require("coding_adventures.scrypt")
--   local hex_key = M.scrypt_hex("my password", "random salt", 16384, 8, 1, 32)
function M.scrypt(password, salt, n, r, p, dk_len)
  -- ── Parameter validation ──────────────────────────────────────────────────

  -- N must be a power of 2 ≥ 2. Test (n & (n-1)) == 0 only holds for powers
  -- of 2; we also require n ≥ 2 (n=1 makes no sense for ROMix).
  if type(n) ~= "number" or n < 2 or (n & (n - 1)) ~= 0 then
    error("scrypt N must be a power of 2 and >= 2", 2)
  end
  -- Limit N to prevent runaway memory allocation (2^20 × r × 128 bytes per block).
  if n > 2 ^ 20 then
    error("scrypt N must not exceed 2^20", 2)
  end
  if type(r) ~= "number" or r < 1 then
    error("scrypt r must be a positive integer", 2)
  end
  if type(p) ~= "number" or p < 1 then
    error("scrypt p must be a positive integer", 2)
  end
  if type(dk_len) ~= "number" or dk_len < 1 or dk_len > 2 ^ 20 then
    error("scrypt dk_len must be between 1 and 2^20", 2)
  end
  -- RFC 7914 §2: p * r must not overflow a specific limit.
  if p * r > 2 ^ 30 then
    error("scrypt p * r exceeds limit", 2)
  end

  -- ── Step 1: Expand password+salt into p mixing blocks ────────────────────
  -- PBKDF2-HMAC-SHA256 with 1 iteration, key length = p × 128r bytes.
  -- The factor 128r comes from: 2r blocks × 64 bytes per block.
  local b = pbkdf2_sha256_raw(password, salt, 1, p * 128 * r)

  -- ── Step 2: ROMix each of the p independent 128r-byte chunks ─────────────
  -- Each chunk is independently hashed through the memory-hard ROMix routine.
  -- In a parallel implementation these p calls could run on separate threads,
  -- hence "parallelisation factor p".
  local blocks = {}
  for i = 1, p do
    local chunk = b:sub((i - 1) * 128 * r + 1, i * 128 * r)
    blocks[i] = ro_mix(chunk, n, r)
  end
  local b_mixed = table.concat(blocks)

  -- ── Step 3: Extract the final derived key ────────────────────────────────
  -- A second PBKDF2 call re-uses the password over the mixed block material.
  -- This provides the output stretching (dk_len may differ from 128r×p).
  return pbkdf2_sha256_raw(password, b_mixed, 1, dk_len)
end

--- scrypt_hex: like scrypt but returns a lowercase hexadecimal string.
--
-- This is the most convenient form for storing derived keys in databases,
-- config files, or log lines.
--
-- @param password  string
-- @param salt      string
-- @param n         integer
-- @param r         integer
-- @param p         integer
-- @param dk_len    integer
-- @return          string — 2×dk_len hex characters
function M.scrypt_hex(password, salt, n, r, p, dk_len)
  local dk = M.scrypt(password, salt, n, r, p, dk_len)
  return (dk:gsub(".", function(c)
    return string.format("%02x", string.byte(c))
  end))
end

return M
