-- ============================================================================
-- CodingAdventures.ZStd — Zstandard (RFC 8878) lossless compression — CMP07
-- ============================================================================
--
-- What Is ZStd?
-- -------------
--
-- Zstandard (ZStd) is a lossless compression algorithm designed by Yann Collet
-- at Facebook (2015) and standardised in RFC 8878. It combines two powerful
-- ideas:
--
--   1. LZ77 back-references (via LZSS token generation) to exploit repetition —
--      the "copy from earlier in the output" trick used by DEFLATE, but with a
--      32 KB window.
--
--   2. FSE (Finite State Entropy) coding for the sequence descriptor symbols.
--      FSE is an asymmetric numeral system (ANS) variant that approaches the
--      Shannon entropy limit in a single pass, beating Huffman coding on most
--      real data.
--
-- Frame Layout (RFC 8878 §3)
-- --------------------------
--
--   ┌────────┬─────┬──────────────────────┬────────┐
--   │ Magic  │ FHD │ Frame_Content_Size   │ Blocks │
--   │ 4 B LE │ 1 B │ 1/2/4/8 B (LE)      │ ...    │
--   └────────┴─────┴──────────────────────┴────────┘
--
-- Each block has a 3-byte header:
--   bit 0       = Last_Block flag
--   bits [2:1]  = Block_Type  (00=Raw, 01=RLE, 10=Compressed, 11=Reserved)
--   bits [23:3] = Block_Size
--
-- Compression Strategy
-- --------------------
--
--   1. Split data into 128 KB blocks (MAX_BLOCK_SIZE).
--   2. For each block, try:
--      a. RLE — all bytes identical → 4 bytes total (3 header + 1 data).
--      b. Compressed (LZ77 + FSE) — if output < input length.
--      c. Raw — verbatim copy as fallback.
--
-- Series
-- ------
--   CMP00 (LZ77)     — Sliding-window back-references
--   CMP01 (LZ78)     — Explicit dictionary (trie)
--   CMP02 (LZSS)     — LZ77 + flag bits
--   CMP03 (LZW)      — LZ78 + pre-initialised alphabet; GIF
--   CMP04 (Huffman)  — Entropy coding
--   CMP05 (DEFLATE)  — LZ77 + Huffman; ZIP/gzip/PNG/zlib
--   CMP06 (Brotli)   — DEFLATE + context modelling + static dict
--   CMP07 (ZStd)     — LZ77 + FSE; high ratio + speed  ← this module
--
-- ============================================================================

local lzss = require("coding_adventures.lzss")

local M = {}

M.VERSION = "0.1.0"

-- ============================================================================
-- Constants
-- ============================================================================

-- MAGIC is the ZStd frame identifier: 0xFD2FB528 stored little-endian.
-- The value was chosen to be unlikely to appear at the start of plaintext.
-- Wire bytes: 0x28, 0xB5, 0x2F, 0xFD.
local MAGIC = 0xFD2FB528

-- MAX_BLOCK_SIZE: ZStd allows blocks up to 128 KB. Larger inputs are split.
local MAX_BLOCK_SIZE = 128 * 1024

-- MAX_OUTPUT: decompression bomb guard — cap output at 256 MB.
local MAX_OUTPUT = 256 * 1024 * 1024

-- ============================================================================
-- LL / ML / OF code tables (RFC 8878 §3.1.1.3)
-- ============================================================================
--
-- These tables map a *code number* to a {baseline, extra_bits} pair.
--
-- For example, LL code 17 (1-indexed: LL_CODES[18]) means:
--   literal_length = 18 + read(1 extra bit), covering lengths 18 and 19.
--
-- The FSE state machine tracks one code number per field; extra bits are
-- read directly from the bitstream after state transitions.
--
-- NOTE: In Lua all tables are 1-indexed. Code 0 → index 1, code N → index N+1.

-- LL_CODES: Literal Length code table, codes 0..35 (indices 1..36).
-- Each entry is {baseline, extra_bits}.
-- Codes 0..15 have 0 extra bits (identity mapping).
-- Codes 16+ cover ranges of lengths with increasing bit widths.
local LL_CODES = {
    -- code 0..15: one symbol per code, 0 extra bits
    {0,0},{1,0},{2,0},{3,0},{4,0},{5,0},{6,0},{7,0},
    {8,0},{9,0},{10,0},{11,0},{12,0},{13,0},{14,0},{15,0},
    -- code 16+: grouped ranges
    {16,1},{18,1},{20,1},{22,1},
    {24,2},{28,2},
    {32,3},{40,3},
    {48,4},{64,6},
    {128,7},{256,8},{512,9},{1024,10},{2048,11},{4096,12},
    {8192,13},{16384,14},{32768,15},{65536,16},
}

-- ML_CODES: Match Length code table, codes 0..52 (indices 1..53).
-- Minimum match length in ZStd is 3. Code 0 = match length 3.
-- Codes 0..31: individual values 3..34 (0 extra bits each).
-- Codes 32+: grouped ranges with increasing widths.
local ML_CODES = {
    -- codes 0..31: individual match lengths 3..34
    {3,0},{4,0},{5,0},{6,0},{7,0},{8,0},{9,0},{10,0},
    {11,0},{12,0},{13,0},{14,0},{15,0},{16,0},{17,0},{18,0},
    {19,0},{20,0},{21,0},{22,0},{23,0},{24,0},{25,0},{26,0},
    {27,0},{28,0},{29,0},{30,0},{31,0},{32,0},{33,0},{34,0},
    -- codes 32+: grouped ranges
    {35,1},{37,1},{39,1},{41,1},
    {43,2},{47,2},
    {51,3},{59,3},
    {67,4},{83,4},
    {99,5},{131,7},
    {259,8},{515,9},{1027,10},{2051,11},
    {4099,12},{8195,13},{16387,14},{32771,15},{65539,16},
}

-- ============================================================================
-- FSE Predefined Distributions (RFC 8878 Appendix B)
-- ============================================================================
--
-- "Predefined_Mode" means no per-frame table description is transmitted.
-- The decoder builds the same table from these fixed distributions, so both
-- sides agree without any side-channel communication.
--
-- Entries of -1 mean probability = 1/table_size (very rare symbol). These
-- symbols each get exactly one slot in the decode table and their encoder
-- state never needs extra bits.
--
-- These distributions are specified in the RFC and MUST NOT be changed.

-- LL_NORM: normalised distribution for Literal Length FSE.
-- 36 entries (1-indexed), table_size = 2^6 = 64 slots.
local LL_NORM = {
     4, 3, 2, 2, 2, 2, 2, 2, 2, 2, 2, 2,
     2, 1, 1, 1, 2, 2, 2, 2, 2, 2, 2, 2,
     2, 3, 2, 1, 1, 1, 1, 1,
    -1,-1,-1,-1,
}
local LL_ACC_LOG = 6  -- table_size = 1 << 6 = 64

-- ML_NORM: normalised distribution for Match Length FSE.
-- 53 entries (1-indexed), table_size = 64 slots.
local ML_NORM = {
     1, 4, 3, 2, 2, 2, 2, 2, 2, 1, 1, 1, 1, 1, 1, 1,
     1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1,
     1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1,
    -1,-1,-1,-1,-1,-1,-1,
}
local ML_ACC_LOG = 6

-- OF_NORM: normalised distribution for Offset FSE.
-- 29 entries (1-indexed), table_size = 2^5 = 32 slots.
local OF_NORM = {
     1, 1, 1, 1, 1, 1, 2, 2, 2, 1, 1, 1, 1, 1, 1, 1,
     1, 1, 1, 1, 1, 1, 1, 1,
    -1,-1,-1,-1,-1,
}
local OF_ACC_LOG = 5  -- table_size = 1 << 5 = 32

-- ============================================================================
-- Integer utility: floor(log2(n)) for positive n
-- ============================================================================
--
-- Using iteration instead of math.log to avoid floating-point precision issues.
-- Example: floor_log2(4) = 2, floor_log2(5) = 2, floor_log2(8) = 3.

local function floor_log2(n)
    -- Precondition: n >= 1 (caller must ensure)
    local result = 0
    -- Shift right until n becomes 1, counting shifts.
    local tmp = n
    while tmp > 1 do
        tmp = tmp >> 1
        result = result + 1
    end
    return result
end

-- ============================================================================
-- FSE Decode Table Builder
-- ============================================================================
--
-- build_decode_table converts a normalised probability distribution (norm[])
-- into an FSE decode table. The table has sz = 2^acc_log entries (1-indexed).
-- Each entry is {sym, nb, base} where:
--   sym  = the decoded symbol (0-indexed symbol number)
--   nb   = number of extra bits to read from the bitstream for the next state
--   base = base value for next-state computation: new_state = base + read(nb)
--
-- Algorithm Overview
-- ------------------
--
-- Phase 1 — Probability -1 symbols go at the high end of the table.
--   These are the rarest symbols. They each get exactly 1 slot. The spec
--   places them at the top (highest indices) by convention.
--
-- Phase 2 — Spread remaining symbols using the step function.
--   The step size step = (sz >> 1) + (sz >> 3) + 3 is co-prime to sz
--   (which is always a power of 2), so the walk visits every remaining
--   slot exactly once. The two-pass approach (first symbols with count > 1,
--   then count == 1) matches the reference encoder ordering.
--
-- Phase 3 — Assign nb and base.
--   For symbol s appearing at table positions i₀, i₁, i₂, ...:
--     ns = sym_next[s] + j   (j = 0 for i₀, 1 for i₁, ...)
--     nb = acc_log - floor(log2(ns))
--     base = ns * (1 << nb) - sz
--   This ensures that when the decoder reads `nb` bits and adds `base`,
--   the result is a valid FSE state in [0, sz).

local function build_decode_table(norm, acc_log)
    local sz   = 1 << acc_log
    local step = (sz >> 1) + (sz >> 3) + 3

    -- Initialise all slots.
    local tbl = {}
    for i = 1, sz do
        tbl[i] = {sym = 0, nb = 0, base = 0}
    end

    -- sym_next[s] tracks how many times symbol s-1 (0-indexed) has appeared
    -- so far during Phase 2/3; used to compute nb and base in Phase 3.
    local sym_next = {}
    for i = 1, #norm do sym_next[i] = 0 end

    -- ── Phase 1: place -1 probability symbols at the high end ────────────
    -- Work downward from slot sz (1-indexed).
    local hi_limit = sz  -- highest free slot (1-indexed), also the upper bound for Phase 2
    for s = 1, #norm do
        if norm[s] == -1 then
            tbl[hi_limit].sym = s - 1  -- store 0-indexed symbol
            hi_limit = hi_limit - 1
            sym_next[s] = 1            -- will be one occurrence
        end
    end

    -- ── Phase 2: spread remaining symbols into slots 1..hi_limit ─────────
    -- Two-pass: first symbols with count > 1, then count == 1.
    -- This deterministic ordering matches the reference C implementation.
    local pos = 1  -- current insertion position (1-indexed)
    for pass = 0, 1 do
        for s = 1, #norm do
            local c = norm[s]
            if c > 0 then
                -- In pass 0: process only symbols with c > 1.
                -- In pass 1: process only symbols with c == 1.
                if (pass == 0) == (c > 1) then
                    sym_next[s] = c  -- will be refined in Phase 3
                    for _ = 1, c do
                        tbl[pos].sym = s - 1  -- 0-indexed symbol
                        -- Step forward, wrapping within [1, hi_limit].
                        pos = ((pos - 1 + step) % sz) + 1
                        while pos > hi_limit do
                            pos = ((pos - 1 + step) % sz) + 1
                        end
                    end
                end
            end
        end
    end

    -- ── Phase 3: compute nb (state bits) and base for each slot ──────────
    -- sn[s] starts at sym_next[s] (= count) and increments for each slot
    -- belonging to symbol s, in ascending table-index order.
    --
    -- For symbol s, if ns = sn[s] when we visit slot i:
    --   nb = acc_log - floor(log2(ns))
    --   base = ns * (1 << nb) - sz
    --
    -- Intuition: the encoder's state range is [sz, 2*sz). After emitting nb
    -- bits, the encoder's truncated state (state >> nb) indexes back into the
    -- cumulative-count table. The decoder reverses this by reading nb bits and
    -- adding `base`, landing in [0, sz).
    local sn = {}
    for i = 1, #norm do sn[i] = sym_next[i] end

    for i = 1, sz do
        local s  = tbl[i].sym      -- 0-indexed symbol
        local s1 = s + 1           -- 1-indexed into sn
        local ns = sn[s1]
        sn[s1] = sn[s1] + 1

        -- Guard: ns must be >= 1. A zero here means the norm table is invalid.
        assert(ns >= 1, "build_decode_table: sym_next underflow for symbol " .. s)

        local nb   = acc_log - floor_log2(ns)
        local base = (ns << nb) - sz
        tbl[i].nb   = nb
        tbl[i].base = base
    end

    return tbl
end

-- ============================================================================
-- FSE Encode Table Builder
-- ============================================================================
--
-- build_encode_sym returns two tables used during encoding:
--   ee[sym+1] = {delta_nb, delta_fs}  — encode transform for each symbol
--   st[slot+1] = output_state         — maps encoder slot to output state
--
-- The encoder maintains a state E in [sz, 2*sz). To encode symbol sym:
--   nb = (E + delta_nb) >> 16
--   emit the low nb bits of E to the backward bitstream
--   E  = st[(E >> nb) + delta_fs + 1]   (1-indexed lookup)
--
-- After all sequences, flush: write (E - sz) as acc_log bits, then sentinel.
--
-- Build Process
-- -------------
--
-- Step 1: Compute cumulative counts (like a CDF).
--   cumul[s] = sum of counts of symbols before s.
--   This defines the "slot" range [cumul[s], cumul[s]+count[s]) for symbol s.
--
-- Step 2: Build a spread table (same algorithm as the decode table).
--   spread[i] = the symbol assigned to decode-table slot i.
--
-- Step 3: Build the encoder state table st[].
--   Iterate i = 1..sz in order. For each i:
--     s = spread[i]
--     j = occurrence index of s (how many times s has appeared so far)
--     st[cumul[s] + j + 1] = i - 1 + sz   (output state = decode index + sz)
--   The encoder will later look up st[slot+1] to get the new state.
--
-- Step 4: Build FseEe entries.
--   For symbol s with count c:
--     max_bits_out mbo = acc_log - floor(log2(c))  (or acc_log if c == 1)
--     delta_nb = (mbo << 16) - (c << mbo)
--     delta_fs = cumul[s] - c

local function build_encode_sym(norm, acc_log)
    local sz   = 1 << acc_log
    local step = (sz >> 1) + (sz >> 3) + 3

    -- Step 1: cumulative counts.
    -- cumul[s] (0-indexed s) = number of slots before symbol s.
    local cumul = {}
    local total = 0
    for s = 0, #norm - 1 do
        cumul[s] = total
        local c = norm[s + 1]
        local cnt = (c == -1) and 1 or math.max(0, c)
        total = total + cnt
    end

    -- Step 2: build spread table (same deterministic spreading as decode).
    local spread = {}  -- spread[i+1] = symbol (0-indexed), for i in 0..sz-1
    for i = 0, sz - 1 do spread[i + 1] = 0 end

    local idx_high = sz  -- 1-indexed; Phase 1 fills from top downward
    for s = 0, #norm - 1 do
        if norm[s + 1] == -1 then
            spread[idx_high] = s
            idx_high = idx_high - 1
        end
    end
    local idx_limit = idx_high  -- highest free slot for Phase 2

    local pos = 1
    for pass = 0, 1 do
        for s = 0, #norm - 1 do
            local c = norm[s + 1]
            if c > 0 then
                local cnt = c
                if (pass == 0) == (cnt > 1) then
                    for _ = 1, cnt do
                        spread[pos] = s
                        pos = ((pos - 1 + step) % sz) + 1
                        while pos > idx_limit do
                            pos = ((pos - 1 + step) % sz) + 1
                        end
                    end
                end
            end
        end
    end

    -- Step 3: build state table st[].
    -- sym_occ[s] counts occurrences of symbol s seen so far (0-indexed s).
    local sym_occ = {}
    for s = 0, #norm - 1 do sym_occ[s] = 0 end

    local st = {}  -- st[slot+1] = output state (in [sz, 2*sz))
    for i = 0, sz - 1 do
        local s = spread[i + 1]  -- 0-indexed symbol
        local j = sym_occ[s]
        sym_occ[s] = sym_occ[s] + 1
        local slot = cumul[s] + j
        -- Encoder output state = decode-table index + sz.
        -- When the decoder is in state i, it decodes symbol s; the encoder
        -- must produce state i + sz so the decoder's next-state read is valid.
        st[slot + 1] = i + sz
    end

    -- Step 4: build FseEe entries.
    -- ee[sym+1] = {delta_nb, delta_fs}
    local ee = {}
    for s = 0, #norm - 1 do
        local c   = norm[s + 1]
        local cnt = (c == -1) and 1 or math.max(0, c)
        if cnt == 0 then
            ee[s + 1] = {delta_nb = 0, delta_fs = 0}
        else
            -- max_bits_out: if count == 1, mbo = acc_log;
            -- otherwise mbo = acc_log - floor(log2(cnt)).
            local mbo
            if cnt == 1 then
                mbo = acc_log
            else
                mbo = acc_log - floor_log2(cnt)
            end
            -- delta_nb stored as a shifted value for fast nb computation:
            --   nb = (state + delta_nb) >> 16
            -- The shift by 16 avoids a division in the hot encode loop.
            -- delta_nb = (mbo << 16) - (cnt << mbo)
            -- This works because state ∈ [sz, 2*sz), and (state + delta_nb) >> 16
            -- gives the correct bit count nb ∈ [mbo-1, mbo].
            local delta_nb = (mbo << 16) - (cnt << mbo)
            local delta_fs = cumul[s] - cnt
            ee[s + 1] = {delta_nb = delta_nb, delta_fs = delta_fs}
        end
    end

    return ee, st
end

-- ============================================================================
-- Reverse Bit-Writer
-- ============================================================================
--
-- ZStd's sequence bitstream is written *backwards* relative to the data flow:
-- the encoder writes bits that the decoder will read last, first. This allows
-- the decoder to initialise FSE states from the end of the stream and decode
-- sequences in forward order.
--
-- Byte layout: [byte₀, byte₁, ..., byteN] where byteN (the last written byte)
-- contains a sentinel bit marking the end of valid data.
--
-- Bit layout within each byte: LSB = first bit written (bit 0 = earliest).
--
-- Example: write bits 1,0,1,1 (4 bits) then flush:
--   reg = 0b1011, bits = 4
--   sentinel = 1 << 4 = 0b10000
--   last byte = 0b00011011 = 0x1B
--   buf = [0x1B]
--
-- The decoder reads: find MSB of last byte (bit 4 = sentinel), discard it,
-- then read bits 3..0 = 0b1011 = the original 4 bits.

local RevBitWriter = {}
RevBitWriter.__index = RevBitWriter

-- new creates a new RevBitWriter.
function RevBitWriter.new()
    return setmetatable({buf = {}, reg = 0, bits = 0}, RevBitWriter)
end

-- add_bits writes the low nb bits of val into the stream (LSB first).
-- Bytes are flushed whenever 8 or more bits have accumulated.
function RevBitWriter:add_bits(val, nb)
    if nb == 0 then return end
    -- Mask val to exactly nb bits. Handle nb == 64 specially to avoid
    -- (1 << 64) which overflows Lua's 64-bit integer range.
    local mask = (nb == 64) and (~0) or ((1 << nb) - 1)
    self.reg  = self.reg | ((val & mask) << self.bits)
    self.bits = self.bits + nb
    -- Flush complete bytes from the low end of reg.
    while self.bits >= 8 do
        self.buf[#self.buf + 1] = self.reg & 0xFF
        self.reg  = self.reg >> 8
        self.bits = self.bits - 8
    end
end

-- flush adds the sentinel bit and emits the final partial byte.
-- After flush, the stream is complete and can be read by RevBitReader.
function RevBitWriter:flush()
    -- The sentinel is a 1 placed at position `self.bits` in the last byte.
    -- All bits below it are valid data; bits above it are zero.
    -- Example: bits=3, reg=0b110 → sentinel=0b1000 → byte=0b00001110
    local sentinel = 1 << self.bits
    self.buf[#self.buf + 1] = (self.reg & 0xFF) | sentinel
    self.reg  = 0
    self.bits = 0
end

-- finish returns the accumulated byte array (array of integers 0-255).
function RevBitWriter:finish()
    return self.buf
end

-- ============================================================================
-- Reverse Bit-Reader
-- ============================================================================
--
-- RevBitReader mirrors RevBitWriter: reads bits from the END of the buffer
-- going backwards toward byte 0.
--
-- Register layout: valid bits are LEFT-ALIGNED (packed into the MSB side).
-- read_bits(n) extracts the top n bits and shifts the register left by n.
--
-- Why left-aligned? Because the writer accumulates bits LSB-first, the LAST
-- bit written within each byte has the highest bit position. Reading from the
-- top of a left-aligned register naturally gives the last-written bits first,
-- which mirrors the encoder's backward order.
--
-- Lua note: In Lua 5.4, integer `>>` is a LOGICAL right shift (fills with 0).
-- So `(-1) >> 1 == 0x7FFFFFFFFFFFFFFF`. This is what we want when extracting
-- bits from a register that may have bit 63 set.

local RevBitReader = {}
RevBitReader.__index = RevBitReader

-- new creates a RevBitReader from a byte array (array of integers 0-255).
--
-- Initialisation:
--   1. Find the sentinel bit in the last byte.
--   2. Extract the valid data bits below the sentinel and place them at the
--      TOP of the 64-bit shift register.
--   3. Call reload() to fill the register from earlier bytes.
function RevBitReader.new(bytes)
    local n = #bytes
    assert(n > 0, "RevBitReader: empty bitstream")

    local last = bytes[n]
    assert(last ~= 0, "RevBitReader: last byte is zero (no sentinel)")

    -- Find the position of the sentinel bit (the highest set bit in last).
    -- sentinel_pos is 0-indexed from LSB.
    -- Example: last = 0b00011110 → sentinel at bit 4 → valid_bits = 4.
    local sentinel_pos = 0
    while (1 << (sentinel_pos + 1)) <= last do
        sentinel_pos = sentinel_pos + 1
    end
    local valid_bits = sentinel_pos  -- number of data bits below the sentinel

    -- Place the valid bits of the sentinel byte at the top of the register.
    -- If valid_bits = 0, there are no data bits in the sentinel byte.
    local mask = (valid_bits > 0) and ((1 << valid_bits) - 1) or 0
    local reg
    if valid_bits == 0 then
        reg = 0
    else
        -- Shift the data bits to the MSB side of the 64-bit register.
        reg = (last & mask) << (64 - valid_bits)
    end

    local self = setmetatable({
        bytes = bytes,
        reg   = reg,
        bits  = valid_bits,
        pos   = n,   -- index of the last (sentinel) byte; reload reads from pos-1 downward
    }, RevBitReader)

    self:reload()
    return self
end

-- reload fills the register from earlier bytes, working backward.
-- New bytes are placed just BELOW the currently loaded bits.
-- We stop when bits > 56 (at least one full byte of head room remains).
function RevBitReader:reload()
    while self.bits <= 56 and self.pos > 1 do
        self.pos = self.pos - 1
        -- Compute the shift to place this byte just below existing bits.
        -- Current top `self.bits` bits are occupied (MSB-aligned); new byte
        -- goes at position 64 - self.bits - 8 (counting from MSB = bit 63).
        local shift = 64 - self.bits - 8
        self.reg  = self.reg | (self.bytes[self.pos] << shift)
        self.bits = self.bits + 8
    end
end

-- read_bits extracts the top nb bits from the register and returns them.
-- This returns the most recently written bits first (highest stream positions
-- first), mirroring the encoder's backward write order.
function RevBitReader:read_bits(nb)
    if nb == 0 then return 0 end
    -- Extract the top nb bits. Since >> is logical in Lua 5.4, this works
    -- even when reg has bit 63 set (negative as signed integer).
    local val = self.reg >> (64 - nb)
    -- Shift the register left to consume those bits.
    self.reg  = (nb == 64) and 0 or (self.reg << nb)
    self.bits = math.max(0, self.bits - nb)
    if self.bits < 24 then
        self:reload()
    end
    return val
end

-- ============================================================================
-- FSE encode/decode helpers
-- ============================================================================

-- fse_encode_sym encodes one symbol into the backward bitstream.
--
-- The encoder maintains state E in [sz, 2*sz). To emit symbol sym:
--   1. nb = (E + delta_nb) >> 16
--   2. Write the low nb bits of E to the bitstream.
--   3. E = st[(E >> nb) + delta_fs + 1]   (1-indexed)
--
-- This is called for each sequence field in REVERSE order (last seq first),
-- with the last symbol written being read first by the decoder.
local function fse_encode_sym(state, sym, ee, st)
    -- sym is 0-indexed; ee and st are 1-indexed.
    local e   = ee[sym + 1]
    local nb  = (state + e.delta_nb) >> 16
    -- Write the low `nb` bits of state to the bitstream (caller handles bw).
    -- We return nb and the bits to write separately so the caller can use bw:add_bits.
    local bits_out = state & ((nb == 64) and (~0) or ((1 << nb) - 1))
    local slot_i   = (state >> nb) + e.delta_fs
    -- slot_i is in [0, sz); index into st is slot_i + 1 (1-indexed).
    local new_state = st[slot_i + 1]
    assert(new_state ~= nil,
        "fse_encode_sym: slot_i=" .. slot_i .. " out of range (sym=" .. sym .. ")")
    return new_state, nb, bits_out
end

-- fse_decode_sym decodes one symbol and updates the FSE state.
--
-- Table entry tbl[state+1] provides:
--   sym  = the decoded symbol (0-indexed)
--   nb   = number of extra state bits to read
--   base = base value for next state: new_state = base + read(nb bits)
--
-- Returns the decoded symbol (0-indexed).
local function fse_decode_sym(state, tbl, br)
    -- state is 0-indexed; tbl is 1-indexed.
    local e      = tbl[state + 1]
    local sym    = e.sym
    local next   = e.base + br:read_bits(e.nb)
    return sym, next
end

-- ============================================================================
-- LL / ML / OF code number computation
-- ============================================================================

-- ll_to_code maps a literal length value to its LL code number (0..35).
-- Uses the last code whose baseline ≤ ll (linear scan; codes are sorted by baseline).
local function ll_to_code(ll)
    local code = 0
    for i = 1, #LL_CODES do
        if LL_CODES[i][1] <= ll then
            code = i - 1  -- 0-indexed code number
        else
            break
        end
    end
    return code
end

-- ml_to_code maps a match length value to its ML code number (0..52).
local function ml_to_code(ml)
    local code = 0
    for i = 1, #ML_CODES do
        if ML_CODES[i][1] <= ml then
            code = i - 1  -- 0-indexed code number
        else
            break
        end
    end
    return code
end

-- ============================================================================
-- Sequence conversion: LZSS tokens → ZStd sequences
-- ============================================================================
--
-- A ZStd sequence = (literal_length, match_length, match_offset).
-- Meaning: emit `ll` literal bytes from the literals section, then copy
-- `ml` bytes starting `off` bytes back in the output buffer.
--
-- LZSS produces alternating Literal and Match tokens. We accumulate consecutive
-- literals and emit a sequence when we hit a match. Trailing literals (after
-- the last match) go into the literals buffer without a sequence entry.

local function tokens_to_seqs(tokens)
    local lits    = {}   -- byte array of all literal bytes (1-indexed)
    local seqs    = {}   -- array of {ll, ml, off} (1-indexed)
    local lit_run = 0    -- count of literal bytes since the last match

    for _, tok in ipairs(tokens) do
        if tok.kind == "literal" then
            lits[#lits + 1] = tok.byte
            lit_run = lit_run + 1
        else
            -- Match token → emit sequence with accumulated literal run.
            seqs[#seqs + 1] = {
                ll  = lit_run,
                ml  = tok.length,
                off = tok.offset,
            }
            lit_run = 0
        end
    end

    -- Any remaining literals become trailing content (no sequence entry).
    return lits, seqs
end

-- ============================================================================
-- Literals Section Encoding/Decoding (Raw_Literals type)
-- ============================================================================
--
-- ZStd literals can be Huffman-coded or raw. We use Raw_Literals (type=0),
-- which stores bytes verbatim — simplest and still effective when LZSS has
-- extracted most of the redundancy.
--
-- Header format (RFC 8878 §3.1.1.2.1):
--   bits [1:0] = Literals_Block_Type (0 = Raw)
--   bits [3:2] = Size_Format:
--     00 or 10 → 1-byte header: size in bits [7:3] (5 bits, values 0..31)
--     01        → 2-byte LE header: size in bits [11:4] (12 bits, 0..4095)
--     11        → 3-byte LE header: size in bits [19:4] (20 bits, 0..1MB)

local function encode_literals_section(lits)
    -- lits: array of integers 0-255.
    local n    = #lits
    local out  = {}

    if n <= 31 then
        -- 1-byte header: size_format=00, type=00.
        -- Header byte = (n << 3) | 0b000.
        out[1] = n << 3
    elseif n <= 4095 then
        -- 2-byte LE header: size_format=01, type=00.
        -- 16-bit value = (n << 4) | 0b0100.
        local hdr = (n << 4) | 0x04
        out[1] = hdr & 0xFF
        out[2] = (hdr >> 8) & 0xFF
    else
        -- 3-byte LE header: size_format=11, type=00.
        -- 24-bit value = (n << 4) | 0b1100.
        local hdr = (n << 4) | 0x0C
        out[1] = hdr & 0xFF
        out[2] = (hdr >> 8) & 0xFF
        out[3] = (hdr >> 16) & 0xFF
    end

    local base = #out
    for i = 1, n do
        out[base + i] = lits[i]
    end
    return out
end

-- decode_literals_section parses the literals section.
-- Returns (lits_array, bytes_consumed) or calls error() on failure.
local function decode_literals_section(data, start)
    -- data: array of integers, start: 1-indexed position
    if start > #data then
        error("zstd: empty literals section")
    end

    local b0    = data[start]
    local ltype = b0 & 0x03  -- bottom 2 bits = Literals_Block_Type

    if ltype ~= 0 then
        error(string.format(
            "zstd: unsupported literals type %d (only Raw=0 supported)", ltype))
    end

    local size_format = (b0 >> 2) & 0x03  -- bits [3:2]
    local n, header_bytes

    if size_format == 0 or size_format == 2 then
        -- 1-byte header: size in bits [7:3]
        n            = b0 >> 3
        header_bytes = 1
    elseif size_format == 1 then
        -- 2-byte LE header: 12-bit size
        if start + 1 > #data then error("zstd: truncated literals header (2-byte)") end
        n            = (b0 >> 4) | (data[start + 1] << 4)
        header_bytes = 2
    else  -- size_format == 3
        -- 3-byte LE header: 20-bit size
        if start + 2 > #data then error("zstd: truncated literals header (3-byte)") end
        n            = (b0 >> 4) | (data[start + 1] << 4) | (data[start + 2] << 12)
        header_bytes = 3
    end

    local data_start = start + header_bytes
    local data_end   = data_start + n - 1

    if data_end > #data then
        error(string.format(
            "zstd: literals data truncated: need %d bytes, have %d",
            data_end, #data))
    end

    local lits = {}
    for i = 1, n do
        lits[i] = data[data_start + i - 1]
    end

    return lits, header_bytes + n
end

-- ============================================================================
-- Sequence Count Encoding/Decoding
-- ============================================================================
--
-- The sequence count is a 1-, 2-, or 3-byte encoding:
--   0..127:       1 byte  (byte 0 = count)
--   128..0x7FFE:  2 bytes LE, high bit of byte 0 set, value = u16 & 0x7FFF
--   0x7FFF+:      3 bytes, byte 0 = 0xFF, next 2 bytes LE = count - 0x7F00

local function encode_seq_count(count)
    if count == 0 then
        return {0}
    elseif count < 128 then
        return {count}
    elseif count < 0x7FFF then
        local v = count | 0x8000
        return {v & 0xFF, (v >> 8) & 0xFF}
    else
        local r = count - 0x7F00
        return {0xFF, r & 0xFF, (r >> 8) & 0xFF}
    end
end

-- decode_seq_count returns (count, bytes_consumed).
local function decode_seq_count(data, start)
    if start > #data then error("zstd: empty sequence count") end
    local b0 = data[start]
    if b0 < 128 then
        return b0, 1
    elseif b0 < 0xFF then
        if start + 1 > #data then error("zstd: truncated sequence count (2-byte)") end
        local v     = b0 | (data[start + 1] << 8)
        local count = v & 0x7FFF
        return count, 2
    else
        if start + 2 > #data then error("zstd: truncated sequence count (3-byte)") end
        local count = 0x7F00 + data[start + 1] + (data[start + 2] << 8)
        return count, 3
    end
end

-- ============================================================================
-- Sequences Section Encoding
-- ============================================================================
--
-- Layout:
--   [sequence_count: 1-3 bytes]
--   [symbol_compression_modes: 1 byte]  → 0x00 = all Predefined
--   [FSE bitstream: backward bit-stream]
--
-- Symbol compression modes byte:
--   bits [7:6] = LL mode  (0 = Predefined)
--   bits [5:4] = OF mode  (0 = Predefined)
--   bits [3:2] = ML mode  (0 = Predefined)
--   bits [1:0] = reserved (0)
--
-- FSE Bitstream Layout (written in REVERSE sequence order):
--   For each sequence (last→first):
--     write OF extra bits, ML extra bits, LL extra bits
--     FSE encode OF symbol, ML symbol, LL symbol
--   After all sequences:
--     flush initial states: (state_of - sz_of) as OF_ACC_LOG bits
--                           (state_ml - sz_ml) as ML_ACC_LOG bits
--                           (state_ll - sz_ll) as LL_ACC_LOG bits
--   Add sentinel and flush.
--
-- The decoder reads the same stream in the natural (left-to-right) order
-- because the backward bit-stream reverses the write order.

local function encode_sequences_section(seqs)
    -- Build encode tables from the predefined distributions.
    local ee_ll, st_ll = build_encode_sym(LL_NORM, LL_ACC_LOG)
    local ee_ml, st_ml = build_encode_sym(ML_NORM, ML_ACC_LOG)
    local ee_of, st_of = build_encode_sym(OF_NORM, OF_ACC_LOG)

    local sz_ll = 1 << LL_ACC_LOG  -- 64
    local sz_ml = 1 << ML_ACC_LOG  -- 64
    local sz_of = 1 << OF_ACC_LOG  -- 32

    -- FSE encoder states start at table_size (= sz). Valid range: [sz, 2*sz).
    local state_ll = sz_ll
    local state_ml = sz_ml
    local state_of = sz_of

    local bw = RevBitWriter.new()

    -- Encode sequences in REVERSE order so the decoder sees them in forward order.
    for i = #seqs, 1, -1 do
        local seq = seqs[i]

        local ll_code = ll_to_code(seq.ll)
        local ml_code = ml_to_code(seq.ml)

        -- Offset encoding (RFC 8878 §3.1.1.3.2.1):
        --   raw = offset + 3   (adjusts for ZStd's "repeat offsets" base)
        --   of_code = floor(log2(raw))
        --   of_extra = raw - (1 << of_code)   (the fractional part)
        -- This is essentially a power-of-2 prefix code for the offset.
        local raw_off = seq.off + 3
        local of_code
        if raw_off <= 1 then
            of_code = 0
        else
            of_code = floor_log2(raw_off)
        end
        local of_extra = raw_off - (1 << of_code)

        -- Write extra bits in the order: OF extras, ML extras, LL extras.
        -- (The backward stream will reverse this for the decoder.)
        bw:add_bits(of_extra, of_code)
        local ml_extra = seq.ml - ML_CODES[ml_code + 1][1]
        bw:add_bits(ml_extra, ML_CODES[ml_code + 1][2])
        local ll_extra = seq.ll - LL_CODES[ll_code + 1][1]
        bw:add_bits(ll_extra, LL_CODES[ll_code + 1][2])

        -- FSE encode symbols in REVERSE decode order.
        -- Decode order: LL, OF, ML  (state transitions happen in this order).
        -- Encode order (reversed): ML, OF, LL  (LL is last written → first read).
        --
        -- We call fse_encode_sym which returns the new state plus bits to write.
        local new_state_ml, nb_ml, bits_ml = fse_encode_sym(state_ml, ml_code, ee_ml, st_ml)
        bw:add_bits(bits_ml, nb_ml)
        state_ml = new_state_ml

        local new_state_of, nb_of, bits_of = fse_encode_sym(state_of, of_code, ee_of, st_of)
        bw:add_bits(bits_of, nb_of)
        state_of = new_state_of

        local new_state_ll, nb_ll, bits_ll = fse_encode_sym(state_ll, ll_code, ee_ll, st_ll)
        bw:add_bits(bits_ll, nb_ll)
        state_ll = new_state_ll
    end

    -- Write the initial FSE states so the decoder can initialise.
    -- Write order: OF, ML, LL — decoder reads LL first, then ML, then OF.
    bw:add_bits(state_of - sz_of, OF_ACC_LOG)
    bw:add_bits(state_ml - sz_ml, ML_ACC_LOG)
    bw:add_bits(state_ll - sz_ll, LL_ACC_LOG)
    bw:flush()

    return bw:finish()  -- returns byte array
end

-- ============================================================================
-- Block-level Compress
-- ============================================================================
--
-- compress_block tries to compress one block using LZ77 + FSE.
-- Returns a byte array if compressed < original, or nil (use raw/RLE instead).

local function compress_block(block)
    -- block: array of integers 0-255

    -- Run LZSS with a 32 KB window, max match 255, min match 3.
    -- lzss.encode expects a 1-indexed integer byte array.
    local tokens = lzss.encode(block, 32768, 255, 3)

    -- Convert LZSS tokens to ZStd sequences.
    local lits, seqs = tokens_to_seqs(tokens)

    -- If LZ77 found no matches, there's nothing to encode in the FSE bitstream.
    -- A compressed block with 0 sequences still has header overhead, so fall back.
    if #seqs == 0 then
        return nil
    end

    local out = {}

    -- 1. Literals section (Raw_Literals).
    local lit_section = encode_literals_section(lits)
    for _, b in ipairs(lit_section) do
        out[#out + 1] = b
    end

    -- 2. Sequence count.
    local sc_bytes = encode_seq_count(#seqs)
    for _, b in ipairs(sc_bytes) do
        out[#out + 1] = b
    end

    -- 3. Symbol compression modes byte: 0x00 = all Predefined.
    out[#out + 1] = 0x00

    -- 4. FSE bitstream.
    local bitstream = encode_sequences_section(seqs)
    for _, b in ipairs(bitstream) do
        out[#out + 1] = b
    end

    if #out >= #block then
        return nil  -- compression not beneficial
    end

    return out
end

-- ============================================================================
-- Block-level Decompress
-- ============================================================================
--
-- decompress_block decodes one ZStd compressed block and appends output bytes
-- to the `out` array. Calls error() on any format violation.

local function decompress_block(data, start, bsize, out)
    -- data:  full frame byte array (integers 0-255)
    -- start: 1-indexed start of this block's payload within data
    -- bsize: number of bytes in the block payload
    -- out:   output byte array (appended in-place)

    local block_end = start + bsize - 1

    -- ── Literals section ─────────────────────────────────────────────────
    local lits, lit_consumed = decode_literals_section(data, start)
    local pos = start + lit_consumed  -- position of next unread byte

    -- ── Sequence count ────────────────────────────────────────────────────
    if pos > block_end then
        -- Block contains only literals (no sequences section).
        for _, b in ipairs(lits) do out[#out + 1] = b end
        return
    end

    local n_seqs, sc_bytes = decode_seq_count(data, pos)
    pos = pos + sc_bytes

    if n_seqs == 0 then
        -- No sequences; output is just literals.
        for _, b in ipairs(lits) do out[#out + 1] = b end
        return
    end

    -- ── Symbol compression modes byte ─────────────────────────────────────
    if pos > block_end then
        error("zstd: missing symbol compression modes byte")
    end
    local modes_byte = data[pos]
    pos = pos + 1

    local ll_mode = (modes_byte >> 6) & 3
    local of_mode = (modes_byte >> 4) & 3
    local ml_mode = (modes_byte >> 2) & 3
    if ll_mode ~= 0 or of_mode ~= 0 or ml_mode ~= 0 then
        error(string.format(
            "zstd: unsupported FSE modes LL=%d OF=%d ML=%d (only Predefined=0 supported)",
            ll_mode, of_mode, ml_mode))
    end

    -- ── FSE bitstream ─────────────────────────────────────────────────────
    -- Extract the bitstream bytes into a fresh array for RevBitReader.
    local bs_len = block_end - pos + 1
    if bs_len <= 0 then
        error("zstd: missing FSE bitstream")
    end
    local bs_bytes = {}
    for i = 1, bs_len do
        bs_bytes[i] = data[pos + i - 1]
    end

    local br = RevBitReader.new(bs_bytes)

    -- Build predefined decode tables.
    local dt_ll = build_decode_table(LL_NORM, LL_ACC_LOG)
    local dt_ml = build_decode_table(ML_NORM, ML_ACC_LOG)
    local dt_of = build_decode_table(OF_NORM, OF_ACC_LOG)

    -- Initialise FSE states from the bitstream.
    -- The encoder wrote: (state_ll - sz_ll), (state_ml - sz_ml), (state_of - sz_of).
    -- The decoder reads them in the same order (the bitstream reverses write order).
    local state_ll = br:read_bits(LL_ACC_LOG)
    local state_ml = br:read_bits(ML_ACC_LOG)
    local state_of = br:read_bits(OF_ACC_LOG)

    -- Track position within the literals buffer (lits[1..]).
    local lit_pos = 1

    -- ── Apply each sequence ────────────────────────────────────────────────
    for _ = 1, n_seqs do
        -- Step 1: Decode symbols (state transitions).
        -- Decode order: LL → OF → ML.
        local ll_code, next_ll = fse_decode_sym(state_ll, dt_ll, br)
        local of_code, next_of = fse_decode_sym(state_of, dt_of, br)
        local ml_code, next_ml = fse_decode_sym(state_ml, dt_ml, br)
        state_ll = next_ll
        state_of = next_of
        state_ml = next_ml

        -- Step 2: Read extra bits to recover the exact field values.
        if ll_code >= #LL_CODES then
            error("zstd: invalid LL code " .. ll_code)
        end
        if ml_code >= #ML_CODES then
            error("zstd: invalid ML code " .. ml_code)
        end
        -- of_code drives both br:read_bits(of_code) and a left-shift of 1.
        -- Values above 28 would shift by ≥ 29 bits, which can overflow Lua's
        -- 64-bit integers and produce a garbage offset. RFC 8878 Table 9 only
        -- defines of_codes 0..28 in the predefined table.
        if of_code > 28 then
            error("zstd: of_code out of range: " .. of_code)
        end

        local ll_info = LL_CODES[ll_code + 1]  -- 1-indexed
        local ml_info = ML_CODES[ml_code + 1]

        local ll = ll_info[1] + br:read_bits(ll_info[2])
        local ml = ml_info[1] + br:read_bits(ml_info[2])

        -- Offset: raw = (1 << of_code) | extra_bits; offset = raw - 3.
        -- The "- 3" adjusts for ZStd's repeat-offset encoding baseline.
        local of_raw = (1 << of_code) | br:read_bits(of_code)
        if of_raw < 3 then
            error(string.format(
                "zstd: decoded offset underflow: of_raw=%d (of_code=%d)", of_raw, of_code))
        end
        local offset = of_raw - 3

        -- Step 3: Emit `ll` literal bytes.
        local lit_end = lit_pos + ll - 1
        if lit_end > #lits then
            error(string.format(
                "zstd: literal run %d overflows literals buffer (pos=%d, len=%d)",
                ll, lit_pos, #lits))
        end
        -- Guard before expanding the literal run: a crafted sequence could use a
        -- huge ll value to inflate output beyond MAX_OUTPUT.
        if #out + ll > MAX_OUTPUT then
            error("zstd: decompressed size exceeds limit of " .. MAX_OUTPUT .. " bytes")
        end
        for i = lit_pos, lit_end do
            out[#out + 1] = lits[i]
        end
        lit_pos = lit_end + 1

        -- Step 4: Copy `ml` bytes from `offset` back in the output buffer.
        -- Offset is 1-indexed from the END of the current output.
        -- Copy byte-by-byte to correctly handle overlapping matches
        -- (e.g., offset=1, ml=4 on [65] produces [65,65,65,65]).
        if offset == 0 or offset > #out then
            error(string.format(
                "zstd: bad match offset %d (output len %d)", offset, #out))
        end
        if #out + ml > MAX_OUTPUT then
            error("zstd: decompressed size exceeds limit of " .. MAX_OUTPUT .. " bytes")
        end
        local copy_start = #out - offset + 1
        for i = 0, ml - 1 do
            out[#out + 1] = out[copy_start + i]
        end
    end

    -- Step 5: Emit any remaining literals after the last sequence.
    -- Guard against decompression bombs: a crafted block could have a huge
    -- trailing-literals section. Check per byte to catch the limit exactly.
    for i = lit_pos, #lits do
        if #out >= MAX_OUTPUT then
            error("zstd: decompressed size exceeds limit of " .. MAX_OUTPUT .. " bytes")
        end
        out[#out + 1] = lits[i]
    end
end

-- ============================================================================
-- Public API
-- ============================================================================

-- Helper: convert a string or byte array to a byte array (integers 0-255).
local function to_byte_array(data)
    if type(data) == "string" then
        return {data:byte(1, #data)}
    end
    return data
end

-- Helper: convert a byte array to a Lua string.
local function to_string(bytes)
    if #bytes == 0 then return "" end
    -- table.unpack has a stack-size limit; chunk for large outputs.
    local parts = {}
    local i = 1
    while i <= #bytes do
        local j = math.min(i + 4095, #bytes)
        parts[#parts + 1] = string.char(table.unpack(bytes, i, j))
        i = j + 1
    end
    return table.concat(parts)
end

-- compress compresses `data` to ZStd format (RFC 8878).
--
-- Input: a Lua string or an array of integers (0-255).
-- Output: a Lua string containing the ZStd frame.
--
-- The output is a valid ZStd frame that can be decompressed by the `zstd`
-- command-line tool or any conforming ZStd implementation.
--
-- Example:
--   local zstd = require("coding_adventures.zstd")
--   local compressed = zstd.compress("the quick brown fox " .. string.rep("!",100))
--   local original   = zstd.decompress(compressed)
function M.compress(data)
    data = to_byte_array(data)
    local out = {}  -- byte array for the output frame

    -- ── ZStd Frame Header ─────────────────────────────────────────────────

    -- Magic number: 0xFD2FB528 in little-endian order.
    -- Wire: 0x28, 0xB5, 0x2F, 0xFD
    out[1] = 0x28
    out[2] = 0xB5
    out[3] = 0x2F
    out[4] = 0xFD

    -- Frame Header Descriptor (FHD):
    --   bits [7:6] = FCS_Field_Size flag:  11 → 8-byte Frame_Content_Size
    --   bit  [5]   = Single_Segment_Flag:   1 → Window_Descriptor omitted
    --   bit  [4]   = Content_Checksum_Flag: 0 → no checksum
    --   bits [3:2] = reserved:              0
    --   bits [1:0] = Dict_ID_Flag:          0 → no dictionary ID
    -- → 0b1110_0000 = 0xE0
    out[5] = 0xE0

    -- Frame_Content_Size (8 bytes LE): the uncompressed byte count.
    -- A decoder uses this to pre-allocate the output buffer efficiently.
    local fcs = #data
    out[6]  = fcs & 0xFF
    out[7]  = (fcs >> 8)  & 0xFF
    out[8]  = (fcs >> 16) & 0xFF
    out[9]  = (fcs >> 24) & 0xFF
    out[10] = (fcs >> 32) & 0xFF
    out[11] = (fcs >> 40) & 0xFF
    out[12] = (fcs >> 48) & 0xFF
    out[13] = (fcs >> 56) & 0xFF

    -- ── Blocks ────────────────────────────────────────────────────────────

    if #data == 0 then
        -- Empty input: emit one empty Raw block.
        -- Block header: Last=1, Type=Raw(00), Size=0
        -- 3-byte LE: bits [0] = Last, bits [2:1] = Type, bits [23:3] = Size
        -- = (0 << 3) | (0 << 1) | 1 = 0x01, 0x00, 0x00
        out[#out + 1] = 0x01
        out[#out + 1] = 0x00
        out[#out + 1] = 0x00
        return to_string(out)
    end

    local offset = 1  -- 1-indexed cursor into data
    while offset <= #data do
        local block_end = math.min(offset + MAX_BLOCK_SIZE - 1, #data)
        local is_last   = (block_end == #data)
        local last_bit  = is_last and 1 or 0

        -- Slice the block.
        local block = {}
        for i = offset, block_end do
            block[#block + 1] = data[i]
        end
        local blen = #block

        -- ── Try RLE block ─────────────────────────────────────────────────
        -- If every byte is identical, encode as a single-byte RLE block.
        -- Header: (blen << 3) | (0b01 << 1) | last_bit
        -- Payload: 1 byte (the repeated value).
        local all_same = true
        local first_byte = block[1]
        for i = 2, blen do
            if block[i] ~= first_byte then
                all_same = false
                break
            end
        end

        if all_same then
            -- RLE block: Last|Type=01|Size
            local hdr = (blen << 3) | (0x02) | last_bit  -- (1 << 1) | last = 0b10 | last
            out[#out + 1] = hdr & 0xFF
            out[#out + 1] = (hdr >> 8) & 0xFF
            out[#out + 1] = (hdr >> 16) & 0xFF
            out[#out + 1] = first_byte
        else
            -- ── Try Compressed block ──────────────────────────────────────
            local compressed = compress_block(block)
            if compressed ~= nil then
                -- Compressed block: Last|Type=10|Size
                local clen = #compressed
                local hdr  = (clen << 3) | 0x04 | last_bit  -- (2 << 1) | last = 0b100 | last
                out[#out + 1] = hdr & 0xFF
                out[#out + 1] = (hdr >> 8) & 0xFF
                out[#out + 1] = (hdr >> 16) & 0xFF
                for _, b in ipairs(compressed) do
                    out[#out + 1] = b
                end
            else
                -- ── Raw block (fallback) ──────────────────────────────────
                -- Raw block: Last|Type=00|Size
                local hdr = (blen << 3) | 0x00 | last_bit
                out[#out + 1] = hdr & 0xFF
                out[#out + 1] = (hdr >> 8) & 0xFF
                out[#out + 1] = (hdr >> 16) & 0xFF
                for _, b in ipairs(block) do
                    out[#out + 1] = b
                end
            end
        end

        offset = block_end + 1
    end

    return to_string(out)
end

-- decompress decompresses a ZStd frame and returns the original data.
--
-- Input: a Lua string or an array of integers (0-255) containing a ZStd frame.
-- Output: a Lua string of the original uncompressed bytes.
--
-- Errors: calls error() if the frame is malformed, truncated, or uses
-- unsupported features (non-predefined FSE tables, Huffman literals, etc.).
--
-- Example:
--   local zstd = require("coding_adventures.zstd")
--   local original = zstd.decompress(zstd.compress("hello world"))
--   assert(original == "hello world")
function M.decompress(data)
    data = to_byte_array(data)

    if #data < 5 then
        error("zstd: frame too short (" .. #data .. " bytes)")
    end

    -- ── Validate magic number ─────────────────────────────────────────────
    -- ZStd magic: 0xFD2FB528 as 4 bytes LE = [0x28, 0xB5, 0x2F, 0xFD].
    local magic = data[1] | (data[2] << 8) | (data[3] << 16) | (data[4] << 24)
    -- Lua integers are 64-bit signed; a 32-bit value with high bit set arrives
    -- as a positive integer (no sign extension for | and <<).
    -- However, MAGIC = 0xFD2FB528 has bit 31 set. In Lua 5.4:
    --   0xFD000000 has bit 31 set → value is 4244635648 (positive 64-bit).
    -- So comparison with MAGIC works correctly.
    if magic ~= MAGIC then
        error(string.format(
            "zstd: bad magic 0x%08X (expected 0x%08X)", magic, MAGIC))
    end

    local pos = 5  -- 1-indexed cursor (next byte to read)

    -- ── Frame Header Descriptor ───────────────────────────────────────────
    local fhd = data[pos]
    pos = pos + 1

    -- bits [7:6]: FCS_Field_Size
    --   00 → 0 bytes (unless Single_Segment=1, then 1 byte)
    --   01 → 2 bytes (value + 256)
    --   10 → 4 bytes
    --   11 → 8 bytes
    local fcs_flag = (fhd >> 6) & 3

    -- bit [5]: Single_Segment_Flag — when set, Window_Descriptor is omitted.
    local single_seg = (fhd >> 5) & 1

    -- bit [4]: Content_Checksum_Flag — we skip validation but note its presence.
    -- (unused in this decoder)

    -- bits [1:0]: Dict_ID_Flag — number of dict-ID bytes (0, 1, 2, or 4).
    local dict_flag = fhd & 3

    -- ── Window Descriptor ─────────────────────────────────────────────────
    -- Present only when Single_Segment_Flag = 0.
    if single_seg == 0 then
        pos = pos + 1  -- skip 1-byte Window_Descriptor (we don't enforce limits)
    end

    -- ── Dict ID ───────────────────────────────────────────────────────────
    -- dict_flag: 0→0 bytes, 1→1 byte, 2→2 bytes, 3→4 bytes.
    local dict_id_bytes_table = {0, 1, 2, 4}  -- 1-indexed by dict_flag+1
    local dict_id_bytes = dict_id_bytes_table[dict_flag + 1]
    pos = pos + dict_id_bytes  -- skip dict ID (we don't support custom dicts)

    -- ── Frame_Content_Size ────────────────────────────────────────────────
    -- We skip the FCS value (we trust the blocks for correctness).
    local fcs_bytes
    if fcs_flag == 0 then
        fcs_bytes = (single_seg == 1) and 1 or 0
    elseif fcs_flag == 1 then
        fcs_bytes = 2
    elseif fcs_flag == 2 then
        fcs_bytes = 4
    else  -- fcs_flag == 3
        fcs_bytes = 8
    end
    pos = pos + fcs_bytes  -- skip FCS

    -- ── Blocks ────────────────────────────────────────────────────────────
    local out = {}  -- output byte array, accumulated in-place

    while true do
        -- Each block begins with a 3-byte little-endian header.
        if pos + 2 > #data then
            error("zstd: truncated block header at pos " .. pos)
        end

        local hdr   = data[pos] | (data[pos+1] << 8) | (data[pos+2] << 16)
        pos = pos + 3

        local last  = (hdr & 1) ~= 0
        local btype = (hdr >> 1) & 3
        local bsize = (hdr >> 3)

        if btype == 0 then
            -- ── Raw block: `bsize` bytes of verbatim content ──────────────
            if pos + bsize - 1 > #data then
                error(string.format(
                    "zstd: raw block truncated: need %d bytes at pos %d (data len %d)",
                    bsize, pos, #data))
            end
            if #out + bsize > MAX_OUTPUT then
                error("zstd: decompressed size exceeds limit of " .. MAX_OUTPUT)
            end
            for i = pos, pos + bsize - 1 do
                out[#out + 1] = data[i]
            end
            pos = pos + bsize

        elseif btype == 1 then
            -- ── RLE block: 1 byte repeated `bsize` times ─────────────────
            if pos > #data then
                error("zstd: RLE block missing byte at pos " .. pos)
            end
            if #out + bsize > MAX_OUTPUT then
                error("zstd: decompressed size exceeds limit of " .. MAX_OUTPUT)
            end
            local rle_byte = data[pos]
            pos = pos + 1
            for _ = 1, bsize do
                out[#out + 1] = rle_byte
            end

        elseif btype == 2 then
            -- ── Compressed block ──────────────────────────────────────────
            if pos + bsize - 1 > #data then
                error(string.format(
                    "zstd: compressed block truncated: need %d bytes at pos %d",
                    bsize, pos))
            end
            decompress_block(data, pos, bsize, out)
            pos = pos + bsize

        else  -- btype == 3
            error("zstd: reserved block type 3")
        end

        if last then break end
    end

    return to_string(out)
end

return M
