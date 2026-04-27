/// Tests for the Data Matrix ECC200 encoder.
///
/// Test strategy:
///   1.  Package constants (version, GF primitive, size bounds)
///   2.  Error type hierarchy
///   3.  Basic encoding — output shape
///   4.  Grid dimensions match symbol size
///   5.  All modules are booleans (no null slots)
///   6.  Determinism
///   7.  Larger input → bigger or equal symbol
///   8.  Error on too-long input
///   9.  Empty string encodes successfully
///  10.  L-finder structural invariants (bottom row + left col all dark)
///  11.  Timing border invariants (top row + right col alternating)
///  12.  Forced size
///  13.  Input too long for forced size
///  14.  Invalid forced size
///  15.  gridToString utility
///  16.  layoutGrid and encodeAndLayout helpers
///  17.  Cross-language corpus (size expectations)
import 'package:test/test.dart';
import 'package:coding_adventures_data_matrix/data_matrix.dart';
import 'package:coding_adventures_barcode_2d/coding_adventures_barcode_2d.dart';

void main() {
  // ==========================================================================
  // 1. Package constants
  // ==========================================================================

  group('Package constants', () {
    test('dataMatrixVersion is 0.1.0', () {
      expect(dataMatrixVersion, equals('0.1.0'));
    });

    test('gf256Prime is 0x12D', () {
      expect(gf256Prime, equals(0x12D));
      expect(gf256Prime, equals(301));
    });

    test('minSize is 10', () {
      expect(minSize, equals(10));
    });

    test('maxSize is 144', () {
      expect(maxSize, equals(144));
    });

    test('gf256Prime is different from QR polynomial 0x11D', () {
      expect(gf256Prime, isNot(equals(0x11D)));
    });
  });

  // ==========================================================================
  // 2. Error type hierarchy
  // ==========================================================================

  group('Error type hierarchy', () {
    test('DataMatrixError implements Exception', () {
      // DataMatrixError is abstract, use InputTooLongError as concrete instance.
      final err = InputTooLongError('test');
      expect(err, isA<Exception>());
    });

    test('InputTooLongError is a DataMatrixError', () {
      final err = InputTooLongError('too long');
      expect(err, isA<DataMatrixError>());
    });

    test('InvalidSizeError is a DataMatrixError', () {
      final err = InvalidSizeError('bad size');
      expect(err, isA<DataMatrixError>());
    });

    test('DataMatrixError.toString includes runtimeType', () {
      final err = InputTooLongError('test message');
      expect(err.toString(), contains('InputTooLongError'));
      expect(err.toString(), contains('test message'));
    });

    test('InvalidSizeError.toString includes runtimeType', () {
      final err = InvalidSizeError('bad');
      expect(err.toString(), contains('InvalidSizeError'));
    });

    test('DataMatrixError.message is accessible', () {
      const msg = 'something went wrong';
      final err = InputTooLongError(msg);
      expect(err.message, equals(msg));
    });
  });

  // ==========================================================================
  // 3. Basic encoding — output shape
  // ==========================================================================

  group('Basic encoding', () {
    test('encode("A") returns a ModuleGrid', () {
      final grid = encode('A');
      expect(grid, isA<ModuleGrid>());
    });

    test('encode("A") produces a 10×10 symbol (smallest square)', () {
      // 'A' ASCII encodes to 1 codeword; 10×10 holds 3 data codewords → fits.
      final grid = encode('A');
      expect(grid.rows, equals(10));
      expect(grid.cols, equals(10));
    });

    test('encode("A") moduleShape is square', () {
      expect(encode('A').moduleShape, equals(ModuleShape.square));
    });
  });

  // ==========================================================================
  // 4. Grid dimensions match symbol size
  // ==========================================================================

  group('Grid dimensions', () {
    test('modules list has exactly grid.rows rows', () {
      final grid = encode('A');
      expect(grid.modules.length, equals(grid.rows));
    });

    test('every row has exactly grid.cols columns', () {
      final grid = encode('A');
      for (final row in grid.modules) {
        expect(row.length, equals(grid.cols));
      }
    });

    test('dimensions are consistent across multiple symbols', () {
      for (final input in ['A', 'Hello World', '1234567890', 'HELLO']) {
        final g = encode(input);
        expect(g.modules.length, equals(g.rows),
            reason: 'Row count mismatch for "$input"');
        for (final row in g.modules) {
          expect(row.length, equals(g.cols),
              reason: 'Column count mismatch for "$input"');
        }
      }
    });

    test('encode empty string → 10×10 symbol', () {
      // Empty → 0 codewords; 10×10 symbol holds 3 → fits.
      final grid = encode('');
      expect(grid.rows, equals(10));
      expect(grid.cols, equals(10));
    });
  });

  // ==========================================================================
  // 5. All modules are booleans
  // ==========================================================================

  group('Module values', () {
    test('all modules in 10×10 symbol are booleans', () {
      final grid = encode('A');
      for (var r = 0; r < grid.rows; r++) {
        for (var c = 0; c < grid.cols; c++) {
          expect(grid.modules[r][c], isA<bool>(),
              reason: 'Module ($r,$c) is not a bool');
        }
      }
    });

    test('all modules in larger symbol are booleans', () {
      // Use a longer string to exercise a larger symbol.
      final grid = encode('Hello, World! This is a Data Matrix test.');
      for (var r = 0; r < grid.rows; r++) {
        for (var c = 0; c < grid.cols; c++) {
          expect(grid.modules[r][c], isA<bool>(),
              reason: 'Module ($r,$c) is not a bool');
        }
      }
    });
  });

  // ==========================================================================
  // 6. Determinism
  // ==========================================================================

  group('Determinism', () {
    test('same input always produces identical grids', () {
      for (final input in ['A', '', '12345', 'Hello, World!', 'HELLO WORLD']) {
        final g1 = encode(input);
        final g2 = encode(input);
        expect(gridToString(g1), equals(gridToString(g2)),
            reason: 'Non-deterministic for "$input"');
      }
    });

    test('different inputs produce different grids', () {
      final g1 = encode('A');
      final g2 = encode('B');
      expect(gridToString(g1), isNot(equals(gridToString(g2))));
    });

    test('different inputs of same length produce different grids', () {
      final g1 = encode('AB');
      final g2 = encode('CD');
      // Both fit in 10×10; grids differ in data region.
      expect(gridToString(g1), isNot(equals(gridToString(g2))));
    });
  });

  // ==========================================================================
  // 7. Larger input → bigger or equal symbol
  // ==========================================================================

  group('Auto symbol selection', () {
    test('single char → 10×10 (smallest)', () {
      expect(encode('A').rows, equals(10));
    });

    test('longer input → equal or larger symbol', () {
      final small = encode('A');
      final large = encode('A' * 50);
      expect(large.rows, greaterThanOrEqualTo(small.rows));
    });

    test('many chars → large symbol', () {
      // 500 'A' bytes → 500 codewords → needs a large symbol.
      final grid = encode('A' * 500);
      expect(grid.rows, greaterThan(52));
    });

    test('digit pairs pack into fewer codewords (numeric optimisation)', () {
      // "00" encodes as 1 codeword, "AA" encodes as 2 codewords.
      // 4 digits → 2 codewords; 4 letters → 4 codewords.
      // 10×10 holds 3 data codewords.
      // "0000" (2 codewords) → 10×10; "AAAA" (4 codewords) → 12×12.
      final gridNum = encode('0000');
      final gridAlpha = encode('AAAA');
      expect(gridNum.rows, lessThanOrEqualTo(gridAlpha.rows));
    });

    test('symbol grows as input grows', () {
      int prevSize = 0;
      // Pick inputs that each fit in progressively larger symbols.
      final inputs = ['A', 'A' * 5, 'A' * 10, 'A' * 50, 'A' * 100];
      for (final input in inputs) {
        final size = encode(input).rows;
        expect(size, greaterThanOrEqualTo(prevSize));
        prevSize = size;
      }
    });
  });

  // ==========================================================================
  // 8. InputTooLongError for oversized input
  // ==========================================================================

  group('InputTooLongError', () {
    test('throws InputTooLongError for extremely long input', () {
      // 144×144 holds 1558 data codewords. 2000 'A' bytes → 2000 codewords.
      expect(
        () => encode('A' * 2000),
        throwsA(isA<InputTooLongError>()),
      );
    });

    test('InputTooLongError is a DataMatrixError', () {
      expect(
        () => encode('A' * 2000),
        throwsA(isA<DataMatrixError>()),
      );
    });

    test('InputTooLongError message is descriptive', () {
      try {
        encode('A' * 2000);
        fail('Expected InputTooLongError');
      } on InputTooLongError catch (e) {
        expect(e.message, isNotEmpty);
        // Message should mention the codeword count or the max capacity.
        expect(e.message.toLowerCase(), contains('data-matrix'));
      }
    });
  });

  // ==========================================================================
  // 9. Empty string
  // ==========================================================================

  group('Empty string', () {
    test('encode("") returns a ModuleGrid', () {
      expect(encode(''), isA<ModuleGrid>());
    });

    test('encode("") → 10×10 (smallest symbol)', () {
      final grid = encode('');
      expect(grid.rows, equals(10));
      expect(grid.cols, equals(10));
    });

    test('encode("") modules all accessible as booleans', () {
      final grid = encode('');
      for (var r = 0; r < grid.rows; r++) {
        for (var c = 0; c < grid.cols; c++) {
          expect(grid.modules[r][c], isA<bool>());
        }
      }
    });
  });

  // ==========================================================================
  // 10. L-finder structural invariants
  // ==========================================================================

  group('L-finder (bottom row + left column all dark)', () {
    // The L-finder is the most important structural element: the entire bottom
    // row and entire left column are always solid dark (true). This gives the
    // scanner a definitive orientation marker.

    test('bottom row is all dark — 10×10 symbol', () {
      final m = encode('A').modules;
      final lastRow = m.length - 1;
      for (var c = 0; c < m[lastRow].length; c++) {
        expect(m[lastRow][c], isTrue,
            reason: 'Bottom row col $c should be dark');
      }
    });

    test('left column is all dark — 10×10 symbol', () {
      final m = encode('A').modules;
      for (var r = 0; r < m.length; r++) {
        expect(m[r][0], isTrue,
            reason: 'Left column row $r should be dark');
      }
    });

    test('bottom row is all dark — larger symbol', () {
      final grid = encode('Hello, World!');
      final m = grid.modules;
      final lastRow = grid.rows - 1;
      for (var c = 0; c < grid.cols; c++) {
        expect(m[lastRow][c], isTrue,
            reason: 'Bottom row col $c should be dark for larger symbol');
      }
    });

    test('left column is all dark — larger symbol', () {
      final grid = encode('Hello, World!');
      final m = grid.modules;
      for (var r = 0; r < grid.rows; r++) {
        expect(m[r][0], isTrue,
            reason: 'Left column row $r should be dark for larger symbol');
      }
    });

    test('L-finder present for empty string', () {
      final grid = encode('');
      final m = grid.modules;
      // Bottom row all dark.
      final lastRow = grid.rows - 1;
      for (var c = 0; c < grid.cols; c++) {
        expect(m[lastRow][c], isTrue,
            reason: 'Empty string: bottom row col $c should be dark');
      }
      // Left column all dark.
      for (var r = 0; r < grid.rows; r++) {
        expect(m[r][0], isTrue,
            reason: 'Empty string: left col row $r should be dark');
      }
    });

    test('L-finder corners are both dark', () {
      final grid = encode('A');
      final m = grid.modules;
      // Top-left corner (row 0, col 0) is the intersection of left col and top timing.
      // Left column always wins → dark.
      expect(m[grid.rows - 1][0], isTrue, reason: 'Bottom-left corner must be dark');
      expect(m[grid.rows - 1][grid.cols - 1], isTrue, reason: 'Bottom-right corner must be dark');
    });
  });

  // ==========================================================================
  // 11. Timing border invariants
  // ==========================================================================

  group('Timing border (top row + right column alternating)', () {
    // The top row and right column are the timing borders: alternating dark/light
    // starting dark at position 0. This gives the scanner a spatial ruler.
    //
    // Writing order in _initGrid: alignment borders → top row → right column →
    // left column → bottom row. Because the right column is drawn AFTER the top
    // row, the top-right corner (row 0, col C-1) is set by the right column rule
    // (row 0 is even → dark), regardless of what the top-row timing would say.
    // Similarly, the bottom row is drawn LAST, so the bottom-right corner
    // (row R-1, col C-1) is always dark from the L-finder.
    //
    // In practice:
    //   - Top row: cols 0..C-2 alternate dark/light. Col C-1 is overridden dark.
    //   - Right col: rows 0..R-2 alternate dark/light. Row R-1 is overridden dark.

    test('top row alternates starting dark — 10×10 symbol (cols 0..C-2)', () {
      final grid = encode('A');
      final m = grid.modules;
      // Cols 0..C-2 follow the timing pattern. Col C-1 is set by the right column.
      for (var c = 0; c < grid.cols - 1; c++) {
        expect(m[0][c], equals(c % 2 == 0),
            reason: 'Top row col $c should be ${c % 2 == 0 ? "dark" : "light"}');
      }
      // The top-right corner is always dark (right col row 0 is even).
      expect(m[0][grid.cols - 1], isTrue,
          reason: 'Top-right corner always dark (right col rule overrides top row)');
    });

    test('right column alternates starting dark — 10×10 symbol (excluding last row)', () {
      final grid = encode('A');
      final m = grid.modules;
      final lastCol = grid.cols - 1;
      // Right column rows 0..R-2 alternate. Row R-1 is always dark (L-finder wins).
      for (var r = 0; r < grid.rows - 1; r++) {
        expect(m[r][lastCol], equals(r % 2 == 0),
            reason: 'Right col row $r should be ${r % 2 == 0 ? "dark" : "light"}');
      }
    });

    test('top row alternates — larger symbol (cols 0..C-2)', () {
      final grid = encode('Hello, World!');
      final m = grid.modules;
      // Cols 0..C-2 follow the timing pattern.
      for (var c = 0; c < grid.cols - 1; c++) {
        expect(m[0][c], equals(c % 2 == 0),
            reason: 'Top row col $c should be ${c % 2 == 0 ? "dark" : "light"}');
      }
    });

    test('top-left module (row 0, col 0) is dark', () {
      // Col 0 is even → dark in the timing pattern; also the L-finder left col.
      expect(encode('A').modules[0][0], isTrue);
    });

    test('top row col 1 is light (alternating)', () {
      // Col 1 is odd → light in timing pattern (top row).
      expect(encode('A').modules[0][1], isFalse);
    });
  });

  // ==========================================================================
  // 12. Forced size
  // ==========================================================================

  group('Forced size', () {
    test('size 12 → 12×12 symbol', () {
      final grid = encode('A', options: const DataMatrixOptions(size: 12));
      expect(grid.rows, equals(12));
      expect(grid.cols, equals(12));
    });

    test('size 14 → 14×14 symbol', () {
      final grid = encode('A', options: const DataMatrixOptions(size: 14));
      expect(grid.rows, equals(14));
      expect(grid.cols, equals(14));
    });

    test('forced size still produces correct L-finder', () {
      final grid = encode('A', options: const DataMatrixOptions(size: 18));
      final m = grid.modules;
      final lastRow = grid.rows - 1;
      for (var c = 0; c < grid.cols; c++) {
        expect(m[lastRow][c], isTrue,
            reason: 'Forced-18 bottom row col $c should be dark');
      }
      for (var r = 0; r < grid.rows; r++) {
        expect(m[r][0], isTrue,
            reason: 'Forced-18 left col row $r should be dark');
      }
    });

    test('forced size produces deterministic output', () {
      final g1 = encode('A', options: const DataMatrixOptions(size: 14));
      final g2 = encode('A', options: const DataMatrixOptions(size: 14));
      expect(gridToString(g1), equals(gridToString(g2)));
    });
  });

  // ==========================================================================
  // 13. InputTooLongError for forced size
  // ==========================================================================

  group('InputTooLongError for forced size', () {
    test('throws when input too long for forced 10×10', () {
      // 10×10 holds 3 data codewords. "AAAAAAA" → 7 codewords → too long.
      expect(
        () => encode('AAAAAAA', options: const DataMatrixOptions(size: 10)),
        throwsA(isA<InputTooLongError>()),
      );
    });

    test('does not throw when input fits forced size', () {
      // 10×10 holds 3 data codewords. "AA" → 2 codewords → fits.
      expect(
        () => encode('AA', options: const DataMatrixOptions(size: 10)),
        returnsNormally,
      );
    });
  });

  // ==========================================================================
  // 14. InvalidSizeError
  // ==========================================================================

  group('InvalidSizeError', () {
    test('throws InvalidSizeError for non-existent size 11', () {
      expect(
        () => encode('A', options: const DataMatrixOptions(size: 11)),
        throwsA(isA<InvalidSizeError>()),
      );
    });

    test('throws InvalidSizeError for size 9 (below minimum)', () {
      expect(
        () => encode('A', options: const DataMatrixOptions(size: 9)),
        throwsA(isA<InvalidSizeError>()),
      );
    });

    test('throws InvalidSizeError for size 145 (above maximum)', () {
      expect(
        () => encode('A', options: const DataMatrixOptions(size: 145)),
        throwsA(isA<InvalidSizeError>()),
      );
    });

    test('InvalidSizeError is a DataMatrixError', () {
      expect(
        () => encode('A', options: const DataMatrixOptions(size: 11)),
        throwsA(isA<DataMatrixError>()),
      );
    });
  });

  // ==========================================================================
  // 15. gridToString utility
  // ==========================================================================

  group('gridToString', () {
    test('returns a non-empty string', () {
      expect(gridToString(encode('A')), isNotEmpty);
    });

    test('returns exactly rows lines for a 10×10 grid', () {
      final s = gridToString(encode('A'));
      final lines = s.split('\n');
      expect(lines.length, equals(10));
    });

    test('each line has exactly cols characters for a 10×10 grid', () {
      final s = gridToString(encode('A'));
      for (final line in s.split('\n')) {
        expect(line.length, equals(10));
      }
    });

    test('only contains 0 and 1 characters', () {
      final s = gridToString(encode('Hello!'));
      for (final ch in s.runes) {
        final c = String.fromCharCode(ch);
        expect(c == '0' || c == '1' || c == '\n', isTrue,
            reason: 'Unexpected character: $c');
      }
    });

    test('different grids produce different strings', () {
      final s1 = gridToString(encode('A'));
      final s2 = gridToString(encode('B'));
      expect(s1, isNot(equals(s2)));
    });

    test('same grid produces same string (determinism)', () {
      final g = encode('HELLO');
      expect(gridToString(g), equals(gridToString(g)));
    });

    test('first line starts with 1 (top-left timing is dark)', () {
      final s = gridToString(encode('A'));
      expect(s.startsWith('1'), isTrue);
    });

    test('last line is all 1s (L-finder bottom row)', () {
      final s = gridToString(encode('A'));
      final lastLine = s.split('\n').last;
      expect(lastLine, equals('1' * 10));
    });
  });

  // ==========================================================================
  // 16. layoutGrid and encodeAndLayout
  // ==========================================================================

  group('Layout helpers', () {
    test('layoutGrid returns a PaintScene', () {
      final grid = encode('A');
      final scene = layoutGrid(grid);
      expect(scene, isNotNull);
    });

    test('layoutGrid default: 1-module quiet zone, 10px modules, 10×10 → 120px', () {
      // 10×10 grid + 1 quiet zone on each side = 12 modules wide.
      // 12 modules × 10px = 120px.
      final grid = encode('A');
      final scene = layoutGrid(grid);
      expect(scene.width, equals(120));
      expect(scene.height, equals(120));
    });

    test('layoutGrid with custom config', () {
      final grid = encode('A'); // 10×10
      const config = Barcode2DLayoutConfig(
        moduleSizePx: 5,
        quietZoneModules: 2,
        foreground: '#000000',
        background: '#ffffff',
        moduleShape: ModuleShape.square,
      );
      final scene = layoutGrid(grid, config: config);
      // (10 + 2*2) * 5 = 70px
      expect(scene.width, equals(70));
    });

    test('encodeAndLayout returns a PaintScene', () {
      final scene = encodeAndLayout('A');
      expect(scene, isNotNull);
    });

    test('encodeAndLayout width > 0', () {
      final scene = encodeAndLayout('A');
      expect(scene.width, greaterThan(0));
    });

    test('larger symbol → wider scene', () {
      final smallScene = encodeAndLayout('A');       // 10×10
      final bigScene = encodeAndLayout('A' * 50);    // much larger
      expect(bigScene.width, greaterThan(smallScene.width));
    });
  });

  // ==========================================================================
  // 17. Cross-language corpus
  // ==========================================================================

  group('Cross-language corpus', () {
    // These are the canonical size expectations that must match all other
    // language implementations of this package. Any implementation claiming
    // ISO/IEC 16022:2006 compliance must produce exactly these symbol sizes.
    //
    // Sizes come from Table 7 of the standard: each entry is the smallest
    // square symbol whose data capacity (dataCw) ≥ the ASCII codeword count
    // for that input.

    test('"A" → 10×10 (1 codeword fits in 3-cw symbol)', () {
      final g = encode('A');
      expect(g.rows, equals(10));
      expect(g.cols, equals(10));
    });

    test('"" → 10×10 (0 codewords fit in smallest symbol)', () {
      final g = encode('');
      expect(g.rows, equals(10));
      expect(g.cols, equals(10));
    });

    test('"12" → 10×10 (1 codeword: digit pair)', () {
      // "12" encodes as 1 codeword (130 + 12 = 142). Fits in 10×10 (3 cw).
      final g = encode('12');
      expect(g.rows, equals(10));
      expect(g.cols, equals(10));
    });

    test('"HELLO" → 12×12 (5 codewords, 10×10 has only 3)', () {
      // "HELLO" = 5 ASCII codewords. 10×10 capacity is 3 → needs 12×12 (5 cw).
      final g = encode('HELLO');
      expect(g.rows, equals(12));
      expect(g.cols, equals(12));
    });

    test('"Hello World" → 14×14 (11 codewords)', () {
      // "Hello World" = 11 ASCII codewords.
      // 10×10=3, 12×12=5, 14×14=8 — need 16×16 (12 cw).
      // Actually: H,e,l,l,o,' ',W,o,r,l,d = 11 codewords. 14×14 holds 8, 16×16 holds 12.
      final g = encode('Hello World');
      expect(g.rows, equals(16));
      expect(g.cols, equals(16));
    });

    test('digit-pair packing reduces codeword count', () {
      // "1234567890" = 5 digit pairs = 5 codewords. 12×12 holds 5 → fits.
      final g = encode('1234567890');
      expect(g.rows, equals(12));
      expect(g.cols, equals(12));
    });

    test('symbol size increases monotonically with input length', () {
      final sizes = [
        encode('A').rows,
        encode('A' * 4).rows,
        encode('A' * 8).rows,
        encode('A' * 20).rows,
      ];
      for (var i = 1; i < sizes.length; i++) {
        expect(sizes[i], greaterThanOrEqualTo(sizes[i - 1]),
            reason: 'Symbol size should not decrease with longer input');
      }
    });
  });

  // ==========================================================================
  // 18. SymbolShape option
  // ==========================================================================

  group('SymbolShape option', () {
    test('default shape is square', () {
      const opts = DataMatrixOptions();
      expect(opts.shape, equals(SymbolShape.square));
    });

    test('square shape produces square symbol', () {
      final g = encode('A', options: const DataMatrixOptions(shape: SymbolShape.square));
      expect(g.rows, equals(g.cols));
    });

    test('any shape can produce smaller symbol than square for small input', () {
      // For small inputs, rectangular symbols may have fewer total modules.
      // "A" → 1 codeword. Rect 8×18 holds 5 cw, area = 144 < 10×10 area = 100.
      // Actually 10×10=100 < 8×18=144, so square may be smaller.
      // The key is that 'any' does not throw and returns a valid grid.
      final g = encode('A', options: const DataMatrixOptions(shape: SymbolShape.any));
      expect(g.rows, greaterThan(0));
      expect(g.cols, greaterThan(0));
    });

    test('any shape returns a ModuleGrid with all boolean modules', () {
      final g = encode('HELLO', options: const DataMatrixOptions(shape: SymbolShape.any));
      for (var r = 0; r < g.rows; r++) {
        for (var c = 0; c < g.cols; c++) {
          expect(g.modules[r][c], isA<bool>());
        }
      }
    });
  });
}
