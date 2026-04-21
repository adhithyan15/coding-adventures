-- coding_adventures.argon2d -- Argon2d (RFC 9106) in pure Lua.
--
-- Argon2d is the DATA-DEPENDENT member of the Argon2 family: the index
-- of the reference block for every new block is derived from the first
-- 64 bits of the previously computed block.  That makes the memory-
-- access pattern password-dependent, which maximises GPU/ASIC
-- resistance but leaks a noisy side channel.  Pick Argon2d only when
-- timing side-channel attacks are NOT in scope (e.g. proof-of-work).
-- For password hashing, prefer Argon2id.
--
-- Reference: https://datatracker.ietf.org/doc/html/rfc9106
-- See also: code/specs/KD03-argon2.md
--
-- LUA 5.3+ ARITHMETIC NOTES
-- -------------------------
-- Lua 5.3+ uses native 64-bit signed integers whose `+` / `-` / `*`
-- all wrap modulo 2^64, and whose `&`, `|`, `~`, `<<`, `>>` are all
-- 64-bit operations (`>>` is logical, not arithmetic).  Unlike Python
-- or Ruby we do NOT need to mask every 64-bit intermediate with
-- 0xFFFFFFFFFFFFFFFF -- the hardware already does it.  The only mask
-- we keep is MASK32 for the "truncate to 32 bits" step inside G.

local blake2b = require("coding_adventures.blake2b")

local M = {}

M.VERSION = "0.1.0"

local MASK32 = 0xFFFFFFFF
local BLOCK_SIZE = 1024
local BLOCK_WORDS = 128
local SYNC_POINTS = 4
local ARGON2_VERSION = 0x13
local TYPE_D = 0

-- 64-bit right rotation.
local function rotr64(x, n)
    return ((x >> n) | (x << (64 - n)))
end

-- Argon2 G mixer (RFC 9106 §3.5): BLAKE2 G round plus a `2*trunc32(a)*trunc32(b)`
-- cross-term that makes memory-hard attacks quadratic in input size.
-- `v` is a 1-indexed table; a/b/c/d are 1-based indices.
local function g_mix(v, a, b, c, d)
    local va, vb, vc, vd = v[a], v[b], v[c], v[d]

    va = va + vb + 2 * (va & MASK32) * (vb & MASK32)
    vd = rotr64(vd ~ va, 32)
    vc = vc + vd + 2 * (vc & MASK32) * (vd & MASK32)
    vb = rotr64(vb ~ vc, 24)
    va = va + vb + 2 * (va & MASK32) * (vb & MASK32)
    vd = rotr64(vd ~ va, 16)
    vc = vc + vd + 2 * (vc & MASK32) * (vd & MASK32)
    vb = rotr64(vb ~ vc, 63)

    v[a], v[b], v[c], v[d] = va, vb, vc, vd
end

-- Argon2 permutation P: eight G-rounds over a 16-word slab starting
-- at `off+1` (1-indexed).
local function permutation_p(v, off)
    g_mix(v, off + 1, off + 5, off + 9,  off + 13)
    g_mix(v, off + 2, off + 6, off + 10, off + 14)
    g_mix(v, off + 3, off + 7, off + 11, off + 15)
    g_mix(v, off + 4, off + 8, off + 12, off + 16)
    g_mix(v, off + 1, off + 6, off + 11, off + 16)
    g_mix(v, off + 2, off + 7, off + 12, off + 13)
    g_mix(v, off + 3, off + 8, off + 9,  off + 14)
    g_mix(v, off + 4, off + 5, off + 10, off + 15)
end

-- Compression G(X, Y): first a row pass, then a column pass, with an
-- outer XOR feed-forward (the "Davies-Meyer flavour" that hardens G
-- against inversion).  X and Y are BLOCK_WORDS-length tables of u64.
local function compress(x, y)
    local r = {}
    for i = 1, BLOCK_WORDS do r[i] = x[i] ~ y[i] end
    local q = {}
    for i = 1, BLOCK_WORDS do q[i] = r[i] end

    for i = 0, 7 do
        permutation_p(q, i * 16)
    end

    local col = {}
    for c = 0, 7 do
        for rr = 0, 7 do
            col[2 * rr + 1] = q[rr * 16 + 2 * c + 1]
            col[2 * rr + 2] = q[rr * 16 + 2 * c + 2]
        end
        permutation_p(col, 0)
        for rr = 0, 7 do
            q[rr * 16 + 2 * c + 1] = col[2 * rr + 1]
            q[rr * 16 + 2 * c + 2] = col[2 * rr + 2]
        end
    end

    local out = {}
    for i = 1, BLOCK_WORDS do out[i] = r[i] ~ q[i] end
    return out
end

local function block_to_bytes(block)
    local parts = {}
    for i = 1, BLOCK_WORDS do
        parts[i] = string.pack("<I8", block[i])
    end
    return table.concat(parts)
end

local function bytes_to_block(data)
    local block = {}
    for i = 1, BLOCK_WORDS do
        block[i] = string.unpack("<I8", data, (i - 1) * 8 + 1)
    end
    return block
end

local function le32(n)
    return string.pack("<I4", n)
end

-- H' -- variable-length BLAKE2b extender (RFC 9106 §3.3).
-- Emits exactly `t` bytes.  For `t <= 64` it's a single BLAKE2b call
-- with digest_size=t; otherwise it chains 32-byte halves of 64-byte
-- BLAKE2b outputs, sizing the last call exactly.
local function blake2b_long(t, x)
    assert(t > 0, "H' output length must be positive")
    local input = le32(t) .. x

    if t <= 64 then
        return blake2b.digest(input, {digest_size = t})
    end

    local r = (t + 31) // 32 - 2
    local v = blake2b.digest(input, {digest_size = 64})
    local parts = {string.sub(v, 1, 32)}
    for _ = 1, r - 1 do
        v = blake2b.digest(v, {digest_size = 64})
        parts[#parts + 1] = string.sub(v, 1, 32)
    end
    local final_size = t - 32 * r
    v = blake2b.digest(v, {digest_size = final_size})
    parts[#parts + 1] = v
    return table.concat(parts)
end

-- indexAlpha -- map J1 to a reference-block column (RFC 9106 §3.4.1.1).
local function index_alpha(j1, r, sl, c, same_lane, q, sl_len)
    local w, start
    if r == 0 and sl == 0 then
        w = c - 1
        start = 0
    elseif r == 0 then
        if same_lane then
            w = sl * sl_len + c - 1
        elseif c == 0 then
            w = sl * sl_len - 1
        else
            w = sl * sl_len
        end
        start = 0
    else
        if same_lane then
            w = q - sl_len + c - 1
        elseif c == 0 then
            w = q - sl_len - 1
        else
            w = q - sl_len
        end
        start = ((sl + 1) * sl_len) % q
    end

    local x = (j1 * j1) >> 32
    local y = (w * x) >> 32
    local rel = w - 1 - y
    return (start + rel) % q
end

-- Fill one (pass, slice, lane) segment -- Argon2d's data-dependent
-- variant: J1/J2 come from the first word of the previously computed
-- block.
local function fill_segment(memory, r, lane, sl, q, sl_len, p)
    local starting_c = 0
    if r == 0 and sl == 0 then starting_c = 2 end

    for i = starting_c, sl_len - 1 do
        local col = sl * sl_len + i
        local prev_col
        if col == 0 then prev_col = q - 1 else prev_col = col - 1 end
        local prev_block = memory[lane][prev_col]

        local pseudo_rand = prev_block[1]
        local j1 = pseudo_rand & MASK32
        local j2 = (pseudo_rand >> 32) & MASK32

        local l_prime = lane
        if not (r == 0 and sl == 0) then
            l_prime = j2 % p
        end

        local z_prime = index_alpha(j1, r, sl, i, l_prime == lane, q, sl_len)
        local ref_block = memory[l_prime][z_prime]

        local new_block = compress(prev_block, ref_block)
        if r == 0 then
            memory[lane][col] = new_block
        else
            local existing = memory[lane][col]
            local xored = {}
            for k = 1, BLOCK_WORDS do xored[k] = existing[k] ~ new_block[k] end
            memory[lane][col] = xored
        end
    end
end

local function validate(password, salt, t, m, p, tag_length, key, ad, version)
    assert(#password <= MASK32, "password length must fit in 32 bits")
    assert(#salt >= 8, "salt must be at least 8 bytes")
    assert(#salt <= MASK32, "salt length must fit in 32 bits")
    assert(#key <= MASK32, "key length must fit in 32 bits")
    assert(#ad <= MASK32, "associated_data length must fit in 32 bits")
    assert(tag_length >= 4, "tag_length must be >= 4")
    assert(tag_length <= MASK32, "tag_length must fit in 32 bits")
    assert(type(p) == "number" and p == math.floor(p) and p >= 1 and p <= 0xFFFFFF,
           "parallelism must be in [1, 2^24-1]")
    assert(m >= 8 * p, "memory_cost must be >= 8*parallelism")
    assert(m <= MASK32, "memory_cost must fit in 32 bits")
    assert(t >= 1, "time_cost must be >= 1")
    assert(version == ARGON2_VERSION, "only Argon2 v1.3 (0x13) is supported")
end

-- argon2d(password, salt, t, m, p, T[, opts]) -- compute Argon2d tag.
--   password     (string)  secret input (bytes)
--   salt         (string)  >= 8 bytes, 16+ recommended
--   time_cost    (int)     passes (t), >= 1
--   memory_cost  (int)     KiB (m), >= 8*parallelism
--   parallelism  (int)     lanes (p), 1..2^24-1
--   tag_length   (int)     output bytes (T), >= 4
--   opts.key             optional MAC key (K), default ""
--   opts.associated_data optional context data (X), default ""
--   opts.version         only 0x13 supported
-- Returns T-byte binary tag.
function M.argon2d(password, salt, time_cost, memory_cost, parallelism, tag_length, opts)
    opts = opts or {}
    local key = opts.key or ""
    local ad = opts.associated_data or ""
    local version = opts.version or ARGON2_VERSION

    validate(password, salt, time_cost, memory_cost, parallelism, tag_length, key, ad, version)

    local p = parallelism
    local t = time_cost
    local segment_length = memory_cost // (SYNC_POINTS * p)
    local m_prime = segment_length * SYNC_POINTS * p
    local q = m_prime // p
    local sl_len = segment_length

    local h0_in = table.concat({
        le32(p), le32(tag_length), le32(memory_cost), le32(t),
        le32(version), le32(TYPE_D),
        le32(#password), password,
        le32(#salt), salt,
        le32(#key), key,
        le32(#ad), ad,
    })
    local h0 = blake2b.digest(h0_in, {digest_size = 64})

    local memory = {}
    for i = 0, p - 1 do
        memory[i] = {}
        local b0 = blake2b_long(BLOCK_SIZE, h0 .. le32(0) .. le32(i))
        local b1 = blake2b_long(BLOCK_SIZE, h0 .. le32(1) .. le32(i))
        memory[i][0] = bytes_to_block(b0)
        memory[i][1] = bytes_to_block(b1)
    end

    for r = 0, t - 1 do
        for sl = 0, SYNC_POINTS - 1 do
            for lane = 0, p - 1 do
                fill_segment(memory, r, lane, sl, q, sl_len, p)
            end
        end
    end

    local final_block = {}
    for k = 1, BLOCK_WORDS do final_block[k] = memory[0][q - 1][k] end
    for lane = 1, p - 1 do
        local lb = memory[lane][q - 1]
        for k = 1, BLOCK_WORDS do final_block[k] = final_block[k] ~ lb[k] end
    end

    return blake2b_long(tag_length, block_to_bytes(final_block))
end

-- argon2d_hex -- like argon2d() but returns lowercase hex.
function M.argon2d_hex(password, salt, time_cost, memory_cost, parallelism, tag_length, opts)
    local raw = M.argon2d(password, salt, time_cost, memory_cost, parallelism, tag_length, opts)
    local parts = {}
    for i = 1, #raw do parts[i] = string.format("%02x", string.byte(raw, i)) end
    return table.concat(parts)
end

return M
