// Unit tests for the Code 39 lane: the portable reference tests
// mirrored case-for-case, then folding, ordering, alphabet, geometry,
// and scene coverage.

import 'package:coding_adventures_barcode_layout_1d/barcode_layout_1d.dart';
import 'package:coding_adventures_code_39/code_39.dart';
import 'package:test/test.dart';

void main() {
  group('code39', () {
    // Reference test 1: version_and_encode.
    test('version and single-char encode', () {
      expect(code39Version, equals('0.1.0'));
      final encoded = encodeCode39Char('A');
      expect(encoded.pattern, equals('WNNNNWNNW'));
      expect(encoded.isStartStop, isFalse);
    });

    // Reference test 2: expands_runs_and_builds_paint_scene.
    test('expands runs and builds a paint scene', () {
      final runs = expandCode39Runs('A');
      expect(runs, hasLength(29));
      expect(runs[0].role, equals('start'));
      expect(runs[10].role, equals('data'));

      // Null options apply the symbology label default (an explicit
      // default options object carries the lane's own '1D barcode').
      final scene = layoutCode39('A');
      expect(scene.metadata['symbology'], equals('code39'));
      expect(scene.metadata['label'], equals('Code 39 barcode for A'));
    });

    // Reference test 3: default_render_config_is_paint_friendly.
    test('default scene options are paint-friendly', () {
      const options = Barcode1DSceneOptions();
      expect(options.includeHumanReadableText, isFalse);
      expect(options.moduleWidth, equals(4));
      expect(options.barHeight, equals(120));
    });

    // Reference tests 4-6 (glyph rendering) are platform-conditional in
    // Rust (render on Windows/macOS, font error on Linux). Dart has no
    // native text backend on any platform, so text-enabled projection
    // always reaches the lane's specified error — the portable branch.
    test('text-enabled projection hits the lane error', () {
      expect(
        () => layoutCode39(
          'OK',
          const Barcode1DSceneOptions(includeHumanReadableText: true),
        ),
        throwsA(isA<Barcode1DV1Error>()),
      );
    });

    test('empty input is valid', () {
      expect(normalizeCode39(''), equals(''));
      final encoded = encodeCode39('');
      expect(encoded.map((e) => e.ch), equals(['*', '*']));
      final runs = expandCode39Runs('');
      // Two delimiters x 9 elements + one inter-character gap.
      expect(runs, hasLength(19));
      final scene = layoutCode39('');
      expect(scene.metadata['label'], equals('Code 39 barcode'));
    });

    test('folds ASCII lowercase only', () {
      expect(normalizeCode39('a'), equals('A'));
      expect(normalizeCode39('hello-1'), equals('HELLO-1'));
      // \u{DF} must NOT fold (spec over reference to_uppercase).
      expect(
        () => normalizeCode39('\u{DF}'),
        throwsA(
          isA<InvalidCode39InputError>().having(
            (e) => e.errorId,
            'errorId',
            'invalid-character',
          ),
        ),
      );
    });

    test('scan order: reserved and invalid at their positions', () {
      String idOf(String input) {
        try {
          normalizeCode39(input);
          return 'ok';
        } on InvalidCode39InputError catch (e) {
          return e.errorId;
        }
      }

      expect(idOf('*'), equals('reserved-character'));
      expect(idOf('@*'), equals('invalid-character'));
      expect(idOf('*@'), equals('reserved-character'));
      expect(idOf('@'), equals('invalid-character'));
      expect(idOf('AB*CD'), equals('reserved-character'));
      expect(idOf('AB@CD'), equals('invalid-character'));
    });

    test('over-limit input fails before scanning', () {
      expect(
        () => normalizeCode39('A' * 4097),
        throwsA(
          isA<InvalidCode39InputError>().having(
            (e) => e.errorId,
            'errorId',
            'input-too-long',
          ),
        ),
      );
      // Exactly at the cap still normalizes.
      expect(normalizeCode39('A' * 4096), hasLength(4096));
    });

    test('lone surrogates throw ArgumentError outside v1', () {
      final loneHigh = String.fromCharCodes([0xd800]);
      expect(() => normalizeCode39('A$loneHigh'), throwsArgumentError);
      // High surrogate followed by a non-low surrogate (either side).
      final highAscii = String.fromCharCodes([0xd800, 0x42]);
      expect(() => normalizeCode39('A$highAscii'), throwsArgumentError);
      final highBmp = String.fromCharCodes([0xd800, 0xff10]);
      expect(() => normalizeCode39(highBmp), throwsArgumentError);
      final loneLow = String.fromCharCodes([0xdc00]);
      expect(() => normalizeCode39('${loneLow}AB'), throwsArgumentError);
    });

    test('valid astral pairs count once, then fail the alphabet', () {
      // U+1F600 is a well-formed pair: scalar counting succeeds (one
      // scalar), and the alphabet scan rejects it as non-Code-39.
      expect(
        () => normalizeCode39('\u{1F600}'),
        throwsA(
          isA<InvalidCode39InputError>().having(
            (e) => e.errorId,
            'errorId',
            'invalid-character',
          ),
        ),
      );
    });

    test('full alphabet encodes with nine-element patterns', () {
      const alphabet = '0123456789ABCDEFGHIJKLMNOPQRSTUVWXYZ-. \$/%+';
      for (var i = 0; i < alphabet.length; i++) {
        final ch = alphabet[i];
        final entry = encodeCode39Char(ch);
        expect(entry.pattern, hasLength(9), reason: ch);
        expect(entry.isStartStop, isFalse, reason: ch);
        for (var j = 0; j < 9; j++) {
          expect(
            entry.pattern[j] == 'W' || entry.pattern[j] == 'N',
            isTrue,
            reason: '$ch[$j]',
          );
        }
      }
      // The delimiter encodes too, flagged as start/stop.
      final star = encodeCode39Char('*');
      expect(star.isStartStop, isTrue);
      expect(star.pattern, hasLength(9));
    });

    test('single-char encode rejects unknown characters', () {
      expect(
        () => encodeCode39Char('@'),
        throwsA(
          isA<InvalidCode39InputError>().having(
            (e) => e.errorId,
            'errorId',
            'invalid-character',
          ),
        ),
      );
      // Unfolded lowercase is not in the table (folding is normalize's).
      expect(
        () => encodeCode39Char('a'),
        throwsA(isA<InvalidCode39InputError>()),
      );
    });

    test('run geometry: widths, gaps, roles, indexes', () {
      final runs = expandCode39Runs('AB');
      // 4 chars x 9 elements + 3 gaps = 39 runs.
      expect(runs, hasLength(39));
      // First char (start *) owns runs 0-8 + gap 9.
      expect(runs[0].role, equals('start'));
      expect(runs[9].role, equals('inter-character-gap'));
      expect(runs[9].color, equals('space'));
      expect(runs[9].modules, equals(1));
      // 'A' = WNNNNWNNW: widths 3,1,1,1,1,3,1,1,3 starting at run 10.
      const widths = [3, 1, 1, 1, 1, 3, 1, 1, 3];
      for (var i = 0; i < 9; i++) {
        expect(runs[10 + i].modules, equals(widths[i]), reason: 'run $i');
        expect(runs[10 + i].sourceLabel, equals('A'));
        expect(runs[10 + i].sourceIndex, equals(1));
        expect(runs[10 + i].role, equals('data'));
      }
      // Alternation across the whole stream.
      for (var i = 1; i < runs.length; i++) {
        expect(runs[i].color, isNot(equals(runs[i - 1].color)));
      }
      // Stop delimiter owns the tail with the stop role.
      expect(runs.last.role, equals('stop'));
      expect(runs.last.sourceIndex, equals(3));
    });

    test('scene carries symbology and text metadata', () {
      final scene = layoutCode39('AB-9');

      expect(scene.metadata['symbology'], equals('code39'));
      expect(scene.metadata['encodedText'], equals('AB-9'));
      expect(scene.metadata['humanReadableText'], equals('AB-9'));
      expect(
        scene.metadata['label'],
        equals('Code 39 barcode for AB-9'),
      );
      expect(scene.background, equals('#ffffff'));
      expect(scene.instructions, isNotEmpty);
    });

    test('caller options merge with symbology defaults', () {
      final scene = layoutCode39(
        'A',
        const Barcode1DSceneOptions(label: 'custom', barHeight: 60),
      );

      expect(scene.metadata['label'], equals('custom'));
      expect(scene.metadata['barHeightPx'], equals('60'));
      expect(scene.metadata['symbology'], equals('code39'));
      expect(scene.metadata['encodedText'], equals('A'));
    });

    test('drawCode39 aliases layoutCode39', () {
      final a = layoutCode39('A');
      final b = drawCode39('A');
      expect(b.metadata, equals(a.metadata));
      expect(b.instructions.length, equals(a.instructions.length));
    });

    // NOTE: the encoder's explicit null guard mirrors the sibling lanes
    // but is unreachable even via `dynamic` (the call-boundary downcast
    // throws first); it documents the contract and stays untested.

    test('encoded characters support value semantics', () {
      const a =
          EncodedCharacter(ch: 'A', isStartStop: false, pattern: 'WNNNNWNNW');
      const b =
          EncodedCharacter(ch: 'A', isStartStop: false, pattern: 'WNNNNWNNW');
      const c =
          EncodedCharacter(ch: '*', isStartStop: true, pattern: 'NWNNWNWNN');
      expect(a, equals(b));
      expect(a.hashCode, equals(b.hashCode));
      expect(a, isNot(equals(c)));
      expect(a == 'not-a-char', isFalse);
      expect(a.toString(), contains('EncodedCharacter(A'));
    });

    test('error values and messages are payload-blind', () {
      final err = InvalidCode39InputError(Code39ErrorCode.reservedCharacter);
      expect(err.errorId, equals('reserved-character'));
      expect(err.message, equals(code39MessageReservedCharacter));
      expect(code39MessageFor(null), equals(code39MessageInvalidCharacter));
    });
  });
}
