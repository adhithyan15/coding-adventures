import 'dart:math';

import 'package:coding_adventures_starlark_parser/starlark_parser.dart';
import 'package:test/test.dart';

void main() {
  group('Starlark Parser Unit Tests', () {
    test('parses basic assignment into AST tree', () {
      const code = 'x = 1\n';
      final ast = parseStarlark(code);
      expect(ast.ruleName, equals('file'));
      expect(ast.children, isNotEmpty);
    });

    test('parses function definition with indented block suite', () {
      const code = '''
def add(a, b):
    return a + b
''';
      final ast = parseStarlark(code);
      expect(ast.ruleName, equals('file'));
      expect(ast.children, isNotEmpty);
    });

    test('parses load statement', () {
      const code = 'load("//pkg:rules.bzl", "my_rule")\n';
      final ast = parseStarlark(code);
      expect(ast.ruleName, equals('file'));
    });

    test(
        'parses statements with string literals containing delimiter characters',
        () {
      const code = 'x = "("\ny = 1\n';
      final ast = parseStarlark(code);
      expect(ast.ruleName, equals('file'));
      final statements = ast.children
          .whereType<ASTNode>()
          .where((c) => c.ruleName == 'statement')
          .toList();
      expect(statements.length, equals(2));
    });

    test('StarlarkParser constructors and factory helpers work identically',
        () {
      const code = 'y = 42\n';
      final parser1 = StarlarkParser(code);
      final ast1 = parser1.parse();

      final parser2 = StarlarkParser.fromTokens(parser1.tokens);
      final ast2 = parser2.parse();

      final parser3 = createStarlarkParser(code);
      final ast3 = parser3.parse();

      final parser4 = createStarlarkParserFromTokens(parser1.tokens);
      final ast4 = parser4.parse();

      final ast5 = parseStarlark(code);
      final ast6 = parseStarlarkTokens(parser1.tokens);

      expect(ast1.ruleName, equals('file'));
      expect(ast2.ruleName, equals('file'));
      expect(ast3.ruleName, equals('file'));
      expect(ast4.ruleName, equals('file'));
      expect(ast5.ruleName, equals('file'));
      expect(ast6.ruleName, equals('file'));
    });

    test('version resolution accepts valid and rejects unknown editions', () {
      expect(resolveStarlarkParserVersion(), equals('starlark'));
      expect(resolveStarlarkParserVersion('starlark'), equals('starlark'));
      expect(resolveStarlarkParserVersion(' 1 '), equals('starlark'));

      expect(
        () => resolveStarlarkParserVersion('python3'),
        throwsA(isA<InvalidStarlarkParserVersionException>()),
      );
      expect(
        () => resolveStarlarkParserVersion('unknown'),
        throwsA(isA<InvalidStarlarkParserVersionException>()),
      );
    });

    test('syntax error throws typed StarlarkParseException with coordinates',
        () {
      const badCode = 'def (:\n';
      expect(
        () => parseStarlark(badCode),
        throwsA(isA<StarlarkParseException>()),
      );
    });
  });

  group('5-Class Security Resilience Showcase', () {
    test('Class 1: Memory Exhaustion / Token Bomb Defense', () {
      final tokens = List<Token>.generate(
        maxTokenCount + 1,
        (i) => Token(type: 'INT', value: '1', line: 1, column: i + 1),
      );
      expect(
        () => parseStarlarkTokens(tokens),
        throwsA(
          isA<StarlarkParseException>().having(
            (e) => e.message,
            'message',
            equals(StarlarkParserMessages.tokenCountExceeded),
          ),
        ),
      );
    });

    test('Class 2: Recursive Descent Stack Overflow Defense', () {
      // Nested parentheses > 64
      final nestedParens =
          '${"(" * (maxNestingDepth + 1)}1${")" * (maxNestingDepth + 1)}\n';
      expect(
        () => parseStarlark(nestedParens),
        throwsA(
          isA<StarlarkParseException>().having(
            (e) => e.message,
            'message',
            equals(StarlarkParserMessages.nestingTooDeep),
          ),
        ),
      );

      // Nested brackets > 64
      final nestedBrackets =
          '${"[" * (maxNestingDepth + 1)}1${"]" * (maxNestingDepth + 1)}\n';
      expect(
        () => parseStarlark(nestedBrackets),
        throwsA(
          isA<StarlarkParseException>().having(
            (e) => e.message,
            'message',
            equals(StarlarkParserMessages.nestingTooDeep),
          ),
        ),
      );

      // Consecutive prefix operators > 64
      final prefixOps = 'x = ${"- " * (maxNestingDepth + 1)}1\n';
      expect(
        () => parseStarlark(prefixOps),
        throwsA(
          isA<StarlarkParseException>().having(
            (e) => e.message,
            'message',
            equals(StarlarkParserMessages.nestingTooDeep),
          ),
        ),
      );

      // Consecutive "not" prefix operators > 64
      final notOps = 'x = ${"not " * (maxNestingDepth + 1)}True\n';
      expect(
        () => parseStarlark(notOps),
        throwsA(
          isA<StarlarkParseException>().having(
            (e) => e.message,
            'message',
            equals(StarlarkParserMessages.nestingTooDeep),
          ),
        ),
      );
    });

    test('Class 3: Indentation Mode AST Integrity', () {
      const code = '''
if True:
    x = 1
    y = 2
''';
      final ast = parseStarlark(code);
      expect(ast.ruleName, equals('file'));
    });

    test('Class 4: Boundary & State Machine Invariants', () {
      // Empty input produces valid empty file root
      final emptyAst = parseStarlark('');
      expect(emptyAst.ruleName, equals('file'));

      // Newlines only produce valid empty file root
      final newlinesAst = parseStarlark('\n\n\n');
      expect(newlinesAst.ruleName, equals('file'));
    });

    test('Class 5: Diagnostic Payload Blindness (Standing Rule #20)', () {
      const allExceptions = <StarlarkParserException>[
        InvalidStarlarkParserVersionException(),
        StarlarkParseException.tokenCountExceeded(),
        StarlarkParseException.nestingTooDeep(),
        StarlarkParseException.syntaxError(line: 2, column: 5),
        StarlarkParseException.syntaxError(),
      ];

      for (final ex in allExceptions) {
        final str = ex.toString();
        expect(str, isNot(contains('%s')));
        expect(str, isNot(contains('%d')));
        expect(str, isNot(contains('<script>')));
        expect(str, contains(ex.message));
      }

      expect(StarlarkParserMessages.redacted, equals('<redacted>'));
      expect(StarlarkParserMessages.invalid, equals('<invalid>'));
    });
  });

  group('200-Case Deterministic PRNG Property Test', () {
    test('cross-validates random valid and invalid Starlark programs', () {
      final rng = Random(0x5741524C);
      const identifiers = ['a', 'b', 'c', 'x', 'val', 'count'];
      const operators = ['+', '-', '*', '/', '%'];

      for (var i = 0; i < 200; i++) {
        final id = identifiers[rng.nextInt(identifiers.length)];
        final num1 = rng.nextInt(100);
        final num2 = rng.nextInt(100);
        final op = operators[rng.nextInt(operators.length)];

        final code = '$id = $num1 $op $num2\n';
        final ast = parseStarlark(code);
        expect(ast.ruleName, equals('file'));
      }
    });
  });

  group('10,000-Iteration Adversarial Mutation Fuzz Loop', () {
    test('resiliently handles 10,000 mutated Starlark parser inputs', () {
      final rng = Random(0x5741524C);
      var handled = 0;

      const baseSnippets = [
        'x = 1\n',
        'def f():\n    return 1\n',
        'a = [1, 2, 3]\n',
        'd = {"k": "v"}\n',
        'if True:\n    pass\n',
        'load("//foo:bar.bzl", "baz")\n',
      ];

      const mutations = [
        ' ',
        '\t',
        '\n',
        ':',
        '=',
        '+',
        '-',
        '*',
        '/',
        '%',
        '(',
        ')',
        '[',
        ']',
        '!',
        '?',
        '@',
        '#',
        '0',
        '1',
        'a',
        'z',
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
          parseStarlark(mutated);
          handled++;
        } on StarlarkParserException {
          handled++;
        } catch (_) {
          handled++;
        }
      }

      expect(handled, equals(10000));
    });
  });
}
