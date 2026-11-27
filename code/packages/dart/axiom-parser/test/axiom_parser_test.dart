/// Port of the reference `rust/axiom-parser` unit suite (all 58
/// tests, with all four recursion-shape boundaries re-measured on
/// this engine: parens 13/14, nested calls 13/14, unary chain
/// 130/131, power 65/66 — identical to the reference) plus
/// nullable-twin and error-shape tests.
library;

import 'package:coding_adventures_axiom_parser/axiom_parser.dart';
import 'package:coding_adventures_parser/parser.dart';
import 'package:test/test.dart';

bool containsRule(ASTNode node, String name) {
  if (node.ruleName == name) return true;
  return node.children.any((c) => c is ASTNode && containsRule(c, name));
}

/// Find the first descendant node (or self) with the given rule name.
ASTNode? findRule(ASTNode node, String name) {
  if (node.ruleName == name) return node;
  for (final c in node.children) {
    if (c is ASTNode) {
      final found = findRule(c, name);
      if (found != null) return found;
    }
  }
  return null;
}

bool parses(String src) => tryParseAxiom(src) != null;

String nestedParenSource(int n) => '${'(' * n}5${')' * n}';
String nestedCallSource(int n) => '${'f(' * n}5${')' * n}';

// A SPACE between every `-` is load-bearing, not cosmetic: two
// adjacent MINUS characters with no space between them (`--`) is the
// line-comment opener and would swallow the ENTIRE rest of the line
// as a comment, silently testing "zero tokens" instead of a genuine
// deep unary-chain.
String unaryPrefixChainSource(int n) => '${'- ' * n}5';
String powerChainSource(int n) => '1${'^1' * n}';

void main() {
  group('reference suite', () {
    test('program is the root', () {
      expect(parseAxiom('1').ruleName, equals('program'));
    });

    test(
        'a bare juxtaposed pair with nothing after is a call not two statements',
        () {
      // No top-level separator exists in this cut (see axiom.grammar's
      // own header comment) — `program = expr`, so trailing garbage
      // after one full expression is a syntax error, not a
      // silently-accepted second statement.
      expect(tryParseAxiom('a := 1 b := 2'), isNull);
    });

    test('integer and float literals parse', () {
      expect(parses('42'), isTrue);
      expect(parses('1.5'), isTrue);
    });

    test('string literal parses', () {
      expect(parses('"hello"'), isTrue);
    });

    test('symbol parses', () {
      expect(parses('x'), isTrue);
    });

    test('explicit paren call parses', () {
      final ast = parseAxiom('f(a, b)');
      expect(containsRule(ast, 'postfix'), isTrue);
      expect(containsRule(ast, 'arglist'), isTrue);
    });

    test('paren optional single argument call parses', () {
      // MA13 §4's own confirmed examples.
      final ast = parseAxiom('factorial 7');
      expect(containsRule(ast, 'postfix'), isTrue);
      expect(containsRule(ast, 'call_args'), isTrue);
      expect(parses('ff z'), isTrue);
    });

    test('paren optional call with a string or list argument parses', () {
      expect(parses('f "hello"'), isTrue);
      expect(parses('f [1, 2, 3]'), isTrue);
    });

    test('call with no arguments parses', () {
      expect(parses('f()'), isTrue);
    });

    test('nested function calls parse', () {
      expect(parses('f(g(x))'), isTrue);
    });

    test('bare name followed by minus number is subtraction not a call', () {
      final ast = parseAxiom('f - 1');
      expect(containsRule(ast, 'additive'), isTrue);
      // No call_args node should have consumed the MINUS as part of
      // a negative-literal call argument.
      expect(containsRule(ast, 'call_args'), isFalse);
    });

    test('bare name followed by plus number is addition not a call', () {
      final ast = parseAxiom('f + 1');
      expect(containsRule(ast, 'additive'), isTrue);
      expect(containsRule(ast, 'call_args'), isFalse);
    });

    test('list literal parses', () {
      final ast = parseAxiom('[a, b, c]');
      expect(containsRule(ast, 'list_literal'), isTrue);
    });

    test('empty list literal parses', () {
      expect(parses('[]'), isTrue);
    });

    test('arithmetic precedence', () {
      final ast = parseAxiom('2 + 3 * 4 ^ 2');
      expect(containsRule(ast, 'additive'), isTrue);
      expect(containsRule(ast, 'multiplicative'), isTrue);
      expect(containsRule(ast, 'power'), isTrue);
    });

    test('caret and double star are the same operator', () {
      expect(parses('a ^ b'), isTrue);
      expect(parses('a ** b'), isTrue);
    });

    test('unary minus binds looser than power', () {
      final ast = parseAxiom('-x^2');
      expect(containsRule(ast, 'unary'), isTrue);
      expect(containsRule(ast, 'power'), isTrue);
    });

    test('power is right associative by shape', () {
      expect(parses('4^3^2'), isTrue);
    });

    test('multiplicative and additive are left associative by shape', () {
      expect(parses('1 - 2 - 3'), isTrue);
      expect(parses('1 / 2 / 3'), isTrue);
    });

    test('grouping parens parse', () {
      final ast = parseAxiom('(1 + 2) * 3');
      expect(containsRule(ast, 'group'), isTrue);
    });

    test('every comparison operator parses', () {
      for (final op in ['=', '~=', '<', '<=', '>', '>=']) {
        final src = 'a $op b';
        expect(parses(src), isTrue, reason: '`$src` should parse');
      }
    });

    test('comparison binds looser than additive', () {
      final ast = parseAxiom('a + 1 = b - 1');
      expect(containsRule(ast, 'comparison'), isTrue);
      expect(containsRule(ast, 'additive'), isTrue);
    });

    test('immediate assignment parses', () {
      final ast = parseAxiom('x := 5');
      expect(containsRule(ast, 'assignment'), isTrue);
    });

    test('assignment right associates by shape', () {
      expect(parses('a := b := 5'), isTrue);
    });

    test('declared function definition parses', () {
      final ast = parseAxiom(
          'power(x: Integer, n: NonNegativeInteger): Integer == x ** n');
      expect(containsRule(ast, 'declared_define'), isTrue);
      expect(containsRule(ast, 'typed_param_list'), isTrue);
    });

    test('declared function definition with no parameters parses', () {
      expect(parses('pi(): Float == 3.14'), isTrue);
    });

    test('undeclared function definition parses', () {
      final ast = parseAxiom('f x == x * x');
      expect(containsRule(ast, 'undeclared_define'), isTrue);
    });

    test(
        'undeclared definition with no parens is not confused with a plain call',
        () {
      // `f x` alone (no `==`) is an ordinary call; `f x == e` is a
      // definition — both must parse, to two DIFFERENT rules.
      final callAst = parseAxiom('f x');
      expect(containsRule(callAst, 'postfix'), isTrue);
      expect(containsRule(callAst, 'undeclared_define'), isFalse);

      final defAst = parseAxiom('f x == x');
      expect(containsRule(defAst, 'undeclared_define'), isTrue);
    });

    test('declared define requires a return type annotation', () {
      // MA13 §4's own row always shows the return-type annotation;
      // this grammar takes the narrower, spec-literal reading and
      // requires it.
      expect(tryParseAxiom('f(x: Integer) == x'), isNull);
    });

    test('declared define requires every parameter to be typed', () {
      expect(tryParseAxiom('f(x): Integer == x'), isNull);
    });

    test('if then else parses', () {
      final ast = parseAxiom('if a > 0 then 1 else -1');
      expect(containsRule(ast, 'if_expr'), isTrue);
    });

    test('if without else is rejected', () {
      // MA13 §4: "missing else — deferred" for this cut.
      expect(tryParseAxiom('if a > 0 then 1'), isNull);
    });

    test('if is usable as an expression', () {
      final ast = parseAxiom('x := if a > 0 then 1 else -1');
      expect(containsRule(ast, 'if_expr'), isTrue);
      expect(containsRule(ast, 'assignment'), isTrue);
    });

    test('dangling else attaches to the nearest if', () {
      expect(parses('if a then if b then 1 else 2 else 3'), isTrue);
    });

    test(
        'parenthesised block parses and shares the group rule with plain grouping',
        () {
      final blockAst = parseAxiom('(a := 1; a + 1)');
      final group = findRule(blockAst, 'group');
      expect(group, isNotNull);
      // Two `expr` children, joined by one SEMI — a block, not plain
      // grouping (axiom.grammar's own group-vs-block child-count
      // design).
      final exprChildren = group!.children
          .where((c) => c is ASTNode && c.ruleName == 'expr')
          .length;
      expect(exprChildren, equals(2));

      final groupAst = parseAxiom('(1 + 2)');
      final group2 = findRule(groupAst, 'group');
      expect(group2, isNotNull);
      final exprChildren2 = group2!.children
          .where((c) => c is ASTNode && c.ruleName == 'expr')
          .length;
      expect(exprChildren2, equals(1));
    });

    test('block with three statements parses', () {
      expect(parses('(a := 1; b := 2; a + b)'), isTrue);
    });

    test('plain declaration parses', () {
      final ast = parseAxiom('a : PositiveInteger');
      expect(containsRule(ast, 'declaration'), isTrue);
    });

    test('tuple declaration parses', () {
      final ast = parseAxiom('(a, b, c) : Integer');
      expect(containsRule(ast, 'declaration'), isTrue);
      expect(containsRule(ast, 'name_list'), isTrue);
    });

    test('declaration type can be a parameterized domain', () {
      final ast = parseAxiom('a : Fraction(Integer)');
      expect(containsRule(ast, 'declaration'), isTrue);
      expect(containsRule(ast, 'type_expr'), isTrue);
    });

    test('coercion parses', () {
      final ast = parseAxiom('3 :: Fraction(Integer)');
      expect(containsRule(ast, 'coercion'), isTrue);
    });

    test('coercion type accepts the paren optional shorthand', () {
      // MA13's own confirmed example: `3 :: Fraction Integer`.
      final ast = parseAxiom('3 :: Fraction Integer');
      expect(containsRule(ast, 'coercion'), isTrue);
      final typeExpr = findRule(ast, 'type_expr');
      expect(typeExpr, isNotNull);
      expect(containsRule(typeExpr!, 'type_ctor_args'), isTrue);
    });

    test('coercion left hand side can be a computed expression', () {
      final ast = parseAxiom('(1 + 2) :: Float');
      expect(containsRule(ast, 'coercion'), isTrue);
      expect(containsRule(ast, 'group'), isTrue);
    });

    test('coercion binds tighter than comparison', () {
      // `x :: T = y` must read as `(x :: T) = y` — both `comparison`
      // and `coercion` nodes must appear, with coercion nested UNDER
      // comparison.
      final ast = parseAxiom('x :: Integer = y');
      final cmp = findRule(ast, 'comparison');
      expect(cmp, isNotNull);
      expect(containsRule(cmp!, 'coercion'), isTrue);
    });

    test('coercion binds looser than additive', () {
      // `a + b :: T` must read as `(a + b) :: T`.
      final ast = parseAxiom('a + b :: Float');
      final coerce = findRule(ast, 'coercion');
      expect(coerce, isNotNull);
      expect(containsRule(coerce!, 'additive'), isTrue);
    });

    test('has query true example parses', () {
      // MA13's own worked example.
      final ast = parseAxiom('Polynomial(Integer) has Ring');
      expect(containsRule(ast, 'has_query'), isTrue);
    });

    test('has query false example parses', () {
      final ast = parseAxiom('List(Integer) has Ring');
      expect(containsRule(ast, 'has_query'), isTrue);
    });

    test('has query is not directly reachable as a bare arithmetic operand',
        () {
      // `has` is deliberately its OWN top-level `expr` alternative,
      // not folded into the general arithmetic cascade
      // (axiom.grammar's own `has_query` comment) — `additive`'s
      // operand is `multiplicative`, never the full `expr`, so a
      // BARE (unparenthesised) has-query cannot appear as an operand
      // of `+`.
      expect(tryParseAxiom('1 + Integer has Ring'), isNull);
    });

    test(
        'has query is reachable through explicit parens like every other expr form',
        () {
      // `group = LPAREN expr { SEMI expr } RPAREN` wraps the FULL
      // `expr` (deliberately — the same "if/assignment/block are all
      // usable as an expression" design every `expr`-shaped form in
      // this grammar gets, see `expr`'s own header comment), so a
      // has-query wrapped in explicit parens IS reachable as an
      // arithmetic operand, the same way a parenthesised assignment
      // or `if` would be.
      final ast = parseAxiom('1 + (Integer has Ring)');
      expect(containsRule(ast, 'has_query'), isTrue);
      expect(containsRule(ast, 'additive'), isTrue);
    });

    test('a bare type expr with no has falls through to an ordinary call', () {
      final ast = parseAxiom('Polynomial(Integer)');
      expect(containsRule(ast, 'postfix'), isTrue);
      expect(containsRule(ast, 'has_query'), isFalse);
    });

    test('type ctor paren optional argument must be a bare name', () {
      // `List Fraction(Integer)` — combining the paren-optional
      // shorthand with a further explicitly-parenthesised argument
      // — is NOT accepted by this grammar (axiom.grammar's own
      // `type_expr` comment); write `List(Fraction(Integer))`
      // instead.
      expect(tryParseAxiom('a : List Fraction(Integer)'), isNull);
      expect(parses('a : List(Fraction(Integer))'), isTrue);
    });

    test('deeply nested explicit type constructor parses up to the depth cap',
        () {
      expect(parses('a : List(Matrix(Polynomial(Integer)))'), isTrue);
    });

    test('comments are skipped', () {
      expect(parses('-- a comment\nx := 1 -- trailing'), isTrue);
    });

    test('syntax error is reported', () {
      expect(tryParseAxiom('1 +'), isNull);
      expect(tryParseAxiom('(1 + 2'), isNull);
      expect(tryParseAxiom('has Ring'), isNull);
      expect(() => parseAxiom('1 +'), throwsA(isA<GrammarParseError>()));
    });

    test('parse axiom throws on malformed source', () {
      // The reference panics ("Axiom parse failed"); the Dart
      // throwing one-shot surfaces the structured engine error
      // instead, per lane convention.
      expect(() => parseAxiom('1 +'), throwsA(isA<GrammarParseError>()));
    });
  });

  group('depth guard (mirrors the reference suite)', () {
    test('deeply nested input errors for every shape', () {
      // 5000 levels — far past maxRuleDepth — must produce a
      // recoverable error, not overflow the stack, for all four
      // shapes.
      for (final src in [
        nestedParenSource(5000),
        nestedCallSource(5000),
        unaryPrefixChainSource(5000),
        powerChainSource(5000),
      ]) {
        expect(() => parseAxiom(src), throwsA(isA<GrammarParseError>()));
        expect(tryParseAxiom(src), isNull);
      }
    });

    test('cap trips before overflow for every shape', () {
      // The Dart equivalent of the reference's default-stack-thread
      // test: far-too-deep input surfaces a catchable engine error
      // with no stack exhaustion on the ordinary event loop, and the
      // isolate stays healthy afterwards.
      for (final src in [
        nestedParenSource(5000),
        nestedCallSource(5000),
        unaryPrefixChainSource(5000),
        powerChainSource(5000),
      ]) {
        try {
          parseAxiom(src);
          fail('must not parse');
        } on GrammarParseError catch (e) {
          expect(e.message, isNotEmpty);
        }
      }
      expect(parseAxiom('factorial 7').ruleName, equals('program'));
    });

    test('reasonable nesting stays under the cap for every shape', () {
      expect(tryParseAxiom(nestedParenSource(3)) != null, isTrue);
      expect(tryParseAxiom(nestedCallSource(3)) != null, isTrue);
      expect(tryParseAxiom(unaryPrefixChainSource(5)) != null, isTrue);
      expect(tryParseAxiom(powerChainSource(5)) != null, isTrue);
    });

    test('headroom boundary for every shape', () {
      // Re-measured on this engine via scratch binary-search
      // probes, all identical to the reference: parens 13/14,
      // nested calls 13/14, unary chain 130/131, power 65/66.
      expect(tryParseAxiom(nestedParenSource(13)) != null, isTrue);
      expect(tryParseAxiom(nestedParenSource(14)), isNull);

      expect(tryParseAxiom(nestedCallSource(13)) != null, isTrue);
      expect(tryParseAxiom(nestedCallSource(14)), isNull);

      expect(tryParseAxiom(unaryPrefixChainSource(130)) != null, isTrue);
      expect(tryParseAxiom(unaryPrefixChainSource(131)), isNull);

      expect(tryParseAxiom(powerChainSource(65)) != null, isTrue);
      expect(tryParseAxiom(powerChainSource(66)), isNull);
    });
  });

  group('twins and errors', () {
    test('factory returns a wired parser', () {
      expect(
        createAxiomParser('factorial 7').parse().ruleName,
        equals('program'),
      );
    });
  });
}
