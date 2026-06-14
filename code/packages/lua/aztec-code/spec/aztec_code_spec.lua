-- spec/aztec_code_spec.lua — tests for coding_adventures.aztec_code
--
-- Run from the package root:
--   cd code/packages/lua/aztec-code && busted spec/ --verbose

package.path = (
  "./src/?.lua;" ..
  "./src/?/init.lua;" ..
  package.path
)

local aztec = require("coding_adventures.aztec_code")

-- ============================================================================
-- Helpers
-- ============================================================================

local function dark(grid, r, c)
  return grid.modules[r][c] == true
end

-- ============================================================================
-- Version
-- ============================================================================

describe("VERSION", function()
  it("is 0.1.0", function()
    assert.equal("0.1.0", aztec.VERSION)
  end)
end)

-- ============================================================================
-- Error constants
-- ============================================================================

describe("error constants", function()
  it("AztecError is a string", function()
    assert.is_string(aztec.AztecError)
  end)

  it("InputTooLongError is a string", function()
    assert.is_string(aztec.InputTooLongError)
  end)
end)

-- ============================================================================
-- encode() — basic shapes
-- ============================================================================

describe("encode", function()
  it("returns a grid for 'A'", function()
    local grid, err = aztec.encode("A")
    assert.is_nil(err)
    assert.not_nil(grid)
    assert.equal(15, grid.rows)
    assert.equal(15, grid.cols)
  end)

  it("returns a 15×15 compact-1 symbol for a single byte", function()
    local grid = aztec.encode("A")
    assert.equal(15, grid.rows)
    assert.equal(15, grid.cols)
  end)

  it("symbol is always square", function()
    for _, data in ipairs({"X", "Hello", "Hello, World!", string.rep("A", 50)}) do
      local grid = aztec.encode(data)
      assert.equal(grid.rows, grid.cols, "Not square for: " .. data)
    end
  end)

  it("larger inputs produce bigger symbols", function()
    local small = aztec.encode("A")
    local large = aztec.encode(string.rep("A", 200))
    assert.is_true(large.rows > small.rows)
  end)

  it("is deterministic", function()
    local g1 = aztec.encode("Hello!")
    local g2 = aztec.encode("Hello!")
    assert.equal(g1.rows, g2.rows)
    assert.equal(g1.cols, g2.cols)
    for r = 1, g1.rows do
      for c = 1, g1.cols do
        assert.equal(g1.modules[r][c], g2.modules[r][c],
          string.format("module mismatch at (%d,%d)", r, c))
      end
    end
  end)

  it("different inputs produce different grids", function()
    local g1 = aztec.encode("ABC")
    local g2 = aztec.encode("XYZ")
    local same = (g1.rows == g2.rows and g1.cols == g2.cols)
    if same then
      local found_diff = false
      for r = 1, g1.rows do
        for c = 1, g1.cols do
          if g1.modules[r][c] ~= g2.modules[r][c] then
            found_diff = true
            break
          end
        end
        if found_diff then break end
      end
      assert.is_true(found_diff, "Different inputs produced identical grids")
    end
  end)

  it("errors on huge input", function()
    local grid, err = aztec.encode(string.rep("x", 3000))
    assert.is_nil(grid)
    assert.not_nil(err)
  end)
end)

-- ============================================================================
-- Bullseye structure
-- ============================================================================

describe("bullseye", function()
  it("center module is dark for compact-1", function()
    local grid = aztec.encode("A")
    local cx = math.floor(grid.rows / 2) + 1  -- 1-indexed center
    local cy = math.floor(grid.cols / 2) + 1
    assert.is_true(dark(grid, cx, cy), "Center module must be dark")
  end)

  it("compact flag is true for small input", function()
    local grid = aztec.encode("A")
    assert.is_true(grid.compact)
  end)

  it("layers field is set", function()
    local grid = aztec.encode("A")
    assert.is_number(grid.layers)
    assert.is_true(grid.layers >= 1)
  end)
end)

-- ============================================================================
-- Options
-- ============================================================================

describe("options", function()
  it("accepts min_ecc_percent", function()
    local grid = aztec.encode("A", { min_ecc_percent = 33 })
    assert.not_nil(grid)
    assert.is_true(grid.rows >= 15)
  end)

  it("higher ECC may produce larger symbol", function()
    local low  = aztec.encode("A", { min_ecc_percent = 23 })
    local high = aztec.encode("A", { min_ecc_percent = 80 })
    assert.is_true(high.rows >= low.rows)
  end)
end)

-- ============================================================================
-- Module grid shape
-- ============================================================================

describe("module grid", function()
  it("has correct row count", function()
    local grid = aztec.encode("Hello")
    assert.equal(grid.rows, #grid.modules)
  end)

  it("each row has correct column count", function()
    local grid = aztec.encode("Hello")
    for r = 1, grid.rows do
      assert.equal(grid.cols, #grid.modules[r],
        "row " .. r .. " has wrong column count")
    end
  end)

  it("modules contain only booleans", function()
    local grid = aztec.encode("A")
    for r = 1, grid.rows do
      for c = 1, grid.cols do
        local v = grid.modules[r][c]
        assert.is_true(type(v) == "boolean",
          string.format("module[%d][%d] is %s, not boolean", r, c, type(v)))
      end
    end
  end)
end)
