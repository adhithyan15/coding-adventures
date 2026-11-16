import 'dart:math';

import 'package:coding_adventures_barcode_layout_1d/barcode_layout_1d.dart';
import 'package:coding_adventures_itf/itf.dart';
import 'package:coding_adventures_itf/src/constants.dart';
import 'package:coding_adventures_itf/src/messages.dart';
import 'package:test/test.dart';

import 'itf_conformance_test.dart' as conformance;

void main() {
  conformance.main();
  group('ITF Package API & Security', () {
    test('version exists', () {
      expect(itfVersion, equals('0.1.0'));
    });

    group('5-Class Barcode/ITF Security Showcase', () {
      test('Class 1: Strict Precedence Enforcement', () {
        // Precedence 1: Lone high surrogate throws ArgumentError
        expect(
          () => normalizeItf('\uD800'),
          throwsA(
            isA<ArgumentError>().having(
              (e) => e.message,
              'message',
              equals(itfMessageInvalidScalars),
            ),
          ),
        );

        // Precedence 1: Lone low surrogate throws ArgumentError
        expect(
          () => normalizeItf('\uDC00'),
          throwsA(
            isA<ArgumentError>().having(
              (e) => e.message,
              'message',
              equals(itfMessageInvalidScalars),
            ),
          ),
        );

        // Precedence 1: Inverted surrogate pair throws ArgumentError
        expect(
          () => normalizeItf('\uDC00\uD800'),
          throwsA(isA<ArgumentError>()),
        );

        // Precedence 1: Valid surrogate pair counts as exactly 1 Unicode scalar
        // 4095 '0's + 1 astral scalar (4097 UTF-16 code units = 4096 scalars, even length).
        // Must pass surrogate validation and scalar limit, then fail with invalid-character (NOT input-too-long).
        final astralEvenValidCapacity = ('0' * 4095) + '\u{1F600}';
        expect(
          () => normalizeItf(astralEvenValidCapacity),
          throwsA(
            isA<InvalidItfInputError>().having(
              (e) => e.errorId,
              'errorId',
              equals(itfErrorInvalidCharacter),
            ),
          ),
        );

        // 4096 '0's + 1 astral scalar (4098 UTF-16 code units = 4097 scalars).
        // Must fail with input-too-long.
        final astralOverLimit = ('0' * 4096) + '\u{1F600}';
        expect(
          () => normalizeItf(astralOverLimit),
          throwsA(
            isA<InvalidItfInputError>().having(
              (e) => e.errorId,
              'errorId',
              equals(itfErrorInputTooLong),
            ),
          ),
        );

        // Precedence 2: Over 4096 scalar characters must fail with input-too-long
        // even if length is odd and contains non-digits (e.g. 4097 'A's)
        final overLimitOddNonDigits = 'A' * 4097;
        expect(
          () => normalizeItf(overLimitOddNonDigits),
          throwsA(
            isA<InvalidItfInputError>()
                .having(
                  (e) => e.errorId,
                  'errorId',
                  equals(itfErrorInputTooLong),
                )
                .having(
                  (e) => e.errorCode,
                  'errorCode',
                  equals(ItfErrorCode.inputTooLong),
                ),
          ),
        );

        // Precedence 3: Length check (0 or odd) occurs before character scan.
        // Odd length with invalid character ('A') must fail with invalid-length, NOT invalid-character!
        expect(
          () => normalizeItf('A'),
          throwsA(
            isA<InvalidItfInputError>().having(
              (e) => e.errorId,
              'errorId',
              equals(itfErrorInvalidLength),
            ),
          ),
        );

        expect(
          () => normalizeItf('123'),
          throwsA(
            isA<InvalidItfInputError>().having(
              (e) => e.errorId,
              'errorId',
              equals(itfErrorInvalidLength),
            ),
          ),
        );

        // Empty string fails with invalid-length
        expect(
          () => normalizeItf(''),
          throwsA(
            isA<InvalidItfInputError>().having(
              (e) => e.errorId,
              'errorId',
              equals(itfErrorInvalidLength),
            ),
          ),
        );

        // Precedence 4: Valid length with non-digit character fails with invalid-character
        expect(
          () => normalizeItf('12A4'),
          throwsA(
            isA<InvalidItfInputError>()
                .having(
                  (e) => e.errorId,
                  'errorId',
                  equals(itfErrorInvalidCharacter),
                )
                .having(
                  (e) => e.errorCode,
                  'errorCode',
                  equals(ItfErrorCode.invalidCharacter),
                ),
          ),
        );
      });

      test('Class 2: Strict ASCII Digits Only (Reject Non-ASCII Digits)', () {
        // Fullwidth digits ('１２') must be rejected as invalid-character
        expect(
          () => normalizeItf('\uFF11\uFF12'),
          throwsA(
            isA<InvalidItfInputError>().having(
              (e) => e.errorId,
              'errorId',
              equals(itfErrorInvalidCharacter),
            ),
          ),
        );

        // Arabic-Indic digits ('١٢') must be rejected as invalid-character
        expect(
          () => normalizeItf('\u0661\u0662'),
          throwsA(
            isA<InvalidItfInputError>().having(
              (e) => e.errorId,
              'errorId',
              equals(itfErrorInvalidCharacter),
            ),
          ),
        );

        // Punctuation and spaces
        expect(
          () => normalizeItf('1 23'),
          throwsA(isA<InvalidItfInputError>()),
        );
        expect(
          () => normalizeItf('1\n23'),
          throwsA(isA<InvalidItfInputError>()),
        );
        expect(
          () => normalizeItf('12-4'),
          throwsA(isA<InvalidItfInputError>()),
        );
      });

      test('Class 3: High Capacity & Boundary Limit Verification', () {
        // Exactly 4096 ASCII digits must be accepted
        final limitInput = '0' * 4096;
        expect(normalizeItf(limitInput), equals(limitInput));

        // 4097 digits must be rejected with input-too-long
        final over4097 = '0' * 4097;
        expect(
          () => normalizeItf(over4097),
          throwsA(
            isA<InvalidItfInputError>().having(
              (e) => e.errorId,
              'errorId',
              equals(itfErrorInputTooLong),
            ),
          ),
        );

        // 4098 digits (even length) must also be rejected with input-too-long
        final over4098 = '0' * 4098;
        expect(
          () => normalizeItf(over4098),
          throwsA(
            isA<InvalidItfInputError>().having(
              (e) => e.errorId,
              'errorId',
              equals(itfErrorInputTooLong),
            ),
          ),
        );
      });

      test('Class 4: Structural & Layout Invariants', () {
        const data = '1234';
        final pairs = encodeItf(data);
        expect(pairs.length, equals(2));
        expect(pairs[0].pair, equals('12'));
        expect(pairs[1].pair, equals('34'));

        final runs = expandItfRuns(data);
        expect(runs, isNotEmpty);

        // Start pattern runs
        expect(runs.first.role, equals(itfRoleStart));
        expect(runs.first.sourceLabel, equals(itfLabelStart));

        // Stop pattern runs
        expect(runs.last.role, equals(itfRoleStop));
        expect(runs.last.sourceLabel, equals(itfLabelStop));

        // Layout scene
        final scene = drawItf(data);
        expect(
          scene.metadata[itfMetadataKeySymbology],
          equals(itfMetadataValueSymbology),
        );
        expect(scene.metadata[itfMetadataKeyPairCount], equals('2'));
        expect(scene.metadata['leftQuietZoneModules'], equals('10'));
        expect(scene.metadata['rightQuietZoneModules'], equals('10'));
        expect(scene.width, greaterThan(0));
        expect(scene.height, equals(120));
      });

      test('Class 5: Payload-Blind Security & Zero Leak', () {
        const secretSensitiveData = '9876543210';
        try {
          normalizeItf('${secretSensitiveData}X');
          fail('Expected InvalidItfInputError');
        } on InvalidItfInputError catch (ex) {
          expect(ex.message, isNot(contains(secretSensitiveData)));
          expect(ex.message, isNot(contains('X')));
        }

        // Default scene label must NOT interpolate caller payload
        final scene = drawItf('1234');
        expect(scene.metadata['label'], equals(itfDefaultSceneLabel));
        expect(scene.metadata['label'], isNot(contains('1234')));
      });
    });

    test('Bijective Decode Oracle for all 100 Digit Pairs', () {
      final patterns = itfDigitPatterns;

      for (var d1 = 0; d1 <= 9; d1++) {
        for (var d2 = 0; d2 <= 9; d2++) {
          final pairStr = '$d1$d2';
          final pairs = encodeItf(pairStr);
          expect(pairs.length, equals(1));
          final encoded = pairs.first;
          expect(encoded.pair, equals(pairStr));

          final bin = encoded.binaryPattern;
          final recoveredBarPattern = StringBuffer();
          final recoveredSpacePattern = StringBuffer();

          var cursor = 0;
          for (var elem = 0; elem < 5; elem++) {
            var barLen = 0;
            while (cursor < bin.length && bin[cursor] == '1') {
              barLen++;
              cursor++;
            }
            expect(barLen == 1 || barLen == 3, isTrue);
            recoveredBarPattern.write(barLen == 3 ? '1' : '0');

            var spaceLen = 0;
            while (cursor < bin.length && bin[cursor] == '0') {
              spaceLen++;
              cursor++;
            }
            expect(spaceLen == 1 || spaceLen == 3, isTrue);
            recoveredSpacePattern.write(spaceLen == 3 ? '1' : '0');
          }
          expect(cursor, equals(bin.length));

          expect(recoveredBarPattern.toString(), equals(patterns[d1]));
          expect(recoveredSpacePattern.toString(), equals(patterns[d2]));

          final decodedD1 = patterns.indexOf(recoveredBarPattern.toString());
          final decodedD2 = patterns.indexOf(recoveredSpacePattern.toString());
          expect(decodedD1, equals(d1));
          expect(decodedD2, equals(d2));
        }
      }
    });

    test('10,000-Iteration PRNG Mutation Fuzz Loop', () {
      final rng = Random(0x17FD00D);

      for (var i = 0; i < 10000; i++) {
        final testType = rng.nextInt(5);
        switch (testType) {
          case 0:
            // Valid random even-length numeric string (length 2 to 40)
            final pairCount = 1 + rng.nextInt(20);
            final sb = StringBuffer();
            for (var p = 0; p < pairCount * 2; p++) {
              sb.write(rng.nextInt(10));
            }
            final valid = sb.toString();
            final normalized = normalizeItf(valid);
            expect(normalized, equals(valid));
            final encoded = encodeItf(valid);
            expect(encoded.length, equals(pairCount));
            final runs = expandItfRuns(valid);
            expect(runs, isNotEmpty);
            final scene = drawItf(valid);
            expect(scene.width, greaterThan(0));
            break;
          case 1:
            // Random odd-length numeric string (must fail with invalid-length)
            final length = 1 + 2 * rng.nextInt(20);
            final sb = StringBuffer();
            for (var p = 0; p < length; p++) {
              sb.write(rng.nextInt(10));
            }
            expect(
              () => normalizeItf(sb.toString()),
              throwsA(
                isA<InvalidItfInputError>().having(
                  (e) => e.errorId,
                  'errorId',
                  equals(itfErrorInvalidLength),
                ),
              ),
            );
            break;
          case 2:
            // Even-length string with non-digit character (must fail with invalid-character)
            final pairCount = 1 + rng.nextInt(10);
            final chars = List<int>.generate(
              pairCount * 2,
              (_) => 0x30 + rng.nextInt(10),
            );
            final corruptIdx = rng.nextInt(chars.length);
            chars[corruptIdx] = rng.nextBool()
                ? 0x41 + rng.nextInt(26) // 'A'-'Z'
                : rng.nextInt(0x30); // Below '0'
            if (chars[corruptIdx] >= 0x30 && chars[corruptIdx] <= 0x39) {
              chars[corruptIdx] = 0x58; // 'X'
            }
            final corrupted = String.fromCharCodes(chars);
            expect(
              () => normalizeItf(corrupted),
              throwsA(
                isA<InvalidItfInputError>().having(
                  (e) => e.errorId,
                  'errorId',
                  equals(itfErrorInvalidCharacter),
                ),
              ),
            );
            break;
          case 3:
            // Over-limit string (> 4096)
            final extra = 1 + rng.nextInt(20);
            final over = '0' * (4096 + extra);
            expect(
              () => normalizeItf(over),
              throwsA(
                isA<InvalidItfInputError>().having(
                  (e) => e.errorId,
                  'errorId',
                  equals(itfErrorInputTooLong),
                ),
              ),
            );
            break;
          case 4:
            // Lone surrogate corruption
            const base = '12';
            final surrogate = rng.nextBool() ? '\uD800' : '\uDC00';
            final corrupted = '$base$surrogate';
            expect(
              () => normalizeItf(corrupted),
              throwsA(isA<ArgumentError>()),
            );
            break;
        }
      }
    });

    test('Custom Barcode1DSceneOptions and Metadata Handling', () {
      final callerMeta = {'customKey': 'customVal'};

      final options = Barcode1DSceneOptions(
        moduleWidth: 6,
        barHeight: 160,
        foreground: '#112233',
        background: '#ffffff',
        label: 'Custom Label',
        metadata: callerMeta,
      );

      final scene = layoutItf('1234', options);
      expect(scene.metadata['label'], equals('Custom Label'));
      expect(scene.metadata['customKey'], equals('customVal'));
      expect(
        scene.metadata[itfMetadataKeySymbology],
        equals(itfMetadataValueSymbology),
      );
      expect(scene.metadata[itfMetadataKeyPairCount], equals('2'));
      expect(scene.height, equals(160));
    });

    test('itfMessageFor coverage', () {
      expect(
        itfMessageFor(ItfErrorCode.inputTooLong),
        equals(itfMessageInputTooLong),
      );
      expect(
        itfMessageFor(ItfErrorCode.invalidLength),
        equals(itfMessageInvalidLength),
      );
      expect(
        itfMessageFor(ItfErrorCode.invalidCharacter),
        equals(itfMessageInvalidCharacter),
      );
      expect(itfMessageFor(null), equals(itfMessageInvalidCharacter));
    });

    test('placeholders and constants', () {
      expect(itfRedactedPlaceholder, equals('<redacted>'));
      expect(itfInvalidPlaceholder, equals('<invalid>'));
    });
  });
}
