# coding_adventures_font_parser (Dart)

Metrics-only OpenType/TrueType font parser. The Dart lane of the Rust
`font-parser` crate.

```dart
import 'package:coding_adventures_font_parser/font_parser.dart';

// `bytes` comes from the caller; the package never touches the filesystem.
final font = load(bytes);
final metrics = fontMetrics(font);
print(metrics.unitsPerEm); // 2048 for Inter

final gidA = glyphId(font, 0x41)!;
final gidV = glyphId(font, 0x56)!;
print(kerning(font, gidA, gidV));
```

## API

- `load` parses font bytes into an opaque `FontFile` (bytes are copied
  in; the handle detaches from the caller's buffer).
- `fontMetrics` reads global metrics, preferring OS/2 typographic values
  over hhea; `glyphId` maps BMP codepoints via cmap Format 4;
  `glyphMetrics` reads hmtx; `kerning` searches kern Format 0.
- `hasMathTable` / `mathConstants` / `italicCorrection` /
  `glyphConstruction` / `minConnectorOverlap` read the OpenType MATH
  table (`StretchAxis.vertical` / `.horizontal`,
  `GlyphVariant` / `AssemblyPart` / `GlyphConstruction` value types).
- `FontFile.table` locates any table by 4-byte tag (for crates reading
  tables this one does not interpret); `indexToLocFormat` / `numGlyphs`
  expose the two outline-reader inputs.
- Errors: the sealed `FontError` with five subtypes carrying the
  reference's exact messages (thrown); `UnsupportedCmapFormat` is
  defined for parity and never thrown. Integers that do not fit the
  reference's u16/u32 slots throw `ArgumentError`, per lane convention.

No outlines, shaping, or rasterization — measuring text only.

## Deliberate divergences from the reference

- Truncated table-directory records read as "table absent"
  (`TableNotFound`) instead of panicking on the unchecked offset index.
- `glyphMetrics` answers `null` when `numberOfHMetrics` is 0 and a
  trailing glyph asks for the shared advance, instead of underflowing
  `num_h_metrics - 1`.
- UTF-16 names decode with explicit surrogate handling (unpaired
  surrogates become U+FFFD) since Dart strings can hold lone
  surrogates.

## Validation

- **Unit** (`test/font_parser_test.dart`): all 28 reference tests (Inter
  Regular metrics, cmap/hmtx/kern behavior over a synthetic font, byte
  readers) plus the `FontFile` surface, name/OS/2 matrices, and one
  test per hostile branch (truncated tables, disagreeing offsets).
- **Oracle** (`test/math_oracle_test.dart`): the 10 reference MATH
  oracle tests against fontTools values from STIX Two Math, plus two
  horizontal-construction oracles generated the same way (glyphs 746
  and 732) and the MATH corruption matrix.
- **Property** (`test/properties_test.dart`): fail-closed vocabulary,
  kern/name round-trips, query purity, id-range agreement, table
  bounds, load detachment, variant order — fixed seeds.
- **Fuzz** (`test/fuzz_test.dart`): 2,000-iteration hostile campaign;
  only `FontError` and `ArgumentError` escape.
- **Coverage**: 100% lines, branches, and functions.
