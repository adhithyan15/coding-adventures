/// Metrics-only OpenType/TrueType font parsing: the [FontFile] handle, the
/// [FontError] vocabulary, global and per-glyph metric queries, and the
/// `name`-table reader.
///
/// An OpenType font file is a binary *table database*. The first bytes are a
/// directory — a list of named tables and where to find each one — and each
/// table stores one kind of information. This library reads the subset needed
/// to **measure text** without touching the OS font stack (`head`, `hhea`,
/// `maxp`, `cmap` format 4, `hmtx`, `kern` format 0, `name`, `OS/2`); the
/// `MATH` table lives in [math.dart](math.dart). It never parses outlines,
/// shapes text, or rasterizes anything.
///
/// Failure model: every fallible entry point throws [FontError]. Queries over
/// optional data return `null` or a neutral default instead — [glyphId]
/// returns `null` for unmapped codepoints, [kerning] returns `0` when the
/// font has no `kern` table — exactly like the reference.
library;

import 'dart:typed_data';

part 'math.dart';

/// Errors thrown when parsing fails.
///
/// Every variant is non-recoverable for the font that caused it: any of
/// these means the bytes are not a valid OpenType/TrueType font (or not one
/// this parser supports).
sealed class FontError implements Exception {
  const FontError();
}

/// The first 4 bytes (`sfntVersion`) were not a recognised magic number.
///
/// Valid values are `0x00010000` (TrueType outlines) and `0x4F54544F`
/// (`OTTO`, CFF/PostScript outlines).
final class InvalidMagic extends FontError {
  const InvalidMagic();

  @override
  bool operator ==(Object other) => other is InvalidMagic;

  @override
  int get hashCode => 0x5f6e7430;

  @override
  String toString() => 'FontError: invalid sfntVersion magic';
}

/// The `head.magicNumber` field was not `0x5F0F3CF5`.
///
/// A checksum sentinel baked into every compliant font; a wrong value means
/// the file is corrupt or truncated.
final class InvalidHeadMagic extends FontError {
  const InvalidHeadMagic();

  @override
  bool operator ==(Object other) => other is InvalidHeadMagic;

  @override
  int get hashCode => 0x5f6e7431;

  @override
  String toString() => 'FontError: invalid head.magicNumber';
}

/// A required table (e.g. `head`, `hmtx`) was not in the directory.
final class TableNotFound extends FontError {
  const TableNotFound(this.table);

  /// The tag (or dotted subtable path) that was missing.
  final String table;

  @override
  bool operator ==(Object other) =>
      other is TableNotFound && other.table == table;

  @override
  int get hashCode => Object.hash(0x5f6e7432, table);

  @override
  String toString() => "FontError: required table '$table' not found";
}

/// A field read attempted to access bytes past the end of the buffer.
///
/// Either the file is truncated or an offset in the directory is wrong. This
/// is also the vocabulary for a present-but-malformed table whose internal
/// offsets disagree (the `MATH` readers say `BufferTooShort` there rather
/// than invent a second corruption error).
final class BufferTooShort extends FontError {
  const BufferTooShort();

  @override
  bool operator ==(Object other) => other is BufferTooShort;

  @override
  int get hashCode => 0x5f6e7433;

  @override
  String toString() => 'FontError: buffer too short';
}

/// No Format 4 cmap subtable was found for platform 3 / encoding 1.
///
/// Defined for parity with the reference; never thrown. Like the reference,
/// [glyphId] answers "no usable subtable" with `null`, not an error.
final class UnsupportedCmapFormat extends FontError {
  const UnsupportedCmapFormat();

  @override
  bool operator ==(Object other) => other is UnsupportedCmapFormat;

  @override
  int get hashCode => 0x5f6e7434;

  @override
  String toString() =>
      'FontError: no Format 4 cmap subtable for platform 3 encoding 1';
}

/// Global typographic metrics extracted from a font file.
///
/// All integer fields are in *design units*; convert to pixels with
/// `pixels = designUnits * fontSizePx / unitsPerEm`.
final class FontMetrics {
  const FontMetrics({
    required this.unitsPerEm,
    required this.ascender,
    required this.descender,
    required this.lineGap,
    required this.xHeight,
    required this.capHeight,
    required this.numGlyphs,
    required this.familyName,
    required this.subfamilyName,
  });

  /// Design units per em square (2048 for Inter Regular).
  final int unitsPerEm;

  /// Baseline to the top of the tallest glyph (positive). Prefers
  /// `OS/2.typoAscender` over `hhea.ascender` when OS/2 is present.
  final int ascender;

  /// Baseline to the bottom of the deepest glyph (negative). Prefers
  /// `OS/2.typoDescender` over `hhea.descender`.
  final int descender;

  /// Additional inter-line spacing (often 0). Natural line height is
  /// `ascender - descender + lineGap`.
  final int lineGap;

  /// Height of a lowercase `x` above the baseline, or `null` if the `OS/2`
  /// table is absent or has version < 2.
  final int? xHeight;

  /// Height of an uppercase `H` above the baseline, or `null` if the `OS/2`
  /// table is absent or has version < 2.
  final int? capHeight;

  /// Total number of glyphs in the font.
  final int numGlyphs;

  /// Font family name (e.g. `Inter`), from `name` nameID 1.
  final String familyName;

  /// Subfamily / style name (e.g. `Regular`), from `name` nameID 2.
  final String subfamilyName;

  @override
  bool operator ==(Object other) =>
      other is FontMetrics &&
      other.unitsPerEm == unitsPerEm &&
      other.ascender == ascender &&
      other.descender == descender &&
      other.lineGap == lineGap &&
      other.xHeight == xHeight &&
      other.capHeight == capHeight &&
      other.numGlyphs == numGlyphs &&
      other.familyName == familyName &&
      other.subfamilyName == subfamilyName;

  @override
  int get hashCode => Object.hash(unitsPerEm, ascender, descender, lineGap,
      xHeight, capHeight, numGlyphs, familyName, subfamilyName);

  @override
  String toString() =>
      'FontMetrics($familyName $subfamilyName, upm=$unitsPerEm, '
      'glyphs=$numGlyphs)';
}

/// Per-glyph horizontal metrics, in design units.
final class GlyphMetrics {
  const GlyphMetrics({
    required this.advanceWidth,
    required this.leftSideBearing,
  });

  /// Horizontal distance from one glyph origin to the next — how far to
  /// advance the pen after drawing this glyph.
  final int advanceWidth;

  /// Space between the glyph's left edge and the ink boundary. Positive
  /// means the ink starts right of the pen; negative (rare) means it bleeds
  /// left.
  final int leftSideBearing;

  @override
  bool operator ==(Object other) =>
      other is GlyphMetrics &&
      other.advanceWidth == advanceWidth &&
      other.leftSideBearing == leftSideBearing;

  @override
  int get hashCode => Object.hash(advanceWidth, leftSideBearing);

  @override
  String toString() =>
      'GlyphMetrics(advance=$advanceWidth, lsb=$leftSideBearing)';
}

// ── Big-endian byte reading ──────────────────────────────────────────────
//
// OpenType stores every multi-byte integer big-endian. Each helper returns
// `null` instead of throwing so call sites can choose their own failure
// vocabulary: [load] throws, nullable queries propagate `null`, and the
// defaulting queries ([fontMetrics], [kerning]) fall back. Public so the
// white-box unit tests can reach them; hidden from the package barrel.

/// Reads a 16-bit big-endian unsigned integer, or `null` past the end.
int? readU16(List<int> buf, int offset) {
  if (offset < 0 || offset + 2 > buf.length) return null;
  return (buf[offset] << 8) | buf[offset + 1];
}

/// Reads a 16-bit big-endian signed integer, or `null` past the end.
int? readI16(List<int> buf, int offset) {
  final v = readU16(buf, offset);
  if (v == null) return null;
  return v >= 0x8000 ? v - 0x10000 : v;
}

/// Reads a 32-bit big-endian unsigned integer, or `null` past the end.
int? readU32(List<int> buf, int offset) {
  if (offset < 0 || offset + 4 > buf.length) return null;
  return (buf[offset] << 24) |
      (buf[offset + 1] << 16) |
      (buf[offset + 2] << 8) |
      buf[offset + 3];
}

/// Pre-parsed table directory: the absolute byte offset of each table this
/// library reads. `null` means the table was not present in the font.
final class _Tables {
  const _Tables({
    required this.head,
    required this.hhea,
    required this.maxp,
    required this.cmap,
    required this.hmtx,
    required this.kern,
    required this.name,
    required this.os2,
    required this.math,
  });

  final int head;
  final int hhea;
  final int maxp;
  final int cmap;
  final int hmtx;
  final int? kern;
  final int? name;
  final int? os2;

  /// The OpenType MATH table. Present only in maths fonts, so optional —
  /// its absence is an ordinary property of most fonts, not an error.
  final int? math;
}

/// An opaque handle to a parsed font file.
///
/// Created by [load], which copies the font bytes in so the caller need not
/// keep the original buffer alive. All metric queries read from that
/// internal buffer; no per-query allocation beyond the returned values.
final class FontFile {
  FontFile._(this._data, this._tables);

  final Uint8List _data;
  final _Tables _tables;

  /// A copy of the raw font bytes.
  Uint8List get data => Uint8List.fromList(_data);

  /// Locates a table by its four-byte [tag] and returns its bytes.
  ///
  /// Returns `null` if the font has no such table, or if the directory
  /// entry's offset and length fall outside the file — a truncated font
  /// reads as "table absent" rather than throwing in the caller.
  ///
  /// Throws [ArgumentError] if [tag] is not exactly 4 bytes.
  Uint8List? table(List<int> tag) {
    if (tag.length != 4) {
      throw ArgumentError.value(tag, 'tag', 'must be exactly 4 bytes');
    }
    // `load` guarantees at least 12 bytes, so offset 4 always reads.
    final numTables = readU16(_data, 4)!;
    for (var i = 0; i < numTables; i++) {
      final rec = 12 + i * 16;
      if (rec + 16 > _data.length) return null;
      var matches = true;
      for (var b = 0; b < 4; b++) {
        if (_data[rec + b] != tag[b]) {
          matches = false;
          break;
        }
      }
      if (!matches) continue;
      // The length check above proves both reads in bounds.
      final offset = readU32(_data, rec + 8)!;
      final length = readU32(_data, rec + 12)!;
      if (offset + length > _data.length) return null;
      return Uint8List.fromList(_data.sublist(offset, offset + length));
    }
    return null;
  }

  /// `head.indexToLocFormat`: 0 means short `loca` offsets, 1 means long.
  ///
  /// Getting this backwards does not fail loudly — it yields plausible,
  /// wrong offsets — so outline readers must take it from here rather than
  /// assume.
  int indexToLocFormat() {
    final v = readI16(_data, _tables.head + 50);
    if (v == null) throw const BufferTooShort();
    return v;
  }

  /// `maxp.numGlyphs`.
  int numGlyphs() {
    final v = readU16(_data, _tables.maxp + 4);
    if (v == null) throw const BufferTooShort();
    return v;
  }
}

// ── Table directory parsing ──────────────────────────────────────────────

/// Finds the byte offset of the table tagged [tag] in the font's directory.
///
/// Records start at byte 12 (after the 12-byte offset table), 16 bytes each:
/// tag (4) + checksum (4) + offset (4) + length (4). A linear scan is fine
/// for the typical 20–30 tables.
///
/// A truncated record reads as "table absent" (`null`) rather than throwing:
/// the caller's `require` step converts that into [TableNotFound], which is
/// the honest error for a file whose directory claims more than its bytes
/// deliver. (The reference indexes the offset field unchecked after a short
/// tag read, which panics on such inputs; this port fails closed instead.)
int? _findTable(Uint8List buf, int numTables, String tag) {
  final tagUnits = tag.codeUnits;
  for (var i = 0; i < numTables; i++) {
    final rec = 12 + i * 16;
    if (rec + 16 > buf.length) return null;
    var matches = true;
    for (var b = 0; b < 4; b++) {
      if (buf[rec + b] != tagUnits[b]) {
        matches = false;
        break;
      }
    }
    if (!matches) continue;
    return readU32(buf, rec + 8);
  }
  return null;
}

/// Validates the offset table and collects every table offset the library
/// reads. The first thing [load] does.
_Tables _parseTableDirectory(Uint8List buf) {
  // The offset table is 12 bytes: the minimum valid font.
  if (buf.length < 12) throw const BufferTooShort();

  // sfntVersion: 0x00010000 = TrueType, 0x4F54544F ("OTTO") = CFF.
  // The length check above proves offsets 0..6 readable, so these cannot
  // be null; the `!` documents the proof rather than testing it.
  final sfntVersion = readU32(buf, 0)!;
  if (sfntVersion != 0x00010000 && sfntVersion != 0x4F54544F) {
    throw const InvalidMagic();
  }

  final numTables = readU16(buf, 4)!;

  int require(String tag) {
    final off = _findTable(buf, numTables, tag);
    if (off == null) throw TableNotFound(tag);
    return off;
  }

  return _Tables(
    head: require('head'),
    hhea: require('hhea'),
    maxp: require('maxp'),
    cmap: require('cmap'),
    hmtx: require('hmtx'),
    kern: _findTable(buf, numTables, 'kern'),
    name: _findTable(buf, numTables, 'name'),
    os2: _findTable(buf, numTables, 'OS/2'),
    math: _findTable(buf, numTables, 'MATH'),
  );
}

// ── load ─────────────────────────────────────────────────────────────────

/// Parses raw font [bytes] and returns a [FontFile] handle.
///
/// Throws [FontError] when the bytes do not start with a valid magic
/// number, a required table is missing, the `head.magicNumber` sentinel is
/// wrong, or any byte read goes out of bounds.
FontFile load(List<int> bytes) {
  final buf = Uint8List.fromList(bytes);
  final tables = _parseTableDirectory(buf);

  // head.magicNumber sits 12 bytes into the head table.
  final magic = readU32(buf, tables.head + 12);
  if (magic == null) throw const BufferTooShort();
  if (magic != 0x5F0F3CF5) throw const InvalidHeadMagic();

  return FontFile._(buf, tables);
}

// ── fontMetrics ──────────────────────────────────────────────────────────

/// Returns global typographic metrics for the font.
///
/// Prefers OS/2 typographic values over hhea values when the OS/2 table is
/// present, matching modern renderers (CSS `line-height`, Core Text,
/// DirectWrite). Never throws: unreadable fields fall back (units-per-em to
/// 1000, metrics to 0 or hhea, names to `(unknown)`), and `xHeight` /
/// `capHeight` become `null`.
FontMetrics fontMetrics(FontFile font) {
  final buf = font._data;
  final t = font._tables;

  // head.unitsPerEm sits 18 bytes into the table.
  final unitsPerEm = readU16(buf, t.head + 18) ?? 1000;

  // hhea doubles as the fallback when OS/2 is absent.
  final hheaBase = t.hhea;
  final hheaAscender = readI16(buf, hheaBase + 4) ?? 0;
  final hheaDescender = readI16(buf, hheaBase + 6) ?? 0;
  final hheaLineGap = readI16(buf, hheaBase + 8) ?? 0;

  // maxp.numGlyphs sits at offset 4 whatever the maxp version.
  final numGlyphs = readU16(buf, t.maxp + 4) ?? 0;

  // OS/2: prefer typo values; xHeight/capHeight need version >= 2.
  var ascender = hheaAscender;
  var descender = hheaDescender;
  var lineGap = hheaLineGap;
  int? xHeight;
  int? capHeight;
  final os2off = t.os2;
  if (os2off != null) {
    final version = readU16(buf, os2off) ?? 0;
    ascender = readI16(buf, os2off + 68) ?? hheaAscender;
    descender = readI16(buf, os2off + 70) ?? hheaDescender;
    lineGap = readI16(buf, os2off + 72) ?? hheaLineGap;
    if (version >= 2) {
      xHeight = readI16(buf, os2off + 86);
      capHeight = readI16(buf, os2off + 88);
    }
  }

  final familyName = _readName(buf, t.name, 1) ?? '(unknown)';
  final subfamilyName = _readName(buf, t.name, 2) ?? '(unknown)';

  return FontMetrics(
    unitsPerEm: unitsPerEm,
    ascender: ascender,
    descender: descender,
    lineGap: lineGap,
    xHeight: xHeight,
    capHeight: capHeight,
    numGlyphs: numGlyphs,
    familyName: familyName,
    subfamilyName: subfamilyName,
  );
}

// ── glyphId — cmap Format 4 ──────────────────────────────────────────────

/// Maps a Unicode [codepoint] to a glyph ID using the font's `cmap` table.
///
/// Only the Basic Multilingual Plane is covered: codepoints above `0xFFFF`
/// return `null` because Format 4 cannot encode them. Returns `null` when
/// the codepoint is absent from the font or no usable subtable exists.
///
/// The lookup binary-searches the segment table's `endCode` array for the
/// first entry ≥ the codepoint, verifies the codepoint falls inside
/// `[startCode, endCode]`, then resolves either directly
/// (`(cp + idDelta) & 0xFFFF`) or through the `idRangeOffset` pointer trick
/// into `glyphIdArray`.
///
/// Throws [ArgumentError] if [codepoint] is not a valid u32.
int? glyphId(FontFile font, int codepoint) {
  if (codepoint < 0 || codepoint > 0xFFFFFFFF) {
    throw ArgumentError.value(
        codepoint, 'codepoint', 'must fit in 32 bits unsigned');
  }
  if (codepoint > 0xFFFF) return null;
  final buf = font._data;
  final cmapOff = font._tables.cmap;

  // cmap index header: version (2) + numSubtables (2).
  final numSubtables = readU16(buf, cmapOff + 2);
  if (numSubtables == null) return null;

  // Scan the 8-byte encoding records for platform 3 / encoding 1
  // (Windows Unicode BMP), accepting platform 0 (Unicode) as fallback.
  int? subtableAbs;
  for (var i = 0; i < numSubtables; i++) {
    final rec = cmapOff + 4 + i * 8;
    final platformId = readU16(buf, rec);
    final encodingId = readU16(buf, rec + 2);
    final subOffset = readU32(buf, rec + 4);
    if (platformId == null || encodingId == null || subOffset == null) {
      return null;
    }
    if (platformId == 3 && encodingId == 1) {
      subtableAbs = cmapOff + subOffset;
      break;
    }
    if (platformId == 0 && subtableAbs == null) {
      subtableAbs = cmapOff + subOffset;
    }
  }

  final sub = subtableAbs;
  if (sub == null) return null;

  // Only Format 4 is supported.
  if (readU16(buf, sub) != 4) return null;

  final segCountX2 = readU16(buf, sub + 6);
  if (segCountX2 == null) return null;
  final segCount = segCountX2 ~/ 2;

  // Array bases relative to the subtable start: endCode at 14, then
  // reservedPad, startCode, idDelta, idRangeOffset, glyphIdArray.
  final endCodesBase = sub + 14;
  final startCodesBase = sub + 16 + segCount * 2;
  final idDeltaBase = sub + 16 + segCount * 4;
  final idRangeOffsetBase = sub + 16 + segCount * 6;

  // First segment whose endCode >= cp (segments sort by endCode).
  var lo = 0;
  var hi = segCount;
  while (lo < hi) {
    final mid = (lo + hi) ~/ 2;
    final endCode = readU16(buf, endCodesBase + mid * 2);
    if (endCode == null) return null;
    if (endCode < codepoint) {
      lo = mid + 1;
    } else {
      hi = mid;
    }
  }
  if (lo >= segCount) return null;

  // The search probed `lo` itself in bounds on the way here: reaching this
  // line with lo == hi < segCount means some iteration narrowed hi down to
  // lo, and hi only ever narrows to a probed midpoint — so the `!` below
  // documents a proof, not an assumption. startCode lives in a different
  // array at higher addresses, so only it needs a fresh check.
  final endCode = readU16(buf, endCodesBase + lo * 2)!;
  final startCode = readU16(buf, startCodesBase + lo * 2);
  if (startCode == null) return null;
  if (codepoint > endCode || codepoint < startCode) return null;

  final idDelta = readI16(buf, idDeltaBase + lo * 2);
  final idRangeOffset = readU16(buf, idRangeOffsetBase + lo * 2);
  if (idDelta == null || idRangeOffset == null) return null;

  final int glyph;
  if (idRangeOffset == 0) {
    // Direct delta, wrapped mod 65536.
    glyph = (codepoint + idDelta) & 0xFFFF;
  } else {
    // The idRangeOffset pointer trick: the entry's absolute byte address
    // is the address of idRangeOffset[i] itself, plus the offset, plus
    // the codepoint's distance into the segment times two.
    final absOff = (idRangeOffsetBase + lo * 2) +
        idRangeOffset +
        (codepoint - startCode) * 2;
    final g = readU16(buf, absOff);
    if (g == null) return null;
    glyph = g;
  }

  // Glyph 0 is .notdef: "mapped to missing" reads as unmapped.
  if (glyph == 0) return null;
  return glyph;
}

// ── glyphMetrics — hmtx ──────────────────────────────────────────────────

/// Returns the horizontal metrics for [glyphIdValue].
///
/// Returns `null` when the glyph is out of range (≥ numGlyphs) or the
/// table bytes are unreadable.
///
/// The `hmtx` table has two sections: `numberOfHMetrics` full
/// (advanceWidth, lsb) records, then per-glyph lsb-only records for the
/// remaining glyphs, which share the last full record's advance width.
///
/// Throws [ArgumentError] if [glyphIdValue] does not fit in 16 bits.
GlyphMetrics? glyphMetrics(FontFile font, int glyphIdValue) {
  if (glyphIdValue < 0 || glyphIdValue > 0xFFFF) {
    throw ArgumentError.value(
        glyphIdValue, 'glyphId', 'must fit in 16 bits unsigned');
  }
  final buf = font._data;
  final t = font._tables;

  final numGlyphs = readU16(buf, t.maxp + 4);
  final numHMetrics = readU16(buf, t.hhea + 34);
  if (numGlyphs == null || numHMetrics == null) return null;
  final hmtxOff = t.hmtx;

  if (glyphIdValue >= numGlyphs) return null;

  final int advanceWidth;
  final int leftSideBearing;
  if (glyphIdValue < numHMetrics) {
    // Full 4-byte record.
    final base = hmtxOff + glyphIdValue * 4;
    final adv = readU16(buf, base);
    final lsb = readI16(buf, base + 2);
    if (adv == null || lsb == null) return null;
    advanceWidth = adv;
    leftSideBearing = lsb;
  } else {
    // Shared advance: the last full record's width plus a per-glyph lsb
    // from the trailing array. A font with zero hMetrics has no "last"
    // advance to share, so there is nothing truthful to return.
    if (numHMetrics == 0) return null;
    final lastAdvance = readU16(buf, hmtxOff + (numHMetrics - 1) * 4);
    if (lastAdvance == null) return null;
    final lsbOff = hmtxOff + numHMetrics * 4 + (glyphIdValue - numHMetrics) * 2;
    final lsb = readI16(buf, lsbOff);
    if (lsb == null) return null;
    advanceWidth = lastAdvance;
    leftSideBearing = lsb;
  }

  return GlyphMetrics(
    advanceWidth: advanceWidth,
    leftSideBearing: leftSideBearing,
  );
}

// ── kerning — kern Format 0 ──────────────────────────────────────────────

/// Returns the kerning adjustment in design units for a glyph pair.
///
/// Returns `0` when the font has no `kern` table, the pair is absent from
/// every Format 0 subtable, or the table bytes are unreadable. Negative
/// means tighter spacing, positive means wider.
///
/// Format 0 stores pairs sorted by the composite key
/// `(left << 16) | right`, so one 32-bit binary search settles both glyphs.
///
/// Throws [ArgumentError] if either glyph ID does not fit in 16 bits.
int kerning(FontFile font, int left, int right) {
  if (left < 0 || left > 0xFFFF) {
    throw ArgumentError.value(left, 'left', 'must fit in 16 bits unsigned');
  }
  if (right < 0 || right > 0xFFFF) {
    throw ArgumentError.value(right, 'right', 'must fit in 16 bits unsigned');
  }
  final buf = font._data;

  final kernOff = font._tables.kern;
  if (kernOff == null) return 0;

  // kern header: version (u16) + nTables (u16).
  final nTables = readU16(buf, kernOff + 2);
  if (nTables == null) return 0;

  var pos = kernOff + 4;
  for (var t = 0; t < nTables; t++) {
    if (pos + 6 > buf.length) break;
    // Subtable header: version + length + coverage. The length check above
    // proves both reads in bounds.
    final length = readU16(buf, pos + 2)!;
    final coverage = readU16(buf, pos + 4)!;

    // Format is the high byte of coverage.
    if (coverage >> 8 == 0) {
      // Format 0 header: nPairs + searchRange + entrySelector + rangeShift.
      final nPairs = readU16(buf, pos + 6);
      if (nPairs == null) break;
      final pairsBase = pos + 14;

      final target = (left << 16) | right;
      var lo = 0;
      var hi = nPairs;
      var found = false;
      var value = 0;
      while (lo < hi) {
        final mid = (lo + hi) ~/ 2;
        final pairOff = pairsBase + mid * 6;
        final pairLeft = readU16(buf, pairOff);
        final pairRight = readU16(buf, pairOff + 2);
        if (pairLeft == null || pairRight == null) break;
        final key = (pairLeft << 16) | pairRight;
        if (key == target) {
          value = readI16(buf, pairOff + 4) ?? 0;
          found = true;
          break;
        } else if (key < target) {
          lo = mid + 1;
        } else {
          hi = mid;
        }
      }
      if (found) return value;
    }

    pos += length;
  }

  return 0;
}

// ── name table ───────────────────────────────────────────────────────────

/// Reads a UTF-16 BE string from the `name` table by [nameId].
///
/// Prefers platform 3 / encoding 1 (Windows Unicode BMP); falls back to
/// the first platform 0 (Unicode) record. Returns `null` when the table is
/// absent or the name ID is not found.
String? _readName(Uint8List buf, int? nameOff, int nameId) {
  if (nameOff == null) return null;

  // Header: format + count + stringOffset.
  final count = readU16(buf, nameOff + 2);
  final stringOffset = readU16(buf, nameOff + 4);
  if (count == null || stringOffset == null) return null;

  // First platform-0 fallback wins; platform 3 / encoding 1 wins outright.
  // A record keeps start and length together: one is never set without
  // the other, so a single null check settles both.
  (int, int)? best;
  for (var i = 0; i < count; i++) {
    final rec = nameOff + 6 + i * 12;
    final platformId = readU16(buf, rec);
    final encodingId = readU16(buf, rec + 2);
    final nid = readU16(buf, rec + 6);
    final length = readU16(buf, rec + 8);
    final strOff = readU16(buf, rec + 10);
    if (platformId == null ||
        encodingId == null ||
        nid == null ||
        length == null ||
        strOff == null) {
      return null;
    }
    if (nid != nameId) continue;
    final absStart = nameOff + stringOffset + strOff;
    if (platformId == 3 && encodingId == 1) {
      best = (absStart, length);
      break;
    }
    if (platformId == 0 && best == null) {
      best = (absStart, length);
    }
  }

  final found = best;
  if (found == null) return null;
  final (start, len) = found;
  if (start + len > buf.length) return null;

  return _decodeUtf16Be(buf, start, len);
}

/// Decodes [length] bytes at [start] as UTF-16 BE.
///
/// Surrogate pairs combine; unpaired surrogates become U+FFFD (Dart strings
/// *can* hold lone surrogates, so the check must be explicit — the decoder
/// cannot lean on the platform to reject them). A trailing odd byte is
/// dropped, matching the reference's chunking.
String _decodeUtf16Be(Uint8List buf, int start, int length) {
  final units = <int>[];
  final end = start + length - 1;
  for (var i = start; i < end; i += 2) {
    units.add((buf[i] << 8) | buf[i + 1]);
  }
  final out = StringBuffer();
  for (var i = 0; i < units.length; i++) {
    final u = units[i];
    if (u >= 0xD800 && u <= 0xDBFF && i + 1 < units.length) {
      final lo = units[i + 1];
      if (lo >= 0xDC00 && lo <= 0xDFFF) {
        out.writeCharCode(0x10000 + ((u - 0xD800) << 10) + (lo - 0xDC00));
        i++;
        continue;
      }
    }
    if (u >= 0xD800 && u <= 0xDFFF) {
      out.writeCharCode(0xFFFD);
    } else {
      out.writeCharCode(u);
    }
  }
  return out.toString();
}
