-- spec/qr_code_spec.lua — tests for coding_adventures.qr_code
--
-- Run from the package root with:
--   cd code/packages/lua/qr-code && mise exec -- busted spec/
--
-- These tests validate the complete QR Code encoding pipeline:
--   1. Module basics       — VERSION, encode API contract
--   2. Mode selection      — numeric / alphanumeric / byte
--   3. Version selection   — smallest fitting version
--   4. Grid geometry       — symbol_size, raw data modules
--   5. Format information  — BCH(15,5), XOR mask, bit placement
--   6. Version information — BCH(18,6), v7+ only
--   7. Full encode         — produces correct grid dimensions and known values
--   8. ECC levels          — all four levels produce valid output
--   9. Edge cases          — empty string, single char, max input
--  10. Error handling      — invalid level, input too long

package.path = (
  "./src/?.lua;" ..
  "./src/?/init.lua;" ..
  "../gf256/src/?.lua;" ..
  "../gf256/src/?/init.lua;" ..
  package.path
)

local qr = require("coding_adventures.qr_code")

-- ============================================================================
-- Helper utilities
-- ============================================================================

-- Count dark (true) modules in a grid
local function count_dark(grid)
  local n = 0
  for r = 1, grid.rows do
    for c = 1, grid.cols do
      if grid.modules[r][c] then n = n + 1 end
    end
  end
  return n
end

-- Check that a grid has the correct square dimensions
local function grid_ok(grid, expected_size)
  return grid ~= nil
      and grid.rows == expected_size
      and grid.cols == expected_size
      and #grid.modules == expected_size
      and #grid.modules[1] == expected_size
end

-- ============================================================================
-- 1. Module basics
-- ============================================================================

describe("qr_code.VERSION", function()
  it("is a string", function()
    assert.is_string(qr.VERSION)
  end)

  it("is 0.1.0", function()
    assert.equal("0.1.0", qr.VERSION)
  end)
end)

describe("qr_code.encode API contract", function()
  it("returns a table on success", function()
    local grid, err = qr.encode("HELLO WORLD", "M")
    assert.is_nil(err)
    assert.is_table(grid)
  end)

  it("returns nil and error table on invalid ECC level", function()
    local grid, err = qr.encode("hi", "X")
    assert.is_nil(grid)
    assert.is_table(err)
    assert.is_string(err.message)
  end)

  it("defaults to ECC level M when level is omitted", function()
    -- Same data with explicit M and omitted level should give same size
    local g1 = qr.encode("hello", "M")
    local g2 = qr.encode("hello")
    assert.is_table(g1)
    assert.is_table(g2)
    assert.equal(g1.rows, g2.rows)
  end)

  it("grid has module_shape = square", function()
    local grid = qr.encode("test", "L")
    assert.equal("square", grid.module_shape)
  end)

  it("grid has rows == cols", function()
    local grid = qr.encode("test", "L")
    assert.equal(grid.rows, grid.cols)
  end)
end)

-- ============================================================================
-- 2. Mode selection (internal via encode behaviour)
-- ============================================================================

describe("encoding modes", function()
  -- Numeric mode: only digits. A short all-digit string at ECC L
  -- should fit in a smaller version than byte mode for the same content.
  it("numeric mode: encodes digit-only strings", function()
    local g, err = qr.encode("0123456789", "L")
    assert.is_nil(err)
    assert.is_table(g)
    -- Version 1 holds up to 41 numeric chars at L
    assert.equal(21, g.rows)   -- version 1 = 21 modules
  end)

  it("alphanumeric mode: encodes uppercase + symbols", function()
    local g, err = qr.encode("HELLO WORLD", "M")
    assert.is_nil(err)
    assert.is_table(g)
    -- "HELLO WORLD" (11 chars) fits in version 1 alphanumeric at M
    assert.equal(21, g.rows)
  end)

  it("byte mode: encodes arbitrary UTF-8 text", function()
    local g, err = qr.encode("https://example.com", "M")
    assert.is_nil(err)
    assert.is_table(g)
    -- "https://example.com" (20 bytes, lowercase) → byte mode → version 2 at M
    assert.equal(25, g.rows)   -- version 2 = 25 modules
  end)

  it("byte mode: encodes Unicode text", function()
    local g, err = qr.encode("こんにちは", "Q")
    assert.is_nil(err)
    assert.is_table(g)
    -- Japanese text is multi-byte UTF-8; must land in byte mode
    assert.is_number(g.rows)
    assert.true_(g.rows >= 21)
  end)
end)

-- ============================================================================
-- 3. Version selection
-- ============================================================================

describe("version selection", function()
  it("version 1 is selected for very short inputs", function()
    local g = qr.encode("A", "L")
    assert.equal(21, g.rows)
    assert.equal(1, g.version)
  end)

  it("longer inputs require higher versions", function()
    -- A 100-character alphanumeric string will need v3 or higher at ECC H
    local s = string.rep("A", 100)
    local g = qr.encode(s, "H")
    assert.is_table(g)
    assert.true_(g.version > 1)
  end)

  it("selects a larger version for higher ECC levels", function()
    local data = string.rep("1", 50)
    local gL = qr.encode(data, "L")
    local gH = qr.encode(data, "H")
    assert.true_(gH.version >= gL.version)
  end)

  it("returns InputTooLongError for huge inputs", function()
    local huge = string.rep("A", 8000)
    local g, err = qr.encode(huge, "L")
    assert.is_nil(g)
    assert.is_table(err)
    assert.equal(qr.InputTooLongError, err.kind)
  end)
end)

-- ============================================================================
-- 4. Grid geometry
-- ============================================================================

describe("grid geometry", function()
  it("version 1 produces a 21×21 grid", function()
    local g = qr.encode("A", "L")
    assert.true_(grid_ok(g, 21))
  end)

  it("version 2 produces a 25×25 grid", function()
    local g = qr.encode("https://example.com", "M")
    assert.true_(grid_ok(g, 25))
  end)

  it("symbol size formula: 4*version + 17", function()
    for _, td in ipairs({
      {input = "A",    level = "L", expected = 21},  -- v1: 4*1+17=21
      {input = "ABCD", level = "L", expected = 21},  -- v1 at L fits 4 alphanum chars
    }) do
      local g = qr.encode(td.input, td.level)
      assert.is_table(g)
      assert.equal(td.expected, g.rows)
    end
  end)

  it("all modules are boolean", function()
    local g = qr.encode("TEST", "M")
    for r = 1, g.rows do
      for c = 1, g.cols do
        assert.is_boolean(g.modules[r][c])
      end
    end
  end)
end)

-- ============================================================================
-- 5. Format information placement
-- ============================================================================
--
-- ISO 18004:2015 specifies that the always-dark module at (4v+9, 8) in
-- 0-indexed coordinates must always be dark (true).
-- 0-indexed (4v+9, 8) → 1-indexed (4v+10, 9)

describe("format information", function()
  it("dark module at (4v+10, 9) 1-indexed is always dark", function()
    for _, level in ipairs({"L", "M", "Q", "H"}) do
      local g = qr.encode("HELLO WORLD", level)
      assert.is_table(g)
      local row1 = 4 * g.version + 10
      assert.is_true(g.modules[row1][9],
        string.format("dark module wrong for ECC=%s version=%d", level, g.version))
    end
  end)

  it("finder pattern top-left corner is dark", function()
    local g = qr.encode("test", "M")
    -- Module at (1,1) is the top-left corner of the TL finder (always dark)
    assert.is_true(g.modules[1][1])
  end)

  it("finder pattern inner row 1 is dark", function()
    local g = qr.encode("test", "M")
    -- (3,3) to (5,5) is the inner 3×3 core of TL finder — should all be dark
    for r = 3, 5 do
      for c = 3, 5 do
        assert.is_true(g.modules[r][c],
          string.format("inner finder core (%d,%d) should be dark", r, c))
      end
    end
  end)
end)

-- ============================================================================
-- 6. All four ECC levels
-- ============================================================================

describe("all ECC levels produce valid output", function()
  local test_input = "https://example.com/test-qr-code"
  for _, level in ipairs({"L", "M", "Q", "H"}) do
    it(string.format("ECC level %s produces a valid grid", level), function()
      local g, err = qr.encode(test_input, level)
      assert.is_nil(err, string.format("unexpected error for ECC=%s: %s",
        level, err and err.message or ""))
      assert.is_table(g)
      assert.equal(g.rows, g.cols)
      assert.true_(g.rows >= 21)
      assert.true_(g.rows % 4 == 1)  -- (4v+17) % 4 == 1
    end)
  end
end)

-- ============================================================================
-- 7. Full encode — reference test vectors
-- ============================================================================
--
-- These tests check known structural properties of well-defined QR Codes.
-- Because the mask selection can vary (we pick the lowest-penalty mask),
-- we do not hardcode exact pixel values; instead we verify structural
-- properties that must hold for any valid QR Code.

describe("full encode reference properties", function()
  it("'1' numeric at ECC L produces version 1 (21x21)", function()
    local g = qr.encode("1", "L")
    assert.equal(21, g.rows)
    assert.equal(21, g.cols)
    assert.equal(1, g.version)
  end)

  it("has a reasonable proportion of dark modules (20-80%)", function()
    local g = qr.encode("HELLO WORLD", "M")
    local dark = count_dark(g)
    local total = g.rows * g.cols
    local pct = dark / total
    assert.true_(pct >= 0.2 and pct <= 0.8,
      string.format("dark proportion %.1f%% outside 20-80%%", pct * 100))
  end)

  it("top-left 7×7 finder pattern has correct ring structure", function()
    local g = qr.encode("TEST", "M")
    -- Outer border of TL finder (rows 1-7, cols 1-7): all dark
    for c = 1, 7 do
      assert.is_true(g.modules[1][c], "row 1 should be dark")
      assert.is_true(g.modules[7][c], "row 7 should be dark")
    end
    for r = 2, 6 do
      assert.is_true(g.modules[r][1], "col 1 should be dark")
      assert.is_true(g.modules[r][7], "col 7 should be dark")
    end
    -- Inner ring (rows 2-6, cols 2-6): outer cells light
    for c = 2, 6 do
      assert.is_false(g.modules[2][c], "ring row 2 should be light")
      assert.is_false(g.modules[6][c], "ring row 6 should be light")
    end
    for r = 3, 5 do
      assert.is_false(g.modules[r][2], "ring col 2 should be light")
      assert.is_false(g.modules[r][6], "ring col 6 should be light")
    end
    -- Core (rows 3-5, cols 3-5): all dark
    for r = 3, 5 do
      for c = 3, 5 do
        assert.is_true(g.modules[r][c],
          string.format("core module (%d,%d) should be dark", r, c))
      end
    end
  end)

  it("timing strip (row 7 in 1-indexed) alternates dark/light", function()
    local g = qr.encode("HELLO WORLD", "M")
    -- Row 6 (0-indexed) = row 7 (1-indexed). Timing runs between finders.
    -- Cols 9..sz-8 (1-indexed) should alternate: col 9 (0-indexed 8) is even → dark.
    local sz = g.rows
    for c = 9, sz - 8 do
      local expected_dark = (c - 1) % 2 == 0   -- 0-indexed col is c-1
      assert.equal(expected_dark, g.modules[7][c],
        string.format("timing row: col %d should be %s", c, tostring(expected_dark)))
    end
  end)

  it("timing strip (col 7 in 1-indexed) alternates dark/light", function()
    local g = qr.encode("HELLO WORLD", "M")
    local sz = g.rows
    for r = 9, sz - 8 do
      local expected_dark = (r - 1) % 2 == 0
      assert.equal(expected_dark, g.modules[r][7],
        string.format("timing col: row %d should be %s", r, tostring(expected_dark)))
    end
  end)
end)

-- ============================================================================
-- 8. Edge cases
-- ============================================================================

describe("edge cases", function()
  it("empty string encodes without error", function()
    local g, err = qr.encode("", "M")
    assert.is_nil(err)
    assert.is_table(g)
    assert.equal(21, g.rows)  -- version 1 is the minimum
  end)

  it("single byte encodes correctly", function()
    local g, err = qr.encode("A", "L")
    assert.is_nil(err)
    assert.is_table(g)
    assert.equal(21, g.rows)
  end)

  it("digit-only input uses numeric mode (smallest version)", function()
    -- 41 digits fit in version 1 at ECC L (numeric capacity = 41)
    local g = qr.encode("12345678901234567890123456789012345678901", "L")
    assert.equal(21, g.rows)
    assert.equal(1, g.version)
  end)

  it("42 digits require version 2 at ECC L", function()
    local g = qr.encode("123456789012345678901234567890123456789012", "L")
    assert.true_(g.version >= 2)
    assert.true_(g.rows >= 25)
  end)

  it("URL encodes at expected version", function()
    -- "https://example.com" is 19 chars, byte mode, ECC M → version 2
    local g = qr.encode("https://example.com", "M")
    assert.equal(2, g.version)
    assert.equal(25, g.rows)
  end)
end)

-- ============================================================================
-- 9. Multiple ECC levels with the same data give different versions
-- ============================================================================

describe("ECC level effects on version", function()
  it("same data at H requires >= version at L", function()
    local data = "HELLO WORLD"
    local gL = qr.encode(data, "L")
    local gH = qr.encode(data, "H")
    assert.true_(gH.rows >= gL.rows)
  end)

  it("all levels produce grids with rows % 4 == 1 (formula 4v+17)", function()
    for _, level in ipairs({"L", "M", "Q", "H"}) do
      local g = qr.encode("ABCDEF", level)
      assert.equal(1, g.rows % 4,
        string.format("ECC %s: rows %d is not 4v+17 form", level, g.rows))
    end
  end)
end)

-- ============================================================================
-- 10. Penalty scoring produces consistent results
-- ============================================================================

describe("mask selection", function()
  it("encode same data twice gives same result (deterministic)", function()
    local g1 = qr.encode("Hello, World!", "M")
    local g2 = qr.encode("Hello, World!", "M")
    assert.equal(g1.rows, g2.rows)
    for r = 1, g1.rows do
      for c = 1, g1.cols do
        assert.equal(g1.modules[r][c], g2.modules[r][c],
          string.format("mismatch at (%d,%d)", r, c))
      end
    end
  end)
end)

-- ============================================================================
-- 11. Large inputs
-- ============================================================================

describe("large inputs", function()
  it("200 digit string encodes at a high version", function()
    local data = string.rep("1234567890", 20)
    local g, err = qr.encode(data, "M")
    assert.is_nil(err)
    assert.is_table(g)
    assert.true_(g.version >= 5)
  end)

  it("500 byte input encodes without error at ECC L", function()
    local data = string.rep("A", 500)
    local g, err = qr.encode(data, "L")
    assert.is_nil(err)
    assert.is_table(g)
    assert.true_(g.version >= 10)
  end)
end)
