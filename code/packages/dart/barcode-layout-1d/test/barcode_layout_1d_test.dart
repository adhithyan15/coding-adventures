import 'package:coding_adventures_barcode_layout_1d/barcode_layout_1d.dart';
import 'package:coding_adventures_paint_instructions/paint_instructions.dart';
import 'package:test/test.dart';

void main() {
  group('barcode-layout-1d native examples', () {
    test('binary expansion coalesces adjacent modules', () {
      final runs = BarcodeLayout1DV1.expandBinary('110100', '*', -1, 'start');
      expect(runs.map((run) => run.modules).toList(), [2, 1, 1, 2]);
      expect(runs.map((run) => run.color).toList(),
          ['bar', 'space', 'bar', 'space']);
      expect(() => runs.clear(), throwsUnsupportedError);
    });

    test('layout assigns a gap to the preceding symbol', () {
      final runs = [
        const Barcode1DRun('bar', 1, '*', -1, 'start'),
        const Barcode1DRun('space', 1, '', -1, 'inter-character-gap'),
        const Barcode1DRun('bar', 2, 'A', 0, 'data'),
      ];
      final layout = BarcodeLayout1DV1.computeLayout(runs, 10);
      expect(layout.totalModules, 24);
      expect(layout.symbolLayouts[0].endModule, 2);
      expect(layout.symbolLayouts[1].startModule, 2);
    });

    test('scene contains bars only and owns input collections', () {
      final runs = <Barcode1DRun>[
        const Barcode1DRun('bar', 1, 'A', 0, 'data'),
        const Barcode1DRun('space', 1, 'A', 0, 'data'),
      ];
      final metadata = <String, String>{
        'caller': 'original',
        'totalModules': 'spoof',
      };
      final first = BarcodeLayout1DV1.projectScene(
          runs, 2, Barcode1DSceneOptions(label: 'Demo', metadata: metadata));
      metadata['caller'] = 'changed';
      runs.clear();
      expect(first.instructions, hasLength(1));
      expect(first.width, 24);
      expect(first.metadata['caller'], 'original');
      expect(first.metadata['totalModules'], '6');
      expect((first.instructions.first as PaintRect).x, 8);
      expect(() => first.instructions.clear(), throwsUnsupportedError);
      final second = BarcodeLayout1DV1.projectScene(
        [const Barcode1DRun('bar', 1, 'A', 0, 'data')],
        2,
      );
      expect((second.instructions.first as PaintRect).metadata['sourceLabel'],
          'A');
    });

    test('text value fails before native resolution', () {
      final invalidRuns = [const Barcode1DRun('bar', 0, 'A', 0, 'data')];
      expect(
        () => BarcodeLayout1DV1.projectScene(invalidRuns, 0,
            const Barcode1DSceneOptions(moduleWidth: 0, humanReadableText: '123')),
        throwsA(isA<Barcode1DV1Error>().having((error) => error.errorId,
            'errorId', 'human-readable-text-unsupported')),
      );
    });

    test('text-enabled fails before native resolution', () {
      final invalidRuns = [const Barcode1DRun('bar', 0, 'A', 0, 'data')];
      expect(
        () => BarcodeLayout1DV1.projectScene(invalidRuns, 0,
            const Barcode1DSceneOptions(
                moduleWidth: 0, includeHumanReadableText: true)),
        throwsA(isA<Barcode1DV1Error>().having((error) => error.errorId,
            'errorId', 'human-readable-text-unsupported')),
      );
    });
  });
}
