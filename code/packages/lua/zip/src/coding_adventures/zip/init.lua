-- ============================================================================
-- CodingAdventures.Zip — ZIP archive format (PKZIP, 1989) — CMP09
-- ============================================================================
--
-- ZIP bundles one or more files into a single `.zip` archive, compressing
-- each entry independently with DEFLATE (method 8) or storing it verbatim
-- (method 0). The same format underlies Java JARs, Office Open XML (.docx),
-- Android APKs, Python wheels, and many more.
--
-- Architecture
-- ────────────
--
--   ┌─────────────────────────────────────────────────────┐
--   │  [Local File Header + File Data]  ← entry 1         │
--   │  [Local File Header + File Data]  ← entry 2         │
--   │  ...                                                │
--   │  ══════════ Central Directory ══════════            │
--   │  [Central Dir Header]  ← entry 1 (has local offset)│
--   │  [Central Dir Header]  ← entry 2                   │
--   │  [End of Central Directory Record]                  │
--   └─────────────────────────────────────────────────────┘
--
-- The dual-header design enables two workflows:
--   - Sequential write: append Local Headers, write CD at the end.
--   - Random-access read: seek to EOCD at the end, read CD, jump to any entry.
--
-- DEFLATE inside ZIP
-- ──────────────────
-- ZIP method 8 stores raw RFC 1951 DEFLATE — no zlib wrapper.  This
-- The encoder uses fixed Huffman blocks (BTYPE=01) with the LZSS module for
-- LZ77 match-finding. The decoder accepts stored, fixed, and dynamic blocks.
--
-- Series
-- ──────
--   CMP02 (LZSS,    1982) — LZ77 + flag bits        ← dependency
--   CMP05 (DEFLATE, 1996) — LZ77 + Huffman           ← inlined here (raw RFC 1951)
--   CMP09 (ZIP,     1989) — DEFLATE container        ← this module

local lzss = require("coding_adventures.lzss")

local M = {}

M.VERSION = "0.2.0"

M.RAW_INFLATE_MAX_OUTPUT = 256 * 1024 * 1024
M.RAW_INFLATE_ERROR_CODES = {
    "invalid-output-limit",
    "unexpected-eof",
    "reserved-block-type",
    "stored-length-mismatch",
    "huffman-oversubscribed",
    "incomplete-code-length-tree",
    "incomplete-literal-length-tree",
    "incomplete-distance-tree",
    "repeat-without-previous",
    "repeat-overrun",
    "invalid-literal-length-symbol",
    "reserved-distance-symbol",
    "invalid-back-reference",
    "output-limit-exceeded",
}

local _add_entry  -- forward declaration; defined below add_file/add_directory

-- ============================================================================
-- Utilities: little-endian byte packing
-- ============================================================================

local function le16(v)
    v = v & 0xFFFF
    return string.char(v & 0xFF, (v >> 8) & 0xFF)
end

local function le32(v)
    v = v & 0xFFFFFFFF
    return string.char(v & 0xFF, (v >> 8) & 0xFF, (v >> 16) & 0xFF, (v >> 24) & 0xFF)
end

-- Read a little-endian u16 from string s at byte offset pos (1-indexed).
-- Returns nil if out of bounds.
local function read_le16(s, pos)
    if pos + 1 > #s then return nil end
    local b1, b2 = s:byte(pos, pos + 1)
    return b1 | (b2 << 8)
end

-- Read a little-endian u32 from string s at byte offset pos (1-indexed).
local function read_le32(s, pos)
    if pos + 3 > #s then return nil end
    local b1, b2, b3, b4 = s:byte(pos, pos + 3)
    return b1 | (b2 << 8) | (b3 << 16) | (b4 << 24)
end

-- ============================================================================
-- CRC-32
-- ============================================================================
--
-- CRC-32 uses polynomial 0xEDB88320 (reflected form of 0x04C11DB7).
-- Table-driven: precompute 256 entries, then process one byte at a time.

local CRC_TABLE = (function()
    local t = {}
    for i = 0, 255 do
        local c = i
        for _ = 1, 8 do
            if c & 1 ~= 0 then
                c = 0xEDB88320 ~ (c >> 1)
            else
                c = c >> 1
            end
        end
        t[i] = c
    end
    return t
end)()

-- crc32 computes the CRC-32 of a byte string, starting from initial (default 0).
--
-- @param data    (string)  input bytes.
-- @param initial (integer) optional seed, use previous result for incremental.
-- @return (integer) 32-bit CRC.
function M.crc32(data, initial)
    local crc = (initial or 0) ~ 0xFFFFFFFF
    for i = 1, #data do
        local byte = data:byte(i)
        crc = CRC_TABLE[(crc ~ byte) & 0xFF] ~ (crc >> 8)
    end
    return crc ~ 0xFFFFFFFF
end

-- ============================================================================
-- MS-DOS Date / Time Encoding
-- ============================================================================
--
-- ZIP stores timestamps in MS-DOS packed format:
--   Time (16-bit): bits 15-11=hours, bits 10-5=minutes, bits 4-0=seconds/2
--   Date (16-bit): bits 15-9=year-1980, bits 8-5=month, bits 4-0=day
-- Combined 32-bit: (date << 16) | time.

-- DOS_EPOCH is 1980-01-01 00:00:00 → date=(0<<9)|(1<<5)|1 = 33 = 0x0021, time=0.
M.DOS_EPOCH = 0x00210000

-- dos_datetime encodes a (year, month, day, hour, minute, second) tuple.
function M.dos_datetime(year, month, day, hour, minute, second)
    hour   = hour   or 0
    minute = minute or 0
    second = second or 0
    local t = (hour << 11) | (minute << 5) | (second // 2)
    local d = ((year - 1980) << 9) | (month << 5) | day
    return (d << 16) | t
end

-- ============================================================================
-- RFC 1951 DEFLATE — Bit I/O
-- ============================================================================
--
-- RFC 1951 packs bits LSB-first within bytes. Huffman codes are sent MSB-first
-- logically, so we bit-reverse them before writing LSB-first.

-- BitWriter accumulates bits then flushes whole bytes.
local function new_bit_writer()
    return {buf = 0, bits = 0, out = {}}
end

local function bw_write_lsb(bw, value, nbits)
    bw.buf = bw.buf | (value << bw.bits)
    bw.bits = bw.bits + nbits
    while bw.bits >= 8 do
        bw.out[#bw.out + 1] = string.char(bw.buf & 0xFF)
        bw.buf  = bw.buf >> 8
        bw.bits = bw.bits - 8
    end
end

local function bw_write_huffman(bw, code, nbits)
    -- Bit-reverse the top nbits of code.
    local rev = 0
    local c = code
    for _ = 1, nbits do
        rev = (rev << 1) | (c & 1)
        c   = c >> 1
    end
    bw_write_lsb(bw, rev, nbits)
end

local function bw_align(bw)
    if bw.bits > 0 then
        bw.out[#bw.out + 1] = string.char(bw.buf & 0xFF)
        bw.buf  = 0
        bw.bits = 0
    end
end

local function bw_finish(bw)
    bw_align(bw)
    return table.concat(bw.out)
end

-- BitReader reads bits LSB-first from a byte string.
local function new_bit_reader(data)
    return {data = data, pos = 1, buf = 0, bits = 0}
end

local function br_fill(br, need)
    while br.bits < need do
        if br.pos > #br.data then return false end
        br.buf  = br.buf | (br.data:byte(br.pos) << br.bits)
        br.pos  = br.pos + 1
        br.bits = br.bits + 8
    end
    return true
end

local function br_read_lsb(br, nbits)
    if nbits == 0 then return 0 end
    if not br_fill(br, nbits) then return nil end
    local mask = (1 << nbits) - 1
    local val  = br.buf & mask
    br.buf  = br.buf >> nbits
    br.bits = br.bits - nbits
    return val
end

local function br_align(br)
    local discard = br.bits % 8
    if discard > 0 then
        br.buf  = br.buf >> discard
        br.bits = br.bits - discard
    end
end

-- ============================================================================
-- RFC 1951 DEFLATE — Fixed Huffman Tables
-- ============================================================================
--
-- RFC 1951 §3.2.6 specifies fixed (pre-defined) Huffman code lengths.
-- Using BTYPE=01 means we never transmit code tables.
--
-- Literal/Length code lengths:
--   Symbols   0–143: 8-bit codes, starting at 0b00110000 (= 48)
--   Symbols 144–255: 9-bit codes, starting at 0b110010000 (= 400)
--   Symbols 256–279: 7-bit codes, starting at 0b0000000 (= 0)
--   Symbols 280–287: 8-bit codes, starting at 0b11000000 (= 192)
--
-- Distance codes: 5-bit codes equal to the symbol number.

-- fixed_ll_encode returns (code, nbits) for LL symbol 0-287.
local function fixed_ll_encode(sym)
    if sym <= 143 then
        return 0x30 + sym, 8
    elseif sym <= 255 then
        return 0x190 + (sym - 144), 9
    elseif sym <= 279 then
        return sym - 256, 7
    else  -- 280-287
        return 0xC0 + (sym - 280), 8
    end
end

-- ============================================================================
-- RFC 1951 DEFLATE — Length / Distance Tables
-- ============================================================================
--
-- Match lengths (3-258) map to LL symbols 257-285 + extra bits.
-- Match distances (1-32768) map to distance codes 0-29 + extra bits.
-- RFC 1951 §3.2.5: symbol 285 = length 258, 0 extra bits (special case).

-- length_table[i] = {base, extra} for LL symbol 257+i (0-indexed offset from 257).
local LENGTH_TABLE = {
    {3,0},{4,0},{5,0},{6,0},{7,0},{8,0},{9,0},{10,0},   -- 257-264
    {11,1},{13,1},{15,1},{17,1},                          -- 265-268
    {19,2},{23,2},{27,2},{31,2},                          -- 269-272
    {35,3},{43,3},{51,3},{59,3},                          -- 273-276
    {67,4},{83,4},{99,4},{115,4},                         -- 277-280
    {131,5},{163,5},{195,5},{227,5},                      -- 281-284
    {258,0},                                              -- 285
}

-- dist_table[i+1] = {base, extra} for distance code i (0-indexed).
local DIST_TABLE = {
    {1,0},{2,0},{3,0},{4,0},
    {5,1},{7,1},{9,2},{13,2},
    {17,3},{25,3},{33,4},{49,4},
    {65,5},{97,5},{129,6},{193,6},
    {257,7},{385,7},{513,8},{769,8},
    {1025,9},{1537,9},{2049,10},{3073,10},
    {4097,11},{6145,11},{8193,12},{12289,12},
    {16385,13},{24577,13},
}

-- encode_length maps a match length (3-258) to (sym, base, extra).
local function encode_length(length)
    for i = #LENGTH_TABLE, 1, -1 do
        if length >= LENGTH_TABLE[i][1] then
            return 257 + i - 1, LENGTH_TABLE[i][1], LENGTH_TABLE[i][2]
        end
    end
    error("encode_length: unreachable for length=" .. length)
end

-- encode_dist maps a match offset (1-32768) to (code, base, extra).
local function encode_dist(offset)
    for i = #DIST_TABLE, 1, -1 do
        if offset >= DIST_TABLE[i][1] then
            return i - 1, DIST_TABLE[i][1], DIST_TABLE[i][2]
        end
    end
    error("encode_dist: unreachable for offset=" .. offset)
end

-- ============================================================================
-- RFC 1951 DEFLATE — Compress (fixed Huffman, BTYPE=01)
-- ============================================================================

-- deflate_compress compresses a byte string to raw RFC 1951 DEFLATE.
-- Returns a binary string (no zlib wrapper).
local function deflate_compress(data)
    local bw = new_bit_writer()

    if #data == 0 then
        -- Empty stored block: [0x01, 0x00, 0x00, 0xFF, 0xFF]
        bw_write_lsb(bw, 1, 1)  -- BFINAL=1
        bw_write_lsb(bw, 0, 2)  -- BTYPE=00
        bw_align(bw)
        bw_write_lsb(bw, 0x0000, 16)  -- LEN=0
        bw_write_lsb(bw, 0xFFFF, 16)  -- NLEN=~0
        return bw_finish(bw)
    end

    -- Tokenize with LZSS (window=32768, max=255, min=3).
    local bytes = {data:byte(1, #data)}
    local tokens = lzss.encode(bytes, 32768, 255, 3)

    -- Block header: BFINAL=1, BTYPE=01 (fixed Huffman).
    bw_write_lsb(bw, 1, 1)  -- BFINAL
    bw_write_lsb(bw, 1, 1)  -- BTYPE bit 0
    bw_write_lsb(bw, 0, 1)  -- BTYPE bit 1

    for _, tok in ipairs(tokens) do
        if tok.kind == "literal" then
            local code, nbits = fixed_ll_encode(tok.byte)
            bw_write_huffman(bw, code, nbits)
        else
            -- length
            local sym, base_len, extra_len_bits = encode_length(tok.length)
            local ll_code, ll_nbits = fixed_ll_encode(sym)
            bw_write_huffman(bw, ll_code, ll_nbits)
            if extra_len_bits > 0 then
                bw_write_lsb(bw, tok.length - base_len, extra_len_bits)
            end
            -- distance
            local dist_code, base_dist, extra_dist_bits = encode_dist(tok.offset)
            bw_write_huffman(bw, dist_code, 5)
            if extra_dist_bits > 0 then
                bw_write_lsb(bw, tok.offset - base_dist, extra_dist_bits)
            end
        end
    end

    -- End-of-block symbol (256).
    local eob_code, eob_nbits = fixed_ll_encode(256)
    bw_write_huffman(bw, eob_code, eob_nbits)

    return bw_finish(bw)
end

function M.raw_deflate(data)
    return deflate_compress(data)
end

-- ============================================================================
-- RFC 1951 DEFLATE — Decompress
-- ============================================================================
--
-- Handles stored, fixed-Huffman, and dynamic-Huffman blocks. Huffman decoding
-- consumes one physical bit at a time, so counted decoding never reads into an
-- untouched suffix after the final block.

local CODE_LENGTH_ORDER = {
    16, 17, 18, 0, 8, 7, 9, 6, 10,
    5, 11, 4, 12, 3, 13, 2, 14, 1, 15,
}

local function build_huffman(lengths)
    local counts = {[0] = 0}
    for bits = 1, 15 do counts[bits] = 0 end
    for _, length in ipairs(lengths) do
        if length > 0 then counts[length] = counts[length] + 1 end
    end

    local left = 1
    for bits = 1, 15 do
        left = left * 2 - counts[bits]
        if left < 0 then return nil, "huffman-oversubscribed" end
    end

    local next_code = {}
    local code = 0
    for bits = 1, 15 do
        code = (code + counts[bits - 1]) << 1
        next_code[bits] = code
    end

    local by_length = {}
    local symbol_count = 0
    for symbol = 0, #lengths - 1 do
        local length = lengths[symbol + 1]
        if length > 0 then
            by_length[length] = by_length[length] or {}
            by_length[length][next_code[length]] = symbol
            next_code[length] = next_code[length] + 1
            symbol_count = symbol_count + 1
        end
    end

    return {
        by_length = by_length,
        complete = left == 0,
        symbol_count = symbol_count,
        one_bit_count = counts[1],
    }
end

local function decode_symbol(br, tree, invalid_error)
    local code = 0
    for length = 1, 15 do
        local bit = br_read_lsb(br, 1)
        if bit == nil then return nil, "unexpected-eof" end
        code = (code << 1) | bit
        local symbols = tree.by_length[length]
        if symbols ~= nil and symbols[code] ~= nil then
            return symbols[code]
        end
    end
    return nil, invalid_error
end

local function fixed_trees()
    local literal_lengths = {}
    for symbol = 0, 287 do
        local length
        if symbol <= 143 then
            length = 8
        elseif symbol <= 255 then
            length = 9
        elseif symbol <= 279 then
            length = 7
        else
            length = 8
        end
        literal_lengths[symbol + 1] = length
    end

    local distance_lengths = {}
    for symbol = 0, 31 do distance_lengths[symbol + 1] = 5 end
    return assert(build_huffman(literal_lengths)), assert(build_huffman(distance_lengths))
end

local FIXED_LITERAL_TREE, FIXED_DISTANCE_TREE = fixed_trees()

local function read_dynamic_trees(br)
    local raw_hlit = br_read_lsb(br, 5)
    local raw_hdist = br_read_lsb(br, 5)
    local raw_hclen = br_read_lsb(br, 4)
    if raw_hlit == nil or raw_hdist == nil or raw_hclen == nil then
        return nil, nil, "unexpected-eof"
    end

    local hlit = raw_hlit + 257
    local hdist = raw_hdist + 1
    local hclen = raw_hclen + 4
    if hlit > 286 then return nil, nil, "invalid-literal-length-symbol" end

    local code_lengths = {}
    for _ = 0, 18 do code_lengths[#code_lengths + 1] = 0 end
    for index = 1, hclen do
        local length = br_read_lsb(br, 3)
        if length == nil then return nil, nil, "unexpected-eof" end
        code_lengths[CODE_LENGTH_ORDER[index] + 1] = length
    end

    local code_length_tree, tree_error = build_huffman(code_lengths)
    if code_length_tree == nil then return nil, nil, tree_error end
    if not code_length_tree.complete then
        return nil, nil, "incomplete-code-length-tree"
    end

    local lengths = {}
    local target = hlit + hdist
    while #lengths < target do
        local symbol, symbol_error = decode_symbol(
            br, code_length_tree, "invalid-literal-length-symbol")
        if symbol == nil then return nil, nil, symbol_error end

        if symbol <= 15 then
            lengths[#lengths + 1] = symbol
        else
            local repeat_count
            local repeated_length = 0
            if symbol == 16 then
                if #lengths == 0 then
                    return nil, nil, "repeat-without-previous"
                end
                local extra = br_read_lsb(br, 2)
                if extra == nil then return nil, nil, "unexpected-eof" end
                repeat_count = extra + 3
                repeated_length = lengths[#lengths]
            elseif symbol == 17 then
                local extra = br_read_lsb(br, 3)
                if extra == nil then return nil, nil, "unexpected-eof" end
                repeat_count = extra + 3
            else
                local extra = br_read_lsb(br, 7)
                if extra == nil then return nil, nil, "unexpected-eof" end
                repeat_count = extra + 11
            end
            if #lengths + repeat_count > target then
                return nil, nil, "repeat-overrun"
            end
            for _ = 1, repeat_count do
                lengths[#lengths + 1] = repeated_length
            end
        end
    end

    local literal_lengths = {}
    for index = 1, hlit do literal_lengths[index] = lengths[index] end
    local distance_lengths = {}
    for index = 1, hdist do
        distance_lengths[index] = lengths[hlit + index]
    end

    local literal_tree, literal_error = build_huffman(literal_lengths)
    if literal_tree == nil then return nil, nil, literal_error end
    if not literal_tree.complete then
        return nil, nil, "incomplete-literal-length-tree"
    end

    local distance_tree, distance_error = build_huffman(distance_lengths)
    if distance_tree == nil then return nil, nil, distance_error end
    local distance_is_valid = distance_tree.complete
        or (distance_tree.symbol_count == 1 and distance_tree.one_bit_count == 1)
        or distance_tree.symbol_count == 0
    if not distance_is_valid then
        return nil, nil, "incomplete-distance-tree"
    end
    return literal_tree, distance_tree
end

-- Keep inflated bytes in completed strings plus one small numeric tail.  A
-- dense Lua table costs roughly sixteen bytes per logical byte on the standard
-- 64-bit VM, so the old one-number-per-byte representation could consume more
-- than four gigabytes at the public 256 MiB output ceiling.
local OUTPUT_CHUNK_SIZE = 4096

local function new_output_buffer()
    return {chunks = {}, tail = {}, total = 0}
end

local function output_append(output, byte)
    output.total = output.total + 1
    output.tail[#output.tail + 1] = byte
    if #output.tail == OUTPUT_CHUNK_SIZE then
        output.chunks[#output.chunks + 1] = string.char(table.unpack(output.tail))
        output.tail = {}
    end
end

local function output_byte(output, index)
    local completed = #output.chunks * OUTPUT_CHUNK_SIZE
    if index <= completed then
        local chunk_index = ((index - 1) // OUTPUT_CHUNK_SIZE) + 1
        local chunk_offset = ((index - 1) % OUTPUT_CHUNK_SIZE) + 1
        return output.chunks[chunk_index]:byte(chunk_offset)
    end
    return output.tail[index - completed]
end

local function output_string(output)
    if #output.tail > 0 then
        output.chunks[#output.chunks + 1] = string.char(table.unpack(output.tail))
        output.tail = {}
    end
    return table.concat(output.chunks)
end

local function decode_compressed_block(br, literal_tree, distance_tree,
                                       output, total, max_output)
    while true do
        local symbol, symbol_error = decode_symbol(
            br, literal_tree, "invalid-literal-length-symbol")
        if symbol == nil then return nil, symbol_error end

        if symbol < 256 then
            if total >= max_output then return nil, "output-limit-exceeded" end
            total = total + 1
            output_append(output, symbol)
        elseif symbol == 256 then
            return total
        elseif symbol <= 285 then
            local length_spec = LENGTH_TABLE[symbol - 256]
            local extra_length = br_read_lsb(br, length_spec[2])
            if extra_length == nil then return nil, "unexpected-eof" end
            local length = length_spec[1] + extra_length

            local distance_symbol, distance_error = decode_symbol(
                br, distance_tree, "reserved-distance-symbol")
            if distance_symbol == nil then return nil, distance_error end
            if distance_symbol >= 30 then return nil, "reserved-distance-symbol" end
            local distance_spec = DIST_TABLE[distance_symbol + 1]
            local extra_distance = br_read_lsb(br, distance_spec[2])
            if extra_distance == nil then return nil, "unexpected-eof" end
            local distance = distance_spec[1] + extra_distance
            if distance < 1 or distance > total then
                return nil, "invalid-back-reference"
            end
            if total + length > max_output then
                return nil, "output-limit-exceeded"
            end

            for _ = 1, length do
                if total >= max_output then
                    return nil, "output-limit-exceeded"
                end
                local source = total - distance + 1
                local byte = output_byte(output, source)
                total = total + 1
                output_append(output, byte)
            end
        else
            return nil, "invalid-literal-length-symbol"
        end
    end
end

local function inflate_counted(data, max_output)
    max_output = max_output == nil and M.RAW_INFLATE_MAX_OUTPUT or max_output
    if type(max_output) ~= "number"
        or max_output ~= math.floor(max_output)
        or max_output < 0
        or max_output > M.RAW_INFLATE_MAX_OUTPUT then
        return nil, "invalid-output-limit"
    end

    local br = new_bit_reader(data)
    local output = new_output_buffer()
    local total = 0

    while true do
        local bfinal = br_read_lsb(br, 1)
        local btype = br_read_lsb(br, 2)
        if bfinal == nil or btype == nil then return nil, "unexpected-eof" end

        if btype == 0 then
            br_align(br)
            local stored_length = br_read_lsb(br, 16)
            local stored_inverse = br_read_lsb(br, 16)
            if stored_length == nil or stored_inverse == nil then
                return nil, "unexpected-eof"
            end
            if (stored_inverse ~ 0xffff) ~= stored_length then
                return nil, "stored-length-mismatch"
            end
            if total + stored_length > max_output then
                return nil, "output-limit-exceeded"
            end
            for _ = 1, stored_length do
                if total >= max_output then
                    return nil, "output-limit-exceeded"
                end
                local byte = br_read_lsb(br, 8)
                if byte == nil then return nil, "unexpected-eof" end
                total = total + 1
                output_append(output, byte)
            end
        elseif btype == 1 or btype == 2 then
            local literal_tree = FIXED_LITERAL_TREE
            local distance_tree = FIXED_DISTANCE_TREE
            if btype == 2 then
                local tree_error
                literal_tree, distance_tree, tree_error = read_dynamic_trees(br)
                if literal_tree == nil then return nil, tree_error end
            end
            local block_total, block_error = decode_compressed_block(
                br, literal_tree, distance_tree, output, total, max_output)
            if block_total == nil then return nil, block_error end
            total = block_total
        else
            return nil, "reserved-block-type"
        end

        if bfinal == 1 then
            return {
                output = output_string(output),
                bytes_consumed = br.pos - 1,
            }
        end
    end
end

function M.raw_inflate_counted(data, max_output)
    return inflate_counted(data, max_output)
end

function M.raw_inflate(data, max_output)
    local result, inflate_error = inflate_counted(data, max_output)
    if result == nil then return nil, inflate_error end
    return result.output
end

-- ============================================================================
-- Entry name validation
-- ============================================================================

local function validate_entry_name(name)
    if name:find("\0", 1, true) then
        return nil, "zip: entry name contains null byte"
    end
    if name:find("\\", 1, true) then
        return nil, "zip: entry name contains backslash"
    end
    if name:sub(1, 1) == "/" then
        return nil, "zip: entry name is an absolute path: " .. name
    end
    for segment in (name .. "/"):gmatch("([^/]*)/") do
        if segment == ".." then
            return nil, "zip: entry name contains path traversal (..): " .. name
        end
    end
    return true
end

-- ============================================================================
-- ZIP Write — ZipWriter
-- ============================================================================
--
-- ZipWriter accumulates entries in memory: for each file it writes a Local
-- File Header + data, records CD metadata, and assembles the full archive
-- on finish().
--
-- Auto-compression policy:
--   - Try DEFLATE. Use method=8 only if compressed < original.
--   - Otherwise use method=0 (Stored).

-- new_writer creates a new ZipWriter state table.
function M.new_writer()
    return {buf = {}, entries = {}}
end

-- add_file adds a file entry to the writer.
--
-- @param writer   (table)   ZipWriter from new_writer().
-- @param name     (string)  entry name (UTF-8).
-- @param data     (string)  file content as binary string.
-- @param compress (boolean) optional; default true.
function M.add_file(writer, name, data, compress)
    if compress == nil then compress = true end
    _add_entry(writer, name, data, compress, 0x81A4)  -- 0o100644 octal
end

-- add_directory adds a directory entry (name should end with '/').
--
-- @param writer (table)  ZipWriter from new_writer().
-- @param name   (string) directory name ending with '/'.
function M.add_directory(writer, name)
    _add_entry(writer, name, "", false, 0x41ED)  -- 0o040755 octal
end

-- _add_entry is the internal implementation for both add_file and add_directory.
_add_entry = function(writer, name, data, compress, unix_mode)
    -- Validate on the write path too — refuse to produce archives with
    -- path-traversal names that other tools might extract unsafely.
    local ok, err = validate_entry_name(name)
    if not ok then error(err) end

    local checksum = M.crc32(data)
    local uncompressed_size = #data

    local method, file_data
    if compress and #data > 0 then
        local compressed = deflate_compress(data)
        if #compressed < #data then
            method    = 8
            file_data = compressed
        else
            method    = 0
            file_data = data
        end
    else
        method    = 0
        file_data = data
    end

    local compressed_size = #file_data
    local local_offset    = 0
    for _, s in ipairs(writer.buf) do
        local_offset = local_offset + #s
    end
    local version_needed = (method == 8) and 20 or 10

    -- Local File Header
    writer.buf[#writer.buf + 1] = le32(0x04034B50)                   -- signature
    writer.buf[#writer.buf + 1] = le16(version_needed)
    writer.buf[#writer.buf + 1] = le16(0x0800)                       -- flags (UTF-8)
    writer.buf[#writer.buf + 1] = le16(method)
    writer.buf[#writer.buf + 1] = le16(M.DOS_EPOCH & 0xFFFF)         -- mod_time
    writer.buf[#writer.buf + 1] = le16((M.DOS_EPOCH >> 16) & 0xFFFF) -- mod_date
    writer.buf[#writer.buf + 1] = le32(checksum)
    writer.buf[#writer.buf + 1] = le32(compressed_size)
    writer.buf[#writer.buf + 1] = le32(uncompressed_size)
    writer.buf[#writer.buf + 1] = le16(#name)
    writer.buf[#writer.buf + 1] = le16(0)                            -- extra_len = 0
    writer.buf[#writer.buf + 1] = name
    writer.buf[#writer.buf + 1] = file_data

    writer.entries[#writer.entries + 1] = {
        name              = name,
        method            = method,
        crc               = checksum,
        compressed_size   = compressed_size,
        uncompressed_size = uncompressed_size,
        local_offset      = local_offset,
        external_attrs    = (unix_mode << 16),
    }
end

-- finish appends the Central Directory and EOCD, returns the archive as a string.
--
-- @param writer (table) ZipWriter from new_writer().
-- @return (string) complete ZIP archive binary.
function M.finish(writer)
    -- Compute current buffer size (cd_offset).
    local cd_offset = 0
    for _, s in ipairs(writer.buf) do
        cd_offset = cd_offset + #s
    end

    -- Central Directory
    local cd_parts = {}
    for _, e in ipairs(writer.entries) do
        local version_needed = (e.method == 8) and 20 or 10
        cd_parts[#cd_parts + 1] = le32(0x02014B50)                   -- CD signature
        cd_parts[#cd_parts + 1] = le16(0x031E)                       -- version made by
        cd_parts[#cd_parts + 1] = le16(version_needed)
        cd_parts[#cd_parts + 1] = le16(0x0800)                       -- flags (UTF-8)
        cd_parts[#cd_parts + 1] = le16(e.method)
        cd_parts[#cd_parts + 1] = le16(M.DOS_EPOCH & 0xFFFF)         -- mod_time
        cd_parts[#cd_parts + 1] = le16((M.DOS_EPOCH >> 16) & 0xFFFF) -- mod_date
        cd_parts[#cd_parts + 1] = le32(e.crc)
        cd_parts[#cd_parts + 1] = le32(e.compressed_size)
        cd_parts[#cd_parts + 1] = le32(e.uncompressed_size)
        cd_parts[#cd_parts + 1] = le16(#e.name)
        cd_parts[#cd_parts + 1] = le16(0)                            -- extra_len
        cd_parts[#cd_parts + 1] = le16(0)                            -- comment_len
        cd_parts[#cd_parts + 1] = le16(0)                            -- disk_start
        cd_parts[#cd_parts + 1] = le16(0)                            -- internal_attrs
        cd_parts[#cd_parts + 1] = le32(e.external_attrs)
        cd_parts[#cd_parts + 1] = le32(e.local_offset)
        cd_parts[#cd_parts + 1] = e.name
    end
    local cd_str  = table.concat(cd_parts)
    local cd_size = #cd_str
    local num_entries = #writer.entries

    -- End of Central Directory Record
    local eocd = table.concat({
        le32(0x06054B50),   -- EOCD signature
        le16(0),            -- disk_number
        le16(0),            -- cd_disk
        le16(num_entries),  -- entries this disk
        le16(num_entries),  -- entries total
        le32(cd_size),
        le32(cd_offset),
        le16(0),            -- comment_len
    })

    -- Combine everything.
    local all = {}
    for _, s in ipairs(writer.buf) do all[#all + 1] = s end
    all[#all + 1] = cd_str
    all[#all + 1] = eocd
    return table.concat(all)
end

-- ============================================================================
-- ZIP Read — ZipEntry and ZipReader
-- ============================================================================
--
-- ZipReader uses the "EOCD-first" strategy:
--   1. Scan backwards for EOCD signature.
--   2. Read CD offset and size.
--   3. Parse all CD headers.
--   4. On read(entry): skip local header, decompress, verify CRC-32.

-- find_eocd scans backwards from end of data for the EOCD signature.
-- Returns position (1-indexed) or nil.
local function find_eocd(data)
    local min_size   = 22
    local max_comment = 65535
    if #data < min_size then return nil end

    local scan_start = math.max(1, #data - min_size - max_comment + 1)
    for i = #data - min_size + 1, scan_start, -1 do
        local sig = read_le32(data, i)
        if sig == 0x06054B50 then
            local comment_len = read_le16(data, i + 20)
            if comment_len ~= nil and i - 1 + min_size + comment_len == #data then
                return i
            end
        end
    end
    return nil
end

-- new_reader parses a ZIP archive binary string.
-- Returns a reader table, or nil + error message on failure.
function M.new_reader(data)
    local eocd_pos = find_eocd(data)
    if eocd_pos == nil then
        return nil, "zip: no End of Central Directory record found"
    end

    local cd_size   = read_le32(data, eocd_pos + 12)
    local cd_offset = read_le32(data, eocd_pos + 16)
    if cd_size == nil or cd_offset == nil then
        return nil, "zip: EOCD too short"
    end
    -- read_le32 returns a signed 64-bit Lua integer; 0xFFFFFFFF arrives as -1.
    -- Guard against sign-extended "huge" values that would pass the bounds check
    -- due to negative arithmetic.
    if cd_size < 0 or cd_offset < 0 then
        return nil, "zip: EOCD cd_size or cd_offset has high bit set (>2 GiB); ZIP64 not supported"
    end
    if cd_offset + cd_size > #data then
        return nil, "zip: Central Directory out of bounds"
    end

    local entries = {}
    local pos     = cd_offset + 1  -- convert to 1-indexed

    while pos + 3 <= cd_offset + cd_size do
        local sig = read_le32(data, pos)
        if sig ~= 0x02014B50 then break end

        if pos - 1 + 46 > #data then
            return nil, "zip: CD entry header out of bounds"
        end

        local method         = read_le16(data, pos + 10)
        local crc            = read_le32(data, pos + 16)
        local compressed_sz  = read_le32(data, pos + 20)
        local size           = read_le32(data, pos + 24)
        local name_len       = read_le16(data, pos + 28)
        local extra_len      = read_le16(data, pos + 30)
        local comment_len    = read_le16(data, pos + 32)
        local local_offset   = read_le32(data, pos + 42)

        if not (method and crc and compressed_sz and size and
                name_len and extra_len and comment_len and local_offset) then
            return nil, "zip: CD entry fields truncated"
        end
        -- Guard sign-extended 32-bit values.
        if compressed_sz < 0 or size < 0 or local_offset < 0 then
            return nil, "zip: CD entry has >2 GiB field; ZIP64 not supported"
        end

        local name_start = pos + 46
        local name_end   = name_start + name_len - 1
        if name_end > #data then
            return nil, "zip: CD entry name out of bounds"
        end

        local name = data:sub(name_start, name_end)

        -- Validate entry name.
        local ok, err = validate_entry_name(name)
        if not ok then return nil, err end

        local next_pos = name_end + 1 + extra_len + comment_len
        if next_pos - 1 > cd_offset + cd_size then
            return nil, "zip: CD entry advance out of bounds"
        end

        entries[#entries + 1] = {
            name            = name,
            size            = size,
            compressed_size = compressed_sz,
            method          = method,
            crc32           = crc,
            is_directory    = name:sub(-1) == "/",
            local_offset    = local_offset,
        }
        pos = next_pos
    end

    return {data = data, entries = entries, cd_offset = cd_offset}
end

-- reader_entries returns all entries in the archive.
--
-- @param reader (table) from new_reader().
-- @return (table) array of entry tables.
function M.reader_entries(reader)
    return reader.entries
end

-- reader_read decompresses and CRC-validates one entry.
-- Returns data string, or nil + error message.
--
-- @param reader (table) from new_reader().
-- @param entry  (table) from reader_entries().
-- @return (string|nil, string|nil) data, errmsg.
function M.reader_read(reader, entry)
    if entry.is_directory then return "" end

    local data   = reader.data
    local lh_off = entry.local_offset + 1  -- 1-indexed

    -- Validate that local_offset points into the local-file region, not into
    -- the Central Directory or beyond. A crafted archive could set local_offset
    -- to any byte position to read unrelated regions of the archive.
    if reader.cd_offset ~= nil and entry.local_offset >= reader.cd_offset then
        return nil, "zip: local header offset for '" .. entry.name .. "' points into Central Directory"
    end

    -- Check local flags (encryption bit).
    local local_flags = read_le16(data, lh_off + 6)
    if local_flags == nil then
        return nil, "zip: local header out of bounds"
    end
    if local_flags & 1 ~= 0 then
        return nil, "zip: entry '" .. entry.name .. "' is encrypted"
    end

    local lh_name_len  = read_le16(data, lh_off + 26)
    local lh_extra_len = read_le16(data, lh_off + 28)
    if lh_name_len == nil or lh_extra_len == nil then
        return nil, "zip: local header fields out of bounds for '" .. entry.name .. "'"
    end

    local data_start = lh_off + 30 + lh_name_len + lh_extra_len
    -- Ensure the header fields don't advance data_start into the CD.
    if reader.cd_offset ~= nil and data_start > reader.cd_offset + 1 then
        return nil, "zip: local header fields advance into Central Directory for '" .. entry.name .. "'"
    end
    local data_end   = data_start + entry.compressed_size - 1
    if data_end > #data then
        return nil, "zip: entry '" .. entry.name .. "' data out of bounds"
    end

    local compressed = data:sub(data_start, data_end)

    local decompressed
    if entry.method == 0 then
        decompressed = compressed
    elseif entry.method == 8 then
        local result, err = M.raw_inflate_counted(compressed, entry.size)
        if result == nil then
            return nil, "zip: entry '" .. entry.name .. "': " .. err
        end
        if result.bytes_consumed ~= #compressed then
            return nil, "zip: compressed payload contains trailing bytes"
        end
        decompressed = result.output
    else
        return nil, "zip: unsupported compression method " .. entry.method
    end

    if #decompressed ~= entry.size then
        return nil, "zip: uncompressed size does not match the directory"
    end

    -- Verify CRC-32.
    local actual_crc = M.crc32(decompressed)
    if actual_crc ~= entry.crc32 then
        return nil, string.format(
            "zip: CRC-32 mismatch for '%s': expected %08X, got %08X",
            entry.name, entry.crc32, actual_crc)
    end

    return decompressed
end

-- read_by_name finds an entry by name and returns its data.
--
-- @param reader (table)  from new_reader().
-- @param name   (string) entry name.
-- @return (string|nil, string|nil) data, errmsg.
function M.read_by_name(reader, name)
    for _, entry in ipairs(reader.entries) do
        if entry.name == name then
            return M.reader_read(reader, entry)
        end
    end
    return nil, "zip: entry '" .. name .. "' not found"
end

-- ============================================================================
-- Convenience Functions
-- ============================================================================

-- zip compresses a list of {name, data} pairs into a ZIP archive.
--
-- @param entries  (table)   array of {name=string, data=string} or {string, string}.
-- @param compress (boolean) optional; default true.
-- @return (string) ZIP archive binary.
function M.zip(entries, compress)
    if compress == nil then compress = true end
    local w = M.new_writer()
    for _, e in ipairs(entries) do
        local name = e.name or e[1]
        local data = e.data or e[2]
        M.add_file(w, name, data, compress)
    end
    return M.finish(w)
end

-- unzip decompresses all file entries from a ZIP archive.
--
-- Returns a table mapping name → data string.
-- Throws (via error()) on CRC mismatch, unsupported method, or corrupt data.
-- Raises an error on duplicate entry names.
--
-- @param data (string) ZIP archive binary.
-- @return (table) {name = data, ...}
function M.unzip(data)
    local reader, err = M.new_reader(data)
    if reader == nil then error(err) end
    local result = {}
    for _, entry in ipairs(reader.entries) do
        if not entry.is_directory then
            if result[entry.name] ~= nil then
                error("zip: duplicate entry name '" .. entry.name .. "'")
            end
            local content, cerr = M.reader_read(reader, entry)
            if content == nil then error(cerr) end
            result[entry.name] = content
        end
    end
    return result
end

return M
