-- ============================================================================
-- CodingAdventures.LZSS
-- ============================================================================
--
-- LZSS lossless compression algorithm (Storer & Szymanski, 1982).
-- Part of the CMP compression series in the coding-adventures monorepo.
--
-- What Is LZSS?
-- -------------
--
-- LZSS refines LZ77 by eliminating the mandatory `next_char` byte that LZ77
-- appends after every token — even pure back-references. Instead, a flag-bit
-- scheme distinguishes the two token kinds:
--
--   Literal(byte)         → 1 byte  (flag bit = 0)
--   Match(offset, length) → 3 bytes (flag bit = 1)
--
-- Tokens are grouped in blocks of 8. Each block begins with a 1-byte flag
-- (LSB = first token, bit 7 = eighth token).
--
-- Break-even Point
-- ----------------
--
-- A match token costs 3 bytes; three literals cost 3 bytes. So min_match = 3
-- is the minimum length that yields any saving. Length >= 4 yields net gain.
--
--   LZ77 per match: 4 bytes (offset:2, length:1, next_char:1)
--   LZSS per match: 3 bytes (offset:2, length:1) + 1/8 flag overhead
--
-- LZSS typically achieves 25–50% better compression than LZ77 on repetitive data.
--
-- Wire Format (CMP02)
-- -------------------
--
--   Bytes 0–3:  original_length  (big-endian uint32)
--   Bytes 4–7:  block_count      (big-endian uint32)
--   Bytes 8+:   blocks
--     Each block:
--       [1 byte]  flag — bit i (LSB-first): 0 = literal, 1 = match
--       [variable] up to 8 items:
--                    flag=0: 1 byte  (literal value)
--                    flag=1: 3 bytes (offset BE uint16 + length uint8)
--
-- The original_length field allows the decoder to trim padding from the final
-- block (unlike LZ77, there is no trailing next_char to mark the end cleanly).
--
-- The Series: CMP00 -> CMP05
-- --------------------------
--
-- CMP00 (LZ77, 1977)     -- Sliding-window backreferences.
-- CMP01 (LZ78, 1978)     -- Explicit dictionary (trie), no sliding window.
-- CMP02 (LZSS, 1982)     -- LZ77 + flag bits; eliminates wasted literals. This module.
-- CMP03 (LZW,  1984)     -- Pre-initialized dictionary; powers GIF.
-- CMP04 (Huffman, 1952)  -- Entropy coding; prerequisite for DEFLATE.
-- CMP05 (DEFLATE, 1996)  -- LZ77 + Huffman; ZIP/gzip/PNG/zlib standard.
-- ============================================================================

local M = {}

M.VERSION = "0.1.0"

-- ── Token constructors ────────────────────────────────────────────────────────

-- literal creates a Literal token.
--
-- @param byte (integer) the raw byte value (0–255).
-- @return table {kind="literal", byte=byte}
function M.literal(byte)
    return {kind = "literal", byte = byte}
end

-- match creates a Match token (back-reference into the sliding window).
--
-- @param offset (integer) distance back in the window (1..window_size).
-- @param length (integer) number of bytes to copy (min_match..max_match).
-- @return table {kind="match", offset=offset, length=length}
function M.match(offset, length)
    return {kind = "match", offset = offset, length = length}
end

-- ── Default parameters ────────────────────────────────────────────────────────

local DEFAULT_WINDOW_SIZE = 4096
local DEFAULT_MAX_MATCH   = 255
local DEFAULT_MIN_MATCH   = 3

M.DEFAULT_WINDOW_SIZE = DEFAULT_WINDOW_SIZE
M.DEFAULT_MAX_MATCH   = DEFAULT_MAX_MATCH
M.DEFAULT_MIN_MATCH   = DEFAULT_MIN_MATCH

-- ── Encoder ───────────────────────────────────────────────────────────────────

-- find_longest_match scans data[search_start..cursor-1] for the best match.
--
-- Matches may overlap: the encoder can extend the match into the lookahead
-- as long as the data at lookahead matches what was already in the window
-- (which is a sliding-window property). The decoder copies byte-by-byte to
-- reproduce this.
--
-- @param data        (table) 1-indexed integer byte array.
-- @param cursor      (integer) current 1-indexed position.
-- @param window_size (integer) maximum lookback distance.
-- @param max_match   (integer) maximum match length.
-- @return (best_offset, best_length)
local function find_longest_match(data, cursor, window_size, max_match)
    local best_offset = 0
    local best_length = 0
    local data_len    = #data

    local search_start  = math.max(1, cursor - window_size)
    -- LZSS does NOT reserve 1 byte for next_char — lookahead extends to EOF.
    local lookahead_end = math.min(cursor + max_match - 1, data_len)

    for pos = search_start, cursor - 1 do
        local length = 0
        while (cursor + length) <= lookahead_end
            and data[pos + length] == data[cursor + length]
        do
            length = length + 1
        end

        if length > best_length then
            best_length = length
            best_offset = cursor - pos  -- Distance back from cursor (1-indexed gap).
        end
    end

    return best_offset, best_length
end

-- encode encodes a byte array into an LZSS token stream.
--
-- Emits Literal tokens for single bytes and Match tokens for back-references.
-- Unlike LZ77, there is no trailing next_char field — the cursor advances by
-- exactly `best_length` positions (not best_length + 1).
--
-- @param data        (table) 1-indexed integer byte array.
-- @param window_size (integer) maximum lookback distance (default 4096).
-- @param max_match   (integer) maximum match length (default 255).
-- @param min_match   (integer) minimum match length for a Match token (default 3).
-- @return (table) array of token tables.
function M.encode(data, window_size, max_match, min_match)
    window_size = window_size or DEFAULT_WINDOW_SIZE
    max_match   = max_match   or DEFAULT_MAX_MATCH
    min_match   = min_match   or DEFAULT_MIN_MATCH

    local tokens   = {}
    local data_len = #data
    local cursor   = 1

    while cursor <= data_len do
        local offset, length = find_longest_match(data, cursor, window_size, max_match)

        if length >= min_match then
            tokens[#tokens + 1] = M.match(offset, length)
            cursor = cursor + length
        else
            tokens[#tokens + 1] = M.literal(data[cursor])
            cursor = cursor + 1
        end
    end

    return tokens
end

-- encode_string is a convenience wrapper that accepts a Lua string.
--
-- @param str         (string) input string.
-- @param window_size (integer) optional, default 4096.
-- @param max_match   (integer) optional, default 255.
-- @param min_match   (integer) optional, default 3.
-- @return (table) array of token tables.
function M.encode_string(str, window_size, max_match, min_match)
    local data = {str:byte(1, #str)}
    return M.encode(data, window_size, max_match, min_match)
end

-- ── Decoder ───────────────────────────────────────────────────────────────────

-- decode decodes an LZSS token stream back to a byte array.
--
-- For Match tokens, bytes are copied one at a time from `offset` positions
-- back in the output. Copying byte-by-byte (not bulk) is essential for
-- overlapping matches (e.g., Match(1, 6) on [65] → six copies of 65).
--
-- @param tokens          (table) array of token tables from encode().
-- @param original_length (integer|nil) if given, truncates output to this length.
-- @return (table) 1-indexed integer byte array.
function M.decode(tokens, original_length)
    local output = {}

    for _, tok in ipairs(tokens) do
        if tok.kind == "literal" then
            output[#output + 1] = tok.byte
        else
            -- Match: copy `tok.length` bytes starting at `tok.offset` back.
            local start = #output - tok.offset + 1
            for _ = 1, tok.length do
                output[#output + 1] = output[start]
                start = start + 1
            end
        end

        -- Honour original_length to trim block-padding.
        if original_length and #output >= original_length then
            break
        end
    end

    if original_length and #output > original_length then
        local trimmed = {}
        for i = 1, original_length do trimmed[i] = output[i] end
        return trimmed
    end

    return output
end

-- decode_to_string decodes tokens and returns a Lua string.
--
-- @param tokens          (table) array of token tables.
-- @param original_length (integer|nil) optional truncation.
-- @return (string) reconstructed string.
function M.decode_to_string(tokens, original_length)
    local bytes = M.decode(tokens, original_length)
    if #bytes == 0 then return "" end
    return string.char(table.unpack(bytes))
end

-- ── Serialisation ─────────────────────────────────────────────────────────────

-- write_uint32_be writes a big-endian uint32 to a table of chars.
local function write_uint32_be(parts, n)
    parts[#parts + 1] = string.char(
        math.floor(n / 16777216) % 256,
        math.floor(n / 65536)    % 256,
        math.floor(n / 256)      % 256,
        n                        % 256
    )
end

-- read_uint32_be reads a big-endian uint32 from a binary string at position `pos`.
-- Returns (value, next_pos).
local function read_uint32_be(data, pos)
    local b1, b2, b3, b4 = data:byte(pos, pos + 3)
    return b1 * 16777216 + b2 * 65536 + b3 * 256 + b4, pos + 4
end

-- serialise_tokens serialises a token list to the CMP02 wire format.
--
-- Groups up to 8 tokens per block. Each block starts with a 1-byte flag
-- (bit i = 0 for Literal, 1 for Match). Literal tokens use 1 byte; Match
-- tokens use 3 bytes (offset BE uint16 + length uint8).
--
-- @param tokens          (table) array of token tables.
-- @param original_length (integer) byte length of the original input.
-- @return (string) binary string in CMP02 wire format.
function M.serialise_tokens(tokens, original_length)
    original_length = original_length or 0
    local blocks = {}

    -- Chunk tokens into groups of 8.
    local i = 1
    while i <= #tokens do
        local chunk_end = math.min(i + 7, #tokens)
        local flag = 0
        local symbol_parts = {}

        for bit = 0, chunk_end - i do
            local tok = tokens[i + bit]
            if tok.kind == "match" then
                -- Set flag bit for this position.
                flag = flag + (2 ^ bit)
                -- Encode offset as BE uint16, length as uint8.
                local off = tok.offset
                symbol_parts[#symbol_parts + 1] = string.char(
                    math.floor(off / 256) % 256,
                    off                   % 256,
                    tok.length            % 256
                )
            else
                -- Literal: 1 byte.
                symbol_parts[#symbol_parts + 1] = string.char(tok.byte % 256)
            end
        end

        blocks[#blocks + 1] = string.char(flag % 256) .. table.concat(symbol_parts)
        i = chunk_end + 1
    end

    local parts = {}
    write_uint32_be(parts, original_length)
    write_uint32_be(parts, #blocks)
    for _, blk in ipairs(blocks) do
        parts[#parts + 1] = blk
    end

    return table.concat(parts)
end

-- deserialise_tokens deserialises a CMP02 binary string to a token list.
--
-- Security: caps block_count against the actual payload size to prevent a
-- crafted header from causing unbounded iteration on minimal input.
--
-- @param data (string) binary string from serialise_tokens / compress.
-- @return (tokens, original_length) where tokens is a table array.
function M.deserialise_tokens(data)
    if #data < 8 then return {}, 0 end

    local original_length, pos = read_uint32_be(data, 1)
    local block_count
    block_count, pos = read_uint32_be(data, pos)

    -- Cap block_count against remaining payload size to prevent DoS.
    local max_possible = #data - 8  -- at minimum 1 byte per block
    if block_count > max_possible then
        block_count = max_possible
    end

    local tokens = {}

    for _ = 1, block_count do
        if pos > #data then break end

        local flag = data:byte(pos)
        pos = pos + 1

        for bit = 0, 7 do
            if pos > #data then break end

            -- Check if this bit is set (match) or not (literal).
            local bit_set = math.floor(flag / (2 ^ bit)) % 2 == 1

            if bit_set then
                -- Match: 3 bytes (offset BE uint16, length uint8).
                if pos + 2 > #data then break end
                local o1, o2, length = data:byte(pos, pos + 2)
                local offset = o1 * 256 + o2
                tokens[#tokens + 1] = M.match(offset, length)
                pos = pos + 3
            else
                -- Literal: 1 byte.
                tokens[#tokens + 1] = M.literal(data:byte(pos))
                pos = pos + 1
            end
        end
    end

    return tokens, original_length
end

-- ── One-shot API ──────────────────────────────────────────────────────────────

-- compress compresses a Lua string using LZSS (CMP02 wire format).
--
-- @param str         (string) input string.
-- @param window_size (integer) default 4096.
-- @param max_match   (integer) default 255.
-- @param min_match   (integer) default 3.
-- @return (string) compressed binary string.
function M.compress(str, window_size, max_match, min_match)
    local tokens = M.encode_string(str, window_size, max_match, min_match)
    return M.serialise_tokens(tokens, #str)
end

-- decompress decompresses data compressed with compress().
--
-- @param data (string) compressed binary string.
-- @return (string) original data.
function M.decompress(data)
    local tokens, original_length = M.deserialise_tokens(data)
    return M.decode_to_string(tokens, original_length)
end

return M
