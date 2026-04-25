/// Tests for the Micro QR Code encoder.
///
/// Test strategy from the spec:
///   1. Symbol dimensions (M1=11×11, M2=13×13, M3=15×15, M4=17×17)
///   2. Auto-version selection
///   3. Structural modules (finder, separator, timing)
///   4. Format information placement
///   5. Capacity boundaries
///   6. ECC level constraints
///   7. Encoding modes
///   8. Bit stream assembly
///   9. Masking and penalty scoring
///  10. Error handling
///  11. Determinism
///  12. Cross-language corpus
import 'package:test/test.dart';
import 'package:coding_adventures_micro_qr/coding_adventures_micro_qr.dart';
import 'package:coding_adventures_barcode_2d/coding_adventures_barcode_2d.dart';

/// Serialize a [ModuleGrid] to a plain-text string.
/// Each row is a sequence of '0' and '1' characters, rows separated by '\n'.
/// Used for snapshot comparison and cross-language verification.
String gridToString(ModuleGrid grid) {
  return grid.modules
      .map((row) => row.map((d) => d ? '1' : '0').join())
      .join('\n');
}

void main() {
  // ==========================================================================
  // 1. Symbol dimensions
  // ==========================================================================

  group('Symbol dimensions', () {
    test('M1 symbol is 11×11', () {
      final g = encode('1');
      expect(g.rows, equals(11));
      expect(g.cols, equals(11));
    });

    test('M2 symbol is 13×13', () {
      final g = encode('HELLO');
      expect(g.rows, equals(13));
      expect(g.cols, equals(13));
    });

    test('M3 symbol is 15×15', () {
      final g = encode('MICRO QR TEST');
      expect(g.rows, equals(15));
      expect(g.cols, equals(15));
    });

    test('M4 symbol is 17×17', () {
      final g = encode('https://a.b');
      expect(g.rows, equals(17));
      expect(g.cols, equals(17));
    });

    test('grid is always square', () {
      for (final input in ['1', 'HELLO', 'hello', 'https://a.b']) {
        final g = encode(input);
        expect(g.rows, equals(g.cols), reason: 'Grid should be square for "$input"');
      }
    });

    test('module shape is always square', () {
      final g = encode('1');
      expect(g.moduleShape, equals(ModuleShape.square));
    });

    test('modules 2D list has correct dimensions', () {
      for (final input in ['1', 'HELLO', 'hello', 'https://a.b']) {
        final g = encode(input);
        expect(g.modules.length, equals(g.rows),
            reason: 'Row count mismatch for "$input"');
        for (final row in g.modules) {
          expect(row.length, equals(g.cols),
              reason: 'Column count mismatch for "$input"');
        }
      }
    });
  });

  // ==========================================================================
  // 2. Auto-version selection
  // ==========================================================================

  group('Auto-version selection', () {
    test('single digit → M1', () {
      expect(encode('1').rows, equals(11));
    });

    test('12345 (5 digits, M1 max) → M1', () {
      expect(encode('12345').rows, equals(11));
    });

    test('6 digits → M2 (exceeds M1 capacity)', () {
      expect(encode('123456').rows, equals(13));
    });

    test('8 digits → M2-L (numeric_cap=10)', () {
      expect(encode('01234567').rows, equals(13));
    });

    test('HELLO (5 alphanumeric chars) → M2', () {
      expect(encode('HELLO').rows, equals(13));
    });

    test('hello (5 lowercase byte chars) → M3 or higher', () {
      // 'hello' = 5 bytes; M2-L byte_cap=4 → need M3-L (byte_cap=9)
      final g = encode('hello');
      expect(g.rows, greaterThanOrEqualTo(15));
    });

    test('https://a.b (11 bytes) → M4', () {
      // M3-L byte_cap=9 < 11 → need M4-L (byte_cap=15)
      expect(encode('https://a.b').rows, equals(17));
    });

    test('MICRO QR TEST (13 alphanumeric) → M3-L', () {
      // M2-L alpha_cap=6 < 13; M3-L alpha_cap=14 ≥ 13 → M3
      expect(encode('MICRO QR TEST').rows, equals(15));
    });

    test('forced version M4 for small input', () {
      final g = encode('1', version: MicroQRVersion.m4);
      expect(g.rows, equals(17));
    });

    test('forced ECC L vs M produce different grids', () {
      final gl = encode('HELLO', ecc: MicroQREccLevel.l);
      final gm = encode('HELLO', ecc: MicroQREccLevel.m);
      expect(gridToString(gl), isNot(equals(gridToString(gm))));
    });

    test('empty string → M1', () {
      // Empty string: length 0 ≤ numericCap=5 for M1
      expect(encode('').rows, equals(11));
    });
  });

  // ==========================================================================
  // 3. Structural modules — finder pattern
  // ==========================================================================

  group('Finder pattern', () {
    test('outer border of finder is all dark (M1)', () {
      final m = encode('1').modules;
      // Top row of finder
      for (var c = 0; c < 7; c++) {
        expect(m[0][c], isTrue, reason: 'row 0 col $c should be dark');
      }
      // Bottom row of finder
      for (var c = 0; c < 7; c++) {
        expect(m[6][c], isTrue, reason: 'row 6 col $c should be dark');
      }
      // Left column of finder
      for (var r = 0; r < 7; r++) {
        expect(m[r][0], isTrue, reason: 'col 0 row $r should be dark');
      }
      // Right column of finder
      for (var r = 0; r < 7; r++) {
        expect(m[r][6], isTrue, reason: 'col 6 row $r should be dark');
      }
    });

    test('inner ring of finder is all light (M1)', () {
      final m = encode('1').modules;
      // Row 1 inner (cols 1-5)
      for (var c = 1; c <= 5; c++) {
        expect(m[1][c], isFalse, reason: 'inner ring row 1 col $c should be light');
      }
      // Row 5 inner (cols 1-5)
      for (var c = 1; c <= 5; c++) {
        expect(m[5][c], isFalse, reason: 'inner ring row 5 col $c should be light');
      }
      // Col 1 inner rows 2-4
      for (var r = 2; r <= 4; r++) {
        expect(m[r][1], isFalse, reason: 'inner ring col 1 row $r should be light');
      }
      // Col 5 inner rows 2-4
      for (var r = 2; r <= 4; r++) {
        expect(m[r][5], isFalse, reason: 'inner ring col 5 row $r should be light');
      }
    });

    test('3×3 core of finder is all dark (M1)', () {
      final m = encode('1').modules;
      for (var r = 2; r <= 4; r++) {
        for (var c = 2; c <= 4; c++) {
          expect(m[r][c], isTrue, reason: 'core ($r,$c) should be dark');
        }
      }
    });

    test('finder pattern is identical across symbol sizes', () {
      // All symbols share the same 7×7 finder in the top-left corner.
      for (final input in ['1', 'HELLO', 'MICRO QR TEST', 'https://a.b']) {
        final m = encode(input).modules;
        // Spot-check: top-left corner always dark
        expect(m[0][0], isTrue, reason: 'finder TL corner for "$input"');
        // Inner ring always light
        expect(m[1][1], isFalse, reason: 'finder inner ring for "$input"');
        // Core always dark
        expect(m[3][3], isTrue, reason: 'finder core for "$input"');
      }
    });
  });

  // ==========================================================================
  // 4. Structural modules — separator
  // ==========================================================================

  group('Separator (L-shape)', () {
    test('row 7 cols 0-7 all light (M2)', () {
      final m = encode('HELLO').modules;
      for (var c = 0; c <= 7; c++) {
        expect(m[7][c], isFalse, reason: 'separator row 7 col $c should be light');
      }
    });

    test('col 7 rows 0-7 all light (M2)', () {
      final m = encode('HELLO').modules;
      for (var r = 0; r <= 7; r++) {
        expect(m[r][7], isFalse, reason: 'separator col 7 row $r should be light');
      }
    });

    test('separator present in all symbol sizes', () {
      for (final input in ['1', 'HELLO', 'MICRO QR TEST', 'https://a.b']) {
        final m = encode(input).modules;
        expect(m[7][0], isFalse, reason: 'sep row 7 col 0 for "$input"');
        expect(m[0][7], isFalse, reason: 'sep col 7 row 0 for "$input"');
        expect(m[7][7], isFalse, reason: 'sep corner (7,7) for "$input"');
      }
    });
  });

  // ==========================================================================
  // 5. Structural modules — timing patterns
  // ==========================================================================

  group('Timing patterns', () {
    test('timing row 0 (cols 8+) alternates dark/light for M4', () {
      final m = encode('https://a.b').modules;
      for (var c = 8; c < 17; c++) {
        expect(m[0][c], equals(c % 2 == 0),
            reason: 'timing row 0 col $c should be ${c % 2 == 0 ? "dark" : "light"}');
      }
    });

    test('timing col 0 (rows 8+) alternates dark/light for M4', () {
      final m = encode('https://a.b').modules;
      for (var r = 8; r < 17; r++) {
        expect(m[r][0], equals(r % 2 == 0),
            reason: 'timing col 0 row $r should be ${r % 2 == 0 ? "dark" : "light"}');
      }
    });

    test('timing row 0 for M1 (cols 8-10)', () {
      final m = encode('1').modules;
      for (var c = 8; c < 11; c++) {
        expect(m[0][c], equals(c % 2 == 0),
            reason: 'timing row 0 col $c for M1');
      }
    });

    test('timing col 0 for M1 (rows 8-10)', () {
      final m = encode('1').modules;
      for (var r = 8; r < 11; r++) {
        expect(m[r][0], equals(r % 2 == 0),
            reason: 'timing col 0 row $r for M1');
      }
    });

    test('timing starts dark at index 8 (even)', () {
      // Index 8 is even → should be dark
      final m = encode('HELLO').modules; // M2, size 13
      expect(m[0][8], isTrue, reason: 'timing row 0 col 8 should be dark');
      expect(m[8][0], isTrue, reason: 'timing col 0 row 8 should be dark');
    });
  });

  // ==========================================================================
  // 6. Format information
  // ==========================================================================

  group('Format information', () {
    test('format info row 8 has some dark modules (M4)', () {
      final m = encode('HELLO', version: MicroQRVersion.m4, ecc: MicroQREccLevel.l).modules;
      final anyDark = Iterable.generate(8, (i) => i + 1).any((c) => m[8][c]);
      expect(anyDark, isTrue, reason: 'format info row 8 should have dark modules');
    });

    test('format info col 8 has some dark modules (M4)', () {
      final m = encode('HELLO', version: MicroQRVersion.m4, ecc: MicroQREccLevel.l).modules;
      final anyDark = Iterable.generate(7, (i) => i + 1).any((r) => m[r][8]);
      expect(anyDark, isTrue, reason: 'format info col 8 should have dark modules');
    });

    test('format info present in M1', () {
      final m = encode('1').modules;
      final count = Iterable.generate(8, (i) => i + 1).where((c) => m[8][c]).length +
          Iterable.generate(7, (i) => i + 1).where((r) => m[r][8]).length;
      expect(count, greaterThan(0), reason: 'M1 format info should have dark modules');
    });

    test('different ECC levels produce different format info', () {
      final ml = encode('HELLO', version: MicroQRVersion.m4, ecc: MicroQREccLevel.l).modules;
      final mm = encode('HELLO', version: MicroQRVersion.m4, ecc: MicroQREccLevel.m).modules;
      final mq = encode('HELLO', version: MicroQRVersion.m4, ecc: MicroQREccLevel.q).modules;

      // Check format info region (row 8 cols 1-8)
      final fmtL = Iterable.generate(8, (i) => ml[8][i + 1]).toList();
      final fmtM = Iterable.generate(8, (i) => mm[8][i + 1]).toList();
      final fmtQ = Iterable.generate(8, (i) => mq[8][i + 1]).toList();

      expect(fmtL, isNot(equals(fmtM)), reason: 'M4-L and M4-M format info should differ');
      expect(fmtM, isNot(equals(fmtQ)), reason: 'M4-M and M4-Q format info should differ');
      expect(fmtL, isNot(equals(fmtQ)), reason: 'M4-L and M4-Q format info should differ');
    });

    test('M4-L mask 0 format word is 0x17F3', () {
      // To verify the format table: encode with a known input that selects mask 0
      // for M4-L. We can't guarantee which mask is selected, but we can verify
      // the format info is one of the 4 known M4-L values.
      final knownM4LValues = {0x17F3, 0x12C4, 0x1D9D, 0x18AA};
      final m = encode('HELLO', version: MicroQRVersion.m4, ecc: MicroQREccLevel.l).modules;

      // Read the 15-bit format word from the grid.
      var fmt = 0;
      for (var i = 0; i < 8; i++) {
        fmt = (fmt << 1) | (m[8][1 + i] ? 1 : 0);
      }
      for (var i = 0; i < 7; i++) {
        fmt = (fmt << 1) | (m[7 - i][8] ? 1 : 0);
      }
      expect(knownM4LValues.contains(fmt), isTrue,
          reason: 'Format word 0x${fmt.toRadixString(16)} should be a known M4-L value');
    });

    test('M1 format word is one of 4 known values', () {
      final knownM1Values = {0x4445, 0x4172, 0x4E2B, 0x4B1C};
      final m = encode('1').modules;

      var fmt = 0;
      for (var i = 0; i < 8; i++) {
        fmt = (fmt << 1) | (m[8][1 + i] ? 1 : 0);
      }
      for (var i = 0; i < 7; i++) {
        fmt = (fmt << 1) | (m[7 - i][8] ? 1 : 0);
      }
      expect(knownM1Values.contains(fmt), isTrue,
          reason: 'M1 format word 0x${fmt.toRadixString(16)} should be a known M1 value');
    });
  });

  // ==========================================================================
  // 7. Capacity boundaries
  // ==========================================================================

  group('Capacity boundaries', () {
    test('M1 max: 5 numeric digits fits', () {
      expect(encode('12345').rows, equals(11));
    });

    test('M1 overflow: 6 numeric digits → M2', () {
      expect(encode('123456').rows, equals(13));
    });

    test('M4 max numeric: 35 digits fits in M4', () {
      final g = encode('1' * 35);
      expect(g.rows, equals(17));
    });

    test('M4 overflow: 36 digits → InputTooLong', () {
      expect(() => encode('1' * 36), throwsA(isA<InputTooLong>()));
    });

    test('M4 max byte: 15 chars', () {
      // 15 lowercase letters = 15 bytes, M4-L byte_cap=15
      final g = encode('a' * 15);
      expect(g.rows, equals(17));
    });

    test('M4-Q max numeric: 21 digits', () {
      final g = encode('1' * 21, ecc: MicroQREccLevel.q);
      expect(g.rows, equals(17));
    });

    test('M2-L max byte: 4 chars', () {
      // 4 lowercase = 4 bytes, M2-L byte_cap=4
      final g = encode('abcd');
      expect(g.rows, equals(13));
    });

    test('M2-L overflow byte: 5 chars → M3', () {
      // 5 bytes > M2-L byte_cap=4 → needs M3
      final g = encode('abcde');
      expect(g.rows, greaterThanOrEqualTo(15));
    });

    test('M2-L max alphanumeric: 6 chars', () {
      // 6 uppercase = M2-L alpha_cap=6
      final g = encode('ABCDEF');
      expect(g.rows, equals(13));
    });

    test('M4-Q max byte: 9 bytes', () {
      final g = encode('a' * 9, ecc: MicroQREccLevel.q);
      expect(g.rows, equals(17));
    });
  });

  // ==========================================================================
  // 8. ECC level constraints
  // ==========================================================================

  group('ECC level constraints', () {
    test('M1 detection-only ECC works', () {
      final g = encode('1', version: MicroQRVersion.m1, ecc: MicroQREccLevel.detection);
      expect(g.rows, equals(11));
    });

    test('M1 rejects ECC L', () {
      expect(
        () => encode('1', version: MicroQRVersion.m1, ecc: MicroQREccLevel.l),
        throwsA(isA<ECCNotAvailable>()),
      );
    });

    test('M1 rejects ECC M', () {
      expect(
        () => encode('1', version: MicroQRVersion.m1, ecc: MicroQREccLevel.m),
        throwsA(isA<ECCNotAvailable>()),
      );
    });

    test('M1 rejects ECC Q', () {
      expect(
        () => encode('1', version: MicroQRVersion.m1, ecc: MicroQREccLevel.q),
        throwsA(isA<ECCNotAvailable>()),
      );
    });

    test('M2 rejects ECC Q', () {
      expect(
        () => encode('1', version: MicroQRVersion.m2, ecc: MicroQREccLevel.q),
        throwsA(isA<ECCNotAvailable>()),
      );
    });

    test('M3 rejects ECC Q', () {
      expect(
        () => encode('1', version: MicroQRVersion.m3, ecc: MicroQREccLevel.q),
        throwsA(isA<ECCNotAvailable>()),
      );
    });

    test('M4-Q works', () {
      final g = encode('HELLO', version: MicroQRVersion.m4, ecc: MicroQREccLevel.q);
      expect(g.rows, equals(17));
    });

    test('M4 all three correcting levels produce different symbols', () {
      final gl = encode('HELLO', version: MicroQRVersion.m4, ecc: MicroQREccLevel.l);
      final gm = encode('HELLO', version: MicroQRVersion.m4, ecc: MicroQREccLevel.m);
      final gq = encode('HELLO', version: MicroQRVersion.m4, ecc: MicroQREccLevel.q);
      expect(gridToString(gl), isNot(equals(gridToString(gm))));
      expect(gridToString(gm), isNot(equals(gridToString(gq))));
      expect(gridToString(gl), isNot(equals(gridToString(gq))));
    });

    test('M2 L and M levels produce different symbols', () {
      final gl = encode('HELLO', version: MicroQRVersion.m2, ecc: MicroQREccLevel.l);
      final gm = encode('HELLO', version: MicroQRVersion.m2, ecc: MicroQREccLevel.m);
      expect(gridToString(gl), isNot(equals(gridToString(gm))));
    });

    test('non-existent combo M1+Q throws ECCNotAvailable', () {
      expect(
        () => encode('1', version: MicroQRVersion.m1, ecc: MicroQREccLevel.q),
        throwsA(isA<ECCNotAvailable>()),
      );
    });
  });

  // ==========================================================================
  // 9. Encoding modes
  // ==========================================================================

  group('Encoding modes', () {
    test('all-digit input uses numeric mode', () {
      // Verify that a digit-only string fits in M1 (numeric only)
      final g = encode('12345');
      expect(g.rows, equals(11)); // M1 is numeric-only
    });

    test('uppercase alphanumeric input', () {
      // "HELLO" = 5 chars in alphanumeric set → M2-L
      final g = encode('HELLO');
      expect(g.rows, equals(13));
    });

    test('lowercase input uses byte mode', () {
      // Lowercase not in alphanumeric set → byte mode required
      final g = encode('hello');
      expect(g.rows, greaterThanOrEqualTo(15)); // needs M3+ for byte mode
    });

    test('mixed-case URL uses byte mode', () {
      final g = encode('https://a.b'); // lowercase → byte mode
      expect(g.rows, equals(17));
    });

    test(r'alphanumeric chars: $%*+-./: are encodable', () {
      // All 45 alphanumeric chars including symbols
      // Note: the $ in the string is the dollar-sign from the alphanumeric set
      final g = encode(r'A1 $%*+-./:', version: MicroQRVersion.m4);
      expect(g.rows, equals(17));
    });

    test('M1 rejects alphanumeric input', () {
      // "HELLO" is 5 alphanumeric chars; M1 only supports numeric
      // Auto-selection will pick M2 instead — so force M1 to trigger error
      expect(
        () => encode('HELLO', version: MicroQRVersion.m1),
        throwsA(isA<InputTooLong>()), // no mode fits in M1 for alpha input
      );
    });

    test('numeric-only string in M1 vs M4 produces different sizes', () {
      final m1 = encode('1');
      final m4 = encode('1', version: MicroQRVersion.m4);
      expect(m1.rows, equals(11));
      expect(m4.rows, equals(17));
    });

    test('spaces (in alphanumeric set) are encodable', () {
      // Space (0x20) is index 36 in the alphanumeric set
      final g = encode('MICRO QR TEST');
      expect(g.rows, equals(15)); // M3-L
    });
  });

  // ==========================================================================
  // 10. Error handling
  // ==========================================================================

  group('Error handling', () {
    test('InputTooLong for 36 numeric digits', () {
      expect(() => encode('1' * 36), throwsA(isA<InputTooLong>()));
    });

    test('InputTooLong message is descriptive', () {
      try {
        encode('1' * 36);
        fail('Expected InputTooLong');
      } on InputTooLong catch (e) {
        expect(e.message, contains('35'));
      }
    });

    test('ECCNotAvailable for M1+L', () {
      expect(
        () => encode('1', version: MicroQRVersion.m1, ecc: MicroQREccLevel.l),
        throwsA(isA<ECCNotAvailable>()),
      );
    });

    test('ECCNotAvailable for M2+Q', () {
      expect(
        () => encode('HELLO', version: MicroQRVersion.m2, ecc: MicroQREccLevel.q),
        throwsA(isA<ECCNotAvailable>()),
      );
    });

    test('MicroQRError is base class of InputTooLong', () {
      expect(
        () => encode('1' * 36),
        throwsA(isA<MicroQRError>()),
      );
    });

    test('MicroQRError is base class of ECCNotAvailable', () {
      expect(
        () => encode('1', version: MicroQRVersion.m1, ecc: MicroQREccLevel.l),
        throwsA(isA<MicroQRError>()),
      );
    });

    test('error toString includes type name', () {
      try {
        encode('1' * 36);
        fail('Expected InputTooLong');
      } on InputTooLong catch (e) {
        expect(e.toString(), contains('InputTooLong'));
      }
    });
  });

  // ==========================================================================
  // 11. Determinism
  // ==========================================================================

  group('Determinism', () {
    test('same input produces identical grids', () {
      for (final input in ['1', '12345', 'HELLO', 'hello', 'https://a.b', 'MICRO QR TEST']) {
        final g1 = encode(input);
        final g2 = encode(input);
        expect(gridToString(g1), equals(gridToString(g2)),
            reason: 'Non-deterministic for "$input"');
      }
    });

    test('different inputs produce different grids (M1)', () {
      final g1 = encode('1');
      final g2 = encode('2');
      expect(gridToString(g1), isNot(equals(gridToString(g2))));
    });

    test('different inputs produce different grids (M2)', () {
      final g1 = encode('HELLO');
      final g2 = encode('WORLD');
      expect(gridToString(g1), isNot(equals(gridToString(g2))));
    });
  });

  // ==========================================================================
  // 12. Cross-language corpus
  // ==========================================================================

  group('Cross-language corpus', () {
    // These test cases are the spec's cross-language verification corpus.
    // All implementations must produce bit-for-bit identical grids for these.
    const testCases = [
      ('1', 11),           // M1: minimal numeric
      ('12345', 11),       // M1: maximum numeric capacity
      ('HELLO', 13),       // M2-L: alphanumeric
      ('01234567', 13),    // M2-L: numeric 8 digits
      ('https://a.b', 17), // M4-L: byte mode URL
      ('MICRO QR TEST', 15), // M3-L: alphanumeric 13 chars
    ];

    for (final (input, expectedSize) in testCases) {
      test('corpus: "$input" → ${expectedSize}×$expectedSize', () {
        final g = encode(input);
        expect(g.rows, equals(expectedSize),
            reason: '"$input" expected ${expectedSize}×$expectedSize '
                'but got ${g.rows}×${g.cols}');
        expect(g.cols, equals(expectedSize));
      });
    }
  });

  // ==========================================================================
  // 13. encodeAt helper
  // ==========================================================================

  group('encodeAt helper', () {
    test('encodeAt produces same result as encode with forced version+ecc', () {
      final g1 = encode('HELLO', version: MicroQRVersion.m4, ecc: MicroQREccLevel.l);
      final g2 = encodeAt('HELLO', MicroQRVersion.m4, MicroQREccLevel.l);
      expect(gridToString(g1), equals(gridToString(g2)));
    });

    test('encodeAt M1 detection', () {
      final g = encodeAt('12345', MicroQRVersion.m1, MicroQREccLevel.detection);
      expect(g.rows, equals(11));
    });

    test('encodeAt M3 L', () {
      final g = encodeAt('HELLO', MicroQRVersion.m3, MicroQREccLevel.l);
      expect(g.rows, equals(15));
    });
  });

  // ==========================================================================
  // 14. layoutGrid and encodeAndLayout
  // ==========================================================================

  group('Layout functions', () {
    test('layoutGrid returns a PaintScene', () {
      final grid = encode('HELLO');
      final scene = layoutGrid(grid);
      // A 13×13 grid with 2-module quiet zone and 10px modules:
      // totalWidth = (13 + 2*2) * 10 = 170
      expect(scene.width, equals(170));
      expect(scene.height, equals(170));
    });

    test('layoutGrid with custom config', () {
      final grid = encode('HELLO');
      const config = Barcode2DLayoutConfig(
        moduleSizePx: 5,
        quietZoneModules: 2,
        foreground: '#000000',
        background: '#ffffff',
        moduleShape: ModuleShape.square,
      );
      final scene = layoutGrid(grid, config: config);
      expect(scene.width, equals((13 + 4) * 5)); // (13 + 2*2) * 5 = 85
    });

    test('encodeAndLayout produces a PaintScene', () {
      final scene = encodeAndLayout('HELLO');
      expect(scene.width, greaterThan(0));
      expect(scene.height, greaterThan(0));
    });

    test('encodeAndLayout M4 produces wider scene', () {
      final scene4 = encodeAndLayout('https://a.b'); // M4: 17×17
      final scene2 = encodeAndLayout('HELLO');       // M2: 13×13
      expect(scene4.width, greaterThan(scene2.width));
    });

    test('layoutGrid default quiet zone is 2', () {
      final grid = encode('1'); // M1: 11×11
      final scene = layoutGrid(grid);
      // width = (11 + 2*2) * 10 = 150
      expect(scene.width, equals(150));
    });
  });

  // ==========================================================================
  // 15. Grid completeness
  // ==========================================================================

  group('Grid completeness', () {
    test('no null modules in M1', () {
      final g = encode('1');
      for (var r = 0; r < g.rows; r++) {
        for (var c = 0; c < g.cols; c++) {
          // Accessing modules[r][c] should not throw
          expect(g.modules[r][c], isA<bool>());
        }
      }
    });

    test('no null modules in M4', () {
      final g = encode('https://a.b');
      for (var r = 0; r < g.rows; r++) {
        for (var c = 0; c < g.cols; c++) {
          expect(g.modules[r][c], isA<bool>());
        }
      }
    });

    test('version package constant is non-empty', () {
      // Use package-qualified name to avoid ambiguity with barcode_2d's version
      expect(microQrVersion, isNotEmpty);
    });
  });

  // ==========================================================================
  // 16. Bit stream and RS ECC (indirect tests via structural invariants)
  // ==========================================================================

  group('RS ECC and bit stream (structural invariants)', () {
    test('different data payloads produce different grids (not just different sizes)', () {
      // Two M2-L alphanumeric inputs of the same length
      final g1 = encode('HELLO', version: MicroQRVersion.m2, ecc: MicroQREccLevel.l);
      final g2 = encode('WORLD', version: MicroQRVersion.m2, ecc: MicroQREccLevel.l);
      // Structural modules (finder, separator, timing) are identical;
      // data + ECC modules differ.
      expect(gridToString(g1), isNot(equals(gridToString(g2))));
    });

    test('M1 finder and timing unchanged regardless of data', () {
      // Structural modules are always the same for a given symbol version.
      final g1 = encode('1');
      final g2 = encode('2');
      final m1 = g1.modules;
      final m2 = g2.modules;

      // Finder pattern: top and bottom rows of finder
      for (var c = 0; c < 7; c++) {
        expect(m1[0][c], equals(m2[0][c]),
            reason: 'finder row 0 col $c should be same for "1" and "2"');
        expect(m1[6][c], equals(m2[6][c]),
            reason: 'finder row 6 col $c should be same for "1" and "2"');
      }

      // Timing: row 0 cols 8-10
      for (var c = 8; c < 11; c++) {
        expect(m1[0][c], equals(m2[0][c]),
            reason: 'timing row 0 col $c should be same for "1" and "2"');
      }
    });

    test('M1 12345 has exactly 11×11 = 121 modules', () {
      final g = encode('12345');
      expect(g.rows * g.cols, equals(121));
    });

    test('M4 symbol has exactly 17×17 = 289 modules', () {
      final g = encode('https://a.b');
      expect(g.rows * g.cols, equals(289));
    });
  });

  // ==========================================================================
  // 17. Masking (indirect tests)
  // ==========================================================================

  group('Masking', () {
    test('structural modules are not changed by different mask selections', () {
      // Encode the same input with different forced ECC (which can change mask
      // selection). Structural modules must remain identical.
      final gl = encode('HELLO', version: MicroQRVersion.m2, ecc: MicroQREccLevel.l);
      final gm = encode('HELLO', version: MicroQRVersion.m2, ecc: MicroQREccLevel.m);

      // Finder outer border should be identical
      for (var c = 0; c < 7; c++) {
        expect(gl.modules[0][c], isTrue, reason: 'finder row 0 col $c in L');
        expect(gm.modules[0][c], isTrue, reason: 'finder row 0 col $c in M');
      }

      // Separator row 7 should be all light in both
      for (var c = 0; c <= 7; c++) {
        expect(gl.modules[7][c], isFalse, reason: 'sep row 7 col $c in L');
        expect(gm.modules[7][c], isFalse, reason: 'sep row 7 col $c in M');
      }

      // Timing row 0 from col 8 onward
      for (var c = 8; c < 13; c++) {
        expect(gl.modules[0][c], equals(c % 2 == 0),
            reason: 'timing in L col $c');
        expect(gm.modules[0][c], equals(c % 2 == 0),
            reason: 'timing in M col $c');
      }
    });
  });
}
