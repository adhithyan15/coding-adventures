/// Port of the reference `rust/matlab-parser` unit suite (19 tests).
library;

import 'package:coding_adventures_matlab_parser/matlab_parser.dart';
import 'package:coding_adventures_parser/parser.dart';
import 'package:test/test.dart';

bool hasRule(ASTNode node, String name) {
  if (node.ruleName == name) return true;
  return node.children.any((c) => c is ASTNode && hasRule(c, name));
}

int countRule(ASTNode node, String name) {
  var here = node.ruleName == name ? 1 : 0;
  for (final c in node.children) {
    if (c is ASTNode) here += countRule(c, name);
  }
  return here;
}

void parses(String src) {
  expect(tryParseMatlab(src), isNotNull, reason: 'should parse: $src');
}

String nestedParenSource(int n) => '${'(' * n}0${')' * n}\n';

void main() {
  group('depth guard', () {
    test('deeply nested input returns error not overflow', () {
      // 5000 levels — far past the cap — must fail with a clean
      // error, never a StackOverflowError: the guard, not the stack,
      // stops the recursion. (Dart has no thread-stack tuning, so
      // this single in-process case covers the reference worker
      // tests.)
      expect(tryParseMatlab(nestedParenSource(5000)), isNull);
    });

    test('nesting up to cap still parses', () {
      // 12 legitimate levels (the MATLAB cascade re-enters more rules
      // per nesting level than r/derive/s).
      expect(
        tryParseMatlab(nestedParenSource(12)),
        isNotNull,
        reason: '12 levels must stay under the cap',
      );
      expect(
        tryParseMatlab(nestedParenSource(13)),
        isNull,
        reason: 'one nesting level past the cap must fail',
      );
    });
  });

  group('matlab-parser', () {
    test('arithmetic and precedence', () {
      parses('y = 2 + 3 * 4\n');
      parses('z = -2 ^ 2\n'); // unary looser than power
      parses('w = a:b:c\n'); // colon range with step
    });

    test('elementwise and matrix operators', () {
      parses('c = A .* B ./ C\n');
      parses('d = A * B \\ C\n');
      parses('e = A .^ 2\n');
    });

    test('transpose is postfix', () {
      final t = parseMatlab("y = A'\n");
      expect(hasRule(t, 'transpose_suffix'), isTrue);
      parses("z = A.' * b\n"); // non-conjugate transpose
      parses('w = (A + B)\'\n');
    });

    test('comparison and logical', () {
      parses('r = a == b & c ~= d\n');
      parses('s = x > 0 && y < 1 || ~z\n');
    });

    test('matrix literals', () {
      final m = parseMatlab('M = [1 2; 3 4]\n');
      expect(hasRule(m, 'matrix_literal'), isTrue);
      expect(countRule(m, 'matrix_row'), equals(2)); // two rows
      parses('r = [1, 2, 3]\n'); // comma columns
      parses('col = [1; 2; 3]\n'); // semicolon rows
      parses('empty = []\n');
      // A newline inside [ ] separates rows (the lexer keeps it).
      final nl = parseMatlab('M = [1 2\n3 4]\n');
      expect(countRule(nl, 'matrix_row'), equals(2));
    });

    test('cell literal and concatenation', () {
      parses("c = {1, 'two', 3}\n");
      parses('h = [A B]\n'); // horizontal concat (juxtaposition)
    });

    test('calls and indexing', () {
      parses('y = f(x, 2)\n');
      final idx = parseMatlab('v = A(2, 3)\n');
      expect(hasRule(idx, 'call_suffix'), isTrue);
      parses('col = A(:, k)\n'); // whole-column
      parses('s = obj.field\n'); // field access
      parses('z = data.values(3)\n'); // field then index
    });

    test('end is the index sentinel inside brackets', () {
      // `A(end)` and `A(2:end)` parse: the bracketed `end` is retagged
      // to NAME.
      parses('last = A(end)\n');
      parses('tail = A(2:end)\n');
      parses('rev = A(end:-1:1)\n');
    });

    test('if elseif else', () {
      final t = parseMatlab(
          'if x > 0\n  y = 1\nelseif x < 0\n  y = -1\nelse\n  y = 0\nend\n');
      expect(hasRule(t, 'if_stmt'), isTrue);
      expect(hasRule(t, 'elseif_clause'), isTrue);
      expect(hasRule(t, 'else_clause'), isTrue);
    });

    test('for and while loops', () {
      final f = parseMatlab('for i = 1:10\n  s = s + i\nend\n');
      expect(hasRule(f, 'for_stmt'), isTrue);
      parses('while x > 0\n  x = x - 1\nend\n');
      parses(
          'for k = 1:n\n  if k == 3\n    break\n  end\nend\n'); // nested, break
    });

    test('function definitions', () {
      parses('function y = sq(x)\n  y = x .^ 2\nend\n');
      final multi =
          parseMatlab('function [a, b] = two()\n  a = 1\n  b = 2\nend\n');
      expect(hasRule(multi, 'func_returns'), isTrue);
      parses('function noret(x)\n  disp(x)\nend\n');
    });

    test('switch and try', () {
      parses('switch x\ncase 1\n  y = 1\notherwise\n  y = 0\nend\n');
      parses('try\n  risky()\ncatch err\n  handle(err)\nend\n');
    });

    test('anonymous function', () {
      final l = parseMatlab('sq = @(x) x .^ 2\n');
      expect(hasRule(l, 'lambda'), isTrue);
    });

    test('semicolons and commas separate statements', () {
      parses('a = 1; b = 2; c = 3\n'); // semicolons suppress display
      parses('x = 1, y = 2\n'); // comma separator
      // A trailing `;` is recorded as the statement terminator.
      final t = parseMatlab('a = 1;\n');
      expect(hasRule(t, 'stmt_term'), isTrue);
    });

    test('unclosed bracket is an error', () {
      expect(tryParseMatlab('x = [1 2\n'), isNull);
    });

    test('dangling operator is an error', () {
      expect(tryParseMatlab('y = 1 +\n'), isNull);
    });
  });
}
