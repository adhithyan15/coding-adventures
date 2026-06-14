-- spec/pdf417_spec.lua — tests for coding_adventures.pdf417
--
-- Run from the package root:
--   cd code/packages/lua/pdf417 && busted spec/ --verbose

package.path = (
  "./src/?.lua;" ..
  "./src/?/init.lua;" ..
  package.path
)

local pdf417 = require("coding_adventures.pdf417")

-- ============================================================================
-- Helpers
-- ============================================================================

local function default_opts()
  return {}  -- auto ECC, auto columns, default row_height
end

-- ============================================================================
-- Version
-- ============================================================================

describe("VERSION", function()
  it("is 0.1.0", function()
    assert.equal("0.1.0", pdf417.VERSION)
  end)
end)

-- ============================================================================
-- Error constants
-- ============================================================================

describe("error constants", function()
  it("PDF417Error is a string", function()
    assert.is_string(pdf417.PDF417Error)
  end)

  it("InputTooLongError is a string", function()
    assert.is_string(pdf417.InputTooLongError)
  end)

  it("InvalidDimensionsError is a string", function()
    assert.is_string(pdf417.InvalidDimensionsError)
  end)

  it("InvalidECCLevelError is a string", function()
    assert.is_string(pdf417.InvalidECCLevelError)
  end)
end)

-- ============================================================================
-- encode() — basic
-- ============================================================================

describe("encode", function()
  it("returns a grid for a short string", function()
    local grid, err = pdf417.encode("Hello, PDF417!", default_opts())
    assert.is_nil(err)
    assert.not_nil(grid)
    assert.is_number(grid.rows)
    assert.is_number(grid.cols)
    assert.is_true(grid.rows >= pdf417.MIN_ROWS)
  end)

  it("returns a grid for an empty string", function()
    local grid, err = pdf417.encode("")
    assert.is_nil(err)
    assert.not_nil(grid)
  end)

  it("is deterministic", function()
    local g1 = pdf417.encode("determinism check")
    local g2 = pdf417.encode("determinism check")
    assert.equal(g1.rows, g2.rows)
    assert.equal(g1.cols, g2.cols)
    for r = 1, g1.rows do
      for c = 1, g1.cols do
        assert.equal(g1.modules[r][c], g2.modules[r][c],
          string.format("mismatch at (%d,%d)", r, c))
      end
    end
  end)

  it("larger input produces more rows or cols", function()
    local small = pdf417.encode("A")
    local large = pdf417.encode(string.rep("A", 200))
    assert.is_true(large.rows * large.cols > small.rows * small.cols)
  end)
end)

-- ============================================================================
-- Module grid shape
-- ============================================================================

describe("module grid", function()
  it("row count matches grid.rows", function()
    local grid = pdf417.encode("shape check")
    assert.equal(grid.rows, #grid.modules)
  end)

  it("each row has grid.cols modules", function()
    local grid = pdf417.encode("shape check")
    for r = 1, grid.rows do
      assert.equal(grid.cols, #grid.modules[r],
        "row " .. r .. " width mismatch")
    end
  end)

  it("modules contain only booleans", function()
    local grid = pdf417.encode("A")
    for r = 1, grid.rows do
      for c = 1, grid.cols do
        local v = grid.modules[r][c]
        assert.is_true(type(v) == "boolean",
          string.format("module[%d][%d] is %s", r, c, type(v)))
      end
    end
  end)
end)

-- ============================================================================
-- ECC level options
-- ============================================================================

describe("ecc_level option", function()
  it("accepts explicit ECC levels 0-8", function()
    for level = 0, 8 do
      local grid, err = pdf417.encode("test", { ecc_level = level })
      assert.is_nil(err, "ECC level " .. level .. " failed: " .. tostring(err))
      assert.not_nil(grid)
    end
  end)

  it("higher ECC makes symbol larger or equal", function()
    local low  = pdf417.encode("data data data", { ecc_level = 0 })
    local high = pdf417.encode("data data data", { ecc_level = 5 })
    local low_area  = low.rows  * low.cols
    local high_area = high.rows * high.cols
    assert.is_true(high_area >= low_area)
  end)
end)

-- ============================================================================
-- Column options
-- ============================================================================

describe("columns option", function()
  it("accepts explicit column counts", function()
    for _, cols in ipairs({3, 5, 10}) do
      local grid, err = pdf417.encode("column test with some data", { columns = cols })
      assert.is_nil(err, "cols=" .. cols .. " failed: " .. tostring(err))
      assert.not_nil(grid)
    end
  end)
end)

-- ============================================================================
-- Constants
-- ============================================================================

describe("constants", function()
  it("GF929_PRIME is 929", function()
    assert.equal(929, pdf417.GF929_PRIME)
  end)

  it("MIN_ROWS is 3", function()
    assert.equal(3, pdf417.MIN_ROWS)
  end)

  it("MAX_ROWS is 90", function()
    assert.equal(90, pdf417.MAX_ROWS)
  end)

  it("MIN_COLS is 1", function()
    assert.equal(1, pdf417.MIN_COLS)
  end)

  it("MAX_COLS is 30", function()
    assert.equal(30, pdf417.MAX_COLS)
  end)
end)
