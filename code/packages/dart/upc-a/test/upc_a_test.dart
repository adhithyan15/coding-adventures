// Unit tests for the UPC-A lane: the 2 reference tests mirrored
// case-for-case, then the full validation matrix (precedence, caps,
// surrogate rejection, digit tables, run geometry, scene metadata).

import 'package:coding_adventures_barcode_layout_1d/barcode_layout_1d.dart';
import 'package:coding_adventures_upc_a/upc_a.dart';
import 'package:test/test.dart';

void _expectErrorId(String input, String id) {
  expect(
    () => normalizeUpcA(input),
    throwsA(
      isA<InvalidUpcAInputError>().having((e) => e.errorId, 'errorId', id),
    ),
    reason: 'input ${input.length} chars',
  );
}

void main() {
  group('upc-a', () {
    // Reference test 1: computes_upc_check_digit.
    test('computes the check digit', () {
      expect(computeUpcACheckDigit('03600029145'), equals('2'));
    });

    // Reference test 2: normalizes_and_builds_scene.
    test('normalizes and builds a scene', () {
      expect(normalizeUpcA('03600029145'), equals('036000291452'));

      final scene = layoutUpcA('03600029145', const Barcode1DSceneOptions());
      expect(scene.metadata['symbology'], equals('upc-a'));
    });

    test('accepts a valid 12-digit payload unchanged', () {
      expect(normalizeUpcA('036000291452'), equals('036000291452'));
    });

    test('wrong check digit reports invalid-check-digit', () {
      _expectErrorId('036000291453', 'invalid-check-digit');
    });

    test('validation precedence: too-long, length, character, check', () {
      _expectErrorId('A' * 4097, 'input-too-long');
      _expectErrorId('', 'invalid-length');
      _expectErrorId('123', 'invalid-length');
      // Length before character: one bad char at a bad length.
      _expectErrorId('A', 'invalid-length');
      _expectErrorId('0360002914A', 'invalid-character');
      // Fullwidth digits are never coerced.
      _expectErrorId('\u{FF10}3600029145', 'invalid-character');
      // Astral characters count once toward length, then fail alphabet.
      _expectErrorId('0360002914\u{1F600}', 'invalid-character');
      _expectErrorId('036000291453', 'invalid-check-digit');
    });

    test('lone surrogates throw ArgumentError outside v1', () {
      final loneHigh = String.fromCharCodes([0xd800]);
      final loneLow = String.fromCharCodes([0xdc00]);
      expect(() => normalizeUpcA('0360002914$loneHigh'), throwsArgumentError);
      expect(() => normalizeUpcA('${loneLow}3600029145'), throwsArgumentError);
    });

    test('check digit rejects short and dirty payloads', () {
      expect(
        () => computeUpcACheckDigit('123'),
        throwsA(
          isA<InvalidUpcAInputError>().having(
            (e) => e.errorId,
            'errorId',
            'invalid-length',
          ),
        ),
      );
      expect(
        () => computeUpcACheckDigit('0360002914A'),
        throwsA(
          isA<InvalidUpcAInputError>().having(
            (e) => e.errorId,
            'errorId',
            'invalid-character',
          ),
        ),
      );
    });

    test('encodes digits with L/R sides and check role', () {
      final encoded = encodeUpcA('03600029145');

      expect(encoded, hasLength(12));
      expect(encoded[0].digit, equals('0'));
      expect(encoded[0].encoding, equals('L'));
      expect(encoded[0].pattern, equals('0001101'));
      expect(encoded[0].sourceIndex, equals(0));
      expect(encoded[0].role, equals('data'));
      expect(encoded[5].encoding, equals('L'));
      expect(encoded[6].encoding, equals('R'));
      expect(encoded[11].digit, equals('2'));
      expect(encoded[11].encoding, equals('R'));
      expect(encoded[11].pattern, equals('1101100'));
      expect(encoded[11].role, equals('check'));
    });

    test('every digit maps through both tables', () {
      // Two payloads cover all ten L patterns in left-half positions.
      final encoded = encodeUpcA('01234567890');
      for (var d = 0; d < 6; d++) {
        expect(
          encoded[d].pattern,
          equals(upcALeftPatterns[d]),
          reason: 'L$d',
        );
      }
      final high = encodeUpcA('67890123450');
      for (var d = 6; d < 10; d++) {
        expect(
          high[d - 6].pattern,
          equals(upcALeftPatterns[d]),
          reason: 'L$d',
        );
      }
      // Right-half R patterns: digits 6..10 of an 11-digit payload
      // plus its computed check digit at 11.
      final right = encodeUpcA('00000012345');
      expect(right[11].digit, equals('7'));
      const want = ['1', '2', '3', '4', '5', '7'];
      for (var i = 0; i < 6; i++) {
        final value = int.parse(want[i]);
        expect(right[6 + i].pattern, equals(upcARightPatterns[value]));
      }
    });

    test('runs span exactly 95 modules with guards', () {
      final runs = expandUpcARuns('03600029145');
      final modules = runs.fold<int>(0, (sum, run) => sum + run.modules);

      expect(modules, equals(95));
      // Guards: start(101) + center(01010) + end(101).
      expect(runs.first.sourceLabel, equals('start'));
      expect(runs.first.role, equals('guard'));
      expect(runs.last.sourceLabel, equals('end'));
      // Strict bar/space alternation across the whole stream.
      for (var i = 1; i < runs.length; i++) {
        expect(runs[i].color, isNot(equals(runs[i - 1].color)));
      }
    });

    test('scene carries symbology, text, and geometry metadata', () {
      final scene = layoutUpcA('03600029145');

      expect(scene.metadata['symbology'], equals('upc-a'));
      expect(scene.metadata['humanReadableText'], equals('036000291452'));
      expect(
        scene.metadata['label'],
        equals('UPC-A barcode for 036000291452'),
      );
      expect(scene.metadata['contentModules'], equals('95'));
      expect(scene.metadata['totalModules'], equals('115'));
      expect(scene.metadata['symbolCount'], equals('15'));
      expect(scene.background, equals('#ffffff'));
      expect(scene.instructions, isNotEmpty);
    });

    test('caller options merge with symbology defaults', () {
      final scene = layoutUpcA(
        '03600029145',
        const Barcode1DSceneOptions(label: 'custom', barHeight: 60),
      );

      expect(scene.metadata['label'], equals('custom'));
      expect(scene.metadata['barHeightPx'], equals('60'));
      // Symbology keys survive the merge.
      expect(scene.metadata['symbology'], equals('upc-a'));
    });

    test('text-enabled projection hits the lane error', () {
      expect(
        () => layoutUpcA(
          '03600029145',
          const Barcode1DSceneOptions(includeHumanReadableText: true),
        ),
        throwsA(isA<Barcode1DV1Error>()),
      );
    });

    test('drawUpcA aliases layoutUpcA', () {
      final a = layoutUpcA('03600029145');
      final b = drawUpcA('03600029145');
      expect(b.metadata, equals(a.metadata));
      expect(b.instructions.length, equals(a.instructions.length));
    });

    // NOTE: the encoder's explicit null guard mirrors the ITF lane but
    // is unreachable even via `dynamic` (the call-boundary downcast
    // throws first); it documents the contract and stays untested,
    // exactly like ITF's.

    test('encoded digits support value semantics', () {
      const a = EncodedDigit(
        digit: '5',
        encoding: 'L',
        pattern: '0110001',
        sourceIndex: 2,
        role: 'data',
      );
      const b = EncodedDigit(
        digit: '5',
        encoding: 'L',
        pattern: '0110001',
        sourceIndex: 2,
        role: 'data',
      );
      const c = EncodedDigit(
        digit: '5',
        encoding: 'R',
        pattern: '1001110',
        sourceIndex: 8,
        role: 'data',
      );
      expect(a, equals(b));
      expect(a.hashCode, equals(b.hashCode));
      expect(a, isNot(equals(c)));
      expect(a == 'not-a-digit', isFalse);
      expect(a.toString(), contains('EncodedDigit(5, L'));
    });

    test('error values and messages are payload-blind', () {
      final err = InvalidUpcAInputError(UpcAErrorCode.invalidCheckDigit);
      expect(err.errorId, equals('invalid-check-digit'));
      expect(err.message, equals(upcAMessageInvalidCheckDigit));
      expect(err.toString(), isNot(contains('036000291453')));
    });
  });
}
