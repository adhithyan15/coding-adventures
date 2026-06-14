-- coding-adventures-aztec-code
--
-- Aztec Code encoder — ISO/IEC 24778:2008 compliant.
--
-- Aztec Code was invented by Andrew Longacre Jr. at Welch Allyn in 1995 and
-- published as a patent-free format. Unlike QR Code (which uses three square
-- finder patterns at three corners), Aztec Code places a single **bullseye
-- finder pattern at the center** of the symbol. The scanner finds the center
-- first, then reads outward in a spiral — no large quiet zone is needed.
--
-- ## Where Aztec Code is used today
--
--   - IATA boarding passes — the barcode on every airline boarding pass
--   - Eurostar and Amtrak rail tickets — printed and on-screen tickets
--   - PostNL, Deutsche Post, La Poste — European postal routing
--   - US military ID cards
--
-- ## Symbol variants
--
--   Compact: 1-4 layers,  size = 11 + 4*layers  (15x15 to 27x27)
--   Full:    1-32 layers, size = 15 + 4*layers  (19x19 to 143x143)
--
-- ## Encoding pipeline (v0.1.0 — byte-mode only)
--
--   input string / bytes
--     -> Binary-Shift codewords from Upper mode
--     -> symbol size selection (smallest compact then full at 23% ECC)
--     -> pad to exact codeword count
--     -> GF(256)/0x12D Reed-Solomon ECC (poly 0x12D, b=1 roots alpha^1..alpha^n)
--     -> bit stuffing (insert complement after 4 consecutive identical bits)
--     -> GF(16) mode message (layers + codeword count + 5 or 6 RS nibbles)
--     -> ModuleGrid  (bullseye -> orientation marks -> mode msg -> data spiral)
--
-- ## v0.1.0 simplifications
--
--   1. Byte-mode only — all input encoded via Binary-Shift from Upper mode.
--      Multi-mode (Digit/Upper/Lower/Mixed/Punct) optimization is v0.2.0.
--   2. 8-bit codewords -> GF(256) RS (same polynomial as Data Matrix: 0x12D).
--      GF(16) and GF(32) RS for 4-bit/5-bit codewords are v0.2.0.
--   3. Default ECC = 23%.
--   4. Auto-select compact vs full (force-compact option is v0.2.0).
--
-- ## Reference implementation
--
-- Mirrors the TypeScript reference at
-- code/packages/typescript/aztec-code/src/index.ts.
--
-- ## Lua conventions
--
-- All grid coordinates inside the module are 1-indexed (Lua convention).
-- Bit indices, layer indices, and the Chebyshev distances follow the
-- TypeScript reference (0-indexed in formulas) and are translated at grid
-- access points with explicit `+ 1` adjustments.
--
-- Lua 5.4: bitwise operators (`~`, `<<`, `>>`, `&`, `|`) are native.

local M = {}

M.VERSION = "0.1.0"

-- ============================================================================
-- Error types
-- ============================================================================
--
-- Aztec encoding can fail when the input data is too large to fit in any
-- supported symbol (1..4 compact layers, 1..32 full layers) at the requested
-- ECC level. We signal failure with a structured error table:
--
--   local grid, err = aztec.encode(data)
--   if err then error(err.message) end

M.AztecError         = "AztecError"
M.InputTooLongError  = "InputTooLongError"

-- ============================================================================
-- GF(16) arithmetic — for the mode message Reed-Solomon block
-- ============================================================================
--
-- GF(16) is the finite field with 16 elements, built from the primitive
-- polynomial p(x) = x^4 + x + 1 (binary 10011 = 0x13). Every non-zero element
-- can be written as a power of the primitive element alpha. alpha is a root
-- of p(x), so alpha^4 = alpha + 1.
--
-- The log table maps a field element (1..15) to its discrete log (0..14).
-- The antilog (exponentiation) table maps a log value to its element.
--
-- alpha^0=1, alpha^1=2, alpha^2=4, alpha^3=8,
-- alpha^4=3, alpha^5=6, alpha^6=12, alpha^7=11,
-- alpha^8=5, alpha^9=10, alpha^10=7, alpha^11=14,
-- alpha^12=15, alpha^13=13, alpha^14=9, alpha^15=1 (period=15)
--
-- The TypeScript reference stores these as 0-indexed JavaScript arrays. In
-- Lua we expose them through accessor functions that take/return 0-indexed
-- field/log values, while the underlying tables are still 1-indexed.

-- LOG16: discrete logarithm where LOG16[1+e] = i means alpha^i = e.
-- Index 1 (i.e. e == 0) is undefined; we store -1 to surface bugs early.
local LOG16 = {
  [1]  = -1,  -- log(0) = undefined
  [2]  =  0,  -- log(1) = 0
  [3]  =  1,  -- log(2) = 1
  [4]  =  4,  -- log(3) = 4
  [5]  =  2,  -- log(4) = 2
  [6]  =  8,  -- log(5) = 8
  [7]  =  5,  -- log(6) = 5
  [8]  = 10,  -- log(7) = 10
  [9]  =  3,  -- log(8) = 3
  [10] = 14,  -- log(9) = 14
  [11] =  9,  -- log(10) = 9
  [12] =  7,  -- log(11) = 7
  [13] =  6,  -- log(12) = 6
  [14] = 13,  -- log(13) = 13
  [15] = 11,  -- log(14) = 11
  [16] = 12,  -- log(15) = 12
}

-- ALOG16: antilogarithm where ALOG16[1+i] = alpha^i.
-- Period is 15, so ALOG16[1+15] == ALOG16[1+0] == 1.
local ALOG16 = {1, 2, 4, 8, 3, 6, 12, 11, 5, 10, 7, 14, 15, 13, 9, 1}

-- gf16_mul(a, b): multiply two GF(16) elements.
--
-- Uses log/antilog: a*b = ALOG16[(LOG16[a] + LOG16[b]) mod 15].
-- Returns 0 if either operand is 0.
local function gf16_mul(a, b)
  if a == 0 or b == 0 then return 0 end
  return ALOG16[1 + ((LOG16[1 + a] + LOG16[1 + b]) % 15)]
end

-- build_gf16_generator(n): RS generator polynomial with roots alpha^1..alpha^n.
--
-- Returns big-endian coefficients; result has length n+1, index 1 is the
-- highest-degree coefficient and index n+1 is the constant.
-- The TypeScript reference emits little-endian arrays (next[j+1] gets the
-- previous coefficient via XOR shift), and we mirror its layout here so the
-- gf16_rs_encode loop indices stay identical to the reference.
local function build_gf16_generator(n)
  -- g[1..k] holds the current polynomial in TypeScript-style "little-endian"
  -- (g[1] is the lowest-degree coefficient).
  local g = {1}
  for i = 1, n do
    local ai = ALOG16[1 + (i % 15)]
    -- next[1..k+1] starts as zeros
    local next_g = {}
    for j = 1, #g + 1 do next_g[j] = 0 end
    -- Multiply g by (x + ai). For each existing coefficient g[j]:
    --   the term g[j] * x lands in next_g[j+1]
    --   the term g[j] * ai lands in next_g[j]
    for j = 1, #g do
      next_g[j + 1] = next_g[j + 1] ~ g[j]
      next_g[j]     = next_g[j]     ~ gf16_mul(ai, g[j])
    end
    g = next_g
  end
  return g
end

-- gf16_rs_encode(data, n): compute n GF(16) RS check nibbles for data nibbles.
--
-- Standard LFSR polynomial division with the generator built above.
-- data is 1-indexed; the returned remainder is 1-indexed of length n.
local function gf16_rs_encode(data, n)
  local g = build_gf16_generator(n)
  local rem = {}
  for i = 1, n do rem[i] = 0 end
  for _, byte in ipairs(data) do
    local fb = byte ~ rem[1]
    -- Shift register left and XOR each cell with g[i+1] * fb.
    for i = 1, n - 1 do
      rem[i] = rem[i + 1] ~ gf16_mul(g[i + 1], fb)
    end
    rem[n] = gf16_mul(g[n + 1], fb)
  end
  return rem
end

-- ============================================================================
-- GF(256)/0x12D arithmetic — for 8-bit data codewords
-- ============================================================================
--
-- Aztec Code uses GF(256) with primitive polynomial:
--   p(x) = x^8 + x^5 + x^4 + x^2 + x + 1  =  0x12D
--
-- This is the SAME polynomial as Data Matrix ECC200, but DIFFERENT from
-- QR Code (0x11D). We implement the field locally because the repo's gf256
-- package is hard-coded to 0x11D.
--
-- Generator convention: b = 1, roots alpha^1..alpha^n (the MA02 / Data Matrix
-- style; QR uses b = 0 with roots alpha^0..alpha^(n-1)).

local GF256_POLY = 0x12D

-- EXP_12D[1+i] = alpha^i in GF(256)/0x12D, doubled for fast multiply
-- (i.e. EXP_12D[1+i] == EXP_12D[1+i+255] for 0 <= i < 255).
local EXP_12D = {}
-- LOG_12D[1+e] = discrete log of e in GF(256)/0x12D.
local LOG_12D = {}

-- Initialise tables. The primitive element is alpha = 2.
do
  local x = 1
  for i = 0, 254 do
    EXP_12D[1 + i]       = x
    EXP_12D[1 + i + 255] = x
    LOG_12D[1 + x]       = i
    x = x << 1
    if (x & 0x100) ~= 0 then
      x = x ~ GF256_POLY
    end
    x = x & 0xFF
  end
  -- alpha^255 == alpha^0 == 1 (cyclic group of order 255).
  EXP_12D[1 + 255] = 1
end

-- gf256_mul(a, b): multiply two GF(256)/0x12D elements via log/antilog.
local function gf256_mul(a, b)
  if a == 0 or b == 0 then return 0 end
  -- LOG values are 0..254, so the sum is 0..508 — within the doubled table.
  return EXP_12D[1 + LOG_12D[1 + a] + LOG_12D[1 + b]]
end

-- build_gf256_generator(n): RS generator polynomial with roots alpha^1..alpha^n.
--
-- Returns big-endian coefficients (highest degree first). Note that the
-- TypeScript reference uses big-endian for GF(256) (`next[j] ^= g[j]`)
-- but little-endian for GF(16); we mirror both to keep index parity with
-- the reference loops.
local function build_gf256_generator(n)
  local g = {1}
  for i = 1, n do
    local ai = EXP_12D[1 + i]
    local next_g = {}
    for j = 1, #g + 1 do next_g[j] = 0 end
    for j = 1, #g do
      next_g[j]     = next_g[j]     ~ g[j]
      next_g[j + 1] = next_g[j + 1] ~ gf256_mul(g[j], ai)
    end
    g = next_g
  end
  return g
end

-- gf256_rs_encode(data, n_check): n_check GF(256)/0x12D RS check bytes.
local function gf256_rs_encode(data, n_check)
  local g = build_gf256_generator(n_check)
  local n = #g - 1
  local rem = {}
  for i = 1, n do rem[i] = 0 end
  for _, b in ipairs(data) do
    local fb = b ~ rem[1]
    for i = 1, n - 1 do
      rem[i] = rem[i + 1] ~ gf256_mul(g[i + 1], fb)
    end
    rem[n] = gf256_mul(g[n + 1], fb)
  end
  return rem
end

-- ============================================================================
-- Aztec Code capacity tables
-- ============================================================================
--
-- Derived from ISO/IEC 24778:2008 Table 1.
-- Each entry is a pair {total_bits, max_bytes_8} where:
--   total_bits   = total data+ECC bit positions in the symbol
--   max_bytes_8  = number of 8-bit codeword slots available
--
-- Index 1 in each table is a placeholder (no 0-layer symbol). Compact symbols
-- support 1..4 layers; full symbols support 1..32 layers.

local COMPACT_CAPACITY = {
  {0,    0   }, -- index 1 unused (no 0-layer symbol)
  {72,   9   }, -- 1 layer, 15x15
  {200,  25  }, -- 2 layers, 19x19
  {392,  49  }, -- 3 layers, 23x23
  {648,  81  }, -- 4 layers, 27x27
}

local FULL_CAPACITY = {
  {0,     0   }, -- index 1 unused
  {88,    11  }, --  1 layer
  {216,   27  }, --  2 layers
  {360,   45  }, --  3 layers
  {520,   65  }, --  4 layers
  {696,   87  }, --  5 layers
  {888,   111 }, --  6 layers
  {1096,  137 }, --  7 layers
  {1320,  165 }, --  8 layers
  {1560,  195 }, --  9 layers
  {1816,  227 }, -- 10 layers
  {2088,  261 }, -- 11 layers
  {2376,  297 }, -- 12 layers
  {2680,  335 }, -- 13 layers
  {3000,  375 }, -- 14 layers
  {3336,  417 }, -- 15 layers
  {3688,  461 }, -- 16 layers
  {4056,  507 }, -- 17 layers
  {4440,  555 }, -- 18 layers
  {4840,  605 }, -- 19 layers
  {5256,  657 }, -- 20 layers
  {5688,  711 }, -- 21 layers
  {6136,  767 }, -- 22 layers
  {6600,  825 }, -- 23 layers
  {7080,  885 }, -- 24 layers
  {7576,  947 }, -- 25 layers
  {8088,  1011}, -- 26 layers
  {8616,  1077}, -- 27 layers
  {9160,  1145}, -- 28 layers
  {9720,  1215}, -- 29 layers
  {10296, 1287}, -- 30 layers
  {10888, 1361}, -- 31 layers
  {11496, 1437}, -- 32 layers
}

-- ============================================================================
-- Data encoding — Binary-Shift from Upper mode (v0.1.0 byte-mode path)
-- ============================================================================
--
-- All input is wrapped in a single Binary-Shift block from Upper mode:
--   1. Emit 5 bits = 0b11111 (Binary-Shift escape in Upper mode)
--   2. If len <= 31: 5 bits for length
--      If len > 31:  5 bits = 0b00000, then 11 bits for length
--   3. Each byte as 8 bits, MSB first

-- encode_bytes_as_bits(input): returns an array of 0/1 values, MSB first.
-- input is a sequence of byte values in [0, 255] (1-indexed table).
local function encode_bytes_as_bits(input)
  local bits = {}
  local n = 0

  -- write_bits writes `count` bits from `value`, MSB first, into the bit array
  local function write_bits(value, count)
    for i = count - 1, 0, -1 do
      n = n + 1
      bits[n] = (value >> i) & 1
    end
  end

  local len = #input
  write_bits(31, 5)  -- Binary-Shift escape (0b11111)

  if len <= 31 then
    write_bits(len, 5)
  else
    -- "long length" escape: 5 zero bits then 11-bit length.
    -- 11 bits encodes 0..2047; ISO requires len <= 2047 here.
    write_bits(0, 5)
    write_bits(len, 11)
  end

  for i = 1, len do
    write_bits(input[i], 8)
  end

  return bits
end

-- ============================================================================
-- Symbol size selection
-- ============================================================================
--
-- Try compact symbols (1..4 layers) first, then full (1..32). Add a 20%
-- conservative stuffing overhead so the data still fits after bit stuffing.
--
-- Returns a table:
--   { compact = boolean, layers = integer, data_cw_count = integer,
--     ecc_cw_count = integer, total_bits = integer }
local function select_symbol(data_bit_count, min_ecc_pct)
  local stuffed_bit_count = math.ceil(data_bit_count * 1.2)

  for layers = 1, 4 do
    local cap = COMPACT_CAPACITY[layers + 1]  -- table is 1-indexed; layer L at index L+1
    local total_bytes = cap[2]
    local ecc_cw_count = math.ceil((min_ecc_pct / 100) * total_bytes)
    local data_cw_count = total_bytes - ecc_cw_count
    if data_cw_count > 0 and math.ceil(stuffed_bit_count / 8) <= data_cw_count then
      return {
        compact       = true,
        layers        = layers,
        data_cw_count = data_cw_count,
        ecc_cw_count  = ecc_cw_count,
        total_bits    = cap[1],
      }
    end
  end

  for layers = 1, 32 do
    local cap = FULL_CAPACITY[layers + 1]
    local total_bytes = cap[2]
    local ecc_cw_count = math.ceil((min_ecc_pct / 100) * total_bytes)
    local data_cw_count = total_bytes - ecc_cw_count
    if data_cw_count > 0 and math.ceil(stuffed_bit_count / 8) <= data_cw_count then
      return {
        compact       = false,
        layers        = layers,
        data_cw_count = data_cw_count,
        ecc_cw_count  = ecc_cw_count,
        total_bits    = cap[1],
      }
    end
  end

  return nil, {
    kind    = M.InputTooLongError,
    message = string.format(
      "Input is too long to fit in any Aztec Code symbol (%d bits needed)",
      data_bit_count),
  }
end

-- ============================================================================
-- Padding
-- ============================================================================
--
-- pad_to_bytes(bits, target_bytes): zero-pad the bit stream up to a whole
-- byte boundary, then up to target_bytes * 8 total bits, then truncate.
-- Returns a fresh 1-indexed array of 0/1 values.
local function pad_to_bytes(bits, target_bytes)
  local out = {}
  for i = 1, #bits do out[i] = bits[i] end
  -- pad to next byte boundary
  while (#out % 8) ~= 0 do out[#out + 1] = 0 end
  -- pad to target capacity
  while #out < target_bytes * 8 do out[#out + 1] = 0 end
  -- truncate (should rarely fire — symbol selection guarantees enough room)
  while #out > target_bytes * 8 do out[#out] = nil end
  return out
end

-- ============================================================================
-- Bit stuffing
-- ============================================================================
--
-- After every 4 consecutive identical bits (all 0 or all 1), insert one
-- complement bit. Applies only to the data+ECC bit stream.
--
-- Example:
--   Input:  1 1 1 1 0 0 0 0
--   After 4 ones: insert 0  -> [1,1,1,1,0]
--   After 4 zeros: insert 1 -> [1,1,1,1,0, 0,0,0,1,0]
--
-- The ISO standard mandates this so that no run of 5 identical bits ever
-- appears in the data area, which would otherwise be confused with the
-- structural patterns (orientation marks etc.).
local function stuff_bits(bits)
  local stuffed = {}
  local n = 0
  local run_val = -1
  local run_len = 0

  for _, bit in ipairs(bits) do
    if bit == run_val then
      run_len = run_len + 1
    else
      run_val = bit
      run_len = 1
    end

    n = n + 1
    stuffed[n] = bit

    if run_len == 4 then
      local stuff_bit = 1 - bit  -- complement
      n = n + 1
      stuffed[n] = stuff_bit
      run_val = stuff_bit
      run_len = 1
    end
  end

  return stuffed
end

-- ============================================================================
-- Mode message encoding
-- ============================================================================
--
-- The mode message encodes layer count and data codeword count, protected by
-- GF(16) Reed-Solomon.
--
-- Compact (28 bits = 7 nibbles):
--   m = ((layers - 1) << 6) | (data_cw_count - 1)
--   2 data nibbles + 5 ECC nibbles
--
-- Full (40 bits = 10 nibbles):
--   m = ((layers - 1) << 11) | (data_cw_count - 1)
--   4 data nibbles + 6 ECC nibbles
local function encode_mode_message(compact, layers, data_cw_count)
  local data_nibbles
  local num_ecc

  if compact then
    local m = ((layers - 1) << 6) | (data_cw_count - 1)
    data_nibbles = {
      m & 0xF,
      (m >> 4) & 0xF,
    }
    num_ecc = 5
  else
    local m = ((layers - 1) << 11) | (data_cw_count - 1)
    data_nibbles = {
      m & 0xF,
      (m >> 4) & 0xF,
      (m >> 8) & 0xF,
      (m >> 12) & 0xF,
    }
    num_ecc = 6
  end

  local ecc_nibbles = gf16_rs_encode(data_nibbles, num_ecc)
  -- concatenate data + ecc nibbles
  local all_nibbles = {}
  for _, nib in ipairs(data_nibbles) do all_nibbles[#all_nibbles + 1] = nib end
  for _, nib in ipairs(ecc_nibbles)  do all_nibbles[#all_nibbles + 1] = nib end

  -- expand each 4-bit nibble into 4 MSB-first bits
  local bits = {}
  for _, nibble in ipairs(all_nibbles) do
    for i = 3, 0, -1 do
      bits[#bits + 1] = (nibble >> i) & 1
    end
  end

  return bits
end

-- ============================================================================
-- Grid construction
-- ============================================================================

-- symbol_size(compact, layers): module-units side length.
local function symbol_size(compact, layers)
  if compact then return 11 + 4 * layers end
  return 15 + 4 * layers
end

-- bullseye_radius(compact): Chebyshev radius of the central bullseye.
local function bullseye_radius(compact)
  if compact then return 5 end
  return 7
end

-- make_work_grid(size): create modules and reserved 2D arrays of `size` rows.
-- Both arrays are 1-indexed; reserved[r][c] = true means structural / occupied.
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

-- draw_bullseye(modules, reserved, cx, cy, compact)
--
-- Draws the bullseye finder pattern centred at (cx, cy) (0-indexed coords;
-- caller must add 1 when accessing modules / reserved). The colour of each
-- module at Chebyshev distance d from the centre is:
--   d <= 1     -> DARK   (solid 3x3 inner core)
--   d > 1, d % 2 == 0 -> LIGHT
--   d > 1, d % 2 == 1 -> DARK
local function draw_bullseye(modules, reserved, cx, cy, compact)
  local br = bullseye_radius(compact)
  for row = cy - br, cy + br do
    for col = cx - br, cx + br do
      local d = math.max(math.abs(col - cx), math.abs(row - cy))
      local dark
      if d <= 1 then
        dark = true
      else
        dark = (d % 2) == 1
      end
      modules[row + 1][col + 1]  = dark
      reserved[row + 1][col + 1] = true
    end
  end
end

-- draw_reference_grid(modules, reserved, cx, cy, size)
--
-- For full Aztec symbols only. Adds reference grid lines at rows/cols whose
-- offset from the centre is a multiple of 16. At each grid intersection both
-- coordinates are even multiples; on a single grid line one coordinate is
-- the multiple. Module colour alternates dark/light based on parity from
-- the centre.
local function draw_reference_grid(modules, reserved, cx, cy, size)
  for row = 0, size - 1 do
    for col = 0, size - 1 do
      local on_h = ((cy - row) % 16) == 0
      local on_v = ((cx - col) % 16) == 0
      if on_h or on_v then
        local dark
        if on_h and on_v then
          dark = true
        elseif on_h then
          dark = ((cx - col) % 2) == 0
        else
          dark = ((cy - row) % 2) == 0
        end
        modules[row + 1][col + 1]  = dark
        reserved[row + 1][col + 1] = true
      end
    end
  end
end

-- draw_orientation_and_mode_message
--
-- Places the four orientation marks (the perimeter corners of the
-- mode-message ring at Chebyshev radius bullseyeRadius+1) as DARK, then
-- writes the mode message bits clockwise starting just after the top-left
-- corner.
--
-- Returns an array of remaining {col, row} positions in the ring (1-indexed
-- table values) that the caller will use for the first data bits. This is
-- how the encoder squeezes a few extra data bits into the otherwise
-- mode-message ring on Compact symbols where the mode message has only 28
-- bits but the ring perimeter has more positions.
local function draw_orientation_and_mode_message(
    modules, reserved, cx, cy, compact, mode_message_bits)
  local r = bullseye_radius(compact) + 1

  -- Enumerate non-corner perimeter positions clockwise from (cx-r+1, cy-r),
  -- i.e. immediately right of the top-left orientation corner.
  local non_corner = {}

  -- Top edge (skip both corners)
  for col = cx - r + 1, cx + r - 1 do
    non_corner[#non_corner + 1] = { col = col, row = cy - r }
  end
  -- Right edge (skip both corners)
  for row = cy - r + 1, cy + r - 1 do
    non_corner[#non_corner + 1] = { col = cx + r, row = row }
  end
  -- Bottom edge: right to left (skip both corners)
  for col = cx + r - 1, cx - r + 1, -1 do
    non_corner[#non_corner + 1] = { col = col, row = cy + r }
  end
  -- Left edge: bottom to top (skip both corners)
  for row = cy + r - 1, cy - r + 1, -1 do
    non_corner[#non_corner + 1] = { col = cx - r, row = row }
  end

  -- Place the four orientation mark corners as DARK.
  local corners = {
    { col = cx - r, row = cy - r },
    { col = cx + r, row = cy - r },
    { col = cx + r, row = cy + r },
    { col = cx - r, row = cy + r },
  }
  for _, p in ipairs(corners) do
    modules[p.row + 1][p.col + 1]  = true
    reserved[p.row + 1][p.col + 1] = true
  end

  -- Place mode message bits along the non-corner ring positions.
  local placed = math.min(#mode_message_bits, #non_corner)
  for i = 1, placed do
    local p = non_corner[i]
    modules[p.row + 1][p.col + 1]  = mode_message_bits[i] == 1
    reserved[p.row + 1][p.col + 1] = true
  end

  -- Remaining positions (if any) carry the first data bits.
  local remaining = {}
  for i = placed + 1, #non_corner do
    remaining[#remaining + 1] = non_corner[i]
  end
  return remaining
end

-- ============================================================================
-- Data layer spiral placement
-- ============================================================================
--
-- Bits are placed in a clockwise spiral starting from the innermost data
-- layer. Each layer band is 2 modules wide. For every position we write the
-- outer cell first then the inner cell. Reserved cells (structural pattern
-- modules and reference grid) are skipped silently.
--
-- For compact symbols: d_inner of first layer = bullseye_radius + 2 = 7
-- For full    symbols: d_inner of first layer = bullseye_radius + 2 = 9
local function place_data_bits(
    modules, reserved, bits, cx, cy, compact, layers,
    mode_ring_remaining_positions)
  local size = #modules
  local bit_index = 1

  -- place_bit writes the next data bit into the cell at (col, row), unless
  -- the cell is reserved (structural). 0-indexed coordinates; out-of-bounds
  -- cells are silently dropped — the bit is still consumed only if we wrote.
  local function place_bit(col, row)
    if row < 0 or row >= size or col < 0 or col >= size then return end
    if reserved[row + 1][col + 1] then return end
    modules[row + 1][col + 1] = (bits[bit_index] or 0) == 1
    bit_index = bit_index + 1
  end

  -- First, fill leftover mode-ring positions (compact symbols only have
  -- non-zero leftovers in this branch; full symbols typically have 0 here).
  for _, p in ipairs(mode_ring_remaining_positions) do
    modules[p.row + 1][p.col + 1] = (bits[bit_index] or 0) == 1
    bit_index = bit_index + 1
  end

  -- Spiral through data layers, outer-then-inner per cell.
  local br = bullseye_radius(compact)
  local d_start = br + 2  -- mode ring at br+1, first data layer inner radius at br+2

  for L = 0, layers - 1 do
    local d_i = d_start + 2 * L  -- inner radius of this layer
    local d_o = d_i + 1          -- outer radius

    -- Top edge: left to right
    for col = cx - d_i + 1, cx + d_i do
      place_bit(col, cy - d_o)
      place_bit(col, cy - d_i)
    end
    -- Right edge: top to bottom
    for row = cy - d_i + 1, cy + d_i do
      place_bit(cx + d_o, row)
      place_bit(cx + d_i, row)
    end
    -- Bottom edge: right to left
    for col = cx + d_i, cx - d_i + 1, -1 do
      place_bit(col, cy + d_o)
      place_bit(col, cy + d_i)
    end
    -- Left edge: bottom to top
    for row = cy + d_i, cy - d_i + 1, -1 do
      place_bit(cx - d_o, row)
      place_bit(cx - d_i, row)
    end
  end
end

-- ============================================================================
-- Public API
-- ============================================================================

-- to_byte_array(data): accept a Lua string or an existing byte-table and
-- normalise to a 1-indexed array of integer byte values in [0, 255].
local function to_byte_array(data)
  if type(data) == "string" then
    local bytes = {}
    for i = 1, #data do
      bytes[i] = string.byte(data, i)
    end
    return bytes
  elseif type(data) == "table" then
    -- shallow copy + validate
    local bytes = {}
    for i, v in ipairs(data) do
      if type(v) ~= "number" or v < 0 or v > 255 or math.floor(v) ~= v then
        error(string.format(
          "AztecError: byte at index %d is not an integer in [0,255]", i))
      end
      bytes[i] = v
    end
    return bytes
  else
    error("AztecError: data must be a string or array of byte values")
  end
end

-- encode(data, options) -> grid, err
--
-- Encodes data as an Aztec Code symbol.
--
-- Parameters:
--   data    — string (UTF-8 / raw bytes) or array of byte values [0..255]
--   options — optional table:
--     min_ecc_percent — number (default 23, range 5..95)
--
-- Returns:
--   On success: grid, nil where grid is a ModuleGrid table with fields:
--     rows         — symbol side length (modules)
--     cols         — same as rows
--     modules      — 1-indexed boolean[rows][cols]; true = dark
--     module_shape — "square"
--     compact      — boolean: true if compact symbol, false if full
--     layers       — number of data layers (1..4 compact / 1..32 full)
--   On failure: nil, err where err is { kind, message }.
function M.encode(data, options)
  options = options or {}
  local min_ecc_pct = options.min_ecc_percent or 23

  -- Reject obviously bogus ECC values upfront.
  if type(min_ecc_pct) ~= "number" or min_ecc_pct < 5 or min_ecc_pct > 95 then
    return nil, {
      kind    = M.AztecError,
      message = string.format(
        "min_ecc_percent must be a number in [5, 95], got %s",
        tostring(min_ecc_pct)),
    }
  end

  -- Check input size before allocating the byte table: to_byte_array builds a
  -- Lua table entry per byte, so checking first avoids a large allocation for
  -- inputs that will be rejected anyway.
  if type(data) == "string" and #data > 2047 then
    return nil, {
      kind    = M.InputTooLongError,
      message = string.format(
        "Input length %d exceeds Aztec Binary-Shift max (2047 bytes).",
        #data),
    }
  end

  local input = to_byte_array(data)

  -- Re-check after conversion in case caller passed a pre-built byte table.
  if #input > 2047 then
    return nil, {
      kind    = M.InputTooLongError,
      message = string.format(
        "Input length %d exceeds Aztec Binary-Shift max (2047 bytes).",
        #input),
    }
  end

  -- Step 1: encode data
  local data_bits = encode_bytes_as_bits(input)

  -- Step 2: select symbol
  local spec, err = select_symbol(#data_bits, min_ecc_pct)
  if err then return nil, err end

  local compact       = spec.compact
  local layers        = spec.layers
  local data_cw_count = spec.data_cw_count
  local ecc_cw_count  = spec.ecc_cw_count

  -- Step 3: pad to data_cw_count bytes
  local padded_bits = pad_to_bytes(data_bits, data_cw_count)

  local data_bytes = {}
  for i = 0, data_cw_count - 1 do
    local byte = 0
    for b = 0, 7 do
      byte = (byte << 1) | (padded_bits[i * 8 + b + 1] or 0)
    end
    -- All-zero codeword avoidance: if the LAST codeword is 0x00, replace it
    -- with 0xFF. The TS reference notes this comes from the spec to avoid
    -- pathological RS inputs. Same-position rule kept for parity.
    if byte == 0 and i == data_cw_count - 1 then
      byte = 0xFF
    end
    data_bytes[i + 1] = byte
  end

  -- Step 4: compute RS ECC
  local ecc_bytes = gf256_rs_encode(data_bytes, ecc_cw_count)

  -- Step 5: build raw bit stream (data || ecc, big-endian per byte) and stuff
  local all_bytes = {}
  for _, b in ipairs(data_bytes) do all_bytes[#all_bytes + 1] = b end
  for _, b in ipairs(ecc_bytes)  do all_bytes[#all_bytes + 1] = b end

  local raw_bits = {}
  for _, byte in ipairs(all_bytes) do
    for i = 7, 0, -1 do
      raw_bits[#raw_bits + 1] = (byte >> i) & 1
    end
  end
  local stuffed_bits = stuff_bits(raw_bits)

  -- Step 6: GF(16) mode message
  local mode_msg = encode_mode_message(compact, layers, data_cw_count)

  -- Step 7: initialise grid
  local size = symbol_size(compact, layers)
  -- Centre coords (0-indexed); we use the integer floor consistently with
  -- the TypeScript reference (which calls Math.floor(size / 2)).
  local cx = size // 2
  local cy = size // 2

  local modules, reserved = make_work_grid(size)

  -- Reference grid first (full only), then bullseye overwrites any overlap.
  if not compact then
    draw_reference_grid(modules, reserved, cx, cy, size)
  end
  draw_bullseye(modules, reserved, cx, cy, compact)

  local mode_ring_remaining = draw_orientation_and_mode_message(
    modules, reserved, cx, cy, compact, mode_msg)

  -- Step 8: place data spiral
  place_data_bits(
    modules, reserved, stuffed_bits,
    cx, cy, compact, layers, mode_ring_remaining)

  -- Build the public grid table. We deep-copy modules so callers cannot
  -- accidentally mutate the encoder's working buffer.
  local out_modules = {}
  for r = 1, size do
    local row = modules[r]
    local copy = {}
    for c = 1, size do copy[c] = row[c] end
    out_modules[r] = copy
  end

  return {
    rows         = size,
    cols         = size,
    modules      = out_modules,
    module_shape = "square",
    compact      = compact,
    layers       = layers,
  }, nil
end

return M
