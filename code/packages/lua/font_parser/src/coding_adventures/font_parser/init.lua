--- coding_adventures.font_parser
--
-- Metrics-only OpenType/TrueType font parser. Zero dependencies.
--
-- OpenType and TrueType font files are binary table databases.  Bytes 0-11
-- hold an *offset table* that names the font format and the number of tables.
-- Starting at byte 12, an array of 16-byte *table records* (tag + checksum +
-- offset + length) lets us jump directly to any table by its 4-byte ASCII tag.
--
-- All multi-byte integers are **big-endian**.  Lua 5.4's `string.unpack` with
-- the `>` format prefix handles this cleanly without manual bit arithmetic.
--
-- ## Tables parsed
--
-- | Tag  | Contents |
-- |------|----------|
-- | head | unitsPerEm |
-- | hhea | ascender, descender, lineGap, numberOfHMetrics |
-- | maxp | numGlyphs |
-- | cmap | Format 4 — Unicode BMP → glyph index |
-- | hmtx | advance width + left-side bearing per glyph |
-- | kern | Format 0 sorted pairs (optional) |
-- | name | family / subfamily strings (optional, UTF-16 BE) |
-- | OS/2 | xHeight, capHeight (optional, version ≥ 2) |
--
-- ## Usage
--
--     local fp = require("coding_adventures.font_parser")
--     local data = io.open("Inter-Regular.ttf", "rb"):read("*a")
--     local font = fp.load(data)
--     local m    = fp.font_metrics(font)
--     print(m.units_per_em)  --> 2048

local M = {}  -- public module table

-- ────────────────────────────────────────────────────────────────────────────
-- Binary read helpers
-- ────────────────────────────────────────────────────────────────────────────
--
-- All helpers take a raw binary string and a **0-based** byte offset.
-- `string.unpack` uses 1-based positions, so we always add 1.

-- read_u8: unsigned 8-bit integer.
local function read_u8(data, off)
  return string.byte(data, off + 1)
end

-- read_u16: unsigned big-endian 16-bit integer.
local function read_u16(data, off)
  return (string.unpack(">I2", data, off + 1))
end

-- read_i16: signed big-endian 16-bit integer (two's complement).
local function read_i16(data, off)
  return (string.unpack(">i2", data, off + 1))
end

-- read_u32: unsigned big-endian 32-bit integer.
local function read_u32(data, off)
  return (string.unpack(">I4", data, off + 1))
end

-- ────────────────────────────────────────────────────────────────────────────
-- Error type
-- ────────────────────────────────────────────────────────────────────────────
--
-- We raise tables so that callers can inspect `err.kind` after `pcall`.

local function font_error(kind, message)
  return {kind = kind, message = message or kind}
end

local function raise(kind, message)
  error(font_error(kind, message), 2)
end

-- ────────────────────────────────────────────────────────────────────────────
-- Offset table + table records
-- ────────────────────────────────────────────────────────────────────────────
--
-- Offset table layout (bytes 0–11):
--   sfntVersion   u32   0x00010000 = TrueType  /  0x4F54544F = "OTTO" (CFF)
--   numTables     u16
--   searchRange   u16   (ignored)
--   entrySelector u16   (ignored)
--   rangeShift    u16   (ignored)
--
-- Each table record (16 bytes, starting at byte 12):
--   tag        4 bytes   ASCII name e.g. "head"
--   checksum   u32       (ignored)
--   offset     u32       absolute byte offset in the file
--   length     u32

local function parse_offset_table(data)
  if #data < 12 then
    raise("BufferTooShort")
  end

  local sfnt_ver = read_u32(data, 0)
  if sfnt_ver ~= 0x00010000 and sfnt_ver ~= 0x4F54544F then
    raise("InvalidMagic")
  end

  local num_tables = read_u16(data, 4)
  local tables = {}

  for i = 0, num_tables - 1 do
    local base = 12 + i * 16
    -- string.sub extracts the 4-byte tag at the correct 1-based position.
    local tag = string.sub(data, base + 1, base + 4)
    local off = read_u32(data, base + 8)
    local len = read_u32(data, base + 12)
    tables[tag] = {offset = off, length = len}
  end

  return tables
end

local function require_table(tables, tag)
  local t = tables[tag]
  if not t then
    raise("TableNotFound", "required table '" .. tag .. "' not found")
  end
  return t.offset, t.length
end

-- ────────────────────────────────────────────────────────────────────────────
-- head table  (unitsPerEm at offset 18)
-- ────────────────────────────────────────────────────────────────────────────

local function parse_head(data, tables)
  local off = require_table(tables, "head")
  return {units_per_em = read_u16(data, off + 18)}
end

-- ────────────────────────────────────────────────────────────────────────────
-- hhea table
-- ────────────────────────────────────────────────────────────────────────────
--
-- Fixed(4)  version
-- i16       ascender      offset 4
-- i16       descender     offset 6
-- i16       lineGap       offset 8
-- ...
-- u16       numberOfHMetrics  offset 34

local function parse_hhea(data, tables)
  local off = require_table(tables, "hhea")
  return {
    ascender       = read_i16(data, off + 4),
    descender      = read_i16(data, off + 6),
    line_gap       = read_i16(data, off + 8),
    num_h_metrics  = read_u16(data, off + 34),
  }
end

-- ────────────────────────────────────────────────────────────────────────────
-- maxp table  (numGlyphs at offset 4)
-- ────────────────────────────────────────────────────────────────────────────

local function parse_maxp(data, tables)
  local off = require_table(tables, "maxp")
  return read_u16(data, off + 4)
end

-- ────────────────────────────────────────────────────────────────────────────
-- cmap table — Format 4 BMP subtable
-- ────────────────────────────────────────────────────────────────────────────
--
-- cmap header: version(2) + numSubtables(2)
-- Encoding record (8 bytes each):
--   platformID  u16    3 = Windows
--   encodingID  u16    1 = Unicode BMP
--   offset      u32    relative to start of cmap table
--
-- Format 4 layout (relative to subtable start):
--   0   format        u16  = 4
--   2   length        u16
--   4   language      u16  (ignored)
--   6   segCountX2    u16
--   8   searchRange   u16  (ignored)
--  10   entrySelector u16  (ignored)
--  12   rangeShift    u16  (ignored)
--  14   endCode[n]         2n bytes
--  14+2n  reservedPad u16
--  16+2n  startCode[n]     2n bytes
--  16+4n  idDelta[n]       2n bytes  (signed)
--  16+6n  idRangeOffset[n] 2n bytes
--  16+8n  glyphIdArray[]   variable

local function parse_cmap(data, tables)
  local cmap_off = require_table(tables, "cmap")
  local num_subtables = read_u16(data, cmap_off + 2)

  -- Find the Format 4 BMP subtable (platform 3, encoding 1).
  local sub_off = nil
  for i = 0, num_subtables - 1 do
    local rec = cmap_off + 4 + i * 8
    local plat = read_u16(data, rec)
    local enc  = read_u16(data, rec + 2)
    local rel  = read_u32(data, rec + 4)
    if plat == 3 and enc == 1 then
      sub_off = cmap_off + rel
      break
    end
  end

  if not sub_off then
    raise("TableNotFound", "no cmap Format 4 subtable")
  end

  if read_u16(data, sub_off) ~= 4 then
    raise("ParseError", "expected cmap Format 4")
  end

  local seg_count = read_u16(data, sub_off + 6) // 2
  local end_codes_base        = sub_off + 14
  local start_codes_base      = sub_off + 16 + seg_count * 2
  local id_delta_base         = sub_off + 16 + seg_count * 4
  local id_range_offset_base  = sub_off + 16 + seg_count * 6

  -- Build a list of segments. Each segment table holds:
  --   end_code, start_code, id_delta, id_range_offset, iro_abs
  -- where iro_abs is the absolute byte address of idRangeOffset[i].
  local segments = {}
  for i = 0, seg_count - 1 do
    segments[i + 1] = {
      end_code         = read_u16(data, end_codes_base + i * 2),
      start_code       = read_u16(data, start_codes_base + i * 2),
      id_delta         = read_i16(data, id_delta_base + i * 2),
      id_range_offset  = read_u16(data, id_range_offset_base + i * 2),
      iro_abs          = id_range_offset_base + i * 2,
    }
  end

  return segments
end

-- cmap_lookup: scan segments for a codepoint and return its glyph index.
--
-- The idRangeOffset "self-relative pointer" trick:
--   If id_range_offset == 0: glyph = (cp + id_delta) & 0xFFFF
--   Otherwise:  abs_off = iro_abs + id_range_offset + (cp - start_code) * 2
--               glyph   = read_u16(data, abs_off)
local function cmap_lookup(segments, data, cp)
  for _, seg in ipairs(segments) do
    if cp <= seg.end_code then
      if cp < seg.start_code then
        return nil  -- cp falls in a gap between segments
      end
      local gid
      if seg.id_range_offset == 0 then
        gid = (cp + seg.id_delta) & 0xFFFF
      else
        local abs_off = seg.iro_abs + seg.id_range_offset + (cp - seg.start_code) * 2
        gid = read_u16(data, abs_off)
      end
      if gid == 0 then return nil end
      return gid
    end
  end
  return nil
end

-- ────────────────────────────────────────────────────────────────────────────
-- hmtx table
-- ────────────────────────────────────────────────────────────────────────────
--
-- numberOfHMetrics full records: advanceWidth(u16) + lsb(i16)
-- Glyphs ≥ numberOfHMetrics share the last advanceWidth.

local function hmtx_offset(data, tables)
  local off = require_table(tables, "hmtx")
  -- Probe the first byte to validate accessibility.
  read_u8(data, off)
  return off
end

local function lookup_glyph_metrics(font, gid)
  if gid < 0 or gid >= font.num_glyphs then
    return nil
  end

  local nhm = font.num_h_metrics
  local off = font.hmtx_off
  local data = font.raw

  -- Clamp to last full record for glyphs beyond numberOfHMetrics.
  local metric_idx = math.min(gid, nhm - 1)
  local advance = read_u16(data, off + metric_idx * 4)

  local lsb
  if gid < nhm then
    lsb = read_i16(data, off + gid * 4 + 2)
  else
    lsb = read_i16(data, off + nhm * 4 + (gid - nhm) * 2)
  end

  return {advance_width = advance, left_side_bearing = lsb}
end

-- ────────────────────────────────────────────────────────────────────────────
-- kern table — Format 0
-- ────────────────────────────────────────────────────────────────────────────
--
-- kern header: version(u16) + nTables(u16)
-- Subtable header (6 bytes): version(u16) + length(u16) + coverage(u16)
--   coverage HIGH byte = format (0 = sorted pairs)
--   coverage LOW bit 0 = horizontal direction
-- Format 0 data (+6): nPairs(u16) + 3× u16 (search helpers) + nPairs×{l,r,v}
--
-- We precompute a Lua table keyed by (left * 65536 + right) for O(1) lookup.

local function parse_kern(data, tables)
  local t = tables["kern"]
  if not t then return {} end

  local off = t.offset
  local n_tables = read_u16(data, off + 2)
  local kern_map = {}
  local cur = off + 4

  for _ = 1, n_tables do
    local sub_len  = read_u16(data, cur + 2)
    local coverage = read_u16(data, cur + 4)
    -- Format is in the HIGH byte of coverage (bits 8–15).
    local fmt = coverage >> 8

    if fmt == 0 then
      local n_pairs   = read_u16(data, cur + 6)
      local pairs_base = cur + 14  -- 6 (header) + 8 (Format 0 header)

      for j = 0, n_pairs - 1 do
        local poff  = pairs_base + j * 6
        local left  = read_u16(data, poff)
        local right = read_u16(data, poff + 2)
        local value = read_i16(data, poff + 4)
        kern_map[left * 65536 + right] = value
      end
    end

    cur = cur + sub_len
  end

  return kern_map
end

-- ────────────────────────────────────────────────────────────────────────────
-- name table
-- ────────────────────────────────────────────────────────────────────────────
--
-- name header: format(u16) + count(u16) + stringOffset(u16)
-- Name record (12 bytes):
--   platformID u16   3 = Windows
--   encodingID u16   1 = Unicode BMP
--   languageID u16   (any)
--   nameID     u16   1 = family  2 = subfamily
--   length     u16
--   offset     u16   relative to stringOffset

-- Decode a UTF-16 BE binary string to a Lua (UTF-8) string.
-- Each pair of bytes is one code unit; surrogate pairs are not handled
-- (most font names are pure ASCII) — non-ASCII characters are replaced
-- with a U+FFFD question box.
local function utf16be_decode(s)
  local out = {}
  for i = 1, #s - 1, 2 do
    local hi = string.byte(s, i)
    local lo = string.byte(s, i + 1)
    local cp = hi * 256 + lo
    -- Surrogate pairs: skip the low surrogate, emit replacement char.
    if cp >= 0xD800 and cp <= 0xDFFF then
      table.insert(out, "\xEF\xBF\xBD")  -- U+FFFD
    elseif cp < 0x80 then
      table.insert(out, string.char(cp))
    elseif cp < 0x800 then
      table.insert(out, string.char(
        0xC0 | (cp >> 6),
        0x80 | (cp & 0x3F)))
    else
      table.insert(out, string.char(
        0xE0 | (cp >> 12),
        0x80 | ((cp >> 6) & 0x3F),
        0x80 | (cp & 0x3F)))
    end
  end
  return table.concat(out)
end

local function parse_name(data, tables)
  local t = tables["name"]
  if not t then return "(unknown)", "(unknown)" end

  local tbl_off  = t.offset
  local count    = read_u16(data, tbl_off + 2)
  local str_base = tbl_off + read_u16(data, tbl_off + 4)

  local family, subfamily

  for i = 0, count - 1 do
    local rec = tbl_off + 6 + i * 12
    local plat = read_u16(data, rec)
    local enc  = read_u16(data, rec + 2)
    local nid  = read_u16(data, rec + 6)
    local nlen = read_u16(data, rec + 8)
    local noff = read_u16(data, rec + 10)

    if plat == 3 and enc == 1 then
      local raw_str = string.sub(data, str_base + noff + 1, str_base + noff + nlen)
      if nid == 1 and not family then
        family = utf16be_decode(raw_str)
      elseif nid == 2 and not subfamily then
        subfamily = utf16be_decode(raw_str)
      end
    end

    if family and subfamily then break end
  end

  return family or "(unknown)", subfamily or "(unknown)"
end

-- ────────────────────────────────────────────────────────────────────────────
-- OS/2 table
-- ────────────────────────────────────────────────────────────────────────────
--
-- version   u16   offset 0    ≥ 2 adds sxHeight / sCapHeight
-- ...
-- sxHeight  i16   offset 86   (version ≥ 2)
-- sCapHeight i16  offset 88

local function parse_os2(data, tables)
  local t = tables["OS/2"]
  if not t then return nil, nil end

  local off = t.offset
  local version = read_u16(data, off)

  if version >= 2 and t.length >= 90 then
    return read_i16(data, off + 86), read_i16(data, off + 88)
  end

  return nil, nil
end

-- ────────────────────────────────────────────────────────────────────────────
-- Public API
-- ────────────────────────────────────────────────────────────────────────────

--- Load a font binary and return an opaque FontFile table.
--
-- Raises a table `{kind=..., message=...}` on failure. Recognised `kind`
-- values: `"BufferTooShort"`, `"InvalidMagic"`, `"TableNotFound"`,
-- `"ParseError"`.
--
-- @param data string  binary font data (result of `io.open(...,"rb"):read("*a")`)
-- @return table  opaque FontFile
function M.load(data)
  local ok, result = pcall(function()
    local tables = parse_offset_table(data)

    local head_d = parse_head(data, tables)
    local hhea_d = parse_hhea(data, tables)
    local num_glyphs = parse_maxp(data, tables)
    local cmap_segs  = parse_cmap(data, tables)
    local hmtx_off   = hmtx_offset(data, tables)

    local kern_map = parse_kern(data, tables)
    local family, subfamily = parse_name(data, tables)
    local x_height, cap_height = parse_os2(data, tables)

    local metrics = {
      units_per_em  = head_d.units_per_em,
      ascender      = hhea_d.ascender,
      descender     = hhea_d.descender,
      line_gap      = hhea_d.line_gap,
      x_height      = x_height,
      cap_height    = cap_height,
      num_glyphs    = num_glyphs,
      family_name   = family,
      subfamily_name = subfamily,
    }

    return {
      _type          = "FontFile",
      raw            = data,
      metrics        = metrics,
      cmap_segments  = cmap_segs,
      num_h_metrics  = hhea_d.num_h_metrics,
      num_glyphs     = num_glyphs,
      hmtx_off       = hmtx_off,
      kern_map       = kern_map,
    }
  end)

  if ok then
    return result
  else
    -- Propagate FontError tables; wrap anything else as ParseError.
    if type(result) == "table" and result.kind then
      error(result, 0)
    else
      error(font_error("ParseError", tostring(result)), 0)
    end
  end
end

--- Return the FontMetrics table for a loaded font.
--
-- Fields: units_per_em, ascender, descender, line_gap, x_height (nil if
-- absent), cap_height (nil if absent), num_glyphs, family_name,
-- subfamily_name.
--
-- @param font table  FontFile returned by load()
-- @return table  FontMetrics
function M.font_metrics(font)
  return font.metrics
end

--- Map a Unicode codepoint to a glyph index.
--
-- Returns nil for codepoints outside the BMP (> 0xFFFF), negative values,
-- or codepoints not present in the font.
--
-- @param font       table    FontFile
-- @param codepoint  integer  Unicode codepoint
-- @return integer|nil  glyph index
function M.glyph_id(font, codepoint)
  if type(codepoint) ~= "number" or codepoint < 0 or codepoint > 0xFFFF then
    return nil
  end
  return cmap_lookup(font.cmap_segments, font.raw, math.floor(codepoint))
end

--- Return per-glyph horizontal metrics.
--
-- Returns nil for out-of-range or negative glyph IDs.
--
-- @param font      table    FontFile
-- @param glyph_id  integer  glyph index
-- @return table|nil  {advance_width, left_side_bearing}
function M.glyph_metrics(font, glyph_id_param)
  if type(glyph_id_param) ~= "number" or glyph_id_param < 0 then
    return nil
  end
  return lookup_glyph_metrics(font, math.floor(glyph_id_param))
end

--- Return the kern value (font units) for the ordered glyph pair.
--
-- Returns 0 when no kern table exists or the pair is not listed.
-- Note: many modern fonts (e.g. Inter v4.0) use GPOS and have no kern table.
--
-- @param font   table    FontFile
-- @param left   integer  left glyph index
-- @param right  integer  right glyph index
-- @return integer  kern value (may be negative)
function M.kerning(font, left, right)
  if type(left) ~= "number" or type(right) ~= "number" then
    return 0
  end
  return font.kern_map[left * 65536 + right] or 0
end

return M
