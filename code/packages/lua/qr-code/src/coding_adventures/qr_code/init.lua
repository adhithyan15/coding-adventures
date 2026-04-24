-- coding-adventures-qr-code
--
-- QR Code encoder — ISO/IEC 18004:2015 compliant.
--
-- QR Code (Quick Response) was invented by Masahiro Hara at Denso Wave in
-- 1994 to track automotive parts. It is now the most widely deployed 2D
-- barcode on earth. This encoder produces valid, scannable QR Codes from
-- any UTF-8 string.
--
-- ## Encoding pipeline
--
--   input string
--     → mode selection    (numeric / alphanumeric / byte)
--     → version selection (smallest version that fits at the chosen ECC level)
--     → bit stream        (mode indicator + char count + data + padding)
--     → blocks + RS ECC   (GF(256) b=0 convention, poly 0x11D)
--     → interleave        (data CWs interleaved, then ECC CWs)
--     → grid init         (finder, separator, timing, alignment, format, dark)
--     → zigzag placement  (two-column snake from bottom-right corner)
--     → mask evaluation   (8 patterns, lowest 4-rule penalty wins)
--     → finalize          (format info + version info v7+)
--     → ModuleGrid        (boolean grid, true = dark)
--
-- ## Relationship to Other Packages
--
--   MA01 gf256       — GF(2^8) field arithmetic used for RS ECC
--   barcode-2d       — provides ModuleGrid type and layout() for rendering
--   paint-instructions — used by barcode-2d for pixel-level output
--
-- ## Quick Start
--
--   local qr = require("coding_adventures.qr_code")
--   local grid = qr.encode("https://example.com", "M")
--   -- grid.rows == grid.cols == 25 (version 2 at ECC M for that URL)
--   -- pass grid to barcode_2d.layout() to get a PaintScene
--
-- Lua 5.4 note: bit operations use the ~ operator (XOR) and << / >> (shift).
-- Tables are 1-indexed throughout this module.

local gf = require("coding_adventures.gf256")

local M = {}

M.VERSION = "0.1.0"

-- ============================================================================
-- Error types
-- ============================================================================
--
-- QR encoding can fail in one way: the input data is too long to fit in any
-- QR Code version (1–40) at the requested ECC level. We signal this with a
-- structured error table.
--
-- Usage:
--   local grid, err = qr.encode(data, "M")
--   if err then error(err.message) end

M.QRCodeError      = "QRCodeError"
M.InputTooLongError = "InputTooLongError"

-- ============================================================================
-- Constants: ECC level indicators and indices
-- ============================================================================
--
-- The 2-bit ECC level indicator embedded in format information is NOT the
-- same as alphabetical order:
--   L = 01,  M = 00,  Q = 11,  H = 10
--
-- This was a deliberate choice by the QR Code designers to reduce the chance
-- of a blank/all-zero format info field being mistaken for valid data.
--
-- ECC_IDX maps L→1, M→2, Q→3, H→4 for 1-indexed table lookups.

local ECC_INDICATOR = { L = 0x01, M = 0x00, Q = 0x03, H = 0x02 }
local ECC_IDX       = { L = 1,    M = 2,    Q = 3,    H = 4    }

-- ============================================================================
-- ISO 18004:2015 — Capacity tables (Table 9)
-- ============================================================================
--
-- ECC_CODEWORDS_PER_BLOCK[eccIdx][version]:
--   number of RS check codewords in each data block.
--   Index 1 is a placeholder (version 0 does not exist); versions run 1–40.
--
-- NUM_BLOCKS[eccIdx][version]:
--   total number of data+ECC blocks for the version and ECC level.
--   Splitting into multiple blocks limits each RS polynomial's degree and
--   allows burst errors in different parts of the symbol to be spread across
--   independent RS codewords.
--
-- Both tables indexed [eccIdx][version] using 1-based Lua indices.
-- eccIdx: 1=L, 2=M, 3=Q, 4=H

-- ECC codewords per block (inner block length used in RS computation):
local ECC_CODEWORDS_PER_BLOCK = {
  -- L:  [1]  1    2    3    4    5    6    7    8    9   10   11   12   13   14   15   16   17   18   19   20   21   22   23   24   25   26   27   28   29   30   31   32   33   34   35   36   37   38   39   40
  {-1,   7,  10,  15,  20,  26,  18,  20,  24,  30,  18,  20,  24,  26,  30,  22,  24,  28,  30,  28,  28,  28,  28,  30,  30,  26,  28,  30,  30,  30,  30,  30,  30,  30,  30,  30,  30,  30,  30,  30,  30},
  -- M:  [2]
  {-1,  10,  16,  26,  18,  24,  16,  18,  22,  22,  26,  30,  22,  22,  24,  24,  28,  28,  26,  26,  26,  26,  28,  28,  28,  28,  28,  28,  28,  28,  28,  28,  28,  28,  28,  28,  28,  28,  28,  28,  28},
  -- Q:  [3]
  {-1,  13,  22,  18,  26,  18,  24,  18,  22,  20,  24,  28,  26,  24,  20,  30,  24,  28,  28,  26,  30,  28,  30,  30,  30,  30,  28,  30,  30,  30,  30,  30,  30,  30,  30,  30,  30,  30,  30,  30,  30},
  -- H:  [4]
  {-1,  17,  28,  22,  16,  22,  28,  26,  26,  24,  28,  24,  28,  22,  24,  24,  30,  28,  28,  26,  28,  30,  24,  30,  30,  30,  30,  30,  30,  30,  30,  30,  30,  30,  30,  30,  30,  30,  30,  30,  30},
}

-- Number of error correction blocks per version and ECC level:
local NUM_BLOCKS = {
  -- L:  [1]  1    2    3    4    5    6    7    8    9   10   11   12   13   14   15   16   17   18   19   20   21   22   23   24   25   26   27   28   29   30   31   32   33   34   35   36   37   38   39   40
  {-1,   1,   1,   1,   1,   1,   2,   2,   2,   2,   4,   4,   4,   4,   4,   6,   6,   6,   6,   7,   8,   8,   9,   9,  10,  12,  12,  12,  13,  14,  15,  16,  17,  18,  19,  19,  20,  21,  22,  24,  25},
  -- M:  [2]
  {-1,   1,   1,   1,   2,   2,   4,   4,   4,   5,   5,   5,   8,   9,   9,  10,  10,  11,  13,  14,  16,  17,  17,  18,  20,  21,  23,  25,  26,  28,  29,  31,  33,  35,  37,  38,  40,  43,  45,  47,  49},
  -- Q:  [3]
  {-1,   1,   1,   2,   2,   4,   4,   6,   6,   8,   8,   8,  10,  12,  16,  12,  17,  16,  18,  21,  20,  23,  23,  25,  27,  29,  34,  34,  35,  38,  40,  43,  45,  48,  51,  53,  56,  59,  62,  65,  68},
  -- H:  [4]
  {-1,   1,   1,   2,   4,   4,   4,   5,   6,   8,   8,  11,  11,  16,  16,  18,  16,  19,  21,  25,  25,  25,  34,  30,  32,  35,  37,  40,  42,  45,  48,  51,  54,  57,  60,  63,  66,  70,  74,  77,  80},
}

-- ============================================================================
-- Alignment pattern centre coordinates, indexed by version (1-indexed).
-- ============================================================================
--
-- The cross-product of these values (excluding positions that overlap finder
-- patterns or timing strips) gives all alignment pattern centres.
-- Source: ISO 18004:2015 Annex E.
--
-- Versions 2+ have alignment patterns. Version 1 has none.
-- The positions here are 0-indexed (matching the TypeScript reference) for
-- consistency with the mathematical formulas; we convert to Lua 1-indexed
-- when accessing the grid.

local ALIGNMENT_POSITIONS = {
  {},                              -- v1  — none
  {6, 18},                         -- v2
  {6, 22},                         -- v3
  {6, 26},                         -- v4
  {6, 30},                         -- v5
  {6, 34},                         -- v6
  {6, 22, 38},                     -- v7
  {6, 24, 42},                     -- v8
  {6, 26, 46},                     -- v9
  {6, 28, 50},                     -- v10
  {6, 30, 54},                     -- v11
  {6, 32, 58},                     -- v12
  {6, 34, 62},                     -- v13
  {6, 26, 46, 66},                 -- v14
  {6, 26, 48, 70},                 -- v15
  {6, 26, 50, 74},                 -- v16
  {6, 30, 54, 78},                 -- v17
  {6, 30, 56, 82},                 -- v18
  {6, 30, 58, 86},                 -- v19
  {6, 34, 62, 90},                 -- v20
  {6, 28, 50, 72, 94},             -- v21
  {6, 26, 50, 74, 98},             -- v22
  {6, 30, 54, 78, 102},            -- v23
  {6, 28, 54, 80, 106},            -- v24
  {6, 32, 58, 84, 110},            -- v25
  {6, 30, 58, 86, 114},            -- v26
  {6, 34, 62, 90, 118},            -- v27
  {6, 26, 50, 74, 98, 122},        -- v28
  {6, 30, 54, 78, 102, 126},       -- v29
  {6, 26, 52, 78, 104, 130},       -- v30
  {6, 30, 56, 82, 108, 134},       -- v31
  {6, 34, 60, 86, 112, 138},       -- v32
  {6, 30, 58, 86, 114, 142},       -- v33
  {6, 34, 62, 90, 118, 146},       -- v34
  {6, 30, 54, 78, 102, 126, 150},  -- v35
  {6, 24, 50, 76, 102, 128, 154},  -- v36
  {6, 28, 54, 80, 106, 132, 158},  -- v37
  {6, 32, 58, 84, 110, 136, 162},  -- v38
  {6, 26, 54, 82, 110, 138, 166},  -- v39
  {6, 30, 58, 86, 114, 142, 170},  -- v40
}

-- ============================================================================
-- Grid geometry helpers
-- ============================================================================
--
-- symbol_size(version): (4 × version + 17) modules on each side.
--   Version 1 → 21×21, Version 40 → 177×177.
--
-- num_raw_data_modules(version): total data+ECC bits available.
--   Subtracts finder, separator, timing, alignment, format, version info areas.
--   Formula from Nayuki's reference implementation (public domain).
--
-- num_data_codewords(version, ecc): message + padding bytes (no ECC).
--   = floor(raw_modules / 8) - sum_of_all_ECC_blocks.
--
-- num_remainder_bits(version): unused trailing bits after codewords (0/3/4/7).

local function symbol_size(version)
  return 4 * version + 17
end

local function num_raw_data_modules(version)
  local result = (16 * version + 128) * version + 64
  if version >= 2 then
    local num_align = math.floor(version / 7) + 2
    result = result - ((25 * num_align - 10) * num_align - 55)
    if version >= 7 then result = result - 36 end
  end
  return result
end

local function num_data_codewords(version, ecc)
  local e = ECC_IDX[ecc]
  return (
    math.floor(num_raw_data_modules(version) / 8) -
    NUM_BLOCKS[e][version + 1] * ECC_CODEWORDS_PER_BLOCK[e][version + 1]
  )
end

local function num_remainder_bits(version)
  return num_raw_data_modules(version) % 8
end

-- ============================================================================
-- Reed-Solomon (b=0 convention)
-- ============================================================================
--
-- QR Code uses b=0 RS: the generator is g(x) = ∏(x + α^i) for i=0..n-1,
-- where α = 2 is the primitive element of GF(256) under poly 0x11D.
--
-- This is DIFFERENT from the b=1 convention (where i starts at 1).
-- The b=0 approach is what QR decoders expect.
--
-- We build generator polynomials on first use and cache them.
-- The generators needed by QR versions 1–40 are a small fixed set:
--   {7, 10, 13, 15, 16, 17, 18, 20, 22, 24, 26, 28, 30}

-- Pre-build the antilog table we need for generator construction.
-- GF(256) ALOG: ALOG[i] = α^i for i=0..254, stored in a Lua 1-indexed table.
-- We build this locally (the gf256 module does not export its tables).
local GF_ALOG = {}
do
  local val = 1
  for i = 0, 254 do
    GF_ALOG[i + 1] = val   -- ALOG[1] = α^0 = 1, ALOG[2] = α^1 = 2, …
    val = val << 1
    if val >= 256 then
      val = val ~ 0x11D
    end
  end
  GF_ALOG[256] = 1  -- α^255 = 1 (cyclic group order 255)
end

-- build_generator(n): compute the monic RS generator of degree n for b=0.
--   Start with g=[1]. Multiply by (x + α^i) for i=0..n-1:
--     new[j] = old[j-1] ⊕ (α^i · old[j])
--   Polynomials stored big-endian: index 1 = highest-degree coefficient.
local GENERATORS = {}

local function build_generator(n)
  if GENERATORS[n] then return GENERATORS[n] end
  -- Start with g(x) = 1 (constant poly, length 1)
  local g = {1}
  for i = 0, n - 1 do
    local ai = GF_ALOG[i + 1]   -- α^i
    local next = {}
    -- Multiply g by (x + α^i):
    --   degree of next = degree of g + 1
    --   next[j] = g[j-1] ⊕ (α^i · g[j])
    for j = 1, #g + 1 do
      next[j] = 0
    end
    for j = 1, #g do
      next[j] = next[j] ~ g[j]
      next[j + 1] = next[j + 1] ~ gf.multiply(g[j], ai)
    end
    g = next
  end
  GENERATORS[n] = g
  return g
end

-- Pre-build all generators used by QR tables (versions 1–40 ECC requirements):
for _, n in ipairs({7, 10, 13, 15, 16, 17, 18, 20, 22, 24, 26, 28, 30}) do
  build_generator(n)
end

-- rs_encode(data, generator): compute RS check bytes via LFSR polynomial division.
--   Returns n remainder bytes (n = #generator - 1).
--
-- Algorithm (shift-register view):
--   For each data byte b:
--     feedback = b XOR rem[1]
--     shift rem left: rem[i] ← rem[i+1]
--     for i = 1..n: rem[i] ^= generator[i+1] · feedback
--
-- This computes D(x)·x^n mod G(x), the RS check polynomial.
-- data is a Lua array (1-indexed); generator is big-endian (index 1 = x^n coeff).
local function rs_encode(data, generator)
  local n = #generator - 1   -- degree of generator = number of check bytes
  local rem = {}
  for i = 1, n do rem[i] = 0 end

  for _, b in ipairs(data) do
    local fb = b ~ rem[1]   -- feedback = data_byte XOR leading remainder byte
    -- Shift the remainder register left by 1 position
    for i = 1, n - 1 do
      rem[i] = rem[i + 1]
    end
    rem[n] = 0
    -- XOR each position with generator coefficient × feedback
    if fb ~= 0 then
      for i = 1, n do
        rem[i] = rem[i] ~ gf.multiply(generator[i + 1], fb)
      end
    end
  end
  return rem
end

-- ============================================================================
-- Data encoding modes
-- ============================================================================
--
-- QR Code supports three encoding modes, selectable per-symbol:
--
-- Numeric     — digits 0-9 only. Packs 3 digits into 10 bits (decimal value 000-999).
-- Alphanumeric — the 45-character QR alphabet (digits, A-Z, 9 punctuation chars).
--                Pairs encode as (idx1 × 45 + idx2) into 11 bits; single into 6.
-- Byte        — raw UTF-8 bytes, 8 bits each. Supports any Unicode string.
--
-- We always pick the most compact mode that can represent the entire input.
-- Mixed-mode (numeric for some runs, byte for others) is more efficient but
-- much more complex; we use single-mode encoding (common for URLs and text).
--
-- 45-character alphanumeric set:
--   0-9  → indices 0-9
--   A-Z  → indices 10-35
--   space→ 36, $→37, %→38, *→39, +→40, -→41, .→42, /→43, :→44

local ALPHANUM_CHARS = "0123456789ABCDEFGHIJKLMNOPQRSTUVWXYZ $%*+-./:"

-- Pre-build an index table for O(1) lookup: alphanum_index[char] = 0-based index
local ALPHANUM_INDEX = {}
for i = 1, #ALPHANUM_CHARS do
  local ch = ALPHANUM_CHARS:sub(i, i)
  ALPHANUM_INDEX[ch] = i - 1   -- 0-based index
end

-- Mode indicator bits (4-bit codes placed at start of bit stream):
--   numeric = 0001, alphanumeric = 0010, byte = 0100
local MODE_INDICATOR = {
  numeric      = 0x1,
  alphanumeric = 0x2,
  byte         = 0x4,
}

-- select_mode(input): choose the best mode for the given string.
-- If every character is a digit → numeric.
-- If every character is in the 45-char QR set → alphanumeric.
-- Otherwise → byte (raw UTF-8).
local function select_mode(input)
  local all_numeric = true
  local all_alphanum = true
  for i = 1, #input do
    local c = input:sub(i, i)
    if c < '0' or c > '9' then all_numeric = false end
    if ALPHANUM_INDEX[c] == nil then all_alphanum = false end
    if not all_numeric and not all_alphanum then break end
  end
  if all_numeric then return "numeric" end
  if all_alphanum then return "alphanumeric" end
  return "byte"
end

-- char_count_bits(mode, version): width of the character-count field.
-- The field width increases for higher versions to accommodate larger inputs.
local function char_count_bits(mode, version)
  if mode == "numeric" then
    return version <= 9 and 10 or (version <= 26 and 12 or 14)
  elseif mode == "alphanumeric" then
    return version <= 9 and 9 or (version <= 26 and 11 or 13)
  else  -- byte
    return version <= 9 and 8 or 16
  end
end

-- ============================================================================
-- BitWriter: accumulate bits and flush to bytes
-- ============================================================================
--
-- Bits are accumulated MSB-first in an array of 0/1 values.
-- to_bytes() groups them into 8-bit bytes, padding the final byte with zeros.
-- This matches the QR Code standard which specifies MSB-first bit ordering.

local BitWriter = {}
BitWriter.__index = BitWriter

local function new_bit_writer()
  return setmetatable({ bits = {}, _len = 0 }, BitWriter)
end

function BitWriter:write(value, count)
  -- Write `count` bits from `value`, MSB first.
  -- Example: write(0b1011, 4) appends 1, 0, 1, 1.
  for i = count - 1, 0, -1 do
    self._len = self._len + 1
    self.bits[self._len] = (value >> i) & 1
  end
end

function BitWriter:bit_length()
  return self._len
end

function BitWriter:to_bytes()
  local bytes = {}
  local i = 1
  while i <= self._len do
    local byte = 0
    for j = 0, 7 do
      -- Combine 8 bits; missing trailing bits are treated as 0.
      byte = (byte << 1) | (self.bits[i + j] or 0)
    end
    bytes[#bytes + 1] = byte
    i = i + 8
  end
  return bytes
end

-- ============================================================================
-- Encoding functions: numeric / alphanumeric / byte
-- ============================================================================

-- encode_numeric: groups of 3 digits → 10 bits; pair → 7 bits; single → 4 bits.
-- Max value for 3 digits: 999 < 1024 = 2^10. Max for 2 digits: 99 < 128 = 2^7.
local function encode_numeric(input, w)
  local i = 1
  while i + 2 <= #input do
    w:write(tonumber(input:sub(i, i + 2)), 10)
    i = i + 3
  end
  if i + 1 <= #input then
    w:write(tonumber(input:sub(i, i + 1)), 7)
    i = i + 2
  end
  if i <= #input then
    w:write(tonumber(input:sub(i, i)), 4)
  end
end

-- encode_alphanumeric: pairs encode as (idx1 × 45 + idx2) → 11 bits; single → 6 bits.
-- The product can be at most 44×45+44 = 2024 < 2048 = 2^11.
local function encode_alphanumeric(input, w)
  local i = 1
  while i + 1 <= #input do
    local c1 = input:sub(i, i)
    local c2 = input:sub(i + 1, i + 1)
    local idx0 = ALPHANUM_INDEX[c1]
    local idx1 = ALPHANUM_INDEX[c2]
    if idx0 == nil or idx1 == nil then
      error("QRCodeError: character not in QR alphanumeric set (precondition violated)")
    end
    w:write(idx0 * 45 + idx1, 11)
    i = i + 2
  end
  if i <= #input then
    local c = input:sub(i, i)
    local idx = ALPHANUM_INDEX[c]
    if idx == nil then
      error("QRCodeError: character not in QR alphanumeric set (precondition violated)")
    end
    w:write(idx, 6)
  end
end

-- encode_byte: each byte of the UTF-8 encoding → 8 bits.
-- Lua strings are byte sequences, so string.byte() gives UTF-8 bytes directly.
local function encode_byte(input, w)
  for i = 1, #input do
    w:write(string.byte(input, i), 8)
  end
end

-- ============================================================================
-- build_data_codewords: assemble the full data codeword sequence
-- ============================================================================
--
-- Format:
--   [mode indicator 4b] [char count Nb] [data bits] [terminator ≤4b]
--   [byte-boundary padding] [0xEC/0x11 filler bytes]
--
-- Output is exactly num_data_codewords(version, ecc) bytes.
-- The 0xEC/0x11 alternation is mandated by ISO 18004 to prevent runs of
-- identical bytes that might confuse decoders.
local function build_data_codewords(input, version, ecc)
  local mode = select_mode(input)
  local capacity = num_data_codewords(version, ecc)
  local w = new_bit_writer()

  -- Mode indicator (4 bits)
  w:write(MODE_INDICATOR[mode], 4)

  -- Character count (width depends on mode and version)
  -- For byte mode: count is the number of UTF-8 bytes = #input in Lua
  -- (Lua strings are already byte sequences, not Unicode code points).
  local char_count = #input   -- works correctly for numeric/alphanumeric/byte
  w:write(char_count, char_count_bits(mode, version))

  -- Actual data bits
  if mode == "numeric" then
    encode_numeric(input, w)
  elseif mode == "alphanumeric" then
    encode_alphanumeric(input, w)
  else
    encode_byte(input, w)
  end

  -- Terminator: up to 4 zero bits (fewer if near capacity)
  local term_len = math.min(4, capacity * 8 - w:bit_length())
  if term_len > 0 then w:write(0, term_len) end

  -- Pad to byte boundary
  local rem = w:bit_length() % 8
  if rem ~= 0 then w:write(0, 8 - rem) end

  -- Fill remaining capacity with alternating 0xEC / 0x11
  local bytes = w:to_bytes()
  local pad = 0xEC
  while #bytes < capacity do
    bytes[#bytes + 1] = pad
    pad = (pad == 0xEC) and 0x11 or 0xEC
  end

  return bytes
end

-- ============================================================================
-- Block processing
-- ============================================================================
--
-- QR data codewords are split into groups of blocks, each independently
-- RS-encoded. Splitting limits the degree of each RS polynomial and, more
-- importantly, allows interleaving: when bytes are round-robined across blocks
-- before being written to the grid, a burst of consecutive corrupted bytes
-- affects at most one or two bytes per block, well within each block's
-- error-correction budget.
--
-- Block structure:
--   totalBlocks = NUM_BLOCKS[eccIdx][version]
--   Each block has shortLen or shortLen+1 data bytes, where:
--     shortLen = floor(totalData / totalBlocks)
--     numLong  = totalData mod totalBlocks  (these blocks get the +1 byte)
--   First (totalBlocks - numLong) blocks are "short"; last numLong are "long".
--
-- compute_blocks returns a list of {data=..., ecc=...} tables.

local function compute_blocks(data, version, ecc)
  local e = ECC_IDX[ecc]
  local total_blocks = NUM_BLOCKS[e][version + 1]
  local ecc_len = ECC_CODEWORDS_PER_BLOCK[e][version + 1]
  local total_data = num_data_codewords(version, ecc)
  local short_len = math.floor(total_data / total_blocks)
  local num_long = total_data % total_blocks
  local gen = build_generator(ecc_len)
  local blocks = {}
  local offset = 1  -- 1-indexed offset into data array

  -- Group 1: (total_blocks - num_long) short blocks of shortLen bytes
  local g1_count = total_blocks - num_long
  for _ = 1, g1_count do
    local d = {}
    for j = 0, short_len - 1 do
      d[j + 1] = data[offset + j]
    end
    blocks[#blocks + 1] = { data = d, ecc = rs_encode(d, gen) }
    offset = offset + short_len
  end

  -- Group 2: num_long long blocks of (shortLen+1) bytes
  for _ = 1, num_long do
    local d = {}
    for j = 0, short_len do
      d[j + 1] = data[offset + j]
    end
    blocks[#blocks + 1] = { data = d, ecc = rs_encode(d, gen) }
    offset = offset + short_len + 1
  end

  return blocks
end

-- interleave_blocks: round-robin data codewords then ECC codewords.
-- First pass: for position i=1.., take blocks[k].data[i] for all k that have it.
-- Second pass: for position i=1.., take blocks[k].ecc[i] for all k that have it.
-- This spreads burst errors across all blocks.
local function interleave_blocks(blocks)
  local result = {}

  -- Find the maximum block lengths
  local max_data = 0
  local max_ecc  = 0
  for _, b in ipairs(blocks) do
    if #b.data > max_data then max_data = #b.data end
    if #b.ecc  > max_ecc  then max_ecc  = #b.ecc  end
  end

  -- Interleave data codewords
  for i = 1, max_data do
    for _, b in ipairs(blocks) do
      if i <= #b.data then result[#result + 1] = b.data[i] end
    end
  end

  -- Interleave ECC codewords
  for i = 1, max_ecc do
    for _, b in ipairs(blocks) do
      if i <= #b.ecc then result[#result + 1] = b.ecc[i] end
    end
  end

  return result
end

-- ============================================================================
-- Grid construction
-- ============================================================================
--
-- The WorkGrid holds two parallel 2D arrays (both size×size):
--   modules[r][c]  — boolean: true = dark, false = light
--   reserved[r][c] — boolean: true = structural (finder/format/etc.), skip during data/mask
--
-- All arrays are 1-indexed (r=1 is the top row, c=1 is the left column).
-- Callers must use 0-indexed coordinates when reading from ALIGNMENT_POSITIONS
-- and convert to 1-indexed before accessing modules/reserved.

local function make_work_grid(size)
  local modules  = {}
  local reserved = {}
  for r = 1, size do
    modules[r]  = {}
    reserved[r] = {}
    for c = 1, size do
      modules[r][c]  = false
      reserved[r][c] = false
    end
  end
  return { size = size, modules = modules, reserved = reserved }
end

-- set_mod: write a module and optionally mark it as reserved.
-- row and col are 1-indexed.
local function set_mod(g, row, col, dark, reserve)
  g.modules[row][col] = dark
  if reserve then g.reserved[row][col] = true end
end

-- ============================================================================
-- Finder patterns (three corners)
-- ============================================================================
--
-- The 7×7 finder pattern placed at three corners of the QR Code:
--
--   ■ ■ ■ ■ ■ ■ ■
--   ■ □ □ □ □ □ ■
--   ■ □ ■ ■ ■ □ ■
--   ■ □ ■ ■ ■ □ ■
--   ■ □ ■ ■ ■ □ ■
--   ■ □ □ □ □ □ ■
--   ■ ■ ■ ■ ■ ■ ■
--
-- The 1:1:3:1:1 dark:light ratio in every scan direction lets any decoder
-- locate and orient the symbol even under partial occlusion or rotation.
-- Module is dark if it is on the outer border (ring 0) OR in the inner 3×3 core.
-- Parameters: top_row and top_col are 1-indexed.
local function place_finder(g, top_row, top_col)
  for dr = 0, 6 do
    for dc = 0, 6 do
      local on_border = (dr == 0 or dr == 6 or dc == 0 or dc == 6)
      local in_core   = (dr >= 2 and dr <= 4 and dc >= 2 and dc <= 4)
      set_mod(g, top_row + dr, top_col + dc, on_border or in_core, true)
    end
  end
end

-- ============================================================================
-- Alignment patterns (version 2+)
-- ============================================================================
--
-- The 5×5 alignment pattern:
--
--   ■ ■ ■ ■ ■
--   ■ □ □ □ ■
--   ■ □ ■ □ ■
--   ■ □ □ □ ■
--   ■ ■ ■ ■ ■
--
-- The centre is always dark. Decoders use these to correct for lens distortion
-- and perspective. Parameters: row and col are 0-indexed centres.
local function place_alignment(g, row0, col0)
  -- Convert 0-indexed centre to 1-indexed grid access
  local row = row0 + 1
  local col = col0 + 1
  for dr = -2, 2 do
    for dc = -2, 2 do
      local on_border = (math.abs(dr) == 2 or math.abs(dc) == 2)
      local is_center = (dr == 0 and dc == 0)
      set_mod(g, row + dr, col + dc, on_border or is_center, true)
    end
  end
end

-- place_all_alignments: place all alignment patterns for the version.
-- Uses the cross-product of ALIGNMENT_POSITIONS[version]. Skips any centre
-- whose 1-indexed grid position is already reserved (overlaps finder/timing).
local function place_all_alignments(g, version)
  local positions = ALIGNMENT_POSITIONS[version]
  for _, row0 in ipairs(positions) do
    for _, col0 in ipairs(positions) do
      -- Convert 0-indexed to 1-indexed for the reserved check
      if not g.reserved[row0 + 1][col0 + 1] then
        place_alignment(g, row0, col0)
      end
    end
  end
end

-- ============================================================================
-- Timing strips
-- ============================================================================
--
-- Alternating dark/light strips along row 6 and column 6 (0-indexed: row/col 7
-- in 1-indexed Lua). Dark when the position index is even.
-- Extend between the finder patterns:
--   Row 6, cols 8..size-9  (0-indexed: 1-indexed cols 9..size-8)
--   Col 6, rows 8..size-9  (0-indexed: 1-indexed rows 9..size-8)
local function place_timing_strips(g)
  local sz = g.size
  -- Row 6 (0-indexed) = row 7 (1-indexed), col from 8 to sz-9 (0-indexed) = 9..sz-8 (1-indexed)
  for c0 = 8, sz - 9 do
    set_mod(g, 7, c0 + 1, c0 % 2 == 0, true)
  end
  -- Col 6 (0-indexed) = col 7 (1-indexed), row from 8 to sz-9 (0-indexed)
  for r0 = 8, sz - 9 do
    set_mod(g, r0 + 1, 7, r0 % 2 == 0, true)
  end
end

-- ============================================================================
-- Format information reservation
-- ============================================================================
--
-- 15 format information bits are stored at two locations in the symbol.
-- We reserve (mark as structural) these positions before data placement;
-- actual values are written after mask selection.
--
-- Copy 1: adjacent to top-left finder
--   Row 6 (0-idx), cols 0..8 (skipping col 6 = timing)
--   Col 6 (0-idx), rows 0..8 (skipping row 6 = timing)
--   (Both in 1-indexed: row/col 7)
--
-- Copy 2:
--   Col 6 (0-idx), rows sz-7..sz-1 (0-indexed)
--   Row 6 (0-idx), cols sz-8..sz-1 (0-indexed)
local function reserve_format_info(g)
  local sz = g.size
  -- Copy 1: row 6 (0-idx = row 7 in 1-idx), cols 0..8 except col 6
  for c0 = 0, 8 do
    if c0 ~= 6 then g.reserved[7][c0 + 1] = true end
  end
  -- Copy 1: col 6 (0-idx = col 7 in 1-idx), rows 0..8 except row 6
  for r0 = 0, 8 do
    if r0 ~= 6 then g.reserved[r0 + 1][7] = true end
  end
  -- Copy 2: col 6 (1-idx=7), rows sz-8..sz-1 (0-indexed) → sz-7..sz (1-indexed)
  for r0 = sz - 7, sz - 1 do
    g.reserved[r0 + 1][7] = true
  end
  -- Copy 2: row 6 (1-idx=7), cols sz-8..sz-1 (0-indexed) → sz-7..sz (1-indexed)
  for c0 = sz - 8, sz - 1 do
    g.reserved[7][c0 + 1] = true
  end
end

-- ============================================================================
-- Format information computation and writing
-- ============================================================================
--
-- The 15-bit format information encodes: [ECC level (2b)] [mask pattern (3b)]
-- Protected by a BCH(15,5) code with generator polynomial G(x) = 0x537
-- (x^10 + x^8 + x^5 + x^4 + x^2 + x + 1), then XOR'd with 0x5412 to prevent
-- an all-zero format info field.
--
-- Bit layout from ISO 18004:2015 (critical lesson from lessons.md):
--
-- Copy 1, row 8 (0-indexed; 1-indexed row = 9):
--   Bits f14,f13,f12,f11,f10,f9 → cols 0,1,2,3,4,5  (MSB first)
--   Bit  f8                     → col 7  (skip col 6 = timing)
--   Bit  f7                     → col 8  (corner)
-- Copy 1, col 8 (0-indexed; 1-indexed col = 9):
--   Bit  f6                     → row 7  (skip row 6 = timing)
--   Bits f5,f4,f3,f2,f1,f0     → rows 5,4,3,2,1,0  (LSB at row 0)
--
-- Copy 2, col 8 (1-indexed = 9), rows sz-7..sz-1 (0-indexed):
--   Bit  f0 → row sz-1, f1 → sz-2, ..., f6 → sz-7 (LSB to MSB going up)
-- Copy 2, row 8 (1-indexed = 9), cols sz-8..sz-1 (0-indexed):
--   Bit  f7 → col sz-8, ..., f14 → col sz-1 (bit 7 to 14 going right)
--
-- This is documented in lessons.md because the ordering is easy to get wrong.

local function compute_format_bits(ecc, mask)
  -- 5-bit data word: [ecc_indicator (2b)] [mask (3b)]
  local data = (ECC_INDICATOR[ecc] << 3) | mask
  -- BCH remainder: compute (data × x^10) mod 0x537
  local rem = data << 10
  for i = 14, 10, -1 do
    if (rem >> i) & 1 == 1 then
      rem = rem ~ (0x537 << (i - 10))
    end
  end
  -- XOR with mask 0x5412 to prevent all-zero
  return ((data << 10) | (rem & 0x3FF)) ~ 0x5412
end

local function write_format_info(g, fmt_bits)
  local sz = g.size

  -- Copy 1, row 8 (0-indexed) → 1-indexed row 9:
  -- Bits f14 down to f9 go left to right across cols 0..5 (0-indexed → 1-indexed 1..6)
  for i = 0, 5 do
    g.modules[9][i + 1] = ((fmt_bits >> (14 - i)) & 1) == 1
  end
  -- Col 6 is the timing strip — skip; col 7 (0-indexed) → 1-indexed col 8
  g.modules[9][8] = ((fmt_bits >> 8) & 1) == 1    -- bit f8
  -- Col 8 (0-indexed) → 1-indexed col 9
  g.modules[9][9] = ((fmt_bits >> 7) & 1) == 1    -- bit f7, corner module

  -- Copy 1, col 8 (0-indexed) → 1-indexed col 9:
  -- Row 7 (0-indexed) → 1-indexed row 8 (skip row 6 = timing)
  g.modules[8][9] = ((fmt_bits >> 6) & 1) == 1    -- bit f6
  -- Rows 5 down to 0 (0-indexed) → 1-indexed 6 down to 1; bit order: f5 at row 5 → f0 at row 0
  for i = 0, 5 do
    g.modules[6 - i][9] = ((fmt_bits >> (5 - i)) & 1) == 1
  end

  -- Copy 2, col 8 (0-indexed) → 1-indexed col 9:
  -- Rows sz-7..sz-1 (0-indexed) → 1-indexed sz-6..sz
  -- f0 at bottom (row sz-1), f6 at top of this strip (row sz-7)
  for i = 0, 6 do
    g.modules[sz - i][9] = ((fmt_bits >> i) & 1) == 1
  end

  -- Copy 2, row 8 (0-indexed) → 1-indexed row 9:
  -- Cols sz-8..sz-1 (0-indexed) → 1-indexed sz-7..sz
  -- f7 at leftmost (col sz-8), f14 at rightmost (col sz-1)
  for i = 7, 14 do
    g.modules[9][sz - 14 + i] = ((fmt_bits >> i) & 1) == 1
  end
end

-- ============================================================================
-- Version information (v7+)
-- ============================================================================
--
-- Versions 7 and higher encode the version number in an 18-bit BCH-coded word
-- stored in two 6×3 blocks:
--   Near top-right: rows 0..5, cols size-11..size-9 (0-indexed)
--   Near bottom-left: rows size-11..size-9, cols 0..5 (0-indexed) [transposed]
--
-- BCH generator: G(x) = x^12+x^11+x^10+x^9+x^8+x^5+x^2+1 = 0x1F25
-- 18-bit word: [version (6b)] [BCH remainder (12b)]
-- Bit placement: bit i → position (5-floor(i/3), size-9-(i%3))
--   and its transpose (size-9-(i%3), 5-floor(i/3))

local function reserve_version_info(g, version)
  if version < 7 then return end
  local sz = g.size
  -- Top-right block: rows 0..5 (0-idx = 1..6 in 1-idx), cols sz-11..sz-9 (0-idx = sz-10..sz-8)
  for r0 = 0, 5 do
    for dc = 0, 2 do
      g.reserved[r0 + 1][sz - 10 + dc] = true
    end
  end
  -- Bottom-left block (transposed): rows sz-11..sz-9, cols 0..5
  for dr = 0, 2 do
    for c0 = 0, 5 do
      g.reserved[sz - 10 + dr][c0 + 1] = true
    end
  end
end

local function compute_version_bits(version)
  local rem = version << 12
  for i = 17, 12, -1 do
    if (rem >> i) & 1 == 1 then
      rem = rem ~ (0x1F25 << (i - 12))
    end
  end
  return (version << 12) | (rem & 0xFFF)
end

local function write_version_info(g, version)
  if version < 7 then return end
  local sz = g.size
  local bits = compute_version_bits(version)
  for i = 0, 17 do
    local dark = ((bits >> i) & 1) == 1
    local a = 5 - math.floor(i / 3)       -- row index (0-indexed)
    local b = sz - 1 - 8 - (i % 3)        -- col index (0-indexed): sz-9-(i%3)
    g.modules[a + 1][b + 1] = dark        -- top-right block
    g.modules[b + 1][a + 1] = dark        -- bottom-left block (transposed)
  end
end

-- ============================================================================
-- Dark module and separators
-- ============================================================================
--
-- The always-dark module at position (4V+9, 8) in 0-indexed coordinates
-- (= row 4V+10, col 9 in 1-indexed).  It is never masked.
local function place_dark_module(g, version)
  local row0 = 4 * version + 9
  set_mod(g, row0 + 1, 9, true, true)
end

-- ============================================================================
-- build_grid: initialise the full structural layout for a version
-- ============================================================================
--
-- Places all structural elements: finders, separators, timing, alignments,
-- format/version placeholders, dark module.
-- Data bits are placed separately in place_bits().
local function build_grid(version)
  local sz = symbol_size(version)
  local g = make_work_grid(sz)

  -- Three finder patterns at three corners (parameters are 0-indexed top-left
  -- corner in module space, converted to 1-indexed for set_mod)
  place_finder(g, 1,       1)           -- top-left,    0-indexed origin (0,0)
  place_finder(g, 1,       sz - 6)      -- top-right,   0-indexed origin (0, sz-7)
  place_finder(g, sz - 6,  1)           -- bottom-left, 0-indexed origin (sz-7, 0)

  -- Separators: 1-module light border just outside each finder.
  -- Top-left finder occupies rows/cols 1..7 (1-indexed). Separator is row/col 8.
  -- We reserve them as structural so data placement skips them.
  for i0 = 0, 7 do
    set_mod(g, 8,      i0 + 1,  false, true)   -- row 7 (0-idx) horizontal
    set_mod(g, i0 + 1, 8,       false, true)   -- col 7 (0-idx) vertical
  end
  -- Top-right finder occupies rows 1..7, cols sz-6..sz (1-indexed).
  -- Separator: row 8 and col sz-7 (0-indexed col sz-8 → 1-indexed sz-7)
  for i0 = 0, 7 do
    set_mod(g, 8,      sz - i0, false, true)
    set_mod(g, i0 + 1, sz - 7,  false, true)
  end
  -- Bottom-left finder occupies rows sz-6..sz, cols 1..7 (1-indexed).
  -- Separator: row sz-7 (0-indexed sz-8 → 1-indexed sz-7) and col 8
  for i0 = 0, 7 do
    set_mod(g, sz - 7,  i0 + 1, false, true)
    set_mod(g, sz - i0, 8,      false, true)
  end

  place_timing_strips(g)
  place_all_alignments(g, version)

  reserve_format_info(g)
  reserve_version_info(g, version)
  place_dark_module(g, version)

  return g
end

-- ============================================================================
-- Bit placement (zigzag scan)
-- ============================================================================
--
-- After all structural modules are placed, the interleaved codeword bits fill
-- the remaining (non-reserved) positions using a two-column right-to-left,
-- upward/downward alternating scan:
--
--   Start at col = size−1 (rightmost), move left 2 columns at a time.
--   Col 6 (0-indexed, the vertical timing strip) is always skipped:
--     when col would be 6, jump to 5.
--   Within each 2-column strip, scan all rows top→bottom or bottom→top
--   (alternating), placing data bits in the right column first, then left.
--   Reserved modules are skipped; data bits fill the rest.
--   After all codeword bits, append num_remainder_bits(version) zero bits.
--
-- Note: column and row indices here are 0-indexed for the scan logic; we
-- convert to 1-indexed when accessing g.modules and g.reserved.
local function place_bits(g, codewords, version)
  local sz = g.size

  -- Flatten codewords to a bit array (MSB first per byte)
  local bits = {}
  for _, cw in ipairs(codewords) do
    for b = 7, 0, -1 do
      bits[#bits + 1] = ((cw >> b) & 1) == 1
    end
  end
  -- Append remainder bits (zero bits, no data)
  for _ = 1, num_remainder_bits(version) do
    bits[#bits + 1] = false
  end

  local bit_idx = 1
  local going_up = true      -- scan direction: true=bottom→top, false=top→bottom
  local col0 = sz - 1        -- 0-indexed leading column of current 2-col strip

  while col0 >= 1 do
    for vi = 0, sz - 1 do
      local row0 = going_up and (sz - 1 - vi) or vi
      -- Two columns: right (col0) then left (col0 - 1)
      for _, dc in ipairs({0, 1}) do
        local c0 = col0 - dc
        if c0 ~= 6 then  -- skip vertical timing strip
          if not g.reserved[row0 + 1][c0 + 1] then
            g.modules[row0 + 1][c0 + 1] = bits[bit_idx] or false
            bit_idx = bit_idx + 1
          end
        end
      end
    end
    going_up = not going_up
    col0 = col0 - 2
    if col0 == 6 then col0 = 5 end  -- hop over timing column
  end
end

-- ============================================================================
-- Masking
-- ============================================================================
--
-- 8 mask patterns from ISO 18004 Table 10. If the condition is true at (row, col)
-- (0-indexed), the data module is flipped (dark↔light). Structural modules
-- (reserved) are never masked.
--
-- The 8 conditions are defined using 0-indexed row and col values.

local MASK_CONDS = {
  function(r, c) return (r + c) % 2 == 0 end,
  function(r, _) return r % 2 == 0 end,
  function(_, c) return c % 3 == 0 end,
  function(r, c) return (r + c) % 3 == 0 end,
  function(r, c) return (math.floor(r / 2) + math.floor(c / 3)) % 2 == 0 end,
  function(r, c) return (r * c) % 2 + (r * c) % 3 == 0 end,
  function(r, c) return ((r * c) % 2 + (r * c) % 3) % 2 == 0 end,
  function(r, c) return ((r + c) % 2 + (r * c) % 3) % 2 == 0 end,
}

-- apply_mask: return a NEW modules array with mask applied to non-reserved cells.
-- Inputs use 1-indexed tables; the mask condition receives 0-indexed row/col.
local function apply_mask(modules, reserved, sz, mask_idx)
  local cond = MASK_CONDS[mask_idx + 1]  -- convert 0-indexed mask to 1-indexed table
  local new_mods = {}
  for r = 1, sz do
    new_mods[r] = {}
    for c = 1, sz do
      if reserved[r][c] then
        new_mods[r][c] = modules[r][c]
      else
        -- XOR (flip) if condition is true; row and col passed as 0-indexed
        new_mods[r][c] = modules[r][c] ~= cond(r - 1, c - 1)
      end
    end
  end
  return new_mods
end

-- ============================================================================
-- Penalty scoring (ISO 18004 Section 7.8.3)
-- ============================================================================
--
-- Four rules contribute to the penalty score. Lower = better.
--
-- Rule 1: Horizontal or vertical runs of ≥ 5 consecutive same-color modules.
--   Penalty per run = run_length − 2.
--   (A run of 5 scores 3, 6 scores 4, etc.)
--
-- Rule 2: 2×2 blocks of same-color modules.
--   Penalty = 3 per block.
--   (Overlapping blocks are each counted.)
--
-- Rule 3: Pattern 10111010000 or its reverse 00001011101 appearing as a run
--   of 11 consecutive modules in any row or column.
--   Penalty = 40 per match.
--   (These look like finder patterns and would confuse decoders.)
--
-- Rule 4: If the dark module proportion deviates from 50%, add a penalty based
--   on the distance (in 5% steps) from 50%:
--   Penalty = min(|floor(ratio/5)×5 − 50|, |ceil(ratio/5)×5 − 50|) / 5 × 10.

local function compute_penalty(modules, sz)
  local penalty = 0

  -- Rule 1: runs of ≥5 same-color in rows and columns
  for r = 1, sz do
    for _, horiz in ipairs({true, false}) do
      local run = 1
      local prev = horiz and modules[r][1] or modules[1][r]
      for i = 2, sz do
        local cur = horiz and modules[r][i] or modules[i][r]
        if cur == prev then
          run = run + 1
        else
          if run >= 5 then penalty = penalty + run - 2 end
          run = 1
          prev = cur
        end
      end
      if run >= 5 then penalty = penalty + run - 2 end
    end
  end

  -- Rule 2: 2×2 same-color blocks
  for r = 1, sz - 1 do
    for c = 1, sz - 1 do
      local d = modules[r][c]
      if d == modules[r][c + 1] and d == modules[r + 1][c] and d == modules[r + 1][c + 1] then
        penalty = penalty + 3
      end
    end
  end

  -- Rule 3: finder-like patterns (horizontal and vertical, both directions)
  -- P1 = 1,0,1,1,1,0,1,0,0,0,0  P2 = 0,0,0,0,1,0,1,1,1,0,1
  local P1 = {1, 0, 1, 1, 1, 0, 1, 0, 0, 0, 0}
  local P2 = {0, 0, 0, 0, 1, 0, 1, 1, 1, 0, 1}
  for a = 1, sz do
    for b = 1, sz - 10 do
      local mH1, mH2, mV1, mV2 = true, true, true, true
      for k = 1, 11 do
        local bH = modules[a][b + k - 1] and 1 or 0
        local bV = modules[b + k - 1][a] and 1 or 0
        if bH ~= P1[k] then mH1 = false end
        if bH ~= P2[k] then mH2 = false end
        if bV ~= P1[k] then mV1 = false end
        if bV ~= P2[k] then mV2 = false end
      end
      if mH1 then penalty = penalty + 40 end
      if mH2 then penalty = penalty + 40 end
      if mV1 then penalty = penalty + 40 end
      if mV2 then penalty = penalty + 40 end
    end
  end

  -- Rule 4: dark module ratio deviation from 50%
  local dark = 0
  for r = 1, sz do
    for c = 1, sz do
      if modules[r][c] then dark = dark + 1 end
    end
  end
  local ratio = (dark / (sz * sz)) * 100
  local prev5 = math.floor(ratio / 5) * 5
  penalty = penalty + math.min(math.abs(prev5 - 50), math.abs(prev5 + 5 - 50)) / 5 * 10

  return penalty
end

-- ============================================================================
-- Version selection
-- ============================================================================
--
-- Find the minimum version (1–40) that can hold the input at the chosen ECC level.
-- Uses exact bit counts for mode indicator, character count field, and data bits.
-- The character count field width changes at version boundaries (9, 26), so we
-- check all 40 versions in order and return the first that fits.

local function select_version(input, ecc)
  local mode    = select_mode(input)
  local byte_len = #input  -- Lua: string length is UTF-8 byte count

  for v = 1, 40 do
    local capacity = num_data_codewords(v, ecc)
    local data_bits
    if mode == "byte" then
      data_bits = byte_len * 8
    elseif mode == "numeric" then
      data_bits = math.ceil(#input * 10 / 3)
    else  -- alphanumeric
      data_bits = math.ceil(#input * 11 / 2)
    end
    local bits_needed = 4 + char_count_bits(mode, v) + data_bits
    if math.ceil(bits_needed / 8) <= capacity then return v end
  end

  return nil, {
    kind    = M.InputTooLongError,
    message = string.format(
      "Input (%d chars, ECC=%s) exceeds version 40 capacity.", #input, ecc)
  }
end

-- ============================================================================
-- Public API
-- ============================================================================

-- encode(data, level): encode a UTF-8 string to a QR Code ModuleGrid.
--
-- Parameters:
--   data  — the string to encode (any UTF-8 content, treated as bytes)
--   level — ECC level: "L" (~7% recovery), "M" (~15%), "Q" (~25%), "H" (~30%).
--           Defaults to "M" if omitted.
--
-- Returns:
--   On success: grid, nil
--     grid.rows    = grid.cols = 4*version+17
--     grid.modules = 2D boolean array (true = dark), 1-indexed
--     grid.module_shape = "square"
--   On failure: nil, err_table
--     err_table.kind    = "InputTooLongError"
--     err_table.message = human-readable description
--
-- The returned grid is compatible with barcode_2d.layout() for rendering.
--
-- Example:
--   local qr = require("coding_adventures.qr_code")
--   local grid, err = qr.encode("https://example.com", "M")
--   if err then error(err.message) end
--   -- grid.rows == 25 (version 2 at ECC M)
function M.encode(data, level)
  level = level or "M"

  -- Validate ECC level
  if not ECC_IDX[level] then
    return nil, {
      kind    = M.QRCodeError,
      message = string.format("Invalid ECC level '%s'; must be L, M, Q, or H.", level)
    }
  end

  -- Guard against absurdly long inputs (QR v40 max is 7089 numeric chars ~2953 bytes)
  if #data > 7089 then
    return nil, {
      kind    = M.InputTooLongError,
      message = string.format(
        "Input length %d exceeds 7089 (QR Code v40 numeric-mode maximum).", #data)
    }
  end

  -- Step 1: select the smallest version that fits
  local version, err = select_version(data, level)
  if err then return nil, err end

  local sz = symbol_size(version)

  -- Step 2: build data codewords
  local data_cw     = build_data_codewords(data, version, level)

  -- Step 3: split into blocks and compute RS ECC for each
  local blocks      = compute_blocks(data_cw, version, level)

  -- Step 4: interleave blocks
  local interleaved = interleave_blocks(blocks)

  -- Step 5: initialise the grid with all structural elements
  local grid = build_grid(version)

  -- Step 6: place the interleaved codeword bits via zigzag scan
  place_bits(grid, interleaved, version)

  -- Step 7: evaluate all 8 masks; pick the one with the lowest penalty score
  local best_mask    = 0
  local best_penalty = math.huge

  for m = 0, 7 do
    local masked   = apply_mask(grid.modules, grid.reserved, sz, m)
    local fmt_bits = compute_format_bits(level, m)
    -- Create a temporary grid with format info to include it in penalty
    local test_g = { size = sz, modules = masked, reserved = grid.reserved }
    write_format_info(test_g, fmt_bits)
    local p = compute_penalty(masked, sz)
    if p < best_penalty then
      best_penalty = p
      best_mask    = m
    end
  end

  -- Step 8: finalize with the best mask
  local final_mods = apply_mask(grid.modules, grid.reserved, sz, best_mask)
  local final_g    = { size = sz, modules = final_mods, reserved = grid.reserved }
  write_format_info(final_g, compute_format_bits(level, best_mask))
  write_version_info(final_g, version)

  -- Return a ModuleGrid table compatible with barcode_2d.layout()
  return {
    rows         = sz,
    cols         = sz,
    modules      = final_mods,
    module_shape = "square",
    version      = version,
    ecc_level    = level,
  }, nil
end

return M
