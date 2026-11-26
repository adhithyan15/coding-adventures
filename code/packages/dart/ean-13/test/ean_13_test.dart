// Unit tests for the EAN-13 lane: the reference test mirrored
// case-for-case, then the full validation matrix (precedence, caps,
// surrogate rejection, parity/digit tables, run geometry, scene
// metadata).

import 'package:coding_adventures_barcode_layout_1d/barcode_layout_1d.dart';
import 'package:coding_adventures_ean_13/ean_13.dart';
import 'package:test/test.dart';

void _expectErrorId(String input, String id) {
  expect(
    () => normalizeEan13(input),
    throwsA(
      isA<InvalidEan13InputError>().having((e) => e.errorId, 'errorId', id),
    ),
    reason: 'input ${input.length} chars',
  );
}

void main() {
  group('ean-13', () {
    // Reference test: computes_check_digit_and_layout.
    test('computes check digit, parity, and scene', () {
      expect(computeEan13CheckDigit('400638133393'), equals('1'));
      expect(leftParityPattern('4006381333931'), equals('LGLLGG'));

      final scene = layoutEan13(
        '400638133393',
        const Barcode1DSceneOptions(),
      );
      expect(scene.metadata['symbology'], equals('ean-13'));
    });

    test('accepts a valid 13-digit payload unchanged', () {
      expect(normalizeEan13('4006381333931'), equals('4006381333931'));
    });

    test('wrong check digit reports invalid-check-digit', () {
      _expectErrorId('4006381333932', 'invalid-check-digit');
    });

    test('validation precedence: too-long, length, character, check', () {
      _expectErrorId('A' * 4097, 'input-too-long');
      _expectErrorId('', 'invalid-length');
      _expectErrorId('123', 'invalid-length');
      // Length before character: one bad char at a bad length.
      _expectErrorId('A', 'invalid-length');
      _expectErrorId('40063813339AB', 'invalid-character');
      // Fullwidth digits are never coerced.
      _expectErrorId('\u{FF10}40063813339', 'invalid-character');
      // Astral characters count once toward length, then fail alphabet.
      _expectErrorId('40063813339\u{1F600}', 'invalid-character');
      _expectErrorId('4006381333932', 'invalid-check-digit');
    });

    test('lone surrogates throw ArgumentError outside v1', () {
      final loneHigh = String.fromCharCodes([0xd800]);
      final loneLow = String.fromCharCodes([0xdc00]);
      expect(
        () => normalizeEan13('40063813339$loneHigh'),
        throwsArgumentError,
      );
      expect(
        () => normalizeEan13('${loneLow}006381333931'),
        throwsArgumentError,
      );
    });

    test('every leading digit selects its parity mix', () {
      for (var leading = 0; leading < 10; leading++) {
        final payload = '$leading${'0' * 11}';
        expect(
          leftParityPattern(payload),
          equals(ean13LeftParityPatterns[leading]),
          reason: 'leading $leading',
        );
      }
    });

    test('encodes twelve digits with L/G/R sides', () {
      final encoded = encodeEan13('400638133393');

      expect(encoded, hasLength(12));
      // Leading digit 4 -> LGLLGG; first encoded digit is index 1.
      expect(encoded[0].digit, equals('0'));
      expect(encoded[0].encoding, equals('L'));
      expect(encoded[0].sourceIndex, equals(1));
      expect(encoded[0].role, equals('data'));
      expect(encoded[1].encoding, equals('G'));
      expect(
        encoded[1].pattern,
        equals(ean13GPatterns[0]),
      );
      // Right half is all R; the last entry is the check digit.
      for (var i = 6; i < 12; i++) {
        expect(encoded[i].encoding, equals('R'));
      }
      expect(encoded[11].digit, equals('1'));
      expect(encoded[11].role, equals('check'));
      expect(encoded[11].sourceIndex, equals(12));
    });

    test('G table covers all ten digits', () {
      // Leading 7 -> LGLGLG: G at left positions 1, 3, 5 (indexes 2,4,6).
      final encoded = encodeEan13('700000000000');
      final gEntries = encoded.sublist(0, 6).where((e) => e.encoding == 'G');
      expect(gEntries, hasLength(3));
      // Leading 0 -> LLLLLL exercises L; leading 9 mixes the rest.
      final mixed = encodeEan13('901234567890');
      final patterns = mixed.map((e) => e.pattern).toSet();
      expect(patterns.length, greaterThan(6));
    });

    test('runs span exactly 95 modules with guards', () {
      final runs = expandEan13Runs('400638133393');
      final modules = runs.fold<int>(0, (sum, run) => sum + run.modules);

      expect(modules, equals(95));
      expect(runs.first.sourceLabel, equals('start'));
      expect(runs.first.role, equals('guard'));
      expect(runs.last.sourceLabel, equals('end'));
      for (var i = 1; i < runs.length; i++) {
        expect(runs[i].color, isNot(equals(runs[i - 1].color)));
      }
    });

    test('scene carries parity and geometry metadata', () {
      final scene = layoutEan13('400638133393');

      expect(scene.metadata['symbology'], equals('ean-13'));
      expect(scene.metadata['humanReadableText'], equals('4006381333931'));
      expect(
        scene.metadata['label'],
        equals('EAN-13 barcode for 4006381333931'),
      );
      expect(scene.metadata['leadingDigit'], equals('4'));
      expect(scene.metadata['leftParity'], equals('LGLLGG'));
      expect(scene.metadata['contentModules'], equals('95'));
      expect(scene.metadata['totalModules'], equals('115'));
      expect(scene.metadata['symbolCount'], equals('15'));
      expect(scene.background, equals('#ffffff'));
      expect(scene.instructions, isNotEmpty);
    });

    test('caller options merge with symbology defaults', () {
      final scene = layoutEan13(
        '400638133393',
        const Barcode1DSceneOptions(label: 'custom', barHeight: 60),
      );

      expect(scene.metadata['label'], equals('custom'));
      expect(scene.metadata['barHeightPx'], equals('60'));
      expect(scene.metadata['symbology'], equals('ean-13'));
      expect(scene.metadata['leftParity'], equals('LGLLGG'));
    });

    test('text-enabled projection hits the lane error', () {
      expect(
        () => layoutEan13(
          '400638133393',
          const Barcode1DSceneOptions(includeHumanReadableText: true),
        ),
        throwsA(isA<Barcode1DV1Error>()),
      );
    });

    test('drawEan13 aliases layoutEan13', () {
      final a = layoutEan13('400638133393');
      final b = drawEan13('400638133393');
      expect(b.metadata, equals(a.metadata));
      expect(b.instructions.length, equals(a.instructions.length));
    });

    // NOTE: the encoder's explicit null guard mirrors the ITF/UPC-A
    // lanes but is unreachable even via `dynamic` (the call-boundary
    // downcast throws first); it documents the contract and stays
    // untested, exactly like theirs.

    test('encoded digits support value semantics', () {
      const a = EncodedDigit(
        digit: '5',
        encoding: 'G',
        pattern: '0111001',
        sourceIndex: 2,
        role: 'data',
      );
      const b = EncodedDigit(
        digit: '5',
        encoding: 'G',
        pattern: '0111001',
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
      expect(a.toString(), contains('EncodedDigit(5, G'));
    });

    test('error values and messages are payload-blind', () {
      final err = InvalidEan13InputError(Ean13ErrorCode.invalidCheckDigit);
      expect(err.errorId, equals('invalid-check-digit'));
      expect(err.message, equals(ean13MessageInvalidCheckDigit));
      expect(err.toString(), isNot(contains('4006381333932')));
    });
  });
}
