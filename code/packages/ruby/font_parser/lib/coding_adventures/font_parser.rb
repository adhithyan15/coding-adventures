# frozen_string_literal: true

require_relative "font_parser/version"

# CodingAdventures::FontParser — metrics-only OpenType/TrueType font parser.
#
# Parses raw font bytes and exposes the numeric metrics needed to lay out
# text without touching the OS font stack.
#
# == Usage
#
#   require "coding_adventures/font_parser"
#   include CodingAdventures::FontParser
#
#   data = File.binread("Inter-Regular.ttf")
#   font = load(data)
#
#   m = font_metrics(font)
#   puts m.units_per_em   # 2048
#   puts m.family_name    # "Inter"
#
#   gid_a = glyph_id(font, 0x0041)   # 'A'
#   gid_v = glyph_id(font, 0x0056)   # 'V'
#   puts kerning(font, gid_a, gid_v) # 0 for Inter (GPOS only)

module CodingAdventures
  module FontParser
    # ─────────────────────────────────────────────────────────────────────────
    # Error class
    # ─────────────────────────────────────────────────────────────────────────

    # Raised when font bytes cannot be parsed.
    #
    # @attr_reader kind [String] discriminant:
    #   "InvalidMagic", "InvalidHeadMagic", "TableNotFound",
    #   "BufferTooShort", "UnsupportedCmapFormat"
    class FontError < StandardError
      attr_reader :kind

      def initialize(kind, message)
        super(message)
        @kind = kind
      end
    end

    # ─────────────────────────────────────────────────────────────────────────
    # Public metric structs
    # ─────────────────────────────────────────────────────────────────────────

    # Global typographic metrics. All integers in design units.
    # Convert to pixels: +pixels = design_units * font_size_px / units_per_em+
    FontMetrics = Struct.new(
      :units_per_em,   # Integer — Inter = 2048
      :ascender,       # Integer — positive, from OS/2 or hhea
      :descender,      # Integer — negative, from OS/2 or hhea
      :line_gap,       # Integer — extra inter-line spacing (often 0)
      :x_height,       # Integer or nil — height of 'x' (OS/2 v≥2)
      :cap_height,     # Integer or nil — height of 'H' (OS/2 v≥2)
      :num_glyphs,     # Integer — total glyph count
      :family_name,    # String  — e.g. "Inter"
      :subfamily_name, # String  — e.g. "Regular"
      keyword_init: true
    )

    # Per-glyph horizontal metrics. All integers in design units.
    GlyphMetrics = Struct.new(
      :advance_width,      # Integer — pen advance
      :left_side_bearing,  # Integer — space before ink (may be negative)
      keyword_init: true
    )

    # ─────────────────────────────────────────────────────────────────────────
    # Internal: FontFile
    # ─────────────────────────────────────────────────────────────────────────

    # Opaque handle to a parsed font. Holds a frozen copy of the bytes.
    # @api private
    class FontFile
      attr_reader :data, :tables

      def initialize(data, tables)
        @data = data.freeze
        @tables = tables
      end
    end

    # @api private
    Tables = Struct.new(
      :head, :hhea, :maxp, :cmap, :hmtx,
      :kern, :name, :os2,
      keyword_init: true
    )

    # ─────────────────────────────────────────────────────────────────────────
    # Big-endian reading helpers
    # ─────────────────────────────────────────────────────────────────────────
    #
    # Ruby's String#unpack1 with ">" (big-endian) format handles the byte
    # swapping. We always bounds-check before calling unpack1.

    # @api private
    def self.read_u16(buf, off)
      raise FontError.new("BufferTooShort", "read_u16 at #{off} out of bounds") if off + 2 > buf.bytesize

      buf.byteslice(off, 2).unpack1("n") # "n" = big-endian uint16
    end

    # @api private
    def self.read_i16(buf, off)
      raise FontError.new("BufferTooShort", "read_i16 at #{off} out of bounds") if off + 2 > buf.bytesize

      buf.byteslice(off, 2).unpack1("n").then { |v| (v >= 0x8000) ? v - 0x10000 : v }
    end

    # @api private
    def self.read_u32(buf, off)
      raise FontError.new("BufferTooShort", "read_u32 at #{off} out of bounds") if off + 4 > buf.bytesize

      buf.byteslice(off, 4).unpack1("N") # "N" = big-endian uint32
    end

    # ─────────────────────────────────────────────────────────────────────────
    # Table directory
    # ─────────────────────────────────────────────────────────────────────────

    # @api private
    def self.find_table(buf, num_tables, tag)
      num_tables.times do |i|
        rec = 12 + i * 16
        return nil if rec + 16 > buf.bytesize

        return read_u32(buf, rec + 8) if buf.byteslice(rec, 4) == tag
      end
      nil
    end

    # @api private
    def self.require_table(buf, num_tables, tag, name)
      off = find_table(buf, num_tables, tag)
      raise FontError.new("TableNotFound", "required table '#{name}' not found") if off.nil?

      off
    end

    # ─────────────────────────────────────────────────────────────────────────
    # load
    # ─────────────────────────────────────────────────────────────────────────

    # Parse raw font bytes and return a FontFile handle.
    #
    # @param data [String, IO] raw font bytes (binary encoding expected)
    # @return [FontFile]
    # @raise [FontError] on parse failures
    def self.load(data)
      # Normalise to a binary string.
      buf = data.is_a?(String) ? data.dup.force_encoding(Encoding::BINARY) : data.read.force_encoding(Encoding::BINARY)

      raise FontError.new("BufferTooShort", "buffer too small") if buf.bytesize < 12

      sfnt = read_u32(buf, 0)
      unless [0x00010000, 0x4F54544F].include?(sfnt)
        raise FontError.new("InvalidMagic", format("invalid sfntVersion 0x%08X", sfnt))
      end

      num_tables = read_u16(buf, 4)

      tables = Tables.new(
        head: require_table(buf, num_tables, "head", "head"),
        hhea: require_table(buf, num_tables, "hhea", "hhea"),
        maxp: require_table(buf, num_tables, "maxp", "maxp"),
        cmap: require_table(buf, num_tables, "cmap", "cmap"),
        hmtx: require_table(buf, num_tables, "hmtx", "hmtx"),
        kern: find_table(buf, num_tables, "kern"),
        name: find_table(buf, num_tables, "name"),
        os2: find_table(buf, num_tables, "OS/2")
      )

      # Validate head.magicNumber sentinel (offset 12 within head table).
      magic = read_u32(buf, tables.head + 12)
      unless magic == 0x5F0F3CF5
        raise FontError.new("InvalidHeadMagic", format("invalid head.magicNumber 0x%08X", magic))
      end

      FontFile.new(buf, tables)
    end

    # ─────────────────────────────────────────────────────────────────────────
    # font_metrics
    # ─────────────────────────────────────────────────────────────────────────

    # Return global typographic metrics.
    #
    # Prefers OS/2 typographic values over hhea when OS/2 is present.
    #
    # @param font [FontFile]
    # @return [FontMetrics]
    def self.font_metrics(font)
      buf = font.data
      t = font.tables

      units_per_em = read_u16(buf, t.head + 18)

      hhea_ascender = read_i16(buf, t.hhea + 4)
      hhea_descender = read_i16(buf, t.hhea + 6)
      hhea_line_gap = read_i16(buf, t.hhea + 8)

      num_glyphs = read_u16(buf, t.maxp + 4)

      ascender = hhea_ascender
      descender = hhea_descender
      line_gap = hhea_line_gap
      x_height = nil
      cap_height = nil

      unless t.os2.nil?
        base = t.os2
        version = read_u16(buf, base)
        ascender = read_i16(buf, base + 68)
        descender = read_i16(buf, base + 70)
        line_gap = read_i16(buf, base + 72)
        if version >= 2
          x_height = read_i16(buf, base + 86)
          cap_height = read_i16(buf, base + 88)
        end
      end

      family_name = read_name_string(buf, t.name, 1) || "(unknown)"
      subfamily_name = read_name_string(buf, t.name, 2) || "(unknown)"

      FontMetrics.new(
        units_per_em:,
        ascender:,
        descender:,
        line_gap:,
        x_height:,
        cap_height:,
        num_glyphs:,
        family_name:,
        subfamily_name:
      )
    end

    # ─────────────────────────────────────────────────────────────────────────
    # glyph_id — cmap Format 4 lookup
    # ─────────────────────────────────────────────────────────────────────────

    # Map a Unicode codepoint to a glyph ID.
    #
    # Returns +nil+ if the codepoint is not in the font or above U+FFFF.
    #
    # @param font [FontFile]
    # @param codepoint [Integer] Unicode codepoint
    # @return [Integer, nil] glyph ID or nil
    def self.glyph_id(font, codepoint)
      return nil unless codepoint.between?(0, 0xFFFF)

      cp = codepoint
      buf = font.data
      cmap_off = font.tables.cmap

      # Find Format 4 subtable.
      num_subtables = read_u16(buf, cmap_off + 2)
      subtable_abs = nil

      num_subtables.times do |i|
        rec = cmap_off + 4 + i * 8
        platform_id = read_u16(buf, rec)
        encoding_id = read_u16(buf, rec + 2)
        sub_off = read_u32(buf, rec + 4)

        if platform_id == 3 && encoding_id == 1
          subtable_abs = cmap_off + sub_off
          break
        end
        subtable_abs ||= cmap_off + sub_off if platform_id == 0
      end

      return nil if subtable_abs.nil?
      return nil unless read_u16(buf, subtable_abs) == 4

      seg_count_x2 = read_u16(buf, subtable_abs + 6)
      seg_count = seg_count_x2 / 2

      end_codes_base = subtable_abs + 14
      start_codes_base = subtable_abs + 16 + seg_count * 2
      id_delta_base = subtable_abs + 16 + seg_count * 4
      id_range_offset_base = subtable_abs + 16 + seg_count * 6

      # Binary search on endCode[].
      lo = 0
      hi = seg_count
      while lo < hi
        mid = (lo + hi) / 2
        if read_u16(buf, end_codes_base + mid * 2) < cp
          lo = mid + 1
        else
          hi = mid
        end
      end

      return nil if lo >= seg_count

      end_code = read_u16(buf, end_codes_base + lo * 2)
      start_code = read_u16(buf, start_codes_base + lo * 2)

      return nil unless cp.between?(start_code, end_code)

      id_delta = read_i16(buf, id_delta_base + lo * 2)
      id_range_offset = read_u16(buf, id_range_offset_base + lo * 2)

      glyph = if id_range_offset.zero?
        (cp + id_delta) & 0xFFFF
      else
        abs_off = (id_range_offset_base + lo * 2) + id_range_offset + (cp - start_code) * 2
        read_u16(buf, abs_off)
      end

      glyph.zero? ? nil : glyph
    end

    # ─────────────────────────────────────────────────────────────────────────
    # glyph_metrics — hmtx lookup
    # ─────────────────────────────────────────────────────────────────────────

    # Return horizontal metrics for a glyph ID.
    #
    # @param font [FontFile]
    # @param gid [Integer] glyph ID
    # @return [GlyphMetrics, nil]
    def self.glyph_metrics(font, gid)
      buf = font.data
      t = font.tables

      num_glyphs = read_u16(buf, t.maxp + 4)
      num_h_metrics = read_u16(buf, t.hhea + 34)
      hmtx_off = t.hmtx

      return nil unless gid.between?(0, num_glyphs - 1)

      if gid < num_h_metrics
        base = hmtx_off + gid * 4
        GlyphMetrics.new(
          advance_width: read_u16(buf, base),
          left_side_bearing: read_i16(buf, base + 2)
        )
      else
        last_advance = read_u16(buf, hmtx_off + (num_h_metrics - 1) * 4)
        lsb_off = hmtx_off + num_h_metrics * 4 + (gid - num_h_metrics) * 2
        GlyphMetrics.new(
          advance_width: last_advance,
          left_side_bearing: read_i16(buf, lsb_off)
        )
      end
    end

    # ─────────────────────────────────────────────────────────────────────────
    # kerning — kern Format 0
    # ─────────────────────────────────────────────────────────────────────────

    # Return kern adjustment for a glyph pair (design units).
    #
    # Returns 0 if no kern table or pair not found.
    # Negative = tighter; positive = wider.
    #
    # @param font [FontFile]
    # @param left [Integer] left glyph ID
    # @param right [Integer] right glyph ID
    # @return [Integer]
    def self.kerning(font, left, right)
      return 0 if font.tables.kern.nil?

      buf = font.data
      kern_off = font.tables.kern
      n_tables = read_u16(buf, kern_off + 2)
      target = (left << 16) | right

      pos = kern_off + 4
      n_tables.times do
        break if pos + 6 > buf.bytesize

        length = read_u16(buf, pos + 2)
        coverage = read_u16(buf, pos + 4)
        sub_format = coverage >> 8

        if sub_format.zero?
          n_pairs = read_u16(buf, pos + 6)
          pairs_base = pos + 14

          lo = 0
          hi = n_pairs
          while lo < hi
            mid = (lo + hi) / 2
            pair_off = pairs_base + mid * 6
            key = (read_u16(buf, pair_off) << 16) | read_u16(buf, pair_off + 2)

            if key == target
              return read_i16(buf, pair_off + 4)
            elsif key < target
              lo = mid + 1
            else
              hi = mid
            end
          end
        end

        pos += length
      end

      0
    end

    # ─────────────────────────────────────────────────────────────────────────
    # name table reading
    # ─────────────────────────────────────────────────────────────────────────

    # @api private
    def self.read_name_string(buf, name_off, name_id)
      return nil if name_off.nil?

      base = name_off
      count = read_u16(buf, base + 2)
      string_offset = read_u16(buf, base + 4)

      best = nil # [platform_id, abs_start, length]

      count.times do |i|
        rec = base + 6 + i * 12
        platform_id = read_u16(buf, rec)
        encoding_id = read_u16(buf, rec + 2)
        nid = read_u16(buf, rec + 6)
        length = read_u16(buf, rec + 8)
        str_off = read_u16(buf, rec + 10)

        next unless nid == name_id

        abs_start = base + string_offset + str_off

        if platform_id == 3 && encoding_id == 1
          best = [3, abs_start, length]
          break
        end
        best ||= [0, abs_start, length] if platform_id.zero?
      end

      return nil if best.nil?

      _platform_id, start, length = best
      raw = buf.byteslice(start, length)
      return nil if raw.nil? || raw.bytesize < length

      # Decode UTF-16 BE. Ruby's String#encode handles this cleanly:
      # mark the bytes as UTF-16 BE, then transcode to UTF-8.
      raw.force_encoding(Encoding::UTF_16BE).encode(Encoding::UTF_8, invalid: :replace, undef: :replace)
    rescue Encoding::InvalidByteSequenceError, Encoding::UndefinedConversionError
      nil
    end
  end
end
