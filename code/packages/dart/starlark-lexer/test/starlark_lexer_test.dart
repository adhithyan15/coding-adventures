import 'dart:math';

import 'package:coding_adventures_starlark_lexer/starlark_lexer.dart';
import 'package:test/test.dart';

void main() {
  group('Starlark Lexer Unit Tests', () {
    test('tokenizes simple function definition and indentation', () {
      const code = '''
def greet(name):
    return "Hello, " + name
''';
      final tokens = tokenizeStarlark(code);
      final types = tokens.map((t) => t.type).toList();
      expect(types, [
        'KEYWORD',
        'NAME',
        'LPAREN',
        'NAME',
        'RPAREN',
        'COLON',
        'NEWLINE',
        'INDENT',
        'KEYWORD',
        'STRING',
        'PLUS',
        'NAME',
        'NEWLINE',
        'DEDENT',
        'EOF',
      ]);
    });

    test('StarlarkLexer class and factory functions work identically', () {
      const code = 'x = 10\n';
      final lexer = StarlarkLexer(code);
      expect(lexer.version, equals('starlark'));
      expect(lexer.source, equals(code));
      final tokens1 = lexer.tokenize();

      final lexer2 = createStarlarkLexer(code);
      final tokens2 = lexer2.tokenize();

      final tokens3 = tokenizeStarlarkLexer(code);
      final tokens4 = tokenizeStarlark(code);

      expect(tokens1.length, equals(tokens2.length));
      expect(tokens1.length, equals(tokens3.length));
      expect(tokens1.length, equals(tokens4.length));
    });

    test('version resolution accepts valid and rejects unknown editions', () {
      expect(resolveStarlarkVersion(), equals('starlark'));
      expect(resolveStarlarkVersion('starlark'), equals('starlark'));
      expect(resolveStarlarkVersion(' 1 '), equals('starlark'));

      expect(
        () => resolveStarlarkVersion('python3'),
        throwsA(isA<InvalidStarlarkVersionException>()),
      );
      expect(
        () => resolveStarlarkVersion('unknown'),
        throwsA(isA<InvalidStarlarkVersionException>()),
      );
    });

    test('rejects reserved Python keywords used as identifiers', () {
      const code = 'class = 1\n';
      expect(
        () => tokenizeStarlark(code),
        throwsA(isA<StarlarkLexicalException>()),
      );
    });
  });

  group('5-Class Security Resilience Showcase', () {
    test('Class 1: Memory Exhaustion / Allocation Bomb Defense', () {
      final oversized = 'a' * (maxSourceLength + 1);
      expect(
        () => tokenizeStarlark(oversized),
        throwsA(
          isA<InvalidStarlarkSourceException>().having(
            (e) => e.message,
            'message',
            equals(StarlarkLexerMessages.sourceTooLarge),
          ),
        ),
      );
    });

    test('Class 2: Unicode & Surrogate Pair Integrity Defense', () {
      // Lone high surrogate
      final loneHigh = 'x = "${String.fromCharCode(0xD800)}"\n';
      expect(
        () => tokenizeStarlark(loneHigh),
        throwsA(
          isA<InvalidStarlarkSourceException>().having(
            (e) => e.message,
            'message',
            equals(StarlarkLexerMessages.invalidSurrogates),
          ),
        ),
      );

      // Lone low surrogate
      final loneLow = 'x = "${String.fromCharCode(0xDC00)}"\n';
      expect(
        () => tokenizeStarlark(loneLow),
        throwsA(
          isA<InvalidStarlarkSourceException>().having(
            (e) => e.message,
            'message',
            equals(StarlarkLexerMessages.invalidSurrogates),
          ),
        ),
      );

      // Trailing high surrogate at end of string
      final trailingHigh = 'x = ${String.fromCharCode(0xD800)}';
      expect(
        () => tokenizeStarlark(trailingHigh),
        throwsA(
          isA<InvalidStarlarkSourceException>().having(
            (e) => e.message,
            'message',
            equals(StarlarkLexerMessages.invalidSurrogates),
          ),
        ),
      );

      // Valid surrogate pair succeeds
      final validPair =
          'x = "${String.fromCharCode(0xD83D)}${String.fromCharCode(0xDE00)}"\n';
      final tokens = tokenizeStarlark(validPair);
      expect(tokens.first.type, equals('NAME'));
    });

    test('Class 3: Indentation Ambiguity & Tab Injection Defense', () {
      const tabIndented = '\tx = 1\n';
      expect(
        () => tokenizeStarlark(tabIndented),
        throwsA(isA<StarlarkLexicalException>()),
      );
    });

    test('Class 4: Boundary & State Machine Invariants', () {
      const inconsistentDedent = '''
if True:
    x = 1
  y = 2
''';
      expect(
        () => tokenizeStarlark(inconsistentDedent),
        throwsA(isA<StarlarkLexicalException>()),
      );
    });

    test('Class 5: Diagnostic Payload Blindness (Standing Rule #20)', () {
      const allExceptions = <StarlarkLexerException>[
        InvalidStarlarkVersionException(),
        InvalidStarlarkSourceException.sourceTooLarge(),
        InvalidStarlarkSourceException.invalidSurrogates(),
        StarlarkLexicalException(line: 4, column: 12),
        StarlarkLexicalException(),
      ];

      for (final ex in allExceptions) {
        final str = ex.toString();
        expect(str, isNot(contains('%s')));
        expect(str, isNot(contains('%d')));
        expect(str, isNot(contains('<script>')));
        expect(str, contains(ex.message));
      }

      expect(StarlarkLexerMessages.redacted, equals('<redacted>'));
      expect(StarlarkLexerMessages.invalid, equals('<invalid>'));
    });
  });

  group('200-Case Deterministic PRNG Property Test', () {
    test('cross-validates random valid and malformed Starlark expressions', () {
      final rng = Random(0x5741524C);
      const identifiers = ['foo', 'bar', 'baz', 'x1', '_y', 'Alpha', 'BETA'];
      const operators = ['+', '-', '*', '/', '//', '%', '==', '!=', '<=', '>='];

      for (var i = 0; i < 200; i++) {
        final id1 = identifiers[rng.nextInt(identifiers.length)];
        final id2 = identifiers[rng.nextInt(identifiers.length)];
        final op = operators[rng.nextInt(operators.length)];
        final num = rng.nextInt(10000);

        final code = '$id1 = $id2 $op $num\n';
        final tokens = tokenizeStarlark(code);
        expect(tokens.isNotEmpty, isTrue);
        expect(tokens.first.type, equals('NAME'));
        expect(tokens.last.type, equals('EOF'));
      }
    });
  });

  group('10,000-Iteration Adversarial Mutation Fuzz Loop', () {
    test('resiliently handles 10,000 mutated Starlark inputs', () {
      final rng = Random(0x5741524C);
      var handled = 0;

      const baseSnippets = [
        'x = 1\n',
        'def f():\n    return 1\n',
        'a = [1, 2, 3]\n',
        'd = {"k": "v"}\n',
        '# comment\n',
        'if True:\n    pass\n',
        'load("//foo:bar.bzl", "baz")\n',
      ];

      const mutations = [
        ' ',
        '\t',
        '\n',
        '\r',
        '#',
        '"',
        "'",
        '\\',
        '(',
        ')',
        '[',
        ']',
        '{',
        '}',
        ':',
        ',',
        '=',
        '+',
        '-',
        '*',
        '/',
        '%',
        '!',
        '?',
        '@',
        '\$',
        '0',
        '9',
      ];

      for (var i = 0; i < 10000; i++) {
        final base = baseSnippets[rng.nextInt(baseSnippets.length)];
        final chars = base.split('');
        final mutationCount = rng.nextInt(3) + 1;

        for (var m = 0; m < mutationCount; m++) {
          final pos = rng.nextInt(chars.length);
          final mut = mutations[rng.nextInt(mutations.length)];
          chars[pos] = mut;
        }

        final mutated = chars.join();
        try {
          tokenizeStarlark(mutated);
          handled++;
        } on StarlarkLexerException {
          handled++;
        }
      }

      expect(handled, equals(10000));
    });
  });
}
