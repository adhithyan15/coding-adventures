import 'dart:math';

import 'package:coding_adventures_java_lexer/java_lexer.dart';
import 'package:coding_adventures_java_lexer/src/constants.dart'
    show maxSourceLength;
import 'package:test/test.dart';

import 'java_lexer_conformance_test.dart' as conformance;

void main() {
  group('Java Lexer Conformance Subsuite', () {
    conformance.main();
  });

  group('5-Class Lexer Security Resilience Showcase', () {
    test('Class 1: Oversized Source Ceiling & Exact Boundary (CWE-400)', () {
      // Exact boundary: maxSourceLength code units of whitespace accepted
      final atBoundary = ' ' * maxSourceLength;
      final tokensBoundary = tokenizeJava(atBoundary);
      expect(tokensBoundary, hasLength(1));
      expect(tokensBoundary.first.type, equals('EOF'));

      // Exact boundary + 1 code unit rejected
      final oversized = ' ' * (maxSourceLength + 1);
      expect(
        () => tokenizeJava(oversized),
        throwsA(
          isA<InvalidJavaSourceException>().having(
            (e) => e.errorId,
            'errorId',
            equals('invalid-source'),
          ),
        ),
      );
    });

    test(
      'Class 1b: Non-ASCII ceiling counts UTF-16 code units (Swift parity)',
      () {
        // 'é' is 1 UTF-16 code unit but 2 UTF-8 bytes. The Swift lane counts
        // source.utf16.count, so both lanes must accept/reject the same input.
        const prefix = '//';
        final atBoundary = prefix + ('é' * (maxSourceLength - prefix.length));
        expect(atBoundary.length, equals(maxSourceLength));
        expect(() => createJavaLexer(atBoundary), returnsNormally);

        final overBoundary = '${atBoundary}é';
        expect(
          () => createJavaLexer(overBoundary),
          throwsA(
            isA<InvalidJavaSourceException>().having(
              (e) => e.errorId,
              'errorId',
              equals('invalid-source'),
            ),
          ),
        );
      },
    );

    test(
      'Class 2: Backtracking Immunity on Pathological Inputs (CWE-1333)',
      () {
        // Pathological unclosed string / numbers with lots of underscores / digits
        final pathologicalNum = '1' + ('0' * 500) + 'L;';
        final tokens = tokenizeJava(pathologicalNum);
        expect(tokens, isNotEmpty);
        expect(tokens.first.type, equals('NUMBER'));

        // Pathological comment blocks
        final pathologicalComment = '/* ' + ('* ' * 1000) + '*/ int z = 1;';
        final commentTokens = tokenizeJava(pathologicalComment);
        expect(commentTokens, isNotEmpty);
        expect(commentTokens.any((t) => t.value == 'int'), isTrue);
      },
    );

    test('Class 3: Lexical Smuggling & Lone Surrogate Refusal', () {
      // Lone high surrogate
      expect(
        () => tokenizeJava('int x = 1;\uD800'),
        throwsA(isA<InvalidJavaSourceException>()),
      );

      // Lone low surrogate
      expect(
        () => tokenizeJava('int x = 1;\uDC00'),
        throwsA(isA<InvalidJavaSourceException>()),
      );

      // Reversed surrogates
      expect(
        () => tokenizeJava('int x = 1;\uDC00\uD800'),
        throwsA(isA<InvalidJavaSourceException>()),
      );
    });

    test('Class 4: Edition Downgrade Keyword Confusion & Version Aliases', () {
      // Invalid version string throws InvalidJavaVersionException
      expect(
        () => tokenizeJava('class B {}', version: 'invalid-99'),
        throwsA(
          isA<InvalidJavaVersionException>().having(
            (e) => e.errorId,
            'errorId',
            equals('invalid-version'),
          ),
        ),
      );

      // 'yield' is a keyword in 14, identifier in 10
      final t14Yield = tokenizeJava('yield 42;', version: '14');
      expect(
        t14Yield.any((t) => t.type == 'KEYWORD' && t.value == 'yield'),
        isTrue,
      );

      final t10Yield = tokenizeJava('int yield = 42;', version: '10');
      expect(
        t10Yield.any((t) => t.type == 'NAME' && t.value == 'yield'),
        isTrue,
      );

      // Version aliases normalization to canonical versions
      final lexer1_0 = createJavaLexer('int x = 1;', version: 'java1_0');
      expect(lexer1_0.version, equals('1.0'));

      final lexer1_4 = createJavaLexer('int x = 1;', version: 'java1.4');
      expect(lexer1_4.version, equals('1.4'));

      final lexer8 = createJavaLexer('int x = 1;', version: 'java8');
      expect(lexer8.version, equals('8'));

      final lexer8Alias = createJavaLexer('int x = 1;', version: '1.8');
      expect(lexer8Alias.version, equals('8'));

      final lexer21 = createJavaLexer('int x = 1;', version: 'java21');
      expect(lexer21.version, equals('21'));

      final lexerDefault = createJavaLexer('int x = 1;');
      expect(lexerDefault.version, equals('21'));
    });

    test(
      'Class 5: Payload-Blind Security & Zero Sensitive Leakage (CWE-209)',
      () {
        final secretPayload = 'super_secret_pin_987654';
        try {
          tokenizeJava('int $secretPayload = ###;', version: '21');
          fail('Expected JavaLexicalException');
        } on JavaLexerException catch (e) {
          expect(e.message.contains(secretPayload), isFalse);
          expect(e.errorId, equals('lexical-error'));
          expect(e.toString().contains(secretPayload), isFalse);
        }

        // Check version error is also payload blind
        final sensitiveVersion = 'v_confidential_999';
        try {
          tokenizeJava('int x = 1;', version: sensitiveVersion);
          fail('Expected InvalidJavaVersionException');
        } on JavaLexerException catch (e) {
          expect(e.message.contains(sensitiveVersion), isFalse);
          expect(e.errorId, equals('invalid-version'));
          expect(e.toString().contains(sensitiveVersion), isFalse);
        }
      },
    );
  });

  group('Validation Precedence and Error Types', () {
    test('Source length validation runs before version resolution', () {
      final oversized = ' ' * (maxSourceLength + 1);
      // Both source is invalid and version is invalid: source error takes precedence
      expect(
        () => tokenizeJava(oversized, version: 'invalid-version-string'),
        throwsA(isA<InvalidJavaSourceException>()),
      );
    });

    test('Constructors and error codes verification', () {
      final exTooLarge = const InvalidJavaSourceException.sourceTooLarge();
      expect(exTooLarge.errorCode, equals(JavaLexerErrorCode.invalidSource));
      expect(exTooLarge.errorId, equals('invalid-source'));

      final exSurrogates = const InvalidJavaSourceException.invalidSurrogates();
      expect(exSurrogates.errorCode, equals(JavaLexerErrorCode.invalidSource));

      final exLexical = const JavaLexicalException(line: 4, column: 12);
      expect(exLexical.line, equals(4));
      expect(exLexical.column, equals(12));
      expect(exLexical.toString(), contains('(4:12)'));

      final exVersion = const InvalidJavaVersionException();
      expect(exVersion.errorCode, equals(JavaLexerErrorCode.invalidVersion));
    });
  });

  group('PRNG Round-Trip Property Test', () {
    test('Token concatenation reconstructs normalized token values', () {
      final rng = Random(0xCAFEBABE);
      final keywords = ['public', 'class', 'static', 'void', 'int', 'final'];
      final identifiers = ['foo', 'bar', 'baz', 'qux', 'alpha', 'beta'];
      final numbers = ['0', '42', '1000', '0xFF', '314'];

      for (var iteration = 0; iteration < 200; iteration++) {
        final tokenCount = 3 + rng.nextInt(10);
        final parts = <String>[];
        for (var i = 0; i < tokenCount; i++) {
          final choice = rng.nextInt(3);
          if (choice == 0) {
            parts.add(keywords[rng.nextInt(keywords.length)]);
          } else if (choice == 1) {
            parts.add(identifiers[rng.nextInt(identifiers.length)]);
          } else {
            parts.add(numbers[rng.nextInt(numbers.length)]);
          }
        }
        final source = parts.join(' ');
        final tokens = tokenizeJava(source, version: '21');

        // Filter out EOF
        final nonEof = tokens.where((t) => t.type != 'EOF').toList();
        expect(
          nonEof.length,
          equals(parts.length),
          reason: 'Mismatch for generated source: $source',
        );
        for (var i = 0; i < parts.length; i++) {
          expect(nonEof[i].value, equals(parts[i]));
        }
      }
    });
  });

  group('10,000-Iteration Mutation Fuzz Loop', () {
    test(
      'Mutated tokens never crash or trigger unexpected exception types',
      () {
        final rng = Random(0x1337D00D);
        final seedTokens = [
          'public',
          'class',
          'Test',
          '{',
          'int',
          'x',
          '=',
          '42',
          ';',
          'assert',
          'true',
          ';',
          '}',
        ];

        for (var i = 0; i < 10000; i++) {
          final buffer = StringBuffer();
          final numTokens = 1 + rng.nextInt(8);
          for (var t = 0; t < numTokens; t++) {
            var token = seedTokens[rng.nextInt(seedTokens.length)];
            // Mutate token characters
            if (rng.nextDouble() < 0.25) {
              final mutationType = rng.nextInt(4);
              if (mutationType == 0) {
                // Insert invalid byte/character
                final insertPos = rng.nextInt(token.length + 1);
                token = token.substring(0, insertPos) +
                    '#' +
                    token.substring(insertPos);
              } else if (mutationType == 1) {
                // Replace with random character
                final replacePos = rng.nextInt(token.length);
                token = token.substring(0, replacePos) +
                    String.fromCharCode(33 + rng.nextInt(90)) +
                    token.substring(replacePos + 1);
              } else if (mutationType == 2) {
                // Delete char
                if (token.length > 1) {
                  final delPos = rng.nextInt(token.length);
                  token =
                      token.substring(0, delPos) + token.substring(delPos + 1);
                }
              } else {
                // Append surrogate if needed
                if (rng.nextBool()) {
                  token = token + '\uD800'; // Lone surrogate
                }
              }
            }
            buffer.write(token);
            if (rng.nextBool()) buffer.write(' ');
          }

          final source = buffer.toString();
          try {
            tokenizeJava(source);
          } on JavaLexerException catch (e) {
            expect(e.errorCode, isNotNull);
            expect(e.errorId, isNotEmpty);
          } catch (e) {
            fail(
              'Unexpected non-JavaLexerException thrown on input "$source": $e',
            );
          }
        }
      },
    );
  });
}
