# frozen_string_literal: true

require "minitest/autorun"
require "coding_adventures/font_parser"

# ─────────────────────────────────────────────────────────────────────────────
# Helpers
# ─────────────────────────────────────────────────────────────────────────────

FONT_FIXTURE = File.expand_path(
  "../../../../fixtures/fonts/Inter-Regular.ttf",
  __dir__  # test/
  # → test/../../../../fixtures = code/fixtures
)

def inter_bytes
  File.binread(FONT_FIXTURE)
end

# Build a minimal valid synthetic font with a kern Format 0 table.
# Tables: head, hhea, maxp, cmap (Format 4 sentinel), hmtx, kern.
def build_synthetic_font(pairs)
  # Helper lambdas — Ruby's Array#pack handles big-endian encoding.
  w16 = ->(v) { [v & 0xFFFF].pack("n") }
  wi16 = ->(v) { [v].pack("s>") }
  w32 = ->(v) { [v & 0xFFFFFFFF].pack("N") }
  tag = ->(s) { s.b[0, 4].ljust(4, "\x00") }

  num_tables = 6
  dir_size = 12 + num_tables * 16
  head_len = 54
  hhea_len = 36
  maxp_len = 6
  cmap_len = 36
  hmtx_len = 5 * 4
  n_pairs = pairs.size
  kern_len = 4 + 6 + 8 + n_pairs * 6

  head_off = dir_size
  hhea_off = head_off + head_len
  maxp_off = hhea_off + hhea_len
  cmap_off = maxp_off + maxp_len
  hmtx_off = cmap_off + cmap_len
  kern_off = hmtx_off + hmtx_len

  buf = String.new("", encoding: Encoding::BINARY)

  # Offset Table
  buf << w32.call(0x00010000) << w16.call(num_tables) << w16.call(64) << w16.call(2) << w16.call(32)

  # Table Records (sorted: cmap < head < hhea < hmtx < kern < maxp)
  [
    ["cmap", cmap_off, cmap_len],
    ["head", head_off, head_len],
    ["hhea", hhea_off, hhea_len],
    ["hmtx", hmtx_off, hmtx_len],
    ["kern", kern_off, kern_len],
    ["maxp", maxp_off, maxp_len]
  ].each do |(t, off, len)|
    buf << tag.call(t) << w32.call(0) << w32.call(off) << w32.call(len)
  end

  # head
  buf << w32.call(0x00010000) << w32.call(0x00010000) << w32.call(0) << w32.call(0x5F0F3CF5)
  buf << w16.call(0) << w16.call(1000) << ("\x00" * 16) # flags, unitsPerEm, created, modified
  buf << wi16.call(0) * 4  # xMin yMin xMax yMax
  buf << w16.call(0) << w16.call(8) << wi16.call(2) << wi16.call(0) << wi16.call(0)

  # hhea
  buf << w32.call(0x00010000) << wi16.call(800) << wi16.call(-200) << wi16.call(0)
  buf << w16.call(1000)       # advanceWidthMax
  buf << wi16.call(0) * 3     # minLSB, minRSB, xMaxExtent
  buf << wi16.call(1) << wi16.call(0) << wi16.call(0)  # caretSlopeRise, Run, Offset
  buf << wi16.call(0) * 4     # reserved
  buf << wi16.call(0) << w16.call(5)  # metricDataFormat, numberOfHMetrics

  # maxp
  buf << w32.call(0x00005000) << w16.call(5)

  # cmap
  buf << w16.call(0) << w16.call(1)           # version, numSubtables
  buf << w16.call(3) << w16.call(1) << w32.call(12)  # enc record: plat 3, enc 1, off 12
  buf << w16.call(4) << w16.call(24) << w16.call(0) << w16.call(2) << w16.call(2) << w16.call(0) << w16.call(0)
  buf << w16.call(0xFFFF) << w16.call(0) << w16.call(0xFFFF) << wi16.call(1) << w16.call(0)

  # hmtx: 5 full records
  5.times { buf << w16.call(600) << wi16.call(50) }

  # kern
  buf << w16.call(0) << w16.call(1)           # version, nTables
  sub_len = 6 + 8 + n_pairs * 6
  buf << w16.call(0) << w16.call(sub_len) << w16.call(0x0001)  # subtable: version, length, coverage
  buf << w16.call(n_pairs) << w16.call(0) << w16.call(0) << w16.call(0)
  sorted = pairs.sort_by { |l, r, _| (l << 16) | r }
  sorted.each { |l, r, v| buf << w16.call(l) << w16.call(r) << wi16.call(v) }

  buf
end

# ─────────────────────────────────────────────────────────────────────────────
# Tests: load
# ─────────────────────────────────────────────────────────────────────────────

class TestLoad < Minitest::Test
  def test_empty_buffer_raises_buffer_too_short
    err = assert_raises(CodingAdventures::FontParser::FontError) { CodingAdventures::FontParser.load("") }
    assert_equal "BufferTooShort", err.kind
  end

  def test_wrong_magic_raises_invalid_magic
    buf = "\x00" * 256
    buf.setbyte(0, 0xDE)
    buf.setbyte(1, 0xAD)
    buf.setbyte(2, 0xBE)
    buf.setbyte(3, 0xEF)
    err = assert_raises(CodingAdventures::FontParser::FontError) { CodingAdventures::FontParser.load(buf) }
    assert_equal "InvalidMagic", err.kind
  end

  def test_load_inter_regular_succeeds
    font = CodingAdventures::FontParser.load(inter_bytes)
    refute_nil font
  end

  def test_load_synthetic_font_succeeds
    font = CodingAdventures::FontParser.load(build_synthetic_font([[1, 2, -140]]))
    refute_nil font
  end

  def test_missing_table_raises_table_not_found
    # Build a buffer with valid sfntVersion but numTables=0 → no head table.
    buf = "\x00\x01\x00\x00" + "\x00\x00" + "\x00" * 6
    err = assert_raises(CodingAdventures::FontParser::FontError) { CodingAdventures::FontParser.load(buf) }
    assert_equal "TableNotFound", err.kind
  end
end

# ─────────────────────────────────────────────────────────────────────────────
# Tests: font_metrics
# ─────────────────────────────────────────────────────────────────────────────

class TestFontMetrics < Minitest::Test
  def setup
    @font = CodingAdventures::FontParser.load(inter_bytes)
  end

  def test_units_per_em_is_2048
    assert_equal 2048, CodingAdventures::FontParser.font_metrics(@font).units_per_em
  end

  def test_family_name_is_inter
    assert_equal "Inter", CodingAdventures::FontParser.font_metrics(@font).family_name
  end

  def test_subfamily_name_is_regular
    assert_equal "Regular", CodingAdventures::FontParser.font_metrics(@font).subfamily_name
  end

  def test_ascender_is_positive
    assert_operator CodingAdventures::FontParser.font_metrics(@font).ascender, :>, 0
  end

  def test_descender_is_non_positive
    assert_operator CodingAdventures::FontParser.font_metrics(@font).descender, :<=, 0
  end

  def test_num_glyphs_is_large
    assert_operator CodingAdventures::FontParser.font_metrics(@font).num_glyphs, :>, 100
  end

  def test_x_height_is_positive
    m = CodingAdventures::FontParser.font_metrics(@font)
    refute_nil m.x_height
    assert_operator m.x_height, :>, 0
  end

  def test_cap_height_is_positive
    m = CodingAdventures::FontParser.font_metrics(@font)
    refute_nil m.cap_height
    assert_operator m.cap_height, :>, 0
  end

  def test_synthetic_font_unknown_family
    font = CodingAdventures::FontParser.load(build_synthetic_font([]))
    assert_equal "(unknown)", CodingAdventures::FontParser.font_metrics(font).family_name
  end

  def test_synthetic_font_units_per_em
    font = CodingAdventures::FontParser.load(build_synthetic_font([]))
    assert_equal 1000, CodingAdventures::FontParser.font_metrics(font).units_per_em
  end
end

# ─────────────────────────────────────────────────────────────────────────────
# Tests: glyph_id
# ─────────────────────────────────────────────────────────────────────────────

class TestGlyphId < Minitest::Test
  def setup
    @font = CodingAdventures::FontParser.load(inter_bytes)
  end

  def test_glyph_id_for_a_is_not_nil
    refute_nil CodingAdventures::FontParser.glyph_id(@font, 0x0041)
  end

  def test_glyph_id_for_v_is_not_nil
    refute_nil CodingAdventures::FontParser.glyph_id(@font, 0x0056)
  end

  def test_glyph_id_for_space_is_not_nil
    refute_nil CodingAdventures::FontParser.glyph_id(@font, 0x0020)
  end

  def test_glyph_ids_for_a_and_v_differ
    gid_a = CodingAdventures::FontParser.glyph_id(@font, 0x0041)
    gid_v = CodingAdventures::FontParser.glyph_id(@font, 0x0056)
    refute_equal gid_a, gid_v
  end

  def test_codepoint_above_ffff_returns_nil
    assert_nil CodingAdventures::FontParser.glyph_id(@font, 0x10000)
  end

  def test_negative_codepoint_returns_nil
    assert_nil CodingAdventures::FontParser.glyph_id(@font, -1)
  end
end

# ─────────────────────────────────────────────────────────────────────────────
# Tests: glyph_metrics
# ─────────────────────────────────────────────────────────────────────────────

class TestGlyphMetrics < Minitest::Test
  def setup
    @font = CodingAdventures::FontParser.load(inter_bytes)
  end

  def test_advance_width_for_a_is_positive
    gid = CodingAdventures::FontParser.glyph_id(@font, 0x0041)
    gm = CodingAdventures::FontParser.glyph_metrics(@font, gid)
    refute_nil gm
    assert_operator gm.advance_width, :>, 0
  end

  def test_advance_width_in_reasonable_range
    gid = CodingAdventures::FontParser.glyph_id(@font, 0x0041)
    gm = CodingAdventures::FontParser.glyph_metrics(@font, gid)
    assert gm.advance_width.between?(100, 2400)
  end

  def test_out_of_range_glyph_returns_nil
    m = CodingAdventures::FontParser.font_metrics(@font)
    assert_nil CodingAdventures::FontParser.glyph_metrics(@font, m.num_glyphs)
  end

  def test_negative_glyph_id_returns_nil
    assert_nil CodingAdventures::FontParser.glyph_metrics(@font, -1)
  end
end

# ─────────────────────────────────────────────────────────────────────────────
# Tests: kerning
# ─────────────────────────────────────────────────────────────────────────────

class TestKerning < Minitest::Test
  def test_inter_no_kern_table_returns_zero
    font = CodingAdventures::FontParser.load(inter_bytes)
    gid_a = CodingAdventures::FontParser.glyph_id(font, 0x0041)
    gid_v = CodingAdventures::FontParser.glyph_id(font, 0x0056)
    assert_equal 0, CodingAdventures::FontParser.kerning(font, gid_a, gid_v)
  end

  def test_synthetic_pair_1_2_negative
    font = CodingAdventures::FontParser.load(build_synthetic_font([[1, 2, -140], [3, 4, 80]]))
    assert_equal(-140, CodingAdventures::FontParser.kerning(font, 1, 2))
  end

  def test_synthetic_pair_3_4_positive
    font = CodingAdventures::FontParser.load(build_synthetic_font([[1, 2, -140], [3, 4, 80]]))
    assert_equal 80, CodingAdventures::FontParser.kerning(font, 3, 4)
  end

  def test_absent_pair_returns_zero
    font = CodingAdventures::FontParser.load(build_synthetic_font([[1, 2, -140], [3, 4, 80]]))
    assert_equal 0, CodingAdventures::FontParser.kerning(font, 1, 4)
  end

  def test_reversed_pair_returns_zero
    font = CodingAdventures::FontParser.load(build_synthetic_font([[1, 2, -140]]))
    assert_equal 0, CodingAdventures::FontParser.kerning(font, 2, 1)
  end
end
