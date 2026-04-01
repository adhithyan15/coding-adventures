defmodule CodingAdventures.FontParserTest do
  use ExUnit.Case, async: true

  alias CodingAdventures.FontParser
  alias CodingAdventures.FontParser.FontError
  alias CodingAdventures.FontParser.FontFile
  alias CodingAdventures.FontParser.FontMetrics
  alias CodingAdventures.FontParser.GlyphMetrics

  # ──────────────────────────────────────────────────────────────────────────
  # Test helpers
  # ──────────────────────────────────────────────────────────────────────────

  @font_fixture Path.expand(
                  "../../../../fixtures/fonts/Inter-Regular.ttf",
                  __DIR__
                )

  defp inter_bytes, do: File.read!(@font_fixture)

  # build_synthetic_font/1 constructs a minimal valid OpenType binary with a
  # kern Format 0 table containing the given pairs.
  #
  # Tables present: head, hhea, maxp, cmap (Format 4 sentinel), hmtx, kern
  # (sorted alphabetically as required by the OpenType spec).
  #
  # This lets us exercise kern logic independently of Inter (which uses GPOS).
  defp build_synthetic_font(pairs) do
    num_tables = 6
    dir_size = 12 + num_tables * 16

    head_len = 54
    hhea_len = 36
    maxp_len = 6
    cmap_len = 36
    hmtx_len = 5 * 4
    n_pairs = length(pairs)
    kern_len = 4 + 6 + 8 + n_pairs * 6

    head_off = dir_size
    hhea_off = head_off + head_len
    maxp_off = hhea_off + hhea_len
    cmap_off = maxp_off + maxp_len
    hmtx_off = cmap_off + cmap_len
    kern_off = hmtx_off + hmtx_len

    # Offset table
    buf = <<0x00010000::32, num_tables::16, 64::16, 2::16, 32::16>>

    # Table records (sorted: cmap < head < hhea < hmtx < kern < maxp)
    records =
      [
        {"cmap", cmap_off, cmap_len},
        {"head", head_off, head_len},
        {"hhea", hhea_off, hhea_len},
        {"hmtx", hmtx_off, hmtx_len},
        {"kern", kern_off, kern_len},
        {"maxp", maxp_off, maxp_len}
      ]
      |> Enum.map(fn {tag, off, len} ->
        padded_tag = String.pad_trailing(tag, 4, "\x00")
        <<padded_tag::binary-4, 0::32, off::32, len::32>>
      end)
      |> Enum.join()

    buf = buf <> records

    # head table (54 bytes total)
    # majorVersion(2) + minorVersion(2) + fontRevision(4) + checkSumAdj(4) +
    # magicNumber(4) + flags(2) + unitsPerEm(2) + created(8) + modified(8) +
    # xMin(2) + yMin(2) + xMax(2) + yMax(2) + macStyle(2) + lowestRecPPEM(2) +
    # fontDirectionHint(2) + indexToLocFormat(2) + glyphDataFormat(2) = 54 bytes
    buf =
      buf <>
        <<
          0x00010000::32,
          0x00010000::32,
          0::32,
          0x5F0F3CF5::32,
          0::16,
          1000::16,
          0::128,
          0::signed-big-16,
          0::signed-big-16,
          0::signed-big-16,
          0::signed-big-16,
          0::16,
          8::16,
          2::signed-big-16,
          0::signed-big-16,
          0::signed-big-16
        >>

    # hhea table
    buf =
      buf <>
        <<
          0x00010000::32,
          800::signed-big-16,
          -200::signed-big-16,
          0::signed-big-16,
          1000::16,
          0::signed-big-16,
          0::signed-big-16,
          0::signed-big-16,
          1::signed-big-16,
          0::signed-big-16,
          0::signed-big-16,
          0::64,
          0::signed-big-16,
          5::16
        >>

    # maxp table
    buf = buf <> <<0x00005000::32, 5::16>>

    # cmap table: version=0, numSubtables=1, then one encoding record
    # pointing to a Format 4 subtable with 1 segment (0xFFFF terminator).
    buf =
      buf <>
        <<
          # cmap header
          0::16,
          1::16,
          # encoding record: plat 3, enc 1, offset 12
          3::16,
          1::16,
          12::32,
          # Format 4 subtable with 1 segment (end=0xFFFF, start=0xFFFF → no coverage)
          4::16,
          24::16,
          0::16,
          # segCountX2 = 2 (1 segment)
          2::16,
          2::16,
          0::16,
          0::16,
          # endCode[0] = 0xFFFF
          0xFFFF::16,
          # reservedPad
          0::16,
          # startCode[0] = 0xFFFF
          0xFFFF::16,
          # idDelta[0] = 1
          1::signed-big-16,
          # idRangeOffset[0] = 0
          0::16
        >>

    # hmtx: 5 records of {600, 50}
    hmtx = for _ <- 1..5, do: <<600::16, 50::signed-big-16>>
    buf = buf <> Enum.join(hmtx)

    # kern table
    sub_len = 6 + 8 + n_pairs * 6

    sorted_pairs =
      pairs
      |> Enum.sort_by(fn {l, r, _} -> l * 65_536 + r end)

    pairs_bin =
      sorted_pairs
      |> Enum.map(fn {l, r, v} -> <<l::16, r::16, v::signed-big-16>> end)
      |> Enum.join()

    buf =
      buf <>
        <<
          # kern header
          0::16,
          1::16,
          # subtable header
          0::16,
          sub_len::16,
          # coverage: format 0, horizontal
          0x0001::16,
          # Format 0 header
          n_pairs::16,
          0::16,
          0::16,
          0::16
        >> <> pairs_bin

    buf
  end

  # ──────────────────────────────────────────────────────────────────────────
  # Tests: load
  # ──────────────────────────────────────────────────────────────────────────

  describe "load/1" do
    test "empty binary raises BufferTooShort" do
      assert_raise FontError, fn -> FontParser.load("") end
      err = catch_error(FontParser.load(""))
      assert err.kind == "BufferTooShort"
    end

    test "wrong magic raises InvalidMagic" do
      buf = <<0xDEADBEEF::32>> <> :binary.copy(<<0>>, 252)
      err = catch_error(FontParser.load(buf))
      assert err.kind == "InvalidMagic"
    end

    test "valid sfnt magic with no tables raises TableNotFound" do
      buf = <<0x00010000::32, 0::16, 0::48>>
      err = catch_error(FontParser.load(buf))
      assert err.kind == "TableNotFound"
    end

    test "Inter Regular loads successfully" do
      font = FontParser.load(inter_bytes())
      assert %FontFile{} = font
    end

    test "synthetic font loads successfully" do
      font = FontParser.load(build_synthetic_font([{1, 2, -140}]))
      assert is_struct(font)
    end
  end

  # ──────────────────────────────────────────────────────────────────────────
  # Tests: font_metrics
  # ──────────────────────────────────────────────────────────────────────────

  describe "font_metrics/1" do
    setup do
      {:ok, font: FontParser.load(inter_bytes())}
    end

    test "units_per_em is 2048", %{font: font} do
      assert FontParser.font_metrics(font).units_per_em == 2048
    end

    test "family_name is Inter", %{font: font} do
      assert FontParser.font_metrics(font).family_name == "Inter"
    end

    test "subfamily_name is Regular", %{font: font} do
      assert FontParser.font_metrics(font).subfamily_name == "Regular"
    end

    test "ascender is positive", %{font: font} do
      assert FontParser.font_metrics(font).ascender > 0
    end

    test "descender is non-positive", %{font: font} do
      assert FontParser.font_metrics(font).descender <= 0
    end

    test "num_glyphs is large", %{font: font} do
      assert FontParser.font_metrics(font).num_glyphs > 100
    end

    test "x_height is positive", %{font: font} do
      m = FontParser.font_metrics(font)
      assert m.x_height != nil
      assert m.x_height > 0
    end

    test "cap_height is positive", %{font: font} do
      m = FontParser.font_metrics(font)
      assert m.cap_height != nil
      assert m.cap_height > 0
    end

    test "returns FontMetrics struct", %{font: font} do
      assert %FontMetrics{} = FontParser.font_metrics(font)
    end

    test "synthetic font has units_per_em 1000" do
      font = FontParser.load(build_synthetic_font([]))
      assert FontParser.font_metrics(font).units_per_em == 1000
    end

    test "synthetic font family name is unknown" do
      font = FontParser.load(build_synthetic_font([]))
      assert FontParser.font_metrics(font).family_name == "(unknown)"
    end
  end

  # ──────────────────────────────────────────────────────────────────────────
  # Tests: glyph_id
  # ──────────────────────────────────────────────────────────────────────────

  describe "glyph_id/2" do
    setup do
      {:ok, font: FontParser.load(inter_bytes())}
    end

    test "glyph_id for 'A' is non-nil", %{font: font} do
      refute is_nil(FontParser.glyph_id(font, 0x0041))
    end

    test "glyph_id for 'V' is non-nil", %{font: font} do
      refute is_nil(FontParser.glyph_id(font, 0x0056))
    end

    test "glyph_id for space is non-nil", %{font: font} do
      refute is_nil(FontParser.glyph_id(font, 0x0020))
    end

    test "glyph_ids for A and V differ", %{font: font} do
      gid_a = FontParser.glyph_id(font, 0x0041)
      gid_v = FontParser.glyph_id(font, 0x0056)
      refute gid_a == gid_v
    end

    test "codepoint above 0xFFFF returns nil", %{font: font} do
      assert is_nil(FontParser.glyph_id(font, 0x10000))
    end

    test "negative codepoint returns nil", %{font: font} do
      assert is_nil(FontParser.glyph_id(font, -1))
    end
  end

  # ──────────────────────────────────────────────────────────────────────────
  # Tests: glyph_metrics
  # ──────────────────────────────────────────────────────────────────────────

  describe "glyph_metrics/2" do
    setup do
      {:ok, font: FontParser.load(inter_bytes())}
    end

    test "advance_width for 'A' is positive", %{font: font} do
      gid = FontParser.glyph_id(font, 0x0041)
      gm = FontParser.glyph_metrics(font, gid)
      refute is_nil(gm)
      assert gm.advance_width > 0
    end

    test "advance_width for 'A' is in reasonable range", %{font: font} do
      gid = FontParser.glyph_id(font, 0x0041)
      gm = FontParser.glyph_metrics(font, gid)
      assert gm.advance_width >= 100 and gm.advance_width <= 2400
    end

    test "returns GlyphMetrics struct", %{font: font} do
      gid = FontParser.glyph_id(font, 0x0041)
      assert %GlyphMetrics{} = FontParser.glyph_metrics(font, gid)
    end

    test "out-of-range glyph returns nil", %{font: font} do
      m = FontParser.font_metrics(font)
      assert is_nil(FontParser.glyph_metrics(font, m.num_glyphs))
    end

    test "negative glyph id returns nil", %{font: font} do
      assert is_nil(FontParser.glyph_metrics(font, -1))
    end
  end

  # ──────────────────────────────────────────────────────────────────────────
  # Tests: kerning
  # ──────────────────────────────────────────────────────────────────────────

  describe "kerning/3" do
    test "Inter A+V returns 0 (Inter uses GPOS not kern table)" do
      font = FontParser.load(inter_bytes())
      gid_a = FontParser.glyph_id(font, 0x0041)
      gid_v = FontParser.glyph_id(font, 0x0056)
      assert FontParser.kerning(font, gid_a, gid_v) == 0
    end

    test "synthetic pair (1,2) returns -140" do
      font = FontParser.load(build_synthetic_font([{1, 2, -140}, {3, 4, 80}]))
      assert FontParser.kerning(font, 1, 2) == -140
    end

    test "synthetic pair (3,4) returns 80" do
      font = FontParser.load(build_synthetic_font([{1, 2, -140}, {3, 4, 80}]))
      assert FontParser.kerning(font, 3, 4) == 80
    end

    test "absent pair returns 0" do
      font = FontParser.load(build_synthetic_font([{1, 2, -140}, {3, 4, 80}]))
      assert FontParser.kerning(font, 1, 4) == 0
    end

    test "reversed pair returns 0" do
      font = FontParser.load(build_synthetic_font([{1, 2, -140}]))
      assert FontParser.kerning(font, 2, 1) == 0
    end
  end
end
