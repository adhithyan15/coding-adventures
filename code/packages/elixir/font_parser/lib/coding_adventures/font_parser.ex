defmodule CodingAdventures.FontParser do
  @moduledoc """
  Metrics-only OpenType/TrueType font parser. Zero runtime dependencies.

  OpenType and TrueType font files are structured as a *table database*: a
  short offset table at byte 0 tells you how many named tables exist, and an
  array of 16-byte table records (tag + checksum + offset + length) lets you
  jump directly to any table by name.

  This module parses the eight tables that provide the data a layout engine
  needs:

  | Table | Contents |
  |-------|----------|
  | `head` | unitsPerEm |
  | `hhea` | ascender, descender, lineGap, numberOfHMetrics |
  | `maxp` | numGlyphs |
  | `cmap` | Format 4 Unicode BMP → glyph-index mapping |
  | `hmtx` | advance width + left-side bearing per glyph |
  | `kern` | Format 0 sorted kerning pairs (optional) |
  | `name` | family / subfamily names, UTF-16 BE (optional) |
  | `OS/2` | x-height, cap-height for version ≥ 2 (optional) |

  All multi-byte integers in the font binary are **big-endian** — in Elixir
  binary patterns `<<v::unsigned-big-16>>` and `<<v::unsigned-16>>` are
  equivalent since big-endian is the default, but we spell it out explicitly.

  ## Usage

      iex> bytes = File.read!("Inter-Regular.ttf")
      iex> font  = CodingAdventures.FontParser.load(bytes)
      iex> m     = CodingAdventures.FontParser.font_metrics(font)
      iex> m.units_per_em
      2048

  """

  # ──────────────────────────────────────────────────────────────────────────
  # Public types
  # ──────────────────────────────────────────────────────────────────────────

  import Bitwise

  defmodule FontError do
    @moduledoc """
    Exception raised when a font binary cannot be parsed.

    Check `kind` for the machine-readable category:

    * `"BufferTooShort"` — binary too short to be a valid font
    * `"InvalidMagic"` — unrecognised sfntVersion magic bytes
    * `"TableNotFound"` — a required table is absent
    * `"ParseError"` — a table is structurally invalid
    """
    defexception [:message, :kind]

    @impl true
    def exception(opts) do
      kind = Keyword.get(opts, :kind, "ParseError")
      msg = Keyword.get(opts, :message, kind)
      %FontError{kind: kind, message: msg}
    end
  end

  defmodule FontMetrics do
    @moduledoc "Global metrics for a loaded font."

    @type t :: %__MODULE__{
            units_per_em: non_neg_integer(),
            ascender: integer(),
            descender: integer(),
            line_gap: integer(),
            x_height: integer() | nil,
            cap_height: integer() | nil,
            num_glyphs: non_neg_integer(),
            family_name: String.t(),
            subfamily_name: String.t()
          }

    defstruct [
      :units_per_em,
      :ascender,
      :descender,
      :line_gap,
      :x_height,
      :cap_height,
      :num_glyphs,
      :family_name,
      :subfamily_name
    ]
  end

  defmodule GlyphMetrics do
    @moduledoc "Per-glyph horizontal metrics from the `hmtx` table."

    @type t :: %__MODULE__{
            advance_width: non_neg_integer(),
            left_side_bearing: integer()
          }

    defstruct [:advance_width, :left_side_bearing]
  end

  # ──────────────────────────────────────────────────────────────────────────
  # Internal representation (opaque to callers)
  # ──────────────────────────────────────────────────────────────────────────

  # After parsing we keep a compact FontFile struct:
  #
  # * metrics         — pre-built FontMetrics
  # * cmap_segments   — list of {end_code, start_code, id_delta,
  #                     id_range_offset, iro_abs_off} for cmap lookups
  # * num_h_metrics   — how many full {advanceWidth, lsb} records exist
  # * num_glyphs      — total glyph count (for bounds checking)
  # * hmtx_abs_off    — absolute byte offset of the hmtx table in `raw`
  # * raw             — the original font binary (kept for indirect cmap +
  #                     hmtx reads)
  # * kern_map        — %{composite_key => value} for O(1) kern lookups

  defmodule FontFile do
    @moduledoc false
    defstruct [
      :metrics,
      :cmap_segments,
      :num_h_metrics,
      :num_glyphs,
      :hmtx_abs_off,
      :raw,
      :kern_map
    ]
  end

  # ──────────────────────────────────────────────────────────────────────────
  # Public API
  # ──────────────────────────────────────────────────────────────────────────

  @doc """
  Parse a font binary and return a `FontFile`.

  Raises `FontError` if the binary is invalid or a required table is missing.
  """
  @spec load(binary()) :: FontFile.t()
  def load(data) when is_binary(data) do
    # Wrap the whole parse so that out-of-bounds binary_part/3 calls
    # (which raise ArgumentError) are converted to FontError{ParseError}.
    try do
      do_load(data)
    rescue
      e in FontError ->
        reraise e, __STACKTRACE__

      _other ->
        raise FontError, kind: "ParseError"
    end
  end

  @doc "Return the `FontMetrics` for a loaded font."
  @spec font_metrics(FontFile.t()) :: FontMetrics.t()
  def font_metrics(%FontFile{metrics: m}), do: m

  @doc """
  Map a Unicode codepoint to a glyph index via the `cmap` Format 4 subtable.

  Returns `nil` for codepoints outside the BMP (`> 0xFFFF`), negative values,
  or codepoints not present in the font.
  """
  @spec glyph_id(FontFile.t(), integer()) :: non_neg_integer() | nil
  def glyph_id(%FontFile{cmap_segments: segs, raw: raw}, cp)
      when is_integer(cp) and cp >= 0 and cp <= 0xFFFF do
    cmap_lookup(segs, raw, cp)
  end

  def glyph_id(_, _), do: nil

  @doc """
  Return `GlyphMetrics` for the given glyph index, or `nil` if out of range.
  """
  @spec glyph_metrics(FontFile.t(), integer()) :: GlyphMetrics.t() | nil
  def glyph_metrics(%FontFile{} = font, gid)
      when is_integer(gid) and gid >= 0 do
    lookup_glyph_metrics(font, gid)
  end

  def glyph_metrics(_, _), do: nil

  @doc """
  Return the kern adjustment (in font units) for the ordered glyph pair, or
  `0` if the kern table is absent or the pair is not listed.

  Note: many modern fonts (including Inter v4.0) use GPOS for kerning and
  have no legacy `kern` table. This function only reads `kern` Format 0.
  """
  @spec kerning(FontFile.t(), non_neg_integer(), non_neg_integer()) :: integer()
  def kerning(%FontFile{kern_map: kern_map}, left, right)
      when is_integer(left) and is_integer(right) and left >= 0 and right >= 0 do
    Map.get(kern_map, left * 65_536 + right, 0)
  end

  def kerning(_, _, _), do: 0

  # ──────────────────────────────────────────────────────────────────────────
  # Top-level parser
  # ──────────────────────────────────────────────────────────────────────────

  defp do_load(data) do
    # ── Offset table ────────────────────────────────────────────────────────
    #
    # Bytes 0–11 of every OpenType/TrueType file:
    #
    #   sfntVersion  u32   0x00010000 = TrueType  /  0x4F54544F = "OTTO" (CFF)
    #   numTables    u16
    #   searchRange  u16   (ignored)
    #   entrySelector u16  (ignored)
    #   rangeShift   u16   (ignored)
    if byte_size(data) < 12 do
      raise FontError, kind: "BufferTooShort"
    end

    sfnt_ver = ru32(data, 0)

    unless sfnt_ver in [0x00010000, 0x4F54_544F] do
      raise FontError, kind: "InvalidMagic"
    end

    num_tables = ru16(data, 4)
    tables = parse_table_records(data, num_tables)

    # ── Required tables ──────────────────────────────────────────────────────
    head_data = parse_head(data, tables)
    hhea_data = parse_hhea(data, tables)
    num_glyphs = parse_maxp(data, tables)
    cmap_segments = parse_cmap(data, tables)
    hmtx_abs_off = parse_hmtx_offset(data, tables)

    # ── Optional tables ──────────────────────────────────────────────────────
    kern_map = parse_kern(data, tables)
    {family_name, subfamily_name} = parse_name(data, tables)
    {x_height, cap_height} = parse_os2(data, tables)

    metrics = %FontMetrics{
      units_per_em: head_data.units_per_em,
      ascender: hhea_data.ascender,
      descender: hhea_data.descender,
      line_gap: hhea_data.line_gap,
      x_height: x_height,
      cap_height: cap_height,
      num_glyphs: num_glyphs,
      family_name: family_name,
      subfamily_name: subfamily_name
    }

    %FontFile{
      metrics: metrics,
      cmap_segments: cmap_segments,
      num_h_metrics: hhea_data.num_h_metrics,
      num_glyphs: num_glyphs,
      hmtx_abs_off: hmtx_abs_off,
      raw: data,
      kern_map: kern_map
    }
  end

  # ──────────────────────────────────────────────────────────────────────────
  # Table directory
  # ──────────────────────────────────────────────────────────────────────────

  # The table record array starts at byte 12.
  # Each record is 16 bytes: tag(4) + checksum(4) + offset(4) + length(4).
  # We build a map of tag string → {offset, length}.

  defp parse_table_records(data, num_tables) do
    # Use explicit step //1 so that a zero-length range (num_tables=0)
    # is empty rather than counting down.
    Enum.reduce(0..(num_tables - 1)//1, %{}, fn i, acc ->
      base = 12 + i * 16
      tag = binary_part(data, base, 4)
      off = ru32(data, base + 8)
      len = ru32(data, base + 12)
      Map.put(acc, tag, {off, len})
    end)
  end

  defp require_table(tables, tag) do
    case Map.get(tables, tag) do
      nil ->
        raise FontError,
          kind: "TableNotFound",
          message: "required table '#{tag}' not found"

      v ->
        v
    end
  end

  # ──────────────────────────────────────────────────────────────────────────
  # head table
  # ──────────────────────────────────────────────────────────────────────────
  #
  # We only need: unitsPerEm at offset 18.

  defp parse_head(data, tables) do
    {off, _len} = require_table(tables, "head")
    %{units_per_em: ru16(data, off + 18)}
  end

  # ──────────────────────────────────────────────────────────────────────────
  # hhea table
  # ──────────────────────────────────────────────────────────────────────────
  #
  # Fixed(4)  version
  # i16       ascender     offset 4
  # i16       descender    offset 6
  # i16       lineGap      offset 8
  # ...
  # u16       numberOfHMetrics  offset 34

  defp parse_hhea(data, tables) do
    {off, _len} = require_table(tables, "hhea")

    %{
      ascender: ri16(data, off + 4),
      descender: ri16(data, off + 6),
      line_gap: ri16(data, off + 8),
      num_h_metrics: ru16(data, off + 34)
    }
  end

  # ──────────────────────────────────────────────────────────────────────────
  # maxp table — numGlyphs at offset 4
  # ──────────────────────────────────────────────────────────────────────────

  defp parse_maxp(data, tables) do
    {off, _len} = require_table(tables, "maxp")
    ru16(data, off + 4)
  end

  # ──────────────────────────────────────────────────────────────────────────
  # cmap table — Format 4
  # ──────────────────────────────────────────────────────────────────────────
  #
  # cmap header:  version(2) + numSubtables(2)
  # Encoding record (8 bytes each):
  #   platformID  u16    3 = Windows
  #   encodingID  u16    1 = Unicode BMP
  #   offset      u32    relative to start of cmap table
  #
  # Format 4 subtable layout (relative to subtable start):
  #   0   format       u16  = 4
  #   2   length       u16
  #   4   language     u16  (ignored)
  #   6   segCountX2   u16
  #   8   searchRange  u16  (ignored)
  #  10   entrySelector u16 (ignored)
  #  12   rangeShift   u16  (ignored)
  #  14   endCode[n]        2n bytes
  #  14+2n  reservedPad u16
  #  16+2n  startCode[n]    2n bytes
  #  16+4n  idDelta[n]      2n bytes  (signed)
  #  16+6n  idRangeOffset[n] 2n bytes
  #  16+8n  glyphIdArray[]  variable

  defp parse_cmap(data, tables) do
    {cmap_off, _len} = require_table(tables, "cmap")
    num_subtables = ru16(data, cmap_off + 2)

    sub_off =
      Enum.reduce_while(0..(num_subtables - 1)//1, nil, fn i, _acc ->
        rec = cmap_off + 4 + i * 8
        plat = ru16(data, rec)
        enc = ru16(data, rec + 2)
        rel = ru32(data, rec + 4)

        if plat == 3 and enc == 1 do
          {:halt, cmap_off + rel}
        else
          {:cont, nil}
        end
      end)

    if is_nil(sub_off) do
      raise FontError, kind: "TableNotFound", message: "no cmap Format 4 subtable"
    end

    unless ru16(data, sub_off) == 4 do
      raise FontError, kind: "ParseError", message: "expected cmap Format 4"
    end

    seg_count = div(ru16(data, sub_off + 6), 2)
    end_codes_base = sub_off + 14
    start_codes_base = sub_off + 16 + seg_count * 2
    id_delta_base = sub_off + 16 + seg_count * 4
    id_range_offset_base = sub_off + 16 + seg_count * 6

    Enum.map(0..(seg_count - 1)//1, fn i ->
      {
        ru16(data, end_codes_base + i * 2),
        ru16(data, start_codes_base + i * 2),
        ri16(data, id_delta_base + i * 2),
        ru16(data, id_range_offset_base + i * 2),
        # Absolute byte address of idRangeOffset[i] — the self-relative
        # pointer base used in indirect glyph lookups.
        id_range_offset_base + i * 2
      }
    end)
  end

  # cmap_lookup/3: linear scan over pre-parsed segments.
  #
  # For each segment we check: endCode >= cp (lower bound for binary search in
  # a TrueType font, but linear is correct and simple here since segment counts
  # are typically < 100).
  #
  # The idRangeOffset indirect lookup uses the C self-relative pointer formula:
  #   abs_off = iro_abs + iro + (cp - start_code) * 2
  # This gives the byte address of glyphIdArray[cp - startCode[i]].

  defp cmap_lookup([], _raw, _cp), do: nil

  defp cmap_lookup([{ec, sc, id_delta, iro, iro_abs} | rest], raw, cp) do
    cond do
      cp > ec ->
        cmap_lookup(rest, raw, cp)

      cp < sc ->
        nil

      iro == 0 ->
        gid = band16(cp + id_delta)
        if gid == 0, do: nil, else: gid

      true ->
        abs_off = iro_abs + iro + (cp - sc) * 2
        gid = ru16(raw, abs_off)
        if gid == 0, do: nil, else: gid
    end
  end

  # ──────────────────────────────────────────────────────────────────────────
  # hmtx table
  # ──────────────────────────────────────────────────────────────────────────
  #
  # numberOfHMetrics full records: advanceWidth(u16) + lsb(i16)
  # followed by (numGlyphs − numberOfHMetrics) lsb-only records.
  # Glyphs ≥ numberOfHMetrics share the last advanceWidth.

  defp parse_hmtx_offset(data, tables) do
    {off, _len} = require_table(tables, "hmtx")
    # Sanity-probe the first byte
    <<_::unsigned-8>> = binary_part(data, off, 1)
    off
  end

  # Two-clause dispatch: out-of-range glyph returns nil immediately.
  defp lookup_glyph_metrics(%FontFile{num_glyphs: ng}, gid) when gid >= ng, do: nil

  defp lookup_glyph_metrics(
         %FontFile{
           num_h_metrics: nhm,
           hmtx_abs_off: hmtx_off,
           raw: raw
         },
         gid
       ) do
    # Clamp to last full record for glyphs beyond numberOfHMetrics.
    metric_idx = min(gid, nhm - 1)
    advance = ru16(raw, hmtx_off + metric_idx * 4)

    lsb =
      if gid < nhm do
        ri16(raw, hmtx_off + gid * 4 + 2)
      else
        # lsb-only section starts right after the nhm full records.
        ri16(raw, hmtx_off + nhm * 4 + (gid - nhm) * 2)
      end

    %GlyphMetrics{advance_width: advance, left_side_bearing: lsb}
  end

  # ──────────────────────────────────────────────────────────────────────────
  # kern table — Format 0
  # ──────────────────────────────────────────────────────────────────────────
  #
  # kern header: version(u16) + nTables(u16)
  # Each subtable header (6 bytes):
  #   version  u16
  #   length   u16
  #   coverage u16   low byte = format (0 = sorted pairs)
  #
  # Format 0 data (offset +6 from subtable start):
  #   nPairs       u16
  #   searchRange  u16  (ignored)
  #   entrySelector u16 (ignored)
  #   rangeShift   u16  (ignored)
  #   nPairs × {left u16, right u16, value i16}
  #
  # We precompute a Map of (left * 65536 + right) → value for O(1) lookups.

  defp parse_kern(data, tables) do
    case Map.get(tables, "kern") do
      nil -> %{}
      {off, _len} -> collect_kern_pairs(data, off)
    end
  end

  defp collect_kern_pairs(data, off) do
    n_tables = ru16(data, off + 2)

    {_final_pos, kern_map} =
      Enum.reduce(0..(n_tables - 1)//1, {off + 4, %{}}, fn _i, {cur, acc} ->
        sub_len = ru16(data, cur + 2)
        coverage = ru16(data, cur + 4)
        # Format is in the HIGH byte of the coverage word (bits 8-15).
        # The low byte contains directional flags (bit 0 = horizontal).
        fmt = coverage >>> 8

        new_acc =
          if fmt == 0 do
            n_pairs = ru16(data, cur + 6)
            pairs_base = cur + 14

            # Explicit step //1 prevents downward iteration when n_pairs = 0.
            Enum.reduce(0..(n_pairs - 1)//1, acc, fn j, m ->
              poff = pairs_base + j * 6
              left = ru16(data, poff)
              right = ru16(data, poff + 2)
              value = ri16(data, poff + 4)
              Map.put(m, left * 65_536 + right, value)
            end)
          else
            acc
          end

        {cur + sub_len, new_acc}
      end)

    kern_map
  end

  # ──────────────────────────────────────────────────────────────────────────
  # name table
  # ──────────────────────────────────────────────────────────────────────────
  #
  # name header: format(u16) + count(u16) + stringOffset(u16)
  # Name records (12 bytes each):
  #   platformID   u16    3 = Windows
  #   encodingID   u16    1 = Unicode BMP
  #   languageID   u16    (any)
  #   nameID       u16    1 = family  2 = subfamily
  #   length       u16
  #   offset       u16    relative to stringOffset

  defp parse_name(data, tables) do
    case Map.get(tables, "name") do
      nil ->
        {"(unknown)", "(unknown)"}

      {off, _len} ->
        count = ru16(data, off + 2)
        str_base = off + ru16(data, off + 4)

        family = find_name_string(data, off, str_base, count, 1) |> utf16be_to_utf8()
        sub = find_name_string(data, off, str_base, count, 2) |> utf16be_to_utf8()
        {family || "(unknown)", sub || "(unknown)"}
    end
  end

  defp find_name_string(data, tbl_off, str_base, count, name_id) do
    Enum.reduce_while(0..(count - 1)//1, nil, fn i, _acc ->
      rec = tbl_off + 6 + i * 12
      plat = ru16(data, rec)
      enc = ru16(data, rec + 2)
      nid = ru16(data, rec + 6)
      nlen = ru16(data, rec + 8)
      noff = ru16(data, rec + 10)

      if plat == 3 and enc == 1 and nid == name_id do
        {:halt, binary_part(data, str_base + noff, nlen)}
      else
        {:cont, nil}
      end
    end)
  end

  # Decode a UTF-16 big-endian binary to a UTF-8 Elixir string.
  # :unicode.characters_to_binary/3 handles surrogate pairs correctly.
  defp utf16be_to_utf8(nil), do: nil

  defp utf16be_to_utf8(bytes) do
    case :unicode.characters_to_binary(bytes, {:utf16, :big}, :utf8) do
      str when is_binary(str) -> str
      _ -> nil
    end
  end

  # ──────────────────────────────────────────────────────────────────────────
  # OS/2 table
  # ──────────────────────────────────────────────────────────────────────────
  #
  # The OS/2 table has many fields before sxHeight / sCapHeight.
  # Full layout up to the v2 fields (all offsets from the table start):
  #
  #  0  version          u16
  #  2  xAvgCharWidth    i16
  #  4  usWeightClass    u16
  #  6  usWidthClass     u16
  #  8  fsType           u16
  # 10  ySubscriptXSize  i16
  # 12  ySubscriptYSize  i16
  # 14  ySubscriptXOffset i16
  # 16  ySubscriptYOffset i16
  # 18  ySuperscriptXSize i16
  # 20  ySuperscriptYSize i16
  # 22  ySuperscriptXOffset i16
  # 24  ySuperscriptYOffset i16
  # 26  yStrikeoutSize   i16
  # 28  yStrikeoutPosition i16
  # 30  sFamilyClass     i16
  # 32  panose[10]       u8×10
  # 42  ulUnicodeRange1  u32
  # 46  ulUnicodeRange2  u32
  # 50  ulUnicodeRange3  u32
  # 54  ulUnicodeRange4  u32
  # 58  achVendID[4]     u8×4
  # 62  fsSelection      u16
  # 64  usFirstCharIndex u16
  # 66  usLastCharIndex  u16
  # 68  sTypoAscender    i16
  # 70  sTypoDescender   i16
  # 72  sTypoLineGap     i16
  # 74  usWinAscent      u16
  # 76  usWinDescent     u16
  # 78  ulCodePageRange1 u32   (version ≥ 1)
  # 82  ulCodePageRange2 u32   (version ≥ 1)
  # 86  sxHeight         i16   (version ≥ 2) ← we want this
  # 88  sCapHeight       i16   (version ≥ 2) ← and this

  defp parse_os2(data, tables) do
    case Map.get(tables, "OS/2") do
      nil ->
        {nil, nil}

      {off, len} ->
        version = ru16(data, off)

        # Need at least 90 bytes to read sCapHeight at offset 88 (2 bytes).
        if version >= 2 and len >= 90 do
          {ri16(data, off + 86), ri16(data, off + 88)}
        else
          {nil, nil}
        end
    end
  end

  # ──────────────────────────────────────────────────────────────────────────
  # Binary read helpers — all big-endian as required by the spec
  # ──────────────────────────────────────────────────────────────────────────

  # Read an unsigned 16-bit big-endian integer.
  defp ru16(data, off) do
    <<v::unsigned-big-16>> = binary_part(data, off, 2)
    v
  end

  # Read a signed 16-bit big-endian integer (two's complement).
  defp ri16(data, off) do
    <<v::signed-big-16>> = binary_part(data, off, 2)
    v
  end

  # Read an unsigned 32-bit big-endian integer.
  defp ru32(data, off) do
    <<v::unsigned-big-32>> = binary_part(data, off, 4)
    v
  end

  # Truncate an integer to the 16-bit unsigned range.
  # Used for idDelta wrap-around in cmap Format 4.
  defp band16(v), do: v &&& 0xFFFF
end
