-- spec/data_matrix_spec.lua — tests for coding_adventures.data_matrix
--
-- Run from the package root with:
--   cd code/packages/lua/data-matrix && mise exec -- busted spec/
--
-- Test strategy mirrors the Go reference (data_matrix_test.go):
--   1.  Module basics            — VERSION, exported symbols
--   2.  GF(256)/0x12D field      — exp/log tables, multiplication laws
--   3.  ASCII encoding           — single chars, digit pairs, extended bytes
--   4.  Pad codewords            — ISO worked example, scrambling formula
--   5.  Symbol selection         — smallest fit, shape filtering, errors
--   6.  Generator polynomials    — degree, monic, root verification
--   7.  RS block encoding        — length, syndrome=0 property
--   8.  Block interleaving       — single-block passthrough, two-block layout
--   9.  Grid initialisation      — L-finder, timing borders, alignment borders
--  10.  Utah placement           — fills the logical grid
--  11.  Logical→physical mapping — single + multi region offsets
--  12.  Full encode pipeline     — end-to-end, structural invariants
--  13.  Error handling           — InputTooLong, bad shape, non-string

package.path = (
  "./src/?.lua;" ..
  "./src/?/init.lua;" ..
  package.path
)

local dm = require("coding_adventures.data_matrix")

-- ============================================================================
-- Helpers
-- ============================================================================

local function count_dark(grid)
  local n = 0
  for r = 1, grid.rows do
    for c = 1, grid.cols do
      if grid.modules[r][c] then n = n + 1 end
    end
  end
  return n
end

-- Evaluate a polynomial g (big-endian, 1-indexed) at x using Horner's rule
-- over GF(256)/0x12D.  acc = ((acc*x) ⊕ coeff) for each coefficient in order.
local function eval_poly(g, x)
  local acc = 0
  for _, coeff in ipairs(g) do
    acc = dm.gf_mul(acc, x) ~ coeff
  end
  return acc
end

-- ============================================================================
-- 1. Module basics
-- ============================================================================

describe("data_matrix.VERSION", function()
  it("is a string", function()
    assert.is_string(dm.VERSION)
  end)
  it("equals 0.1.0", function()
    assert.equal("0.1.0", dm.VERSION)
  end)
end)

describe("data_matrix exports", function()
  it("exposes encode", function()
    assert.is_function(dm.encode)
  end)
  it("exposes shape constants", function()
    assert.equal("square", dm.SHAPE_SQUARE)
    assert.equal("rectangular", dm.SHAPE_RECTANGULAR)
    assert.equal("any", dm.SHAPE_ANY)
  end)
  it("exposes error kinds", function()
    assert.is_string(dm.InputTooLongError)
    assert.is_string(dm.DataMatrixError)
  end)
end)

-- ============================================================================
-- 2. GF(256)/0x12D field arithmetic
-- ============================================================================

describe("GF(256)/0x12D exp table", function()
  -- Recurrence: α^0 = 1; α^{i+1} = α^i << 1 (mod 0x12D if overflow).
  --   α^7  = 0x80
  --   α^8  = 0x80<<1 = 0x100; XOR 0x12D = 0x2D
  --   α^9  = 0x5A
  --   α^10 = 0xB4
  --   α^255 wraps back to 1.
  it("matches known exponent values", function()
    local cases = {
      {0, 0x01}, {1, 0x02}, {2, 0x04}, {3, 0x08},
      {4, 0x10}, {5, 0x20}, {6, 0x40}, {7, 0x80},
      {8, 0x2D}, {9, 0x5A}, {10, 0xB4},
    }
    for _, c in ipairs(cases) do
      assert.equal(c[2], dm.GF_EXP[c[1] + 1])
    end
    assert.equal(dm.GF_EXP[1], dm.GF_EXP[256])  -- α^255 == α^0
  end)
end)

describe("GF(256)/0x12D log table", function()
  it("is the inverse of exp: log[exp[i]] == i", function()
    for i = 0, 254 do
      local v = dm.GF_EXP[i + 1]
      assert.equal(i, dm.GF_LOG[v + 1])
    end
  end)
end)

describe("gf_mul", function()
  it("absorbs zero", function()
    assert.equal(0, dm.gf_mul(0, 0xFF))
    assert.equal(0, dm.gf_mul(0xFF, 0))
  end)
  it("has identity 1", function()
    for _, v in ipairs({1, 2, 0x80, 0x2D, 0xAA, 0xFF}) do
      assert.equal(v, dm.gf_mul(1, v))
      assert.equal(v, dm.gf_mul(v, 1))
    end
  end)
  it("returns known products", function()
    assert.equal(4,    dm.gf_mul(2, 2))      -- α^1 × α^1 = α^2 = 4
    assert.equal(8,    dm.gf_mul(2, 4))      -- α × α^2 = α^3 = 8
    assert.equal(0x2D, dm.gf_mul(0x80, 2))   -- α^7 × α = α^8 = 0x2D
  end)
  it("is commutative", function()
    local samples = {0, 1, 2, 0x80, 0x2D, 0xAA, 0xFF}
    for _, a in ipairs(samples) do
      for _, b in ipairs(samples) do
        assert.equal(dm.gf_mul(a, b), dm.gf_mul(b, a))
      end
    end
  end)
  it("is distributive: a × (b ⊕ c) == (a×b) ⊕ (a×c)", function()
    local a, b, c = 0x53, 0xCA, 0x72
    assert.equal(dm.gf_mul(a, b ~ c), dm.gf_mul(a, b) ~ dm.gf_mul(a, c))
  end)
  it("α has order 255 (only α^0 and α^255 are 1)", function()
    for i = 1, 254 do
      assert.is_true(dm.GF_EXP[i + 1] ~= 1)
    end
  end)
end)

-- ============================================================================
-- 3. ASCII encoding
-- ============================================================================

describe("encode_ascii single chars", function()
  it("encodes value+1 for printable ASCII", function()
    assert.same({66},  dm.encode_ascii("A"))   -- 65 + 1
    assert.same({98},  dm.encode_ascii("a"))   -- 97 + 1
    assert.same({33},  dm.encode_ascii(" "))   -- 32 + 1
    assert.same({34},  dm.encode_ascii("!"))   -- 33 + 1
    assert.same({91},  dm.encode_ascii("Z"))   -- 90 + 1
    assert.same({1},   dm.encode_ascii("\0"))  -- 0 + 1
  end)
end)

describe("encode_ascii digit pairs", function()
  -- Rule: codeword = 130 + (d1×10 + d2)
  it("packs two digits into one codeword", function()
    assert.same({142}, dm.encode_ascii("12"))   -- 130 + 12
    assert.same({164}, dm.encode_ascii("34"))   -- 130 + 34
    assert.same({130}, dm.encode_ascii("00"))   -- 130 + 0
    assert.same({229}, dm.encode_ascii("99"))   -- 130 + 99
  end)
  it("handles longer digit runs", function()
    assert.same({142, 164},      dm.encode_ascii("1234"))
    assert.same({142, 164, 186, 208}, dm.encode_ascii("12345678"))
  end)
  it("falls back to single chars on odd-length runs", function()
    -- "12" packs (142); "3" → 51+1 = 52 single
    assert.same({142, 52}, dm.encode_ascii("123"))
  end)
  it("does not pair across non-digits", function()
    assert.same({50, 66}, dm.encode_ascii("1A"))   -- '1'→50, 'A'→66
    assert.same({66, 50}, dm.encode_ascii("A1"))
  end)
end)

describe("encode_ascii extended bytes", function()
  -- Rule: UPPER_SHIFT (235) then (value − 127)
  it("emits two codewords for byte 128", function()
    assert.same({235, 1}, dm.encode_ascii(string.char(128)))
  end)
  it("emits two codewords for byte 255", function()
    assert.same({235, 128}, dm.encode_ascii(string.char(255)))
  end)
end)

describe("encode_ascii edge cases", function()
  it("returns empty array for empty input", function()
    assert.same({}, dm.encode_ascii(""))
  end)
end)

-- ============================================================================
-- 4. Pad codewords
-- ============================================================================

describe("pad_codewords ISO worked example", function()
  -- "A" → {66} in a 10×10 symbol (data_cw = 3) → {66, 129, 70}
  -- k=2: 129 (literal); k=3: 129 + (149*3 mod 253) + 1 = 324 → 70 after −254.
  it("matches the ISO 16022 worked example", function()
    assert.same({66, 129, 70}, dm.pad_codewords({66}, 3))
  end)
  it("first pad is always literal 129", function()
    local out = dm.pad_codewords({66, 50}, 5)
    assert.equal(129, out[3])
  end)
  it("is no-op when already at capacity", function()
    assert.same({1, 2, 3}, dm.pad_codewords({1, 2, 3}, 3))
  end)
  it("output length equals data_cw for many symbol sizes", function()
    for _, n in ipairs({3, 5, 8, 12, 18, 22, 30, 36, 44, 62, 86, 144}) do
      local out = dm.pad_codewords({66}, n)
      assert.equal(n, #out)
    end
  end)
  it("scrambled pads are in the byte range 1..254", function()
    local out = dm.pad_codewords({}, 50)
    for _, v in ipairs(out) do
      assert.is_true(v >= 1 and v <= 254)
    end
  end)
end)

-- ============================================================================
-- 5. Symbol selection
-- ============================================================================

describe("select_symbol", function()
  it("picks 10×10 for 1 codeword (smallest square)", function()
    local e, err = dm.select_symbol(1, dm.SHAPE_SQUARE)
    assert.is_nil(err)
    assert.equal(10, e.symbol_rows)
    assert.equal(10, e.symbol_cols)
  end)
  it("respects exact-capacity boundary", function()
    -- 10×10 has data_cw = 3 → still fits 3 codewords
    local e3 = dm.select_symbol(3, dm.SHAPE_SQUARE)
    assert.equal(10, e3.symbol_rows)
    -- 4 codewords forces 12×12 (data_cw = 5)
    local e4 = dm.select_symbol(4, dm.SHAPE_SQUARE)
    assert.equal(12, e4.symbol_rows)
  end)
  it("returns smallest rectangular when shape='rectangular'", function()
    -- Smallest rectangle is 8×18 (data_cw = 5)
    local e = dm.select_symbol(1, dm.SHAPE_RECTANGULAR)
    assert.equal(8,  e.symbol_rows)
    assert.equal(18, e.symbol_cols)
  end)
  it("returns InputTooLongError above 1558 codewords", function()
    local e, err = dm.select_symbol(1559, dm.SHAPE_SQUARE)
    assert.is_nil(e)
    assert.is_table(err)
    assert.equal(dm.InputTooLongError, err.kind)
    assert.equal(1559, err.encoded)
    assert.equal(1558, err.max)
  end)
  it("rejects unknown shapes", function()
    local _, err = dm.select_symbol(1, "diamond")
    assert.is_table(err)
    assert.equal(dm.DataMatrixError, err.kind)
  end)
  it("'any' picks smallest data_cw across both families", function()
    local e = dm.select_symbol(5, dm.SHAPE_ANY)
    assert.is_true(e.data_cw >= 5)
  end)
  it("'square' shape never returns a rectangular size", function()
    local e = dm.select_symbol(1, dm.SHAPE_SQUARE)
    assert.equal(e.symbol_rows, e.symbol_cols)
  end)
end)

-- ============================================================================
-- 6. Generator polynomials (b=1, GF(256)/0x12D)
-- ============================================================================

describe("build_generator", function()
  it("has degree n_ecc (length n_ecc+1) and is monic", function()
    for _, n in ipairs({5, 7, 10, 12, 14, 18, 20, 24, 28}) do
      local g = dm.build_generator(n)
      assert.equal(n + 1, #g)
      assert.equal(1, g[1])         -- leading coefficient must be 1
    end
  end)
  it("has α¹..α^n as roots (b=1 convention)", function()
    for _, n in ipairs({5, 7, 10, 14}) do
      local g = dm.build_generator(n)
      for root = 1, n do
        local x = dm.GF_EXP[root + 1]
        assert.equal(0, eval_poly(g, x))
      end
    end
  end)
end)

-- ============================================================================
-- 7. Reed-Solomon block encoding
-- ============================================================================

describe("rs_encode_block", function()
  it("returns exactly n_ecc bytes", function()
    for _, n in ipairs({5, 7, 10, 14, 18, 20, 24, 28}) do
      local g = dm.build_generator(n)
      local data = {1, 2, 3, 4, 5, 6, 7, 8}
      local ecc = dm.rs_encode_block(data, g)
      assert.equal(n, #ecc)
    end
  end)
  it("produces zero ECC for all-zero data", function()
    local g = dm.build_generator(10)
    local zeros = {}
    for i = 1, 10 do zeros[i] = 0 end
    local ecc = dm.rs_encode_block(zeros, g)
    for i = 1, #ecc do assert.equal(0, ecc[i]) end
  end)
  it("makes the combined data+ECC stream syndrome-free", function()
    -- For "A" in 10×10: data = {66, 129, 70}; n_ecc = 5; combined codeword
    -- C(α^k) must equal 0 for k = 1..5.
    local data = {66, 129, 70}
    local g = dm.build_generator(5)
    local ecc = dm.rs_encode_block(data, g)
    local combined = {}
    for i = 1, #data do combined[i] = data[i] end
    for i = 1, #ecc  do combined[#combined + 1] = ecc[i] end
    for root = 1, 5 do
      local x = dm.GF_EXP[root + 1]
      assert.equal(0, eval_poly(combined, x))
    end
  end)
end)

-- ============================================================================
-- 8. Block interleaving
-- ============================================================================

describe("compute_interleaved", function()
  it("single-block symbol → data || ecc (no actual interleave)", function()
    local entry = dm.select_symbol(1, dm.SHAPE_SQUARE)   -- 10×10
    local padded = dm.pad_codewords(dm.encode_ascii("A"), entry.data_cw)
    local out = dm.compute_interleaved(padded, entry)
    assert.equal(entry.data_cw + entry.ecc_cw, #out)
    -- First data_cw bytes equal the padded data
    for i = 1, entry.data_cw do
      assert.equal(padded[i], out[i])
    end
  end)
  it("two-block 32×32 round-robins data on alternating positions", function()
    local entry = dm.SQUARE_SIZES[10]  -- 32×32 (index 10 in 1-indexed table)
    assert.equal(32, entry.symbol_rows)
    assert.equal(2,  entry.num_blocks)
    local data = {}
    for i = 1, 62 do data[i] = i end
    local out = dm.compute_interleaved(data, entry)
    -- 62 data + 2*18 ECC = 98
    assert.equal(98, #out)
    -- Block 1 has data[1..31], block 2 has data[32..62]; round-robin
    -- gives out[1] = data[1] (blk1), out[2] = data[32] (blk2), …
    for i = 0, 30 do
      assert.equal(data[i + 1],     out[2 * i + 1])
      assert.equal(data[i + 1 + 31], out[2 * i + 2])
    end
  end)
end)

-- ============================================================================
-- 9. Grid initialisation
-- ============================================================================

describe("init_grid", function()
  it("draws solid-dark left column and bottom row (L-finder)", function()
    for i = 1, 5 do
      local entry = dm.SQUARE_SIZES[i]
      local g = dm.init_grid(entry)
      for r = 1, entry.symbol_rows do
        assert.is_true(g[r][1])                -- left column dark
      end
      for c = 1, entry.symbol_cols do
        assert.is_true(g[entry.symbol_rows][c]) -- bottom row dark
      end
    end
  end)
  it("alternates timing on the top row (cols 2..n)", function()
    -- After left column override, col 1 is dark; col 2 should be light
    -- (since 0-idx col 1 is odd → false).  Then alternation continues.
    local entry = dm.SQUARE_SIZES[1]   -- 10×10
    local g = dm.init_grid(entry)
    for c = 2, entry.symbol_cols - 1 do
      local expected = ((c - 1) % 2 == 0)
      assert.equal(expected, g[1][c])
    end
  end)
  it("alternates timing on the right column (rows 1..n-1)", function()
    local entry = dm.SQUARE_SIZES[1]   -- 10×10
    local g = dm.init_grid(entry)
    for r = 1, entry.symbol_rows - 1 do
      local expected = ((r - 1) % 2 == 0)
      assert.equal(expected, g[r][entry.symbol_cols])
    end
  end)
  it("draws alignment borders for multi-region 32×32", function()
    local entry = dm.SQUARE_SIZES[10]   -- 32×32
    local g = dm.init_grid(entry)
    -- 32×32 = 2×2 regions, region_h = region_w = 14.  The first inner
    -- alignment row (Lua row 16) is "solid dark" but the column-direction
    -- alignment border (col 17, the second AB col) WRITES AFTER and may
    -- override row 16, col 17 to its alternating value.  So we sample a
    -- few cells we know are guaranteed dark in the AB row.
    assert.is_true(g[16][2])    -- first cell of AB row, after L-finder
    assert.is_true(g[16][8])    -- mid-region
    assert.is_true(g[16][16])   -- AB col-stripe (also dark)
    -- Right column gets overridden by timing: g[16][32] should be light
    -- because (r-1)=15 is odd.
    assert.is_false(g[16][32])
  end)
end)

-- ============================================================================
-- 10. Utah placement
-- ============================================================================

describe("utah_placement", function()
  it("returns a grid of correct dimensions", function()
    -- Logical grid for 10×10 symbol = 8×8
    local cws = {}
    for i = 1, 100 do cws[i] = i % 256 end   -- enough to fill
    local g = dm.utah_placement(cws, 8, 8)
    assert.equal(8, #g)
    assert.equal(8, #g[1])
  end)
  it("fills every module with a boolean (no nils)", function()
    local cws = {}
    for i = 1, 200 do cws[i] = i % 256 end
    local g = dm.utah_placement(cws, 8, 8)
    for r = 1, 8 do
      for c = 1, 8 do
        assert.is_boolean(g[r][c])
      end
    end
  end)
end)

-- ============================================================================
-- 11. Logical → physical mapping
-- ============================================================================

describe("logical_to_physical", function()
  it("offsets by the outer 1-module border for single-region symbols", function()
    local entry = dm.SQUARE_SIZES[1]   -- 10×10, single 8×8 region
    -- (0,0) logical → (1,1) 0-indexed physical → (2,2) 1-indexed Lua
    local r, c = dm.logical_to_physical(0, 0, entry)
    assert.equal(2, r)
    assert.equal(2, c)
  end)
  it("skips alignment borders in multi-region symbols", function()
    local entry = dm.SQUARE_SIZES[10]   -- 32×32, 2×2 regions of 14×14
    -- Logical (14, 0) is the first row of the SECOND region.  Physical row
    -- = floor(14/14) * (14+2) + (14%14) + 1 = 16 + 0 + 1 = 17 (0-idx) → 18 Lua.
    local r, _ = dm.logical_to_physical(14, 0, entry)
    assert.equal(18, r)
  end)
end)

-- ============================================================================
-- 12. Full encode pipeline
-- ============================================================================

describe("encode end-to-end", function()
  it("encodes 'A' into a 10×10 symbol", function()
    local g, err = dm.encode("A")
    assert.is_nil(err)
    assert.equal(10, g.rows)
    assert.equal(10, g.cols)
    assert.equal("square", g.module_shape)
  end)
  it("returns a properly sized modules table", function()
    local g = dm.encode("A")
    assert.equal(g.rows, #g.modules)
    assert.equal(g.cols, #g.modules[1])
    for r = 1, g.rows do
      for c = 1, g.cols do
        assert.is_boolean(g.modules[r][c])
      end
    end
  end)
  it("L-finder invariants hold (left column + bottom row solid dark)", function()
    local g = dm.encode("HELLO WORLD")
    for r = 1, g.rows do
      assert.is_true(g.modules[r][1])
    end
    for c = 1, g.cols do
      assert.is_true(g.modules[g.rows][c])
    end
  end)
  it("top row and right column timing patterns alternate (after L override)", function()
    local g = dm.encode("HELLO")
    -- top row: col 1 = dark (L), col 2..n-1 alternate per (c-1) even
    for c = 2, g.cols - 1 do
      local expected = ((c - 1) % 2 == 0)
      assert.equal(expected, g.modules[1][c])
    end
    -- right column: row 1..n-1 alternate per (r-1) even
    for r = 1, g.rows - 1 do
      local expected = ((r - 1) % 2 == 0)
      assert.equal(expected, g.modules[r][g.cols])
    end
  end)
  it("scales up the symbol as input grows", function()
    local g_short = dm.encode("A")            -- 1 codeword → 10×10
    local g_long  = dm.encode(string.rep("A", 50))   -- many codewords → larger
    assert.is_true(g_long.rows > g_short.rows)
  end)
  it("is deterministic — same input → same modules", function()
    local g1 = dm.encode("HELLO")
    local g2 = dm.encode("HELLO")
    assert.equal(g1.rows, g2.rows)
    for r = 1, g1.rows do
      for c = 1, g1.cols do
        assert.equal(g1.modules[r][c], g2.modules[r][c])
      end
    end
  end)
  it("encodes long digit-only inputs efficiently via pair packing", function()
    -- 20 digits = 10 codewords → fits 14×14 (data_cw = 8) NO; needs 16×16 (12)
    local g = dm.encode(string.rep("1", 20))
    assert.is_true(g.rows >= 14)
  end)
  it("supports rectangular shape", function()
    local g = dm.encode("HELLO", { shape = dm.SHAPE_RECTANGULAR })
    assert.is_table(g)
    -- All rectangular sizes have rows < cols
    assert.is_true(g.rows < g.cols)
  end)
  it("supports SHAPE_ANY", function()
    local g = dm.encode("HELLO", { shape = dm.SHAPE_ANY })
    assert.is_table(g)
  end)
  it("encodes empty string into the smallest symbol", function()
    local g = dm.encode("")
    assert.is_table(g)
    assert.equal(10, g.rows)
  end)
  it("dark module count is non-trivial (sanity)", function()
    local g = dm.encode("HELLO WORLD")
    local dark = count_dark(g)
    -- At least the L-finder cells (~ rows + cols - 1 cells) plus data
    assert.is_true(dark > g.rows + g.cols)
    -- And not solid dark
    assert.is_true(dark < g.rows * g.cols)
  end)
end)

-- ============================================================================
-- 13. Error handling
-- ============================================================================

describe("encode error handling", function()
  it("returns InputTooLongError for inputs that exceed 144×144", function()
    local huge = string.rep("A", 2000)   -- 2000 ASCII codewords > 1558
    local g, err = dm.encode(huge)
    assert.is_nil(g)
    assert.is_table(err)
    assert.equal(dm.InputTooLongError, err.kind)
  end)
  it("returns DataMatrixError for unknown shape", function()
    local g, err = dm.encode("hi", { shape = "diamond" })
    assert.is_nil(g)
    assert.is_table(err)
    assert.equal(dm.DataMatrixError, err.kind)
  end)
  it("returns DataMatrixError for non-string input", function()
    local g, err = dm.encode(123)
    assert.is_nil(g)
    assert.is_table(err)
    assert.equal(dm.DataMatrixError, err.kind)
  end)
end)
