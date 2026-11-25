/// Port of the reference `rust/s-parser` unit suite (24 tests).
library;

import 'package:coding_adventures_lexer/lexer.dart';
import 'package:coding_adventures_parser/parser.dart';
import 'package:coding_adventures_s_parser/s_parser.dart';
import 'package:test/test.dart';

bool containsRule(ASTNode node, String name) {
  if (node.ruleName == name) return true;
  return node.children.any((c) => c is ASTNode && containsRule(c, name));
}

/// Every token value appearing under [node], in order.
List<String> tokenValues(ASTNode node) {
  final out = <String>[];
  void walk(ASTNode n) {
    for (final c in n.children) {
      if (c is ASTNode) {
        walk(c);
      } else if (c is Token) {
        out.add(c.value);
      }
    }
  }

  walk(node);
  return out;
}

bool parses(String src) => tryParseS(src) != null;

String nestedParenSource(int n) => '${'(' * n}1${')' * n}\n';

void main() {
  group('depth guard', () {
    test('deeply nested input returns error not overflow', () {
      expect(tryParseS(nestedParenSource(5000)), isNull);
    });

    test('nesting up to cap still parses', () {
      // 15 legitimate levels (one more than r/derive: the s cascade
      // re-enters one fewer rule per nesting level).
      expect(
        tryParseS(nestedParenSource(15)),
        isNotNull,
        reason: '15 levels must stay under the cap',
      );
      expect(
        tryParseS(nestedParenSource(16)),
        isNull,
        reason: 'one nesting level past the cap must fail',
      );
    });
  });

  group('s-parser', () {
    test('program is the root', () {
      expect(parseS('1\n').ruleName, equals('program'));
    });

    test('all assignment forms parse', () {
      for (final src in [
        'x <- 1\n',
        'x _ 1\n',
        '1 -> x\n',
        'x <<- 1\n',
        'a <- b <- 3\n',
      ]) {
        expect(parses(src), isTrue, reason: 'should parse: $src');
        expect(containsRule(parseS(src), 'assignment'), isTrue, reason: src);
      }
    });

    test('combine call with positional args', () {
      final ast = parseS('c(1, 2, 3)\n');
      expect(containsRule(ast, 'call_suffix'), isTrue);
      final vals = tokenValues(ast);
      expect(vals.contains('c') && vals.contains('3'), isTrue);
    });

    test('named arguments parse', () {
      expect(parses('mean(x, na.rm = TRUE)\n'), isTrue);
      // `==` inside a call is a positional comparison, not a named arg.
      expect(parses('f(x == 1)\n'), isTrue);
    });

    test('empty named argument parses', () {
      expect(parses('switch("a", a = , b = "hit")\n'), isTrue);
      expect(parses('switch("b", a = "A", b = )\n'), isTrue);
      expect(parses('switch("a", a = , b = , c = "z")\n'), isTrue);
      expect(parses('f(x = )\n'), isTrue);
      expect(parses('f(x = 1, y = 2)\n'), isTrue);
    });

    test('arithmetic and precedence nodes', () {
      final ast = parseS('1 + 2 * 3 ^ 2\n');
      for (final rule in ['additive', 'multiplicative', 'power']) {
        expect(containsRule(ast, rule), isTrue, reason: 'missing $rule');
      }
    });

    test('comparison and sequence', () {
      expect(containsRule(parseS('a < b\n'), 'comparison'), isTrue);
      expect(containsRule(parseS('1:10\n'), 'range'), isTrue);
    });

    test('indexing parses', () {
      expect(containsRule(parseS('x[1]\n'), 'index_suffix'), isTrue);
    });

    test('function definition parses', () {
      final ast = parseS('sq <- function(v) v * v\n');
      expect(containsRule(ast, 'func_def'), isTrue);
      expect(containsRule(ast, 'param_list'), isTrue);
    });

    test('function with default argument', () {
      expect(parses('f <- function(x, n = 1) x + n\n'), isTrue);
    });

    test('if else expression', () {
      final ast = parseS('if (x > 0) 1 else -1\n');
      expect(containsRule(ast, 'if_expr'), isTrue);
    });

    test('for and while and repeat', () {
      expect(
        containsRule(parseS('for (i in 1:3) print(i)\n'), 'for_expr'),
        isTrue,
      );
      expect(
        containsRule(parseS('while (x < 10) x <- x + 1\n'), 'while_expr'),
        isTrue,
      );
      expect(containsRule(parseS('repeat break\n'), 'repeat_expr'), isTrue);
    });

    test('multi statement block with newlines', () {
      final ast = parseS('{\n  x <- 1\n  y <- 2\n  x + y\n}\n');
      expect(containsRule(ast, 'block'), isTrue);
    });

    test('semicolons separate statements', () {
      expect(parses('x <- 1; y <- 2; x + y\n'), isTrue);
    });

    test('call spanning multiple lines', () {
      // Interior newlines were dropped by the lexer, so this is one
      // call.
      expect(parses('sum(1,\n    2,\n    3)\n'), isTrue);
    });

    test('trailing newline optional', () {
      expect(parses('mean(x)'), isTrue);
    });

    test('syntax error is reported', () {
      // A dangling binary operator has no right operand.
      expect(tryParseS('1 +\n'), isNull);
    });

    test('the canonical session parses', () {
      expect(
        parses('x <- c(1, 2, 3)\nmean(x)\nx * 10 + c(1, 2)\nsd(x)\n'),
        isTrue,
      );
    });

    test('infix operators parse', () {
      expect(containsRule(parseS('x %% 3\n'), 'special'), isTrue);
      expect(parses('a %in% b\n'), isTrue);
      expect(parses('"%plus%" <- function(a, b) a + b\n'), isTrue);
    });

    test('dollar and double bracket parse', () {
      expect(containsRule(parseS('df\$x\n'), 'dollar_suffix'), isTrue);
      expect(
        containsRule(parseS('df[["x"]]\n'), 'dindex_suffix'),
        isTrue,
      );
      expect(containsRule(parseS('df[1, 2]\n'), 'index_suffix'), isTrue);
    });

    test('precedence fix keeps colon inside arithmetic', () {
      // `1:3+1` must parse (range nested under additive).
      final ast = parseS('1:3+1\n');
      expect(containsRule(ast, 'additive'), isTrue);
      expect(containsRule(ast, 'range'), isTrue);
    });
  });
}
