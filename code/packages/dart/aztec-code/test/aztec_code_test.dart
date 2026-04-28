/// Tests for the Dart Aztec Code encoder.
///
/// ## Test strategy
///
/// The suite is organised into groups that mirror the encoding pipeline:
///
///  1. Package metadata (version constant)
///  2. Error types (AztecError, InputTooLongError)
///  3. Symbol dimensions and grid invariants
///  4. Compact symbol selection (layers 1–4, 15×15 to 27×27)
///  5. Full symbol selection (layers 1–32, 19×19 to 143×143)
///  6. Determinism — same input always yields the same grid
///  7. Larger input produces a bigger symbol
///  8. InputTooLongError for oversized payloads
///  9. minEccPercent option influences symbol size
/// 10. Layout / PaintScene wrapper
/// 11. Structural invariants — modules are booleans, grid is square
import 'package:test/test.dart';
import 'package:coding_adventures_aztec_code/coding_adventures_aztec_code.dart';
import 'package:coding_adventures_barcode_2d/coding_adventures_barcode_2d.dart';

/// Serialize a [ModuleGrid] to a plain-text string for snapshot comparison.
///
/// Each row becomes a sequence of '1' (dark) and '0' (light) characters,
/// rows separated by '\n'.
String gridToString(ModuleGrid grid) {
  return grid.modules
      .map((row) => row.map((d) => d ? '1' : '0').join())
      .join('\n');
}

void main() {
  // ==========================================================================
  // 1. Package metadata
  // ==========================================================================

  group('Package metadata', () {
    test('aztecCodeVersion is 0.1.0', () {
      expect(aztecCodeVersion, equals('0.1.0'));
    });
  });

  // ==========================================================================
  // 2. Error types
  // ==========================================================================

  group('Error types', () {
    test('AztecError implements Exception', () {
      const err = AztecError('test error');
      expect(err, isA<Exception>());
      expect(err.message, equals('test error'));
    });

    test('AztecError toString includes message', () {
      const err = AztecError('something went wrong');
      expect(err.toString(), contains('something went wrong'));
    });

    test('InputTooLongError extends AztecError', () {
      const err = InputTooLongError('too long');
      expect(err, isA<AztecError>());
      expect(err, isA<Exception>());
      expect(err.message, equals('too long'));
    });

    test('InputTooLongError toString includes message', () {
      const err = InputTooLongError('exceeded capacity');
      expect(err.toString(), contains('exceeded capacity'));
    });
  });

  // ==========================================================================
  // 3. Symbol dimensions and grid invariants
  // ==========================================================================

  group('Symbol dimensions and grid invariants', () {
    test('encode returns a non-null ModuleGrid', () {
      final grid = encode('IATA BP DATA');
      // If we reach this line, encode() did not throw and returned a value.
      expect(grid.rows, greaterThan(0));
    });

    test('grid is always square (rows == cols)', () {
      for (final input in ['A', 'HELLO WORLD', '12345678', 'IATA BP DATA']) {
        final grid = encode(input);
        expect(
          grid.rows,
          equals(grid.cols),
          reason: 'Grid should be square for "$input"',
        );
      }
    });

    test('modules 2D list outer length equals rows', () {
      final grid = encode('HELLO');
      expect(grid.modules.length, equals(grid.rows));
    });

    test('modules 2D list inner length equals cols', () {
      final grid = encode('HELLO');
      for (final row in grid.modules) {
        expect(row.length, equals(grid.cols));
      }
    });

    test('all modules are booleans (true or false)', () {
      final grid = encode('IATA BP DATA');
      for (final row in grid.modules) {
        for (final module in row) {
          // A Dart bool can only be true or false — this assertion catches
          // any accidental int or null leaking into the grid.
          expect(module, isA<bool>());
        }
      }
    });

    test('module shape is ModuleShape.square', () {
      final grid = encode('A');
      expect(grid.moduleShape, equals(ModuleShape.square));
    });

    test('grid dimensions match the Aztec size formula', () {
      // Compact symbols: size = 11 + 4×layers (15, 19, 23, 27).
      // Full symbols:    size = 15 + 4×layers (19, 23, ..., 143).
      // We cannot know the exact layer count without re-implementing
      // _selectSymbol, but we can assert the size follows the formula
      // for one-character inputs (always compact-1 = 15×15).
      final grid = encode('A');
      // 'A' is 1 byte → 10 data bits (5-bit escape + 5-bit length + 8-bit byte).
      // At 23% ECC, compact-1 (9 codewords, 7 data) fits easily.
      expect(grid.rows, equals(15));
      expect(grid.cols, equals(15));
    });
  });

  // ==========================================================================
  // 4. Compact symbol selection
  // ==========================================================================

  group('Compact symbol selection', () {
    // Aztec compact symbols span layers 1–4:
    //   Layer 1 → 15×15   (9 total codewords,  7 data at 23% ECC)
    //   Layer 2 → 19×19   (25 total codewords, 19 data at 23% ECC)
    //   Layer 3 → 23×23   (49 total codewords, 37 data at 23% ECC)
    //   Layer 4 → 27×27   (81 total codewords, 63 data at 23% ECC)

    test('single ASCII character → compact-1 (15×15)', () {
      final grid = encode('A');
      expect(grid.rows, equals(15));
    });

    test('short string → compact-1 (15×15)', () {
      // 'Hi' = 2 bytes → 5+5+16 = 26 bits → 4 codewords stuffed → fits compact-1.
      final grid = encode('Hi');
      expect(grid.rows, equals(15));
    });

    test('5-byte string fits compact-1 (15×15)', () {
      // Compact-1: 9 total codewords, ceil(23%×9)=3 ECC → 6 data codewords = 48 bits.
      // Binary-Shift header = 10 bits; 5 payload bytes = 40 bits → total 50 bits → fits.
      // 6 bytes exceeds compact-1 (needs 58 bits > 48 bits available) → promote to compact-2.
      final grid = encode('12345');
      expect(grid.rows, equals(15));
    });

    test('medium string → compact-2 (19×19)', () {
      // 'IATA BP DATA' = 12 bytes → needs more than 7 data codewords.
      final grid = encode('IATA BP DATA');
      expect(grid.rows, equals(19));
    });

    test('compact-3 symbol is 23×23', () {
      // Compact-3 spans 17–31 bytes at 23% ECC.
      // At 17 bytes → compact-2 overflows (needs more than 19 data codewords available).
      final input = 'A' * 17;
      final grid = encode(input);
      expect(grid.rows, equals(23));
    });

    test('compact-4 symbol is 27×27', () {
      // Compact-4 spans 32–50 bytes at 23% ECC.
      final input = 'B' * 32;
      final grid = encode(input);
      expect(grid.rows, equals(27));
    });
  });

  // ==========================================================================
  // 5. Full symbol selection
  // ==========================================================================

  group('Full symbol selection', () {
    // Full symbols start when compact-4 is not enough.
    // Compact-4 holds 63 data bytes at 23% ECC.
    // Full-1 holds ~8 data bytes (11 cw, ~8 data at 23% ECC).
    // Full symbols only appear once the payload overflows compact-4.

    test('large payload forces a full symbol (size > 27)', () {
      // 51+ bytes overflows compact-4 (which fits up to 50 bytes at 23% ECC).
      // Full symbols have size = 15 + 4×layers, so full-1 = 19×19.
      // At 51 bytes we expect the encoder to jump into the full-symbol range.
      final input = 'X' * 51;
      final grid = encode(input);
      expect(grid.rows, greaterThan(27));
    });

    test('very large payload is accepted without throwing', () {
      // 500 ASCII bytes should fit comfortably in a mid-tier full symbol.
      final input = 'A' * 500;
      expect(() => encode(input), returnsNormally);
      final grid = encode(input);
      expect(grid.rows, greaterThan(27));
    });
  });

  // ==========================================================================
  // 6. Determinism
  // ==========================================================================

  group('Determinism', () {
    test('same input always yields identical grid', () {
      final g1 = encode('IATA BP DATA');
      final g2 = encode('IATA BP DATA');
      expect(gridToString(g1), equals(gridToString(g2)));
    });

    test('different inputs yield different grids', () {
      final g1 = encode('HELLO');
      final g2 = encode('WORLD');
      expect(gridToString(g1), isNot(equals(gridToString(g2))));
    });

    test('encode is deterministic across multiple calls', () {
      final results = List.generate(5, (_) => gridToString(encode('TEST')));
      for (final r in results) {
        expect(r, equals(results[0]));
      }
    });
  });

  // ==========================================================================
  // 7. Larger input → bigger symbol
  // ==========================================================================

  group('Larger input produces bigger symbol', () {
    test('1 byte < 10 bytes in symbol size', () {
      final g1 = encode('A');          // 1 byte
      final g2 = encode('A' * 10);    // 10 bytes
      expect(g2.rows, greaterThanOrEqualTo(g1.rows));
    });

    test('10 bytes < 100 bytes in symbol size', () {
      final g1 = encode('A' * 10);
      final g2 = encode('A' * 100);
      expect(g2.rows, greaterThan(g1.rows));
    });

    test('100 bytes < 500 bytes in symbol size', () {
      final g1 = encode('A' * 100);
      final g2 = encode('A' * 500);
      expect(g2.rows, greaterThan(g1.rows));
    });
  });

  // ==========================================================================
  // 8. InputTooLongError for oversized payloads
  // ==========================================================================

  group('InputTooLongError', () {
    test('throws InputTooLongError for a 10 000-byte payload at 90% ECC', () {
      // At 90% ECC barely any data capacity remains.  10 000 bytes will never
      // fit in any symbol at that ECC level.
      final opts = const AztecOptions(minEccPercent: 90);
      final input = 'X' * 10000;
      expect(
        () => encode(input, options: opts),
        throwsA(isA<InputTooLongError>()),
      );
    });

    test('InputTooLongError is also an AztecError', () {
      const AztecOptions opts = AztecOptions(minEccPercent: 90);
      final input = 'X' * 10000;
      expect(
        () => encode(input, options: opts),
        throwsA(isA<AztecError>()),
      );
    });

    test('InputTooLongError is also an Exception', () {
      const AztecOptions opts = AztecOptions(minEccPercent: 90);
      final input = 'X' * 10000;
      expect(
        () => encode(input, options: opts),
        throwsA(isA<Exception>()),
      );
    });
  });

  // ==========================================================================
  // 9. minEccPercent option
  // ==========================================================================

  group('minEccPercent option', () {
    test('higher ECC may require a larger symbol for the same input', () {
      // At 23% ECC a medium string might fit in compact-2 (19×19).
      // At 50% ECC the same string needs more ECC codewords, so a larger symbol.
      const input = 'HELLO AZTEC CODE 2024';
      final g23 = encode(input, options: const AztecOptions(minEccPercent: 23));
      final g50 = encode(input, options: const AztecOptions(minEccPercent: 50));
      // The 50%-ECC symbol must be at least as large (could be same if slack).
      expect(g50.rows, greaterThanOrEqualTo(g23.rows));
    });

    test('default options (23%) and explicit 23% produce identical output', () {
      const input = 'SAME INPUT';
      final g1 = encode(input);
      final g2 = encode(input, options: const AztecOptions(minEccPercent: 23));
      expect(gridToString(g1), equals(gridToString(g2)));
    });

    test('low ECC (10%) may produce a smaller symbol than 23%', () {
      // A payload that sits right on the compact-1/compact-2 boundary at 23%
      // might still fit compact-1 at a lower ECC level.
      // We pick 'HELLO WORLD' (11 bytes): at 23% ECC it may force compact-2;
      // at 10% ECC it may squeeze into compact-1.  Either way the 10%-ECC
      // symbol should be no larger.
      const input = 'HELLO WORLD';
      final g10 = encode(input, options: const AztecOptions(minEccPercent: 10));
      final g23 = encode(input, options: const AztecOptions(minEccPercent: 23));
      expect(g10.rows, lessThanOrEqualTo(g23.rows));
    });
  });

  // ==========================================================================
  // 10. Layout / PaintScene wrapper
  // ==========================================================================

  group('Layout wrapper', () {
    test('encodeAndLayout returns a PaintScene', () {
      final scene = encodeAndLayout('HELLO');
      expect(scene, isNotNull);
    });

    test('encodeAndLayout scene has positive dimensions', () {
      final scene = encodeAndLayout('HELLO');
      expect(scene.width, greaterThan(0));
      expect(scene.height, greaterThan(0));
    });

    test('layoutGrid accepts a grid and returns a PaintScene', () {
      final grid = encode('HELLO');
      final scene = layoutGrid(grid);
      expect(scene, isNotNull);
      expect(scene.width, greaterThan(0));
    });

    test('explain returns an AnnotatedModuleGrid', () {
      final annotated = explain('HELLO');
      expect(annotated, isA<AnnotatedModuleGrid>());
      expect(annotated.rows, greaterThan(0));
      expect(annotated.cols, equals(annotated.rows));
    });

    test('explain grid matches encode grid', () {
      const input = 'HELLO';
      final grid = encode(input);
      final annotated = explain(input);
      expect(gridToString(annotated), equals(gridToString(grid)));
    });
  });

  // ==========================================================================
  // 11. Structural invariants
  // ==========================================================================

  group('Structural invariants', () {
    // The Aztec bullseye is a set of concentric square rings at the centre.
    // For a compact-1 symbol (15×15), the centre cell (7,7) must always be DARK
    // because the innermost 3×3 core (d≤1) is all-dark.

    test('compact-1 centre module (7,7) is dark', () {
      final grid = encode('A'); // compact-1 = 15×15
      expect(grid.rows, equals(15));
      expect(grid.modules[7][7], isTrue,
          reason: 'Centre of bullseye must always be dark');
    });

    test('compact-1 has non-trivial dark and light modules', () {
      final grid = encode('A');
      var darkCount = 0;
      var lightCount = 0;
      for (final row in grid.modules) {
        for (final m in row) {
          if (m) {
            darkCount++;
          } else {
            lightCount++;
          }
        }
      }
      // A 15×15 symbol has 225 modules. The bullseye alone sets ~60 modules.
      expect(darkCount, greaterThan(0));
      expect(lightCount, greaterThan(0));
    });

    test('every row in modules has exactly cols elements', () {
      final grid = encode('IATA BP DATA');
      for (var r = 0; r < grid.rows; r++) {
        expect(
          grid.modules[r].length,
          equals(grid.cols),
          reason: 'Row $r should have ${grid.cols} columns',
        );
      }
    });

    test('encode empty string produces a valid symbol', () {
      // The binary-shift header (5+5 = 10 bits) still fits in compact-1.
      expect(() => encode(''), returnsNormally);
      final grid = encode('');
      expect(grid.rows, greaterThan(0));
      expect(grid.rows, equals(grid.cols));
    });

    test('encode single byte 0x00 produces a valid symbol', () {
      // Edge case: the all-zero codeword avoidance path (0xFF substitution).
      final grid = encode(String.fromCharCode(0));
      expect(grid.rows, greaterThan(0));
    });

    test('encode non-ASCII UTF-8 data produces a valid symbol', () {
      // 'Héllo' encodes as UTF-16 code units; the encoder sees raw code units.
      expect(() => encode('Héllo'), returnsNormally);
      final grid = encode('Héllo');
      expect(grid.rows, greaterThan(0));
    });
  });
}
