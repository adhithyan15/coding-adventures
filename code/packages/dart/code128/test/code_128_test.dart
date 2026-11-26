// Unit tests for the Code 128 lane: the reference test mirrored
// case-for-case, then domain, checksum, framing, geometry, and scene
// coverage.

import 'package:coding_adventures_barcode_layout_1d/barcode_layout_1d.dart';
import 'package:coding_adventures_code_128/code_128.dart';
import 'package:test/test.dart';

void main() {
  group('code128', () {
    // Reference test: computes_checksum_and_builds_scene.
    test('computes checksum and builds a scene', () {
      final encoded = encodeCode128B('HELLO');
      expect(encoded.first.role, equals('start'));
      expect(encoded.last.role, equals('stop'));

      final scene = layoutCode128('HELLO', const Barcode1DSceneOptions());
      expect(scene.metadata['symbology'], equals('code128'));
      expect(scene.metadata['codeSet'], equals('B'));
    });

    test('checksum weights values by position', () {
      // HELLO: 104 + 40*1 + 37*2 + 44*3 + 44*4 + 47*5 = 761; 761 % 103.
      expect(computeCode128Checksum([40, 37, 44, 44, 47]), equals(40));
      // Empty data checksums to the bare start value.
      expect(computeCode128Checksum([]), equals(104 % 103));
      final scene = layoutCode128('HELLO');
      expect(scene.metadata['checksum'], equals('40'));
    });

    test('empty input is valid', () {
      expect(normalizeCode128B(''), equals(''));
      final encoded = encodeCode128B('');
      expect(encoded.map((e) => e.role), equals(['start', 'check', 'stop']));
      expect(expandCode128Runs(''), isNotEmpty);
    });

    test('printable ASCII domain only', () {
      String idOf(String input) {
        try {
          normalizeCode128B(input);
          return 'ok';
        } on InvalidCode128InputError catch (e) {
          return e.errorId;
        }
      }

      expect(idOf(' '), equals('ok'));
      expect(idOf('~'), equals('ok'));
      expect(idOf('Hello, World!'), equals('ok'));
      expect(idOf('\x00'), equals('invalid-character'));
      expect(idOf('\x7f'), equals('invalid-character'));
      expect(idOf('\u{E9}'), equals('invalid-character'));
      expect(idOf('A' * 4097), equals('input-too-long'));
      expect(normalizeCode128B('A' * 4096), hasLength(4096));
    });

    test('lone surrogates throw ArgumentError outside v1', () {
      final loneHigh = String.fromCharCodes([0xd800]);
      final loneLow = String.fromCharCodes([0xdc00]);
      expect(() => normalizeCode128B('A$loneHigh'), throwsArgumentError);
      expect(() => normalizeCode128B('${loneLow}AB'), throwsArgumentError);
      final highAscii = String.fromCharCodes([0xd800, 0x42]);
      expect(() => normalizeCode128B(highAscii), throwsArgumentError);
      final highBmp = String.fromCharCodes([0xd800, 0xff10]);
      expect(() => normalizeCode128B(highBmp), throwsArgumentError);
    });

    test('valid astral pairs count once, then fail the alphabet', () {
      expect(
        () => normalizeCode128B('\u{1F600}'),
        throwsA(
          isA<InvalidCode128InputError>().having(
            (e) => e.errorId,
            'errorId',
            'invalid-character',
          ),
        ),
      );
    });

    test('encoding frames data with start, check, and stop', () {
      final encoded = encodeCode128B('AB');

      expect(encoded.map((e) => e.role),
          equals(['start', 'data', 'data', 'check', 'stop']));
      expect(encoded[0].label, equals('Start B'));
      expect(encoded[0].value, equals(104));
      expect(encoded[0].sourceIndex, equals(-1));
      // A=33, B=34: 104 + 33*1 + 34*2 = 205; 205 % 103 = 102.
      expect(encoded[3].label, equals('Checksum 102'));
      expect(encoded[3].value, equals(102));
      expect(encoded[3].sourceIndex, equals(2));
      expect(encoded[4].label, equals('Stop'));
      expect(encoded[4].value, equals(106));
      expect(encoded[4].pattern, hasLength(13));
      for (final entry in encoded.sublist(0, 4)) {
        expect(entry.pattern, hasLength(11));
      }
    });

    test('run geometry: no gaps, strict alternation', () {
      final runs = expandCode128Runs('AB');
      // 5 symbols x 11 modules + 2 stop modules, no inter-char gaps.
      final modules = runs.fold<int>(0, (sum, run) => sum + run.modules);
      expect(modules, equals(5 * 11 + 2));
      expect(
        runs.where((r) => r.role == 'inter-character-gap'),
        isEmpty,
      );
      for (var i = 1; i < runs.length; i++) {
        expect(runs[i].color, isNot(equals(runs[i - 1].color)));
      }
      expect(runs.first.color, equals('bar'));
    });

    test('scene carries symbology, set, checksum, and text metadata', () {
      final scene = layoutCode128('AB');

      expect(scene.metadata['symbology'], equals('code128'));
      expect(scene.metadata['codeSet'], equals('B'));
      expect(scene.metadata['checksum'], equals('102'));
      expect(scene.metadata['humanReadableText'], equals('AB'));
      expect(scene.metadata['label'], equals('Code 128 barcode for AB'));
      expect(scene.background, equals('#ffffff'));
      expect(scene.instructions, isNotEmpty);
    });

    test('caller options merge with symbology defaults', () {
      final scene = layoutCode128(
        'AB',
        const Barcode1DSceneOptions(label: 'custom', barHeight: 60),
      );

      expect(scene.metadata['label'], equals('custom'));
      expect(scene.metadata['barHeightPx'], equals('60'));
      expect(scene.metadata['symbology'], equals('code128'));
      expect(scene.metadata['codeSet'], equals('B'));
    });

    test('text-enabled projection hits the lane error', () {
      expect(
        () => layoutCode128(
          'AB',
          const Barcode1DSceneOptions(includeHumanReadableText: true),
        ),
        throwsA(isA<Barcode1DV1Error>()),
      );
    });

    test('drawCode128 aliases layoutCode128', () {
      final a = layoutCode128('AB');
      final b = drawCode128('AB');
      expect(b.metadata, equals(a.metadata));
      expect(b.instructions.length, equals(a.instructions.length));
    });

    // NOTE: the encoder's explicit null guard mirrors the sibling lanes
    // but is unreachable even via `dynamic` (the call-boundary downcast
    // throws first); it documents the contract and stays untested.

    test('encoded symbols support value semantics', () {
      const a = EncodedCode128Symbol(
        label: 'A',
        value: 33,
        pattern: '10100011000',
        sourceIndex: 0,
        role: 'data',
      );
      const b = EncodedCode128Symbol(
        label: 'A',
        value: 33,
        pattern: '10100011000',
        sourceIndex: 0,
        role: 'data',
      );
      const c = EncodedCode128Symbol(
        label: 'B',
        value: 34,
        pattern: '10001011000',
        sourceIndex: 1,
        role: 'data',
      );
      expect(a, equals(b));
      expect(a.hashCode, equals(b.hashCode));
      expect(a, isNot(equals(c)));
      expect(a == 'not-a-symbol', isFalse);
      expect(a.toString(), contains('EncodedCode128Symbol(A, 33)'));
    });

    test('error values and messages are payload-blind', () {
      final err = InvalidCode128InputError(Code128ErrorCode.inputTooLong);
      expect(err.errorId, equals('input-too-long'));
      expect(err.message, equals(code128MessageInputTooLong));
      expect(code128MessageFor(null), equals(code128MessageInvalidCharacter));
    });
  });
}
