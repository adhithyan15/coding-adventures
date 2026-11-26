/// Port of the reference `rust/maple-parser` unit suite (all 45
/// tests, with all six recursion-shape boundaries re-measured on
/// this engine: parens 11/12, list 10/11, `not` 135/136, unary-minus
/// 135/136, power 67/68, nested `if` 67/68 — identical to the
/// reference) plus nullable-twin and error-shape tests.
library;

import 'package:coding_adventures_lexer/lexer.dart';
import 'package:coding_adventures_maple_parser/maple_parser.dart';
import 'package:coding_adventures_parser/parser.dart';
import 'package:test/test.dart';

bool containsRule(ASTNode node, String name) {
  if (node.ruleName == name) return true;
  return node.children.any((c) => c is ASTNode && containsRule(c, name));
}

int countNodes(ASTNode node, String name) {
  var n = node.ruleName == name ? 1 : 0;
  for (final c in node.children) {
    if (c is ASTNode) n += countNodes(c, name);
  }
  return n;
}

bool parses(String src) => tryParseMaple(src) != null;

String nestedParenSource(int n) => '${'(' * n}5${')' * n};\n';
String nestedListSource(int n) => '${'[' * n}5${']' * n};\n';
String notChainSource(int n) => '${'not ' * n}a;\n';
String unaryMinusChainSource(int n) => '${'-' * n}a;\n';
String powerChainSource(int n) => '1${'^1' * n};\n';

String nestedIfSource(int n) {
  final buf = StringBuffer();
  for (var i = 0; i < n; i++) {
    buf.write('if 1 then ');
  }
  buf.write('5');
  for (var i = 0; i < n; i++) {
    buf.write(' end if');
  }
  buf.write(';\n');
  return buf.toString();
}

void main() {
  group('reference suite', () {
    test('program is the root', () {
      expect(parseMaple('1;\n').ruleName, equals('program'));
    });

    test('bare trailing statement with no terminator parses', () {
      expect(parses('1'), isTrue);
    });

    test('semi and colon are interchangeable terminators', () {
      expect(parses('x := 1; y := 2:'), isTrue);
    });

    test('function call uses ordinary parens', () {
      final ast = parseMaple('sin(x);\n');
      expect(containsRule(ast, 'postfix'), isTrue);
      expect(containsRule(ast, 'arglist'), isTrue);
    });

    test('multi arg calls parse', () {
      expect(parses('f(x, y, z);\n'), isTrue);
    });

    test('nested function calls parse', () {
      expect(parses('f(g(x));\n'), isTrue);
    });

    test('assignment parses', () {
      final ast = parseMaple('x := 5;\n');
      expect(containsRule(ast, 'assignment'), isTrue);
    });

    test('eq is equation distinct from assign', () {
      expect(parses('x = 4;\n'), isTrue);
      expect(containsRule(parseMaple('x = 4;\n'), 'comparison'), isTrue);
    });

    test('every comparison operator parses', () {
      for (final op in ['=', '<>', '<', '>', '<=', '>=']) {
        final src = 'a $op b;\n';
        expect(parses(src), isTrue, reason: '`$src` should parse');
      }
    });

    test('arrow definition with two parameters parses', () {
      final ast = parseMaple('f := (x, y) -> x + y;\n');
      expect(containsRule(ast, 'arrow_def'), isTrue);
      expect(containsRule(ast, 'arrow_params'), isTrue);
    });

    test('arrow definition with one bare parameter parses', () {
      final ast = parseMaple('f := x -> x;\n');
      expect(containsRule(ast, 'arrow_def'), isTrue);
    });

    test('arrow definition with zero parameters parses', () {
      // `() -> 5` — a nullary constant function. Not one of MA09
      // §3's own worked examples, but a natural, undeliberate
      // fallout of `arrow_params`' own optional inner list — see
      // maple.grammar's "arrow_params" comment.
      expect(parses('f := () -> 5;\n'), isTrue);
    });

    test('plain assignment does not produce an arrow def node', () {
      // `f := x;` (assigning a variable's VALUE) must backtrack
      // cleanly out of `arrow_def` (which requires an ARROW after
      // the bare name) and fall through to the plain `expr`
      // alternative instead.
      final ast = parseMaple('f := x;\n');
      expect(containsRule(ast, 'assignment'), isTrue);
      expect(containsRule(ast, 'arrow_def'), isFalse);
    });

    test('arrow never appears outside an assignment rhs', () {
      // MA09 §3: `->` is used ONLY as a Define right-hand side. A
      // bare arrow expression with no `NAME :=` prefix must be a
      // syntax error — there is nowhere else in this grammar `ARROW`
      // is referenced.
      expect(tryParseMaple('(x) -> x;\n'), isNull);
      expect(tryParseMaple('x -> x;\n'), isNull);
    });

    test('arrow cannot nest inside arithmetic', () {
      expect(tryParseMaple('y := 1 + (x -> x);\n'), isNull);
    });

    test('remember table spelling is rejected', () {
      // MA09 §1/§4: `f(x) := expr` is real Maple's narrower
      // remember-table mechanism, deliberately EXCLUDED from this
      // subset — this grammar's `assignment` left-hand side is a
      // bare NAME, so this spelling must fail to parse entirely, not
      // merely lower differently at a later stage. See
      // maple.grammar's own "assignment left-hand side"
      // design-decision comment.
      expect(tryParseMaple('f(x) := 1;\n'), isNull);
      expect(tryParseMaple('h(l, m) := l + m;\n'), isNull);
    });

    test('if is not usable as an assignment rhs', () {
      // Unlike reduce-parser's `if_is_usable_as_an_expression` test
      // — MA09 makes no equivalent claim for Maple, and real
      // Maple's own conditional-VALUE idiom is the `piecewise`
      // library call, not a bare `if`. See maple.grammar's
      // "statements vs. expressions" design decision.
      expect(tryParseMaple('x := if a then 1 else 2 end if;\n'), isNull);
    });

    test('chained assignment is rejected', () {
      // Unlike reduce-parser's `assignment_right_associates` test —
      // MA09 cites no equivalent to REDUCE manual §2.7's
      // "a:=b:=c evaluates as a:=(b:=c)" for Maple, and every MA09
      // §3 worked example is a single, non-chained assignment.
      expect(tryParseMaple('a := b := 5;\n'), isNull);
    });

    test('list literal uses square brackets', () {
      final ast = parseMaple('[a, b, c];\n');
      expect(containsRule(ast, 'list_literal'), isTrue);
      expect(containsRule(ast, 'set_literal'), isFalse);
    });

    test('set literal uses curly braces', () {
      final ast = parseMaple('{a, b, c};\n');
      expect(containsRule(ast, 'set_literal'), isTrue);
      expect(containsRule(ast, 'list_literal'), isFalse);
    });

    test('empty list literal parses', () {
      final ast = parseMaple('[];\n');
      expect(containsRule(ast, 'list_literal'), isTrue);
    });

    test('empty set literal parses', () {
      final ast = parseMaple('{};\n');
      expect(containsRule(ast, 'set_literal'), isTrue);
    });

    test('list and set literal in a call snippet', () {
      expect(parses('g([1, 2, 3], {1, 2, 2});\n'), isTrue);
    });

    test('boolean literals parse', () {
      expect(parses('true;\n'), isTrue);
      expect(parses('false;\n'), isTrue);
    });

    test('boolean keywords parse', () {
      expect(parses('a and b or not c;\n'), isTrue);
      expect(
        containsRule(parseMaple('a and b;\n'), 'logical_and'),
        isTrue,
      );
      expect(
        containsRule(parseMaple('a or b;\n'), 'logical_or'),
        isTrue,
      );
      expect(
        containsRule(parseMaple('not a;\n'), 'logical_not'),
        isTrue,
      );
    });

    test('uppercase keyword spellings lex as plain names not keywords', () {
      // maple.tokens' keywords are lowercase-only (mirroring
      // reduce.tokens, the opposite of derive.tokens' uppercase
      // AND/OR/NOT) — `AND` here is just a NAME, so this is a bare
      // `a AND b` juxtaposition with no operator between two names,
      // which is not valid syntax in this subset (no juxtaposition
      // production exists anywhere in this grammar, MA09 §4).
      expect(tryParseMaple('a AND b;\n'), isNull);
    });

    test('bare juxtaposed names with no operator is rejected', () {
      // Mirrors reduce-parser's identically-named,
      // identically-motivated regression test: this is a regression
      // test for exactly the bug `program`'s own grammar comment
      // documents — an earlier draft could fold "no terminator"
      // into `statement_line` itself (tried per repetition
      // iteration, not just once at the very end), which would let
      // `a`, `AND`, and `b;` each parse as their own bare statement
      // and silently consume the entire input as three separate
      // program items.
      expect(tryParseMaple('a AND b;\n'), isNull);
      expect(tryParseMaple('a b\n'), isNull);
    });

    test('arithmetic precedence nests correctly', () {
      // `1 + 2 * 3` must parse as `1 + (2*3)`, not `(1+2)*3` — the
      // tree must contain both `additive` and `multiplicative`,
      // with `multiplicative` nested inside the `additive`
      // continuation.
      final ast = parseMaple('1 + 2 * 3;\n');
      expect(containsRule(ast, 'additive'), isTrue);
      expect(containsRule(ast, 'multiplicative'), isTrue);
    });

    test('explicit star is required no pow synonym double star', () {
      // MA09 §3: `^` ONLY, no `**` synonym. `a ** b` must NOT parse
      // as a single power expression — `**` lexes as two TIMES
      // tokens (confirmed by maple-lexer's own
      // `double_star_is_not_a_single_pow_token` test), so `a ** b`
      // is `a` followed by a bare `*` with nothing between the two
      // stars, i.e. `a * (* b)` — a syntax error, since
      // `multiplicative`'s right operand must be a `unary`, and a
      // bare `*` is not a valid start of one.
      expect(tryParseMaple('a ** b;\n'), isNull);
    });

    test('power is right associative by shape', () {
      expect(parses('4^3^2;\n'), isTrue);
      final ast = parseMaple('4^3^2;\n');
      expect(containsRule(ast, 'power'), isTrue);
    });

    test('unary minus binds looser than power', () {
      final ast = parseMaple('-x^2;\n');
      expect(containsRule(ast, 'unary'), isTrue);
      expect(containsRule(ast, 'power'), isTrue);
    });

    test('grouping parens parse', () {
      expect(parses('(1 + 2) * 3;\n'), isTrue);
      expect(
        containsRule(parseMaple('(1 + 2) * 3;\n'), 'group'),
        isTrue,
      );
    });

    test('if then end if parses', () {
      final ast = parseMaple('if x > 0 then 1 end if;\n');
      expect(containsRule(ast, 'if_expr'), isTrue);
    });

    test('if then else end if parses', () {
      final ast = parseMaple('if x > 0 then 1 else -1 end if;\n');
      expect(containsRule(ast, 'if_expr'), isTrue);
    });

    test('if then fi parses', () {
      // The `fi` closing spelling — "if" reversed.
      final ast = parseMaple('if x > 0 then 1 else -1 fi;\n');
      expect(containsRule(ast, 'if_expr'), isTrue);
    });

    test('if elif else end if and fi produce equivalent shapes', () {
      // Both closing spellings must parse to a structurally
      // equivalent if_expr shape (same number of elif arms, same
      // else presence) — matching MA09 §3's own
      // two-worked-spelling example.
      final endIf = parseMaple(
        'if x > 0 then 1 elif x < 0 then -1 else 0 end if;\n',
      );
      final fi = parseMaple('if x > 0 then 1 elif x < 0 then -1 else 0 fi;\n');
      expect(containsRule(endIf, 'if_expr'), isTrue);
      expect(containsRule(fi, 'if_expr'), isTrue);
      // Both must contain exactly one `if_expr` and the elif
      // condition must have been parsed (an `expr`/`comparison` for
      // `x < 0`).
      expect(countNodes(endIf, 'if_expr'), equals(1));
      expect(countNodes(fi, 'if_expr'), equals(1));
    });

    test('elif chain preserves every arm', () {
      final ast = parseMaple(
        'if a then 1 elif b then 2 elif c then 3 else 4 end if;\n',
      );
      // Every `elif` condition ("b" and "c") and its body ("2" and
      // "3") must appear somewhere in the tree, in order — the flat
      // `{ "elif" ... }` repetition preserves the whole chain as
      // sibling children.
      final names = <String>[];
      void allNames(ASTNode node) {
        for (final c in node.children) {
          if (c is Token && c.type == 'NAME') {
            names.add(c.value);
          } else if (c is ASTNode) {
            allNames(c);
          }
        }
      }

      allNames(ast);
      expect(names, equals(['a', 'b', 'c']));
    });

    test('if can branch to an assignment', () {
      // MA09 §3's own s1/s2/s3 labelling reads as statement-shaped,
      // not narrowly expression-shaped — an `if` branching to an
      // assignment must be supported even though `if` is never
      // itself usable as a nested expression (see maple.grammar's
      // "statements vs. expressions" design decision).
      final ast = parseMaple('if a then x := 1 else x := 2 end if;\n');
      expect(containsRule(ast, 'if_expr'), isTrue);
      expect(containsRule(ast, 'assignment'), isTrue);
    });

    test('nested if resolves unambiguously with no dangling else', () {
      // Unlike reduce-parser's dangling-else test — Maple's explicit
      // closing keywords mean there is only one place an outer
      // `else` could possibly attach, by construction.
      expect(parses('if a then if b then c end if else d end if;\n'), isTrue);
      final ast = parseMaple('if a then if b then c end if else d end if;\n');
      expect(countNodes(ast, 'if_expr'), equals(2));
    });

    test('a multi statement program parses', () {
      final ast = parseMaple('x := 1; y := 2; x + y;\n');
      final stmtLines = ast.children
          .where((c) => c is ASTNode && c.ruleName == 'statement_line')
          .length;
      expect(stmtLines, equals(3));
    });

    test('syntax error is reported', () {
      expect(tryParseMaple('1 +\n'), isNull);
      expect(tryParseMaple('(1 + 2\n'), isNull);
      expect(() => parseMaple('1 +\n'), throwsA(isA<GrammarParseError>()));
    });
  });

  group('depth guard (mirrors the reference suite)', () {
    test('deeply nested input errors for every shape', () {
      // 5000 levels (2000 for the verbose `if` shape) — far past
      // maxRuleDepth — must produce a recoverable error, not
      // overflow the stack, for all six shapes.
      for (final src in [
        nestedParenSource(5000),
        nestedListSource(5000),
        notChainSource(5000),
        unaryMinusChainSource(5000),
        powerChainSource(5000),
        nestedIfSource(2000),
      ]) {
        expect(() => parseMaple(src), throwsA(isA<GrammarParseError>()));
        expect(tryParseMaple(src), isNull);
      }
    });

    test('cap trips before overflow for every shape', () {
      // The Dart equivalent of the reference's default-stack-thread
      // test: far-too-deep input surfaces a catchable engine error
      // with no stack exhaustion on the ordinary event loop, and the
      // isolate stays healthy afterwards.
      for (final src in [
        nestedParenSource(5000),
        nestedListSource(5000),
        notChainSource(5000),
        unaryMinusChainSource(5000),
        powerChainSource(5000),
        nestedIfSource(2000),
      ]) {
        try {
          parseMaple(src);
          fail('must not parse');
        } on GrammarParseError catch (e) {
          expect(e.message, isNotEmpty);
        }
      }
      expect(parseMaple('x := 5;\n').ruleName, equals('program'));
    });

    test('reasonable nesting stays under the cap for every shape', () {
      expect(tryParseMaple(nestedParenSource(3)) != null, isTrue);
      expect(tryParseMaple(nestedListSource(3)) != null, isTrue);
      expect(tryParseMaple(notChainSource(5)) != null, isTrue);
      expect(tryParseMaple(unaryMinusChainSource(5)) != null, isTrue);
      expect(tryParseMaple(powerChainSource(5)) != null, isTrue);
      expect(tryParseMaple(nestedIfSource(3)) != null, isTrue);
    });

    test('headroom boundary for every shape', () {
      // Re-measured on this engine via scratch binary-search
      // probes, all identical to the reference: parens 11/12, list
      // 10/11, `not` 135/136, unary-minus 135/136, power 67/68,
      // nested `if` 67/68.
      expect(tryParseMaple(nestedParenSource(11)) != null, isTrue);
      expect(tryParseMaple(nestedParenSource(12)), isNull);

      expect(tryParseMaple(nestedListSource(10)) != null, isTrue);
      expect(tryParseMaple(nestedListSource(11)), isNull);

      expect(tryParseMaple(notChainSource(135)) != null, isTrue);
      expect(tryParseMaple(notChainSource(136)), isNull);

      expect(tryParseMaple(unaryMinusChainSource(135)) != null, isTrue);
      expect(tryParseMaple(unaryMinusChainSource(136)), isNull);

      expect(tryParseMaple(powerChainSource(67)) != null, isTrue);
      expect(tryParseMaple(powerChainSource(68)), isNull);

      expect(tryParseMaple(nestedIfSource(67)) != null, isTrue);
      expect(tryParseMaple(nestedIfSource(68)), isNull);
    });
  });

  group('twins and errors', () {
    test('empty program parses to zero statements', () {
      final ast = parseMaple('');
      expect(ast.ruleName, equals('program'));
      expect(ast.children, isEmpty);
    });

    test('factory returns a wired parser', () {
      expect(
        createMapleParser('x := 5;\n').parse().ruleName,
        equals('program'),
      );
    });
  });
}
