-- ============================================================================
-- CodingAdventures.Brotli (coding_adventures.brotli)
-- ============================================================================
--
-- Brotli lossless compression algorithm (2013, RFC 7932).
-- Part of the CMP compression series in the coding-adventures monorepo.
--
-- What Is Brotli?
-- ---------------
--
-- Brotli is a lossless compression algorithm developed at Google that achieves
-- significantly better compression ratios than DEFLATE, particularly on web
-- content (HTML, CSS, JavaScript). It became the dominant algorithm for HTTP
-- `Content-Encoding: br` compression.
--
-- Brotli builds on DEFLATE's foundation (LZ matching + Huffman coding) but
-- adds three major innovations:
--
--   1. Context-dependent literal trees — instead of one Huffman tree for all
--      literals, Brotli assigns each literal to one of 4 context buckets based
--      on the preceding byte. Each context bucket gets its own Huffman tree.
--
--   2. Insert-and-copy commands — instead of DEFLATE's flat stream of literal
--      and back-reference tokens, Brotli uses commands that bundle an insert
--      run (raw literals) with a copy operation (back-reference). The lengths
--      of both halves are encoded as a single Huffman symbol (ICC code).
--
--   3. Larger sliding window — 65535 bytes (CMP06 subset) vs DEFLATE's 4096,
--      allowing matches across much longer distances.
--
-- How a Brotli command works:
-- ---------------------------
--
--   Command {
--     insert_length:  number of raw literal bytes to emit
--     copy_length:    number of bytes to copy from the history buffer
--     copy_distance:  how far back to look (1 = immediately preceding byte)
--     literals:       the actual insert_length bytes
--   }
--
--   Bit stream layout:
--     Each regular command emits (in order):
--       [ICC symbol] [insert_extra bits] [copy_extra bits]
--       [literal_0] [literal_1] ... [literal_N]   (N = insert_length)
--       [dist symbol] [dist_extra bits]
--
--     End of regular commands:
--       [ICC=63 sentinel]
--       [flush_literal_0] [flush_literal_1] ...   (trailing literals after last copy)
--
--   The "flush literals" (bytes that follow the sentinel without an ICC code)
--   allow the decoder to recover trailing bytes that could not be bundled into
--   a copy command. The decoder reads them by context-Huffman-decoding until
--   it reaches the original_length.
--
-- Context Modeling:
-- -----------------
--
--   Context function (last byte p1):
--     bucket 0 — space or punctuation (0x00–0x2F, 0x3A–0x40, 0x5B–0x60, 0x7B–0xFF)
--     bucket 1 — digit ('0'–'9')
--     bucket 2 — uppercase letter ('A'–'Z')
--     bucket 3 — lowercase letter ('a'–'z')
--
--   This captures dominant structure: letters after spaces, digits after digits,
--   and case transitions. Four buckets → four separate Huffman trees.
--
-- Wire Format (CMP06):
-- --------------------
--
--   Header (10 bytes):
--     [4B] original_length   big-endian uint32
--     [1B] icc_entry_count   uint8 (1–64)
--     [1B] dist_entry_count  uint8 (0–32)
--     [1B] ctx0_entry_count  uint8
--     [1B] ctx1_entry_count  uint8
--     [1B] ctx2_entry_count  uint8
--     [1B] ctx3_entry_count  uint8
--   ICC code-length table (icc_entry_count × 2B): symbol uint8, code_length uint8
--   Dist code-length table (dist_entry_count × 2B): symbol uint8, code_length uint8
--   Literal tree 0 table (ctx0_entry_count × 3B): symbol uint16 BE, code_length uint8
--   Literal tree 1 table (ctx1_entry_count × 3B): same
--   Literal tree 2 table (ctx2_entry_count × 3B): same
--   Literal tree 3 table (ctx3_entry_count × 3B): same
--   Bit stream: LSB-first packed bits, zero-padded to byte boundary
--
-- Dependencies:
--   coding_adventures.huffman_tree (DT27) — canonical Huffman tree builder
--   No LZSS dependency — LZ matching is done inline.
--
-- ============================================================================

local HuffmanTree = require("coding_adventures.huffman_tree")

local M = {}
M.VERSION = "0.1.0"

-- ---------------------------------------------------------------------------
-- ICC (Insert-Copy Code) table — 64 entries
-- ---------------------------------------------------------------------------
--
-- Each ICC code bundles an insert-length range and a copy-length range into a
-- single Huffman symbol. The exact insert and copy lengths are refined with
-- extra bits after the symbol.
--
-- Format: {insert_base, insert_extra, copy_base, copy_extra}
--   insert_length = insert_base + extra_bits(insert_extra bits)
--   copy_length   = copy_base  + extra_bits(copy_extra bits)
--
-- Code 63 is the end-of-data sentinel (insert=0, copy=0).
--
-- The table covers insert ranges:
--   codes  0–15: insert=0         (insert_base=0, insert_extra=0)
--   codes 16–23: insert=1         (insert_base=1, insert_extra=0)
--   codes 24–31: insert=2         (insert_base=2, insert_extra=0)
--   codes 32–39: insert=3–4       (insert_base=3, insert_extra=1)
--   codes 40–47: insert=5–8       (insert_base=5, insert_extra=2)
--   codes 48–55: insert=9–16      (insert_base=9, insert_extra=3)
--   codes 56–62: insert=17–32     (insert_base=17, insert_extra=4)
--
-- Within each group, 8 copy slots cover: 4, 5, 6, 8–9, 10–11, 14–17, 18–21, 26–29
-- (or the exact copy with 0, 0, 0, 1, 1, 2, 2, 3 extra bits respectively).

local ICC_TABLE = {
  -- codes 0-15: insert=0
  {0, 0,   4, 0}, {0, 0,   5, 0}, {0, 0,   6, 0}, {0, 0,   8, 1},
  {0, 0,  10, 1}, {0, 0,  14, 2}, {0, 0,  18, 2}, {0, 0,  26, 3},
  {0, 0,  34, 3}, {0, 0,  50, 4}, {0, 0,  66, 4}, {0, 0,  98, 5},
  {0, 0, 130, 5}, {0, 0, 194, 6}, {0, 0, 258, 7}, {0, 0, 514, 8},
  -- codes 16-23: insert=1
  {1, 0,   4, 0}, {1, 0,   5, 0}, {1, 0,   6, 0}, {1, 0,   8, 1},
  {1, 0,  10, 1}, {1, 0,  14, 2}, {1, 0,  18, 2}, {1, 0,  26, 3},
  -- codes 24-31: insert=2
  {2, 0,   4, 0}, {2, 0,   5, 0}, {2, 0,   6, 0}, {2, 0,   8, 1},
  {2, 0,  10, 1}, {2, 0,  14, 2}, {2, 0,  18, 2}, {2, 0,  26, 3},
  -- codes 32-39: insert=3+extra(1 bit) → 3 or 4
  {3, 1,   4, 0}, {3, 1,   5, 0}, {3, 1,   6, 0}, {3, 1,   8, 1},
  {3, 1,  10, 1}, {3, 1,  14, 2}, {3, 1,  18, 2}, {3, 1,  26, 3},
  -- codes 40-47: insert=5+extra(2 bits) → 5–8
  {5, 2,   4, 0}, {5, 2,   5, 0}, {5, 2,   6, 0}, {5, 2,   8, 1},
  {5, 2,  10, 1}, {5, 2,  14, 2}, {5, 2,  18, 2}, {5, 2,  26, 3},
  -- codes 48-55: insert=9+extra(3 bits) → 9–16
  {9, 3,   4, 0}, {9, 3,   5, 0}, {9, 3,   6, 0}, {9, 3,   8, 1},
  {9, 3,  10, 1}, {9, 3,  14, 2}, {9, 3,  18, 2}, {9, 3,  26, 3},
  -- codes 56-62: insert=17+extra(4 bits) → 17–32
  {17, 4,  4, 0}, {17, 4,  5, 0}, {17, 4,  6, 0}, {17, 4,  8, 1},
  {17, 4, 10, 1}, {17, 4, 14, 2}, {17, 4, 18, 2},
  -- code 63: end-of-data sentinel
  {0, 0,   0, 0},
}
-- ICC_TABLE[i+1] gives the entry for ICC code i (Lua is 1-indexed).

-- Maximum insert_length encodable in the ICC table:
-- groups 0–6 cover inserts 0,1,2,3–4,5–8,9–16,17–32.
-- Group 6 (codes 56–62): max insert = 17 + (2^4 - 1) = 32.
local MAX_INSERT_PER_ICC = 32

-- ---------------------------------------------------------------------------
-- Distance code table — 32 entries (codes 0–31)
-- ---------------------------------------------------------------------------
--
-- Each distance code covers a range of back-reference offsets. Like ICC codes,
-- the exact offset is encoded as: base + extra_bits(extra_bits_count bits).
-- This extends CMP05's 24 distance codes (up to 4096) to 32 codes (up to 65535).

local DIST_TABLE = {
  -- {code, base, extra_bits}  (0-indexed codes, so index = code+1)
  { 0,     1,  0}, { 1,     2,  0}, { 2,     3,  0}, { 3,     4,  0},
  { 4,     5,  1}, { 5,     7,  1}, { 6,     9,  2}, { 7,    13,  2},
  { 8,    17,  3}, { 9,    25,  3}, {10,    33,  4}, {11,    49,  4},
  {12,    65,  5}, {13,    97,  5}, {14,   129,  6}, {15,   193,  6},
  {16,   257,  7}, {17,   385,  7}, {18,   513,  8}, {19,   769,  8},
  {20,  1025,  9}, {21,  1537,  9}, {22,  2049, 10}, {23,  3073, 10},
  {24,  4097, 11}, {25,  6145, 11}, {26,  8193, 12}, {27, 12289, 12},
  {28, 16385, 13}, {29, 24577, 13}, {30, 32769, 14}, {31, 49153, 14},
}

-- Build lookup arrays: DIST_BASE[code] and DIST_EXTRA[code] (0-indexed codes).
local DIST_BASE = {}
local DIST_EXTRA = {}
for _, e in ipairs(DIST_TABLE) do
  DIST_BASE[e[1]] = e[2]
  DIST_EXTRA[e[1]] = e[3]
end

-- ---------------------------------------------------------------------------
-- Helper: literal_context(p1)
-- ---------------------------------------------------------------------------
--
-- Classify the preceding byte (p1, an integer 0–255, or -1 for start-of-stream)
-- into one of 4 context buckets:
--   0 — space/punctuation (or no preceding byte)
--   1 — digit '0'–'9' (48–57)
--   2 — uppercase 'A'–'Z' (65–90)
--   3 — lowercase 'a'–'z' (97–122)
--
-- These buckets capture the dominant statistical structure of English text:
--   - After a space, the next byte is likely a letter (starting a word).
--   - After a digit, the next byte is likely another digit or punctuation.
--   - After a lowercase letter, the next byte is almost certainly another letter.
--   - After uppercase, case transitions are more common.

local function literal_context(p1)
  -- p1 is an integer (last emitted byte value), or -1 at start of stream.
  if p1 < 0 then return 0 end
  -- Lowercase: 'a' = 97, 'z' = 122
  if p1 >= 97 and p1 <= 122 then return 3 end
  -- Uppercase: 'A' = 65, 'Z' = 90
  if p1 >= 65 and p1 <= 90 then return 2 end
  -- Digit: '0' = 48, '9' = 57
  if p1 >= 48 and p1 <= 57 then return 1 end
  -- Everything else: space and punctuation
  return 0
end

-- ---------------------------------------------------------------------------
-- Helper: find_dist_code(offset)
-- ---------------------------------------------------------------------------
--
-- Given a copy distance (back-reference offset), find the distance code whose
-- range contains that offset. Returns the distance code (0–31).

local function find_dist_code(offset)
  for _, e in ipairs(DIST_TABLE) do
    local max_dist = e[2] + (1 << e[3]) - 1
    if offset <= max_dist then return e[1] end
  end
  return 31  -- max code for offset up to 65535
end

-- ---------------------------------------------------------------------------
-- Helper: find_icc_code(insert_len, copy_len)
-- ---------------------------------------------------------------------------
--
-- Given insert_length and copy_length, find the ICC code that covers both.
-- Scans all 63 non-sentinel codes (0–62) for the first match.
--
-- "Covers" means:
--   insert_base <= insert_len <= insert_base + 2^insert_extra - 1
--   copy_base   <= copy_len  <= copy_base   + 2^copy_extra   - 1
--
-- Returns the ICC code (0–62). Assumes the caller has already ensured that
-- insert_len <= MAX_INSERT_PER_ICC.

local function find_icc_code(insert_len, copy_len)
  for i = 0, 62 do
    local e = ICC_TABLE[i + 1]
    local ib, ie, cb, ce = e[1], e[2], e[3], e[4]
    local max_insert = ib + (1 << ie) - 1
    local max_copy   = cb + (1 << ce) - 1
    if insert_len >= ib and insert_len <= max_insert
       and copy_len >= cb and copy_len <= max_copy then
      return i
    end
  end
  -- Fallback: use insert=0 group, find copy range.
  for i = 0, 15 do
    local e = ICC_TABLE[i + 1]
    local cb, ce = e[3], e[4]
    local max_copy = cb + (1 << ce) - 1
    if copy_len >= cb and copy_len <= max_copy then
      return i
    end
  end
  return 14  -- code 14: insert=0, copy=258+extra(7 bits); safe fallback
end

-- ---------------------------------------------------------------------------
-- Helper: find_best_icc_copy(insert_len, copy_len)
-- ---------------------------------------------------------------------------
--
-- The ICC table has gaps in copy-length coverage. For example, copy_len=7
-- is not directly representable (codes cover 4,5,6,8-9,...). Find the largest
-- copy length <= copy_len that IS representable given insert_len.
--
-- Returns (icc_code, actual_copy_len).

local function find_best_icc_copy(insert_len, copy_len)
  -- Try the exact length first.
  for i = 0, 62 do
    local e = ICC_TABLE[i + 1]
    local ib, ie, cb, ce = e[1], e[2], e[3], e[4]
    local max_insert = ib + (1 << ie) - 1
    local max_copy   = cb + (1 << ce) - 1
    if insert_len >= ib and insert_len <= max_insert
       and copy_len >= cb and copy_len <= max_copy then
      return i, copy_len  -- exact match
    end
  end
  -- Reduce copy_len until we find a slot.
  local best_copy = 4  -- minimum match length
  local best_icc  = nil
  for i = 0, 62 do
    local e = ICC_TABLE[i + 1]
    local ib, ie, cb, ce = e[1], e[2], e[3], e[4]
    local max_insert = ib + (1 << ie) - 1
    local max_copy   = cb + (1 << ce) - 1
    if insert_len >= ib and insert_len <= max_insert then
      -- This insert group fits; find the best copy range <= copy_len.
      local usable_copy = math.min(copy_len, max_copy)
      if usable_copy >= cb and usable_copy > best_copy then
        best_copy = usable_copy
        best_icc  = i
      end
    end
  end
  if best_icc then return best_icc, best_copy end
  -- Last resort: use code 0 (insert=0, copy=4) — discard the match.
  return 0, 4
end

-- ---------------------------------------------------------------------------
-- Bit I/O helpers
-- ---------------------------------------------------------------------------

-- Pack a list of 0/1 integers into bytes, LSB-first.
-- The first bit in the list occupies bit 0 (LSB) of the first output byte.
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
  -- Flush remaining bits (zero-padded to byte boundary).
  if bit_pos > 0 then
    out[#out + 1] = string.char(byte_val)
  end
  return table.concat(out)
end

-- Expand bytes into a flat list of 0/1 integers, LSB-first.
-- Byte 1's bit 0 is list[1], byte 1's bit 7 is list[8], byte 2's bit 0 is list[9]...
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

-- Read n raw bits from bits (list of 0/1), LSB-first, starting at pos.
-- Returns value and updated pos (both 1-based).
local function read_bits(bits, pos, n)
  local val = 0
  for i = 0, n - 1 do
    val = val | ((bits[pos + i] or 0) << i)
  end
  return val, pos + n
end

-- Decode one Huffman symbol by reading bits one at a time until a prefix match.
-- rev_map: bit_string → symbol integer
local function next_huffman_symbol(bits, pos, rev_map)
  local acc = ""
  while true do
    acc = acc .. tostring(bits[pos] or 0)
    pos = pos + 1
    local sym = rev_map[acc]
    if sym ~= nil then return sym, pos end
  end
end

-- Reconstruct canonical codes (bit_string → symbol) from sorted (symbol, code_length)
-- pairs. This mirrors how the encoder stores the trees in the wire format.
--
-- Canonical code assignment rule:
--   1. Sort entries by (code_length ASC, symbol ASC).
--   2. Assign codes in numerical order, left-shifting when code_length increases.
--
-- For a single-symbol tree, the code is always "0" (code length = 1).
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
    -- Format as zero-padded binary string of length code_len.
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

-- Pack a big-endian 4-byte integer.
local function pack_uint32_be(n)
  return string.char(
    (n >> 24) & 0xFF,
    (n >> 16) & 0xFF,
    (n >>  8) & 0xFF,
     n        & 0xFF
  )
end

-- Pack a big-endian 2-byte integer.
local function pack_uint16_be(n)
  return string.char((n >> 8) & 0xFF, n & 0xFF)
end

-- Unpack a big-endian 4-byte integer from a string at position pos (1-based).
local function unpack_uint32_be(data, pos)
  local a, b, c, d = data:byte(pos, pos + 3)
  return (a << 24) | (b << 16) | (c << 8) | d
end

-- ---------------------------------------------------------------------------
-- Pass 1: LZ matching → commands + flush literals
-- ---------------------------------------------------------------------------
--
-- Scan input from left to right. At each position, look back up to 65535 bytes
-- to find the longest match of length >= 4.
--
-- Trailing literal bytes that could not be bundled into a copy command are
-- returned separately as "flush_literals". They are emitted AFTER the sentinel
-- (ICC=63) in the bit stream, so the decoder can recover them by reading
-- context-Huffman-coded bytes until original_length is reached.
--
-- The algorithm is O(n * window_size) — acceptable for a reference implementation.
--
-- Returns:
--   cmds: list of {insert_length, copy_length, copy_distance, literals}
--         The last entry is the sentinel: {0, 0, 0, {}, sentinel=true}.
--   flush_literals: table of byte integers (may be empty).

local function lz_match(data, data_len)
  local cmds = {}
  local insert_buf = {}
  local pos = 1  -- 1-indexed

  while pos <= data_len do
    local window_start = math.max(1, pos - 65535)
    local best_len = 0
    local best_off = 0

    -- Search backwards for the longest match.
    local i = pos - 1
    while i >= window_start do
      if data:byte(i) == data:byte(pos) then
        local match_len = 0
        local max_match = math.min(258, data_len - pos + 1)
        while match_len < max_match
              and data:byte(i + match_len) == data:byte(pos + match_len) do
          match_len = match_len + 1
        end
        if match_len > best_len then
          best_len = match_len
          best_off = pos - i
        end
      end
      i = i - 1
    end

    -- Only accept a match when the insert buffer is still encodable in a single
    -- ICC code (<=32 bytes) AND the match length >= 4.
    if best_len >= 4 and #insert_buf <= MAX_INSERT_PER_ICC then
      -- Find the best (ICC-representable) copy length <= best_len.
      local icc, actual_copy = find_best_icc_copy(#insert_buf, best_len)
      local lits = {}
      for _, b in ipairs(insert_buf) do
        lits[#lits + 1] = b
      end
      cmds[#cmds + 1] = {
        insert_length = #insert_buf,
        copy_length   = actual_copy,
        copy_distance = best_off,
        literals      = lits,
      }
      insert_buf = {}
      pos = pos + actual_copy
    else
      insert_buf[#insert_buf + 1] = data:byte(pos)
      pos = pos + 1
    end
  end

  -- Sentinel command.
  cmds[#cmds + 1] = {
    insert_length = 0,
    copy_length   = 0,
    copy_distance = 0,
    literals      = {},
    sentinel      = true,
  }

  -- Any remaining insert_buf bytes become flush_literals (emitted after sentinel).
  return cmds, insert_buf
end

-- ---------------------------------------------------------------------------
-- Public API: compress
-- ---------------------------------------------------------------------------

--- Compress a byte array (table of integers 0–255) or string using Brotli (CMP06).
-- @param data  string or table of integers (byte array)
-- @return      table of integers (compressed byte array in CMP06 wire format)
function M.compress(data)
  -- Accept both string and byte-table input.
  local str
  if type(data) == "string" then
    str = data
  else
    local chars = {}
    for _, b in ipairs(data) do
      chars[#chars + 1] = string.char(b)
    end
    str = table.concat(chars)
  end

  local original_length = #str

  -- ── Empty input special case ──────────────────────────────────────────────
  --
  -- Empty input encodes as:
  --   Header: [0x00000000][0x01][0x00][0x00][0x00][0x00][0x00]
  --   ICC table: 1 entry — symbol=63 (sentinel), code_length=1
  --   Bit stream: \x00 (one bit "0" padded to a byte)
  if original_length == 0 then
    local header = pack_uint32_be(0) .. string.char(1, 0, 0, 0, 0, 0)
    local icc_entry = string.char(63, 1)  -- symbol=63, code_length=1
    local bit_stream = string.char(0x00)
    local result_str = header .. icc_entry .. bit_stream
    local result = {}
    for i = 1, #result_str do
      result[#result + 1] = result_str:byte(i)
    end
    return result
  end

  -- ── Pass 1: LZ matching → raw commands + flush literals ───────────────────
  local commands, flush_literals = lz_match(str, original_length)

  -- ── Pass 2a: Tally frequencies ────────────────────────────────────────────
  --
  -- Simulate the output to track context (p1 = last emitted byte) while
  -- counting symbol frequencies for:
  --   lit_freq[ctx+1]: frequency of each literal byte in context ctx (4 tables)
  --   icc_freq:        frequency of each ICC code (0–63)
  --   dist_freq:       frequency of each distance code (0–31)
  --
  -- Context is tracked as an integer p1 (last emitted byte, or -1 at start).

  local lit_freq = {{}, {}, {}, {}}
  local icc_freq = {}
  local dist_freq = {}
  local p1 = -1  -- last emitted byte value; -1 = start of stream

  for _, cmd in ipairs(commands) do
    if cmd.sentinel then
      goto continue_freq
    end

    -- Tally ICC and distance first (they come first in the bit stream per command).
    local icc = find_icc_code(cmd.insert_length, cmd.copy_length)
    icc_freq[icc] = (icc_freq[icc] or 0) + 1
    local dc = find_dist_code(cmd.copy_distance)
    dist_freq[dc] = (dist_freq[dc] or 0) + 1

    -- Tally literal frequencies (routed by context).
    for _, byte_val in ipairs(cmd.literals) do
      local ctx = literal_context(p1)
      lit_freq[ctx + 1][byte_val] = (lit_freq[ctx + 1][byte_val] or 0) + 1
      p1 = byte_val
    end

    -- Simulate the copy to advance p1.
    -- We don't need a full history array here because context only uses p1;
    -- but we DO need to know what byte was last written after the copy.
    -- The copy reproduces bytes from the sliding window; for context tracking
    -- we only need the last byte of the copy, but we track the whole copy to
    -- correctly compute context for subsequent commands.
    -- Approach: rebuild copied bytes on-the-fly using a running output list.
    -- For efficiency with the 65535-byte window, we keep a "tail" buffer.
    -- Actually, we need the history to replay copies.  We'll track p1 inline
    -- by replaying the copies into a growing output array.
    ::continue_freq::
  end

  -- Redo pass 2a properly with a full history for copy simulation.
  -- Reset and use a proper history array this time.
  lit_freq = {{}, {}, {}, {}}
  icc_freq = {}
  dist_freq = {}
  local history = {}  -- full output simulation for window look-ups

  local function ctx_from_history()
    return literal_context(history[#history] or -1)
  end

  -- Helper: context from p1 integer.
  local function ctx_from_p1(pv)
    return literal_context(pv)
  end
  _ = ctx_from_p1  -- suppress unused warning

  p1 = -1
  for _, cmd in ipairs(commands) do
    if cmd.sentinel then
      goto continue_freq2
    end

    -- Tally ICC.
    local icc = find_icc_code(cmd.insert_length, cmd.copy_length)
    icc_freq[icc] = (icc_freq[icc] or 0) + 1

    -- Tally distance.
    local dc = find_dist_code(cmd.copy_distance)
    dist_freq[dc] = (dist_freq[dc] or 0) + 1

    -- Tally literals.
    for _, byte_val in ipairs(cmd.literals) do
      local ctx = literal_context(p1)
      lit_freq[ctx + 1][byte_val] = (lit_freq[ctx + 1][byte_val] or 0) + 1
      history[#history + 1] = byte_val
      p1 = byte_val
    end

    -- Simulate copy.
    local src_start = #history - cmd.copy_distance + 1
    for ci = 0, cmd.copy_length - 1 do
      local b = history[src_start + ci]
      history[#history + 1] = b
      p1 = b
    end

    ::continue_freq2::
  end

  -- Tally flush literal frequencies (emitted AFTER the sentinel).
  for _, byte_val in ipairs(flush_literals) do
    local ctx = literal_context(p1)
    lit_freq[ctx + 1][byte_val] = (lit_freq[ctx + 1][byte_val] or 0) + 1
    p1 = byte_val
  end

  -- Always count the end-of-data sentinel.
  icc_freq[63] = (icc_freq[63] or 0) + 1

  -- ── Pass 2b: Build Huffman trees ──────────────────────────────────────────
  --
  -- Build canonical Huffman trees from frequency tables.
  -- HuffmanTree.build returns a tree; tree:canonical_code_table() returns
  -- a map {symbol → bit_string}.

  local function build_tree(freq_map)
    local weights = {}
    for sym, freq in pairs(freq_map) do
      weights[#weights + 1] = {sym, freq}
    end
    if #weights == 0 then return nil end
    local tree = HuffmanTree.build(weights)
    return tree:canonical_code_table()
  end

  local icc_code_table = build_tree(icc_freq)
  local dist_code_table = {}
  if next(dist_freq) ~= nil then
    dist_code_table = build_tree(dist_freq) or {}
  end
  local lit_code_tables = {}
  for ctx = 1, 4 do
    if next(lit_freq[ctx]) ~= nil then
      lit_code_tables[ctx] = build_tree(lit_freq[ctx])
    else
      lit_code_tables[ctx] = nil
    end
  end

  -- ── Pass 2c: Encode commands to bit stream ────────────────────────────────
  --
  -- Bit stream layout for each regular command:
  --   [ICC symbol] [insert_extra bits] [copy_extra bits]
  --   [literal_0] [literal_1] ... [literal_N]   (N = insert_length)
  --   [dist symbol] [dist_extra bits]
  --
  -- Then the sentinel: [ICC=63]
  -- Then flush literals: [literal_0] ... (context-coded, no ICC wrapping)

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

  history = {}  -- reset for encoding pass
  p1 = -1

  for _, cmd in ipairs(commands) do
    if cmd.sentinel then
      -- Emit sentinel ICC=63.
      append_bits(icc_code_table[63])
      -- Emit flush literals (after sentinel, context-coded).
      for _, byte_val in ipairs(flush_literals) do
        local ctx = literal_context(p1)
        append_bits(lit_code_tables[ctx + 1][byte_val])
        p1 = byte_val
      end
      goto continue_encode
    end

    -- 1. Encode ICC symbol.
    local icc = find_icc_code(cmd.insert_length, cmd.copy_length)
    local e = ICC_TABLE[icc + 1]
    local ib, ie, cb, ce = e[1], e[2], e[3], e[4]
    append_bits(icc_code_table[icc])

    -- 2. Insert extra bits.
    append_raw_bits_lsb(cmd.insert_length - ib, ie)

    -- 3. Copy extra bits.
    append_raw_bits_lsb(cmd.copy_length - cb, ce)

    -- 4. Literal symbols (each routed through per-context tree).
    for _, byte_val in ipairs(cmd.literals) do
      local ctx = literal_context(p1)
      append_bits(lit_code_tables[ctx + 1][byte_val])
      history[#history + 1] = byte_val
      p1 = byte_val
    end

    -- 5. Distance symbol + extra bits.
    local dc = find_dist_code(cmd.copy_distance)
    append_bits(dist_code_table[dc])
    append_raw_bits_lsb(cmd.copy_distance - DIST_BASE[dc], DIST_EXTRA[dc])

    -- 6. Simulate copy for context tracking.
    local src_start = #history - cmd.copy_distance + 1
    for ci = 0, cmd.copy_length - 1 do
      local b = history[src_start + ci]
      history[#history + 1] = b
      p1 = b
    end

    ::continue_encode::
  end

  local bit_stream = pack_bits_lsb_first(bit_list)

  -- ── Assemble wire format ──────────────────────────────────────────────────
  --
  -- Sort code-length tables by (code_length ASC, symbol ASC) — the canonical
  -- form required for reconstruction during decompression.

  local function make_lengths_table(code_table)
    if code_table == nil then return {} end
    local lengths = {}
    for sym, code in pairs(code_table) do
      lengths[#lengths + 1] = {sym, #code}
    end
    table.sort(lengths, function(a, b)
      if a[2] ~= b[2] then return a[2] < b[2] else return a[1] < b[1] end
    end)
    return lengths
  end

  local icc_lengths  = make_lengths_table(icc_code_table)
  local dist_lengths = make_lengths_table(dist_code_table)
  local lit_lengths  = {}
  for ctx = 1, 4 do
    lit_lengths[ctx] = make_lengths_table(lit_code_tables[ctx])
  end

  -- Header (10 bytes).
  local header = pack_uint32_be(original_length)
    .. string.char(
      #icc_lengths,
      #dist_lengths,
      #lit_lengths[1],
      #lit_lengths[2],
      #lit_lengths[3],
      #lit_lengths[4]
    )

  -- ICC code-length table: 2 bytes per entry (symbol uint8, code_length uint8).
  local icc_bytes = {}
  for _, pair in ipairs(icc_lengths) do
    icc_bytes[#icc_bytes + 1] = string.char(pair[1], pair[2])
  end

  -- Dist code-length table: 2 bytes per entry.
  local dist_bytes = {}
  for _, pair in ipairs(dist_lengths) do
    dist_bytes[#dist_bytes + 1] = string.char(pair[1], pair[2])
  end

  -- Literal tree code-length tables: 3 bytes per entry (symbol uint16 BE, code_length uint8).
  local lit_bytes = {}
  for ctx = 1, 4 do
    for _, pair in ipairs(lit_lengths[ctx]) do
      lit_bytes[#lit_bytes + 1] = pack_uint16_be(pair[1]) .. string.char(pair[2])
    end
  end

  local result_str = header
    .. table.concat(icc_bytes)
    .. table.concat(dist_bytes)
    .. table.concat(lit_bytes)
    .. bit_stream

  -- Convert to byte array.
  local result = {}
  for i = 1, #result_str do
    result[#result + 1] = result_str:byte(i)
  end
  return result
end

-- ---------------------------------------------------------------------------
-- Public API: decompress
-- ---------------------------------------------------------------------------

--- Decompress a byte array (or string) in CMP06 wire format.
-- @param data  table of integers or string (compressed byte array from compress())
-- @return      table of integers (decompressed byte array)
function M.decompress(data)
  -- Accept both string and byte-table input.
  local str
  if type(data) == "string" then
    str = data
  else
    local chars = {}
    for _, b in ipairs(data) do
      chars[#chars + 1] = string.char(b)
    end
    str = table.concat(chars)
  end

  if #str < 10 then return {} end

  -- ── Parse header ──────────────────────────────────────────────────────────
  local original_length = unpack_uint32_be(str, 1)
  local icc_entry_count  = str:byte(5)
  local dist_entry_count = str:byte(6)
  local ctx_entry_count  = {str:byte(7), str:byte(8), str:byte(9), str:byte(10)}

  if original_length == 0 then return {} end

  local off = 11  -- 1-based, after 10-byte header

  -- ── Parse ICC code-length table ───────────────────────────────────────────
  local icc_lengths = {}
  for _ = 1, icc_entry_count do
    local sym      = str:byte(off)
    local code_len = str:byte(off + 1)
    icc_lengths[#icc_lengths + 1] = {sym, code_len}
    off = off + 2
  end

  -- ── Parse dist code-length table ─────────────────────────────────────────
  local dist_lengths = {}
  for _ = 1, dist_entry_count do
    local sym      = str:byte(off)
    local code_len = str:byte(off + 1)
    dist_lengths[#dist_lengths + 1] = {sym, code_len}
    off = off + 2
  end

  -- ── Parse 4 literal tree code-length tables ───────────────────────────────
  -- Each entry is 3 bytes: symbol uint16 BE, code_length uint8.
  local lit_lengths = {}
  for ctx = 1, 4 do
    lit_lengths[ctx] = {}
    for _ = 1, ctx_entry_count[ctx] do
      local sym_hi   = str:byte(off)
      local sym_lo   = str:byte(off + 1)
      local code_len = str:byte(off + 2)
      local sym = (sym_hi << 8) | sym_lo
      lit_lengths[ctx][#lit_lengths[ctx] + 1] = {sym, code_len}
      off = off + 3
    end
  end

  -- ── Reconstruct Huffman reverse maps (bit_string → symbol) ───────────────
  local icc_rev_map  = reconstruct_canonical_codes(icc_lengths)
  local dist_rev_map = reconstruct_canonical_codes(dist_lengths)
  local lit_rev_maps = {}
  for ctx = 1, 4 do
    lit_rev_maps[ctx] = reconstruct_canonical_codes(lit_lengths[ctx])
  end

  -- ── Unpack bit stream ─────────────────────────────────────────────────────
  local bits = unpack_bits_lsb_first(str:sub(off))
  local bit_pos = 1  -- 1-based index into bits

  -- ── Decode command stream ─────────────────────────────────────────────────
  --
  -- Each iteration:
  --   1. Decode ICC symbol.
  --   2. If ICC == 63 (sentinel): decode flush literals until #output == original_length.
  --   3. Otherwise: read insert_extra, copy_extra, then insert_length literals,
  --      then (if copy_length > 0) distance symbol + distance extra bits + copy.

  local output = {}
  local p1 = -1  -- last emitted byte value; -1 = start of stream

  while true do
    local icc
    icc, bit_pos = next_huffman_symbol(bits, bit_pos, icc_rev_map)

    if icc == 63 then
      -- End-of-data sentinel.
      -- Decode flush literals (trailing bytes after the last copy command)
      -- until we have all original_length bytes.
      while #output < original_length do
        local ctx = literal_context(p1)
        local byte_val
        byte_val, bit_pos = next_huffman_symbol(bits, bit_pos, lit_rev_maps[ctx + 1])
        output[#output + 1] = byte_val
        p1 = byte_val
      end
      break
    end

    local e = ICC_TABLE[icc + 1]
    local ib, ie, cb, ce = e[1], e[2], e[3], e[4]

    -- Read insert extra bits.
    local ins_extra
    ins_extra, bit_pos = read_bits(bits, bit_pos, ie)
    local insert_length = ib + ins_extra

    -- Read copy extra bits.
    local copy_extra
    copy_extra, bit_pos = read_bits(bits, bit_pos, ce)
    local copy_length = cb + copy_extra

    -- Decode and emit insert_length literals.
    for _ = 1, insert_length do
      local ctx = literal_context(p1)
      local byte_val
      byte_val, bit_pos = next_huffman_symbol(bits, bit_pos, lit_rev_maps[ctx + 1])
      output[#output + 1] = byte_val
      p1 = byte_val
    end

    -- Decode and perform copy.
    if copy_length > 0 then
      local dc
      dc, bit_pos = next_huffman_symbol(bits, bit_pos, dist_rev_map)

      local dist_extra
      dist_extra, bit_pos = read_bits(bits, bit_pos, DIST_EXTRA[dc])
      local copy_distance = DIST_BASE[dc] + dist_extra

      -- Copy byte-by-byte (handles overlapping matches like "AAAA...").
      local src_start = #output - copy_distance + 1
      for _ = 1, copy_length do
        local b = output[src_start]
        output[#output + 1] = b
        p1 = b
        src_start = src_start + 1
      end
    end
  end

  -- Trim to original_length (defensive guard against off-by-one in edge cases).
  while #output > original_length do
    output[#output] = nil
  end

  return output
end

-- ---------------------------------------------------------------------------
-- Public API: compress_string / decompress_string
-- ---------------------------------------------------------------------------
--
-- Convenience wrappers that accept and return Lua strings instead of byte
-- arrays. This makes it easier to use the API in string-oriented code without
-- manually converting back and forth.

--- Compress a string using Brotli (CMP06).
-- @param s  string
-- @return   string (compressed bytes in CMP06 wire format)
function M.compress_string(s)
  local byte_arr = M.compress(s)
  local chars = {}
  for _, b in ipairs(byte_arr) do
    chars[#chars + 1] = string.char(b)
  end
  return table.concat(chars)
end

--- Decompress CMP06 wire-format string data.
-- @param s  string (compressed bytes from compress_string())
-- @return   string (original uncompressed data)
function M.decompress_string(s)
  local byte_arr = M.decompress(s)
  local chars = {}
  for _, b in ipairs(byte_arr) do
    chars[#chars + 1] = string.char(b)
  end
  return table.concat(chars)
end

return M
