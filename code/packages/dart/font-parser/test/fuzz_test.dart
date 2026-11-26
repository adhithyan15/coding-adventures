/// Hostile-input fuzz campaign for `coding_adventures_font_parser`, using
/// the in-lane `fuzz` package: raw byte strings drive `load` plus the full
/// query battery, and the model asserts the fail-closed vocabulary (only
/// [FontError] and [ArgumentError] ever escape), glyph-id range agreement,
/// and table bounds. Fixed seed, so the campaign reproduces exactly.
library;

import 'dart:typed_data';

import 'package:coding_adventures_font_parser/font_parser.dart';
import 'package:coding_adventures_fuzz/coding_adventures_fuzz.dart';
import 'package:test/test.dart';

import 'font_fixtures.dart';

FuzzOutcome target(Uint8List input, CoverageBitmap cov) {
  cov.recordEdge(input.length % 64);
  FontFile font;
  try {
    font = load(input);
  } on FontError {
    cov.recordEdge(100);
    return const OutcomePass();
  } catch (e) {
    return OutcomeCrash('unexpected-escape', ['load:${e.runtimeType}']);
  }
  cov.recordEdge(101);

  try {
    // fontMetrics never throws, even on hostile-but-loadable fonts.
    final metrics = fontMetrics(font);
    cov.recordEdge(102 + (metrics.numGlyphs == 0 ? 0 : 1));

    final n = input.length;
    int byte(int i) => input[i % n];
    final cp = (byte(0) << 8) | byte(1);
    final gid = (byte(2) << 8) | byte(3);
    cov.recordEdge(110 + (cp % 8));

    // Glyph ids are null or real glyphs, with readable metrics.
    final mapped = glyphId(font, cp);
    if (mapped != null) {
      if (mapped <= 0 || mapped >= metrics.numGlyphs) {
        return const OutcomeCrash('model-divergence', ['gid-range']);
      }
      if (glyphMetrics(font, mapped) == null) {
        return const OutcomeCrash('model-divergence', ['gid-metrics']);
      }
      cov.recordEdge(120);
    }
    glyphMetrics(font, gid);
    kerning(font, byte(4), byte(5));
    cov.recordEdge(121);

    // Tables stay within the file.
    for (var t = 0; t + 4 <= n && t < 16; t += 4) {
      final bytes = font.table(input.sublist(t, t + 4));
      if (bytes != null && bytes.length > font.data.length) {
        return const OutcomeCrash('model-divergence', ['table-bounds']);
      }
    }
    cov.recordEdge(122);
    font.indexToLocFormat();
    font.numGlyphs();

    // MATH readers agree with the table's presence bit.
    final hasMath = hasMathTable(font);
    cov.recordEdge(hasMath ? 130 : 131);
    if (!hasMath) {
      if (mathConstants(font) != null ||
          italicCorrection(font, gid) != null ||
          minConnectorOverlap(font) != null ||
          glyphConstruction(font, gid, StretchAxis.vertical) != null ||
          glyphConstruction(font, gid, StretchAxis.horizontal) != null) {
        return const OutcomeCrash('model-divergence', ['math-absence']);
      }
    } else {
      mathConstants(font);
      italicCorrection(font, gid);
      minConnectorOverlap(font);
      final v = glyphConstruction(font, gid, StretchAxis.vertical);
      final h = glyphConstruction(font, gid, StretchAxis.horizontal);
      for (final construction in [v, h]) {
        if (construction == null) continue;
        final sizes = construction.variants.map((e) => e.advance).toList();
        for (var i = 1; i < sizes.length; i++) {
          if (sizes[i - 1] > sizes[i]) {
            return const OutcomeCrash('model-divergence', ['variant-order']);
          }
        }
      }
      cov.recordEdge(132);
    }

    cov.recordEdge(140 + (n % 8));
    return const OutcomePass();
  } on FontError {
    cov.recordEdge(150);
    return const OutcomePass();
  } on ArgumentError {
    cov.recordEdge(151);
    return const OutcomePass();
  } catch (e) {
    return OutcomeCrash('unexpected-escape', ['query:${e.runtimeType}']);
  }
}

void main() {
  test('hostile campaign finds no escapes or divergences', () {
    final inter = interRegular();
    final seeds = <Uint8List>[
      Uint8List(0),
      Uint8List.fromList(List.filled(12, 0)),
      Uint8List.fromList([0xDE, 0xAD, 0xBE, 0xEF, 0, 6]),
      buildSyntheticFontWithKern(const [(1, 2, -140), (3, 4, 80)]),
      buildSyntheticFontWithKern(
        const [],
        numGlyphs: 7,
        numHMetrics: 5,
        trailingLsb: [-10, 20],
        nameStrings: [
          (3, 1, 1, utf16be('Fuzz')),
          (0, 4, 2, utf16be('Family')),
        ],
        os2: (4, 900, -300, 50, 500, 700),
      ),
      fontWithMathTable().sublist(0, 2000),
      stixMath().sublist(0, 1000),
      inter.sublist(0, 1024),
      inter.sublist(0, 4096),
    ];
    final report = fuzzTarget(
      seeds,
      FuzzOptions(maxIterations: 2000, seed: BigInt.from(1109)),
      target,
    );
    expect(report.crashes, isEmpty,
        reason: report.crashes
            .map((c) => '${c.kind} ${c.frames.join(',')}')
            .join('\n'));
    expect(report.iterations, 2000);
  });
}
