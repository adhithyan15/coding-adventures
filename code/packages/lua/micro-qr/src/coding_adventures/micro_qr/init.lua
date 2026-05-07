-- coding-adventures-micro-qr
--
-- Micro QR Code encoder — ISO/IEC 18004:2015 Annex E compliant.
--
-- ## What is Micro QR Code?
--
-- Micro QR Code is the compact sibling of regular QR Code, designed for
-- applications where even the smallest standard QR (21×21) is too large.
-- Think surface-mount electronic component labels on circuit boards, miniature
-- product markings, and tiny industrial tags scanned in controlled environments.
--
-- The defining structural difference: Micro QR uses a SINGLE finder pattern in
-- the top-left corner, rather than regular QR's three corner finders. Because
-- there is only one, orientation is always unambiguous — the data area is always
-- to the bottom-right of the single finder. This saves enormous space at the
-- cost of needing a controlled scanning environment.
--
-- ## Symbol sizes
--
--   M1: 11×11   M2: 13×13   M3: 15×15   M4: 17×17
--   formula: size = 2 × version_number + 9
--
-- ## Key differences from regular QR Code
--
--   - Single finder pattern at top-left only (one 7×7 square, not three).
--   - Timing patterns at row 0 / col 0 (not row 6 / col 6).
--   - Only 4 mask patterns (not 8).
--   - Format XOR mask 0x4445 (not 0x5412).
--   - Single copy of format info (not two).
--   - 2-module quiet zone (not 4).
--   - Narrower mode indicators (0–3 bits instead of 4).
--   - Single block RS encoding (no interleaving).
--
-- ## Encoding pipeline
--
--   input string
--     → auto-select smallest symbol (M1..M4) and encoding mode
--     → build bit stream (mode indicator + char count + data + terminator + padding)
--     → Reed-Solomon ECC (GF(256)/0x11D, b=0, single block)
--     → initialize grid (finder, L-shaped separator, timing at row0/col0, format reserved)
--     → zigzag data placement (two-column snake from bottom-right)
--     → evaluate 4 mask patterns, pick lowest penalty
--     → write format information (15 bits, single copy, XOR 0x4445)
--
-- ## Lua conventions
--
-- All grid row/column indices are 1-indexed (Lua convention).
-- Bit operations use Lua 5.4 native operators: &, |, ~, <<, >>.
-- We implement GF(256) arithmetic inline to stay self-contained —
-- the repo's gf256 package uses 0x11D which matches Micro QR, but
-- embedding it avoids the inter-package dependency at runtime.

local M = {}

M.VERSION = "0.1.0"

-- ============================================================================
-- GF(256)/0x11D arithmetic
-- ============================================================================
--
-- Micro QR Reed-Solomon uses GF(256) with primitive polynomial:
--   p(x) = x^8 + x^4 + x^3 + x^2 + 1  =  0x11D
--
-- This is the SAME polynomial used by regular QR Code (and by the gf256
-- package at code/packages/lua/gf256). We inline it here to keep the package
-- self-contained: the only external dependency is Lua 5.4 itself.
--
-- Two look-up tables speed up multiplication:
--   EXP_11D[1+i] = alpha^i in GF(256)/0x11D   (i = 0..254, doubled for wrap)
--   LOG_11D[1+e] = discrete log of e           (e = 1..255)
--
-- Multiplication: a * b = EXP_11D[LOG_11D[a] + LOG_11D[b]] (mod 255).
-- Using doubled EXP avoids the mod by just indexing past 255.

local EXP_11D = {}  -- length 512
local LOG_11D = {}  -- length 256

do
  local x = 1
  for i = 0, 254 do
    EXP_11D[1 + i]       = x
    EXP_11D[1 + i + 255] = x
    LOG_11D[1 + x]       = i
    x = x << 1
    if (x & 0x100) ~= 0 then
      x = x ~ 0x11D
    end
    x = x & 0xFF
  end
  EXP_11D[1 + 255] = 1  -- alpha^255 = alpha^0 = 1 (cyclic group)
end

-- gf_mul(a, b): multiply two GF(256)/0x11D field elements.
local function gf_mul(a, b)
  if a == 0 or b == 0 then return 0 end
  return EXP_11D[1 + LOG_11D[1 + a] + LOG_11D[1 + b]]
end

-- ============================================================================
-- Reed-Solomon encoder — GF(256)/0x11D, b=0 convention
-- ============================================================================
--
-- For Micro QR we need ECC codeword counts: 2, 5, 6, 8, 10, 14.
-- Rather than computing generator polynomials at runtime, we embed them as
-- compile-time constants from ISO 18004:2015 and the Go/TypeScript references.
--
-- The generator polynomial of degree n has n+1 coefficients (including leading 1):
--   g(x) = (x + α^0)(x + α^1)···(x + α^{n-1})
-- Coefficients listed highest-degree first.

local RS_GENERATORS = {
  -- 2 ECC codewords (M1 detection):
  -- g(x) = (x + α^0)(x + α^1) = x^2 + 3x + 2
  [2]  = {0x01, 0x03, 0x02},

  -- 5 ECC codewords (M2-L)
  [5]  = {0x01, 0x1f, 0xf6, 0x44, 0xd9, 0x68},

  -- 6 ECC codewords (M2-M, M3-L)
  [6]  = {0x01, 0x3f, 0x4e, 0x17, 0x9b, 0x05, 0x37},

  -- 8 ECC codewords (M3-M, M4-L)
  [8]  = {0x01, 0x63, 0x0d, 0x60, 0x6d, 0x5b, 0x10, 0xa2, 0xa3},

  -- 10 ECC codewords (M4-M)
  [10] = {0x01, 0xf6, 0x75, 0xa8, 0xd0, 0xc3, 0xe3, 0x36, 0xe1, 0x3c, 0x45},

  -- 14 ECC codewords (M4-Q)
  [14] = {0x01, 0xf6, 0x9a, 0x60, 0x97, 0x8a, 0xf1, 0xa4, 0xa1, 0x8e,
          0xfc, 0x7a, 0x52, 0xad, 0xac},
}

-- rs_encode(data, ecc_count): compute ecc_count ECC bytes for data bytes.
--
-- LFSR polynomial division algorithm:
--   ecc = [0] × n
--   for each data byte b:
--     feedback = b XOR ecc[0]
--     shift ecc left (drop ecc[0], append 0)
--     for i in 0..n-1:
--       ecc[i] ^= GF_MUL(G[n-i], feedback)
--
-- data is a 1-indexed Lua table of byte values.
-- Returns a 1-indexed table of ecc_count ECC bytes.
local function rs_encode(data, ecc_count)
  local gen = RS_GENERATORS[ecc_count]
  assert(gen, string.format("no RS generator for ecc_count=%d", ecc_count))

  local n = ecc_count
  local rem = {}
  for i = 1, n do rem[i] = 0 end

  for _, b in ipairs(data) do
    local fb = b ~ rem[1]
    -- shift left: drop rem[1], shift rem[2..n] to rem[1..n-1], set rem[n]=0
    for i = 1, n - 1 do
      rem[i] = rem[i + 1]
    end
    rem[n] = 0
    -- XOR with scaled generator (skip if feedback is zero — no change)
    if fb ~= 0 then
      for i = 1, n do
        rem[i] = rem[i] ~ gf_mul(gen[i + 1], fb)
      end
    end
  end

  return rem
end

-- ============================================================================
-- Symbol configurations (compile-time constants)
-- ============================================================================
--
-- There are exactly 8 valid (version, ECC) combinations in Micro QR.
-- We encode them as a list in smallest-first order so the auto-selection
-- loop can just iterate and pick the first one that fits.
--
-- Fields:
--   version   — "M1".."M4" (string label)
--   ecc       — "DETECTION"|"L"|"M"|"Q"
--   sym_ind   — 0..7 (used in format information)
--   size      — symbol side length (11/13/15/17)
--   data_cw   — data codewords (full bytes; M1 last is 4-bit)
--   ecc_cw    — ECC codewords
--   numeric   — max numeric chars (0 = not supported)
--   alpha     — max alphanumeric chars (0 = not supported)
--   byte      — max byte-mode chars (0 = not supported)
--   term_bits — terminator zero-bit count (3/5/7/9)
--   mode_bits — mode indicator field width (0/1/2/3)
--   cc_num    — character count field bits for numeric
--   cc_alpha  — character count field bits for alphanumeric
--   cc_byte   — character count field bits for byte
--   m1_half   — true only for M1 (last data codeword is 4-bit nibble)

local SYMBOL_CONFIGS = {
  -- ── M1 / DETECTION ──────────────────────────────────────────────────────
  -- 11×11. Numeric only. 5 digits max. Error detection only (no correction).
  -- 3 data codewords but the last is only 4 bits: total = 20 bits.
  {
    version = "M1", ecc = "DETECTION", sym_ind = 0, size = 11,
    data_cw = 3, ecc_cw = 2,
    numeric = 5, alpha = 0, byte_cap = 0,
    term_bits = 3, mode_bits = 0,
    cc_num = 3, cc_alpha = 0, cc_byte = 0,
    m1_half = true,
  },

  -- ── M2 / L ──────────────────────────────────────────────────────────────
  -- 13×13. Numeric, alphanumeric, and byte modes. 5 data + 5 ECC codewords.
  {
    version = "M2", ecc = "L", sym_ind = 1, size = 13,
    data_cw = 5, ecc_cw = 5,
    numeric = 10, alpha = 6, byte_cap = 4,
    term_bits = 5, mode_bits = 1,
    cc_num = 4, cc_alpha = 3, cc_byte = 4,
    m1_half = false,
  },

  -- ── M2 / M ──────────────────────────────────────────────────────────────
  -- Same 13×13 grid, 4 data + 6 ECC = more redundancy, less data.
  {
    version = "M2", ecc = "M", sym_ind = 2, size = 13,
    data_cw = 4, ecc_cw = 6,
    numeric = 8, alpha = 5, byte_cap = 3,
    term_bits = 5, mode_bits = 1,
    cc_num = 4, cc_alpha = 3, cc_byte = 4,
    m1_half = false,
  },

  -- ── M3 / L ──────────────────────────────────────────────────────────────
  -- 15×15. 11 data + 6 ECC codewords.
  {
    version = "M3", ecc = "L", sym_ind = 3, size = 15,
    data_cw = 11, ecc_cw = 6,
    numeric = 23, alpha = 14, byte_cap = 9,
    term_bits = 7, mode_bits = 2,
    cc_num = 5, cc_alpha = 4, cc_byte = 4,
    m1_half = false,
  },

  -- ── M3 / M ──────────────────────────────────────────────────────────────
  -- Same 15×15 grid, 9 data + 8 ECC.
  {
    version = "M3", ecc = "M", sym_ind = 4, size = 15,
    data_cw = 9, ecc_cw = 8,
    numeric = 18, alpha = 11, byte_cap = 7,
    term_bits = 7, mode_bits = 2,
    cc_num = 5, cc_alpha = 4, cc_byte = 4,
    m1_half = false,
  },

  -- ── M4 / L ──────────────────────────────────────────────────────────────
  -- 17×17. 16 data + 8 ECC codewords. Maximum capacity for numeric/alpha/byte.
  {
    version = "M4", ecc = "L", sym_ind = 5, size = 17,
    data_cw = 16, ecc_cw = 8,
    numeric = 35, alpha = 21, byte_cap = 15,
    term_bits = 9, mode_bits = 3,
    cc_num = 6, cc_alpha = 5, cc_byte = 5,
    m1_half = false,
  },

  -- ── M4 / M ──────────────────────────────────────────────────────────────
  -- Same 17×17 grid, 14 data + 10 ECC.
  {
    version = "M4", ecc = "M", sym_ind = 6, size = 17,
    data_cw = 14, ecc_cw = 10,
    numeric = 30, alpha = 18, byte_cap = 13,
    term_bits = 9, mode_bits = 3,
    cc_num = 6, cc_alpha = 5, cc_byte = 5,
    m1_half = false,
  },

  -- ── M4 / Q ──────────────────────────────────────────────────────────────
  -- Same 17×17 grid, 10 data + 14 ECC. Highest redundancy in Micro QR (~25%).
  {
    version = "M4", ecc = "Q", sym_ind = 7, size = 17,
    data_cw = 10, ecc_cw = 14,
    numeric = 21, alpha = 13, byte_cap = 9,
    term_bits = 9, mode_bits = 3,
    cc_num = 6, cc_alpha = 5, cc_byte = 5,
    m1_half = false,
  },
}

-- ============================================================================
-- Pre-computed format information table
-- ============================================================================
--
-- The 15-bit format word encodes:
--   [symbol_indicator (3b)][mask_pattern (2b)][BCH-10 remainder (10b)]
-- then XOR-masked with 0x4445 (Micro QR specific; regular QR uses 0x5412).
--
-- Indexed as FORMAT_TABLE[sym_ind + 1][mask + 1] (1-indexed for Lua).
--
-- All 32 values (8 symbol+ECC × 4 masks), pre-computed from ISO 18004:2015.

local FORMAT_TABLE = {
  {0x4445, 0x4172, 0x4E2B, 0x4B1C},  -- M1 / DETECTION  (sym_ind=0)
  {0x5528, 0x501F, 0x5F46, 0x5A71},  -- M2-L             (sym_ind=1)
  {0x6649, 0x637E, 0x6C27, 0x6910},  -- M2-M             (sym_ind=2)
  {0x7764, 0x7253, 0x7D0A, 0x783D},  -- M3-L             (sym_ind=3)
  {0x06DE, 0x03E9, 0x0CB0, 0x0987},  -- M3-M             (sym_ind=4)
  {0x17F3, 0x12C4, 0x1D9D, 0x18AA},  -- M4-L             (sym_ind=5)
  {0x24B2, 0x2185, 0x2EDC, 0x2BEB},  -- M4-M             (sym_ind=6)
  {0x359F, 0x30A8, 0x3FF1, 0x3AC6},  -- M4-Q             (sym_ind=7)
}

-- ============================================================================
-- Alphanumeric character set
-- ============================================================================
--
-- The 45-character set used by alphanumeric mode (identical to regular QR).
-- Position of a character in this string IS its numeric index for encoding:
--   pair encoding = firstIndex × 45 + secondIndex in 11 bits
--   single trailing char = 6 bits

local ALPHANUM_CHARS = "0123456789ABCDEFGHIJKLMNOPQRSTUVWXYZ $%*+-./:";

-- alphanum_index(c): returns 0-based index of byte value c in ALPHANUM_CHARS,
-- or nil if not found.
local ALPHANUM_INDEX = {}  -- char value → 0-based index
do
  for i = 1, #ALPHANUM_CHARS do
    local byte_val = string.byte(ALPHANUM_CHARS, i)
    ALPHANUM_INDEX[byte_val] = i - 1  -- 0-indexed
  end
end

-- ============================================================================
-- Mode selection
-- ============================================================================
--
-- Three encoding modes supported (kanji is future work):
--   "numeric"      — digits 0–9 only
--   "alphanumeric" — digits + A-Z + 7 symbols (the 45-char set)
--   "byte"         — raw UTF-8 bytes
--
-- We pick the most compact mode that covers the entire input string AND
-- is supported by the given symbol configuration.

-- is_all_numeric(s): true if every byte of s is an ASCII digit.
local function is_all_numeric(s)
  for i = 1, #s do
    local b = string.byte(s, i)
    if b < 48 or b > 57 then return false end  -- '0'=48, '9'=57
  end
  return true
end

-- is_all_alphanumeric(s): true if every byte of s is in ALPHANUM_CHARS.
local function is_all_alphanumeric(s)
  for i = 1, #s do
    local b = string.byte(s, i)
    if ALPHANUM_INDEX[b] == nil then return false end
  end
  return true
end

-- select_mode(input, cfg): returns "numeric"|"alphanumeric"|"byte" or nil.
-- Returns nil if no mode is supported by this configuration for this input.
local function select_mode(input, cfg)
  -- Numeric: all digits AND this symbol supports numeric
  if is_all_numeric(input) and cfg.cc_num > 0 then
    return "numeric"
  end
  -- Alphanumeric: all in 45-char set AND this symbol supports alphanumeric
  if is_all_alphanumeric(input) and cfg.alpha > 0 then
    return "alphanumeric"
  end
  -- Byte: anything AND this symbol supports byte mode
  if cfg.byte_cap > 0 then
    return "byte"
  end
  return nil
end

-- mode_indicator_value(mode, cfg): returns the mode indicator integer value.
--
-- M1 has no indicator (0 bits, only numeric mode exists).
-- M2 uses 1 bit: 0=numeric, 1=alphanumeric.
-- M3 uses 2 bits: 00=numeric, 01=alphanumeric, 10=byte.
-- M4 uses 3 bits: 000=numeric, 001=alphanumeric, 010=byte.
local function mode_indicator_value(mode, cfg)
  if cfg.mode_bits == 0 then return 0 end  -- M1
  if cfg.mode_bits == 1 then
    return mode == "numeric" and 0 or 1
  end
  if cfg.mode_bits == 2 then
    if mode == "numeric"      then return 0 end
    if mode == "alphanumeric" then return 1 end
    return 2  -- byte
  end
  -- mode_bits == 3 (M4)
  if mode == "numeric"      then return 0 end
  if mode == "alphanumeric" then return 1 end
  return 2  -- byte
end

-- char_count_bits(mode, cfg): width of the character count field.
local function char_count_bits(mode, cfg)
  if mode == "numeric"      then return cfg.cc_num   end
  if mode == "alphanumeric" then return cfg.cc_alpha end
  return cfg.cc_byte  -- byte
end

-- ============================================================================
-- Bit writer — accumulates bits, flushes to bytes
-- ============================================================================
--
-- QR and Micro QR use MSB-first bit ordering: the first bit written becomes
-- the most-significant bit of the first byte.
--
-- Internally we keep an array of 0/1 values (one per bit), which is the
-- simplest approach for step-by-step bit construction. We flush to a byte
-- array on demand.

-- new_bit_writer(): creates a fresh bit writer.
local function new_bit_writer()
  return {bits = {}, n = 0}
end

-- bw_write(bw, value, count): append `count` bits from `value`, MSB first.
local function bw_write(bw, value, count)
  for i = count - 1, 0, -1 do
    bw.n = bw.n + 1
    bw.bits[bw.n] = (value >> i) & 1
  end
end

-- bw_to_bytes(bw): pack bits into a 1-indexed byte array (trailing zero-pad).
local function bw_to_bytes(bw)
  local bytes = {}
  local nbytes = (bw.n + 7) // 8
  for i = 1, nbytes do
    local b = 0
    for j = 0, 7 do
      local bit_idx = (i - 1) * 8 + j + 1
      local bit = bw.bits[bit_idx] or 0
      b = (b << 1) | bit
    end
    bytes[i] = b
  end
  return bytes
end

-- ============================================================================
-- Data encoding helpers
-- ============================================================================

-- encode_numeric(input, bw): pack decimal digit groups into the bit writer.
--
-- Groups of 3 digits → 10 bits (values 0–999).
-- Remaining 2 digits → 7 bits (values 0–99).
-- Single trailing digit → 4 bits (values 0–9).
--
-- Example: "12345" → "123" (10b=0001111011) + "45" (7b=0101101) = 17 bits.
local function encode_numeric(input, bw)
  local n = #input
  local i = 1
  while i + 2 <= n do
    local val = (string.byte(input, i) - 48) * 100
               + (string.byte(input, i+1) - 48) * 10
               + (string.byte(input, i+2) - 48)
    bw_write(bw, val, 10)
    i = i + 3
  end
  if i + 1 <= n then
    local val = (string.byte(input, i) - 48) * 10
               + (string.byte(input, i+1) - 48)
    bw_write(bw, val, 7)
    i = i + 2
  end
  if i <= n then
    bw_write(bw, string.byte(input, i) - 48, 4)
  end
end

-- encode_alphanumeric(input, bw): pack alphanumeric characters into bit writer.
--
-- Pairs → (first_index × 45 + second_index) in 11 bits.
-- Trailing single character → 6 bits.
local function encode_alphanumeric(input, bw)
  local n = #input
  local i = 1
  while i + 1 <= n do
    local a = ALPHANUM_INDEX[string.byte(input, i)]
    local b_val = ALPHANUM_INDEX[string.byte(input, i+1)]
    bw_write(bw, a * 45 + b_val, 11)
    i = i + 2
  end
  if i <= n then
    local a = ALPHANUM_INDEX[string.byte(input, i)]
    bw_write(bw, a, 6)
  end
end

-- encode_byte(input, bw): write each UTF-8 byte as 8 bits.
local function encode_byte(input, bw)
  for i = 1, #input do
    bw_write(bw, string.byte(input, i), 8)
  end
end

-- ============================================================================
-- Data codeword assembly
-- ============================================================================
--
-- For all symbols except M1:
--   [mode indicator (0/1/2/3 bits)] [char count] [data] [terminator]
--   [zero-pad to byte boundary] [0xEC/0x11 fill to reach data_cw bytes]
--
-- For M1 (m1_half = true):
--   Total capacity = 20 bits (2 full bytes + 4-bit nibble).
--   Pad to 20 bits with zeros; no 0xEC/0x11 padding.
--   Pack into 3 bytes where byte[3] has data in upper 4 bits, lower 4 = 0.

-- build_data_codewords(input, cfg, mode): returns a 1-indexed byte array.
local function build_data_codewords(input, cfg, mode)
  -- Total usable data bit capacity.
  -- M1 special: 3 codewords but last is only 4 bits → 3×8−4 = 20 bits.
  local total_bits = cfg.data_cw * 8
  if cfg.m1_half then
    total_bits = total_bits - 4  -- M1: 20 bits
  end

  local bw = new_bit_writer()

  -- Mode indicator (skipped for M1 which has no choice)
  if cfg.mode_bits > 0 then
    bw_write(bw, mode_indicator_value(mode, cfg), cfg.mode_bits)
  end

  -- Character count field.
  -- For byte mode: count UTF-8 bytes (each byte counts separately).
  -- For others: count string length in rune/character units.
  -- Since Lua strings are byte strings, #input gives byte count directly.
  -- For numeric/alphanumeric, inputs are all ASCII so #input = char count.
  local char_count = #input
  bw_write(bw, char_count, char_count_bits(mode, cfg))

  -- Encoded data bits
  if mode == "numeric" then
    encode_numeric(input, bw)
  elseif mode == "alphanumeric" then
    encode_alphanumeric(input, bw)
  else  -- byte
    encode_byte(input, bw)
  end

  -- Terminator: up to term_bits zero bits, truncated if capacity exhausted.
  local remaining = total_bits - bw.n
  if remaining > 0 then
    bw_write(bw, 0, math.min(cfg.term_bits, remaining))
  end

  -- M1 special case: pad to exactly 20 bits, pack into 3 bytes.
  if cfg.m1_half then
    local bits = bw.bits
    -- Extend to 20 bits with zeros
    while bw.n < 20 do
      bw.n = bw.n + 1
      bits[bw.n] = 0
    end
    -- Pack: byte0 = bits[1..8], byte1 = bits[9..16], byte2 = bits[17..20]<<4
    local b0 = 0
    local b1 = 0
    local b2 = 0
    for j = 0, 7 do
      b0 = (b0 << 1) | (bits[j + 1] or 0)
    end
    for j = 0, 7 do
      b1 = (b1 << 1) | (bits[8 + j + 1] or 0)
    end
    -- Only upper nibble: bits[17..20] shift to bits 7..4 of byte2
    for j = 0, 3 do
      b2 = (b2 << 1) | (bits[16 + j + 1] or 0)
    end
    b2 = b2 << 4  -- shift to upper nibble
    return {b0, b1, b2}
  end

  -- Pad to byte boundary with zero bits
  local rem = bw.n % 8
  if rem ~= 0 then
    bw_write(bw, 0, 8 - rem)
  end

  -- Convert to byte array
  local bytes = bw_to_bytes(bw)

  -- Fill remaining data codewords with alternating 0xEC / 0x11.
  -- These bytes were chosen because their patterns avoid degenerate sequences
  -- in the Reed-Solomon encoded stream.
  local pad = 0xEC
  while #bytes < cfg.data_cw do
    bytes[#bytes + 1] = pad
    pad = pad == 0xEC and 0x11 or 0xEC
  end

  return bytes
end

-- ============================================================================
-- Symbol selection (auto-detect version+ECC)
-- ============================================================================
--
-- Iterate SYMBOL_CONFIGS in order (smallest/lowest-ECC first) and return
-- the first configuration that:
--   1. Matches the requested version (if non-nil) and ECC (if non-nil).
--   2. Supports a valid encoding mode for the input.
--   3. Has enough capacity for the input length.

-- select_config(input, version, ecc): returns a config table or nil, err_msg.
-- version: nil or "M1"|"M2"|"M3"|"M4"
-- ecc:     nil or "DETECTION"|"L"|"M"|"Q"
local function select_config(input, version, ecc)
  -- Build candidate list (filter by version/ecc constraints)
  local candidates = {}
  for _, cfg in ipairs(SYMBOL_CONFIGS) do
    if version == nil or cfg.version == version then
      if ecc == nil or cfg.ecc == ecc then
        candidates[#candidates + 1] = cfg
      end
    end
  end

  if #candidates == 0 then
    return nil, string.format(
      "no symbol configuration matches version=%s ecc=%s",
      tostring(version), tostring(ecc))
  end

  for _, cfg in ipairs(candidates) do
    local mode = select_mode(input, cfg)
    if mode then
      local input_len = #input  -- byte count (= char count for ASCII)
      local cap
      if mode == "numeric"      then cap = cfg.numeric    end
      if mode == "alphanumeric" then cap = cfg.alpha      end
      if mode == "byte"         then cap = cfg.byte_cap   end
      if cap and cap > 0 and input_len <= cap then
        return cfg, mode
      end
    end
  end

  return nil, string.format(
    "input (length %d) does not fit in any Micro QR symbol " ..
    "(version=%s, ecc=%s). Maximum is 35 numeric chars in M4-L.",
    #input, tostring(version), tostring(ecc))
end

-- ============================================================================
-- Working grid construction
-- ============================================================================
--
-- The working grid has two parallel 2D arrays:
--   modules[r][c]  — boolean, true = dark
--   reserved[r][c] — boolean, true = structural (not for data)
-- Both are 1-indexed.

-- make_work_grid(size): create a fresh size×size working grid.
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
  return modules, reserved
end

-- grid_set(modules, reserved, r, c, dark, res): set module value and reservation.
-- r, c are 1-indexed.
local function grid_set(modules, reserved, r, c, dark, res)
  modules[r][c] = dark
  if res then reserved[r][c] = true end
end

-- ============================================================================
-- Structural module placement
-- ============================================================================

-- place_finder(modules, reserved):
-- Place the 7×7 finder pattern at rows 1–7, cols 1–7 (1-indexed).
--
-- The pattern:
--   ■ ■ ■ ■ ■ ■ ■
--   ■ □ □ □ □ □ ■
--   ■ □ ■ ■ ■ □ ■
--   ■ □ ■ ■ ■ □ ■
--   ■ □ ■ ■ ■ □ ■
--   ■ □ □ □ □ □ ■
--   ■ ■ ■ ■ ■ ■ ■
--
-- Dark modules: outer border (row/col 0 or 6, 0-indexed) and 3×3 core (rows 2–4).
-- The 1:1:3:1:1 dark-light-dark ratio is what scanners detect as a finder.
local function place_finder(modules, reserved)
  for dr = 0, 6 do
    for dc = 0, 6 do
      local on_border = (dr == 0 or dr == 6 or dc == 0 or dc == 6)
      local in_core   = (dr >= 2 and dr <= 4 and dc >= 2 and dc <= 4)
      grid_set(modules, reserved, dr+1, dc+1, on_border or in_core, true)
    end
  end
end

-- place_separator(modules, reserved, size):
-- L-shaped separator: row 7 (cols 0–7) and col 7 (rows 0–7), all light.
--
-- In regular QR, finder patterns have separators on all four sides. In Micro QR
-- the finder is at the top-left corner, so only the bottom and right sides need
-- separating from the data area (top and left edges ARE the symbol boundary).
local function place_separator(modules, reserved)
  for i = 0, 7 do
    grid_set(modules, reserved, 8,   i+1, false, true)  -- row 7 (0-idx), bottom
    grid_set(modules, reserved, i+1, 8,   false, true)  -- col 7 (0-idx), right
  end
end

-- place_timing(modules, reserved, size):
-- Timing pattern extensions beyond the finder+separator area.
--
-- Row 0 (1-indexed row 1), cols 8 to size-1 (0-indexed): dark if col is even.
-- Col 0 (1-indexed col 1), rows 8 to size-1 (0-indexed): dark if row is even.
--
-- The finder pattern already covers cols 0–6 and the separator covers col 7 on
-- row 0; similarly for col 0. The timing extension starts at position 8
-- (0-indexed) = index 9 (1-indexed).
local function place_timing(modules, reserved, size)
  -- Row 0 (1-indexed row 1): extend timing from col 8 onward
  for c0 = 8, size - 1 do
    grid_set(modules, reserved, 1, c0+1, c0 % 2 == 0, true)
  end
  -- Col 0 (1-indexed col 1): extend timing from row 8 onward
  for r0 = 8, size - 1 do
    grid_set(modules, reserved, r0+1, 1, r0 % 2 == 0, true)
  end
end

-- reserve_format_info(modules, reserved):
-- Mark the 15 format information positions as reserved (initially light).
--
-- The 15 modules form an L-shape:
--   Row 8 (1-indexed row 9), cols 1–8  → 8 modules (bits f14..f7)
--   Col 8 (1-indexed col 9), rows 1–7  → 7 modules (bits f6..f0)
--
-- These are overwritten after mask selection by write_format_info().
local function reserve_format_info(modules, reserved)
  for c0 = 1, 8 do
    grid_set(modules, reserved, 9, c0+1, false, true)   -- row 8, cols 1-8 (0-idx)
  end
  for r0 = 1, 7 do
    grid_set(modules, reserved, r0+1, 9, false, true)   -- col 8, rows 1-7 (0-idx)
  end
end

-- write_format_info(modules, fmt_bits):
-- Write a 15-bit format word into the reserved format positions.
--
-- Placement (f14 = MSB, f0 = LSB):
--   Row 8 (r=9), col 1 (c=2)  ← f14 (MSB)
--   Row 8 (r=9), col 2 (c=3)  ← f13
--   ...
--   Row 8 (r=9), col 8 (c=9)  ← f7
--   Col 8 (c=9), row 7 (r=8)  ← f6
--   Col 8 (c=9), row 6 (r=7)  ← f5
--   ...
--   Col 8 (c=9), row 1 (r=2)  ← f0 (LSB)
--
-- The row-8 strip goes left-to-right (MSB first).
-- The col-8 strip goes upward (row 7 → row 1), so LSB is nearest the finder.
local function write_format_info(modules, fmt_bits)
  -- Row 8 (1-indexed row 9), cols 1–8 (0-indexed) = 1-indexed cols 2–9
  -- Bits f14 down to f7
  for i = 0, 7 do
    modules[9][2 + i] = ((fmt_bits >> (14 - i)) & 1) == 1
  end
  -- Col 8 (1-indexed col 9), rows 7 down to 1 (0-indexed) = 1-indexed rows 8 down to 2
  -- Bits f6 down to f0
  for i = 0, 6 do
    modules[8 - i][9] = ((fmt_bits >> (6 - i)) & 1) == 1
  end
end

-- build_grid(cfg): create and populate the initial working grid.
local function build_grid(cfg)
  local size = cfg.size
  local modules, reserved = make_work_grid(size)
  place_finder(modules, reserved)
  place_separator(modules, reserved)
  place_timing(modules, reserved, size)
  reserve_format_info(modules, reserved)
  return modules, reserved
end

-- ============================================================================
-- Data placement — two-column zigzag
-- ============================================================================
--
-- The zigzag scans from bottom-right, moving left two columns at a time,
-- alternating upward/downward direction:
--
--   col = size-1, direction = up (scan from row size-1 down to row 0)
--     for each row in direction:
--       try col (right cell) and col-1 (left cell)
--       skip if reserved
--       place next bit (or 0 for remainder)
--   flip direction, col -= 2
--   repeat while col >= 1
--
-- M1 has 4 remainder bits (the last 4 unreserved positions after data+ECC).
-- All other versions have 0 remainder bits.

-- place_bits(modules, reserved, bits, size):
-- bits is a 1-indexed array of 0/1 values (or booleans).
local function place_bits(modules, reserved, bits, size)
  local bit_idx = 1
  local up = true

  local col = size - 1  -- 0-indexed starting column (rightmost)
  while col >= 1 do
    for vi = 0, size - 1 do
      local row0  -- 0-indexed row
      if up then
        row0 = size - 1 - vi  -- scan from bottom to top
      else
        row0 = vi             -- scan from top to bottom
      end

      -- Try the right cell (col) then the left cell (col-1)
      for _, dc in ipairs({0, 1}) do
        local c0 = col - dc  -- 0-indexed column
        local r1 = row0 + 1  -- 1-indexed row
        local c1 = c0 + 1    -- 1-indexed col

        if not reserved[r1][c1] then
          if bit_idx <= #bits then
            local b = bits[bit_idx]
            modules[r1][c1] = (type(b) == "boolean") and b or (b == 1)
            bit_idx = bit_idx + 1
          else
            modules[r1][c1] = false  -- remainder bit = 0
          end
        end
      end
    end

    up = not up
    col = col - 2
  end
end

-- ============================================================================
-- Masking
-- ============================================================================
--
-- Micro QR uses 4 mask patterns (the first 4 of regular QR's 8).
-- A module is flipped if it is NOT reserved and the mask condition is true.
--
-- | Pattern | Condition                    |
-- |---------|------------------------------|
-- | 0       | (row + col) mod 2 == 0       |
-- | 1       | row mod 2 == 0               |
-- | 2       | col mod 3 == 0               |
-- | 3       | (row + col) mod 3 == 0       |
--
-- Row and col are 0-indexed when evaluating the condition.

-- mask_condition(mask_idx, r0, c0): true if module should be flipped.
-- r0, c0 are 0-indexed.
local function mask_condition(mask_idx, r0, c0)
  if mask_idx == 0 then return (r0 + c0) % 2 == 0 end
  if mask_idx == 1 then return r0 % 2 == 0         end
  if mask_idx == 2 then return c0 % 3 == 0         end
  -- mask_idx == 3
  return (r0 + c0) % 3 == 0
end

-- apply_mask(modules, reserved, size, mask_idx):
-- Returns a NEW 2D array with the mask applied to non-reserved modules.
local function apply_mask(modules, reserved, size, mask_idx)
  local result = {}
  for r = 1, size do
    result[r] = {}
    for c = 1, size do
      if reserved[r][c] then
        result[r][c] = modules[r][c]
      else
        -- Flip if condition is true; r-1 and c-1 convert to 0-indexed
        local cond = mask_condition(mask_idx, r - 1, c - 1)
        result[r][c] = modules[r][c] ~= cond
      end
    end
  end
  return result
end

-- ============================================================================
-- Penalty scoring
-- ============================================================================
--
-- Four penalty rules (same as regular QR Code). The mask with the lowest total
-- penalty is selected. Ties are broken by preferring the lower-numbered pattern.
--
-- Rule 1: runs of ≥5 same-color modules in any row or column.
--   Score += (run_length - 2) for each qualifying run.
--
-- Rule 2: 2×2 same-color blocks.
--   Score += 3 for each 2×2 block where all four modules share a color.
--
-- Rule 3: finder-pattern-like 11-module sequences.
--   Score += 40 for each occurrence of 1 0 1 1 1 0 1 0 0 0 0 or its reverse.
--
-- Rule 4: dark-module proportion deviation from 50%.
--   dark_pct = dark_count × 100 / total
--   prev5 = largest multiple of 5 ≤ dark_pct
--   Score += min(|prev5-50|, |prev5+5-50|) / 5 × 10

local FINDER_PATTERN_1 = {1, 0, 1, 1, 1, 0, 1, 0, 0, 0, 0}
local FINDER_PATTERN_2 = {0, 0, 0, 0, 1, 0, 1, 1, 1, 0, 1}

local function compute_penalty(modules, size)
  local penalty = 0

  -- Rule 1: runs of ≥5 same-color modules
  for a = 1, size do
    for _, horiz in ipairs({true, false}) do
      local run = 1
      local prev = horiz and modules[a][1] or modules[1][a]
      for i = 2, size do
        local cur = horiz and modules[a][i] or modules[i][a]
        if cur == prev then
          run = run + 1
        else
          if run >= 5 then penalty = penalty + (run - 2) end
          run = 1
          prev = cur
        end
      end
      if run >= 5 then penalty = penalty + (run - 2) end
    end
  end

  -- Rule 2: 2×2 same-color blocks
  for r = 1, size - 1 do
    for c = 1, size - 1 do
      local d = modules[r][c]
      if d == modules[r][c+1] and d == modules[r+1][c] and d == modules[r+1][c+1] then
        penalty = penalty + 3
      end
    end
  end

  -- Rule 3: finder-pattern-like 11-module sequences
  for a = 1, size do
    local limit = size - 11
    for b = 0, limit do
      local mh1, mh2, mv1, mv2 = true, true, true, true
      for k = 1, 11 do
        local bh = modules[a][b + k] and 1 or 0
        local bv = modules[b + k][a] and 1 or 0
        if bh ~= FINDER_PATTERN_1[k] then mh1 = false end
        if bh ~= FINDER_PATTERN_2[k] then mh2 = false end
        if bv ~= FINDER_PATTERN_1[k] then mv1 = false end
        if bv ~= FINDER_PATTERN_2[k] then mv2 = false end
      end
      if mh1 then penalty = penalty + 40 end
      if mh2 then penalty = penalty + 40 end
      if mv1 then penalty = penalty + 40 end
      if mv2 then penalty = penalty + 40 end
    end
  end

  -- Rule 4: dark-module proportion penalty
  local dark = 0
  for r = 1, size do
    for c = 1, size do
      if modules[r][c] then dark = dark + 1 end
    end
  end
  local total = size * size
  local dark_pct = (dark * 100) // total
  local prev5 = (dark_pct // 5) * 5
  local next5 = prev5 + 5
  local d1 = math.abs(prev5 - 50)
  local d2 = math.abs(next5 - 50)
  penalty = penalty + (math.min(d1, d2) // 5) * 10

  return penalty
end

-- ============================================================================
-- Public API
-- ============================================================================

-- encode(input, options) → grid, err
--
-- Encodes a string to a Micro QR Code symbol.
--
-- Parameters:
--   input   — string (UTF-8 / raw bytes)
--   options — optional table:
--     version  — "M1"|"M2"|"M3"|"M4" (nil = auto-select)
--     ecc      — "DETECTION"|"L"|"M"|"Q" (nil = auto-select)
--
-- Returns:
--   On success: grid, nil
--     grid = {
--       rows         — symbol side length (11/13/15/17)
--       cols         — same as rows
--       modules      — 1-indexed boolean[rows][cols]; true = dark
--       module_shape — "square"
--       version      — "M1"|"M2"|"M3"|"M4"
--       ecc          — "DETECTION"|"L"|"M"|"Q"
--     }
--   On failure: nil, err_message (string)
function M.encode(input, options)
  options = options or {}

  if type(input) ~= "string" then
    return nil, "MicroQRError: input must be a string"
  end

  local req_version = options.version  -- nil or "M1".."M4"
  local req_ecc     = options.ecc      -- nil or "DETECTION"|"L"|"M"|"Q"

  -- Step 1: Select symbol configuration
  local cfg, mode_or_err = select_config(input, req_version, req_ecc)
  if cfg == nil then
    return nil, "MicroQRError: " .. mode_or_err
  end
  local mode = mode_or_err  -- renamed for clarity

  -- Step 2: Build data codewords
  local data_cw = build_data_codewords(input, cfg, mode)

  -- Step 3: Compute Reed-Solomon ECC codewords
  local ecc_cw = rs_encode(data_cw, cfg.ecc_cw)

  -- Step 4: Flatten codewords to a boolean bit array.
  -- For M1: the last data codeword (index data_cw) is a half-codeword
  -- containing data only in its upper 4 bits. We emit only 4 bits for it.
  local final_cw = {}
  for _, b in ipairs(data_cw) do final_cw[#final_cw + 1] = b end
  for _, b in ipairs(ecc_cw)  do final_cw[#final_cw + 1] = b end

  local bits = {}
  for cw_idx, cw in ipairs(final_cw) do
    local bits_in_cw = 8
    if cfg.m1_half and cw_idx == cfg.data_cw then
      -- M1 last data codeword: only 4 bits (upper nibble)
      bits_in_cw = 4
    end
    -- Emit bits_in_cw bits from the most-significant end
    for b = bits_in_cw - 1, 0, -1 do
      bits[#bits + 1] = ((cw >> (b + (8 - bits_in_cw))) & 1) == 1
    end
  end

  -- Step 5: Build the initial grid (structural modules)
  local modules, reserved = build_grid(cfg)

  -- Step 6: Place data+ECC bits via zigzag
  place_bits(modules, reserved, bits, cfg.size)

  -- Step 7: Evaluate all 4 mask patterns; pick lowest penalty
  local best_mask    = 0
  local best_penalty = math.maxinteger

  for mask_idx = 0, 3 do
    local masked = apply_mask(modules, reserved, cfg.size, mask_idx)
    local fmt_bits = FORMAT_TABLE[cfg.sym_ind + 1][mask_idx + 1]
    -- Write format info into a temporary copy to include in penalty scoring.
    -- We must deep-copy `masked` to avoid modifying it.
    local tmp = {}
    for r = 1, cfg.size do
      tmp[r] = {}
      for c = 1, cfg.size do tmp[r][c] = masked[r][c] end
    end
    write_format_info(tmp, fmt_bits)
    local p = compute_penalty(tmp, cfg.size)
    if p < best_penalty then
      best_penalty = p
      best_mask = mask_idx
    end
  end

  -- Step 8: Apply best mask and write final format information
  local final_modules = apply_mask(modules, reserved, cfg.size, best_mask)
  local final_fmt = FORMAT_TABLE[cfg.sym_ind + 1][best_mask + 1]
  write_format_info(final_modules, final_fmt)

  return {
    rows         = cfg.size,
    cols         = cfg.size,
    modules      = final_modules,
    module_shape = "square",
    version      = cfg.version,
    ecc          = cfg.ecc,
  }, nil
end

return M
