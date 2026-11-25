/// Metrics-only OpenType/TrueType font parser.
///
/// [load] parses raw font bytes into a [FontFile]; [fontMetrics] reads global
/// metrics, [glyphId]/[glyphMetrics]/[kerning] answer per-glyph queries, and
/// [mathConstants]/[italicCorrection]/[glyphConstruction]/
/// [minConnectorOverlap]/[hasMathTable] read the OpenType MATH table. All
/// failures throw [FontError].
library;

export 'src/font_parser.dart' hide readU16, readI16, readU32;
