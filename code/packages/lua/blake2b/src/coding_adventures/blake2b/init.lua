-- blake2b -- BLAKE2b cryptographic hash function (RFC 7693) in pure Lua.
--
-- BLAKE2b is the 64-bit variant of the BLAKE2 family: faster than MD5 on
-- modern hardware, and at least as secure as SHA-3 against every known
-- attack.  It was designed in 2012 as a drop-in replacement for SHA-2 in
-- performance-sensitive contexts, and it is the hash used internally by
-- Argon2, libsodium, WireGuard, Noise Protocol, and IPFS.
--
-- WHY THIS PACKAGE EXISTS
-- -----------------------
-- The larger "HF06" spec in this repo stands up BLAKE2b in ten languages
-- because it is a hard prerequisite for Argon2.  This Lua port mirrors
-- the Python, Go, TypeScript, Rust, Ruby, Elixir, and Swift siblings --
-- same KAT tables, same public surface, same parameterization.
--
-- HOW BLAKE2b WORKS -- A GUIDED TOUR
-- ----------------------------------
-- 1. PARAMETER-BLOCK INITIALIZATION (RFC 7693 section 2.5)
--    An 8-word initial state `h` is computed by XOR-ing the SHA-512 IVs
--    (the fractional-parts-of-sqrt constants reused as "nothing up my
--    sleeve" values) with a 64-byte "parameter block" encoding:
--      - digest_size (1 byte): how many bytes of output you want, 1..64
--      - key_length  (1 byte): length of optional MAC key, 0..64
--      - fanout (1): always 1 for sequential
--      - depth  (1): always 1 for sequential
--      - leaf_length, node_offset, node_depth, inner_length: all 0
--      - salt     (16 bytes): optional domain-separator part 1
--      - personal (16 bytes): optional domain-separator part 2
--
-- 2. COMPRESSION (F function, RFC 7693 section 3.2)
--    Each 128-byte input block is absorbed into the state by running
--    twelve ARX rounds over a 16-word working vector v = h || IV, with
--    a 128-bit byte counter folded into v[12..13] and a final-flag
--    inversion applied to v[14] on the last block.  The quarter-round
--    G uses the famous (32, 24, 16, 63) rotation constants.
--
-- 3. DAVIES-MEYER FEED-FORWARD
--    After twelve rounds, h[i] XOR= v[i] XOR v[i+8] for i in 0..7.
--    This XOR back-mixes the pre-compression state so that even an
--    attacker who could invert G's permutation cannot invert F.
--
-- 4. FINAL-BLOCK FLAGGING (the classic BLAKE2 off-by-one)
--    Only the LAST real block is flagged final.  If the message is an
--    exact multiple of 128 bytes, DO NOT add an empty padding block --
--    flag the last real block.  Consequently `update()` must keep at
--    least one byte in its internal buffer: it can only compress a
--    block when more data is known to follow.
--
-- LUA 5.3+ 64-BIT ARITHMETIC NOTES
-- --------------------------------
-- Lua 5.3+ uses native 64-bit signed integers with bitwise operators,
-- and integer addition wraps naturally on 64-bit overflow.  That means
-- `x + y` in this file is already `(x + y) mod 2^64` -- we do not need
-- the `& 0xFFFFFFFFFFFFFFFF` mask that the Python/Ruby ports apply.
--
-- Lua's `>>` is a LOGICAL (zero-fill) right shift, not arithmetic, so
-- a 64-bit right rotation is simply:
--   (x >> n) | (x << (64 - n))
-- No masking required.
--
-- For byte extraction from a 64-bit word we do `(x >> (8*i)) & 0xFF`.
-- The low byte is `x & 0xFF`, which correctly selects the least
-- significant 8 bits even when x is "negative" in Lua's signed view.
--
-- Usage:
--   local blake2b = require("coding_adventures.blake2b")
--   local hex = blake2b.hex("hello")           -- 128-char hex string
--   local raw = blake2b.digest("hello")        -- 64-char raw string
--   local h   = blake2b.Hasher.new{ digest_size = 32 }
--   h:update("hello "); h:update("world")
--   local out = h:hex_digest()

local M = {}

M.VERSION = "0.1.0"

-- ---------------------------------------------------------------------------
-- Initial Hash Values (IVs) -- identical to SHA-512
--
-- First 64 bits of the fractional parts of the square roots of the first
-- eight primes (2, 3, 5, 7, 11, 13, 17, 19).  Reusing SHA-512's IVs is a
-- deliberate design choice: reviewers can verify there is no hidden
-- backdoor without trusting BLAKE2-specific constants.
-- ---------------------------------------------------------------------------
local IV = {
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
-- Message-schedule permutations (SIGMA)
--
-- Ten distinct permutations of 0..15.  Round i uses SIGMA[i % 10] to pick
-- the order in which to mix the 16 message words into the working vector.
-- Rounds 10 and 11 reuse SIGMA[0] and SIGMA[1] (twelve total rounds, ten
-- distinct permutations).
-- ---------------------------------------------------------------------------
local SIGMA = {
    { 0,  1,  2,  3,  4,  5,  6,  7,  8,  9, 10, 11, 12, 13, 14, 15},
    {14, 10,  4,  8,  9, 15, 13,  6,  1, 12,  0,  2, 11,  7,  5,  3},
    {11,  8, 12,  0,  5,  2, 15, 13, 10, 14,  3,  6,  7,  1,  9,  4},
    { 7,  9,  3,  1, 13, 12, 11, 14,  2,  6,  5, 10,  4,  0, 15,  8},
    { 9,  0,  5,  7,  2,  4, 10, 15, 14,  1, 11, 12,  6,  8,  3, 13},
    { 2, 12,  6, 10,  0, 11,  8,  3,  4, 13,  7,  5, 15, 14,  1,  9},
    {12,  5,  1, 15, 14, 13,  4, 10,  0,  7,  6,  3,  9,  2,  8, 11},
    {13, 11,  7, 14, 12,  1,  3,  9,  5,  0, 15,  4,  8,  6,  2, 10},
    { 6, 15, 14,  9, 11,  3,  0,  8, 12,  2, 13,  7,  1,  4, 10,  5},
    {10,  2,  8,  4,  7,  6,  1,  5, 15, 11,  9, 14,  3, 12, 13,  0},
}

-- Block size in bytes.  BLAKE2b compresses one 128-byte block at a time.
local BLOCK_SIZE = 128

-- ---------------------------------------------------------------------------
-- rotr64(x, n) -- right-rotate a 64-bit word by n bits.
--
-- Lua 5.3+ `>>` is logical (zero-fill), so no mask is required -- the high
-- bits shifted out by `<<` wrap into the low positions cleared by `>>`.
-- ---------------------------------------------------------------------------
local function rotr64(x, n)
    return (x >> n) | (x << (64 - n))
end

-- ---------------------------------------------------------------------------
-- parse_le64(s, offset) -- read 8 little-endian bytes from string `s`
-- starting at 1-indexed `offset`, returning a 64-bit integer.
--
-- Matches the LE word layout used by BLAKE2b's message parsing and
-- parameter block.
-- ---------------------------------------------------------------------------
local function parse_le64(s, offset)
    local b0 = string.byte(s, offset)
    local b1 = string.byte(s, offset + 1)
    local b2 = string.byte(s, offset + 2)
    local b3 = string.byte(s, offset + 3)
    local b4 = string.byte(s, offset + 4)
    local b5 = string.byte(s, offset + 5)
    local b6 = string.byte(s, offset + 6)
    local b7 = string.byte(s, offset + 7)
    return b0
         | (b1 << 8)
         | (b2 << 16)
         | (b3 << 24)
         | (b4 << 32)
         | (b5 << 40)
         | (b6 << 48)
         | (b7 << 56)
end

-- ---------------------------------------------------------------------------
-- u64_to_le_bytes(w) -- pack a 64-bit word as 8 little-endian bytes (string)
-- ---------------------------------------------------------------------------
local function u64_to_le_bytes(w)
    return string.char(
         w         & 0xff,
        (w >>  8)  & 0xff,
        (w >> 16)  & 0xff,
        (w >> 24)  & 0xff,
        (w >> 32)  & 0xff,
        (w >> 40)  & 0xff,
        (w >> 48)  & 0xff,
        (w >> 56)  & 0xff
    )
end

-- ---------------------------------------------------------------------------
-- G(v, a, b, c, d, x, y) -- the BLAKE2b quarter-round.
--
-- Mutates four words v[a], v[b], v[c], v[d] of the 16-word working vector
-- by mixing in two message words x, y.  Uses only addition, XOR, and
-- rotation (the "ARX" primitive family) -- no S-boxes, no table lookups.
--
-- The rotation constants (32, 24, 16, 63) are from RFC 7693 Appendix D.
-- Changing any of them breaks compatibility with every BLAKE2b
-- implementation on earth.
-- ---------------------------------------------------------------------------
local function G(v, a, b, c, d, x, y)
    v[a] = v[a] + v[b] + x
    v[d] = rotr64(v[d] ~ v[a], 32)
    v[c] = v[c] + v[d]
    v[b] = rotr64(v[b] ~ v[c], 24)
    v[a] = v[a] + v[b] + y
    v[d] = rotr64(v[d] ~ v[a], 16)
    v[c] = v[c] + v[d]
    v[b] = rotr64(v[b] ~ v[c], 63)
end

-- ---------------------------------------------------------------------------
-- F(h, block, t, final) -- the BLAKE2b compression function.
--
-- Absorbs one 128-byte block into the 8-word state `h`.  `block` is a 128-
-- byte Lua string; `t` is the total byte count fed through the hash so
-- far (INCLUDING this block's contribution, even if zero-padded); `final`
-- is true only for the very last compression call.
--
-- h is an array-table indexed 1..8 (Lua convention), mutated in place.
-- ---------------------------------------------------------------------------
local function F(h, block, t, final)
    -- Parse the block as sixteen little-endian 64-bit words.
    local m = {}
    for i = 0, 15 do
        m[i] = parse_le64(block, i * 8 + 1)
    end

    -- Build the 16-word working vector: state (indices 0..7) followed by
    -- the IVs (indices 8..15).  We index 0..15 here to match the RFC
    -- notation verbatim; conversions to 1-based indexing happen at the
    -- boundary.
    local v = {}
    for i = 1, 8 do
        v[i - 1] = h[i]
        v[i + 7] = IV[i]
    end

    -- Fold the 128-bit byte counter into v[12..13].
    --
    -- For messages up to 2^64 - 1 bytes (the practical limit), the high
    -- 64 bits of `t` are always zero.  We therefore XOR only the low
    -- 64 bits.  Lua cannot easily represent a 128-bit counter, but the
    -- spec's reserved high word is zero for every realistic input, so
    -- this matches the reference implementation exactly.
    v[12] = v[12] ~ t
    -- v[13] ^= 0  (no-op; message < 2^64 bytes)

    -- Final-block domain separation: invert v[14] so the last compression
    -- cannot be confused with any intermediate one.  This prevents
    -- length-extension attacks at the construction level.
    if final then
        v[14] = v[14] ~ 0xffffffffffffffff
    end

    -- Twelve rounds.  Each round applies G to four columns, then to four
    -- diagonals -- exactly the "double-round" shape ChaCha20 uses.
    for i = 0, 11 do
        local s = SIGMA[(i % 10) + 1]
        -- Columns
        G(v, 0, 4,  8, 12, m[s[ 1]], m[s[ 2]])
        G(v, 1, 5,  9, 13, m[s[ 3]], m[s[ 4]])
        G(v, 2, 6, 10, 14, m[s[ 5]], m[s[ 6]])
        G(v, 3, 7, 11, 15, m[s[ 7]], m[s[ 8]])
        -- Diagonals
        G(v, 0, 5, 10, 15, m[s[ 9]], m[s[10]])
        G(v, 1, 6, 11, 12, m[s[11]], m[s[12]])
        G(v, 2, 7,  8, 13, m[s[13]], m[s[14]])
        G(v, 3, 4,  9, 14, m[s[15]], m[s[16]])
    end

    -- Feed-forward: XOR both halves of the working vector back into the
    -- state.  Makes F one-way.
    for i = 1, 8 do
        h[i] = h[i] ~ v[i - 1] ~ v[i + 7]
    end
end

-- ---------------------------------------------------------------------------
-- validate(digest_size, key, salt, personal)
--
-- Rejects out-of-range parameters with a descriptive error.  The four
-- checks mirror the validation logic in every sibling implementation.
-- ---------------------------------------------------------------------------
local function validate(digest_size, key, salt, personal)
    if math.type(digest_size) ~= "integer"
        or digest_size < 1 or digest_size > 64 then
        error("digest_size must be an integer in [1, 64], got "
              .. tostring(digest_size), 3)
    end
    if #key > 64 then
        error("key length must be in [0, 64], got " .. #key, 3)
    end
    if #salt ~= 0 and #salt ~= 16 then
        error("salt must be exactly 16 bytes (or empty), got " .. #salt, 3)
    end
    if #personal ~= 0 and #personal ~= 16 then
        error("personal must be exactly 16 bytes (or empty), got "
              .. #personal, 3)
    end
end

-- ---------------------------------------------------------------------------
-- initial_state(digest_size, key_len, salt, personal)
--
-- Builds the parameter-block-XORed initial state as an 8-element
-- 1-indexed table.  The parameter block is 64 bytes laid out as eight
-- little-endian 64-bit words; we parse it through the same parse_le64
-- helper used for message blocks.
-- ---------------------------------------------------------------------------
local function initial_state(digest_size, key_len, salt, personal)
    -- 64-byte parameter block, zero-filled.
    local p = {}
    for i = 1, 64 do p[i] = 0 end

    p[1] = digest_size
    p[2] = key_len
    p[3] = 1  -- fanout (sequential)
    p[4] = 1  -- depth  (sequential)
    -- bytes 5..32 remain zero (leaf_length, node_offset, node_depth,
    -- inner_length, and the 14 reserved bytes)
    if #salt == 16 then
        for i = 1, 16 do p[32 + i] = string.byte(salt, i) end
    end
    if #personal == 16 then
        for i = 1, 16 do p[48 + i] = string.byte(personal, i) end
    end

    local p_str = string.char(table.unpack(p))

    local h = {}
    for i = 1, 8 do
        local pw = parse_le64(p_str, (i - 1) * 8 + 1)
        h[i] = IV[i] ~ pw
    end
    return h
end

-- ---------------------------------------------------------------------------
-- Hasher -- the streaming value-type API.
--
-- Usage:
--   local h = blake2b.Hasher.new{ digest_size = 32, key = "" }
--   h:update("hello "); h:update("world")
--   h:hex_digest()
--
-- `digest()` is non-destructive: repeated calls return the same value,
-- and `update()` may be called afterward to continue the stream.
-- `copy()` returns an independent deep copy.
-- ---------------------------------------------------------------------------
local Hasher = {}
Hasher.__index = Hasher

function Hasher.new(opts)
    opts = opts or {}
    local digest_size = opts.digest_size or 64
    local key = opts.key or ""
    local salt = opts.salt or ""
    local personal = opts.personal or ""

    validate(digest_size, key, salt, personal)

    local self = setmetatable({}, Hasher)
    self._digest_size = digest_size
    self._state = initial_state(digest_size, #key, salt, personal)
    -- Buffer is a plain string.  We pay O(n) for each append but use no
    -- C extensions; for the KAT sizes (<= 10 KiB) this is entirely fine.
    self._buffer = ""
    self._byte_count = 0

    if #key > 0 then
        -- Keyed mode: the key, zero-padded to BLOCK_SIZE, becomes the
        -- first block of input.  Subsequent updates behave as if the
        -- padded key had been prepended to the message.
        self._buffer = key .. string.rep("\0", BLOCK_SIZE - #key)
    end
    return self
end

function Hasher:update(data)
    self._buffer = self._buffer .. data
    -- Flush only when the buffer STRICTLY exceeds BLOCK_SIZE.  We must
    -- keep at least one byte around so that digest() has a real block
    -- to flag final.  This is the canonical BLAKE2 off-by-one rule.
    while #self._buffer > BLOCK_SIZE do
        self._byte_count = self._byte_count + BLOCK_SIZE
        F(self._state,
          string.sub(self._buffer, 1, BLOCK_SIZE),
          self._byte_count,
          false)
        self._buffer = string.sub(self._buffer, BLOCK_SIZE + 1)
    end
    return self
end

function Hasher:digest()
    -- Non-destructive: copy state, apply one final compression on a
    -- zero-padded copy of the buffer, serialize, truncate.  The original
    -- hasher state is unchanged.
    local state = {}
    for i = 1, 8 do state[i] = self._state[i] end

    local buffer = self._buffer
    local byte_count = self._byte_count + #buffer
    local final_block = buffer .. string.rep("\0", BLOCK_SIZE - #buffer)
    F(state, final_block, byte_count, true)

    local parts = {}
    for i = 1, 8 do
        parts[i] = u64_to_le_bytes(state[i])
    end
    local full = table.concat(parts)
    return string.sub(full, 1, self._digest_size)
end

function Hasher:hex_digest()
    local raw = self:digest()
    local parts = {}
    for i = 1, #raw do
        parts[i] = string.format("%02x", string.byte(raw, i))
    end
    return table.concat(parts)
end

function Hasher:copy()
    local other = setmetatable({}, Hasher)
    other._digest_size = self._digest_size
    other._state = {}
    for i = 1, 8 do other._state[i] = self._state[i] end
    other._buffer = self._buffer
    other._byte_count = self._byte_count
    return other
end

M.Hasher = Hasher

-- ---------------------------------------------------------------------------
-- One-shot convenience functions.
-- ---------------------------------------------------------------------------

function M.digest(data, opts)
    local h = Hasher.new(opts)
    h:update(data)
    return h:digest()
end

function M.hex(data, opts)
    local h = Hasher.new(opts)
    h:update(data)
    return h:hex_digest()
end

-- Aliases mirroring sibling-language naming.
M.blake2b = M.digest
M.blake2b_hex = M.hex

return M
