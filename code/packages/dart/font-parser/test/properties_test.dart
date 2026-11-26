/// Property tests for `coding_adventures_font_parser`, using the in-lane
/// `pbt` package: the fail-closed vocabulary over hostile bytes, kern
/// round-trips over random pair sets, name round-trips over random BMP
/// strings, query purity, glyph-ID range agreement, table bounds, load
/// detachment, and STIX construction shape — all with fixed seeds.
library;

import 'dart:typed_data';

import 'package:coding_adventures_font_parser/font_parser.dart';
import 'package:coding_adventures_pbt/coding_adventures_pbt.dart';
import 'package:test/test.dart';

import 'font_fixtures.dart';

void check(String name, Verdict verdict) {
  if (verdict is Fail) {
    fail('$name falsified: [${verdict.failureId}] ${verdict.message}');
  }
}

bool equalBytes(List<int> a, List<int> b) {
  if (a.length != b.length) return false;
  for (var i = 0; i < a.length; i++) {
    if (a[i] != b[i]) return false;
  }
  return true;
}

/// Runs [query] and reports whether it threw outside the expected
/// vocabulary: [FontError] for corrupt data, [ArgumentError] for
/// out-of-range integers.
bool throwsUnexpected(Object? Function() query) {
  try {
    query();
    return false;
  } on FontError {
    return false;
  } on ArgumentError {
    return false;
  }
}

void main() {
  group('properties', () {
    test('load fails closed on hostile bytes', () {
      check(
        'load vocabulary',
        checkProperty<Uint8List>(
          BigInt.from(1101),
          PbtOptions(numTests: 300, seed: BigInt.from(1101)),
          (rng) => Gen.bytes(0, 300).sample(rng),
          (input) => !throwsUnexpected(() => load(input)),
        ),
      );
    });

    test('every query fails closed on hostile fonts', () {
      // Whatever load accepts, the query battery must answer or fail
      // with FontError/ArgumentError — never RangeError, StateError, or
      // any other escape.
      check(
        'query vocabulary',
        checkProperty<Uint8List>(
          BigInt.from(1102),
          PbtOptions(numTests: 300, seed: BigInt.from(1102)),
          (rng) => Gen.bytes(0, 300).sample(rng),
          (input) {
            FontFile? font;
            try {
              font = load(input);
            } on FontError {
              return true;
            }
            final f = font;
            final probe = [
              () => fontMetrics(f),
              () => glyphId(f, input.isEmpty ? 0 : input[0] * 257),
              () => glyphMetrics(
                  f, input.length < 2 ? 0 : (input[0] << 8) | input[1]),
              () => kerning(f, input.isEmpty ? 0 : input[0],
                  input.length < 2 ? 0 : input[1]),
              () => f
                  .table(input.length < 4 ? [0, 0, 0, 0] : input.sublist(0, 4)),
              () => f.indexToLocFormat(),
              () => f.numGlyphs(),
              () => hasMathTable(f),
              () => mathConstants(f),
              () => italicCorrection(f, input.isEmpty ? 0 : input[0] * 257),
              () => glyphConstruction(
                  f,
                  input.isEmpty ? 0 : input[0] * 257,
                  input.length < 2 || input[1].isEven
                      ? StretchAxis.vertical
                      : StretchAxis.horizontal),
              () => minConnectorOverlap(f),
            ];
            for (final query in probe) {
              if (throwsUnexpected(query)) return false;
            }
            return true;
          },
        ),
      );
    });

    test('kern pairs round-trip', () {
      final pairGen = Gen.tuple3(
        Gen.integer(BigInt.zero, BigInt.from(0xFFFF)),
        Gen.integer(BigInt.zero, BigInt.from(0xFFFF)),
        Gen.integer(BigInt.from(-32768), BigInt.from(32767)),
      );
      check(
        'kern round-trip',
        checkProperty<List<(BigInt, BigInt, BigInt)>>(
          BigInt.from(1103),
          PbtOptions(numTests: 120, seed: BigInt.from(1103)),
          (rng) => Gen.list(0, 12, pairGen).sample(rng),
          (pairs) {
            // Duplicate keys would race in the pair table; the builder
            // sorts, so dedup by key keeping the first occurrence.
            final seen = <int>{};
            final unique = <(int, int, int)>[];
            for (final (l, r, v) in pairs) {
              final key = (l.toInt() << 16) | r.toInt();
              if (seen.add(key)) unique.add((l.toInt(), r.toInt(), v.toInt()));
            }
            final font = load(buildSyntheticFontWithKern(unique));
            for (final (l, r, v) in unique) {
              if (kerning(font, l, r) != v) return false;
            }
            return true;
          },
        ),
      );
    });

    test('names round-trip over BMP strings', () {
      final unitGen = Gen.oneOf([
        Gen.integer(BigInt.from(1), BigInt.from(0xD7FF)),
        Gen.integer(BigInt.from(0xE000), BigInt.from(0xFFFF)),
      ]);
      check(
        'name round-trip',
        checkProperty<List<BigInt>>(
          BigInt.from(1104),
          PbtOptions(numTests: 120, seed: BigInt.from(1104)),
          (rng) => Gen.list(0, 16, unitGen).sample(rng),
          (units) {
            final text = String.fromCharCodes(units.map((u) => u.toInt()));
            final bytes = utf16be(text);
            final font = load(buildSyntheticFontWithKern(
              const [],
              nameStrings: [(3, 1, 1, bytes), (3, 1, 2, bytes)],
            ));
            final m = fontMetrics(font);
            return m.familyName == text && m.subfamilyName == text;
          },
        ),
      );
    });

    test('queries are pure', () {
      // Metric queries hold no hidden state: repeat calls agree exactly.
      check(
        'purity',
        checkProperty<(int, int)>(
          BigInt.from(1105),
          PbtOptions(numTests: 120, seed: BigInt.from(1105)),
          (rng) {
            final sample = Gen.tuple2(
                    Gen.integer(BigInt.zero, BigInt.from(0xFFFF)),
                    Gen.integer(BigInt.zero, BigInt.from(0xFFFF)))
                .sample(rng);
            return (sample.$1.toInt(), sample.$2.toInt());
          },
          (input) {
            final (a, b) = (input.$1.toInt(), input.$2.toInt());
            final inter = load(interRegular());
            final math = load(fontWithMathTable());
            if (fontMetrics(inter) != fontMetrics(inter)) return false;
            if (glyphId(inter, a) != glyphId(inter, a)) return false;
            if (glyphMetrics(inter, a) != glyphMetrics(inter, a)) {
              return false;
            }
            if (kerning(inter, a, b) != kerning(inter, a, b)) return false;
            if (mathConstants(math) != mathConstants(math)) return false;
            if (italicCorrection(math, a) != italicCorrection(math, a)) {
              return false;
            }
            if (glyphConstruction(math, a, StretchAxis.vertical) !=
                glyphConstruction(math, a, StretchAxis.vertical)) {
              return false;
            }
            return true;
          },
        ),
      );
    });

    test('glyph ids agree with numGlyphs', () {
      // Every mapped id is a real glyph: nonzero (never .notdef) and
      // below numGlyphs, with readable metrics.
      check(
        'id range agreement',
        checkProperty<int>(
          BigInt.from(1106),
          PbtOptions(numTests: 200, seed: BigInt.from(1106)),
          (rng) =>
              Gen.integer(BigInt.zero, BigInt.from(0xFFFF)).sample(rng).toInt(),
          (cp) {
            final font = load(interRegular());
            final gid = glyphId(font, cp);
            if (gid == null) return true;
            final n = fontMetrics(font).numGlyphs;
            if (gid <= 0 || gid >= n) return false;
            return glyphMetrics(font, gid) != null;
          },
        ),
      );
    });

    test('tables stay within the file', () {
      check(
        'table bounds',
        checkProperty<Uint8List>(
          BigInt.from(1107),
          PbtOptions(numTests: 200, seed: BigInt.from(1107)),
          (rng) => Gen.bytes(4, 4).sample(rng),
          (tag) {
            for (final font in [
              load(interRegular()),
              load(buildSyntheticFontWithKern(const [(1, 2, 3)])),
              load(fontWithMathTable()),
            ]) {
              final bytes = font.table(tag);
              if (bytes == null) continue;
              if (bytes.length > font.data.length) return false;
            }
            return true;
          },
        ),
      );
    });

    test('load detaches from the input buffer', () {
      check(
        'detachment',
        checkProperty<Uint8List>(
          BigInt.from(1108),
          PbtOptions(numTests: 120, seed: BigInt.from(1108)),
          (rng) => Gen.bytes(0, 300).sample(rng),
          (input) {
            FontFile? font;
            try {
              font = load(input);
            } on FontError {
              return true;
            }
            final before = fontMetrics(font);
            for (var i = 0; i < input.length; i++) {
              input[i] = 0xFF;
            }
            return fontMetrics(font) == before;
          },
        ),
      );
    });

    test('constructions grow monotonically', () {
      // Every STIX construction lists variants in increasing size: a
      // renderer picks the first big enough, so order is load-bearing.
      check(
        'variant order',
        checkProperty<int>(
          BigInt.from(1109),
          PbtOptions(numTests: 200, seed: BigInt.from(1109)),
          (rng) =>
              Gen.integer(BigInt.zero, BigInt.from(5000)).sample(rng).toInt(),
          (gid) {
            final font = load(fontWithMathTable());
            for (final axis in StretchAxis.values) {
              final construction = glyphConstruction(font, gid, axis);
              if (construction == null) continue;
              final sizes =
                  construction.variants.map((v) => v.advance).toList();
              for (var i = 1; i < sizes.length; i++) {
                if (sizes[i - 1] > sizes[i]) return false;
              }
            }
            return true;
          },
        ),
      );
    });
  });
}
