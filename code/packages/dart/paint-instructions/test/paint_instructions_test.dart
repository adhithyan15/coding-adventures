/// Tests for coding_adventures_paint_instructions.
///
/// Coverage goal: ≥ 90% of all reachable lines.
///
/// Test groups:
///   - PathCommand construction and equality
///   - PaintRect construction, defaults, equality
///   - PaintPath construction, defaults, equality
///   - PaintScene construction
///   - createScene helper defaults
///   - paintRect helper defaults
///   - paintPath helper defaults
///   - parseColorRGBA8: happy paths (all four formats)
///   - parseColorRGBA8: error paths
///   - Sealed class exhaustiveness (switch expression)
import 'package:coding_adventures_paint_instructions/coding_adventures_paint_instructions.dart';
import 'package:test/test.dart';

void main() {
  // ==========================================================================
  // PathCommand
  // ==========================================================================

  group('PathCommand', () {
    group('MoveTo', () {
      test('kind is move_to', () {
        final cmd = PathCommand.moveTo(10.0, 20.0);
        expect(cmd.kind, equals('move_to'));
      });

      test('x and y are stored correctly', () {
        final cmd = PathCommand.moveTo(3.5, 7.25);
        expect(cmd.x, equals(3.5));
        expect(cmd.y, equals(7.25));
      });

      test('equality: same coords are equal', () {
        final a = PathCommand.moveTo(1.0, 2.0);
        final b = PathCommand.moveTo(1.0, 2.0);
        expect(a, equals(b));
      });

      test('equality: different coords are not equal', () {
        final a = PathCommand.moveTo(1.0, 2.0);
        final b = PathCommand.moveTo(1.0, 3.0);
        expect(a, isNot(equals(b)));
      });

      test('hashCode matches for equal commands', () {
        final a = PathCommand.moveTo(5.0, 5.0);
        final b = PathCommand.moveTo(5.0, 5.0);
        expect(a.hashCode, equals(b.hashCode));
      });

      test('toString contains coordinates', () {
        final cmd = PathCommand.moveTo(10.0, 20.0);
        expect(cmd.toString(), contains('10'));
        expect(cmd.toString(), contains('20'));
      });
    });

    group('LineTo', () {
      test('kind is line_to', () {
        final cmd = PathCommand.lineTo(50.0, 60.0);
        expect(cmd.kind, equals('line_to'));
      });

      test('x and y are stored correctly', () {
        final cmd = PathCommand.lineTo(100.0, 200.0);
        expect(cmd.x, equals(100.0));
        expect(cmd.y, equals(200.0));
      });

      test('equality: same coords are equal', () {
        final a = PathCommand.lineTo(4.0, 8.0);
        final b = PathCommand.lineTo(4.0, 8.0);
        expect(a, equals(b));
      });

      test('equality: different coords are not equal', () {
        final a = PathCommand.lineTo(4.0, 8.0);
        final b = PathCommand.lineTo(4.0, 9.0);
        expect(a, isNot(equals(b)));
      });

      test('hashCode matches for equal commands', () {
        final a = PathCommand.lineTo(3.0, 3.0);
        final b = PathCommand.lineTo(3.0, 3.0);
        expect(a.hashCode, equals(b.hashCode));
      });
    });

    group('Close', () {
      test('kind is close', () {
        final cmd = PathCommand.close();
        expect(cmd.kind, equals('close'));
      });

      test('x and y are always 0.0', () {
        final cmd = PathCommand.close();
        expect(cmd.x, equals(0.0));
        expect(cmd.y, equals(0.0));
      });

      test('two Close instances are equal', () {
        expect(PathCommand.close(), equals(PathCommand.close()));
      });

      test('Close is not equal to MoveTo', () {
        expect(PathCommand.close(), isNot(equals(PathCommand.moveTo(0, 0))));
      });

      test('toString mentions close', () {
        expect(PathCommand.close().toString(), contains('Close'));
      });
    });

    test('MoveTo != LineTo even with same coords', () {
      // They are different sealed subtypes.
      expect(PathCommand.moveTo(0, 0), isNot(equals(PathCommand.lineTo(0, 0))));
    });
  });

  // ==========================================================================
  // PaintRect
  // ==========================================================================

  group('paintRect helper', () {
    test('creates a PaintRect with explicit values', () {
      final r = paintRect(x: 5, y: 10, width: 20, height: 30, fill: '#ff0000');
      expect(r.x, equals(5));
      expect(r.y, equals(10));
      expect(r.width, equals(20));
      expect(r.height, equals(30));
      expect(r.fill, equals('#ff0000'));
    });

    test('fill defaults to #000000 when omitted', () {
      final r = paintRect(x: 0, y: 0, width: 1, height: 1);
      expect(r.fill, equals('#000000'));
    });

    test('fill defaults to #000000 when empty string supplied', () {
      final r = paintRect(x: 0, y: 0, width: 1, height: 1, fill: '');
      expect(r.fill, equals('#000000'));
    });

    test('metadata defaults to empty map', () {
      final r = paintRect(x: 0, y: 0, width: 1, height: 1);
      expect(r.metadata, isEmpty);
    });

    test('metadata is preserved when supplied', () {
      final r = paintRect(
        x: 0, y: 0, width: 1, height: 1,
        metadata: {'role': 'finder', 'row': 0},
      );
      expect(r.metadata['role'], equals('finder'));
    });

    test('instructionKind is rect', () {
      final r = paintRect(x: 0, y: 0, width: 1, height: 1);
      expect(r.instructionKind, equals('rect'));
    });

    test('equality: identical parameters produce equal rects', () {
      final a = paintRect(x: 1, y: 2, width: 3, height: 4, fill: '#abc');
      // Note: '#abc' != '#000000', so this is fine (fill stays '#abc').
      final b = paintRect(x: 1, y: 2, width: 3, height: 4, fill: '#abc');
      expect(a, equals(b));
    });

    test('equality: different x is not equal', () {
      final a = paintRect(x: 1, y: 2, width: 3, height: 4);
      final b = paintRect(x: 2, y: 2, width: 3, height: 4);
      expect(a, isNot(equals(b)));
    });

    test('hashCode matches for equal rects', () {
      final a = paintRect(x: 5, y: 6, width: 7, height: 8, fill: '#abc');
      final b = paintRect(x: 5, y: 6, width: 7, height: 8, fill: '#abc');
      expect(a.hashCode, equals(b.hashCode));
    });

    test('toString contains position info', () {
      final r = paintRect(x: 3, y: 4, width: 5, height: 6);
      expect(r.toString(), contains('3'));
      expect(r.toString(), contains('4'));
    });
  });

  // ==========================================================================
  // PaintPath
  // ==========================================================================

  group('paintPath helper', () {
    final triangle = [
      PathCommand.moveTo(0, 0),
      PathCommand.lineTo(50, 100),
      PathCommand.lineTo(100, 0),
      PathCommand.close(),
    ];

    test('creates a PaintPath with supplied commands', () {
      final p = paintPath(commands: triangle, fill: '#ff0000');
      expect(p.commands, hasLength(4));
      expect(p.fill, equals('#ff0000'));
    });

    test('fill defaults to #000000 when omitted', () {
      final p = paintPath(commands: triangle);
      expect(p.fill, equals('#000000'));
    });

    test('fill defaults to #000000 when empty string supplied', () {
      final p = paintPath(commands: triangle, fill: '');
      expect(p.fill, equals('#000000'));
    });

    test('metadata defaults to empty map', () {
      final p = paintPath(commands: triangle);
      expect(p.metadata, isEmpty);
    });

    test('instructionKind is path', () {
      final p = paintPath(commands: triangle);
      expect(p.instructionKind, equals('path'));
    });

    test('equality: same commands and fill are equal', () {
      final a = paintPath(commands: triangle, fill: '#123456');
      final b = paintPath(commands: triangle, fill: '#123456');
      expect(a, equals(b));
    });

    test('equality: different fill is not equal', () {
      final a = paintPath(commands: triangle, fill: '#000000');
      final b = paintPath(commands: triangle, fill: '#ffffff');
      expect(a, isNot(equals(b)));
    });

    test('equality: extra command makes paths not equal', () {
      final cmdsA = [...triangle];
      final cmdsB = [...triangle, PathCommand.close()];
      final a = paintPath(commands: cmdsA);
      final b = paintPath(commands: cmdsB);
      expect(a, isNot(equals(b)));
    });

    test('hashCode matches for equal paths', () {
      final a = paintPath(commands: triangle, fill: '#aabbcc');
      final b = paintPath(commands: triangle, fill: '#aabbcc');
      expect(a.hashCode, equals(b.hashCode));
    });

    test('toString mentions command count', () {
      final p = paintPath(commands: triangle);
      expect(p.toString(), contains('4'));
    });
  });

  // ==========================================================================
  // PaintScene / createScene
  // ==========================================================================

  group('createScene helper', () {
    test('creates a PaintScene with supplied values', () {
      final instr = [paintRect(x: 0, y: 0, width: 10, height: 10)];
      final scene = createScene(
        width: 100,
        height: 200,
        background: '#aabbcc',
        instructions: instr,
      );
      expect(scene.width, equals(100));
      expect(scene.height, equals(200));
      expect(scene.background, equals('#aabbcc'));
      expect(scene.instructions, hasLength(1));
    });

    test('background defaults to #ffffff', () {
      final scene = createScene(width: 10, height: 10, instructions: []);
      expect(scene.background, equals('#ffffff'));
    });

    test('background defaults to #ffffff when empty string supplied', () {
      final scene =
          createScene(width: 10, height: 10, instructions: [], background: '');
      expect(scene.background, equals('#ffffff'));
    });

    test('metadata defaults to empty map', () {
      final scene = createScene(width: 10, height: 10, instructions: []);
      expect(scene.metadata, isEmpty);
    });

    test('metadata is preserved when supplied', () {
      final scene = createScene(
        width: 10, height: 10, instructions: [],
        metadata: {'source': 'qr-code'},
      );
      expect(scene.metadata['source'], equals('qr-code'));
    });

    test('instructions list is preserved in order', () {
      final r1 = paintRect(x: 0, y: 0, width: 5, height: 5, fill: '#ff0000');
      final r2 = paintRect(x: 5, y: 5, width: 5, height: 5, fill: '#0000ff');
      final scene = createScene(
        width: 20, height: 20, instructions: [r1, r2],
      );
      expect(scene.instructions[0], equals(r1));
      expect(scene.instructions[1], equals(r2));
    });

    test('toString mentions dimensions', () {
      final scene = createScene(width: 210, height: 210, instructions: []);
      expect(scene.toString(), contains('210'));
    });
  });

  // ==========================================================================
  // parseColorRGBA8 — happy paths
  // ==========================================================================

  group('parseColorRGBA8', () {
    group('#rrggbb (6-digit)', () {
      test('red #ff0000', () {
        final c = parseColorRGBA8('#ff0000');
        expect(c, equals(PaintColorRGBA8(r: 255, g: 0, b: 0, a: 255)));
      });

      test('green #00ff00', () {
        final c = parseColorRGBA8('#00ff00');
        expect(c, equals(PaintColorRGBA8(r: 0, g: 255, b: 0, a: 255)));
      });

      test('blue #0000ff', () {
        final c = parseColorRGBA8('#0000ff');
        expect(c, equals(PaintColorRGBA8(r: 0, g: 0, b: 255, a: 255)));
      });

      test('white #ffffff → a=255', () {
        final c = parseColorRGBA8('#ffffff');
        expect(c.a, equals(255));
      });

      test('black #000000', () {
        final c = parseColorRGBA8('#000000');
        expect(c, equals(PaintColorRGBA8(r: 0, g: 0, b: 0, a: 255)));
      });

      test('uppercase hex is accepted', () {
        final c = parseColorRGBA8('#FF0000');
        expect(c.r, equals(255));
      });

      test('mixed case is accepted', () {
        final c = parseColorRGBA8('#fF00Ff');
        expect(c.r, equals(255));
        expect(c.g, equals(0));
        expect(c.b, equals(255));
      });
    });

    group('#rrggbbaa (8-digit)', () {
      test('semi-transparent blue #0000ff80', () {
        final c = parseColorRGBA8('#0000ff80');
        expect(c.r, equals(0));
        expect(c.g, equals(0));
        expect(c.b, equals(255));
        expect(c.a, equals(128));
      });

      test('fully transparent #00000000', () {
        final c = parseColorRGBA8('#00000000');
        expect(c.a, equals(0));
      });

      test('fully opaque #ffffffff', () {
        final c = parseColorRGBA8('#ffffffff');
        expect(c, equals(PaintColorRGBA8(r: 255, g: 255, b: 255, a: 255)));
      });
    });

    group('#rgb (3-digit shorthand)', () {
      test('#f00 expands to #ff0000ff', () {
        final c = parseColorRGBA8('#f00');
        expect(c, equals(PaintColorRGBA8(r: 255, g: 0, b: 0, a: 255)));
      });

      test('#fff expands to #ffffffff', () {
        final c = parseColorRGBA8('#fff');
        expect(c, equals(PaintColorRGBA8(r: 255, g: 255, b: 255, a: 255)));
      });

      test('#000 expands to #000000ff', () {
        final c = parseColorRGBA8('#000');
        expect(c, equals(PaintColorRGBA8(r: 0, g: 0, b: 0, a: 255)));
      });

      test('#abc expands correctly: #aabbcc', () {
        final c = parseColorRGBA8('#abc');
        expect(c.r, equals(0xAA));
        expect(c.g, equals(0xBB));
        expect(c.b, equals(0xCC));
        expect(c.a, equals(255));
      });
    });

    group('#rgba (4-digit shorthand)', () {
      test('#f00f expands to #ff0000ff', () {
        final c = parseColorRGBA8('#f00f');
        expect(c, equals(PaintColorRGBA8(r: 255, g: 0, b: 0, a: 255)));
      });

      test('#0008 expands to #000000 with a=0x88=136', () {
        final c = parseColorRGBA8('#0008');
        expect(c.r, equals(0));
        expect(c.a, equals(0x88));
      });
    });

    group('whitespace trimming', () {
      test('leading and trailing spaces are trimmed', () {
        final c = parseColorRGBA8('  #ff0000  ');
        expect(c.r, equals(255));
      });
    });

    group('equality and hashCode', () {
      test('two equal colours are ==', () {
        final a = PaintColorRGBA8(r: 10, g: 20, b: 30, a: 255);
        final b = PaintColorRGBA8(r: 10, g: 20, b: 30, a: 255);
        expect(a, equals(b));
      });

      test('colours with different alpha differ', () {
        final a = PaintColorRGBA8(r: 0, g: 0, b: 0, a: 255);
        final b = PaintColorRGBA8(r: 0, g: 0, b: 0, a: 0);
        expect(a, isNot(equals(b)));
      });

      test('hashCode matches for equal colours', () {
        final a = PaintColorRGBA8(r: 1, g: 2, b: 3, a: 4);
        final b = PaintColorRGBA8(r: 1, g: 2, b: 3, a: 4);
        expect(a.hashCode, equals(b.hashCode));
      });

      test('toString contains channel values', () {
        final c = PaintColorRGBA8(r: 255, g: 0, b: 0, a: 128);
        expect(c.toString(), contains('255'));
        expect(c.toString(), contains('128'));
      });
    });

    // ---- Error paths ----

    group('error cases', () {
      test('throws FormatException when missing # prefix', () {
        expect(
          () => parseColorRGBA8('ff0000'),
          throwsA(isA<FormatException>()),
        );
      });

      test('throws FormatException for 5-digit hex', () {
        expect(
          () => parseColorRGBA8('#12345'),
          throwsA(isA<FormatException>()),
        );
      });

      test('throws FormatException for 7-digit hex', () {
        expect(
          () => parseColorRGBA8('#1234567'),
          throwsA(isA<FormatException>()),
        );
      });

      test('throws FormatException for invalid hex digits', () {
        expect(
          () => parseColorRGBA8('#zzzzzz'),
          throwsA(isA<FormatException>()),
        );
      });

      test('throws FormatException for empty string', () {
        expect(
          () => parseColorRGBA8(''),
          throwsA(isA<FormatException>()),
        );
      });

      test('throws FormatException for just #', () {
        expect(
          () => parseColorRGBA8('#'),
          throwsA(isA<FormatException>()),
        );
      });
    });
  });

  // ==========================================================================
  // Sealed class switch exhaustiveness
  // ==========================================================================

  group('PaintInstruction sealed switch', () {
    // Deliberately exercises every permitted subtype with no wildcard/else
    // branch: adding an eighth subtype to PaintInstruction should make this
    // fail to compile (exhaustiveness check), not fail silently at runtime
    // -- that's the whole point of a sealed class.
    String kindOf(PaintInstruction instr) => switch (instr) {
          PaintRect() => 'rect',
          PaintPath() => 'path',
          PaintLine() => 'line',
          PaintGlyphRun() => 'glyph_run',
          PaintGroup() => 'group',
          PaintClip() => 'clip',
          PaintLayer() => 'layer',
        };

    test('switch on PaintRect produces "rect"', () {
      final instr = paintRect(x: 0, y: 0, width: 1, height: 1);
      expect(kindOf(instr), equals('rect'));
    });

    test('switch on PaintPath produces "path"', () {
      final instr = paintPath(commands: [PathCommand.close()]);
      expect(kindOf(instr), equals('path'));
    });

    test('switch on PaintLine produces "line"', () {
      final instr = paintLine(
        x1: 0,
        y1: 0,
        x2: 1,
        y2: 1,
        stroke: '#000',
        strokeWidth: 1,
      );
      expect(kindOf(instr), equals('line'));
    });

    test('switch on PaintGlyphRun produces "glyph_run"', () {
      final instr = paintGlyphRun(
        glyphs: const [],
        fontRef: 'font',
        fontSize: 1,
        fill: '#000',
      );
      expect(kindOf(instr), equals('glyph_run'));
    });

    test('switch on PaintGroup produces "group"', () {
      final instr = paintGroup(children: const []);
      expect(kindOf(instr), equals('group'));
    });

    test('switch on PaintClip produces "clip"', () {
      final instr =
          paintClip(x: 0, y: 0, width: 1, height: 1, children: const []);
      expect(kindOf(instr), equals('clip'));
    });

    test('switch on PaintLayer produces "layer"', () {
      final instr = paintLayer(children: const []);
      expect(kindOf(instr), equals('layer'));
    });
  });

  // ==========================================================================
  // PaintRect stroke (P2D02 extension)
  // ==========================================================================

  group('PaintRect stroke', () {
    test('stroke and strokeWidth default to empty and zero', () {
      const r = PaintRect(x: 0, y: 0, width: 10, height: 10, fill: '#000', metadata: {});
      expect(r.stroke, equals(''));
      expect(r.strokeWidth, equals(0.0));
    });

    test('stroke and strokeWidth are stored when provided', () {
      const r = PaintRect(
        x: 0,
        y: 0,
        width: 10,
        height: 10,
        fill: '',
        metadata: {},
        stroke: '#000000',
        strokeWidth: 1.5,
      );
      expect(r.stroke, equals('#000000'));
      expect(r.strokeWidth, equals(1.5));
    });
  });

  // ==========================================================================
  // Transform2D
  // ==========================================================================

  group('Transform2D', () {
    test('identity reports isIdentity true', () {
      expect(Transform2D.identity.isIdentity, isTrue);
    });

    test('non-identity reports isIdentity false', () {
      const t = Transform2D(a: 2, b: 0, c: 0, d: 1, e: 0, f: 0);
      expect(t.isIdentity, isFalse);
    });
  });

  // ==========================================================================
  // PaintGlyphPlacement
  // ==========================================================================

  test('PaintGlyphPlacement stores glyphId x y', () {
    const p = PaintGlyphPlacement(glyphId: 104, x: 8, y: 16);
    expect(p.glyphId, equals(104));
    expect(p.x, equals(8.0));
    expect(p.y, equals(16.0));
  });

  // ==========================================================================
  // PaintLine
  // ==========================================================================

  test('paintLine stores endpoints stroke and strokeWidth', () {
    final line = paintLine(x1: 0, y1: 0, x2: 10, y2: 20, stroke: '#000000', strokeWidth: 1);
    expect(line.x1, equals(0.0));
    expect(line.y1, equals(0.0));
    expect(line.x2, equals(10.0));
    expect(line.y2, equals(20.0));
    expect(line.stroke, equals('#000000'));
    expect(line.strokeWidth, equals(1.0));
  });

  // ==========================================================================
  // PaintGlyphRun
  // ==========================================================================

  test('paintGlyphRun stores glyphs fontRef fontSize fill', () {
    final run = paintGlyphRun(
      glyphs: const [
        PaintGlyphPlacement(glyphId: 104, x: 0, y: 0),
        PaintGlyphPlacement(glyphId: 105, x: 8, y: 0),
      ],
      fontRef: 'terminal-mono',
      fontSize: 16,
      fill: '#000000',
    );
    expect(run.glyphs.length, equals(2));
    expect(run.fontRef, equals('terminal-mono'));
    expect(run.fontSize, equals(16.0));
    expect(run.fill, equals('#000000'));
  });

  // ==========================================================================
  // PaintGroup
  // ==========================================================================

  group('PaintGroup', () {
    test('has no transform and no opacity by default', () {
      final group = paintGroup(children: const []);
      expect(group.transform, isNull);
      expect(group.opacity, isNull);
    });

    test('preserves children in order', () {
      final rect = paintRect(x: 0, y: 0, width: 1, height: 1);
      final group = paintGroup(children: [rect]);
      expect(group.children.length, equals(1));
      expect(group.children[0], same(rect));
    });

    test('explicit transform and opacity are stored', () {
      const group = PaintGroup(
        children: [],
        transform: Transform2D(a: 2, b: 0, c: 0, d: 2, e: 0, f: 0),
        opacity: 0.5,
      );
      expect(group.transform!.a, equals(2.0));
      expect(group.opacity, equals(0.5));
    });
  });

  // ==========================================================================
  // PaintClip
  // ==========================================================================

  test('paintClip stores clip bounds and children', () {
    final glyph = paintGlyphRun(
      glyphs: const [PaintGlyphPlacement(glyphId: 97, x: 0, y: 0)],
      fontRef: 'font',
      fontSize: 1,
      fill: '#000',
    );
    final clip = paintClip(x: 0, y: 0, width: 8, height: 16, children: [glyph]);
    expect(clip.x, equals(0.0));
    expect(clip.y, equals(0.0));
    expect(clip.width, equals(8.0));
    expect(clip.height, equals(16.0));
    expect(clip.children.length, equals(1));
  });

  // ==========================================================================
  // PaintLayer
  // ==========================================================================

  group('PaintLayer', () {
    test('has no filters and no transform opacity blendMode by default', () {
      final layer = paintLayer(children: const []);
      expect(layer.hasFilters, isFalse);
      expect(layer.blendMode, isNull);
      expect(layer.opacity, isNull);
      expect(layer.transform, isNull);
    });

    test('hasFilters can be set explicitly', () {
      const layer = PaintLayer(children: [], hasFilters: true);
      expect(layer.hasFilters, isTrue);
    });
  });

  // ==========================================================================
  // Version constant
  // ==========================================================================

  test('version constant is a non-empty semver string', () {
    expect(version, isNotEmpty);
    expect(version, contains('.'));
  });
}
