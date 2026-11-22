import 'dart:math';

import 'package:coding_adventures_typescript_lexer/coding_adventures_typescript_lexer.dart';
import 'package:test/test.dart';

void main() {
  group('5-Class TypeScript Lexer Security Resilience Showcase', () {
    test('Class 1: CWE-400 Source Length DoS Defense', () {
      final oversized = 'a' * (maxSourceLength + 1);
      expect(
        () => tokenizeTypescript(oversized),
        throwsA(
          isA<InvalidTypescriptSourceException>().having(
            (e) => e.errorId,
            'errorId',
            equals(errorIdInvalidSource),
          ),
        ),
      );

      final atLimit = 'a' * maxSourceLength;
      final tokens = tokenizeTypescript(atLimit);
      expect(tokens, isNotEmpty);
    });

    test('Class 2: CWE-400 Lone / Reversed UTF-16 Surrogates', () {
      // Lone high surrogate
      expect(
        () => tokenizeTypescript('var x = "\uD800";'),
        throwsA(isA<InvalidTypescriptSourceException>()),
      );

      // Lone low surrogate
      expect(
        () => tokenizeTypescript('var x = "\uDC00";'),
        throwsA(isA<InvalidTypescriptSourceException>()),
      );

      // Reversed surrogates
      expect(
        () => tokenizeTypescript('var x = "\uDC00\uD800";'),
        throwsA(isA<InvalidTypescriptSourceException>()),
      );

      // Valid paired surrogate (astral emoji)
      final tokens = tokenizeTypescript('var x = "\u{1F600}";');
      expect(tokens, isNotEmpty);
    });

    test('Class 3: CWE-209 Payload-Blind Confidentiality', () {
      const secret = 'SUPER_SECRET_TOKEN_9999';
      try {
        tokenizeTypescript('var $secret = @#INVALID#@;');
        fail('Expected lexical exception');
      } on TypescriptLexerException catch (e) {
        expect(e.message, equals(messageLexicalError));
        expect(e.toString(), contains('TypeScript lexical analysis failed.'));
        expect(e.toString(), isNot(contains(secret)));
        expect(e.toString(), isNot(contains('@#INVALID#@')));
      }

      // Check version error is also payload-blind
      try {
        tokenizeTypescript('var x = 1;', version: 'INVALID_VERSION_EXPLOIT');
        fail('Expected version exception');
      } on TypescriptLexerException catch (e) {
        expect(e.message, equals(messageInvalidVersion));
        expect(e.toString(), isNot(contains('INVALID_VERSION_EXPLOIT')));
      }
    });

    test('Class 4: Multi-Edition Version Resolution & Rejection', () {
      // Default / empty
      expect(resolveTypescriptVersion(), equals('typescript'));
      expect(resolveTypescriptVersion(''), equals('typescript'));
      expect(resolveTypescriptVersion('  typescript  '), equals('typescript'));

      // Versioned editions & aliases
      expect(resolveTypescriptVersion('ts1.0'), equals('ts1.0'));
      expect(resolveTypescriptVersion('1.0'), equals('ts1.0'));
      expect(resolveTypescriptVersion('TS2.0'), equals('ts2.0'));
      expect(resolveTypescriptVersion('2.0'), equals('ts2.0'));
      expect(resolveTypescriptVersion('ts3.0'), equals('ts3.0'));
      expect(resolveTypescriptVersion('3.0'), equals('ts3.0'));
      expect(resolveTypescriptVersion('ts4.0'), equals('ts4.0'));
      expect(resolveTypescriptVersion('4.0'), equals('ts4.0'));
      expect(resolveTypescriptVersion('ts5.0'), equals('ts5.0'));
      expect(resolveTypescriptVersion('5.0'), equals('ts5.0'));
      expect(resolveTypescriptVersion('ts5.8'), equals('ts5.8'));
      expect(resolveTypescriptVersion('5.8'), equals('ts5.8'));

      // Unknown editions
      for (final bad in ['ts0.9', 'ts6.0', 'javascript', 'unknown', 'ts']) {
        expect(
          () => resolveTypescriptVersion(bad),
          throwsA(isA<InvalidTypescriptVersionException>()),
        );
      }
    });

    test('Class 5: Lexical Coverage Across All 7 Editions', () {
      final codeSamples = {
        'typescript':
            'let x: number = 10; function add(a: number): number { return a + 1; }',
        'ts1.0': 'var x = 1; interface Foo { bar: string; }',
        'ts2.0': 'let x: string | null = null; readonly y: number = 2;',
        'ts3.0': 'type Tuple = [string, number]; import type { X } from "mod";',
        'ts4.0':
            'type Tail<T extends any[]> = T extends [any, ...infer R] ? R : never;',
        'ts5.0':
            'class C { @logged method() {} } const x: const [1, 2] = [1, 2];',
        'ts5.8': '#!/usr/bin/env node\nconst x = 42;',
      };

      for (final entry in codeSamples.entries) {
        final version = entry.key;
        final code = entry.value;
        final tokens = tokenizeTypescript(code, version: version);
        expect(tokens, isNotEmpty, reason: 'Failed to lex $version');
        expect(tokens.last.type, equals('EOF'));
      }
    });
  });

  group('TypeScript Lexer API & Aliases', () {
    test('TypescriptLexer class and constructor', () {
      final lexer = TypescriptLexer('const a = 1;', version: 'ts5.8');
      expect(lexer.source, equals('const a = 1;'));
      expect(lexer.version, equals('ts5.8'));
      final tokens = lexer.tokenize();
      expect(tokens, isNotEmpty);
    });

    test('Lexer constructor validates bounds and surrogates before version',
        () {
      // Source length exceeded with invalid version: should throw SourceTooLarge first
      final huge = 'a' * (maxSourceLength + 1);
      expect(
        () => TypescriptLexer(huge, version: 'INVALID_VERSION'),
        throwsA(isA<InvalidTypescriptSourceException>()),
      );

      // Invalid surrogates with invalid version: should throw InvalidSurrogates first
      expect(
        () => TypescriptLexer('var x = "\uD800";', version: 'INVALID_VERSION'),
        throwsA(isA<InvalidTypescriptSourceException>()),
      );
    });

    test('Factory and convenience functions agree', () {
      const code = 'const pi: number = 3.14;';
      final tokens1 = tokenizeTypescript(code);
      final tokens2 = tokenizeTypeScript(code);
      final tokens3 = tokenizeTypescriptLexer(code);
      final tokens4 = createTypescriptLexer(code).tokenize();
      final tokens5 = createTypeScriptLexer(code).tokenize();

      expect(tokens1.length, equals(tokens2.length));
      expect(tokens1.length, equals(tokens3.length));
      expect(tokens1.length, equals(tokens4.length));
      expect(tokens1.length, equals(tokens5.length));
    });

    test('Exception formatting and location', () {
      const errWithLoc = TypescriptLexicalException(line: 3, column: 15);
      expect(errWithLoc.line, equals(3));
      expect(errWithLoc.column, equals(15));
      expect(errWithLoc.errorId, equals(errorIdLexicalError));
      expect(errWithLoc.toString(),
          equals('TypeScript lexical analysis failed. (3:15)'));

      const errWithoutLoc = TypescriptLexicalException();
      expect(errWithoutLoc.toString(),
          equals('TypeScript lexical analysis failed.'));

      const verErr = InvalidTypescriptVersionException();
      expect(verErr.errorId, equals(errorIdInvalidVersion));
      expect(verErr.toString(), equals(messageInvalidVersion));

      const srcErr = InvalidTypescriptSourceException.sourceTooLarge();
      expect(srcErr.errorId, equals(errorIdInvalidSource));
      expect(srcErr.toString(), equals(messageSourceTooLarge));

      const surrErr = InvalidTypescriptSourceException.invalidSurrogates();
      expect(surrErr.errorId, equals(errorIdInvalidSource));
      expect(surrErr.toString(), equals(messageInvalidSurrogates));
    });
  });

  group('Deterministic PRNG Property & Fuzz Tests', () {
    test('200-case PRNG property test ensures robust, version-stable lexing',
        () {
      final rng = Random(42);
      final keywords = <String>[
        'let',
        'const',
        'var',
        'if',
        'else',
        'while',
        'for',
        'function',
        'return',
        'class',
        'import',
        'export',
        'type',
        'interface',
      ];
      final operators = <String>[
        '=',
        '+',
        '-',
        '*',
        '/',
        '==',
        '===',
        '!=',
        '!==',
        '=>',
        ':',
        ';',
        ',',
        '.',
        '!',
      ];
      final delimiters = <String>['(', ')', '{', '}', '[', ']'];

      for (var i = 0; i < 200; i++) {
        final version = supportedTypescriptVersions[
            rng.nextInt(supportedTypescriptVersions.length)];
        final numTokens = rng.nextInt(30) + 1;
        final buffer = StringBuffer();

        for (var j = 0; j < numTokens; j++) {
          final choice = rng.nextInt(5);
          switch (choice) {
            case 0:
              buffer.write(keywords[rng.nextInt(keywords.length)]);
            case 1:
              buffer.write('id_${rng.nextInt(100)}');
            case 2:
              buffer.write('${rng.nextInt(1000)}');
            case 3:
              buffer.write(operators[rng.nextInt(operators.length)]);
            case 4:
              buffer.write(delimiters[rng.nextInt(delimiters.length)]);
          }
          buffer.write(' ');
        }

        final source = buffer.toString();
        final tokens = tokenizeTypescript(source, version: version);
        expect(tokens, isNotEmpty);
        expect(tokens.last.type, equals('EOF'));
      }
    });

    test('10,000-iteration mutation fuzz loop rejects or tokenizes safely', () {
      final rng = Random(1337);
      const baseProgram =
          'const answer: number = 42; function compute(x: number): number { return x * 2; }';
      final alphabet =
          'abcdefghijklmnopqrstuvwxyzABCDEFGHIJKLMNOPQRSTUVWXYZ0123456789 \t\n\r_+-*/=<>!~@#%^&|()[]{}:;,.\'"\\`';

      for (var i = 0; i < 10000; i++) {
        final version =
            supportedTypescriptVersions[i % supportedTypescriptVersions.length];
        final chars = baseProgram.split('');

        // Apply 1 to 3 random mutations
        final mutations = rng.nextInt(3) + 1;
        for (var m = 0; m < mutations; m++) {
          final pos = rng.nextInt(chars.length);
          final mutType = rng.nextInt(3);
          switch (mutType) {
            case 0: // replace
              chars[pos] = alphabet[rng.nextInt(alphabet.length)];
            case 1: // delete
              if (chars.length > 5) chars.removeAt(pos);
            case 2: // insert
              chars.insert(pos, alphabet[rng.nextInt(alphabet.length)]);
          }
        }

        final mutated = chars.join('');
        try {
          final tokens = tokenizeTypescript(mutated, version: version);
          expect(tokens, isNotEmpty);
        } on TypescriptLexerException {
          // Expected safe lexical or source rejection
        }
      }
    });
  });
}
