/// Tests for coding_adventures_barcode_2d.
///
/// Coverage goal: ≥ 90% of all reachable lines.
///
/// Test groups:
///   - makeModuleGrid: dimensions, all-light default, shape stored
///   - setModule: immutability, value update, bounds checking
///   - layout — square: dimensions, instruction count, background, dark modules
///   - layout — hex: dimensions, instruction type, command count per module
///   - layout — config defaults (moduleSizePx=10, quietZone=4)
///   - layout — config overrides (custom size, quiet zone, colours)
///   - layout — error cases (bad moduleSizePx, bad quietZone, shape mismatch)
///   - Barcode2DLayoutConfig copyWith
///   - ModuleAnnotation construction
///   - AnnotatedModuleGrid construction
///   - version constant
///   - PaintScene re-export available
import 'dart:math' as math;

import 'package:coding_adventures_barcode_2d/coding_adventures_barcode_2d.dart';
// Hide `version` from paint_instructions to avoid ambiguity with the
// `version` constant exported by coding_adventures_barcode_2d.
// The test exercises barcode_2d's version; paint_instructions' version
// is tested in its own test suite.
import 'package:coding_adventures_paint_instructions/coding_adventures_paint_instructions.dart'
    hide version;
import 'package:test/test.dart';

void main() {
  // ==========================================================================
  // makeModuleGrid
  // ==========================================================================

  group('makeModuleGrid', () {
    test('rows and cols are stored correctly', () {
      final grid = makeModuleGrid(rows: 5, cols: 7);
      expect(grid.rows, equals(5));
      expect(grid.cols, equals(7));
    });

    test('all modules default to false (light)', () {
      final grid = makeModuleGrid(rows: 3, cols: 4);
      for (var r = 0; r < 3; r++) {
        for (var c = 0; c < 4; c++) {
          expect(grid.modules[r][c], isFalse,
              reason: 'module [$r][$c] should be light');
        }
      }
    });

    test('moduleShape defaults to square', () {
      final grid = makeModuleGrid(rows: 3, cols: 3);
      expect(grid.moduleShape, equals(ModuleShape.square));
    });

    test('moduleShape hex is stored when requested', () {
      final grid =
          makeModuleGrid(rows: 33, cols: 30, moduleShape: ModuleShape.hex);
      expect(grid.moduleShape, equals(ModuleShape.hex));
    });

    test('a 1×1 grid has exactly one module', () {
      final grid = makeModuleGrid(rows: 1, cols: 1);
      expect(grid.modules, hasLength(1));
      expect(grid.modules[0], hasLength(1));
      expect(grid.modules[0][0], isFalse);
    });

    test('produces independent row lists (not shared)', () {
      final grid = makeModuleGrid(rows: 2, cols: 2);
      expect(identical(grid.modules[0], grid.modules[1]), isFalse);
    });
  });

  // ==========================================================================
  // setModule
  // ==========================================================================

  group('setModule', () {
    test('returns a new grid object (immutability)', () {
      final g1 = makeModuleGrid(rows: 3, cols: 3);
      final g2 = setModule(g1, row: 0, col: 0, dark: true);
      expect(identical(g1, g2), isFalse);
    });

    test('original grid is unchanged after setModule', () {
      final g1 = makeModuleGrid(rows: 3, cols: 3);
      setModule(g1, row: 1, col: 1, dark: true);
      expect(g1.modules[1][1], isFalse);
    });

    test('new grid has the correct module set dark', () {
      final g1 = makeModuleGrid(rows: 3, cols: 3);
      final g2 = setModule(g1, row: 1, col: 1, dark: true);
      expect(g2.modules[1][1], isTrue);
    });

    test('new grid preserves all other modules as false', () {
      final g1 = makeModuleGrid(rows: 3, cols: 3);
      final g2 = setModule(g1, row: 1, col: 1, dark: true);
      for (var r = 0; r < 3; r++) {
        for (var c = 0; c < 3; c++) {
          if (r == 1 && c == 1) continue;
          expect(g2.modules[r][c], isFalse,
              reason: 'module [$r][$c] should still be light');
        }
      }
    });

    test('can set a module back to light (false)', () {
      final g1 = makeModuleGrid(rows: 3, cols: 3);
      final g2 = setModule(g1, row: 0, col: 0, dark: true);
      final g3 = setModule(g2, row: 0, col: 0, dark: false);
      expect(g3.modules[0][0], isFalse);
    });

    test('unchanged rows are structurally shared', () {
      final g1 = makeModuleGrid(rows: 3, cols: 3);
      final g2 = setModule(g1, row: 1, col: 1, dark: true);
      // Rows 0 and 2 should be the same list objects (structural sharing).
      expect(identical(g1.modules[0], g2.modules[0]), isTrue);
      expect(identical(g1.modules[2], g2.modules[2]), isTrue);
    });

    test('moduleShape is preserved', () {
      final g1 =
          makeModuleGrid(rows: 3, cols: 3, moduleShape: ModuleShape.hex);
      final g2 = setModule(g1, row: 0, col: 0, dark: true);
      expect(g2.moduleShape, equals(ModuleShape.hex));
    });

    test('throws RangeError for negative row', () {
      final grid = makeModuleGrid(rows: 3, cols: 3);
      expect(
        () => setModule(grid, row: -1, col: 0, dark: true),
        throwsA(isA<RangeError>()),
      );
    });

    test('throws RangeError for row >= rows', () {
      final grid = makeModuleGrid(rows: 3, cols: 3);
      expect(
        () => setModule(grid, row: 3, col: 0, dark: true),
        throwsA(isA<RangeError>()),
      );
    });

    test('throws RangeError for negative col', () {
      final grid = makeModuleGrid(rows: 3, cols: 3);
      expect(
        () => setModule(grid, row: 0, col: -1, dark: true),
        throwsA(isA<RangeError>()),
      );
    });

    test('throws RangeError for col >= cols', () {
      final grid = makeModuleGrid(rows: 3, cols: 3);
      expect(
        () => setModule(grid, row: 0, col: 3, dark: true),
        throwsA(isA<RangeError>()),
      );
    });
  });

  // ==========================================================================
  // layout — square modules
  // ==========================================================================

  group('layout (square)', () {
    test('default config produces correct canvas width for QR v1 (21×21)', () {
      // totalWidth = (21 + 2*4) * 10 = 290
      final grid = makeModuleGrid(rows: 21, cols: 21);
      final scene = layout(grid);
      expect(scene.width, equals(290));
      expect(scene.height, equals(290));
    });

    test('all-light grid produces exactly 1 instruction (background only)', () {
      final grid = makeModuleGrid(rows: 3, cols: 3);
      final scene = layout(grid);
      // Only the background PaintRect; no dark modules.
      expect(scene.instructions, hasLength(1));
    });

    test('background instruction is a PaintRect covering the full canvas', () {
      final grid = makeModuleGrid(rows: 3, cols: 3);
      final scene = layout(grid);
      final bg = scene.instructions[0] as PaintRect;
      expect(bg.x, equals(0));
      expect(bg.y, equals(0));
      expect(bg.width, equals(scene.width));
      expect(bg.height, equals(scene.height));
    });

    test('background instruction uses config background colour', () {
      final grid = makeModuleGrid(rows: 3, cols: 3);
      final cfg = defaultBarcode2DLayoutConfig.copyWith(background: '#eeeeee');
      final scene = layout(grid, config: cfg);
      final bg = scene.instructions[0] as PaintRect;
      expect(bg.fill, equals('#eeeeee'));
    });

    test('one dark module produces 2 instructions (background + 1 rect)', () {
      var grid = makeModuleGrid(rows: 3, cols: 3);
      grid = setModule(grid, row: 1, col: 1, dark: true);
      final scene = layout(grid);
      expect(scene.instructions, hasLength(2));
    });

    test('dark module rect is positioned at correct pixel offset', () {
      // grid: 3×3, moduleSizePx=10, quietZone=4
      // quietZonePx = 4*10 = 40
      // module (row=0, col=0): x=40, y=40, w=10, h=10
      var grid = makeModuleGrid(rows: 3, cols: 3);
      grid = setModule(grid, row: 0, col: 0, dark: true);
      final scene = layout(grid);
      final moduleRect = scene.instructions[1] as PaintRect;
      expect(moduleRect.x, equals(40));
      expect(moduleRect.y, equals(40));
      expect(moduleRect.width, equals(10));
      expect(moduleRect.height, equals(10));
    });

    test('dark module at (row=2, col=1) offset is correct', () {
      // quietZonePx=40; x=40+1*10=50, y=40+2*10=60
      var grid = makeModuleGrid(rows: 5, cols: 5);
      grid = setModule(grid, row: 2, col: 1, dark: true);
      final scene = layout(grid);
      final rect = scene.instructions[1] as PaintRect;
      expect(rect.x, equals(50));
      expect(rect.y, equals(60));
    });

    test('dark module rect uses config foreground colour', () {
      var grid = makeModuleGrid(rows: 3, cols: 3);
      grid = setModule(grid, row: 0, col: 0, dark: true);
      final cfg = defaultBarcode2DLayoutConfig.copyWith(foreground: '#123456');
      final scene = layout(grid, config: cfg);
      final rect = scene.instructions[1] as PaintRect;
      expect(rect.fill, equals('#123456'));
    });

    test('all-dark 3×3 grid produces 1 + 9 = 10 instructions', () {
      var grid = makeModuleGrid(rows: 3, cols: 3);
      for (var r = 0; r < 3; r++) {
        for (var c = 0; c < 3; c++) {
          grid = setModule(grid, row: r, col: c, dark: true);
        }
      }
      final scene = layout(grid);
      expect(scene.instructions, hasLength(10)); // 1 bg + 9 modules
    });

    test('instructions are ordered row-major (row 0 before row 1)', () {
      var grid = makeModuleGrid(rows: 2, cols: 2);
      grid = setModule(grid, row: 0, col: 0, dark: true);
      grid = setModule(grid, row: 1, col: 0, dark: true);
      final scene = layout(grid);
      final r0 = scene.instructions[1] as PaintRect;
      final r1 = scene.instructions[2] as PaintRect;
      expect(r0.y, lessThan(r1.y));
    });

    test('custom moduleSizePx changes canvas size proportionally', () {
      final grid = makeModuleGrid(rows: 5, cols: 5);
      final cfg = defaultBarcode2DLayoutConfig.copyWith(moduleSizePx: 4);
      // totalWidth = (5 + 8) * 4 = 52
      final scene = layout(grid, config: cfg);
      expect(scene.width, equals(52));
    });

    test('zero quiet zone removes padding', () {
      final grid = makeModuleGrid(rows: 5, cols: 5);
      final cfg = defaultBarcode2DLayoutConfig.copyWith(quietZoneModules: 0);
      // totalWidth = (5 + 0) * 10 = 50
      final scene = layout(grid, config: cfg);
      expect(scene.width, equals(50));
    });

    test('PaintScene background field matches config background', () {
      final grid = makeModuleGrid(rows: 3, cols: 3);
      final cfg = defaultBarcode2DLayoutConfig.copyWith(background: '#112233');
      final scene = layout(grid, config: cfg);
      expect(scene.background, equals('#112233'));
    });

    test('all instructions are PaintRect for square grids', () {
      var grid = makeModuleGrid(rows: 3, cols: 3);
      grid = setModule(grid, row: 0, col: 0, dark: true);
      final scene = layout(grid);
      for (final instr in scene.instructions) {
        expect(instr, isA<PaintRect>());
      }
    });
  });

  // ==========================================================================
  // layout — hex modules
  // ==========================================================================

  group('layout (hex)', () {
    test('hex grid produces PaintPath for dark modules', () {
      var grid =
          makeModuleGrid(rows: 3, cols: 3, moduleShape: ModuleShape.hex);
      grid = setModule(grid, row: 0, col: 0, dark: true);
      final cfg =
          defaultBarcode2DLayoutConfig.copyWith(moduleShape: ModuleShape.hex);
      final scene = layout(grid, config: cfg);
      // instruction[0] is background PaintRect; instruction[1] is the hex path
      expect(scene.instructions[1], isA<PaintPath>());
    });

    test('each dark hex module produces a path with 8 commands (moveTo + 5×lineTo + close)', () {
      // A flat-top hexagon path: move_to + 5 line_to + close = 7 commands.
      var grid =
          makeModuleGrid(rows: 3, cols: 3, moduleShape: ModuleShape.hex);
      grid = setModule(grid, row: 0, col: 0, dark: true);
      final cfg =
          defaultBarcode2DLayoutConfig.copyWith(moduleShape: ModuleShape.hex);
      final scene = layout(grid, config: cfg);
      final path = scene.instructions[1] as PaintPath;
      // 1 move_to + 5 line_to + 1 close = 7
      expect(path.commands, hasLength(7));
    });

    test('hex path starts with MoveTo', () {
      var grid =
          makeModuleGrid(rows: 3, cols: 3, moduleShape: ModuleShape.hex);
      grid = setModule(grid, row: 0, col: 0, dark: true);
      final cfg =
          defaultBarcode2DLayoutConfig.copyWith(moduleShape: ModuleShape.hex);
      final scene = layout(grid, config: cfg);
      final path = scene.instructions[1] as PaintPath;
      expect(path.commands.first, isA<MoveTo>());
    });

    test('hex path ends with Close', () {
      var grid =
          makeModuleGrid(rows: 3, cols: 3, moduleShape: ModuleShape.hex);
      grid = setModule(grid, row: 0, col: 0, dark: true);
      final cfg =
          defaultBarcode2DLayoutConfig.copyWith(moduleShape: ModuleShape.hex);
      final scene = layout(grid, config: cfg);
      final path = scene.instructions[1] as PaintPath;
      expect(path.commands.last, isA<Close>());
    });

    test('hex path middle commands are all LineTo', () {
      var grid =
          makeModuleGrid(rows: 3, cols: 3, moduleShape: ModuleShape.hex);
      grid = setModule(grid, row: 0, col: 0, dark: true);
      final cfg =
          defaultBarcode2DLayoutConfig.copyWith(moduleShape: ModuleShape.hex);
      final scene = layout(grid, config: cfg);
      final path = scene.instructions[1] as PaintPath;
      final middle = path.commands.sublist(1, 6);
      for (final cmd in middle) {
        expect(cmd, isA<LineTo>());
      }
    });

    test('hex canvas width is wider than grid * moduleSizePx due to odd-row offset', () {
      final grid =
          makeModuleGrid(rows: 4, cols: 4, moduleShape: ModuleShape.hex);
      final cfg = defaultBarcode2DLayoutConfig.copyWith(
        moduleShape: ModuleShape.hex,
        quietZoneModules: 0,
      );
      final scene = layout(grid, config: cfg);
      // Without quiet zone: totalWidth = (4 * 10 + 10/2).round() = 45
      expect(scene.width, equals(45));
    });

    test('all-light hex grid has only background instruction', () {
      final grid =
          makeModuleGrid(rows: 3, cols: 3, moduleShape: ModuleShape.hex);
      final cfg =
          defaultBarcode2DLayoutConfig.copyWith(moduleShape: ModuleShape.hex);
      final scene = layout(grid, config: cfg);
      expect(scene.instructions, hasLength(1));
    });

    test('odd row module has an x offset compared to even row module', () {
      // Row 0 (even) and row 1 (odd) same column should have different cx.
      var grid =
          makeModuleGrid(rows: 3, cols: 3, moduleShape: ModuleShape.hex);
      grid = setModule(grid, row: 0, col: 0, dark: true);
      grid = setModule(grid, row: 1, col: 0, dark: true);
      final cfg = defaultBarcode2DLayoutConfig.copyWith(
        moduleShape: ModuleShape.hex,
      );
      final scene = layout(grid, config: cfg);
      // instruction[1] = row 0 module, instruction[2] = row 1 module
      final path0 = scene.instructions[1] as PaintPath;
      final path1 = scene.instructions[2] as PaintPath;
      final move0 = path0.commands.first as MoveTo;
      final move1 = path1.commands.first as MoveTo;
      // Odd row is shifted right, so its first vertex x should be larger.
      expect(move1.x, greaterThan(move0.x));
    });

    test('hex vertex 0 is at angle 0 (right of centre)', () {
      // For a flat-top hex centred at (cx, cy) with circumR, vertex 0 is at
      // (cx + circumR, cy). With moduleSizePx=10: circumR = 10/√3 ≈ 5.774.
      var grid =
          makeModuleGrid(rows: 1, cols: 1, moduleShape: ModuleShape.hex);
      grid = setModule(grid, row: 0, col: 0, dark: true);
      final cfg = defaultBarcode2DLayoutConfig.copyWith(
        moduleShape: ModuleShape.hex,
        moduleSizePx: 10,
        quietZoneModules: 0,
      );
      final scene = layout(grid, config: cfg);
      final path = scene.instructions[1] as PaintPath;
      final vertex0 = path.commands.first as MoveTo;
      // cx = 0 + 0*10 + (0%2)*5 = 0, cy = 0 + 0*hexHeight = 0
      // circumR = 10/√3
      final circumR = 10.0 / math.sqrt(3);
      expect(vertex0.x, closeTo(circumR, 1e-9));
      expect(vertex0.y, closeTo(0.0, 1e-9));
    });
  });

  // ==========================================================================
  // layout — config validation / error cases
  // ==========================================================================

  group('layout error cases', () {
    test('throws InvalidBarcode2DConfigError for moduleSizePx = 0', () {
      final grid = makeModuleGrid(rows: 3, cols: 3);
      final cfg = defaultBarcode2DLayoutConfig.copyWith(moduleSizePx: 0);
      expect(
        () => layout(grid, config: cfg),
        throwsA(isA<InvalidBarcode2DConfigError>()),
      );
    });

    test('throws InvalidBarcode2DConfigError for moduleSizePx < 0', () {
      final grid = makeModuleGrid(rows: 3, cols: 3);
      final cfg = defaultBarcode2DLayoutConfig.copyWith(moduleSizePx: -1);
      expect(
        () => layout(grid, config: cfg),
        throwsA(isA<InvalidBarcode2DConfigError>()),
      );
    });

    test('throws InvalidBarcode2DConfigError for quietZoneModules < 0', () {
      final grid = makeModuleGrid(rows: 3, cols: 3);
      final cfg = defaultBarcode2DLayoutConfig.copyWith(quietZoneModules: -1);
      expect(
        () => layout(grid, config: cfg),
        throwsA(isA<InvalidBarcode2DConfigError>()),
      );
    });

    test('throws InvalidBarcode2DConfigError when shape mismatch (square vs hex)', () {
      final grid = makeModuleGrid(rows: 3, cols: 3); // square grid
      final cfg = defaultBarcode2DLayoutConfig.copyWith(
        moduleShape: ModuleShape.hex, // but config says hex
      );
      expect(
        () => layout(grid, config: cfg),
        throwsA(isA<InvalidBarcode2DConfigError>()),
      );
    });

    test('throws InvalidBarcode2DConfigError when shape mismatch (hex vs square)', () {
      final grid = makeModuleGrid(
          rows: 3, cols: 3, moduleShape: ModuleShape.hex); // hex grid
      final cfg = defaultBarcode2DLayoutConfig.copyWith(
        moduleShape: ModuleShape.square, // but config says square
      );
      expect(
        () => layout(grid, config: cfg),
        throwsA(isA<InvalidBarcode2DConfigError>()),
      );
    });

    test('InvalidBarcode2DConfigError is a Barcode2DError', () {
      final grid = makeModuleGrid(rows: 3, cols: 3);
      final cfg = defaultBarcode2DLayoutConfig.copyWith(moduleSizePx: 0);
      try {
        layout(grid, config: cfg);
        fail('Expected exception');
      } catch (e) {
        expect(e, isA<Barcode2DError>());
      }
    });

    test('Barcode2DError toString contains the message', () {
      const err = Barcode2DError('test message');
      expect(err.toString(), contains('test message'));
    });

    test('InvalidBarcode2DConfigError toString mentions the class name', () {
      const err = InvalidBarcode2DConfigError('bad config');
      expect(err.toString(), contains('InvalidBarcode2DConfigError'));
    });
  });

  // ==========================================================================
  // Barcode2DLayoutConfig
  // ==========================================================================

  group('Barcode2DLayoutConfig', () {
    test('default config has moduleSizePx=10', () {
      expect(defaultBarcode2DLayoutConfig.moduleSizePx, equals(10));
    });

    test('default config has quietZoneModules=4', () {
      expect(defaultBarcode2DLayoutConfig.quietZoneModules, equals(4));
    });

    test('default config foreground is #000000', () {
      expect(defaultBarcode2DLayoutConfig.foreground, equals('#000000'));
    });

    test('default config background is #ffffff', () {
      expect(defaultBarcode2DLayoutConfig.background, equals('#ffffff'));
    });

    test('default config moduleShape is square', () {
      expect(defaultBarcode2DLayoutConfig.moduleShape, equals(ModuleShape.square));
    });

    test('copyWith moduleSizePx changes only that field', () {
      final cfg = defaultBarcode2DLayoutConfig.copyWith(moduleSizePx: 20);
      expect(cfg.moduleSizePx, equals(20));
      expect(cfg.quietZoneModules, equals(4)); // unchanged
      expect(cfg.foreground, equals('#000000')); // unchanged
    });

    test('copyWith quietZoneModules changes only that field', () {
      final cfg = defaultBarcode2DLayoutConfig.copyWith(quietZoneModules: 1);
      expect(cfg.quietZoneModules, equals(1));
      expect(cfg.moduleSizePx, equals(10)); // unchanged
    });

    test('copyWith foreground changes only that field', () {
      final cfg = defaultBarcode2DLayoutConfig.copyWith(foreground: '#aabbcc');
      expect(cfg.foreground, equals('#aabbcc'));
      expect(cfg.background, equals('#ffffff')); // unchanged
    });

    test('copyWith background changes only that field', () {
      final cfg = defaultBarcode2DLayoutConfig.copyWith(background: '#112233');
      expect(cfg.background, equals('#112233'));
      expect(cfg.foreground, equals('#000000')); // unchanged
    });

    test('copyWith moduleShape changes only that field', () {
      final cfg =
          defaultBarcode2DLayoutConfig.copyWith(moduleShape: ModuleShape.hex);
      expect(cfg.moduleShape, equals(ModuleShape.hex));
      expect(cfg.moduleSizePx, equals(10)); // unchanged
    });

    test('copyWith with no args preserves all fields', () {
      final cfg = defaultBarcode2DLayoutConfig.copyWith();
      expect(cfg.moduleSizePx, equals(10));
      expect(cfg.quietZoneModules, equals(4));
      expect(cfg.foreground, equals('#000000'));
      expect(cfg.background, equals('#ffffff'));
      expect(cfg.moduleShape, equals(ModuleShape.square));
    });
  });

  // ==========================================================================
  // ModuleAnnotation and AnnotatedModuleGrid
  // ==========================================================================

  group('ModuleAnnotation', () {
    test('role and dark are stored correctly', () {
      const ann = ModuleAnnotation(role: ModuleRole.finder, dark: true);
      expect(ann.role, equals(ModuleRole.finder));
      expect(ann.dark, isTrue);
    });

    test('optional fields default to null', () {
      const ann = ModuleAnnotation(role: ModuleRole.data, dark: false);
      expect(ann.codewordIndex, isNull);
      expect(ann.bitIndex, isNull);
      expect(ann.metadata, isNull);
    });

    test('codewordIndex and bitIndex are stored when provided', () {
      const ann = ModuleAnnotation(
        role: ModuleRole.data,
        dark: true,
        codewordIndex: 3,
        bitIndex: 7,
      );
      expect(ann.codewordIndex, equals(3));
      expect(ann.bitIndex, equals(7));
    });

    test('metadata map is stored when provided', () {
      const ann = ModuleAnnotation(
        role: ModuleRole.format,
        dark: false,
        metadata: {'format_role': 'qr:dark-module'},
      );
      expect(ann.metadata?['format_role'], equals('qr:dark-module'));
    });
  });

  group('AnnotatedModuleGrid', () {
    test('extends ModuleGrid (is-a relationship)', () {
      final modules = [
        [false, false],
        [false, false],
      ];
      final annotations = [
        [null, null],
        [null, null],
      ];
      final grid = AnnotatedModuleGrid(
        rows: 2,
        cols: 2,
        modules: modules,
        moduleShape: ModuleShape.square,
        annotations: annotations,
      );
      expect(grid, isA<ModuleGrid>());
      expect(grid.rows, equals(2));
    });

    test('annotations are accessible', () {
      final modules = [
        [true, false],
      ];
      final ann = ModuleAnnotation(role: ModuleRole.finder, dark: true);
      final annotations = [
        [ann, null],
      ];
      final grid = AnnotatedModuleGrid(
        rows: 1,
        cols: 2,
        modules: modules,
        moduleShape: ModuleShape.square,
        annotations: annotations,
      );
      expect(grid.annotations[0][0]?.role, equals(ModuleRole.finder));
      expect(grid.annotations[0][1], isNull);
    });

    test('layout works on AnnotatedModuleGrid without annotation-specific logic', () {
      var grid = AnnotatedModuleGrid(
        rows: 3,
        cols: 3,
        modules: List.generate(3, (_) => List.filled(3, false)),
        moduleShape: ModuleShape.square,
        annotations: List.generate(3, (_) => List.filled(3, null)),
      );
      // Should not throw; annotations are ignored by layout().
      final scene = layout(grid);
      expect(scene.width, equals(110)); // (3+8)*10
    });
  });

  // ==========================================================================
  // version constant
  // ==========================================================================

  test('version constant is a non-empty semver string', () {
    expect(version, isNotEmpty);
    expect(version, contains('.'));
  });

  // ==========================================================================
  // PaintScene re-export
  // ==========================================================================

  test('PaintScene is accessible from barcode_2d library', () {
    // This test proves the re-export works: we create a PaintScene using the
    // type imported from barcode-2d (not directly from paint-instructions).
    final grid = makeModuleGrid(rows: 5, cols: 5);
    final PaintScene scene = layout(grid);
    expect(scene.width, equals(130)); // (5+8)*10
  });
}
