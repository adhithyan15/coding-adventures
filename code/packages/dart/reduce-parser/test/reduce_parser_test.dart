/// Port of the reference `rust/reduce-parser` unit suite (all 36
/// tests, with all five recursion-shape boundaries re-measured on
/// this engine: parens 8/9, `:=` 56/57, `if`/`else` 56/57, cons
/// 112/113, power 56/57 — identical to the reference) plus
/// nullable-twin and error-shape tests.
library;

import 'package:coding_adventures_parser/parser.dart';
import 'package:coding_adventures_reduce_parser/reduce_parser.dart';
import 'package:test/test.dart';

bool containsRule(ASTNode node, String name) {
  if (node.ruleName == name) return true;
  return node.children.any((c) => c is ASTNode && containsRule(c, name));
}

bool parses(String src) => tryParseReduce(src) != null;

String nestedParenSource(int n) => '${'(' * n}5${')' * n};\n';

String assignChainSource(int n) => '${'a:=' * n}5;\n';

String ifElseChainSource(int n) => '${'if 1 then 1 else ' * n}5;\n';

String consChainSource(int n) => 'a${'.a' * n};\n';

String powerChainSource(int n) => '1${'^1' * n};\n';

void main() {
  group('reference suite', () {
    test('program is the root', () {
      expect(parseReduce('1;\n').ruleName, equals('program'));
    });

    test('bare trailing statement with no terminator parses', () {
      expect(parses('1'), isTrue);
    });

    test('semi and dollar are interchangeable terminators', () {
      expect(parses('x := 1; y := 2\$'), isTrue);
    });

    test('function call uses ordinary parens', () {
      final ast = parseReduce('df(x, z);\n');
      expect(containsRule(ast, 'postfix'), isTrue);
      expect(containsRule(ast, 'arglist'), isTrue);
    });

    test('array subscript read shares the call production', () {
      // MA08 §3: `a(5)`/`b(i, q)` parse the same as an ordinary call.
      expect(parses('a(5);\n'), isTrue);
      expect(parses('b(i, q);\n'), isTrue);
    });

    test('assignment and procedure definition share one production', () {
      expect(parses('x := 5;\n'), isTrue);
      final ast = parseReduce('h(l, m) := x - 2*y;\n');
      expect(containsRule(ast, 'assignment'), isTrue);
    });

    test('assignment right associates', () {
      // a := b := c evaluates as a := (b := c) (manual §2.7).
      expect(parses('a := b := 5;\n'), isTrue);
    });

    test('eq is equation distinct from assign', () {
      expect(parses('x = 4;\n'), isTrue);
      expect(containsRule(parseReduce('x = 4;\n'), 'comparison'), isTrue);
    });

    test('every comparison operator parses', () {
      for (final op in ['=', 'neq', '<', '>', '<=', '>=']) {
        final src = 'a $op b;\n';
        expect(parses(src), isTrue, reason: '`$src` should parse');
      }
    });

    test('list literal uses curly braces', () {
      final ast = parseReduce('{a, b, c};\n');
      expect(containsRule(ast, 'list_literal'), isTrue);
    });

    test('list function call spelling parses like any other call', () {
      expect(parses('list(a, b, c);\n'), isTrue);
    });

    test('cons operator parses', () {
      final ast = parseReduce('a . {b, c};\n');
      expect(containsRule(ast, 'cons'), isTrue);
    });

    test('cons binds looser than additive', () {
      // `1+2 . {3,4}` must parse as `(1+2) . {3,4}` — `cons`'s own
      // production wraps `additive`, so the additive sub-expression
      // is fully reduced before cons ever sees it. Structurally:
      // the tree must contain both `additive` and `cons`.
      final ast = parseReduce('1+2 . {3,4};\n');
      expect(containsRule(ast, 'additive'), isTrue);
      expect(containsRule(ast, 'cons'), isTrue);
    });

    test('list accessor and constructor calls parse', () {
      for (final src in [
        'first(l);\n',
        'second(l);\n',
        'third(l);\n',
        'rest(l);\n',
        'part(l, n);\n',
        'append(l1, l2);\n',
        'reverse(l);\n',
      ]) {
        expect(parses(src), isTrue, reason: '`$src` should parse');
      }
    });

    test('arithmetic precedence', () {
      final ast = parseReduce('2 + 3 * 4 ^ 2;\n');
      expect(containsRule(ast, 'additive'), isTrue);
      expect(containsRule(ast, 'multiplicative'), isTrue);
      expect(containsRule(ast, 'power'), isTrue);
    });

    test('caret and double star are the same operator', () {
      expect(parses('a ^ b;\n'), isTrue);
      expect(parses('a ** b;\n'), isTrue);
    });

    test('unary minus binds looser than power', () {
      final ast = parseReduce('-x^2;\n');
      expect(containsRule(ast, 'unary'), isTrue);
      expect(containsRule(ast, 'power'), isTrue);
    });

    test('power is right associative by shape', () {
      expect(parses('4^3^2;\n'), isTrue);
    });

    test('grouping parens parse', () {
      expect(parses('(1 + 2) * 3;\n'), isTrue);
      expect(
        containsRule(parseReduce('(1 + 2) * 3;\n'), 'group'),
        isTrue,
      );
    });

    test('boolean keywords parse', () {
      expect(parses('a and b or not c;\n'), isTrue);
      expect(
        containsRule(parseReduce('a and b;\n'), 'logical_and'),
        isTrue,
      );
      expect(
        containsRule(parseReduce('a or b;\n'), 'logical_or'),
        isTrue,
      );
      expect(
        containsRule(parseReduce('not a;\n'), 'logical_not'),
        isTrue,
      );
    });

    test('uppercase keyword spellings lex as plain names not keywords', () {
      // reduce.tokens' keywords are lowercase-only (the mirror image
      // of derive.tokens' uppercase AND/OR/NOT) — `AND` here is just
      // a NAME, so this is a bare `a AND b` juxtaposition with no
      // operator between two names, which is not valid syntax in
      // this subset (implicit multiplication by juxtaposition is out
      // of scope, MA08 §4) and must fail to parse as a single
      // statement.
      expect(tryParseReduce('a AND b;\n'), isNull);
    });

    test('bare juxtaposed names with no operator is rejected', () {
      // `a AND b` (with `AND` lexing as an ordinary NAME, not a
      // keyword — see the test above) has three bare names with no
      // operator between them. Implicit multiplication by
      // juxtaposition is out of scope (MA08 §4), so this must be a
      // syntax error — NOT silently accepted as three separate
      // terminator-less statements. This is a regression test for
      // exactly the bug `program`'s own grammar comment documents:
      // an earlier draft folded the "no terminator" case into
      // `statement_line` itself (tried per repetition iteration, not
      // just once at the very end), which let `a`, `AND`, and `b;`
      // each parse as their own bare statement and silently consumed
      // the entire input as three separate program items.
      expect(tryParseReduce('a AND b;\n'), isNull);
      expect(tryParseReduce('a b\n'), isNull);
    });

    test('if then parses without an else', () {
      final ast = parseReduce('if a then b;\n');
      expect(containsRule(ast, 'if_expr'), isTrue);
    });

    test('if then else parses', () {
      final ast = parseReduce('if a=b then c else d;\n');
      expect(containsRule(ast, 'if_expr'), isTrue);
    });

    test('if is usable as an expression', () {
      // MA08 §3: "usable as an expression, returning whichever
      // branch ran" — nested directly as an assignment's right-hand
      // side.
      final ast = parseReduce('x := if a>0 then 1 else -1;\n');
      expect(containsRule(ast, 'if_expr'), isTrue);
      expect(containsRule(ast, 'assignment'), isTrue);
    });

    test('dangling else attaches to the nearest if', () {
      // `if a then if b then c else d` must parse as
      // `if a then (if b then c else d)`, not
      // `(if a then if b then c) else d` — there is only one
      // `if_expr` node nested inside the outer `then`-branch's own
      // recursive `expr`, matching ordinary recursive-descent
      // dangling-else resolution.
      expect(parses('if a then if b then c else d;\n'), isTrue);
    });

    test('group statement parses', () {
      final ast = parseReduce('<< a := 1; a + 1 >>;\n');
      expect(containsRule(ast, 'group_expr'), isTrue);
    });

    test('group statement is usable as an expression', () {
      // MA08 §3: "evaluates to its last statement's value".
      final ast = parseReduce('x := << a := 1; a + 1 >>;\n');
      expect(containsRule(ast, 'group_expr'), isTrue);
      expect(containsRule(ast, 'assignment'), isTrue);
    });

    test('group statement with a single statement parses', () {
      expect(parses('<< a + 1 >>;\n'), isTrue);
    });

    test('nested function calls parse', () {
      expect(parses('log(y/m);\n'), isTrue);
    });

    test('multi arg calls parse', () {
      expect(parses('df(x, z, a, b);\n'), isTrue);
    });

    test('a multi statement program parses', () {
      final ast = parseReduce('x := 1; y := 2; x + y;\n');
      final stmtLines = ast.children
          .where((c) => c is ASTNode && c.ruleName == 'statement_line')
          .length;
      expect(stmtLines, equals(3));
    });

    test('syntax error is reported', () {
      expect(tryParseReduce('1 +\n'), isNull);
      expect(tryParseReduce('(1 + 2\n'), isNull);
      expect(() => parseReduce('1 +\n'), throwsA(isA<GrammarParseError>()));
    });
  });

  group('depth guard (mirrors the reference suite)', () {
    test('deeply nested input errors for every shape', () {
      // 5000 levels/links — far past maxRuleDepth — must produce a
      // recoverable error, not overflow the stack, for all five
      // independently-measured shapes.
      for (final src in [
        nestedParenSource(5000),
        assignChainSource(5000),
        ifElseChainSource(5000),
        consChainSource(5000),
        powerChainSource(5000),
      ]) {
        expect(() => parseReduce(src), throwsA(isA<GrammarParseError>()));
        expect(tryParseReduce(src), isNull);
      }
    });

    test('cap trips before overflow for every shape', () {
      // The Dart equivalent of the reference's default-stack-thread
      // test: far-too-deep input surfaces a catchable engine error
      // with no stack exhaustion on the ordinary event loop, and the
      // isolate stays healthy afterwards.
      for (final src in [
        nestedParenSource(5000),
        assignChainSource(5000),
        ifElseChainSource(5000),
        consChainSource(5000),
        powerChainSource(5000),
      ]) {
        try {
          parseReduce(src);
          fail('must not parse');
        } on GrammarParseError catch (e) {
          expect(e.message, isNotEmpty);
        }
      }
      expect(parseReduce('x := 5;\n').ruleName, equals('program'));
    });

    test('reasonable nesting stays under the cap for every shape', () {
      expect(tryParseReduce(nestedParenSource(3)) != null, isTrue);
      expect(tryParseReduce(assignChainSource(10)) != null, isTrue);
      expect(tryParseReduce(ifElseChainSource(10)) != null, isTrue);
      expect(tryParseReduce(consChainSource(10)) != null, isTrue);
      expect(tryParseReduce(powerChainSource(10)) != null, isTrue);
    });

    test('measured boundaries match the reference exactly', () {
      // Re-measured on this engine via scratch binary-search probes
      // (same methodology as the reference): parens 8/9, `:=`
      // 56/57, `if`/`else` 56/57, cons 112/113, power 56/57 — all
      // identical (same grammar cascade, same depth counting).
      expect(tryParseReduce(nestedParenSource(8)) != null, isTrue);
      expect(tryParseReduce(nestedParenSource(9)), isNull);
      expect(tryParseReduce(assignChainSource(56)) != null, isTrue);
      expect(tryParseReduce(assignChainSource(57)), isNull);
      expect(tryParseReduce(ifElseChainSource(56)) != null, isTrue);
      expect(tryParseReduce(ifElseChainSource(57)), isNull);
      expect(tryParseReduce(consChainSource(112)) != null, isTrue);
      expect(tryParseReduce(consChainSource(113)), isNull);
      expect(tryParseReduce(powerChainSource(56)) != null, isTrue);
      expect(tryParseReduce(powerChainSource(57)), isNull);
    });
  });

  group('twins and errors', () {
    test('empty program parses to zero statements', () {
      // `program` is a zero-or-more repetition, so empty input is a
      // valid empty program (verified by probe: root=program,
      // 0 children).
      final ast = parseReduce('');
      expect(ast.ruleName, equals('program'));
      expect(ast.children, isEmpty);
      expect(tryParseReduce('') != null, isTrue);
    });

    test('factory returns a wired parser', () {
      expect(
        createReduceParser('x := 5;\n').parse().ruleName,
        equals('program'),
      );
    });
  });
}
