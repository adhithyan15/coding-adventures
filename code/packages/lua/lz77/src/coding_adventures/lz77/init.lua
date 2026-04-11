-- ============================================================================
-- CodingAdventures.LZ77
-- ============================================================================
--
-- LZ77 lossless compression algorithm (Lempel & Ziv, 1977).
-- Part of the CMP compression series in the coding-adventures monorepo.
--
-- What Is LZ77?
-- -------------
--
-- LZ77 replaces repeated byte sequences with compact backreferences into a
-- sliding window of recently seen data. It is the foundation of DEFLATE,
-- gzip, PNG, and zlib.
--
-- The Sliding Window Model
-- ------------------------
--
--     ┌─────────────────────────────────┬──────────────────┐
--     │         SEARCH BUFFER           │ LOOKAHEAD BUFFER  │
--     │  (already processed — the       │  (not yet seen —  │
--     │   last window_size bytes)       │  next max_match)  │
--     └─────────────────────────────────┴──────────────────┘
--                                        ↑
--                                    cursor (current position)
--
-- At each step the encoder finds the longest match in the search buffer.
-- If found and long enough (>= min_match), emit a backreference token.
-- Otherwise emit a literal token.
--
-- Token: {offset, length, next_char}
-- -----------------------------------
--
-- - offset:    distance back the match starts (1..window_size), or 0.
-- - length:    number of bytes the match covers (0 = literal).
-- - next_char: literal byte immediately after the match (0..255).
--
-- Overlapping Matches
-- -------------------
--
-- When offset < length, the match extends into bytes not yet decoded.
-- The decoder must copy byte-by-byte (not bulk) to handle this.
--
-- The Series: CMP00 -> CMP05
-- --------------------------
--
-- CMP00 (LZ77, 1977) -- Sliding-window backreferences. This module.
-- CMP01 (LZ78, 1978) -- Explicit dictionary (trie), no sliding window.
-- CMP02 (LZSS, 1982) -- LZ77 + flag bits; eliminates wasted literals.
-- CMP03 (LZW,  1984) -- Pre-initialized dictionary; powers GIF.
-- CMP04 (Huffman, 1952) -- Entropy coding; prerequisite for DEFLATE.
-- CMP05 (DEFLATE, 1996) -- LZ77 + Huffman; ZIP/gzip/PNG/zlib standard.
-- ============================================================================

local M = {}

M.VERSION = "0.1.0"

-- token creates a new token table.
--
-- @param offset    (integer) distance back the match starts, or 0.
-- @param length    (integer) number of bytes the match covers.
-- @param next_char (integer) literal byte immediately after the match.
-- @return table {offset, length, next_char}
function M.token(offset, length, next_char)
    return {offset = offset, length = length, next_char = next_char}
end

-- find_longest_match scans the search buffer for the longest match.
--
-- @param data        (table) input bytes as a 1-indexed Lua array of integers.
-- @param cursor      (integer) current 1-indexed position.
-- @param window_size (integer) maximum lookback distance.
-- @param max_match   (integer) maximum match length.
-- @return (best_offset, best_length)
local function find_longest_match(data, cursor, window_size, max_match)
    local best_offset = 0
    local best_length = 0
    local data_len = #data

    -- The search buffer starts at most window_size bytes back (1-indexed).
    local search_start = math.max(1, cursor - window_size)

    -- The lookahead cannot extend past end of input. Reserve 1 byte for next_char.
    local lookahead_end = math.min(cursor + max_match - 1, data_len - 1)

    for pos = search_start, cursor - 1 do
        local length = 0
        -- Match byte by byte. Matches may overlap (extend past cursor).
        while (cursor + length) <= lookahead_end
            and data[pos + length] == data[cursor + length]
        do
            length = length + 1
        end

        if length > best_length then
            best_length = length
            best_offset = cursor - pos  -- Distance back from cursor (1-indexed).
        end
    end

    return best_offset, best_length
end

-- encode encodes a byte array into an LZ77 token stream.
--
-- @param data        (table) input as a 1-indexed Lua array of integers.
-- @param window_size (integer) maximum lookback distance (default 4096).
-- @param max_match   (integer) maximum match length (default 255).
-- @param min_match   (integer) minimum match length for backreference (default 3).
-- @return (table) array of token tables.
function M.encode(data, window_size, max_match, min_match)
    window_size = window_size or 4096
    max_match   = max_match   or 255
    min_match   = min_match   or 3

    local tokens = {}
    local data_len = #data
    local cursor = 1

    while cursor <= data_len do
        -- Edge case: last byte has no room for next_char after a match.
        if cursor == data_len then
            tokens[#tokens + 1] = M.token(0, 0, data[cursor])
            cursor = cursor + 1
        else
            local offset, length = find_longest_match(data, cursor, window_size, max_match)

            if length >= min_match then
                -- Emit a backreference token.
                local next_char = data[cursor + length]
                tokens[#tokens + 1] = M.token(offset, length, next_char)
                cursor = cursor + length + 1
            else
                -- Emit a literal token.
                tokens[#tokens + 1] = M.token(0, 0, data[cursor])
                cursor = cursor + 1
            end
        end
    end

    return tokens
end

-- encode_string is a convenience wrapper that accepts a Lua string.
-- Converts the string to a byte array, encodes, and returns tokens.
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

-- decode decodes an LZ77 token stream back to a byte array.
--
-- @param tokens         (table) array of token tables.
-- @param initial_buffer (table) optional seed for search buffer (default {}).
-- @return (table) 1-indexed Lua array of integer bytes.
function M.decode(tokens, initial_buffer)
    local output = {}
    if initial_buffer then
        for _, b in ipairs(initial_buffer) do
            output[#output + 1] = b
        end
    end

    for _, tok in ipairs(tokens) do
        if tok.length > 0 then
            -- Copy length bytes from position (output_len - offset + 1).
            -- Use 1-indexed positions.
            local start = #output - tok.offset + 1
            -- Copy byte-by-byte to handle overlapping matches (offset < length).
            for i = 0, tok.length - 1 do
                output[#output + 1] = output[start + i]
            end
        end
        -- Always append next_char.
        output[#output + 1] = tok.next_char
    end

    return output
end

-- decode_to_string decodes tokens and returns a Lua string.
--
-- @param tokens         (table) array of token tables.
-- @param initial_buffer (table) optional seed for search buffer.
-- @return (string) reconstructed string.
function M.decode_to_string(tokens, initial_buffer)
    local bytes = M.decode(tokens, initial_buffer)
    return string.char(table.unpack(bytes))
end

-- serialise_tokens serialises a token list to a binary string.
--
-- Format:
--   [4 bytes: token count (big-endian uint32)]
--   [N x 4 bytes: each token (offset: uint16 BE, length: uint8, next_char: uint8)]
--
-- @param tokens (table) array of token tables.
-- @return (string) binary string.
function M.serialise_tokens(tokens)
    local parts = {}
    local count = #tokens

    -- Write count as big-endian uint32.
    parts[#parts + 1] = string.char(
        math.floor(count / 16777216) % 256,
        math.floor(count / 65536)    % 256,
        math.floor(count / 256)      % 256,
        count                        % 256
    )

    for _, tok in ipairs(tokens) do
        local off = tok.offset
        -- Write offset as big-endian uint16, then length uint8, next_char uint8.
        parts[#parts + 1] = string.char(
            math.floor(off / 256) % 256,
            off                   % 256,
            tok.length            % 256,
            tok.next_char         % 256
        )
    end

    return table.concat(parts)
end

-- deserialise_tokens deserialises a binary string back to a token list.
--
-- @param data (string) binary string (output of serialise_tokens).
-- @return (table) array of token tables.
function M.deserialise_tokens(data)
    if #data < 4 then return {} end

    -- Read token count (big-endian uint32).
    local b1, b2, b3, b4 = data:byte(1, 4)
    local count = b1 * 16777216 + b2 * 65536 + b3 * 256 + b4

    local tokens = {}

    for i = 1, count do
        local base = 4 + (i - 1) * 4 + 1
        if base + 3 > #data then break end

        local o1, o2, length, next_char = data:byte(base, base + 3)
        local offset = o1 * 256 + o2
        tokens[#tokens + 1] = M.token(offset, length, next_char)
    end

    return tokens
end

-- compress compresses a Lua string using LZ77.
--
-- @param str         (string) input string.
-- @param window_size (integer) default 4096.
-- @param max_match   (integer) default 255.
-- @param min_match   (integer) default 3.
-- @return (string) compressed binary string.
function M.compress(str, window_size, max_match, min_match)
    local tokens = M.encode_string(str, window_size, max_match, min_match)
    return M.serialise_tokens(tokens)
end

-- decompress decompresses data compressed with compress().
--
-- @param data (string) compressed binary string.
-- @return (string) original data.
function M.decompress(data)
    local tokens = M.deserialise_tokens(data)
    return M.decode_to_string(tokens)
end

return M
