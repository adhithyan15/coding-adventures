-- coding-adventures-pdf417
--
-- # PDF417 stacked linear barcode encoder — ISO/IEC 15438:2015 compliant.
--
-- PDF417 (Portable Data File 417) was invented by Ynjiun P. Wang at Symbol
-- Technologies in 1991.  The name encodes its geometry: every codeword has
-- exactly **4** bars and **4** spaces (8 elements), and every codeword
-- occupies exactly **17** modules of horizontal space.  4 + 1 + 7 = "417".
--
-- # Where PDF417 is deployed
--
--   - AAMVA          — North American driver's licences and government IDs.
--   - IATA BCBP      — Airline boarding passes (the long thin barcode you
--                      scan at the gate).
--   - USPS           — Domestic shipping labels.
--   - US immigration — Form I-94, customs declarations.
--   - Healthcare     — Patient wristbands, medication labels.
--
-- # Encoding pipeline
--
--   raw bytes
--     -> byte compaction     (codeword 924 latch + 6-bytes-to-5-codewords base-900)
--     -> length descriptor   (codeword 0 = total codewords in symbol)
--     -> RS ECC              (GF(929) Reed-Solomon, b=3 convention, alpha=3)
--     -> dimension selection (auto: roughly square symbol)
--     -> padding             (codeword 900 fills unused slots)
--     -> row indicators      (LRI + RRI per row, encode R/C/ECC level)
--     -> cluster table lookup (codeword -> 17-module bar/space pattern)
--     -> start/stop patterns (fixed per row)
--     -> ModuleGrid          (abstract boolean grid)
--
-- # v0.1.0 scope
--
-- This release implements **byte compaction only** — every input byte (or
-- group of 6 bytes) is encoded directly without character-set translation.
-- Text and numeric compaction are planned for v0.2.0.  Byte mode handles
-- arbitrary binary content correctly, so it is the safe default for
-- general-purpose PDF417 encoding.
--
-- # Quick start
--
--   local pdf417 = require("coding_adventures.pdf417")
--   local grid = pdf417.encode("HELLO WORLD")
--   -- grid.rows / grid.cols give symbol dimensions in modules
--   -- grid.modules[r][c] == true  means dark, false means light  (1-indexed)
--
-- All public coordinates and tables are 1-indexed (Lua convention).  Lua 5.4
-- bitwise operators are used (`>>`, `&`, `|`).

local cluster_tables_mod = require("coding_adventures.pdf417.cluster_tables")

local CLUSTER_TABLES = cluster_tables_mod.CLUSTER_TABLES
local START_PATTERN  = cluster_tables_mod.START_PATTERN
local STOP_PATTERN   = cluster_tables_mod.STOP_PATTERN

local M = {}

M.VERSION = "0.1.0"

-- ============================================================================
-- Public error kinds
-- ============================================================================
--
-- Encoding can fail in three structured ways.  Errors are returned as the
-- second value (grid, err) — the same idiom used by data-matrix and qr-code
-- so that callers can pattern-match on err.kind.

M.PDF417Error              = "PDF417Error"
M.InputTooLongError        = "InputTooLongError"
M.InvalidDimensionsError   = "InvalidDimensionsError"
M.InvalidECCLevelError     = "InvalidECCLevelError"

-- ============================================================================
-- Constants
-- ============================================================================

local GF929_PRIME = 929         -- prime modulus of the field
local GF929_ALPHA = 3           -- primitive root mod 929 (generator)
local GF929_ORDER = 928         -- multiplicative group order = PRIME - 1

local LATCH_BYTE = 924          -- "latch to byte compaction (any length)"
local PADDING_CW = 900          -- neutral padding codeword

local MIN_ROWS = 3
local MAX_ROWS = 90
local MIN_COLS = 1
local MAX_COLS = 30

-- Re-export for tests.
M.GF929_PRIME      = GF929_PRIME
M.GF929_ALPHA      = GF929_ALPHA
M.GF929_ORDER      = GF929_ORDER
M.LATCH_BYTE       = LATCH_BYTE
M.PADDING_CW       = PADDING_CW
M.MIN_ROWS         = MIN_ROWS
M.MAX_ROWS         = MAX_ROWS
M.MIN_COLS         = MIN_COLS
M.MAX_COLS         = MAX_COLS
M.CLUSTER_TABLES   = CLUSTER_TABLES
M.START_PATTERN    = START_PATTERN
M.STOP_PATTERN     = STOP_PATTERN

-- ============================================================================
-- GF(929) arithmetic
-- ============================================================================
--
-- GF(929) is the integers modulo 929.  Since 929 is prime, every non-zero
-- element has a multiplicative inverse.  We use log/antilog lookup tables for
-- O(1) multiplication, built once at module load time.
--
-- The tables take ~7 KB total (929 entries × 2 tables × ~4 bytes per Lua
-- integer) and are built in well under a millisecond.
--
-- Because our tables are 1-indexed (Lua), we store:
--   GF_EXP[i + 1] = alpha^i        for i in 0 .. ORDER (ORDER = ORDER-th wraps to alpha^0)
--   GF_LOG[v + 1] = i              such that alpha^i == v   (v in 1 .. PRIME-1)
--
-- This makes gf_mul a tiny three-line function: two table lookups and one
-- modular addition.

local GF_EXP = {}
local GF_LOG = {}

do
  -- Build exp/log tables.  Algorithm:
  --   start with val = 1 = alpha^0; each step multiply by alpha (mod 929).
  -- After ORDER (=928) steps every non-zero element appears exactly once,
  -- proving alpha = 3 is primitive for GF(929).
  local val = 1
  for i = 0, GF929_ORDER - 1 do
    GF_EXP[i + 1] = val
    GF_LOG[val + 1] = i
    val = (val * GF929_ALPHA) % GF929_PRIME
  end
  -- alpha^ORDER == alpha^0 == 1, so let GF_EXP[ORDER + 1] = 1 for wrap-around
  -- convenience inside gf_mul (no need to take modulo of the index).
  GF_EXP[GF929_ORDER + 1] = GF_EXP[1]
end

-- gf_mul(a, b): multiply two field elements via log/antilog tables.
--
-- For a, b non-zero:  a * b = alpha^{(log[a] + log[b]) mod ORDER}.
-- If either operand is 0 the product is 0 (additive identity absorbs).
local function gf_mul(a, b)
  if a == 0 or b == 0 then return 0 end
  local la = GF_LOG[a + 1]
  local lb = GF_LOG[b + 1]
  return GF_EXP[((la + lb) % GF929_ORDER) + 1]
end

-- gf_add(a, b): add two field elements modulo 929.
local function gf_add(a, b)
  return (a + b) % GF929_PRIME
end

M.gf_mul  = gf_mul
M.gf_add  = gf_add
M.GF_EXP  = GF_EXP
M.GF_LOG  = GF_LOG

-- ============================================================================
-- Reed-Solomon generator polynomial
-- ============================================================================
--
-- For ECC level L we generate k = 2^(L+1) ECC codewords.  The PDF417
-- generator uses the b = 3 convention: roots are alpha^3, alpha^4, ...,
-- alpha^{k+2}.  (b = 3 differs from QR Code's b = 0 and Data Matrix's b = 1;
-- the b value is part of every Reed-Solomon variant's spec.)
--
--   g(x) = (x - alpha^3)(x - alpha^4) ... (x - alpha^{k+2})
--
-- We build g iteratively by multiplying in each linear factor (x - alpha^j).
-- The result has k + 1 coefficients [g_k, g_{k-1}, ..., g_1, g_0] big-endian
-- with g_k = 1 (monic polynomial).

local function build_generator(ecc_level)
  -- k = 2^(ecc_level+1).  In Lua 5.4 we use the integer left shift to keep
  -- the result as an integer rather than a float (the `^` operator returns a
  -- float even for integer operands).
  local k = 1 << (ecc_level + 1)

  local g = { 1 }                         -- g(x) = 1 initially

  for j = 3, k + 2 do
    local root      = GF_EXP[(j % GF929_ORDER) + 1]   -- alpha^j
    local neg_root  = (GF929_PRIME - root) % GF929_PRIME   -- -alpha^j  (mod 929)

    local new_g = {}
    for i = 1, #g + 1 do new_g[i] = 0 end
    -- Multiply g(x) by (x - alpha^j):  new_g[i] += g[i],
    -- new_g[i+1] += g[i] * (-alpha^j).
    for i = 1, #g do
      new_g[i]     = gf_add(new_g[i],     g[i])
      new_g[i + 1] = gf_add(new_g[i + 1], gf_mul(g[i], neg_root))
    end
    g = new_g
  end
  return g
end

M.build_generator = build_generator

-- ============================================================================
-- Reed-Solomon encoder
-- ============================================================================
--
-- Standard shift-register (LFSR) polynomial long-division.  No interleaving —
-- all data feeds a single RS encoder.  This is simpler than QR Code, which
-- splits data into blocks for burst-error resilience; PDF417 instead relies
-- on the row-cluster structure to spread bursts across multiple codewords.
--
-- For each input data codeword:
--   feedback = (d + ecc[1]) mod 929
--   shift register left
--   for i = 1..k: ecc[i] = (ecc[i] + g[k+1-i+1] * feedback) mod 929
--
-- After processing all data the register holds the k ECC codewords.

local function rs_encode(data, ecc_level)
  local g = build_generator(ecc_level)
  local k = #g - 1                  -- number of ECC codewords

  local ecc = {}
  for i = 1, k do ecc[i] = 0 end

  for _, d in ipairs(data) do
    local feedback = gf_add(d, ecc[1])
    -- Shift the register left by one position.
    for i = 1, k - 1 do
      ecc[i] = ecc[i + 1]
    end
    ecc[k] = 0
    -- Accumulate feedback * generator coefficient into each cell.
    for i = 1, k do
      -- g is big-endian: g[1] is the leading coefficient (degree k).
      -- Position i in the register corresponds to coefficient g[k+1-i+1] = g[k-i+2].
      ecc[i] = gf_add(ecc[i], gf_mul(g[k - i + 2], feedback))
    end
  end

  return ecc
end

M.rs_encode = rs_encode

-- ============================================================================
-- Byte compaction
-- ============================================================================
--
-- Byte compaction handles arbitrary 8-bit data.  6 input bytes pack into
-- 5 codewords by treating the bytes as a 48-bit big-endian unsigned integer
-- and expressing it in base 900:
--
--   n = b0 * 256^5 + b1 * 256^4 + ... + b5
--   codewords = digits(n, base = 900)   -- exactly 5 digits, big-endian
--
-- 48 bits = 281,474,976,710,656 < 900^5 = 590,490,000,000,000, so the
-- conversion is always lossless.
--
-- Remaining 1..5 bytes (the tail) are emitted directly: each byte becomes one
-- codeword in the range 0..255.  Decoders distinguish "full group of 6"
-- from "tail bytes" by the symbol's overall codeword count, so no explicit
-- length terminator is needed inside the byte run.
--
-- Lua 5.4 integers are 64-bit signed, so 48-bit arithmetic fits comfortably
-- without needing a bigint library.  We use integer division (`//`) and
-- modulo (`%`) which both stay in the integer domain when both operands are
-- integers.

local function byte_compact(bytes)
  local cws = { LATCH_BYTE }
  local i = 1
  local n = #bytes

  -- Process complete 6-byte groups.
  while i + 5 <= n do
    -- Pack 6 bytes into a 48-bit integer.  bytes is an array of integers in
    -- the range 0..255.
    local v = 0
    for j = 0, 5 do
      v = v * 256 + bytes[i + j]
    end
    -- Convert v to 5 base-900 digits, most-significant first.
    local group = { 0, 0, 0, 0, 0 }
    for j = 5, 1, -1 do
      group[j] = v % 900
      v = v // 900
    end
    for j = 1, 5 do
      cws[#cws + 1] = group[j]
    end
    i = i + 6
  end

  -- Tail: 1..5 remaining bytes encoded as themselves.
  while i <= n do
    cws[#cws + 1] = bytes[i]
    i = i + 1
  end

  return cws
end

M.byte_compact = byte_compact

-- ============================================================================
-- Auto-selection of ECC level
-- ============================================================================
--
-- These thresholds match the recommendation table from the PDF417 standard:
-- pick a level whose ECC overhead is roughly proportional to the data size
-- so that small symbols stay small and large symbols still recover from
-- realistic damage.

local function auto_ecc_level(data_count)
  if data_count <=  40 then return 2 end
  if data_count <= 160 then return 3 end
  if data_count <= 320 then return 4 end
  if data_count <= 863 then return 5 end
  return 6
end

M.auto_ecc_level = auto_ecc_level

-- ============================================================================
-- Dimension selection
-- ============================================================================
--
-- Heuristic: aim for a roughly square symbol.  c = ceil(sqrt(total / 3)),
-- clamped to [1, 30]; r = ceil(total / c), clamped to [3, 90].  If the first
-- pass produces fewer than 3 rows we recompute c with r = 3 so the symbol
-- is at least the legal minimum height.
--
-- The "/ 3" comes from each PDF417 codeword being 17 modules wide vs. ~3-4
-- modules tall (with the default rowHeight=3); square *visual* aspect
-- requires ~3× more codewords per row than per column.

local function ceil_div(a, b)
  -- math.ceil on a Lua integer division.  Since both a and b are positive
  -- here we can use the integer-only formulation (a + b - 1) // b.
  return (a + b - 1) // b
end

local function clamp(v, lo, hi)
  if v < lo then return lo end
  if v > hi then return hi end
  return v
end

local function choose_dimensions(total)
  local c = clamp(math.ceil(math.sqrt(total / 3)), MIN_COLS, MAX_COLS)
  local r = math.max(MIN_ROWS, ceil_div(total, c))

  if r < MIN_ROWS then
    r = MIN_ROWS
    c = clamp(ceil_div(total, r), MIN_COLS, MAX_COLS)
    r = math.max(MIN_ROWS, ceil_div(total, c))
  end

  r = math.min(MAX_ROWS, r)
  return c, r
end

M.choose_dimensions = choose_dimensions

-- ============================================================================
-- Row indicator computation
-- ============================================================================
--
-- Each row carries two row-indicator codewords (LRI and RRI) that together
-- encode the symbol's metadata so a scanner can recover R, C, and the ECC
-- level even when only a partial row is read:
--
--   R_info = floor((R - 1) / 3)
--   C_info = C - 1
--   L_info = 3*L + (R - 1) % 3
--
-- For row r (0-indexed), cluster = r % 3:
--   Cluster 0:  LRI = 30*floor(r/3) + R_info,  RRI = 30*floor(r/3) + C_info
--   Cluster 1:  LRI = 30*floor(r/3) + L_info,  RRI = 30*floor(r/3) + R_info
--   Cluster 2:  LRI = 30*floor(r/3) + C_info,  RRI = 30*floor(r/3) + L_info
--
-- The "30 * row_group" prefix encodes which group of three rows we are in,
-- letting a scanner reconstruct the row index from any successfully decoded
-- indicator.  R_info / C_info / L_info each occupy 0..29 so the sum stays
-- within a single 0..928 codeword.

local function compute_lri(r, rows, cols, ecc_level)
  local r_info    = (rows - 1) // 3
  local c_info    = cols - 1
  local l_info    = 3 * ecc_level + (rows - 1) % 3
  local row_group = r // 3
  local cluster   = r % 3

  if cluster == 0 then return 30 * row_group + r_info end
  if cluster == 1 then return 30 * row_group + l_info end
  return 30 * row_group + c_info
end

local function compute_rri(r, rows, cols, ecc_level)
  local r_info    = (rows - 1) // 3
  local c_info    = cols - 1
  local l_info    = 3 * ecc_level + (rows - 1) % 3
  local row_group = r // 3
  local cluster   = r % 3

  if cluster == 0 then return 30 * row_group + c_info end
  if cluster == 1 then return 30 * row_group + r_info end
  return 30 * row_group + l_info
end

M.compute_lri = compute_lri
M.compute_rri = compute_rri

-- ============================================================================
-- Pattern expansion (codeword -> modules)
-- ============================================================================
--
-- Every PDF417 codeword in the cluster tables is stored as a packed u32 with
-- 4 bits per element, alternating bar / space starting with a bar:
--
--   bits 31..28 = b1, bits 27..24 = s1, bits 23..20 = b2, bits 19..16 = s2,
--   bits 15..12 = b3, bits 11..8  = s3, bits 7..4   = b4, bits 3..0   = s4
--
-- We expand into a 1-indexed boolean array: true = dark, false = light.
-- The total run length is always 17 modules per codeword.
--
-- Because Lua tables grow dynamically, callers pass in the table they want
-- to append into so we don't allocate per-codeword.

local function expand_pattern(packed, modules)
  local widths = {
    (packed >> 28) & 0xf,   -- b1 (bar)
    (packed >> 24) & 0xf,   -- s1 (space)
    (packed >> 20) & 0xf,   -- b2
    (packed >> 16) & 0xf,   -- s2
    (packed >> 12) & 0xf,   -- b3
    (packed >>  8) & 0xf,   -- s3
    (packed >>  4) & 0xf,   -- b4
    (packed      ) & 0xf,   -- s4
  }
  local dark = true
  for _, w in ipairs(widths) do
    for _ = 1, w do
      modules[#modules + 1] = dark
    end
    dark = not dark
  end
end

-- expand_widths: like expand_pattern but takes an array of widths directly,
-- used for the start (8 elements -> 17 modules) and stop (9 -> 18 modules)
-- patterns where there is no codeword to look up.

local function expand_widths(widths, modules)
  local dark = true
  for _, w in ipairs(widths) do
    for _ = 1, w do
      modules[#modules + 1] = dark
    end
    dark = not dark
  end
end

M.expand_pattern = expand_pattern
M.expand_widths  = expand_widths

-- ============================================================================
-- Rasterisation: codeword sequence -> ModuleGrid
-- ============================================================================
--
-- Each row consists of:
--   start pattern (17) | LRI (17) | data * cols (17 each) | RRI (17) | stop (18)
-- so the total module width is  start(17) + LRI(17) + RRI(17) + stop(18)
-- + 17*cols = 69 + 17*cols.
--
-- Vertically each logical PDF417 row repeats `row_height` times to give the
-- symbol some optical thickness.  row_height = 3 is the standard default.
--
-- The output `modules` table is 1-indexed: modules[r][c] is true if the
-- module at row r, column c is dark.

local function rasterize(sequence, rows, cols, ecc_level, row_height)
  local module_width  = 69 + 17 * cols
  local module_height = rows * row_height

  -- Pre-allocate the full grid as all-false.
  local modules = {}
  for r = 1, module_height do
    local row = {}
    for c = 1, module_width do
      row[c] = false
    end
    modules[r] = row
  end

  -- Pre-compute start and stop module sequences (identical for every row).
  local start_modules = {}
  expand_widths(START_PATTERN, start_modules)
  local stop_modules = {}
  expand_widths(STOP_PATTERN, stop_modules)

  for r = 0, rows - 1 do
    local cluster = r % 3
    local cluster_table = CLUSTER_TABLES[cluster + 1]   -- 1-indexed Lua

    local row_modules = {}

    -- 1. Start pattern (17 modules).
    for _, v in ipairs(start_modules) do
      row_modules[#row_modules + 1] = v
    end

    -- 2. Left Row Indicator (17 modules).
    local lri = compute_lri(r, rows, cols, ecc_level)
    expand_pattern(cluster_table[lri + 1], row_modules)

    -- 3. Data codewords (17 modules each).
    for j = 0, cols - 1 do
      local cw = sequence[r * cols + j + 1]   -- 1-indexed
      expand_pattern(cluster_table[cw + 1], row_modules)
    end

    -- 4. Right Row Indicator (17 modules).
    local rri = compute_rri(r, rows, cols, ecc_level)
    expand_pattern(cluster_table[rri + 1], row_modules)

    -- 5. Stop pattern (18 modules).
    for _, v in ipairs(stop_modules) do
      row_modules[#row_modules + 1] = v
    end

    -- Sanity check: every row must be exactly module_width modules wide.
    -- A mismatch indicates a corrupted cluster table or off-by-one in the
    -- pipeline above.
    if #row_modules ~= module_width then
      error(string.format(
        "PDF417 internal error: row %d has %d modules, expected %d",
        r, #row_modules, module_width))
    end

    -- Write this logical row `row_height` times into the grid.
    local module_row_base = r * row_height
    for h = 1, row_height do
      local mr = module_row_base + h
      local target = modules[mr]
      for c = 1, module_width do
        if row_modules[c] then
          target[c] = true
        end
      end
    end
  end

  return {
    rows         = module_height,
    cols         = module_width,
    modules      = modules,
    module_shape = "square",
  }
end

M.rasterize = rasterize

-- ============================================================================
-- Public API: encode
-- ============================================================================
--
-- M.encode(data, opts) -> grid, err
--
-- Encode arbitrary bytes as a PDF417 symbol and return the ModuleGrid.
--
-- Parameters:
--   data : string | table     bytes to encode.  Strings are treated as raw
--                             bytes (each char becomes one byte 0..255).
--                             Tables must be 1-indexed arrays of integers
--                             0..255.
--   opts : table | nil        encoder options.
--          opts.ecc_level    : 0..8.  Default: auto-selected.
--          opts.columns      : 1..30. Default: auto-selected.
--          opts.row_height   : >= 1.  Default: 3.
--
-- Returns:
--   grid : table              ModuleGrid on success, with fields:
--          rows, cols         module dimensions
--          modules[r][c]      bool, true = dark, 1-indexed
--          module_shape       always "square" for PDF417
--   err  : nil on success
--          table on failure with fields {kind, message, ...}.
--
-- Errors:
--   InvalidECCLevelError      ecc_level out of [0, 8]
--   InvalidDimensionsError    columns out of [1, 30]
--   InputTooLongError         data too large for any valid symbol
--   PDF417Error               other bad input (e.g. non-string non-table)

local function to_byte_array(data)
  if type(data) == "string" then
    local bytes = {}
    for i = 1, #data do
      bytes[i] = string.byte(data, i)
    end
    return bytes, nil
  end

  if type(data) == "table" then
    local bytes = {}
    for i = 1, #data do
      local v = data[i]
      if type(v) ~= "number" or v ~= math.floor(v) or v < 0 or v > 255 then
        return nil, {
          kind    = M.PDF417Error,
          message = string.format(
            "PDF417Error: data[%d] must be an integer in 0..255, got %s",
            i, tostring(v)),
        }
      end
      bytes[i] = v
    end
    return bytes, nil
  end

  return nil, {
    kind    = M.PDF417Error,
    message = "PDF417Error: data must be a string or array of bytes, got " .. type(data),
  }
end

function M.encode(data, opts)
  local bytes, err = to_byte_array(data)
  if err then return nil, err end

  opts = opts or {}

  -- ── Validate ECC level ─────────────────────────────────────────────────
  if opts.ecc_level ~= nil then
    if type(opts.ecc_level) ~= "number"
       or opts.ecc_level ~= math.floor(opts.ecc_level)
       or opts.ecc_level < 0 or opts.ecc_level > 8 then
      return nil, {
        kind    = M.InvalidECCLevelError,
        message = string.format(
          "InvalidECCLevelError: ecc_level must be an integer in 0..8, got %s",
          tostring(opts.ecc_level)),
      }
    end
  end

  -- ── Byte compaction ────────────────────────────────────────────────────
  local data_cwords = byte_compact(bytes)

  -- ── Auto-select ECC level ──────────────────────────────────────────────
  -- The "+1" accounts for the length descriptor we are about to prepend.
  local ecc_level = opts.ecc_level
  if ecc_level == nil then
    ecc_level = auto_ecc_level(#data_cwords + 1)
  end
  local ecc_count = 1 << (ecc_level + 1)

  -- ── Length descriptor ──────────────────────────────────────────────────
  -- The very first codeword of the symbol counts itself, all data codewords,
  -- and all ECC codewords (but NOT the padding).  Decoders use it to find the
  -- boundary between data and pad/ECC.
  local length_desc = 1 + #data_cwords + ecc_count

  -- Full data array for RS encoding: [length_desc] .. data_cwords
  local full_data = { length_desc }
  for _, cw in ipairs(data_cwords) do
    full_data[#full_data + 1] = cw
  end

  -- ── RS ECC ─────────────────────────────────────────────────────────────
  local ecc_cwords = rs_encode(full_data, ecc_level)

  -- ── Choose dimensions ──────────────────────────────────────────────────
  local total = #full_data + #ecc_cwords
  local cols, rows
  if opts.columns ~= nil then
    if type(opts.columns) ~= "number"
       or opts.columns ~= math.floor(opts.columns)
       or opts.columns < MIN_COLS or opts.columns > MAX_COLS then
      return nil, {
        kind    = M.InvalidDimensionsError,
        message = string.format(
          "InvalidDimensionsError: columns must be an integer in 1..30, got %s",
          tostring(opts.columns)),
      }
    end
    cols = opts.columns
    rows = math.max(MIN_ROWS, ceil_div(total, cols))
    if rows > MAX_ROWS then
      return nil, {
        kind    = M.InputTooLongError,
        message = string.format(
          "InputTooLongError: data requires %d rows (max %d) with %d columns",
          rows, MAX_ROWS, cols),
        rows    = rows,
        cols    = cols,
      }
    end
  else
    cols, rows = choose_dimensions(total)
  end

  -- Verify capacity (defence-in-depth against any future heuristic changes).
  if cols * rows < total then
    return nil, {
      kind    = M.InputTooLongError,
      message = string.format(
        "InputTooLongError: cannot fit %d codewords in %dx%d grid",
        total, rows, cols),
      total   = total,
    }
  end

  -- ── Pad to fill grid exactly ───────────────────────────────────────────
  local padding_count = cols * rows - total
  local padded = {}
  for _, cw in ipairs(full_data) do
    padded[#padded + 1] = cw
  end
  for _ = 1, padding_count do
    padded[#padded + 1] = PADDING_CW
  end

  -- Final flat sequence: data + padding then ECC codewords appended.
  local sequence = {}
  for _, cw in ipairs(padded)     do sequence[#sequence + 1] = cw end
  for _, cw in ipairs(ecc_cwords) do sequence[#sequence + 1] = cw end

  -- ── Validate row_height ────────────────────────────────────────────────
  local row_height = opts.row_height
  if row_height == nil then
    row_height = 3
  end
  if type(row_height) ~= "number"
     or row_height ~= math.floor(row_height)
     or row_height < 1 then
    row_height = math.max(1, math.floor(row_height or 3))
  end

  -- ── Rasterise ──────────────────────────────────────────────────────────
  return rasterize(sequence, rows, cols, ecc_level, row_height), nil
end

return M
