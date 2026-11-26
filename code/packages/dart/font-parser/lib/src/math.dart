/// The OpenType `MATH` table: the constants, italic corrections, and glyph
/// constructions a maths typesetter needs.
part of 'font_parser.dart';

/// Which way a glyph stretches.
///
/// A `MATH` table keeps two independent constructions per glyph, and asking
/// on the wrong axis silently returns nothing — so a brace that should grow
/// tall simply does not, with no error to notice.
enum StretchAxis {
  /// How `(` grows to wrap a tall fraction.
  vertical,

  /// How `\overbrace` grows to span its contents.
  horizontal,
}

/// One ready-made larger glyph.
final class GlyphVariant {
  const GlyphVariant({required this.glyphId, required this.advance});

  final int glyphId;

  /// Its size along the stretch axis, in design units — height for a
  /// vertical construction, width for a horizontal one.
  final int advance;

  @override
  bool operator ==(Object other) =>
      other is GlyphVariant &&
      other.glyphId == glyphId &&
      other.advance == advance;

  @override
  int get hashCode => Object.hash(glyphId, advance);

  @override
  String toString() => 'GlyphVariant($glyphId, advance=$advance)';
}

/// One piece of a glyph built by assembly.
final class AssemblyPart {
  const AssemblyPart({
    required this.glyphId,
    required this.startConnector,
    required this.endConnector,
    required this.fullAdvance,
    required this.isExtender,
  });

  final int glyphId;

  /// How much of this part may overlap the piece *before* it.
  final int startConnector;

  /// How much may overlap the piece *after* it.
  final int endConnector;

  /// Its full size along the axis.
  final int fullAdvance;

  /// An extender may be repeated to reach the required size; the others
  /// appear exactly once. A renderer that repeats a non-extender draws two
  /// brace tips in the middle of a brace.
  final bool isExtender;

  @override
  bool operator ==(Object other) =>
      other is AssemblyPart &&
      other.glyphId == glyphId &&
      other.startConnector == startConnector &&
      other.endConnector == endConnector &&
      other.fullAdvance == fullAdvance &&
      other.isExtender == isExtender;

  @override
  int get hashCode => Object.hash(
      glyphId, startConnector, endConnector, fullAdvance, isExtender);

  @override
  String toString() =>
      'AssemblyPart($glyphId, extender=$isExtender, full=$fullAdvance)';
}

/// How one glyph grows.
///
/// Two mechanisms, and a renderer needs both. **Variants** are complete
/// larger glyphs the designer drew — the sizes of `(` that cover most
/// cases. **Assembly** is for when none is big enough: pieces stacked with
/// overlap, which is how a brace spanning half a page is built from a top,
/// a bottom, and a repeated extender.
final class GlyphConstruction {
  GlyphConstruction({
    required List<GlyphVariant> variants,
    required List<AssemblyPart> assembly,
    required this.assemblyItalicsCorrection,
  })  : variants = List.unmodifiable(variants),
        assembly = List.unmodifiable(assembly);

  /// Larger pre-drawn glyphs, in increasing size.
  final List<GlyphVariant> variants;

  /// Pieces to stack when no variant is large enough. Empty when the font
  /// offers none, which means the largest variant is the limit.
  final List<AssemblyPart> assembly;

  /// Italic correction for the assembled glyph.
  final int assemblyItalicsCorrection;

  @override
  bool operator ==(Object other) =>
      other is GlyphConstruction &&
      _equalVariants(other.variants, variants) &&
      _equalParts(other.assembly, assembly) &&
      other.assemblyItalicsCorrection == assemblyItalicsCorrection;

  @override
  int get hashCode => Object.hash(Object.hashAll(variants),
      Object.hashAll(assembly), assemblyItalicsCorrection);

  @override
  String toString() => 'GlyphConstruction(${variants.length} variants, '
      '${assembly.length} parts, italics=$assemblyItalicsCorrection)';
}

bool _equalVariants(List<GlyphVariant> a, List<GlyphVariant> b) {
  if (a.length != b.length) return false;
  for (var i = 0; i < a.length; i++) {
    if (a[i] != b[i]) return false;
  }
  return true;
}

bool _equalParts(List<AssemblyPart> a, List<AssemblyPart> b) {
  if (a.length != b.length) return false;
  for (var i = 0; i < a.length; i++) {
    if (a[i] != b[i]) return false;
  }
  return true;
}

/// The subset of `MathConstants` a typesetter actually needs.
///
/// Not all ~56 values are exposed — only those Appendix G consumes for
/// fractions, radicals, and scripts. The rest wait for a consumer rather
/// than being transcribed speculatively: an untested constant is a
/// liability, not a feature.
///
/// All values are in font design units; `scriptPercentScaleDown` and
/// `scriptScriptPercentScaleDown` are percentages instead.
final class MathConstants {
  const MathConstants({
    required this.scriptPercentScaleDown,
    required this.scriptScriptPercentScaleDown,
    required this.delimitedSubFormulaMinHeight,
    required this.displayOperatorMinHeight,
    required this.mathLeading,
    required this.axisHeight,
    required this.accentBaseHeight,
    required this.fractionRuleThickness,
    required this.fractionNumeratorShiftUp,
    required this.fractionDenominatorShiftDown,
    required this.radicalRuleThickness,
    required this.radicalVerticalGap,
    required this.superscriptShiftUp,
    required this.subscriptShiftDown,
  });

  /// Percentage the font scales down to for script (superscript) size.
  final int scriptPercentScaleDown;

  /// Percentage for scriptscript (super-superscript) size.
  final int scriptScriptPercentScaleDown;

  /// Minimum height of a `\left…\right` subformula before it is treated as
  /// delimited rather than inline.
  final int delimitedSubFormulaMinHeight;

  /// Minimum height for a display-style operator such as `\sum`.
  final int displayOperatorMinHeight;

  /// Extra leading between lines of a maths formula.
  final int mathLeading;

  /// Height of the maths axis above the baseline. The single most
  /// load-bearing constant: fraction bars, radical rules, and `\left(`
  /// delimiters all centre on it.
  final int axisHeight;

  /// Height of the base above which accents are placed.
  final int accentBaseHeight;

  /// Thickness of the bar in a fraction; also the default rule thickness
  /// for over- and under-lines.
  final int fractionRuleThickness;

  /// How far the numerator's baseline shifts up, in text style.
  final int fractionNumeratorShiftUp;

  /// How far the denominator's baseline shifts down, in text style.
  final int fractionDenominatorShiftDown;

  /// Thickness of the radical's overbar.
  final int radicalRuleThickness;

  /// Vertical clearance between the radical rule and its contents.
  final int radicalVerticalGap;

  /// How far a superscript's baseline shifts up, in an uncramped style.
  final int superscriptShiftUp;

  /// How far a subscript's baseline shifts down.
  final int subscriptShiftDown;

  @override
  bool operator ==(Object other) =>
      other is MathConstants &&
      other.scriptPercentScaleDown == scriptPercentScaleDown &&
      other.scriptScriptPercentScaleDown == scriptScriptPercentScaleDown &&
      other.delimitedSubFormulaMinHeight == delimitedSubFormulaMinHeight &&
      other.displayOperatorMinHeight == displayOperatorMinHeight &&
      other.mathLeading == mathLeading &&
      other.axisHeight == axisHeight &&
      other.accentBaseHeight == accentBaseHeight &&
      other.fractionRuleThickness == fractionRuleThickness &&
      other.fractionNumeratorShiftUp == fractionNumeratorShiftUp &&
      other.fractionDenominatorShiftDown == fractionDenominatorShiftDown &&
      other.radicalRuleThickness == radicalRuleThickness &&
      other.radicalVerticalGap == radicalVerticalGap &&
      other.superscriptShiftUp == superscriptShiftUp &&
      other.subscriptShiftDown == subscriptShiftDown;

  @override
  int get hashCode => Object.hash(
        scriptPercentScaleDown,
        scriptScriptPercentScaleDown,
        delimitedSubFormulaMinHeight,
        displayOperatorMinHeight,
        mathLeading,
        axisHeight,
        accentBaseHeight,
        fractionRuleThickness,
        fractionNumeratorShiftUp,
        fractionDenominatorShiftDown,
        radicalRuleThickness,
        radicalVerticalGap,
        superscriptShiftUp,
        subscriptShiftDown,
      );

  @override
  String toString() => 'MathConstants(axis=$axisHeight, '
      'script=$scriptPercentScaleDown/$scriptScriptPercentScaleDown)';
}

// ── MathConstants ────────────────────────────────────────────────────────

/// Reads the font's [MathConstants], if it has a `MATH` table.
///
/// Returns `null` for an ordinary text font: most fonts have no `MATH`
/// table, so its absence is a property rather than a failure. Throws when
/// the table is present but malformed — a caller that treated "no maths
/// font" and "corrupt maths font" the same would silently typeset with
/// default constants and produce output that looks almost right.
MathConstants? mathConstants(FontFile font) {
  final mathOffset = font._tables.math;
  if (mathOffset == null) return null;
  return _readMathConstants(font._data, mathOffset);
}

MathConstants _readMathConstants(Uint8List buf, int mathOffset) {
  // MATH header: version (u32), then three Offset16s to the subtables.
  final constantsOffset = readU16(buf, mathOffset + 4);
  if (constantsOffset == null) throw const BufferTooShort();
  if (constantsOffset == 0) {
    throw const TableNotFound('MATH.MathConstants');
  }
  final base = mathOffset + constantsOffset;

  // The first four are bare int16/uint16 with NO device table. Everything
  // after them is a MathValue — int16 value + Offset16 device table — so
  // the stride doubles from four bytes in. Reading MathValues as bare
  // int16 appears to work (the first field really is the value) while
  // silently halving the stride, so every constant after the first comes
  // out wrong. Index by record, never by byte.
  int valueAt(int index) {
    final v = readI16(buf, base + 8 + index * 4);
    if (v == null) throw const BufferTooShort();
    return v;
  }

  int reqI16(int offset) {
    final v = readI16(buf, offset);
    if (v == null) throw const BufferTooShort();
    return v;
  }

  int reqU16(int offset) {
    final v = readU16(buf, offset);
    if (v == null) throw const BufferTooShort();
    return v;
  }

  return MathConstants(
    scriptPercentScaleDown: reqI16(base),
    scriptScriptPercentScaleDown: reqI16(base + 2),
    delimitedSubFormulaMinHeight: reqU16(base + 4),
    displayOperatorMinHeight: reqU16(base + 6),
    // Record index = the constant's spec-list position minus the four bare
    // fields above; comments carry the spec position so the arithmetic is
    // checkable without counting records.
    mathLeading: valueAt(0), // spec 4
    axisHeight: valueAt(1), // spec 5
    accentBaseHeight: valueAt(2), // spec 6
    subscriptShiftDown: valueAt(4), // spec 8
    superscriptShiftUp: valueAt(7), // spec 11
    fractionNumeratorShiftUp: valueAt(28), // spec 32
    fractionDenominatorShiftDown: valueAt(30), // spec 34
    fractionRuleThickness: valueAt(34), // spec 38
    radicalVerticalGap: valueAt(45), // spec 49
    radicalRuleThickness: valueAt(47), // spec 51
  );
}

// ── Italic corrections ───────────────────────────────────────────────────

/// Italic correction for one glyph, in design units.
///
/// The horizontal gap a slanted glyph needs after it: set a subscript after
/// an italic *f* without it and the two collide. Returns `null` when the
/// font has no `MATH` table or no entry for this glyph; both mean zero
/// correction.
///
/// Throws [ArgumentError] if [glyphIdValue] does not fit in 16 bits.
int? italicCorrection(FontFile font, int glyphIdValue) {
  if (glyphIdValue < 0 || glyphIdValue > 0xFFFF) {
    throw ArgumentError.value(
        glyphIdValue, 'glyphId', 'must fit in 16 bits unsigned');
  }
  final mathOffset = font._tables.math;
  if (mathOffset == null) return null;
  return _readItalicCorrection(font._data, mathOffset, glyphIdValue);
}

int? _readItalicCorrection(Uint8List buf, int mathOffset, int glyphIdValue) {
  final glyphInfoOffset = readU16(buf, mathOffset + 6);
  if (glyphInfoOffset == null) throw const BufferTooShort();
  if (glyphInfoOffset == 0) return null;
  final glyphInfo = mathOffset + glyphInfoOffset;

  // MathGlyphInfo begins with an Offset16 to MathItalicsCorrectionInfo.
  final italicsOffset = readU16(buf, glyphInfo);
  if (italicsOffset == null) throw const BufferTooShort();
  if (italicsOffset == 0) return null;
  final italics = glyphInfo + italicsOffset;

  // MathItalicsCorrectionInfo: Offset16 coverage, uint16 count, then that
  // many MathValue records.
  final coverageOffset = readU16(buf, italics);
  final count = readU16(buf, italics + 2);
  if (coverageOffset == null || count == null) {
    throw const BufferTooShort();
  }
  final index = _coverageIndex(buf, italics + coverageOffset, glyphIdValue);
  if (index == null) return null;
  if (index >= count) {
    // The coverage table and the value array disagree. That is a corrupt
    // font rather than a missing glyph, so say so instead of returning
    // `null` and letting a wrong-but-plausible zero flow onward.
    throw const BufferTooShort();
  }
  final v = readI16(buf, italics + 4 + index * 4);
  if (v == null) throw const BufferTooShort();
  return v;
}

// ── Coverage tables ──────────────────────────────────────────────────────

/// Finds a glyph's index within an OpenType coverage table.
///
/// Format 1 is an explicit sorted list; format 2 is sorted ranges each
/// carrying the index its first glyph maps to. Both scan linearly: the
/// tables are small, and a binary search over a format-2 range list is a
/// classic off-by-one site — and this returns an index used to offset into
/// a value array, where being one out yields a wrong number, not an error.
///
/// An unknown coverage format is corruption, not absence.
int? _coverageIndex(Uint8List buf, int offset, int glyphIdValue) {
  final format = readU16(buf, offset);
  if (format == null) throw const BufferTooShort();
  if (format == 1) {
    final count = readU16(buf, offset + 2);
    if (count == null) throw const BufferTooShort();
    for (var i = 0; i < count; i++) {
      final gid = readU16(buf, offset + 4 + i * 2);
      if (gid == null) throw const BufferTooShort();
      if (gid == glyphIdValue) return i;
    }
    return null;
  }
  if (format == 2) {
    final rangeCount = readU16(buf, offset + 2);
    if (rangeCount == null) throw const BufferTooShort();
    for (var i = 0; i < rangeCount; i++) {
      final rec = offset + 4 + i * 6;
      final start = readU16(buf, rec);
      final end = readU16(buf, rec + 2);
      final startIndex = readU16(buf, rec + 4);
      if (start == null || end == null || startIndex == null) {
        throw const BufferTooShort();
      }
      if (glyphIdValue >= start && glyphIdValue <= end) {
        return startIndex + (glyphIdValue - start);
      }
    }
    return null;
  }
  throw const BufferTooShort();
}

// ── MathVariants ─────────────────────────────────────────────────────────

/// The minimum overlap between adjacent assembly parts, in design units.
///
/// Parts must overlap by at least this much or the seams show as hairline
/// gaps at some sizes — the defect that looks like a rendering artefact
/// rather than a layout bug.
///
/// Returns `null` when the font has no `MATH` table, and `0` when the
/// table has no MathVariants subtable.
int? minConnectorOverlap(FontFile font) {
  final mathOffset = font._tables.math;
  if (mathOffset == null) return null;
  final buf = font._data;
  final variantsOffset = readU16(buf, mathOffset + 8);
  if (variantsOffset == null) throw const BufferTooShort();
  if (variantsOffset == 0) return 0;
  final v = readU16(buf, mathOffset + variantsOffset);
  if (v == null) throw const BufferTooShort();
  return v;
}

/// How a glyph grows to fit its contents, from the `MATH` table.
///
/// This is what makes `\left(` wrap a tall fraction instead of staying the
/// size of an ordinary parenthesis. Returns `null` when the font offers
/// this glyph no construction on that axis — the common case, since only
/// delimiters and a few operators stretch.
///
/// Throws [ArgumentError] if [glyphIdValue] does not fit in 16 bits.
GlyphConstruction? glyphConstruction(
    FontFile font, int glyphIdValue, StretchAxis axis) {
  if (glyphIdValue < 0 || glyphIdValue > 0xFFFF) {
    throw ArgumentError.value(
        glyphIdValue, 'glyphId', 'must fit in 16 bits unsigned');
  }
  final mathOffset = font._tables.math;
  if (mathOffset == null) return null;
  return _readGlyphConstruction(font._data, mathOffset, glyphIdValue, axis);
}

GlyphConstruction? _readGlyphConstruction(
    Uint8List buf, int mathOffset, int glyphIdValue, StretchAxis axis) {
  // MATH header: majorVersion, minorVersion, then three Offset16 —
  // MathConstants at 4, MathGlyphInfo at 6, MathVariants at 8. Reading +6
  // here parses MathGlyphInfo as MathVariants: it does not fail, it
  // returns plausible numbers from the wrong table.
  final variantsOffset = readU16(buf, mathOffset + 8);
  if (variantsOffset == null) throw const BufferTooShort();
  if (variantsOffset == 0) return null;
  final variants = mathOffset + variantsOffset;

  // MathVariants: minConnectorOverlap, vertCoverage, horizCoverage,
  // vertCount, horizCount, then the two construction-offset arrays.
  final vertCoverage = readU16(buf, variants + 2);
  final horizCoverage = readU16(buf, variants + 4);
  final vertCount = readU16(buf, variants + 6);
  if (vertCoverage == null || horizCoverage == null || vertCount == null) {
    throw const BufferTooShort();
  }

  final int coverage;
  final int countOffset;
  final int arrayOffset;
  if (axis == StretchAxis.vertical) {
    coverage = vertCoverage;
    countOffset = variants + 6;
    arrayOffset = variants + 10;
  } else {
    // The horizontal array follows the vertical one, so its start depends
    // on how many vertical constructions there are.
    coverage = horizCoverage;
    countOffset = variants + 8;
    arrayOffset = variants + 10 + vertCount * 2;
  }
  if (coverage == 0) return null;
  final count = readU16(buf, countOffset);
  if (count == null) throw const BufferTooShort();

  final index = _coverageIndex(buf, variants + coverage, glyphIdValue);
  if (index == null) return null;
  if (index >= count) {
    // Coverage and the construction array disagree: the table is
    // malformed, and reporting "no construction" is safer than reading an
    // offset from beyond the array.
    return null;
  }

  final constructionOffset = readU16(buf, arrayOffset + index * 2);
  if (constructionOffset == null) throw const BufferTooShort();
  if (constructionOffset == 0) return null;
  final construction = variants + constructionOffset;

  // MathGlyphConstruction: Offset16 glyphAssembly, uint16 variantCount,
  // then the variant records.
  final assemblyOffset = readU16(buf, construction);
  final variantCount = readU16(buf, construction + 2);
  if (assemblyOffset == null || variantCount == null) {
    throw const BufferTooShort();
  }

  final records = <GlyphVariant>[];
  for (var i = 0; i < variantCount; i++) {
    final record = construction + 4 + i * 4;
    final gid = readU16(buf, record);
    final advance = readU16(buf, record + 2);
    if (gid == null || advance == null) throw const BufferTooShort();
    records.add(GlyphVariant(glyphId: gid, advance: advance));
  }

  final assembly = <AssemblyPart>[];
  var assemblyItalicsCorrection = 0;
  if (assemblyOffset != 0) {
    final table = construction + assemblyOffset;
    // GlyphAssembly: a MathValueRecord (int16 + Offset16 device table),
    // then partCount, then the part records. Reading the italics
    // correction as a bare int16 would put every part record two bytes
    // early — the same trap `MathConstants` has.
    final italics = readI16(buf, table);
    final partCount = readU16(buf, table + 4);
    if (italics == null || partCount == null) throw const BufferTooShort();
    assemblyItalicsCorrection = italics;
    for (var i = 0; i < partCount; i++) {
      final record = table + 6 + i * 10;
      final gid = readU16(buf, record);
      final startConnector = readU16(buf, record + 2);
      final endConnector = readU16(buf, record + 4);
      final fullAdvance = readU16(buf, record + 6);
      final flags = readU16(buf, record + 8);
      if (gid == null ||
          startConnector == null ||
          endConnector == null ||
          fullAdvance == null ||
          flags == null) {
        throw const BufferTooShort();
      }
      assembly.add(AssemblyPart(
        glyphId: gid,
        startConnector: startConnector,
        endConnector: endConnector,
        fullAdvance: fullAdvance,
        // Bit 0 of partFlags; the other bits are reserved.
        isExtender: flags & 0x0001 != 0,
      ));
    }
  }

  return GlyphConstruction(
    variants: records,
    assembly: assembly,
    assemblyItalicsCorrection: assemblyItalicsCorrection,
  );
}

/// Whether the font carries a `MATH` table at all.
bool hasMathTable(FontFile font) => font._tables.math != null;
