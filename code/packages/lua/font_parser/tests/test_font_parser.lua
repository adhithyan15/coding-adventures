-- Tests for coding_adventures.font_parser
--
-- Exercises the public API against:
--  1. The real Inter Regular v4.0 font (ttf fixture).
--  2. Synthetic minimal OpenType binaries built in-memory to test
--     kern-specific logic (Inter v4.0 uses GPOS, not the legacy kern table).

package.path = "../src/?.lua;" .. "../src/?/init.lua;" .. package.path

local fp = require("coding_adventures.font_parser")

-- ─────────────────────────────────────────────────────────────────────────────
-- Helpers
-- ─────────────────────────────────────────────────────────────────────────────

local FONT_PATH = "../../../../fixtures/fonts/Inter-Regular.ttf"

local function inter_bytes()
  local f = assert(io.open(FONT_PATH, "rb"))
  local data = f:read("*a")
  f:close()
  return data
end

-- Build a minimal valid OpenType binary containing:
--   Tables: cmap (Format 4 sentinel), head, hhea, hmtx, kern (Format 0), maxp
--
-- `pairs` is a Lua array of {left, right, value} tables.
-- All values are integers in font units.
local function build_synthetic_font(pairs)
  -- Pack helpers (string.pack uses 1-based, so just call directly)
  local function w16(v)  return string.pack(">I2", v & 0xFFFF) end
  local function wi16(v) return string.pack(">i2", v) end
  local function w32(v)  return string.pack(">I4", v & 0xFFFFFFFF) end
  local function tag(s)  return string.sub(s .. "\0\0\0\0", 1, 4) end

  local num_tables = 6
  local dir_size = 12 + num_tables * 16

  local head_len = 54
  local hhea_len = 36
  local maxp_len = 6
  local cmap_len = 36
  local hmtx_len = 5 * 4
  local n_pairs  = #pairs
  local kern_len = 4 + 6 + 8 + n_pairs * 6

  local head_off = dir_size
  local hhea_off = head_off + head_len
  local maxp_off = hhea_off + hhea_len
  local cmap_off = maxp_off + maxp_len
  local hmtx_off = cmap_off + cmap_len
  local kern_off = hmtx_off + hmtx_len

  local buf = ""

  -- Offset table
  buf = buf .. w32(0x00010000) .. w16(num_tables) .. w16(64) .. w16(2) .. w16(32)

  -- Table records (sorted alphabetically: cmap < head < hhea < hmtx < kern < maxp)
  local records = {
    {tag("cmap"), cmap_off, cmap_len},
    {tag("head"), head_off, head_len},
    {tag("hhea"), hhea_off, hhea_len},
    {tag("hmtx"), hmtx_off, hmtx_len},
    {tag("kern"), kern_off, kern_len},
    {tag("maxp"), maxp_off, maxp_len},
  }
  for _, r in ipairs(records) do
    buf = buf .. r[1] .. w32(0) .. w32(r[2]) .. w32(r[3])
  end

  -- head (54 bytes)
  -- version(4) + fontRevision(4) + checkSumAdj(4) + magicNumber(4) +
  -- flags(2) + unitsPerEm(2) + created(8) + modified(8) +
  -- xMin(2) + yMin(2) + xMax(2) + yMax(2) + macStyle(2) + lowestRecPPEM(2) +
  -- fontDirectionHint(2) + indexToLocFormat(2) + glyphDataFormat(2) = 54
  buf = buf
    .. w32(0x00010000) .. w32(0x00010000) .. w32(0) .. w32(0x5F0F3CF5)
    .. w16(0) .. w16(1000)
    .. string.rep("\0", 16)  -- created + modified
    .. wi16(0) .. wi16(0) .. wi16(0) .. wi16(0)  -- xMin yMin xMax yMax
    .. w16(0) .. w16(8) .. wi16(2) .. wi16(0) .. wi16(0)

  -- hhea (36 bytes)
  buf = buf
    .. w32(0x00010000) .. wi16(800) .. wi16(-200) .. wi16(0)
    .. w16(1000)
    .. wi16(0) .. wi16(0) .. wi16(0)
    .. wi16(1) .. wi16(0) .. wi16(0)
    .. string.rep("\0", 8)  -- reserved[4]
    .. wi16(0) .. w16(5)

  -- maxp (6 bytes)
  buf = buf .. w32(0x00005000) .. w16(5)

  -- cmap: version=0, numSubtables=1, enc record (plat=3, enc=1, off=12),
  -- Format 4 subtable with 1 segment (0xFFFF terminator).
  buf = buf
    .. w16(0) .. w16(1)
    .. w16(3) .. w16(1) .. w32(12)
    .. w16(4) .. w16(24) .. w16(0)
    .. w16(2) .. w16(2) .. w16(0) .. w16(0)
    .. w16(0xFFFF)
    .. w16(0)
    .. w16(0xFFFF)
    .. wi16(1)
    .. w16(0)

  -- hmtx: 5 full records {600, 50}
  for _ = 1, 5 do
    buf = buf .. w16(600) .. wi16(50)
  end

  -- kern table
  local sub_len = 6 + 8 + n_pairs * 6

  -- Sort pairs by composite key for binary search compliance.
  local sorted = {}
  for _, p in ipairs(pairs) do sorted[#sorted + 1] = p end
  table.sort(sorted, function(a, b)
    return a[1] * 65536 + a[2] < b[1] * 65536 + b[2]
  end)

  buf = buf
    .. w16(0) .. w16(1)
    .. w16(0) .. w16(sub_len) .. w16(0x0001)
    .. w16(n_pairs) .. w16(0) .. w16(0) .. w16(0)

  for _, p in ipairs(sorted) do
    buf = buf .. w16(p[1]) .. w16(p[2]) .. wi16(p[3])
  end

  return buf
end

-- ─────────────────────────────────────────────────────────────────────────────
-- Tests: load
-- ─────────────────────────────────────────────────────────────────────────────

describe("load", function()
  it("raises BufferTooShort on empty string", function()
    local ok, err = pcall(fp.load, "")
    assert.is_false(ok)
    assert.are.equal("BufferTooShort", err.kind)
  end)

  it("raises InvalidMagic on wrong magic bytes", function()
    local buf = string.pack(">I4", 0xDEADBEEF) .. string.rep("\0", 252)
    local ok, err = pcall(fp.load, buf)
    assert.is_false(ok)
    assert.are.equal("InvalidMagic", err.kind)
  end)

  it("raises TableNotFound when numTables=0 and no head table", function()
    local buf = string.pack(">I4", 0x00010000) .. string.pack(">I2", 0) .. string.rep("\0", 6)
    local ok, err = pcall(fp.load, buf)
    assert.is_false(ok)
    assert.are.equal("TableNotFound", err.kind)
  end)

  it("loads Inter Regular without error", function()
    local font = fp.load(inter_bytes())
    assert.is_not_nil(font)
    assert.are.equal("FontFile", font._type)
  end)

  it("loads synthetic font without error", function()
    local font = fp.load(build_synthetic_font({{1, 2, -140}}))
    assert.is_not_nil(font)
  end)
end)

-- ─────────────────────────────────────────────────────────────────────────────
-- Tests: font_metrics
-- ─────────────────────────────────────────────────────────────────────────────

describe("font_metrics", function()
  local font

  setup(function()
    font = fp.load(inter_bytes())
  end)

  it("units_per_em is 2048", function()
    assert.are.equal(2048, fp.font_metrics(font).units_per_em)
  end)

  it("family_name is Inter", function()
    assert.are.equal("Inter", fp.font_metrics(font).family_name)
  end)

  it("subfamily_name is Regular", function()
    assert.are.equal("Regular", fp.font_metrics(font).subfamily_name)
  end)

  it("ascender is positive", function()
    assert.is_true(fp.font_metrics(font).ascender > 0)
  end)

  it("descender is non-positive", function()
    assert.is_true(fp.font_metrics(font).descender <= 0)
  end)

  it("num_glyphs is large", function()
    assert.is_true(fp.font_metrics(font).num_glyphs > 100)
  end)

  it("x_height is positive", function()
    local m = fp.font_metrics(font)
    assert.is_not_nil(m.x_height)
    assert.is_true(m.x_height > 0)
  end)

  it("cap_height is positive", function()
    local m = fp.font_metrics(font)
    assert.is_not_nil(m.cap_height)
    assert.is_true(m.cap_height > 0)
  end)

  it("synthetic font units_per_em is 1000", function()
    local f = fp.load(build_synthetic_font({}))
    assert.are.equal(1000, fp.font_metrics(f).units_per_em)
  end)

  it("synthetic font family_name is (unknown)", function()
    local f = fp.load(build_synthetic_font({}))
    assert.are.equal("(unknown)", fp.font_metrics(f).family_name)
  end)
end)

-- ─────────────────────────────────────────────────────────────────────────────
-- Tests: glyph_id
-- ─────────────────────────────────────────────────────────────────────────────

describe("glyph_id", function()
  local font

  setup(function()
    font = fp.load(inter_bytes())
  end)

  it("glyph_id for 'A' (0x0041) is non-nil", function()
    assert.is_not_nil(fp.glyph_id(font, 0x0041))
  end)

  it("glyph_id for 'V' (0x0056) is non-nil", function()
    assert.is_not_nil(fp.glyph_id(font, 0x0056))
  end)

  it("glyph_id for space (0x0020) is non-nil", function()
    assert.is_not_nil(fp.glyph_id(font, 0x0020))
  end)

  it("glyph_ids for A and V differ", function()
    local gid_a = fp.glyph_id(font, 0x0041)
    local gid_v = fp.glyph_id(font, 0x0056)
    assert.are_not.equal(gid_a, gid_v)
  end)

  it("codepoint above 0xFFFF returns nil", function()
    assert.is_nil(fp.glyph_id(font, 0x10000))
  end)

  it("negative codepoint returns nil", function()
    assert.is_nil(fp.glyph_id(font, -1))
  end)
end)

-- ─────────────────────────────────────────────────────────────────────────────
-- Tests: glyph_metrics
-- ─────────────────────────────────────────────────────────────────────────────

describe("glyph_metrics", function()
  local font

  setup(function()
    font = fp.load(inter_bytes())
  end)

  it("advance_width for 'A' is positive", function()
    local gid = fp.glyph_id(font, 0x0041)
    local gm = fp.glyph_metrics(font, gid)
    assert.is_not_nil(gm)
    assert.is_true(gm.advance_width > 0)
  end)

  it("advance_width for 'A' is in reasonable range", function()
    local gid = fp.glyph_id(font, 0x0041)
    local gm = fp.glyph_metrics(font, gid)
    assert.is_true(gm.advance_width >= 100 and gm.advance_width <= 2400)
  end)

  it("out-of-range glyph returns nil", function()
    local m = fp.font_metrics(font)
    assert.is_nil(fp.glyph_metrics(font, m.num_glyphs))
  end)

  it("negative glyph_id returns nil", function()
    assert.is_nil(fp.glyph_metrics(font, -1))
  end)
end)

-- ─────────────────────────────────────────────────────────────────────────────
-- Tests: kerning
-- ─────────────────────────────────────────────────────────────────────────────

describe("kerning", function()
  it("Inter A+V returns 0 (Inter uses GPOS not kern table)", function()
    local font = fp.load(inter_bytes())
    local gid_a = fp.glyph_id(font, 0x0041)
    local gid_v = fp.glyph_id(font, 0x0056)
    assert.are.equal(0, fp.kerning(font, gid_a, gid_v))
  end)

  it("synthetic pair (1,2) returns -140", function()
    local font = fp.load(build_synthetic_font({{1, 2, -140}, {3, 4, 80}}))
    assert.are.equal(-140, fp.kerning(font, 1, 2))
  end)

  it("synthetic pair (3,4) returns 80", function()
    local font = fp.load(build_synthetic_font({{1, 2, -140}, {3, 4, 80}}))
    assert.are.equal(80, fp.kerning(font, 3, 4))
  end)

  it("absent pair returns 0", function()
    local font = fp.load(build_synthetic_font({{1, 2, -140}, {3, 4, 80}}))
    assert.are.equal(0, fp.kerning(font, 1, 4))
  end)

  it("reversed pair returns 0", function()
    local font = fp.load(build_synthetic_font({{1, 2, -140}}))
    assert.are.equal(0, fp.kerning(font, 2, 1))
  end)
end)
