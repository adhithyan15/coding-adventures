/// Port of the reference `rust/r-parser` unit suite (16 tests).
library;

import 'package:coding_adventures_lexer/lexer.dart';
import 'package:coding_adventures_parser/parser.dart';
import 'package:coding_adventures_r_parser/r_parser.dart';
import 'package:test/test.dart';

bool containsRule(ASTNode node, String name) {
  if (node.ruleName == name) return true;
  return node.children.any((c) => c is ASTNode && containsRule(c, name));
}

/// The value of the first operator/keyword token anywhere under
/// [node] belonging to a node whose rule is [rule].
String? firstTokenOf(ASTNode node, String rule) {
  String? tok(ASTNode n) {
    for (final c in n.children) {
      if (c is Token) return c.value;
    }
    return null;
  }

  String? search(ASTNode n) {
    if (n.ruleName == rule) {
      final t = tok(n);
      if (t != null) return t;
    }
    for (final c in n.children) {
      if (c is ASTNode) {
        final t = search(c);
        if (t != null) return t;
      }
    }
    return null;
  }

  return search(node);
}

bool parses(String src) => tryParseR(src) != null;

String nestedParenSource(int n) => '${'(' * n}1${')' * n}\n';

void main() {
  group('depth guard', () {
    test('deeply nested input returns error not overflow', () {
      // 5000 levels — far past the cap — must fail with a clean
      // error, never a StackOverflowError: the guard, not the stack,
      // stops the recursion. (Dart has no thread-stack tuning, so
      // this single in-process case covers all three reference worker
      // tests.)
      expect(tryParseR(nestedParenSource(5000)), isNull);
    });

    test('nesting up to cap still parses', () {
      expect(
        tryParseR(nestedParenSource(14)),
        isNotNull,
        reason: '14 levels must stay under the cap',
      );
      expect(
        tryParseR(nestedParenSource(15)),
        isNull,
        reason: 'one nesting level past the cap must fail',
      );
    });
  });

  group('r-parser', () {
    test('program is the root', () {
      expect(parseR('1\n').ruleName, equals('program'));
    });

    test('underscore name parses as one identifier', () {
      // The R difference: `data_frame` is one name, used here as an
      // lvalue.
      expect(parses('data_frame <- 1\n'), isTrue);
    });

    test('r assignment operators parse', () {
      for (final src in [
        'x <- 1\n',
        'x = 1\n', // R: `=` is assignment
        'x <<- 1\n',
        '1 -> x\n',
        '1 ->> x\n', // R: right super-assignment
      ]) {
        expect(parses(src), isTrue, reason: 'should parse: $src');
        expect(containsRule(parseR(src), 'assignment'), isTrue, reason: src);
      }
    });

    test('equals assignment uses the eq token', () {
      // `x = 1` should match the assignment rule with the `=` operator.
      expect(firstTokenOf(parseR('x = 1\n'), 'assignment'), equals('='));
    });

    test('named argument still distinct from assignment', () {
      // Inside a call, `mean(x = 1)` is a named argument (tried before
      // the positional `expr` alternative), not a nested assignment
      // statement.
      final ast = parseR('mean(x = 1)\n');
      expect(containsRule(ast, 'arg_list'), isTrue);
      expect(parses('f(x == 1)\n'), isTrue);
    });

    test('empty named argument parses', () {
      // A named argument may omit its value, which `switch`'s
      // empty-arm fall-through relies on.
      expect(parses('switch("a", a = , b = "hit")\n'), isTrue);
      expect(parses('switch("b", a = "A", b = )\n'), isTrue);
      expect(parses('switch("a", a = , b = , c = "z")\n'), isTrue);
      // Empty value parses in any call (eval rejects it outside
      // switch).
      expect(parses('f(x = )\n'), isTrue);
      expect(parses('f(x = 1, y = 2)\n'), isTrue);
    });

    test('typed na constants parse', () {
      for (final src in ['NA_integer_\n', 'NA_real_\n', 'NA_character_\n']) {
        expect(parses(src), isTrue, reason: 'should parse: $src');
      }
    });

    test('shared with s constructs parse', () {
      // Arithmetic precedence cascade, sequence, infix, indexing, $,
      // calls.
      final ast = parseR('y <- 1:3 + 2 * 3 ^ 2\n');
      for (final rule in ['additive', 'multiplicative', 'power', 'range']) {
        expect(containsRule(ast, rule), isTrue, reason: 'missing $rule');
      }
      expect(containsRule(parseR('x %in% c(1, 2)\n'), 'special'), isTrue);
      expect(containsRule(parseR('x[1]\n'), 'index_suffix'), isTrue);
      expect(containsRule(parseR('x[[1]]\n'), 'dindex_suffix'), isTrue);
      expect(containsRule(parseR('df\$col\n'), 'dollar_suffix'), isTrue);
      expect(containsRule(parseR('c(1, 2, 3)\n'), 'call_suffix'), isTrue);
    });

    test('functions and control flow parse', () {
      expect(
        containsRule(parseR('sq <- function(v) v * v\n'), 'func_def'),
        isTrue,
      );
      expect(
        containsRule(parseR('f <- function(x, n = 1) x + n\n'), 'param_list'),
        isTrue,
      );
      expect(
        containsRule(parseR('if (x > 0) 1 else -1\n'), 'if_expr'),
        isTrue,
      );
      expect(
        containsRule(parseR('for (i in 1:3) print(i)\n'), 'for_expr'),
        isTrue,
      );
      expect(
        containsRule(parseR('while (x < 10) x <- x + 1\n'), 'while_expr'),
        isTrue,
      );
      expect(containsRule(parseR('repeat break\n'), 'repeat_expr'), isTrue);
      expect(
        containsRule(parseR('{\n  a <- 1\n  a + 1\n}\n'), 'block'),
        isTrue,
      );
    });

    test('multi line and semicolons', () {
      expect(parses('sum(1,\n    2,\n    3)\n'), isTrue);
      expect(parses('x <- 1; y <- 2; x + y\n'), isTrue);
      expect(parses('mean(x)'), isTrue); // trailing newline optional
    });

    test('syntax error is reported', () {
      expect(tryParseR('1 +\n'), isNull);
    });

    test('the canonical r session parses', () {
      expect(
        parses(
            'data_frame <- c(1, 2, 3)\nmean(data_frame)\ndata_frame * 10 + c(1, 2)\n'),
        isTrue,
      );
    });

    test('pipe and lambda parse', () {
      // The pipe produces a `pipe` node; the backslash lambda a
      // `func_def`.
      expect(containsRule(parseR('x |> f()\n'), 'pipe'), isTrue);
      expect(parses('c(3, 1, 2) |> sort() |> rev()\n'), isTrue);
      expect(
        containsRule(parseR('sq <- \\(x) x ^ 2\n'), 'func_def'),
        isTrue,
      );
      expect(parses('sapply(1:3, \\(n) n * n)\n'), isTrue);
    });
  });
}
