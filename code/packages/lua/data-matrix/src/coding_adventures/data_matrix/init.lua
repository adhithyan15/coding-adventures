-- coding-adventures-data-matrix
--
-- Data Matrix ECC200 encoder — ISO/IEC 16022:2006 compliant.
--
-- # What is Data Matrix?
--
-- Data Matrix is a two-dimensional matrix barcode invented by RVSI Acuity
-- CiMatrix in 1989 under the name "DataCode" and standardised as ISO/IEC
-- 16022:2006.  The ECC200 variant — using Reed-Solomon over GF(256) — has
-- replaced the older ECC000–ECC140 lineage and is the dominant form worldwide.
--
-- Where Data Matrix is used:
--
--   - PCBs: every board carries a Data Matrix etched on the substrate for
--     traceability through automated assembly lines.
--   - Pharmaceuticals: US FDA DSCSA mandates Data Matrix on unit-dose packages.
--   - Aerospace parts: etched/dot-peened marks survive decades of heat and
--     abrasion that would destroy ink-printed labels.
--   - Medical devices: GS1 DataMatrix on surgical instruments and implants.
--   - USPS registered mail and customs forms.
--
-- # Key differences from QR Code
--
--   - GF(256) uses 0x12D (NOT QR's 0x11D).  Same field size, different field.
--   - Reed-Solomon b=1 convention (roots α¹…αⁿ) instead of QR's b=0 (α⁰…α^{n-1}).
--   - L-shaped finder (left column + bottom row all dark) + clock border.
--   - Diagonal "Utah" placement algorithm — there is NO masking step!
--   - 36 symbol sizes: 30 square (10×10 … 144×144) + 6 rectangular.
--
-- # Encoding pipeline
--
--   input string
--     → ASCII encoding      (chars+1; digit pairs packed into one codeword)
--     → symbol selection    (smallest symbol whose capacity ≥ codeword count)
--     → pad to capacity     (scrambled-pad codewords fill unused slots)
--     → RS blocks + ECC     (GF(256)/0x12D, b=1 convention)
--     → interleave blocks   (data round-robin then ECC round-robin)
--     → grid init           (L-finder + timing border + alignment borders)
--     → Utah placement      (diagonal codeword placement, NO masking)
--     → ModuleGrid          (boolean grid, true = dark module)
--
-- # Relationship to other packages
--
--   coding-adventures-gf256       — generic GF(2^8); we build local 0x12D tables
--   coding-adventures-barcode-2d  — provides ModuleGrid contract + layout()
--
-- # Quick start
--
--   local dm = require("coding_adventures.data_matrix")
--   local grid, err = dm.encode("HELLO")
--   -- grid.rows == grid.cols == 14 (5 ASCII codewords → 14×14 symbol)
--   -- grid.modules[r][c] == true means dark, false means light (1-indexed)
--
-- Lua 5.4 note: bit operations use ~ (XOR) and << / >> (shift).  All public
-- coordinates and tables are 1-indexed (Lua convention).  Internally we
-- mirror the Go reference with 0-indexed math then translate at grid access.

local M = {}

M.VERSION = "0.1.0"

-- ============================================================================
-- Public error kinds
-- ============================================================================
--
-- Encoding can fail in one structured way: the input encodes to more
-- codewords than the largest 144×144 symbol can hold (1558 data codewords).
-- We mirror the qr-code style: return (nil, {kind=..., message=...}).

M.DataMatrixError    = "DataMatrixError"
M.InputTooLongError  = "InputTooLongError"

-- ============================================================================
-- Public option enums
-- ============================================================================
--
-- Symbol shape preference controls which families are considered during the
-- "smallest fitting symbol" search.
--
--   "square"       (default) — only 10×10 … 144×144 squares
--   "rectangular"            — only 8×18 … 16×48 rectangles
--   "any"                    — both, smallest by data capacity (area tiebreak)

M.SHAPE_SQUARE       = "square"
M.SHAPE_RECTANGULAR  = "rectangular"
M.SHAPE_ANY          = "any"

-- ============================================================================
-- GF(256) over polynomial 0x12D — the Data Matrix field
-- ============================================================================
--
-- p(x) = x^8 + x^5 + x^4 + x^2 + x + 1  =  0x12D  =  301
--
-- IMPORTANT: this is DIFFERENT from QR Code's 0x11D polynomial.  Both are
-- degree-8 irreducible polynomials over GF(2), but the resulting fields are
-- non-isomorphic (the multiplication tables differ).  Never mix QR and Data
-- Matrix tables.
--
-- The generator α = 2 (polynomial x) generates all 255 non-zero elements
-- under multiplication, so we can pre-build log/antilog tables:
--
--   GF_EXP[i] = α^i  (i = 0..254; index 255 wraps to α^0 = 1)
--   GF_LOG[v] = k such that α^k = v  (v = 1..255)
--
-- Tables are 1-indexed Lua arrays:
--   GF_EXP[i+1] = α^i
--   GF_LOG[v+1] = log_α(v)        -- (v=0 entry is a placeholder)

local GF_POLY = 0x12D

local GF_EXP = {}
local GF_LOG = {}

do
  -- Build exp/log tables.  Algorithm:
  --   start with val=1 (= α^0); each step left-shift 1 bit (multiply by α=x).
  --   if bit 8 sets (val ≥ 256), XOR with 0x12D to reduce mod the polynomial.
  -- After 255 steps every non-zero element appears exactly once, proving α=2
  -- is primitive for 0x12D.
  local val = 1
  for i = 0, 254 do
    GF_EXP[i + 1] = val           -- GF_EXP[1] = α^0 = 1, GF_EXP[2] = α^1, …
    GF_LOG[val + 1] = i           -- store 0-based exponent
    val = val << 1
    if (val & 0x100) ~= 0 then
      val = val ~ GF_POLY
    end
  end
  -- α^255 = α^0 = 1 (multiplicative order = 255)
  GF_EXP[256] = GF_EXP[1]
  -- GF_LOG[1] (i.e. log of 0) is left as nil / 0; never read for input 0.
  GF_LOG[1] = 0
end

-- gf_mul(a, b): multiply two field elements via log/antilog tables.
--
-- For a,b ≠ 0:  a × b = α^{(log[a] + log[b]) mod 255}
-- If either operand is 0, the product is 0 (the additive identity absorbs
-- multiplication, just like ordinary integers).
--
-- This turns polynomial multiplication + reduction into two table lookups
-- and one addition modulo 255 — effectively O(1) per multiply.
local function gf_mul(a, b)
  if a == 0 or b == 0 then return 0 end
  -- GF_LOG is 1-indexed and stores 0-based exponents; GF_EXP is 1-indexed
  -- with GF_EXP[i+1] = α^i.  ((la + lb) % 255) + 1 gives the GF_EXP index.
  local la = GF_LOG[a + 1]
  local lb = GF_LOG[b + 1]
  return GF_EXP[((la + lb) % 255) + 1]
end

M.gf_mul = gf_mul    -- exported for testing
M.GF_EXP = GF_EXP    -- exported for testing
M.GF_LOG = GF_LOG    -- exported for testing
M.GF_POLY = GF_POLY  -- exported for testing

-- ============================================================================
-- Symbol size table
-- ============================================================================
--
-- An "entry" describes one valid Data Matrix ECC200 symbol size.
--
-- A "data region" is one rectangular interior sub-area.  Small symbols
-- (≤ 26×26) have a single 1×1 region.  Larger symbols subdivide into a
-- regular grid of regions separated by 2-module-wide alignment borders.
--
-- The Utah placement algorithm operates on the "logical data matrix" — the
-- concatenation of all region interiors treated as one flat grid.  After
-- placement we map back to physical symbol coordinates.
--
-- Fields per entry (all integers):
--   symbol_rows, symbol_cols   — total module dimensions including outer border
--   region_rows, region_cols   — count of data regions in each axis
--   region_h,    region_w      — height/width of one data region interior
--   data_cw                    — total data codeword capacity
--   ecc_cw                     — total ECC codeword capacity
--   num_blocks                 — number of interleaved RS blocks
--   ecc_per_block              — ECC bytes per block (same for every block)

-- ── Square symbols (24 sizes) — ISO/IEC 16022:2006 Table 7 ──────────────────
local SQUARE_SIZES = {
  { symbol_rows= 10, symbol_cols= 10, region_rows=1, region_cols=1, region_h= 8, region_w= 8, data_cw=   3, ecc_cw=  5, num_blocks= 1, ecc_per_block= 5 },
  { symbol_rows= 12, symbol_cols= 12, region_rows=1, region_cols=1, region_h=10, region_w=10, data_cw=   5, ecc_cw=  7, num_blocks= 1, ecc_per_block= 7 },
  { symbol_rows= 14, symbol_cols= 14, region_rows=1, region_cols=1, region_h=12, region_w=12, data_cw=   8, ecc_cw= 10, num_blocks= 1, ecc_per_block=10 },
  { symbol_rows= 16, symbol_cols= 16, region_rows=1, region_cols=1, region_h=14, region_w=14, data_cw=  12, ecc_cw= 12, num_blocks= 1, ecc_per_block=12 },
  { symbol_rows= 18, symbol_cols= 18, region_rows=1, region_cols=1, region_h=16, region_w=16, data_cw=  18, ecc_cw= 14, num_blocks= 1, ecc_per_block=14 },
  { symbol_rows= 20, symbol_cols= 20, region_rows=1, region_cols=1, region_h=18, region_w=18, data_cw=  22, ecc_cw= 18, num_blocks= 1, ecc_per_block=18 },
  { symbol_rows= 22, symbol_cols= 22, region_rows=1, region_cols=1, region_h=20, region_w=20, data_cw=  30, ecc_cw= 20, num_blocks= 1, ecc_per_block=20 },
  { symbol_rows= 24, symbol_cols= 24, region_rows=1, region_cols=1, region_h=22, region_w=22, data_cw=  36, ecc_cw= 24, num_blocks= 1, ecc_per_block=24 },
  { symbol_rows= 26, symbol_cols= 26, region_rows=1, region_cols=1, region_h=24, region_w=24, data_cw=  44, ecc_cw= 28, num_blocks= 1, ecc_per_block=28 },
  { symbol_rows= 32, symbol_cols= 32, region_rows=2, region_cols=2, region_h=14, region_w=14, data_cw=  62, ecc_cw= 36, num_blocks= 2, ecc_per_block=18 },
  { symbol_rows= 36, symbol_cols= 36, region_rows=2, region_cols=2, region_h=16, region_w=16, data_cw=  86, ecc_cw= 42, num_blocks= 2, ecc_per_block=21 },
  { symbol_rows= 40, symbol_cols= 40, region_rows=2, region_cols=2, region_h=18, region_w=18, data_cw= 114, ecc_cw= 48, num_blocks= 2, ecc_per_block=24 },
  { symbol_rows= 44, symbol_cols= 44, region_rows=2, region_cols=2, region_h=20, region_w=20, data_cw= 144, ecc_cw= 56, num_blocks= 4, ecc_per_block=14 },
  { symbol_rows= 48, symbol_cols= 48, region_rows=2, region_cols=2, region_h=22, region_w=22, data_cw= 174, ecc_cw= 68, num_blocks= 4, ecc_per_block=17 },
  { symbol_rows= 52, symbol_cols= 52, region_rows=2, region_cols=2, region_h=24, region_w=24, data_cw= 204, ecc_cw= 84, num_blocks= 4, ecc_per_block=21 },
  { symbol_rows= 64, symbol_cols= 64, region_rows=4, region_cols=4, region_h=14, region_w=14, data_cw= 280, ecc_cw=112, num_blocks= 4, ecc_per_block=28 },
  { symbol_rows= 72, symbol_cols= 72, region_rows=4, region_cols=4, region_h=16, region_w=16, data_cw= 368, ecc_cw=144, num_blocks= 4, ecc_per_block=36 },
  { symbol_rows= 80, symbol_cols= 80, region_rows=4, region_cols=4, region_h=18, region_w=18, data_cw= 456, ecc_cw=192, num_blocks= 4, ecc_per_block=48 },
  { symbol_rows= 88, symbol_cols= 88, region_rows=4, region_cols=4, region_h=20, region_w=20, data_cw= 576, ecc_cw=224, num_blocks= 4, ecc_per_block=56 },
  { symbol_rows= 96, symbol_cols= 96, region_rows=4, region_cols=4, region_h=22, region_w=22, data_cw= 696, ecc_cw=272, num_blocks= 4, ecc_per_block=68 },
  { symbol_rows=104, symbol_cols=104, region_rows=4, region_cols=4, region_h=24, region_w=24, data_cw= 816, ecc_cw=336, num_blocks= 6, ecc_per_block=56 },
  { symbol_rows=120, symbol_cols=120, region_rows=6, region_cols=6, region_h=18, region_w=18, data_cw=1050, ecc_cw=408, num_blocks= 6, ecc_per_block=68 },
  { symbol_rows=132, symbol_cols=132, region_rows=6, region_cols=6, region_h=20, region_w=20, data_cw=1304, ecc_cw=496, num_blocks= 8, ecc_per_block=62 },
  { symbol_rows=144, symbol_cols=144, region_rows=6, region_cols=6, region_h=22, region_w=22, data_cw=1558, ecc_cw=620, num_blocks=10, ecc_per_block=62 },
}

-- ── Rectangular symbols (6 sizes) — ISO/IEC 16022:2006 Table 7 ──────────────
local RECT_SIZES = {
  { symbol_rows= 8, symbol_cols=18, region_rows=1, region_cols=1, region_h= 6, region_w=16, data_cw= 5, ecc_cw= 7, num_blocks=1, ecc_per_block= 7 },
  { symbol_rows= 8, symbol_cols=32, region_rows=1, region_cols=2, region_h= 6, region_w=14, data_cw=10, ecc_cw=11, num_blocks=1, ecc_per_block=11 },
  { symbol_rows=12, symbol_cols=26, region_rows=1, region_cols=1, region_h=10, region_w=24, data_cw=16, ecc_cw=14, num_blocks=1, ecc_per_block=14 },
  { symbol_rows=12, symbol_cols=36, region_rows=1, region_cols=2, region_h=10, region_w=16, data_cw=22, ecc_cw=18, num_blocks=1, ecc_per_block=18 },
  { symbol_rows=16, symbol_cols=36, region_rows=1, region_cols=2, region_h=14, region_w=16, data_cw=32, ecc_cw=24, num_blocks=1, ecc_per_block=24 },
  { symbol_rows=16, symbol_cols=48, region_rows=1, region_cols=2, region_h=14, region_w=22, data_cw=49, ecc_cw=28, num_blocks=1, ecc_per_block=28 },
}

M.SQUARE_SIZES = SQUARE_SIZES   -- exported for testing
M.RECT_SIZES   = RECT_SIZES     -- exported for testing

-- ============================================================================
-- Reed-Solomon generator polynomials (b=1, GF(256)/0x12D)
-- ============================================================================
--
-- The RS generator polynomial is g(x) = ∏(x + α^k) for k = 1..n_ecc.
-- Note: roots start at α¹ (b=1), NOT α⁰ as in QR Code.
--
-- We store polynomials big-endian: index 1 is the leading (highest-degree)
-- coefficient.  For nEcc check bytes the polynomial has degree nEcc and
-- length nEcc+1.

local GENERATOR_CACHE = {}

-- build_generator(n_ecc): construct g(x) = (x+α¹)(x+α²)…(x+α^{n_ecc}).
--
-- We start with g = [1] (= 1) and multiply by each linear factor in turn.
-- For factor (x + α^i), the new coefficients are:
--
--   new[j]   ← old[j-1] ⊕ (α^i · old[j])      (shift up + multiply-add)
--
-- where old[j-1] represents the multiplication by x and α^i · old[j] is the
-- constant-term contribution.  Both stored big-endian (index 1 = leading).
local function build_generator(n_ecc)
  if GENERATOR_CACHE[n_ecc] then return GENERATOR_CACHE[n_ecc] end

  local g = { 1 }
  for i = 1, n_ecc do
    local ai = GF_EXP[i + 1]   -- α^i  (GF_EXP is 1-indexed: α^i is at index i+1)
    local nxt = {}
    for j = 1, #g + 1 do nxt[j] = 0 end
    for j = 1, #g do
      nxt[j]     = nxt[j]     ~ g[j]                  -- multiply by x
      nxt[j + 1] = nxt[j + 1] ~ gf_mul(g[j], ai)      -- multiply by α^i
    end
    g = nxt
  end

  GENERATOR_CACHE[n_ecc] = g
  return g
end

M.build_generator = build_generator   -- exported for testing

-- Pre-build all generators required by the symbol size tables.  This avoids
-- per-encode latency for first-use construction.
do
  local seen = {}
  for _, e in ipairs(SQUARE_SIZES) do
    if not seen[e.ecc_per_block] then
      build_generator(e.ecc_per_block)
      seen[e.ecc_per_block] = true
    end
  end
  for _, e in ipairs(RECT_SIZES) do
    if not seen[e.ecc_per_block] then
      build_generator(e.ecc_per_block)
      seen[e.ecc_per_block] = true
    end
  end
end

-- ============================================================================
-- Reed-Solomon block encoding (b=1 convention, GF(256)/0x12D)
-- ============================================================================

-- rs_encode_block(data, generator) → table of nEcc check bytes.
--
-- Computes R(x) = D(x) · x^{nEcc} mod G(x) using a streaming LFSR shift
-- register, the standard systematic RS encoding approach:
--
--   for each data byte d:
--     feedback = d XOR rem[1]
--     shift register left by one position
--     for i = 1..nEcc: rem[i] ^= generator[i+1] · feedback
--
-- generator is big-endian (generator[1] = 1, the leading coefficient).
-- Both data and the returned ECC are 1-indexed Lua arrays of integers 0..255.
local function rs_encode_block(data, generator)
  local n_ecc = #generator - 1
  local rem = {}
  for i = 1, n_ecc do rem[i] = 0 end

  for _, d in ipairs(data) do
    local fb = d ~ rem[1]
    -- Shift register left
    for i = 1, n_ecc - 1 do
      rem[i] = rem[i + 1]
    end
    rem[n_ecc] = 0
    -- XOR each position with generator coefficient × feedback
    if fb ~= 0 then
      for i = 1, n_ecc do
        rem[i] = rem[i] ~ gf_mul(generator[i + 1], fb)
      end
    end
  end
  return rem
end

M.rs_encode_block = rs_encode_block   -- exported for testing

-- ============================================================================
-- ASCII data encoding (ISO/IEC 16022:2006 §5.2.4.1)
-- ============================================================================
--
-- ASCII mode is the default and most common Data Matrix data scheme.
--
-- Rules (applied scanning left to right):
--
--   1. Two consecutive ASCII digits (0x30–0x39) → ONE codeword =
--      130 + (d1 × 10 + d2).  This digit-pair optimisation halves the
--      codeword budget for numeric strings — critical for manufacturing lot
--      codes, serial numbers, and other digit-heavy content.
--
--   2. Single ASCII char (0–127) → codeword = ASCII_value + 1.
--      Examples: 'A' (65) → 66, space (32) → 33.
--
--   3. Extended ASCII (128–255) → TWO codewords: 235 (UPPER_SHIFT) then
--      ASCII_value − 127.  Used for Latin-1 / Windows-1252 content.
--
-- Examples:
--
--   "A"    → {66}                (65 + 1)
--   " "    → {33}                (32 + 1)
--   "12"   → {142}               (130 + 12, digit pair)
--   "1234" → {142, 174}          (two digit pairs)
--   "1A"   → {50, 66}            (no pair: 'A' is not a digit)
--   "00"   → {130}               (130 + 0)
--   "99"   → {229}               (130 + 99)
--   "123"  → {142, 52}           (one pair "12" → 142, then "3" → 52)
--
-- Returns a 1-indexed Lua array of integers in [0, 255].

local function encode_ascii(input)
  local cws = {}
  local n = #input
  local i = 1
  while i <= n do
    local c = string.byte(input, i)
    -- Digit-pair check: current and next byte are both ASCII '0'..'9'
    if c >= 0x30 and c <= 0x39 and i + 1 <= n then
      local c2 = string.byte(input, i + 1)
      if c2 >= 0x30 and c2 <= 0x39 then
        local d1 = c  - 0x30
        local d2 = c2 - 0x30
        cws[#cws + 1] = 130 + d1 * 10 + d2
        i = i + 2
        goto continue
      end
    end
    if c <= 127 then
      cws[#cws + 1] = c + 1
      i = i + 1
    else
      -- Extended ASCII (128–255): UPPER_SHIFT (235) then value − 127
      cws[#cws + 1] = 235
      cws[#cws + 1] = c - 127
      i = i + 1
    end
    ::continue::
  end
  return cws
end

M.encode_ascii = encode_ascii   -- exported for testing

-- ============================================================================
-- Pad codewords (ISO/IEC 16022:2006 §5.2.3)
-- ============================================================================
--
-- After ASCII encoding the message may be shorter than the chosen symbol's
-- data capacity.  We fill the remainder with structured pad codewords:
--
--   1. The FIRST pad is always the literal value 129.
--
--   2. Subsequent pads use a scrambled value derived from their 1-indexed
--      position k within the full codeword stream:
--
--        scrambled = 129 + (149 × k mod 253) + 1
--        if scrambled > 254: scrambled −= 254
--
--      The scrambling prevents long runs of identical bytes from creating
--      degenerate placement patterns under the Utah algorithm.  Long runs
--      of the same byte would cluster related modules together and bias
--      the burst-error-correction structure of the symbol.
--
-- Worked example for "A" (codewords = {66}) in a 10×10 symbol (data_cw = 3):
--
--   k=2: 129                                  (first pad — literal)
--   k=3: 129 + (149×3 mod 253) + 1
--      = 129 + 194 + 1
--      = 324; 324 > 254 → 324 − 254 = 70
--   Result: {66, 129, 70}
--
-- Returns a NEW 1-indexed array of length data_cw.

local function pad_codewords(codewords, data_cw)
  local out = {}
  for i = 1, #codewords do out[i] = codewords[i] end

  if #out >= data_cw then return out end

  -- First pad is always literal 129
  out[#out + 1] = 129
  -- k starts at the 1-indexed position of the NEXT pad byte (i.e. now-current
  -- length + 1, which is one past the literal we just appended).
  local k = #out + 1
  while #out < data_cw do
    local scrambled = 129 + (149 * k) % 253 + 1
    if scrambled > 254 then scrambled = scrambled - 254 end
    out[#out + 1] = scrambled
    k = k + 1
  end
  return out
end

M.pad_codewords = pad_codewords   -- exported for testing

-- ============================================================================
-- Symbol selection
-- ============================================================================
--
-- Given the encoded codeword count, pick the smallest symbol that can hold it.
-- Smallest means: lowest data_cw, with symbol area as the tiebreaker (the
-- denser of two equal-capacity symbols wins).

local function select_symbol(cw_count, shape)
  shape = shape or M.SHAPE_SQUARE

  local candidates = {}
  if shape == M.SHAPE_SQUARE then
    for _, e in ipairs(SQUARE_SIZES) do candidates[#candidates + 1] = e end
  elseif shape == M.SHAPE_RECTANGULAR then
    for _, e in ipairs(RECT_SIZES) do candidates[#candidates + 1] = e end
  elseif shape == M.SHAPE_ANY then
    for _, e in ipairs(SQUARE_SIZES) do candidates[#candidates + 1] = e end
    for _, e in ipairs(RECT_SIZES)   do candidates[#candidates + 1] = e end
  else
    return nil, {
      kind    = M.DataMatrixError,
      message = "DataMatrixError: unknown shape '" .. tostring(shape) .. "' (expected square/rectangular/any)",
    }
  end

  -- Sort ascending by data_cw, with area as tiebreaker.  The list has
  -- ≤ 30 entries and is already mostly sorted, so sort cost is trivial.
  table.sort(candidates, function(a, b)
    if a.data_cw ~= b.data_cw then return a.data_cw < b.data_cw end
    return (a.symbol_rows * a.symbol_cols) < (b.symbol_rows * b.symbol_cols)
  end)

  for _, e in ipairs(candidates) do
    if e.data_cw >= cw_count then return e, nil end
  end

  return nil, {
    kind     = M.InputTooLongError,
    message  = string.format(
      "InputTooLongError: encoded %d codewords, maximum is 1558 (144×144 symbol)",
      cw_count),
    encoded  = cw_count,
    max      = 1558,
  }
end

M.select_symbol = select_symbol   -- exported for testing

-- ============================================================================
-- Block splitting + ECC computation + interleaving
-- ============================================================================
--
-- Most symbols use a single RS block, but larger ones split data across
-- multiple blocks for burst-error resilience.  The split is "ISO style":
--
--   base_len    = data_cw // num_blocks
--   extra       = data_cw % num_blocks
--   blocks 1..extra get base_len + 1 codewords each; remainder get base_len.
--
-- ECC is computed independently per block, then BOTH data and ECC are
-- interleaved round-robin across blocks before placement:
--
--   for pos = 1..max_data_per_block: for blk: append data[blk][pos]
--   for pos = 1..ecc_per_block:      for blk: append ecc[blk][pos]
--
-- Round-robin interleaving spreads bursts: a scratch destroying N contiguous
-- modules damages at most ⌈N/num_blocks⌉ codewords in any single block, which
-- is far more likely to remain within that block's RS correction capacity.

local function compute_interleaved(data, entry)
  local num_blocks    = entry.num_blocks
  local ecc_per_block = entry.ecc_per_block
  local data_cw       = entry.data_cw
  local generator     = build_generator(ecc_per_block)

  local base_len = math.floor(data_cw / num_blocks)
  local extra    = data_cw - base_len * num_blocks   -- = data_cw % num_blocks

  -- Split data into per-block arrays (1-indexed).
  local data_blocks = {}
  local offset = 0
  for b = 1, num_blocks do
    local len = base_len
    if b <= extra then len = len + 1 end
    local blk = {}
    for j = 1, len do blk[j] = data[offset + j] end
    data_blocks[b] = blk
    offset = offset + len
  end

  -- Compute ECC for each block independently.
  local ecc_blocks = {}
  for b = 1, num_blocks do
    ecc_blocks[b] = rs_encode_block(data_blocks[b], generator)
  end

  -- Interleave data round-robin.
  local interleaved = {}
  local max_data_len = 0
  for b = 1, num_blocks do
    if #data_blocks[b] > max_data_len then max_data_len = #data_blocks[b] end
  end
  for pos = 1, max_data_len do
    for b = 1, num_blocks do
      if pos <= #data_blocks[b] then
        interleaved[#interleaved + 1] = data_blocks[b][pos]
      end
    end
  end

  -- Interleave ECC round-robin (each block has the same ecc_per_block bytes).
  for pos = 1, ecc_per_block do
    for b = 1, num_blocks do
      interleaved[#interleaved + 1] = ecc_blocks[b][pos]
    end
  end

  return interleaved
end

M.compute_interleaved = compute_interleaved   -- exported for testing

-- ============================================================================
-- Grid initialisation (L-finder + timing border + alignment borders)
-- ============================================================================
--
-- We allocate an all-light boolean grid sized to the physical symbol, then
-- write the fixed structural elements:
--
--   Top row (row 1):              alternating dark/light starting dark.
--                                 This is the timing clock for the top edge.
--   Right col (col C):            alternating dark/light starting dark.
--                                 Timing clock for the right edge.
--   Left col (col 1):             ALL DARK — vertical leg of the L-finder.
--   Bottom row (row R):           ALL DARK — horizontal leg of the L-finder.
--
-- The asymmetry between the L-bar (solid dark) and the timing clocks
-- (alternating) tells a scanner the orientation of the symbol — all four
-- 90-degree rotations remain unambiguous.
--
-- For multi-region symbols (e.g. 32×32 = 2×2 regions), 2-module-wide
-- alignment borders separate the data regions:
--
--   First AB row:  all dark
--   Second AB row: alternating dark/light starting dark
--
-- Writing order:
--   1. alignment borders (so outer borders override at intersections)
--   2. top timing row + right timing column
--   3. left column (overrides timing at (1,1))
--   4. bottom row (overrides everything — written last)
--
-- All grid coordinates are 1-indexed Lua tables: grid[row][col].

local function init_grid(entry)
  local R, C = entry.symbol_rows, entry.symbol_cols

  -- Allocate all-light grid
  local grid = {}
  for r = 1, R do
    local row = {}
    for c = 1, C do row[c] = false end
    grid[r] = row
  end

  -- ── Alignment borders (multi-region symbols only) ──────────────────────
  -- Written FIRST so that outer-border passes can override at intersections.
  --
  -- For region row index rr ∈ 0..region_rows−2 (zero-based) we have an
  -- alignment border immediately AFTER data region rr+1.  Its physical row
  -- in 0-indexed coords is:
  --   ab_row0_0idx = 1 (outer) + (rr+1) * region_h + rr * 2 (prev ABs)
  -- Convert to 1-indexed Lua by adding 1:
  for rr = 0, entry.region_rows - 2 do
    local ab_row0 = 1 + (rr + 1) * entry.region_h + rr * 2 + 1   -- 1-indexed
    local ab_row1 = ab_row0 + 1
    for c = 1, C do
      grid[ab_row0][c] = true                  -- solid dark
      grid[ab_row1][c] = ((c - 1) % 2 == 0)    -- alternating, starts dark
    end
  end
  for rc = 0, entry.region_cols - 2 do
    local ab_col0 = 1 + (rc + 1) * entry.region_w + rc * 2 + 1   -- 1-indexed
    local ab_col1 = ab_col0 + 1
    for r = 1, R do
      grid[r][ab_col0] = true
      grid[r][ab_col1] = ((r - 1) % 2 == 0)
    end
  end

  -- ── Top row: timing clock — alternating, starts dark ───────────────────
  for c = 1, C do
    grid[1][c] = ((c - 1) % 2 == 0)
  end

  -- ── Right column: timing clock — alternating, starts dark ──────────────
  for r = 1, R do
    grid[r][C] = ((r - 1) % 2 == 0)
  end

  -- ── Left column: L-finder left leg — all dark ──────────────────────────
  for r = 1, R do
    grid[r][1] = true
  end

  -- ── Bottom row: L-finder bottom leg — all dark (written LAST) ──────────
  for c = 1, C do
    grid[R][c] = true
  end

  return grid
end

M.init_grid = init_grid   -- exported for testing

-- ============================================================================
-- Utah placement algorithm
-- ============================================================================
--
-- The Utah placement algorithm is the most distinctive part of Data Matrix
-- encoding.  It was named "Utah" because the 8-module codeword shape vaguely
-- resembles the outline of the US state of Utah — a 3-module-wide rectangle
-- with a notch cut from the upper-left corner.
--
-- The algorithm scans the LOGICAL grid (concatenation of all data region
-- interiors) in a diagonal zigzag.  For each codeword, 8 bits are placed at 8
-- fixed offsets relative to a reference position (row, col).  After each
-- codeword the reference moves diagonally:
--
--   upward leg:   row -= 2, col += 2
--   downward leg: row += 2, col -= 2
--
-- Four special "corner" patterns handle positions where the standard Utah
-- shape would extend outside the grid boundary.
--
-- There is NO masking step after placement.  The diagonal traversal naturally
-- distributes bits across the symbol, so the degenerate cluster patterns that
-- force QR Code to apply a mask cannot occur in Data Matrix.
--
-- Internally we work in 0-indexed coordinates to mirror the Go reference;
-- the apply_wrap and place_* helpers take and return 0-indexed (row, col).
-- Only at the end (logical_to_physical) do we convert back to 1-indexed
-- physical Lua tables.

-- apply_wrap(row, col, n_rows, n_cols) → (row', col') (all 0-indexed).
--
-- When the standard Utah shape extends past the logical grid edge, these
-- four rules from ISO/IEC 16022:2006 Annex F fold the coordinates back into
-- the valid range:
--
--   1. row < 0 AND col == 0       → (1, 3)              top-left singularity
--   2. row < 0 AND col == n_cols  → (0, col − 2)        wrapped past right edge
--   3. row < 0                    → (row + n_rows, col − 4)   top → bottom, left
--   4. col < 0                    → (row − 4, col + n_cols)   left → right, up
local function apply_wrap(row, col, n_rows, n_cols)
  if row < 0 and col == 0 then
    return 1, 3
  end
  if row < 0 and col == n_cols then
    return 0, col - 2
  end
  if row < 0 then
    return row + n_rows, col - 4
  end
  if col < 0 then
    return row - 4, col + n_cols
  end
  return row, col
end

-- place_bits(cw, positions, n_rows, n_cols, grid, used)
--
-- Internal helper: given an 8-element list of {row, col, bit_shift} triples
-- (all 0-indexed coords; bit_shift is 7=MSB, 0=LSB), write the byte's bits
-- into the (logical) grid.  Cells already marked in `used` are skipped.
local function place_bits(cw, positions, n_rows, n_cols, grid, used)
  for _, p in ipairs(positions) do
    local r, c, bit = p[1], p[2], p[3]
    if r >= 0 and r < n_rows and c >= 0 and c < n_cols and not used[r + 1][c + 1] then
      grid[r + 1][c + 1] = ((cw >> bit) & 1) == 1
      used[r + 1][c + 1] = true
    end
  end
end

-- place_utah(cw, row, col, n_rows, n_cols, grid, used)
--
-- The standard Utah 8-module pattern at reference (row, col):
--
--    col: c-2  c-1   c
--   r-2:  .   [1]  [2]
--   r-1: [3]  [4]  [5]
--   r  : [6]  [7]  [8]
--
-- Bits 1..8 of the codeword (1=LSB, 8=MSB).  MSB at (row, col); LSB at
-- (row-2, col-1).  Boundary wrap is applied per cell via apply_wrap.
local function place_utah(cw, row, col, n_rows, n_cols, grid, used)
  -- Build the 8 placements with on-the-fly wrap.
  -- Each entry: {row, col, bit_shift (7=MSB, 0=LSB)}
  local raws = {
    {row,     col,     7},   -- bit 8 (MSB)
    {row,     col - 1, 6},   -- bit 7
    {row,     col - 2, 5},   -- bit 6
    {row - 1, col,     4},   -- bit 5
    {row - 1, col - 1, 3},   -- bit 4
    {row - 1, col - 2, 2},   -- bit 3
    {row - 2, col,     1},   -- bit 2
    {row - 2, col - 1, 0},   -- bit 1 (LSB)
  }
  local wrapped = {}
  for i, p in ipairs(raws) do
    local r, c = apply_wrap(p[1], p[2], n_rows, n_cols)
    wrapped[i] = { r, c, p[3] }
  end
  place_bits(cw, wrapped, n_rows, n_cols, grid, used)
end

-- The four corner patterns each fix-pattern eight specific cells that the
-- standard Utah shape cannot reach when the reference position is near a
-- corner.  Coordinates here are absolute 0-indexed positions in the logical
-- grid.  Source: ISO/IEC 16022:2006 §F.2.

local function place_corner1(cw, n_rows, n_cols, grid, used)
  place_bits(cw, {
    {0,         n_cols - 2, 7},
    {0,         n_cols - 1, 6},
    {1,         0,          5},
    {2,         0,          4},
    {n_rows - 2, 0,         3},
    {n_rows - 1, 0,         2},
    {n_rows - 1, 1,         1},
    {n_rows - 1, 2,         0},
  }, n_rows, n_cols, grid, used)
end

local function place_corner2(cw, n_rows, n_cols, grid, used)
  place_bits(cw, {
    {0,          n_cols - 2, 7},
    {0,          n_cols - 1, 6},
    {1,          n_cols - 1, 5},
    {2,          n_cols - 1, 4},
    {n_rows - 1, 0,          3},
    {n_rows - 1, 1,          2},
    {n_rows - 1, 2,          1},
    {n_rows - 1, 3,          0},
  }, n_rows, n_cols, grid, used)
end

local function place_corner3(cw, n_rows, n_cols, grid, used)
  place_bits(cw, {
    {0,          n_cols - 1, 7},
    {1,          0,          6},
    {2,          0,          5},
    {n_rows - 2, 0,          4},
    {n_rows - 1, 0,          3},
    {n_rows - 1, 1,          2},
    {n_rows - 1, 2,          1},
    {n_rows - 1, 3,          0},
  }, n_rows, n_cols, grid, used)
end

local function place_corner4(cw, n_rows, n_cols, grid, used)
  place_bits(cw, {
    {n_rows - 3, n_cols - 1, 7},
    {n_rows - 2, n_cols - 1, 6},
    {n_rows - 1, n_cols - 3, 5},
    {n_rows - 1, n_cols - 2, 4},
    {n_rows - 1, n_cols - 1, 3},
    {0,          0,          2},
    {1,          0,          1},
    {2,          0,          0},
  }, n_rows, n_cols, grid, used)
end

-- utah_placement(codewords, n_rows, n_cols) → grid (1-indexed bool table)
--
-- Run the diagonal Utah walk over the logical grid.
--
-- The reference position (row, col) starts at (4, 0) and zigzags:
--
--   1. Upward-right leg: place at (row, col), then row -= 2, col += 2 until
--      out of bounds.  Then step to next diagonal start: row += 1, col += 3.
--
--   2. Downward-left leg: place at (row, col), then row += 2, col -= 2 until
--      out of bounds.  Then step to next diagonal start: row += 3, col += 1.
--
-- Between legs, the four corner functions fire when (row, col) matches their
-- triggers (each gated by an n_rows / n_cols modulo condition).
--
-- Termination: when both row ≥ n_rows and col ≥ n_cols, all modules have
-- been visited.  Any unvisited modules at the end receive ISO's
-- "right-and-bottom fill" pattern: dark iff (r + c) mod 2 == 1 (using 0-idx).

local function utah_placement(codewords, n_rows, n_cols)
  local grid = {}
  local used = {}
  for r = 1, n_rows do
    local g_row, u_row = {}, {}
    for c = 1, n_cols do
      g_row[c] = false
      u_row[c] = false
    end
    grid[r] = g_row
    used[r] = u_row
  end

  local cw_idx = 1
  local row, col = 4, 0

  -- place_with(fn): if codewords remain, apply fn with the next codeword.
  local function place_with(fn)
    if cw_idx <= #codewords then
      fn(codewords[cw_idx], n_rows, n_cols, grid, used)
      cw_idx = cw_idx + 1
    end
  end

  while true do
    -- ── Corner special cases ─────────────────────────────────────────────
    -- Corner 1: reference at (n_rows, 0) when n_rows or n_cols divisible by 4.
    if row == n_rows and col == 0 and (n_rows % 4 == 0 or n_cols % 4 == 0) then
      place_with(place_corner1)
    end
    -- Corner 2: reference at (n_rows-2, 0) when n_cols mod 4 ≠ 0.
    if row == n_rows - 2 and col == 0 and (n_cols % 4) ~= 0 then
      place_with(place_corner2)
    end
    -- Corner 3: reference at (n_rows-2, 0) when n_cols mod 8 == 4.
    if row == n_rows - 2 and col == 0 and (n_cols % 8) == 4 then
      place_with(place_corner3)
    end
    -- Corner 4: reference at (n_rows+4, 2) when n_cols mod 8 == 0.
    if row == n_rows + 4 and col == 2 and (n_cols % 8) == 0 then
      place_with(place_corner4)
    end

    -- ── Upward-right diagonal leg ────────────────────────────────────────
    while true do
      if row >= 0 and row < n_rows and col >= 0 and col < n_cols
         and not used[row + 1][col + 1] then
        if cw_idx <= #codewords then
          place_utah(codewords[cw_idx], row, col, n_rows, n_cols, grid, used)
          cw_idx = cw_idx + 1
        end
      end
      row = row - 2
      col = col + 2
      if row < 0 or col >= n_cols then break end
    end

    row = row + 1
    col = col + 3

    -- ── Downward-left diagonal leg ───────────────────────────────────────
    while true do
      if row >= 0 and row < n_rows and col >= 0 and col < n_cols
         and not used[row + 1][col + 1] then
        if cw_idx <= #codewords then
          place_utah(codewords[cw_idx], row, col, n_rows, n_cols, grid, used)
          cw_idx = cw_idx + 1
        end
      end
      row = row + 2
      col = col - 2
      if row >= n_rows or col < 0 then break end
    end

    row = row + 3
    col = col + 1

    if row >= n_rows and col >= n_cols then break end
    if cw_idx > #codewords then break end
  end

  -- ── ISO "right-and-bottom fill" for any unvisited modules ──────────────
  -- Some symbol sizes leave 2–4 corner modules untouched by the diagonal
  -- walk.  ISO/IEC 16022 §10 specifies these become dark iff
  -- (r + c) mod 2 == 1 in 0-indexed coords.
  for r = 0, n_rows - 1 do
    for c = 0, n_cols - 1 do
      if not used[r + 1][c + 1] then
        grid[r + 1][c + 1] = ((r + c) % 2 == 1)
      end
    end
  end

  return grid
end

M.utah_placement = utah_placement   -- exported for testing

-- ============================================================================
-- Logical → Physical coordinate mapping
-- ============================================================================
--
-- The Utah algorithm outputs values for the LOGICAL grid (region interiors
-- concatenated).  The PHYSICAL grid additionally contains:
--   - 1-module outer border (finder + timing) on all four sides
--   - 2-module-wide alignment borders between data regions
--
-- For a symbol with region_rows × region_cols data regions, each of size
-- region_h × region_w, the conversion (0-indexed) is:
--
--   phys_row_0 = floor(r / region_h) × (region_h + 2) + (r mod region_h) + 1
--   phys_col_0 = floor(c / region_w) × (region_w + 2) + (c mod region_w) + 1
--
-- The "+2" accounts for the 2-module alignment border between regions; the
-- "+1" accounts for the 1-module outer border.  For single-region symbols
-- (1×1) this simplifies to phys_row_0 = r+1, phys_col_0 = c+1 — i.e.
-- shift one cell down/right inside the outer border.
--
-- We then add 1 to convert 0-indexed to Lua's 1-indexed grids.

local function logical_to_physical(r0, c0, entry)
  local rh = entry.region_h
  local rw = entry.region_w
  local phys_row_0 = math.floor(r0 / rh) * (rh + 2) + (r0 % rh) + 1
  local phys_col_0 = math.floor(c0 / rw) * (rw + 2) + (c0 % rw) + 1
  return phys_row_0 + 1, phys_col_0 + 1   -- 1-indexed for Lua
end

M.logical_to_physical = logical_to_physical   -- exported for testing

-- ============================================================================
-- Public API: encode
-- ============================================================================
--
-- M.encode(data, opts) -> grid, err
--
-- Encode an arbitrary byte/UTF-8 string into a Data Matrix ECC200 ModuleGrid.
-- The smallest fitting symbol is selected automatically.
--
-- Parameters:
--   data : string                          The data to encode (treated as bytes).
--                                          ASCII (≤ 127) is most efficient;
--                                          extended bytes consume 2 codewords each.
--   opts : table | nil                     Optional encoder options.
--          opts.shape : "square" (default) | "rectangular" | "any"
--
-- Returns:
--   grid : table                           ModuleGrid on success, with fields:
--          rows, cols                      symbol dimensions (modules)
--          modules[r][c]                   1-indexed bool: true = dark
--          module_shape                    always "square" for Data Matrix
--          symbol_rows, symbol_cols        echo of rows / cols
--          data_cw, ecc_cw                 capacities used
--   err  : nil   on success
--        | table on failure with fields:
--          kind     "InputTooLongError" | "DataMatrixError"
--          message  human-readable description
--          encoded  (InputTooLong only) number of codewords produced
--          max      (InputTooLong only) maximum supported (always 1558)

function M.encode(data, opts)
  if type(data) ~= "string" then
    return nil, {
      kind    = M.DataMatrixError,
      message = "DataMatrixError: data must be a string, got " .. type(data),
    }
  end

  opts = opts or {}
  local shape = opts.shape or M.SHAPE_SQUARE

  -- Step 1: ASCII encode the input bytes.
  local cws = encode_ascii(data)

  -- Step 2: Select the smallest fitting symbol.
  local entry, err = select_symbol(#cws, shape)
  if err then return nil, err end

  -- Step 3: Pad to the symbol's data-codeword capacity.
  local padded = pad_codewords(cws, entry.data_cw)

  -- Step 4–5: Compute RS ECC (per block) and interleave round-robin.
  local interleaved = compute_interleaved(padded, entry)

  -- Step 6: Initialise the physical grid (L-finder + timing + alignment borders).
  local phys = init_grid(entry)

  -- Step 7: Run the Utah diagonal placement on the LOGICAL data matrix.
  local n_rows = entry.region_rows * entry.region_h
  local n_cols = entry.region_cols * entry.region_w
  local logical = utah_placement(interleaved, n_rows, n_cols)

  -- Step 8: Map logical → physical and merge into the physical grid.
  for r0 = 0, n_rows - 1 do
    for c0 = 0, n_cols - 1 do
      local pr, pc = logical_to_physical(r0, c0, entry)
      phys[pr][pc] = logical[r0 + 1][c0 + 1]
    end
  end

  -- Step 9: Return the ModuleGrid.  No masking — Data Matrix never masks.
  return {
    rows         = entry.symbol_rows,
    cols         = entry.symbol_cols,
    modules      = phys,
    module_shape = "square",
    symbol_rows  = entry.symbol_rows,
    symbol_cols  = entry.symbol_cols,
    data_cw      = entry.data_cw,
    ecc_cw       = entry.ecc_cw,
  }, nil
end

return M
