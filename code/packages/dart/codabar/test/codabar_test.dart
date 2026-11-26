// Unit tests for the Codabar lane: the 2 reference tests mirrored
// case-for-case, then guard form, options, folding, the alphabet,
// geometry, and scene coverage.

import 'package:coding_adventures_barcode_layout_1d/barcode_layout_1d.dart';
import 'package:coding_adventures_codabar/codabar.dart';
import 'package:test/test.dart';

String _idOf(String data, {String? start, String? stop}) {
  try {
    normalizeCodabar(data, start: start, stop: stop);
    return 'ok';
  } on InvalidCodabarInputError catch (e) {
    return e.errorId;
  }
}

void main() {
  group('codabar', () {
    // Reference test 1: normalizes_and_expands_codabar.
    test('normalizes and expands with custom guards', () {
      expect(
          normalizeCodabar('40156', start: 'B', stop: 'D'), equals('B40156D'));

      final runs = expandCodabarRuns('40156', start: 'B', stop: 'D');
      expect(runs, isNotEmpty);
      expect(runs[0].role, equals('start'));
    });

    // Reference test 2: builds_paint_scene.
    test('builds a paint scene with guard metadata', () {
      final scene = layoutCodabar(
        '40156',
        start: 'B',
        stop: 'D',
        options: const Barcode1DSceneOptions(),
      );
      expect(scene.metadata['symbology'], equals('codabar'));
      expect(scene.metadata['start'], equals('B'));
      expect(scene.metadata['stop'], equals('D'));
    });

    test('empty body normalizes to AA', () {
      expect(normalizeCodabar(''), equals('AA'));
      expect(encodeCodabar(''), hasLength(2));
    });

    test('options default independently to A', () {
      expect(normalizeCodabar('0', start: 'B'), equals('B0A'));
      expect(normalizeCodabar('0', stop: 'D'), equals('A0D'));
      expect(normalizeCodabar('0'), equals('A0A'));
    });

    test('guard form: partial, single, and interior guards fail', () {
      expect(_idOf('A40156'), equals('invalid-guard'));
      expect(_idOf('40156D'), equals('invalid-guard'));
      expect(_idOf('A'), equals('invalid-guard'));
      expect(_idOf('40A56'), equals('invalid-guard'));
      // Fully guarded input passes through unchanged.
      expect(normalizeCodabar('A12B'), equals('A12B'));
    });

    test('non-guard options fail with invalid-guard', () {
      expect(_idOf('0', start: 'X'), equals('invalid-guard'));
      expect(_idOf('0', stop: '5'), equals('invalid-guard'));
      expect(_idOf('0', start: ''), equals('invalid-guard'));
      expect(_idOf('0', start: 'AB'), equals('invalid-guard'));
    });

    test('disagreeing options conflict on guarded input', () {
      expect(_idOf('A12B', start: 'C'), equals('guard-conflict'));
      expect(_idOf('A12B', stop: 'A'), equals('guard-conflict'));
      expect(_idOf('A12B', start: 'A', stop: 'B'), equals('ok'));
      // Conflict precedes the body scan.
      expect(_idOf('A*B', start: 'C'), equals('guard-conflict'));
      // ...and options precede it on unguarded input too.
      expect(_idOf('12*34', start: 'X'), equals('invalid-guard'));
    });

    test('folds ASCII lowercase in input and options', () {
      expect(normalizeCodabar('a12b'), equals('A12B'));
      expect(
        normalizeCodabar('40156', start: 'b', stop: 'd'),
        equals('B40156D'),
      );
      // \u{DF} never folds (spec over reference to_uppercase).
      expect(_idOf('\u{DF}'), equals('invalid-character'));
    });

    test('body alphabet: all sixteen encode', () {
      const body = '0123456789-\$:/.+';
      final encoded = encodeCodabar(body);
      expect(encoded, hasLength(body.length + 2));
      for (final entry in encoded) {
        expect(entry.pattern.length, greaterThanOrEqualTo(9));
      }
      // '*' and '@' are outside the body alphabet.
      expect(_idOf('12*34'), equals('invalid-character'));
      expect(_idOf('@'), equals('invalid-character'));
    });

    test('over-limit input fails before scanning', () {
      expect(_idOf('A' * 4097), equals('input-too-long'));
      expect(_idOf('@' * 4097), equals('input-too-long'));
      expect(normalizeCodabar('0' * 4096), hasLength(4098));
    });

    test('lone surrogates throw ArgumentError outside v1', () {
      final loneHigh = String.fromCharCodes([0xd800]);
      expect(() => normalizeCodabar('A$loneHigh'), throwsArgumentError);
      expect(() => normalizeCodabar('0', start: loneHigh), throwsArgumentError);
      // High surrogate followed by a non-low surrogate (either side).
      final highAscii = String.fromCharCodes([0xd800, 0x42]);
      expect(() => normalizeCodabar('A$highAscii'), throwsArgumentError);
      final highBmp = String.fromCharCodes([0xd800, 0xff10]);
      expect(() => normalizeCodabar(highBmp), throwsArgumentError);
      final loneLow = String.fromCharCodes([0xdc00]);
      expect(() => normalizeCodabar('${loneLow}AB'), throwsArgumentError);
    });

    test('valid astral pairs count once, then fail the alphabet', () {
      // U+1F600 is a well-formed pair: scalar counting succeeds (one
      // scalar), and the body scan rejects it as non-Codabar.
      expect(
        () => normalizeCodabar('\u{1F600}'),
        throwsA(
          isA<InvalidCodabarInputError>().having(
            (e) => e.errorId,
            'errorId',
            'invalid-character',
          ),
        ),
      );
    });

    test('run geometry: patterns, gaps, roles', () {
      final runs = expandCodabarRuns('0');
      // A0A: guards are 10 modules, '0' is 9, plus two gaps.
      final modules = runs.fold<int>(0, (sum, run) => sum + run.modules);
      expect(modules, equals(10 + 9 + 10 + 2));
      expect(runs.first.role, equals('start'));
      expect(runs.first.sourceLabel, equals('A'));
      expect(runs.last.role, equals('stop'));
      // One narrow gap between neighbors, never trailing.
      final gaps = runs.where((r) => r.role == 'inter-character-gap');
      expect(gaps, hasLength(2));
      for (var i = 1; i < runs.length; i++) {
        expect(runs[i].color, isNot(equals(runs[i - 1].color)));
      }
    });

    test('scene carries symbology, guards, and text metadata', () {
      final scene = layoutCodabar('40156', start: 'B', stop: 'D');

      expect(scene.metadata['symbology'], equals('codabar'));
      expect(scene.metadata['start'], equals('B'));
      expect(scene.metadata['stop'], equals('D'));
      expect(scene.metadata['humanReadableText'], equals('B40156D'));
      expect(
        scene.metadata['label'],
        equals('Codabar barcode for B40156D'),
      );
      expect(scene.background, equals('#ffffff'));
      expect(scene.instructions, isNotEmpty);
    });

    test('caller options merge with symbology defaults', () {
      final scene = layoutCodabar(
        '0',
        options: const Barcode1DSceneOptions(label: 'custom', barHeight: 60),
      );

      expect(scene.metadata['label'], equals('custom'));
      expect(scene.metadata['barHeightPx'], equals('60'));
      expect(scene.metadata['symbology'], equals('codabar'));
      expect(scene.metadata['start'], equals('A'));
    });

    test('text-enabled projection hits the lane error', () {
      expect(
        () => layoutCodabar(
          '0',
          options: const Barcode1DSceneOptions(
            includeHumanReadableText: true,
          ),
        ),
        throwsA(isA<Barcode1DV1Error>()),
      );
    });

    test('drawCodabar aliases layoutCodabar', () {
      final a = layoutCodabar('0', start: 'C', stop: 'C');
      final b = drawCodabar('0', start: 'C', stop: 'C');
      expect(b.metadata, equals(a.metadata));
      expect(b.instructions.length, equals(a.instructions.length));
    });

    // NOTE: the encoder's explicit null guard mirrors the sibling lanes
    // but is unreachable even via `dynamic` (the call-boundary downcast
    // throws first); it documents the contract and stays untested.

    test('encoded symbols support value semantics', () {
      const a = EncodedCodabarSymbol(
        ch: 'A',
        pattern: '1011001001',
        sourceIndex: 0,
        role: 'start',
      );
      const b = EncodedCodabarSymbol(
        ch: 'A',
        pattern: '1011001001',
        sourceIndex: 0,
        role: 'start',
      );
      const c = EncodedCodabarSymbol(
        ch: 'B',
        pattern: '1001001011',
        sourceIndex: 0,
        role: 'start',
      );
      expect(a, equals(b));
      expect(a.hashCode, equals(b.hashCode));
      expect(a, isNot(equals(c)));
      expect(a == 'not-a-symbol', isFalse);
      expect(a.toString(), contains('EncodedCodabarSymbol(A'));
    });

    test('error values and messages are payload-blind', () {
      final err = InvalidCodabarInputError(CodabarErrorCode.guardConflict);
      expect(err.errorId, equals('guard-conflict'));
      expect(err.message, equals(codabarMessageGuardConflict));
      expect(codabarMessageFor(null), equals(codabarMessageInvalidCharacter));
    });
  });
}
