/// Port of the reference `rust/derive-parser` unit suite (16 tests).
library;

import 'package:coding_adventures_derive_parser/derive_parser.dart';
import 'package:coding_adventures_parser/parser.dart';
import 'package:test/test.dart';

bool containsRule(ASTNode node, String name) {
  if (node.ruleName == name) return true;
  return node.children.any((c) => c is ASTNode && containsRule(c, name));
}

bool parses(String src) => tryParseDerive(src) != null;

String nestedParenSource(int n) => '${'(' * n}1${')' * n}\n';

void main() {
  group('derive-parser', () {
    test('program is the root', () {
      expect(parseDerive('1\n').ruleName, equals('program'));
    });

    test('function application uses ordinary parens', () {
      final ast = parseDerive('DIF(u, x)\n');
      expect(containsRule(ast, 'postfix'), isTrue);
      expect(containsRule(ast, 'arglist'), isTrue);
    });

    test('assign shared by variable and function definition', () {
      expect(parses('x := 5\n'), isTrue);
      final ast = parseDerive('F(x) := x^2 + 1\n');
      expect(containsRule(ast, 'assignment'), isTrue);
    });

    test('eq is equation distinct from assign', () {
      expect(parses('x = 4\n'), isTrue);
      expect(containsRule(parseDerive('x = 4\n'), 'comparison'), isTrue);
    });

    test('vector and matrix literals parse', () {
      expect(parses('[a, b, c]\n'), isTrue);
      expect(parses('[a, b; c, d]\n'), isTrue);
      expect(
        containsRule(parseDerive('[a, b, c]\n'), 'vector'),
        isTrue,
      );
    });

    test('boolean keywords parse', () {
      expect(parses('a AND b OR NOT c\n'), isTrue);
      expect(
        containsRule(parseDerive('a AND b\n'), 'logical_and'),
        isTrue,
      );
      expect(
        containsRule(parseDerive('a OR b\n'), 'logical_or'),
        isTrue,
      );
      expect(containsRule(parseDerive('NOT a\n'), 'logical_not'), isTrue);
    });

    test('arithmetic precedence', () {
      final ast = parseDerive('2 + 3 * 4 ^ 2\n');
      expect(containsRule(ast, 'additive'), isTrue);
      expect(containsRule(ast, 'multiplicative'), isTrue);
      expect(containsRule(ast, 'power'), isTrue);
    });

    test('unary minus binds looser than power', () {
      // -x^2 must parse as -(x^2): `unary` wraps `power`, not the
      // reverse.
      final ast = parseDerive('-x^2\n');
      expect(containsRule(ast, 'unary'), isTrue);
      expect(containsRule(ast, 'power'), isTrue);
    });

    test('power is right associative by shape', () {
      // 4^3^2 parses without error; right-associativity is exercised
      // by the recursive-descent shape (power's RHS is `unary`, which
      // falls back to `power`).
      expect(parses('4^3^2\n'), isTrue);
    });

    test('grouping parens parse', () {
      expect(parses('(1 + 2) * 3\n'), isTrue);
      expect(
        containsRule(parseDerive('(1 + 2) * 3\n'), 'group'),
        isTrue,
      );
    });

    test('nested function calls parse', () {
      expect(parses('SIN(COS(x))\n'), isTrue);
    });

    test('multi arg builtin calls parse', () {
      expect(parses('INT(u, x, a, b)\n'), isTrue);
      expect(parses('SUM(expr, var, start, end)\n'), isTrue);
    });

    test('syntax error is reported', () {
      expect(tryParseDerive('1 +\n'), isNull);
      expect(tryParseDerive('(1 + 2\n'), isNull);
    });

    test('deeply nested input returns error not overflow', () {
      // 5000 levels — far past the cap — must fail with a clean
      // error, never a StackOverflowError: the guard, not the stack,
      // stops the recursion.
      expect(tryParseDerive(nestedParenSource(5000)), isNull);
    });

    test('nesting up to cap still parses', () {
      // The measured 14/15 boundary holds here too: the cap counts
      // rule frames identically in both engines.
      expect(
        tryParseDerive(nestedParenSource(14)),
        isNotNull,
        reason: '14 levels must stay under the cap',
      );
      expect(
        tryParseDerive(nestedParenSource(15)),
        isNull,
        reason: 'one nesting level past the cap must fail',
      );
    });

    test('deep error matches the reference message', () {
      // The entry rule partially succeeds on 5000-deep input, so both
      // engines report through the furthest-failure path — with the
      // guard's own expectation text inside. Verified byte-identical
      // against the Rust crate (which reports the same message at
      // 1:16, plus its own prefix).
      try {
        parseDerive(nestedParenSource(5000));
        fail('expected GrammarParseError');
      } on GrammarParseError catch (e) {
        expect(
          e.message,
          equals(
            'Expected input within the supported nesting limit or '
            'assignment or expr, got "("',
          ),
        );
      }
    });
  });
}
