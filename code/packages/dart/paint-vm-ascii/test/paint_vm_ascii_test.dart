import 'package:coding_adventures_paint_instructions/coding_adventures_paint_instructions.dart'
    hide version;
import 'package:coding_adventures_paint_vm_ascii/coding_adventures_paint_vm_ascii.dart';
import 'package:test/test.dart';

String textOf(PaintVmAsciiResult result) => switch (result) {
      PaintVmAsciiOk(:final text) => text,
      PaintVmAsciiErr(:final error) => throw StateError('render failed: $error'),
    };

PaintVmAsciiError errorOf(PaintVmAsciiResult result) => switch (result) {
      PaintVmAsciiOk() => throw StateError('expected an error, got Ok'),
      PaintVmAsciiErr(:final error) => error,
    };

PaintGlyphRun glyphRunFromString(String text, {double x = 0, double y = 0}) {
  final glyphs = <PaintGlyphPlacement>[];
  for (var i = 0; i < text.length; i++) {
    glyphs.add(PaintGlyphPlacement(
      glyphId: text.codeUnitAt(i),
      x: x + i * 8,
      y: y,
    ));
  }
  return paintGlyphRun(
    glyphs: glyphs,
    fontRef: 'terminal-mono',
    fontSize: 16,
    fill: '#000000',
  );
}

void main() {
  group('version', () {
    test('is a non-empty semver string', () {
      expect(version, isNotEmpty);
      expect(RegExp(r'^\d+\.\d+\.\d+$').hasMatch(version), isTrue);
    });
  });

  group('scale validation', () {
    test('rejects zero scaleX', () {
      final scene = createScene(width: 8, height: 16, instructions: []);
      final result = render(scene, const AsciiOptions(scaleX: 0, scaleY: 16));
      expect(errorOf(result), isA<InvalidScaleX>());
    });

    test('rejects negative scaleY', () {
      final scene = createScene(width: 8, height: 16, instructions: []);
      final result = render(scene, const AsciiOptions(scaleX: 8, scaleY: -1));
      expect(errorOf(result), isA<InvalidScaleY>());
    });
  });

  group('scene dimensions', () {
    test('rejects negative width', () {
      final scene = createScene(width: -1, height: 16, instructions: []);
      expect(errorOf(renderDefault(scene)), isA<InvalidSceneDimensions>());
    });

    test('rejects a scene that exceeds the per-axis cell cap', () {
      final scene = createScene(width: 8 * 3000, height: 16, instructions: []);
      expect(errorOf(renderDefault(scene)), isA<SceneTooLarge>());
    });

    test('rejects a zero-width huge-height scene (product-cap bypass)', () {
      final scene = createScene(width: 0, height: 16 * 5000000, instructions: []);
      expect(errorOf(renderDefault(scene)), isA<SceneTooLarge>());
    });

    test('an empty scene renders to an empty string', () {
      final scene = createScene(width: 8, height: 16, instructions: []);
      expect(textOf(renderDefault(scene)), '');
    });
  });

  group('PaintRect', () {
    test('rejects negative width', () {
      final scene = createScene(
        width: 80,
        height: 16,
        instructions: [
          const PaintRect(x: 0, y: 0, width: -1, height: 1, fill: '#000000', metadata: {}),
        ],
      );
      expect(errorOf(renderDefault(scene)), isA<InvalidRectangleGeometry>());
    });

    test('fills a rectangle with the fill block character', () {
      final scene = createScene(
        width: 24,
        height: 48,
        instructions: [paintRect(x: 0, y: 0, width: 24, height: 48, fill: '#000000')],
      );
      final text = textOf(renderDefault(scene));
      expect(text, '███\n███\n███');
    });

    test('an empty fill string paints nothing', () {
      // Uses the raw PaintRect constructor rather than the paintRect()
      // helper: the helper defaults a blank fill to opaque black (see its
      // doc comment), so it can't express "no fill" on its own.
      final scene = createScene(
        width: 24,
        height: 16,
        instructions: const [
          PaintRect(x: 0, y: 0, width: 24, height: 16, fill: '', metadata: {}),
        ],
      );
      expect(textOf(renderDefault(scene)), '');
    });

    test('stroke draws a box border', () {
      // Raw constructor again, for the same reason: fill must stay blank.
      final scene = createScene(
        width: 24,
        height: 48,
        instructions: const [
          PaintRect(
            x: 0,
            y: 0,
            width: 24,
            height: 48,
            fill: '',
            metadata: {},
            stroke: '#000000',
          ),
        ],
      );
      final text = textOf(renderDefault(scene));
      expect(text, '┌─┐\n│ │\n└─┘');
    });
  });

  group('PaintLine', () {
    test('rejects non-finite coordinates', () {
      final scene = createScene(
        width: 80,
        height: 16,
        instructions: [
          const PaintLine(
            x1: double.nan,
            y1: 0,
            x2: 10,
            y2: 0,
            stroke: '#000000',
            strokeWidth: 1,
            metadata: {},
          ),
        ],
      );
      expect(errorOf(renderDefault(scene)), isA<InvalidLineGeometry>());
    });

    test('draws a horizontal line', () {
      final scene = createScene(
        width: 32,
        height: 16,
        instructions: [
          paintLine(x1: 0, y1: 0, x2: 24, y2: 0, stroke: '#000000', strokeWidth: 1),
        ],
      );
      expect(textOf(renderDefault(scene)), '────');
    });

    test('draws a vertical line', () {
      final scene = createScene(
        width: 8,
        height: 48,
        instructions: [
          paintLine(x1: 0, y1: 0, x2: 0, y2: 32, stroke: '#000000', strokeWidth: 1),
        ],
      );
      expect(textOf(renderDefault(scene)), '│\n│\n│');
    });

    test('draws a diagonal line via Bresenham', () {
      final scene = createScene(
        width: 32,
        height: 32,
        instructions: [
          paintLine(x1: 0, y1: 0, x2: 24, y2: 32, stroke: '#000000', strokeWidth: 1),
        ],
      );
      final text = textOf(renderDefault(scene));
      expect(text, '──\n  ──');
    });

    test('a shallow-slope diagonal line terminates (regression: a zero-seeded '
        'Bresenham error term can overshoot the target row and loop forever)',
        () {
      // deltaRow=1, deltaCol=3 against a target whose row got clamped by
      // the clip is exactly the ratio that exposed the bug: with `error`
      // seeded to 0 instead of `deltaCol - deltaRow`, `row` steps past
      // `r2` and the loop's `row == r2 && col == c2` break condition never
      // becomes true again. This test's only real assertion is that
      // `render` returns at all.
      final scene = createScene(
        width: 32,
        height: 16,
        instructions: [
          paintLine(x1: 0, y1: 0, x2: 24, y2: 32, stroke: '#000000', strokeWidth: 1),
        ],
      );
      expect(renderDefault(scene), isA<PaintVmAsciiOk>());
    });
  });

  group('PaintGlyphRun', () {
    test('places glyphs at scaled coordinates', () {
      final scene = createScene(
        width: 32,
        height: 16,
        instructions: [glyphRunFromString('Hi')],
      );
      expect(textOf(renderDefault(scene)), 'Hi');
    });

    test('a non-finite glyph position is skipped, not fatal', () {
      final scene = createScene(
        width: 32,
        height: 16,
        instructions: [
          paintGlyphRun(
            glyphs: [
              const PaintGlyphPlacement(glyphId: 72, x: 0, y: 0),
              PaintGlyphPlacement(glyphId: 105, x: double.nan, y: 0),
            ],
            fontRef: 'terminal-mono',
            fontSize: 16,
            fill: '#000000',
          ),
        ],
      );
      expect(textOf(renderDefault(scene)), 'H');
    });

    test('control characters are replaced with ?', () {
      final scene = createScene(
        width: 8,
        height: 16,
        instructions: [
          paintGlyphRun(
            glyphs: [const PaintGlyphPlacement(glyphId: 0x07, x: 0, y: 0)],
            fontRef: 'terminal-mono',
            fontSize: 16,
            fill: '#000000',
          ),
        ],
      );
      expect(textOf(renderDefault(scene)), '?');
    });

    test('a lone UTF-16 surrogate code point is replaced with ?', () {
      final scene = createScene(
        width: 8,
        height: 16,
        instructions: [
          paintGlyphRun(
            glyphs: [const PaintGlyphPlacement(glyphId: 0xD800, x: 0, y: 0)],
            fontRef: 'terminal-mono',
            fontSize: 16,
            fill: '#000000',
          ),
        ],
      );
      expect(textOf(renderDefault(scene)), '?');
    });

    test('a supplementary-plane code point renders as its own glyph', () {
      final scene = createScene(
        width: 8,
        height: 16,
        instructions: [
          paintGlyphRun(
            glyphs: [const PaintGlyphPlacement(glyphId: 0x1F600, x: 0, y: 0)],
            fontRef: 'terminal-mono',
            fontSize: 16,
            fill: '#000000',
          ),
        ],
      );
      expect(textOf(renderDefault(scene)), String.fromCharCode(0x1F600));
    });
  });

  group('PaintGroup', () {
    test('renders plain group children', () {
      final scene = createScene(
        width: 16,
        height: 16,
        instructions: [
          paintGroup(children: [glyphRunFromString('Hi')]),
        ],
      );
      expect(textOf(renderDefault(scene)), 'Hi');
    });

    test('rejects a non-identity transform', () {
      final scene = createScene(
        width: 16,
        height: 16,
        instructions: [
          PaintGroup(
            children: const [],
            transform: const Transform2D(a: 2, b: 0, c: 0, d: 1, e: 0, f: 0),
          ),
        ],
      );
      expect(errorOf(renderDefault(scene)), isA<UnsupportedInstruction>());
    });

    test('rejects non-default opacity', () {
      final scene = createScene(
        width: 16,
        height: 16,
        instructions: [PaintGroup(children: const [], opacity: 0.5)],
      );
      expect(errorOf(renderDefault(scene)), isA<UnsupportedInstruction>());
    });
  });

  group('PaintClip', () {
    test('rejects non-finite geometry', () {
      final scene = createScene(
        width: 16,
        height: 16,
        instructions: [
          const PaintClip(
              x: double.nan, y: 0, width: 8, height: 8, children: [], metadata: {}),
        ],
      );
      expect(errorOf(renderDefault(scene)), isA<InvalidClipGeometry>());
    });

    test('rejects an x+width extent that overflows to infinity', () {
      final scene = createScene(
        width: 16,
        height: 16,
        instructions: [
          PaintClip(
            x: double.maxFinite,
            y: 0,
            width: double.maxFinite,
            height: 8,
            children: const [],
          ),
        ],
      );
      expect(errorOf(renderDefault(scene)), isA<InvalidClipGeometry>());
    });

    test('clips children to its bounds', () {
      final scene = createScene(
        width: 16,
        height: 16,
        instructions: [
          paintClip(
            x: 0,
            y: 0,
            width: 8,
            height: 16,
            children: [glyphRunFromString('Hi')],
          ),
        ],
      );
      expect(textOf(renderDefault(scene)), 'H');
    });
  });

  group('PaintLayer', () {
    test('renders plain layer children', () {
      final scene = createScene(
        width: 16,
        height: 16,
        instructions: [
          paintLayer(children: [glyphRunFromString('Hi')]),
        ],
      );
      expect(textOf(renderDefault(scene)), 'Hi');
    });

    test('rejects a layer with filters', () {
      final scene = createScene(
        width: 16,
        height: 16,
        instructions: [PaintLayer(children: const [], hasFilters: true)],
      );
      expect(errorOf(renderDefault(scene)), isA<UnsupportedInstruction>());
    });

    test('rejects a non-normal blend mode', () {
      final scene = createScene(
        width: 16,
        height: 16,
        instructions: [PaintLayer(children: const [], blendMode: 'multiply')],
      );
      expect(errorOf(renderDefault(scene)), isA<UnsupportedInstruction>());
    });
  });

  group('nesting depth', () {
    PaintGroup nestedGroups(int depth, PaintInstruction leaf) {
      var current = leaf;
      for (var i = 0; i < depth; i++) {
        current = paintGroup(children: [current]);
      }
      return current as PaintGroup;
    }

    test('a deeply nested scene is rejected rather than overflowing the stack', () {
      final scene = createScene(
        width: 16,
        height: 16,
        instructions: [nestedGroups(200, glyphRunFromString('x'))],
      );
      expect(errorOf(renderDefault(scene)), isA<SceneTooDeep>());
    });

    test('a moderately nested scene still renders', () {
      final scene = createScene(
        width: 16,
        height: 16,
        instructions: [nestedGroups(10, glyphRunFromString('x'))],
      );
      expect(textOf(renderDefault(scene)), 'x');
    });
  });

  group('PaintPath', () {
    test('is unsupported by this backend', () {
      final scene = createScene(
        width: 16,
        height: 16,
        instructions: [
          paintPath(commands: [PathCommand.moveTo(0, 0), PathCommand.close()]),
        ],
      );
      expect(errorOf(renderDefault(scene)), isA<UnsupportedInstruction>());
    });
  });

  group('box-drawing merges', () {
    test('two rectangles sharing an edge merge into tee characters', () {
      // Raw PaintRect constructors: the paintRect() helper defaults a
      // blank fill to opaque black, which would defeat this stroke-only
      // test.
      final scene = createScene(
        width: 40,
        height: 48,
        instructions: const [
          PaintRect(
            x: 0,
            y: 0,
            width: 16,
            height: 48,
            fill: '',
            metadata: {},
            stroke: '#000000',
          ),
          PaintRect(
            x: 16,
            y: 0,
            width: 16,
            height: 48,
            fill: '',
            metadata: {},
            stroke: '#000000',
          ),
        ],
      );
      final text = textOf(renderDefault(scene));
      expect(text, '┌─┬─┐\n│ │ │\n└─┴─┘');
    });
  });

  group('trailing whitespace trimming', () {
    test('trims trailing spaces per line and trailing blank lines', () {
      final scene = createScene(
        width: 40,
        height: 32,
        instructions: [glyphRunFromString('Hi', y: 0)],
      );
      final text = textOf(renderDefault(scene));
      expect(text, 'Hi');
      expect(text.endsWith('\n'), isFalse);
    });
  });
}
