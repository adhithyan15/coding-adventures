import 'dart:math';

import 'package:coding_adventures_csharp_lexer/csharp_lexer.dart';
import 'package:coding_adventures_csharp_lexer/src/constants.dart'
    show maxSourceLength;
import 'package:coding_adventures_csharp_lexer/src/messages.dart'
    show formatLocation, messageFor;
import 'package:test/test.dart';

import 'csharp_lexer_conformance_test.dart' as conformance;

void main() {
  group('C# Lexer Conformance Subsuite', () {
    conformance.main();
  });

  group('5-Class Lexer Security Resilience Showcase', () {
    test('Class 1: Oversized Source Ceiling & Exact Boundary (CWE-400)', () {
      // Exact boundary: maxSourceLength code units of whitespace accepted
      final atBoundary = ' ' * maxSourceLength;
      final tokensBoundary = tokenizeCSharp(atBoundary);
      expect(tokensBoundary, hasLength(1));
      expect(tokensBoundary.first.type, equals('EOF'));

      // Exact boundary + 1 code unit rejected
      final oversized = ' ' * (maxSourceLength + 1);
      expect(
        () => tokenizeCSharp(oversized),
        throwsA(
          isA<InvalidCSharpSourceException>().having(
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
        expect(() => createCSharpLexer(atBoundary), returnsNormally);

        final overBoundary = '${atBoundary}é';
        expect(
          () => createCSharpLexer(overBoundary),
          throwsA(
            isA<InvalidCSharpSourceException>().having(
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
        final tokens = tokenizeCSharp(pathologicalNum);
        expect(tokens, isNotEmpty);
        expect(tokens.first.type, equals('NUMBER'));

        // Pathological comment blocks
        final pathologicalComment = '/* ' + ('* ' * 1000) + '*/ int z = 1;';
        final commentTokens = tokenizeCSharp(pathologicalComment);
        expect(commentTokens, isNotEmpty);
        expect(commentTokens.any((t) => t.value == 'int'), isTrue);
      },
    );

    test('Class 3: Lexical Smuggling & Lone Surrogate Refusal', () {
      // Lone high surrogate
      expect(
        () => tokenizeCSharp('int x = 1;\uD800'),
        throwsA(isA<InvalidCSharpSourceException>()),
      );

      // Lone low surrogate
      expect(
        () => tokenizeCSharp('int x = 1;\uDC00'),
        throwsA(isA<InvalidCSharpSourceException>()),
      );

      // Reversed surrogates
      expect(
        () => tokenizeCSharp('int x = 1;\uDC00\uD800'),
        throwsA(isA<InvalidCSharpSourceException>()),
      );

      // High surrogate followed by code unit above low surrogate max
      expect(
        () => tokenizeCSharp('int x = 1;\uD800\uE000'),
        throwsA(isA<InvalidCSharpSourceException>()),
      );
    });

    test('Class 4: Edition Downgrade Keyword Confusion & Version Aliases', () {
      // Invalid version string throws InvalidCSharpVersionException
      expect(
        () => tokenizeCSharp('class B {}', version: 'invalid-99'),
        throwsA(
          isA<InvalidCSharpVersionException>().having(
            (e) => e.errorId,
            'errorId',
            equals('invalid-version'),
          ),
        ),
      );

      // 'yield' is a keyword in 2.0, identifier in 1.0
      final t20Yield = tokenizeCSharp('yield return 42;', version: '2.0');
      expect(
        t20Yield.any((t) => t.type == 'KEYWORD' && t.value == 'yield'),
        isTrue,
      );

      final t10Yield = tokenizeCSharp('int yield = 42;', version: '1.0');
      expect(
        t10Yield.any((t) => t.type == 'NAME' && t.value == 'yield'),
        isTrue,
      );

      // Version aliases normalization to canonical versions
      final lexer1_0 = createCSharpLexer('int x = 1;', version: 'csharp1_0');
      expect(lexer1_0.version, equals('1.0'));

      final lexer2_0 = createCSharpLexer('int x = 1;', version: 'csharp2.0');
      expect(lexer2_0.version, equals('2.0'));

      final lexer12 = createCSharpLexer('int x = 1;', version: 'csharp12.0');
      expect(lexer12.version, equals('12.0'));

      final lexer12Alias = createCSharpLexer('int x = 1;', version: '12');
      expect(lexer12Alias.version, equals('12.0'));

      final lexerDefault = createCSharpLexer('int x = 1;');
      expect(lexerDefault.version, equals('12.0'));
    });

    test(
      'Class 5: Payload-Blind Security & Zero Sensitive Leakage (CWE-209)',
      () {
        final secretPayload = 'super_secret_pin_987654';
        try {
          tokenizeCSharp('int $secretPayload = `;;;', version: '12.0');
          fail('Expected CSharpLexicalException');
        } on CSharpLexerException catch (e) {
          expect(e.message.contains(secretPayload), isFalse);
          expect(e.errorId, equals('lexical-error'));
          expect(e.toString().contains(secretPayload), isFalse);
        }

        // Check version error is also payload blind
        final sensitiveVersion = 'v_confidential_999';
        try {
          tokenizeCSharp('int x = 1;', version: sensitiveVersion);
          fail('Expected InvalidCSharpVersionException');
        } on CSharpLexerException catch (e) {
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
        () => tokenizeCSharp(oversized, version: 'invalid-version-string'),
        throwsA(isA<InvalidCSharpSourceException>()),
      );
    });

    test('Constructors and error codes verification', () {
      final exTooLarge = const InvalidCSharpSourceException.sourceTooLarge();
      expect(exTooLarge.errorCode, equals(CSharpLexerErrorCode.invalidSource));
      expect(exTooLarge.errorId, equals('invalid-source'));

      final exSurrogates =
          const InvalidCSharpSourceException.invalidSurrogates();
      expect(
          exSurrogates.errorCode, equals(CSharpLexerErrorCode.invalidSource));

      final exLexical = const CSharpLexicalException(line: 4, column: 12);
      expect(exLexical.line, equals(4));
      expect(exLexical.column, equals(12));
      expect(exLexical.toString(), contains('(4:12)'));

      final exVersion = const InvalidCSharpVersionException();
      expect(exVersion.errorCode, equals(CSharpLexerErrorCode.invalidVersion));
    });
  });

  group('PRNG Round-Trip Property Test', () {
    test('Token concatenation reconstructs normalized token values', () {
      final rng = Random(0xCAFEBABE);
      final keywords = ['public', 'class', 'static', 'void', 'int', 'struct'];
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
        final tokens = tokenizeCSharp(source, version: '12.0');

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
          'void',
          'Run',
          '(',
          ')',
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
                    '`' +
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
            tokenizeCSharp(source);
          } on CSharpLexerException catch (e) {
            expect(e.errorCode, isNotNull);
            expect(e.errorId, isNotEmpty);
          } catch (e) {
            fail(
              'Unexpected non-CSharpLexerException thrown on input "$source": $e',
            );
          }
        }
      },
    );
  });

  group('Diagnostic and grammar helper coverage', () {
    test('messageFor maps all error codes and formatLocation handles null', () {
      expect(messageFor(CSharpLexerErrorCode.invalidVersion), isNotEmpty);
      expect(messageFor(CSharpLexerErrorCode.invalidSource), isNotEmpty);
      expect(messageFor(CSharpLexerErrorCode.lexicalError), isNotEmpty);
      expect(formatLocation('msg', null, null), equals('msg'));
    });

    test('loadCSharpTokenGrammar resolves versions', () {
      final grammar = loadCSharpTokenGrammar(version: '10.0');
      expect(grammar.definitions, isNotEmpty);
    });
  });
}
