-- ============================================================================
-- CodingAdventures.Deflate (coding_adventures.deflate)
-- ============================================================================
--
-- DEFLATE lossless compression algorithm (1996, RFC 1951).
-- Part of the CMP compression series in the coding-adventures monorepo.
--
-- What Is DEFLATE?
-- ----------------
--
-- DEFLATE is the dominant general-purpose lossless compression algorithm,
-- powering ZIP, gzip, PNG, and HTTP/2 HPACK header compression. It combines:
--
--   Pass 1 — LZSS tokenization (CMP02): replace repeated substrings with
--            back-references into a 4096-byte sliding window.
--
--   Pass 2 — Dual canonical Huffman coding (DT27): entropy-code the token
--            stream with two separate Huffman trees:
--              LL tree:   literals (0-255), end-of-data (256), lengths (257-284)
--              Dist tree: distance codes (0-23, for offsets 1-4096)
--
-- The key insight: LZ removes patterns; Huffman removes symbol-frequency bias
-- in the remaining data. On typical text, DEFLATE achieves 60-70% reduction.
--
-- Wire Format (CMP05)
-- -------------------
--
--   [4B] original_length    big-endian uint32
--   [2B] ll_entry_count     big-endian uint16
--   [2B] dist_entry_count   big-endian uint16 (0 if no matches)
--   [ll_entry_count × 3B]   (symbol uint16 BE, code_length uint8)
--   [dist_entry_count × 3B] same format
--   [remaining bytes]       LSB-first packed bit stream
--
-- Dependencies
-- ------------
--
--   coding_adventures.huffman_tree  (DT27) — Huffman tree builder
--   coding_adventures.lzss          (CMP02) — LZSS tokenizer
--
-- ============================================================================

local HuffmanTree = require("coding_adventures.huffman_tree")
local LZSS = require("coding_adventures.lzss")

local M = {}
M.VERSION = "0.1.0"

-- ---------------------------------------------------------------------------
-- Length code table (LL symbols 257-284)
-- ---------------------------------------------------------------------------
--
-- Each length symbol covers a range of match lengths. The exact length is
-- encoded as extra_bits raw bits after the Huffman code.

local LENGTH_TABLE = {
  --  {symbol, base_length, extra_bits}
  {257,   3, 0}, {258,   4, 0}, {259,   5, 0}, {260,   6, 0},
  {261,   7, 0}, {262,   8, 0}, {263,   9, 0}, {264,  10, 0},
  {265,  11, 1}, {266,  13, 1}, {267,  15, 1}, {268,  17, 1},
  {269,  19, 2}, {270,  23, 2}, {271,  27, 2}, {272,  31, 2},
  {273,  35, 3}, {274,  43, 3}, {275,  51, 3}, {276,  59, 3},
  {277,  67, 4}, {278,  83, 4}, {279,  99, 4}, {280, 115, 4},
  {281, 131, 5}, {282, 163, 5}, {283, 195, 5}, {284, 227, 5},
}

-- Build fast-lookup tables from the length table.
local LENGTH_BASE = {}
local LENGTH_EXTRA = {}
for _, e in ipairs(LENGTH_TABLE) do
  LENGTH_BASE[e[1]] = e[2]
  LENGTH_EXTRA[e[1]] = e[3]
end

-- ---------------------------------------------------------------------------
-- Distance code table (codes 0-23)
-- ---------------------------------------------------------------------------

local DIST_TABLE = {
  --  {code, base_dist, extra_bits}
  { 0,    1,  0}, { 1,    2,  0}, { 2,    3,  0}, { 3,    4,  0},
  { 4,    5,  1}, { 5,    7,  1}, { 6,    9,  2}, { 7,   13,  2},
  { 8,   17,  3}, { 9,   25,  3}, {10,   33,  4}, {11,   49,  4},
  {12,   65,  5}, {13,   97,  5}, {14,  129,  6}, {15,  193,  6},
  {16,  257,  7}, {17,  385,  7}, {18,  513,  8}, {19,  769,  8},
  {20, 1025,  9}, {21, 1537,  9}, {22, 2049, 10}, {23, 3073, 10},
}

local DIST_BASE = {}
local DIST_EXTRA = {}
for _, e in ipairs(DIST_TABLE) do
  DIST_BASE[e[1]] = e[2]
  DIST_EXTRA[e[1]] = e[3]
end

-- ---------------------------------------------------------------------------
-- Helpers
-- ---------------------------------------------------------------------------

local function length_symbol(length)
  for _, e in ipairs(LENGTH_TABLE) do
    local max_len = e[2] + (1 << e[3]) - 1
    if length <= max_len then return e[1] end
  end
  return 284
end

local function dist_code(offset)
  for _, e in ipairs(DIST_TABLE) do
    local max_dist = e[2] + (1 << e[3]) - 1
    if offset <= max_dist then return e[1] end
  end
  return 23
end

-- ---------------------------------------------------------------------------
-- Bit I/O helpers
-- ---------------------------------------------------------------------------

-- Pack a list of 0/1 integers into bytes, LSB-first.
-- The first bit occupies bit 0 (LSB) of the first byte.
local function pack_bits_lsb_first(bits)
  local out = {}
  local byte_val = 0
  local bit_pos = 0
  for _, b in ipairs(bits) do
    if b == 1 then
      byte_val = byte_val | (1 << bit_pos)
    end
    bit_pos = bit_pos + 1
    if bit_pos == 8 then
      out[#out + 1] = string.char(byte_val)
      byte_val = 0
      bit_pos = 0
    end
  end
  if bit_pos > 0 then
    out[#out + 1] = string.char(byte_val)
  end
  return table.concat(out)
end

-- Expand bytes into a list of 0/1 integers, LSB-first.
local function unpack_bits_lsb_first(data)
  local bits = {}
  for i = 1, #data do
    local byte_val = data:byte(i)
    for j = 0, 7 do
      bits[#bits + 1] = (byte_val >> j) & 1
    end
  end
  return bits
end

-- Read n raw bits from bits (list of 0/1), LSB-first.
local function read_bits(bits, pos, n)
  local val = 0
  for i = 0, n - 1 do
    val = val | (bits[pos + i] << i)
  end
  return val, pos + n
end

-- Decode one Huffman symbol by reading bits until a prefix match.
local function next_huffman_symbol(bits, pos, rev_map)
  local acc = ""
  while true do
    acc = acc .. tostring(bits[pos] or 0)
    pos = pos + 1
    local sym = rev_map[acc]
    if sym then return sym, pos end
  end
end

-- Reconstruct canonical codes (bit_string → symbol) from sorted pairs.
local function reconstruct_canonical_codes(lengths)
  if #lengths == 0 then return {} end
  if #lengths == 1 then return {["0"] = lengths[1][1]} end
  local result = {}
  local code = 0
  local prev_len = lengths[1][2]
  for _, pair in ipairs(lengths) do
    local sym, code_len = pair[1], pair[2]
    if code_len > prev_len then
      code = code << (code_len - prev_len)
    end
    -- Format code as zero-padded binary string of length code_len.
    local bit_str = ""
    for i = code_len - 1, 0, -1 do
      bit_str = bit_str .. tostring((code >> i) & 1)
    end
    result[bit_str] = sym
    code = code + 1
    prev_len = code_len
  end
  return result
end

-- Pack a big-endian 4-byte integer into a string.
local function pack_uint32_be(n)
  return string.char(
    (n >> 24) & 0xFF,
    (n >> 16) & 0xFF,
    (n >>  8) & 0xFF,
     n        & 0xFF
  )
end

-- Pack a big-endian 2-byte integer into a string.
local function pack_uint16_be(n)
  return string.char((n >> 8) & 0xFF, n & 0xFF)
end

-- Unpack a big-endian 4-byte integer from a string at position pos (1-based).
local function unpack_uint32_be(data, pos)
  local a, b, c, d = data:byte(pos, pos + 3)
  return (a << 24) | (b << 16) | (c << 8) | d
end

-- Unpack a big-endian 2-byte integer from a string at position pos (1-based).
local function unpack_uint16_be(data, pos)
  local a, b = data:byte(pos, pos + 1)
  return (a << 8) | b
end

-- ---------------------------------------------------------------------------
-- Public API: compress
-- ---------------------------------------------------------------------------

--- Compress a string using DEFLATE (CMP05).
-- @param data string  The data to compress.
-- @return string  Compressed bytes in CMP05 wire format.
function M.compress(data)
  local original_length = #data

  if original_length == 0 then
    -- Empty input: LL tree has only symbol 256 (end-of-data), code "0".
    local header = pack_uint32_be(0) .. pack_uint16_be(1) .. pack_uint16_be(0)
    local ll_entry = pack_uint16_be(256) .. string.char(1)  -- symbol=256, len=1
    local bit_stream = string.char(0x00)  -- code "0" → 1 bit → 0x00
    return header .. ll_entry .. bit_stream
  end

  -- Pass 1: LZSS tokenization.
  -- encode_string accepts a Lua string (encode takes a byte-array table).
  local tokens = LZSS.encode_string(data)

  -- Pass 2a: Tally frequencies.
  local ll_freq = {}
  local dist_freq = {}

  for _, tok in ipairs(tokens) do
    if tok.kind == "literal" then
      ll_freq[tok.byte] = (ll_freq[tok.byte] or 0) + 1
    else
      local sym = length_symbol(tok.length)
      ll_freq[sym] = (ll_freq[sym] or 0) + 1
      local dc = dist_code(tok.offset)
      dist_freq[dc] = (dist_freq[dc] or 0) + 1
    end
  end
  ll_freq[256] = (ll_freq[256] or 0) + 1

  -- Pass 2b: Build canonical Huffman trees.
  local ll_weights = {}
  for sym, freq in pairs(ll_freq) do
    ll_weights[#ll_weights + 1] = {sym, freq}
  end
  local ll_tree = HuffmanTree.build(ll_weights)
  local ll_code_table = ll_tree:canonical_code_table()  -- {symbol → bit_string}

  local dist_code_table = {}
  if next(dist_freq) ~= nil then
    local dist_weights = {}
    for sym, freq in pairs(dist_freq) do
      dist_weights[#dist_weights + 1] = {sym, freq}
    end
    local dist_tree = HuffmanTree.build(dist_weights)
    dist_code_table = dist_tree:canonical_code_table()
  end

  -- Pass 2c: Encode token stream to bit list.
  local bit_list = {}

  local function append_bits(str)
    for i = 1, #str do
      bit_list[#bit_list + 1] = tonumber(str:sub(i, i))
    end
  end

  local function append_raw_bits_lsb(val, n)
    for i = 0, n - 1 do
      bit_list[#bit_list + 1] = (val >> i) & 1
    end
  end

  for _, tok in ipairs(tokens) do
    if tok.kind == "literal" then
      append_bits(ll_code_table[tok.byte])
    else
      local sym = length_symbol(tok.length)
      local extra_bits_count = LENGTH_EXTRA[sym]
      local extra_val = tok.length - LENGTH_BASE[sym]

      local dc = dist_code(tok.offset)
      local dist_extra_bits = DIST_EXTRA[dc]
      local dist_extra_val = tok.offset - DIST_BASE[dc]

      append_bits(ll_code_table[sym])
      append_raw_bits_lsb(extra_val, extra_bits_count)
      append_bits(dist_code_table[dc])
      append_raw_bits_lsb(dist_extra_val, dist_extra_bits)
    end
  end

  -- End-of-data symbol.
  append_bits(ll_code_table[256])

  local bit_stream = pack_bits_lsb_first(bit_list)

  -- Assemble wire format.
  -- Sorted (symbol, code_length) pairs.
  local ll_lengths = {}
  for sym, code in pairs(ll_code_table) do
    ll_lengths[#ll_lengths + 1] = {sym, #code}
  end
  table.sort(ll_lengths, function(a, b)
    if a[2] ~= b[2] then return a[2] < b[2] else return a[1] < b[1] end
  end)

  local dist_lengths = {}
  for sym, code in pairs(dist_code_table) do
    dist_lengths[#dist_lengths + 1] = {sym, #code}
  end
  table.sort(dist_lengths, function(a, b)
    if a[2] ~= b[2] then return a[2] < b[2] else return a[1] < b[1] end
  end)

  local header = pack_uint32_be(original_length)
      .. pack_uint16_be(#ll_lengths)
      .. pack_uint16_be(#dist_lengths)

  local ll_bytes = {}
  for _, pair in ipairs(ll_lengths) do
    ll_bytes[#ll_bytes + 1] = pack_uint16_be(pair[1]) .. string.char(pair[2])
  end

  local dist_bytes = {}
  for _, pair in ipairs(dist_lengths) do
    dist_bytes[#dist_bytes + 1] = pack_uint16_be(pair[1]) .. string.char(pair[2])
  end

  return header .. table.concat(ll_bytes) .. table.concat(dist_bytes) .. bit_stream
end

-- ---------------------------------------------------------------------------
-- Public API: decompress
-- ---------------------------------------------------------------------------

--- Decompress CMP05 wire-format data.
-- @param data string  Compressed bytes from compress().
-- @return string  The original uncompressed data.
function M.decompress(data)
  if #data < 8 then return "" end

  local original_length = unpack_uint32_be(data, 1)
  local ll_entry_count = unpack_uint16_be(data, 5)
  local dist_entry_count = unpack_uint16_be(data, 7)

  if original_length == 0 then return "" end

  local off = 9  -- 1-based, after 8-byte header

  -- Parse LL code-length table.
  local ll_lengths = {}
  for _ = 1, ll_entry_count do
    local sym = unpack_uint16_be(data, off)
    local code_len = data:byte(off + 2)
    ll_lengths[#ll_lengths + 1] = {sym, code_len}
    off = off + 3
  end

  -- Parse dist code-length table.
  local dist_lengths = {}
  for _ = 1, dist_entry_count do
    local sym = unpack_uint16_be(data, off)
    local code_len = data:byte(off + 2)
    dist_lengths[#dist_lengths + 1] = {sym, code_len}
    off = off + 3
  end

  -- Reconstruct canonical codes (bit_string → symbol).
  local ll_rev_map = reconstruct_canonical_codes(ll_lengths)
  local dist_rev_map = reconstruct_canonical_codes(dist_lengths)

  -- Unpack bit stream.
  local bits = unpack_bits_lsb_first(data:sub(off))
  local bit_pos = 1  -- 1-based index into bits

  -- Decode token stream.
  local output = {}
  while true do
    local ll_sym
    ll_sym, bit_pos = next_huffman_symbol(bits, bit_pos, ll_rev_map)

    if ll_sym == 256 then
      break  -- end-of-data
    elseif ll_sym < 256 then
      output[#output + 1] = ll_sym  -- literal byte
    else
      -- Length code 257-284.
      local extra = LENGTH_EXTRA[ll_sym]
      local extra_val, new_pos = read_bits(bits, bit_pos, extra)
      bit_pos = new_pos
      local length_val = LENGTH_BASE[ll_sym] + extra_val

      local dist_sym
      dist_sym, bit_pos = next_huffman_symbol(bits, bit_pos, dist_rev_map)
      local dextra = DIST_EXTRA[dist_sym]
      local dextra_val, new_pos2 = read_bits(bits, bit_pos, dextra)
      bit_pos = new_pos2
      local offset_val = DIST_BASE[dist_sym] + dextra_val

      -- Copy byte-by-byte from the back-reference position.
      -- Byte-by-byte is required for overlapping matches (offset < length).
      local start = #output - offset_val + 1  -- 1-based
      for _ = 1, length_val do
        output[#output + 1] = output[start]
        start = start + 1
      end
    end
  end

  -- Convert output byte list to string.
  local chars = {}
  for _, b in ipairs(output) do
    chars[#chars + 1] = string.char(b)
  end
  return table.concat(chars)
end

return M
