-- ============================================================================
-- CodingAdventures.HuffmanCompression
-- ============================================================================
--
-- CMP04: Huffman Compression — Entropy coding (1952)
-- Part of the CMP compression series in the coding-adventures monorepo.
--
-- What Is Huffman Compression?
-- ----------------------------
--
-- Huffman compression assigns shorter bit codes to symbols that appear more
-- frequently and longer codes to symbols that appear rarely. The trick is that
-- these codes are *prefix-free*: no code is a prefix of another, so the
-- decoder never needs separator characters — it just walks a binary tree.
--
-- For example, with input "AAABBC":
--   A appears 3 times → code "0"   (1 bit)
--   B appears 2 times → code "10"  (2 bits)
--   C appears 1 time  → code "11"  (2 bits)
--
-- Original: 6 bytes × 8 bits = 48 bits.
-- Encoded:  3×1 + 2×2 + 1×2 = 9 bits → padded to 2 bytes.
-- (Plus header overhead for portability.)
--
-- The key insight: common symbols get short codes, rare symbols get long codes.
-- Entropy theory proves this assignment is *optimal* for the given frequencies.
--
-- CMP04 Wire Format
-- -----------------
--
-- The wire format is designed to be self-contained: the decoder only needs the
-- compressed bytes to reconstruct the original data without any side channel.
--
--   Bytes 0–3:    original_length  (big-endian uint32)
--                 How many bytes to produce during decompression.
--
--   Bytes 4–7:    symbol_count     (big-endian uint32)
--                 Number of distinct byte values (N) in the original data.
--                 This tells the decoder how many entries follow in the table.
--
--   Bytes 8–8+2N: code-lengths table — N entries, each 2 bytes:
--                   [0] symbol value  (uint8, 0–255)
--                   [1] code length   (uint8, 1–16)
--                 Sorted by (code_length, symbol_value) ascending.
--                 This sort order is what allows canonical code reconstruction.
--
--   Bytes 8+2N+:  bit stream — packed LSB-first, zero-padded to byte boundary.
--                 Contains one code per original byte, in order.
--
-- Why Canonical Codes?
-- --------------------
--
-- A standard Huffman tree can produce different code bit-strings for the same
-- set of lengths (the tree shape is not unique). Canonical codes normalise this:
-- given only the length table, the decoder can deterministically reconstruct
-- the exact same codes the encoder used.
--
-- The canonical assignment rule (DEFLATE / zlib style):
--   1. Sort symbols by (length, symbol_value) — shorter lengths first.
--   2. First code = 0 (padded to its length).
--   3. Each subsequent code = previous code + 1, with a left-shift when
--      the length increases: code = (prev_code + 1) << (new_len - prev_len).
--
-- Example with AAABBC (lengths: A=1, B=2, C=2):
--   Sorted: [(A,1), (B,2), (C,2)]
--   A: code=0, len=1  → "0"
--   B: code=(0+1)<<(2-1)=2, len=2 → "10"
--   C: code=2+1=3, len=2 → "11"
--
-- Bit Packing: LSB-First
-- -----------------------
--
-- Bits are packed LSB-first (least significant bit first) inside each byte.
-- This is the same convention used by LZW (CMP03), DEFLATE, and PNG.
--
-- Example: encoding "A" (code "0"), "B" (code "10"), "B" (code "10")
--   Bit string:  "0" + "10" + "10" = "01010"  (5 bits)
--   Pack LSB-first into byte:
--     bit 0 → position 0 of byte 0: 0
--     bit 1 → position 1 of byte 0: 1
--     bit 2 → position 2 of byte 0: 0
--     bit 3 → position 3 of byte 0: 1
--     bit 4 → position 4 of byte 0: 0
--     → byte 0 = 0b00001010 = 0x0A, padded to 1 byte.
--
-- Wait — but the codes are MSB-first strings like "10". To pack them LSB-first
-- we write them bit by bit: '1' then '0'. So the first bit of "10" (which is
-- '1') goes into the lowest available bit position. This is correct: the
-- decoder reads back the bits in the same order using the same LSB-first scheme.
--
-- The Series: CMP00 → CMP05
-- --------------------------
--
-- CMP00 (LZ77,     1977) — Sliding-window backreferences.
-- CMP01 (LZ78,     1978) — Explicit dictionary (trie).
-- CMP02 (LZSS,     1982) — LZ77 + flag bits; eliminates wasted literals.
-- CMP03 (LZW,      1984) — Pre-initialised dictionary; powers GIF.
-- CMP04 (Huffman,  1952) — Entropy coding; prerequisite for DEFLATE. This module.
-- CMP05 (DEFLATE,  1996) — LZ77 + Huffman; ZIP/gzip/PNG/zlib standard.
-- ============================================================================

local M = {}

M.VERSION = "0.1.1"

-- HuffmanTree is our dependency (DT27). We rely on:
--   HuffmanTree.build({{sym, freq}, ...})  → tree
--   tree:canonical_code_table()            → {sym → bit_string}
--   tree:decode_all(bits, count)           → {sym, sym, ...}
local HuffmanTree = require("coding_adventures.huffman_tree")

-- ── Big-Endian uint32 helpers ─────────────────────────────────────────────────

-- pack_u32_be serialises an unsigned 32-bit integer in big-endian byte order.
--
-- Big-endian ("network byte order") means the most-significant byte comes first.
-- For the value 0x01020304:
--   byte 0 = 0x01, byte 1 = 0x02, byte 2 = 0x03, byte 3 = 0x04
--
-- We use string.pack which is available in Lua 5.3+. The format ">I4" means:
--   ">"  = big-endian
--   "I4" = unsigned integer, 4 bytes
--
-- @param n  integer  Value 0–4294967295.
-- @return string  4-byte binary string.
local function pack_u32_be(n)
    return string.pack(">I4", n)
end

-- unpack_u32_be deserialises a big-endian uint32 from a binary string.
--
-- string.unpack returns the value AND the index of the next byte after the
-- consumed data. We return both so the caller can advance its read position.
--
-- @param s       string   Binary string containing the uint32.
-- @param offset  integer  1-indexed starting position (default 1).
-- @return integer, integer  The decoded value and the next read position.
local function unpack_u32_be(s, offset)
    return string.unpack(">I4", s, offset)
end

-- ── Bit Packing ───────────────────────────────────────────────────────────────

-- pack_bits_lsb_first packs a string of '0'/'1' characters into raw bytes,
-- LSB-first.
--
-- "LSB-first" means: the first character of the bit string maps to the least
-- significant bit (bit 0) of the first byte. The second character maps to bit 1,
-- and so on. After 8 bits are accumulated, a byte is emitted and the buffer
-- resets. Any remaining bits in the last partial byte are zero-padded on the
-- high side.
--
-- This is the same packing used in GIF, LZW (CMP03), and DEFLATE.
--
-- Example: bits = "10110" (5 bits)
--   i=1 '1' → buffer |= (1 << 0) = 0x01, bit_pos=1
--   i=2 '0' → buffer |= (0 << 1) = 0x01, bit_pos=2
--   i=3 '1' → buffer |= (1 << 2) = 0x05, bit_pos=3
--   i=4 '1' → buffer |= (1 << 3) = 0x0D, bit_pos=4
--   i=5 '0' → buffer |= (0 << 4) = 0x0D, bit_pos=5
--   flush partial → char(0x0D) = '\13'
--   Output: 1 byte = '\13'
--
-- @param bits  string  Sequence of '0' and '1' characters.
-- @return string  Packed binary bytes, zero-padded to byte boundary.
local function pack_bits_lsb_first(bits)
    local output   = {}
    local buffer   = 0
    local bit_pos  = 0

    for i = 1, #bits do
        local b = tonumber(bits:sub(i, i))
        -- Place this bit at position `bit_pos` within the accumulator.
        buffer  = buffer | (b << bit_pos)
        bit_pos = bit_pos + 1

        if bit_pos == 8 then
            -- Accumulator is full — flush it as a byte.
            output[#output + 1] = string.char(buffer)
            buffer  = 0
            bit_pos = 0
        end
    end

    -- Flush any remaining partial byte (zero-padded on the high side).
    if bit_pos > 0 then
        output[#output + 1] = string.char(buffer)
    end

    return table.concat(output)
end

-- ── Canonical Code Reconstruction ────────────────────────────────────────────

-- Symbols are single bytes, so a valid code-length table has at most 256 entries.
local MAX_SYMBOLS = 256

-- Longest code length the decoder accepts.  Real codes over 256 symbols are far
-- shorter; 56 keeps every shift, and the (len << 56 | code) key, inside a
-- 64-bit Lua integer.
local MAX_CODE_LEN = 56

-- canonical_codes_from_lengths reconstructs canonical Huffman codes from a
-- sorted code-length table, and VALIDATES the table because it comes straight
-- from untrusted input.
--
-- This is the inverse of the canonical assignment done during compression.
-- Given the sorted (symbol, length) pairs, we replay the same numeric
-- assignment and produce a lookup from (length, code) → symbol.
--
-- The input `lengths` must be sorted by (length, symbol) ascending — this is
-- exactly how the CMP04 header stores the table.
--
-- Assignment algorithm (DEFLATE style):
--   code = 0
--   For each entry (in sorted order):
--     If this entry's length > previous length:
--       code = code << (len - prev_len)   ← shift left to "make room"
--     Assign (len, code) → symbol
--     code = code + 1
--
-- Why does this work? The left-shift allocates a whole sub-tree of the correct
-- height to the new length tier. Incrementing assigns the next code at that tier.
-- The prefix-free property is maintained automatically.
--
-- SECURITY: errors are raised for
--   * lengths of 0 (matches nothing) or above MAX_CODE_LEN (shift overflow),
--   * tables not sorted by length (the encoder always sorts), and
--   * over-subscribed tables (Kraft sum > 1): before each assignment the
--     running code must still fit in `len` bits, i.e. code >> len == 0.
--
-- The lookup key is (len << 56) | code, an integer, so no per-symbol bit
-- strings are built and lookups are O(1).
--
-- @param lengths  table  Array of {symbol, length} pairs, sorted by (len, sym).
-- @return table, integer  Mapping {key → symbol}, and the longest code length.
local function canonical_codes_from_lengths(lengths)
    local code_to_sym = {}
    local code        = 0
    local prev_len    = lengths[1] and lengths[1][2] or 0
    local max_len     = 0

    for _, entry in ipairs(lengths) do
        local sym = entry[1]
        local len = entry[2]

        if len == 0 or len > MAX_CODE_LEN then
            error(string.format("CMP04 decompress: invalid code length %d in table", len))
        end
        if len < prev_len then
            error("CMP04 decompress: code-lengths table is not sorted by length")
        end

        -- Moving to a longer code length: shift left to allocate the new tier.
        code = code << (len - prev_len)

        if (code >> len) ~= 0 then
            error("CMP04 decompress: code-lengths table over-subscribes the code space")
        end

        code_to_sym[(len << 56) | code] = sym
        code     = code + 1
        prev_len = len
        max_len  = len
    end

    return code_to_sym, max_len
end

-- ── Encoder ───────────────────────────────────────────────────────────────────

-- compress compresses a Lua string using Huffman coding (CMP04 wire format).
--
-- Algorithm walkthrough (see module header for the full format diagram):
--
--   Step 1: Frequency count
--   -----------------------
--   Iterate every byte in the input. A table indexed by byte value (0–255)
--   accumulates how often each value appears. Only bytes that actually appear
--   in the input enter the Huffman tree (not all 256 possible byte values).
--
--   Step 2: Build the Huffman tree (DT27)
--   --------------------------------------
--   Collect the {byte_value, count} pairs and pass them to HuffmanTree.build().
--   The tree is built greedily via a min-heap (see huffman_tree/init.lua).
--
--   Step 3: Get canonical codes
--   ---------------------------
--   tree:canonical_code_table() returns {sym → bit_string}. The bit strings
--   are MSB-first canonical codes like "0", "10", "11".
--
--   Step 4: Sort the code-length table
--   -----------------------------------
--   For the wire header we need pairs sorted by (length, symbol). This sort
--   order is also what canonical_codes_from_lengths() requires on the decode
--   side, so it serves double duty.
--
--   Step 5: Encode the input
--   ------------------------
--   For each byte in the original input, look up its canonical code and append
--   it to a bit-string accumulator.
--
--   Step 6: Pack bits LSB-first
--   ---------------------------
--   pack_bits_lsb_first() converts the accumulated '0'/'1' string into raw bytes.
--
--   Step 7: Assemble the wire format
--   ---------------------------------
--   Concatenate: original_length (4 bytes) + symbol_count (4 bytes) +
--                code-length table (2×N bytes) + packed bit stream.
--
-- Edge case — empty input:
--   Returns just the 8-byte header (original_length=0, symbol_count=0) with no
--   code table and no bit stream. Decompressor returns "" immediately.
--
-- Edge case — single distinct symbol:
--   HuffmanTree assigns code "0" (1 bit per symbol). The canonical table has
--   exactly one entry.
--
-- @param data  string  Input string (may contain any byte values 0–255).
-- @return string  Compressed data in CMP04 wire format.
function M.compress(data)
    -- ── Step 1: Count byte frequencies ──────────────────────────────────────

    local freq = {}
    for i = 1, #data do
        local b = data:byte(i)
        freq[b] = (freq[b] or 0) + 1
    end

    -- Handle the empty-input edge case immediately.
    if #data == 0 then
        return pack_u32_be(0) .. pack_u32_be(0)
    end

    -- ── Step 2: Build the Huffman tree ───────────────────────────────────────

    -- Build a sorted list of {symbol, frequency} pairs.
    -- Sorting by symbol ensures deterministic tree construction.
    local weights = {}
    for sym, count in pairs(freq) do
        weights[#weights + 1] = {sym, count}
    end
    table.sort(weights, function(a, b) return a[1] < b[1] end)

    local tree = HuffmanTree.build(weights)

    -- ── Step 3: Get canonical codes ──────────────────────────────────────────

    -- canonical_code_table() returns {sym → "0"/"10"/"11"/...}
    local codes = tree:canonical_code_table()

    -- ── Step 4: Build sorted code-length table ───────────────────────────────

    -- Each entry: {symbol_byte, code_length}
    -- Sorted by (code_length, symbol_byte) ascending.
    local lengths = {}
    for sym, bits in pairs(codes) do
        lengths[#lengths + 1] = {sym, #bits}
    end
    table.sort(lengths, function(a, b)
        if a[2] ~= b[2] then return a[2] < b[2] end
        return a[1] < b[1]
    end)

    -- ── Step 5: Encode the input bytes into a bit string ────────────────────

    local bit_parts = {}
    for i = 1, #data do
        bit_parts[#bit_parts + 1] = codes[data:byte(i)]
    end
    local bit_string = table.concat(bit_parts)

    -- ── Step 6: Pack bits LSB-first into raw bytes ───────────────────────────

    local bit_bytes = pack_bits_lsb_first(bit_string)

    -- ── Step 7: Assemble CMP04 wire format ───────────────────────────────────

    local parts = {}

    -- Header: original_length (4 bytes) + symbol_count (4 bytes)
    parts[#parts + 1] = pack_u32_be(#data)
    parts[#parts + 1] = pack_u32_be(#lengths)

    -- Code-length table: N × 2 bytes (symbol, length)
    for _, entry in ipairs(lengths) do
        parts[#parts + 1] = string.char(entry[1], entry[2])
    end

    -- Bit stream
    parts[#parts + 1] = bit_bytes

    return table.concat(parts)
end

-- ── Decoder ───────────────────────────────────────────────────────────────────

-- decompress decompresses CMP04 wire-format data back to the original string.
--
-- Algorithm walkthrough:
--
--   Step 1: Parse the 8-byte header
--   --------------------------------
--   Read original_length (bytes 0–3) and symbol_count (bytes 4–7) as
--   big-endian uint32. string.unpack returns (value, next_position) so we
--   chain the calls using the returned positions.
--
--   Step 2: Parse the code-length table
--   -------------------------------------
--   Read N × 2 bytes starting at byte 8. Each pair is (symbol_byte, code_len).
--   The entries are already sorted by (code_len, symbol_byte) because that is
--   how compress() wrote them.
--
--   Step 3: Reconstruct canonical codes
--   -------------------------------------
--   canonical_codes_from_lengths() replays the same numeric assignment the
--   encoder used, producing {bit_string → symbol}. This is the lookup table
--   the decoder will use.
--
--   Step 4: Unpack the bit stream
--   -----------------------------
--   The byte stream after the header+table is the packed LSB-first bit stream.
--   unpack_bits_lsb_first() expands it to a '0'/'1' string.
--
--   Step 5: Decode original_length symbols
--   ----------------------------------------
--   We cannot use HuffmanTree:decode_all() here because we reconstructed a
--   code→symbol table, not a tree object. Instead we do a greedy prefix scan:
--   extend the current code prefix one bit at a time until it matches an entry
--   in the code_to_sym table. This is O(L) per symbol where L is the max code
--   length — equivalent to tree-walking.
--
--   Step 6: Bytes → string
--   ----------------------
--   Convert the decoded symbol integers (0–255) back to a Lua string in chunks
--   to avoid hitting Lua's stack limit on large outputs.
--
-- Edge cases:
--   - Empty (original_length=0): return "" immediately.
--   - Single symbol: all bits are '0'; the prefix "0" always matches.
--
-- @param data  string  Binary string in CMP04 wire format.
-- @return string  Decompressed original string.
function M.decompress(data)
    -- Need at least the 8-byte header.
    if #data < 8 then
        return ""
    end

    -- ── Step 1: Parse header ─────────────────────────────────────────────────

    local original_length, pos1 = unpack_u32_be(data, 1)
    local symbol_count,    pos2 = unpack_u32_be(data, pos1)

    if original_length == 0 then
        return ""
    end

    -- ── Step 2: Parse code-length table ──────────────────────────────────────

    -- SECURITY: the header is attacker-controlled.  Symbols are single bytes,
    -- so a table can name at most 256 of them; refuse more before sizing
    -- anything from the count.
    if symbol_count > MAX_SYMBOLS then
        error(string.format("CMP04 decompress: symbol_count %d exceeds %d",
            symbol_count, MAX_SYMBOLS))
    end

    -- pos2 is the 1-indexed position of the first byte of the code-length table.
    local table_pos = pos2
    local bit_start = table_pos + symbol_count * 2
    if #data < bit_start - 1 then
        error("CMP04 decompress: input too short for the code-lengths table")
    end

    local lengths = {}
    local seen    = {}
    for _ = 1, symbol_count do
        local sym = data:byte(table_pos)
        local len = data:byte(table_pos + 1)
        -- A byte value can have only one code; a repeat makes the table ambiguous.
        if seen[sym] then
            error(string.format(
                "CMP04 decompress: symbol %d appears twice in code-lengths table", sym))
        end
        seen[sym] = true
        lengths[#lengths + 1] = {sym, len}
        table_pos = table_pos + 2
    end

    -- ── Step 3: Reconstruct canonical codes ──────────────────────────────────

    -- lengths is already sorted by (len, sym) — the encoder wrote them that way.
    -- canonical_codes_from_lengths validates the table and returns
    -- {(len << 56 | code) → symbol} plus the longest code length.
    local code_to_sym, max_len = canonical_codes_from_lengths(lengths)

    -- ── Step 4: Bound the declared length by the payload ─────────────────────
    --
    -- SECURITY: every code is at least one bit, so N payload bytes hold at most
    -- 8*N symbols.  Reject a larger declared length before doing any work sized
    -- from it.
    local total_bits = (#data - (bit_start - 1)) * 8
    if original_length > total_bits then
        error(string.format(
            "CMP04 decompress: bit stream exhausted: declared length %d exceeds the %d bits available",
            original_length, total_bits))
    end

    -- ── Step 5: Greedy prefix decode straight from the bytes ─────────────────
    --
    -- Bits are LSB-first within each byte, so bit i (0-based) of the stream is
    -- (byte[i // 8] >> (i % 8)) & 1 — O(1), with no expanded '0'/'1' string.
    -- We build the candidate code MSB-first as an integer.  Canonical codes are
    -- prefix-free, so the first (len, code) hit is the symbol.  Once the
    -- accumulated length reaches the longest code without a match, no later bit
    -- can match (an incomplete table fed unmatched bits), so we fail at once
    -- instead of scanning the rest of the input.
    --
    -- Why not use HuffmanTree:decode_all()?
    -- We reconstructed a flat code→symbol table from the lengths, not a tree
    -- object, and the header only stores lengths, not frequencies.

    local symbols = {}
    local count   = 0
    local acc     = 0
    local acc_len = 0
    local pos     = 0

    while count < original_length do
        if pos >= total_bits then
            error(string.format(
                "CMP04 decompress: bit stream exhausted after %d symbols; expected %d",
                count, original_length))
        end

        local byte = data:byte(bit_start + (pos >> 3))
        acc     = (acc << 1) | ((byte >> (pos & 7)) & 1)
        acc_len = acc_len + 1
        pos     = pos + 1

        local sym = code_to_sym[(acc_len << 56) | acc]
        if sym then
            count = count + 1
            symbols[count] = sym
            acc     = 0
            acc_len = 0
        elseif acc_len >= max_len then
            error("CMP04 decompress: bit pattern does not match any code")
        end
    end

    -- ── Step 6: Convert symbols (integers) to a string ───────────────────────
    --
    -- We process in 256-byte chunks to avoid hitting Lua's C stack limit when
    -- calling table.unpack on large arrays. (Lua's C stack is limited to a few
    -- hundred levels; table.unpack of 10000 elements will overflow it.)

    if #symbols == 0 then return "" end

    local out_parts = {}
    local n         = #symbols
    local CHUNK     = 256
    local i         = 1
    while i <= n do
        local j = math.min(i + CHUNK - 1, n)
        out_parts[#out_parts + 1] = string.char(table.unpack(symbols, i, j))
        i = j + 1
    end

    return table.concat(out_parts)
end

return M
