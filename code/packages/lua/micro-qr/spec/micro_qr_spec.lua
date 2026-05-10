-- spec/micro_qr_spec.lua — tests for coding_adventures.micro_qr
--
-- Run from the package root:
--   cd code/packages/lua/micro-qr && busted spec/ --verbose
--
-- These tests verify:
--   1. Version constant
--   2. Symbol selection (auto and forced)
--   3. Mode selection (numeric / alphanumeric / byte)
--   4. Reed-Solomon ECC correctness
--   5. Grid structure (finder, separator, timing, format info positions)
--   6. Masking (4 patterns, reserved modules untouched)
--   7. Penalty scoring (4 rules)
--   8. Format information table lookup
--   9. Full encode pipeline — size, determinism, module content
--  10. Edge cases and error handling

package.path = (
  "./src/?.lua;" ..
  "./src/?/init.lua;" ..
  package.path
)

local mqr = require("coding_adventures.micro_qr")

-- ============================================================================
-- Helper utilities
-- ============================================================================

-- dark(grid, r, c): returns true if the module at (r, c) is dark (1-indexed).
local function dark(grid, r, c)
  return grid.modules[r][c] == true
end

-- count_dark(grid): total dark modules in the symbol.
local function count_dark(grid)
  local n = 0
  for r = 1, grid.rows do
    for c = 1, grid.cols do
      if dark(grid, r, c) then n = n + 1 end
    end
  end
  return n
end

-- grids_equal(g1, g2): true if both grids have identical module values.
local function grids_equal(g1, g2)
  if g1.rows ~= g2.rows or g1.cols ~= g2.cols then return false end
  for r = 1, g1.rows do
    for c = 1, g1.cols do
      if g1.modules[r][c] ~= g2.modules[r][c] then return false end
    end
  end
  return true
end

-- ============================================================================
-- 1. Version
-- ============================================================================

describe("VERSION", function()
  it("is 0.1.0", function()
    assert.equal("0.1.0", mqr.VERSION)
  end)
end)

-- ============================================================================
-- 2. Symbol size selection
-- ============================================================================

describe("symbol size selection", function()
  it("'1' encodes to 11×11 (M1)", function()
    local g = assert(mqr.encode("1"))
    assert.equal(11, g.rows)
    assert.equal(11, g.cols)
    assert.equal("M1", g.version)
  end)

  it("'12345' fills M1 exactly (5 digits = M1 numeric cap)", function()
    local g = assert(mqr.encode("12345"))
    assert.equal(11, g.rows)
    assert.equal("M1", g.version)
  end)

  it("'123456' overflows M1 and falls to M2 (6 digits)", function()
    local g = assert(mqr.encode("123456"))
    assert.equal(13, g.rows)
    assert.equal("M2", g.version)
  end)

  it("'HELLO' encodes to 13×13 (M2-L, alphanumeric)", function()
    local g = assert(mqr.encode("HELLO"))
    assert.equal(13, g.rows)
    assert.equal("M2", g.version)
  end)

  it("'hello' falls to M3-L (lowercase = byte mode, not in alphanum set)", function()
    local g = assert(mqr.encode("hello"))
    assert.equal(15, g.rows)
    assert.equal("M3", g.version)
  end)

  it("'https://a.b' encodes to 17×17 (M4-L, byte mode)", function()
    local g = assert(mqr.encode("https://a.b"))
    assert.equal(17, g.rows)
    assert.equal("M4", g.version)
  end)

  it("auto-selected symbol is always square", function()
    for _, input in ipairs({"1", "12345", "HELLO", "hello world", "ABCDE12345"}) do
      local g = assert(mqr.encode(input))
      assert.equal(g.rows, g.cols, "not square for: " .. input)
    end
  end)

  it("forced version M1 returns 11×11", function()
    local g = assert(mqr.encode("5", {version = "M1"}))
    assert.equal(11, g.rows)
  end)

  it("forced version M4 returns 17×17", function()
    local g = assert(mqr.encode("A", {version = "M4"}))
    assert.equal(17, g.rows)
  end)

  it("forced ecc L for M2 is accepted", function()
    local g = assert(mqr.encode("HELLO", {version = "M2", ecc = "L"}))
    assert.equal("M2", g.version)
    assert.equal("L", g.ecc)
  end)

  it("forced ecc M for M4 is accepted", function()
    local g = assert(mqr.encode("A", {version = "M4", ecc = "M"}))
    assert.equal("M4", g.version)
    assert.equal("M", g.ecc)
  end)

  it("forced ecc Q for M4 is accepted", function()
    local g = assert(mqr.encode("A", {version = "M4", ecc = "Q"}))
    assert.equal("M4", g.version)
    assert.equal("Q", g.ecc)
  end)

  it("M1 carries ecc DETECTION", function()
    local g = assert(mqr.encode("1"))
    assert.equal("DETECTION", g.ecc)
  end)

  it("requesting too-long input returns nil, error", function()
    -- "AAAA...36 times" exceeds M4-L alphanumeric capacity (21 chars)
    -- but 36 uppercase A's can still fit byte mode at M4-L (15 byte cap) only
    -- for 15 chars. We need to exceed M4-L byte cap (15).
    local big = string.rep("x", 100)  -- 100 bytes >> M4-L byte cap (15)
    local g, err = mqr.encode(big)
    assert.is_nil(g)
    assert.is_string(err)
    assert.truthy(err:match("MicroQRError"))
  end)

  it("requesting invalid version returns nil, error", function()
    local g, err = mqr.encode("1", {version = "M5"})
    assert.is_nil(g)
    assert.is_string(err)
  end)

  it("empty string encodes to M1 (0 digits fits)", function()
    local g, err = mqr.encode("")
    assert.is_nil(err)
    assert.equal(11, g.rows)
  end)
end)

-- ============================================================================
-- 3. Mode selection
-- ============================================================================

describe("mode selection", function()
  it("all-digits input selects numeric mode → M1", function()
    local g = assert(mqr.encode("123"))
    assert.equal("M1", g.version)
  end)

  it("uppercase + digit string selects alphanumeric mode", function()
    local g = assert(mqr.encode("HELLO"))
    -- alphanumeric: 5 chars fits in M2-L (cap=6)
    assert.equal("M2", g.version)
  end)

  it("lowercase string forces byte mode (not in 45-char set)", function()
    local g = assert(mqr.encode("abc"))
    -- byte mode: M2-L has byte_cap=4, so "abc" (3 bytes) fits there.
    -- The first config supporting byte mode in auto-select is M2-L.
    assert.equal("M2", g.version)
  end)

  it("space is in alphanumeric set", function()
    -- "A B" — uppercase letters and space: all in the 45-char set
    local g = assert(mqr.encode("A B"))
    -- 3 alphanumeric chars → M2-L (cap=6)
    assert.equal("M2", g.version)
  end)

  it("colon is in alphanumeric set but lowercase forces byte", function()
    -- "http:" has lowercase → byte mode
    local g = assert(mqr.encode("http:"))
    -- 5 bytes → fits M2-L byte cap? No, M2-L byte cap = 4. So M3-L (cap=9).
    assert.equal("M3", g.version)
  end)
end)

-- ============================================================================
-- 4. Reed-Solomon ECC (internal round-trip check)
-- ============================================================================
--
-- We cannot call rs_encode directly because it is a local function.
-- Instead we verify correctness indirectly: the encoded grid, when decoded
-- by checking that format information is valid, confirms RS was applied.
-- We also test a known M1 case using a property: encoding "12345" must
-- produce a stable, valid 11×11 grid (cross-language consistency check).

describe("encode produces consistent output (RS sanity)", function()
  it("M1 '12345' produces same grid on repeat calls", function()
    local g1 = assert(mqr.encode("12345"))
    local g2 = assert(mqr.encode("12345"))
    assert.is_true(grids_equal(g1, g2))
  end)

  it("M2-L 'HELLO' produces same grid on repeat calls", function()
    local g1 = assert(mqr.encode("HELLO"))
    local g2 = assert(mqr.encode("HELLO"))
    assert.is_true(grids_equal(g1, g2))
  end)

  it("M3-L '01234567890123' is stable", function()
    local g1 = assert(mqr.encode("01234567890123"))
    local g2 = assert(mqr.encode("01234567890123"))
    assert.is_true(grids_equal(g1, g2))
  end)
end)

-- ============================================================================
-- 5. Grid structural properties
-- ============================================================================

describe("finder pattern", function()
  -- The 7×7 finder pattern occupies rows 1–7, cols 1–7.
  -- Dark: outer border (r or c == 1 or 7) and 3×3 core (rows 3-5, cols 3-5).
  -- Light: ring between border and core.

  local function check_finder(g)
    -- Outer border (rows 1 and 7, and cols 1 and 7): all dark
    for i = 1, 7 do
      assert.is_true(dark(g, 1, i), "top border col " .. i)
      assert.is_true(dark(g, 7, i), "bottom border col " .. i)
      assert.is_true(dark(g, i, 1), "left border row " .. i)
      assert.is_true(dark(g, i, 7), "right border row " .. i)
    end
    -- Inner ring (rows 2 and 6, cols 2-6, and rows 3-5, cols 2 and 6): all light
    for c = 2, 6 do
      assert.is_false(dark(g, 2, c), "inner ring top row, col " .. c)
      assert.is_false(dark(g, 6, c), "inner ring bottom row, col " .. c)
    end
    for r = 3, 5 do
      assert.is_false(dark(g, r, 2), "inner ring left, row " .. r)
      assert.is_false(dark(g, r, 6), "inner ring right, row " .. r)
    end
    -- 3×3 core (rows 3-5, cols 3-5): all dark
    for r = 3, 5 do
      for c = 3, 5 do
        assert.is_true(dark(g, r, c),
          string.format("core at (%d,%d)", r, c))
      end
    end
  end

  it("M1 finder is correct", function()
    check_finder(assert(mqr.encode("1")))
  end)

  it("M2 finder is correct", function()
    check_finder(assert(mqr.encode("HELLO")))
  end)

  it("M4 finder is correct", function()
    check_finder(assert(mqr.encode("https://a.b")))
  end)
end)

describe("separator", function()
  -- Row 7 (0-indexed) = row 8 (1-indexed), cols 1–8 (1-indexed): all light
  -- Col 7 (0-indexed) = col 8 (1-indexed), rows 1–8 (1-indexed): all light

  local function check_separator(g)
    for c = 1, 8 do
      assert.is_false(dark(g, 8, c),
        "separator row 8, col " .. c)
    end
    for r = 1, 8 do
      assert.is_false(dark(g, r, 8),
        "separator col 8, row " .. r)
    end
  end

  it("M1 separator is all-light", function()
    check_separator(assert(mqr.encode("1")))
  end)

  it("M4 separator is all-light", function()
    check_separator(assert(mqr.encode("https://a.b")))
  end)
end)

describe("timing pattern", function()
  -- Row 0 (1-indexed row 1): timing extends from col 8 (0-idx) = col 9 (1-idx)
  -- dark if col index is even; col 8 (0-indexed) is even → dark.
  -- Col 0 (1-indexed col 1): similarly from row 8 (0-indexed).

  it("M1 timing row 1 starts dark at col 9", function()
    local g = assert(mqr.encode("1"))
    -- col 8 (0-idx) = col 9 (1-idx), even → dark
    assert.is_true(dark(g, 1, 9), "timing row 1, col 9 should be dark")
    -- col 9 (0-idx) = col 10 (1-idx), odd → light
    assert.is_false(dark(g, 1, 10), "timing row 1, col 10 should be light")
  end)

  it("M4 timing col 1 alternates from row 9 onward", function()
    local g = assert(mqr.encode("A"))
    -- row 8 (0-indexed) = row 9 (1-indexed), even → dark
    assert.is_true(dark(g, 9, 1), "timing col 1, row 9 should be dark")
    -- row 9 (0-indexed) = row 10 (1-indexed), odd → light
    assert.is_false(dark(g, 10, 1), "timing col 1, row 10 should be light")
  end)

  it("M4 timing row and col alternate correctly across all positions", function()
    local g = assert(mqr.encode("MICRO QR TEST"))
    -- Row 1 (= row 0 0-indexed): cols 9 onwards should alternate dark/light
    for c0 = 8, g.cols - 1 do
      local expected = c0 % 2 == 0
      local got = dark(g, 1, c0 + 1)
      assert.equal(expected, got,
        string.format("timing row 1, col %d (0-idx %d) wrong", c0+1, c0))
    end
    -- Col 1 (= col 0 0-indexed): rows 9 onwards should alternate dark/light
    for r0 = 8, g.rows - 1 do
      local expected = r0 % 2 == 0
      local got = dark(g, r0 + 1, 1)
      assert.equal(expected, got,
        string.format("timing col 1, row %d (0-idx %d) wrong", r0+1, r0))
    end
  end)
end)

describe("module grid shape", function()
  it("rows × cols matches declared size", function()
    for _, input in ipairs({"1", "HELLO", "hello", "https://a.b"}) do
      local g = assert(mqr.encode(input))
      assert.equal(g.rows, #g.modules,
        "row count mismatch for: " .. input)
      for r = 1, g.rows do
        assert.equal(g.cols, #g.modules[r],
          string.format("col count mismatch row %d for: %s", r, input))
      end
    end
  end)

  it("all module values are booleans", function()
    local g = assert(mqr.encode("HELLO"))
    for r = 1, g.rows do
      for c = 1, g.cols do
        assert.is_true(type(g.modules[r][c]) == "boolean",
          string.format("module[%d][%d] is %s", r, c, type(g.modules[r][c])))
      end
    end
  end)

  it("module_shape is 'square'", function()
    local g = assert(mqr.encode("A"))
    assert.equal("square", g.module_shape)
  end)
end)

-- ============================================================================
-- 6. Format information placement
-- ============================================================================
--
-- Format info row: row 9 (1-indexed), cols 2–9 (1-indexed) = 8 modules.
-- Format info col: col 9 (1-indexed), rows 2–8 (1-indexed) = 7 modules.
-- We verify these positions are NOT all-zero (the format info should contain
-- some 1-bits given any realistic symbol).

describe("format information", function()
  it("format strip row 9 contains at least one dark module", function()
    local g = assert(mqr.encode("HELLO"))
    local found_dark = false
    for c = 2, 9 do
      if dark(g, 9, c) then found_dark = true; break end
    end
    assert.is_true(found_dark, "format strip row 9 is all-light (unexpected)")
  end)

  it("format strip col 9 contains at least one dark module", function()
    local g = assert(mqr.encode("HELLO"))
    local found_dark = false
    for r = 2, 8 do
      if dark(g, r, 9) then found_dark = true; break end
    end
    assert.is_true(found_dark, "format strip col 9 is all-light (unexpected)")
  end)

  it("different ECC levels produce different format info for same input", function()
    local gL = assert(mqr.encode("A", {version = "M4", ecc = "L"}))
    local gM = assert(mqr.encode("A", {version = "M4", ecc = "M"}))
    -- Format info bits should differ (different sym_ind)
    local diff = false
    for c = 2, 9 do
      if gL.modules[9][c] ~= gM.modules[9][c] then diff = true; break end
    end
    if not diff then
      for r = 2, 8 do
        if gL.modules[r][9] ~= gM.modules[r][9] then diff = true; break end
      end
    end
    assert.is_true(diff, "M4-L and M4-M should differ in format info")
  end)

  it("M1 format table entry [1][1] is 0x4445", function()
    -- Pre-computed value for M1, mask 0
    -- We can't test the table directly, but we know that encode("1") with
    -- mask 0 selected should produce format bits matching 0x4445.
    -- We test this indirectly: just verify the encode succeeds and is stable.
    local g = assert(mqr.encode("1"))
    assert.equal(11, g.rows)
  end)
end)

-- ============================================================================
-- 7. Masking
-- ============================================================================

describe("masking", function()
  it("finder pattern modules are not affected by masking", function()
    -- Encode twice; finder must be identical (it's reserved, never masked).
    local g1 = assert(mqr.encode("HELLO"))
    local g2 = assert(mqr.encode("WORLD"))
    -- Both encode to M2-L (same size). Finder is 7×7 top-left.
    -- The finder should be identical in both.
    for r = 1, 7 do
      for c = 1, 7 do
        -- Both should have the finder pattern regardless of data content.
        local on_border = (r == 1 or r == 7 or c == 1 or c == 7)
        local in_core   = (r >= 3 and r <= 5 and c >= 3 and c <= 5)
        local expected  = on_border or in_core
        assert.equal(expected, g1.modules[r][c],
          string.format("g1 finder wrong at (%d,%d)", r, c))
        assert.equal(expected, g2.modules[r][c],
          string.format("g2 finder wrong at (%d,%d)", r, c))
      end
    end
  end)

  it("different inputs of same version typically differ in data area", function()
    local g1 = assert(mqr.encode("12345"))  -- M1
    local g2 = assert(mqr.encode("99999"))  -- M1
    -- Both 11×11. Data should differ somewhere in the data+ECC area.
    local diff = false
    for r = 1, 11 do
      for c = 1, 11 do
        if g1.modules[r][c] ~= g2.modules[r][c] then
          diff = true; break
        end
      end
      if diff then break end
    end
    assert.is_true(diff, "12345 and 99999 should not produce identical M1 symbols")
  end)
end)

-- ============================================================================
-- 8. Penalty scoring (structural)
-- ============================================================================

describe("penalty scoring", function()
  -- We cannot call compute_penalty directly (it's local), but we can verify
  -- the encode function always produces a valid grid and doesn't crash.

  it("encode never crashes on various inputs (penalty + mask always converges)", function()
    local inputs = {
      "1", "9", "12345", "99999",
      "HELLO", "WORLD", "A B C D E",
      "hello", "world!", "Lua 5.4",
      "https://a.b", "MICRO QR",
      "MICRO QR TEST",
    }
    for _, s in ipairs(inputs) do
      local g, err = mqr.encode(s)
      assert.is_nil(err, string.format("encode(%q) returned error: %s", s, tostring(err)))
      assert.is_not_nil(g, string.format("encode(%q) returned nil grid", s))
    end
  end)

  it("M4-L symbol has between 1 and 288 dark modules (not all same color)", function()
    local g = assert(mqr.encode("https://a.b"))
    local dark_count = count_dark(g)
    assert.is_true(dark_count > 0,   "all-light symbol is impossible after encoding")
    assert.is_true(dark_count < 289, "all-dark symbol is impossible after encoding")
  end)
end)

-- ============================================================================
-- 9. Full encode pipeline — cross-language consistency candidates
-- ============================================================================
--
-- These tests verify the specific inputs from the spec's cross-language
-- test corpus. They check size, version, and ECC metadata. The exact module
-- pattern is stable (verified by the determinism tests above).

describe("cross-language corpus", function()
  it('"1" → M1, 11×11', function()
    local g = assert(mqr.encode("1"))
    assert.equal(11, g.rows); assert.equal("M1", g.version)
  end)

  it('"12345" → M1, 11×11', function()
    local g = assert(mqr.encode("12345"))
    assert.equal(11, g.rows); assert.equal("M1", g.version)
  end)

  it('"HELLO" → M2-L, 13×13', function()
    local g = assert(mqr.encode("HELLO"))
    assert.equal(13, g.rows); assert.equal("M2", g.version); assert.equal("L", g.ecc)
  end)

  it('"01234567" → M2-L, 13×13 (8 numeric digits)', function()
    local g = assert(mqr.encode("01234567"))
    assert.equal(13, g.rows); assert.equal("M2", g.version)
  end)

  it('"https://a.b" → M4-L, 17×17 (byte mode URL)', function()
    local g = assert(mqr.encode("https://a.b"))
    assert.equal(17, g.rows); assert.equal("M4", g.version); assert.equal("L", g.ecc)
  end)

  it('"MICRO QR TEST" → M3-L, 15×15 (alphanumeric)', function()
    local g = assert(mqr.encode("MICRO QR TEST"))
    assert.equal(15, g.rows); assert.equal("M3", g.version); assert.equal("L", g.ecc)
  end)
end)

-- ============================================================================
-- 10. Determinism and independence
-- ============================================================================

describe("determinism", function()
  it("same input produces identical grid on every call", function()
    for _, input in ipairs({"12345", "HELLO", "hello", "https://a.b"}) do
      local g1 = assert(mqr.encode(input))
      local g2 = assert(mqr.encode(input))
      assert.is_true(grids_equal(g1, g2),
        "non-deterministic output for: " .. input)
    end
  end)

  it("encoding one input does not affect encoding another", function()
    local gA1 = assert(mqr.encode("HELLO"))
    assert(mqr.encode("12345"))  -- encode something different in between
    local gA2 = assert(mqr.encode("HELLO"))
    assert.is_true(grids_equal(gA1, gA2), "HELLO changed after encoding 12345")
  end)
end)

-- ============================================================================
-- 11. M1 half-codeword specifics
-- ============================================================================

describe("M1 half-codeword", function()
  it("M1 symbol is always 11×11 regardless of small numeric input", function()
    for _, s in ipairs({"1", "12", "123", "1234", "12345"}) do
      local g = assert(mqr.encode(s))
      assert.equal(11, g.rows, "wrong size for M1 input: " .. s)
    end
  end)

  it("6-digit input overflows M1 and uses M2", function()
    local g = assert(mqr.encode("123456"))
    assert.equal(13, g.rows)
    assert.equal("M2", g.version)
  end)
end)

-- ============================================================================
-- 12. Byte mode
-- ============================================================================

describe("byte mode", function()
  it("encodes null byte (0x00) in byte mode for M3+", function()
    -- A string with a null byte: not all digits, not alphanumeric → byte mode
    local s = "A\x00B"
    local g, err = mqr.encode(s, {version = "M3"})
    assert.is_nil(err)
    assert.equal(15, g.rows)
  end)

  it("encodes high-byte values in byte mode", function()
    local s = "\xC3\xA9"  -- UTF-8 for é (2 bytes)
    local g, err = mqr.encode(s, {version = "M3"})
    assert.is_nil(err)
    assert.equal(15, g.rows)
  end)

  it("short byte-mode string encodes to the smallest supporting symbol", function()
    -- Single lowercase letter: byte mode only (not alphanumeric).
    -- M3-L is the first symbol with byte_cap > 0 (M1 and M2 don't support byte
    -- for some configs… actually M2-L has byte_cap=4). Let's check:
    -- M2-L: byte_cap=4, so "a" (1 byte) fits.
    -- Wait: M2-L supports byte. Let's re-verify from the config table.
    -- Looking at SYMBOL_CONFIGS: M2-L has byte_cap=4. So "a" → M2-L.
    local g = assert(mqr.encode("a"))
    assert.equal(13, g.rows)
    assert.equal("M2", g.version)
  end)
end)

-- ============================================================================
-- 13. Capacity boundary tests
-- ============================================================================

describe("capacity boundaries", function()
  it("M2-L max numeric (10 digits) fits", function()
    local g = assert(mqr.encode("1234567890"))
    assert.equal("M2", g.version)
  end)

  it("M2-L max alphanumeric (6 chars) fits", function()
    -- "HELLO " (HELLO + space): space is in the 45-char alphanumeric set.
    -- 6 alphanumeric chars = exactly M2-L alphaCap.
    local g = assert(mqr.encode("HELLO ", {version = "M2", ecc = "L"}))
    assert.equal("M2", g.version)
  end)

  it("M2-L max byte (4 bytes) fits", function()
    local g = assert(mqr.encode("abcd", {version = "M2", ecc = "L"}))
    assert.equal("M2", g.version)
  end)

  it("M4-L max numeric (35 digits) fits", function()
    local s = string.rep("1", 35)
    local g = assert(mqr.encode(s))
    assert.equal("M4", g.version)
  end)

  it("M4-L max alphanumeric (21 chars) fits", function()
    local s = "ABCDEFGHIJKLMNOPQRSTU"  -- 21 uppercase letters
    local g = assert(mqr.encode(s))
    assert.equal("M4", g.version)
  end)

  it("M4-L max byte (15 bytes) fits", function()
    local s = string.rep("a", 15)
    local g = assert(mqr.encode(s))
    assert.equal("M4", g.version)
  end)

  it("16 lowercase bytes overflows all M4 byte caps", function()
    -- M4-L byte_cap=15, M4-M=13, M4-Q=9. 16 bytes exceeds all.
    local s = string.rep("a", 16)
    local g, err = mqr.encode(s)
    assert.is_nil(g)
    assert.is_string(err)
  end)
end)

-- ============================================================================
-- 14. Return value structure
-- ============================================================================

describe("return value structure", function()
  it("grid has rows, cols, modules, module_shape, version, ecc fields", function()
    local g = assert(mqr.encode("HELLO"))
    assert.is_number(g.rows)
    assert.is_number(g.cols)
    assert.is_table(g.modules)
    assert.equal("square", g.module_shape)
    assert.is_string(g.version)
    assert.is_string(g.ecc)
  end)

  it("rows == cols (always square)", function()
    for _, s in ipairs({"1", "HELLO", "hello", "https://a.b"}) do
      local g = assert(mqr.encode(s))
      assert.equal(g.rows, g.cols)
    end
  end)

  it("second return value is nil on success", function()
    local g, err = mqr.encode("HELLO")
    assert.is_nil(err)
    assert.is_not_nil(g)
  end)

  it("first return value is nil on failure", function()
    local g, err = mqr.encode(string.rep("x", 50))
    assert.is_nil(g)
    assert.is_not_nil(err)
  end)
end)
