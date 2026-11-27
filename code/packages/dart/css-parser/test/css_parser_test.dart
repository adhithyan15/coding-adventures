/// Port of the reference `rust/css-parser` unit suite (13 tests).
library;

import 'package:coding_adventures_css_parser/css_parser.dart';
import 'package:coding_adventures_lexer/lexer.dart';
import 'package:coding_adventures_parser/parser.dart';
import 'package:test/test.dart';

/// Test-local nullable twin: `null` on any engine error (the lane,
/// like the reference, offers no `Result` API of its own).
ASTNode? soft(String src) {
  try {
    return parseCss(src);
  } on LexerError {
    return null;
  } on GrammarParseError {
    return null;
  }
}

void assertStylesheetRoot(ASTNode ast) {
  expect(
    ast.ruleName,
    equals('stylesheet'),
    reason: "Expected root rule 'stylesheet', got '${ast.ruleName}'",
  );
}

bool findRule(ASTNode node, String target) {
  if (node.ruleName == target) return true;
  for (final child in node.children) {
    if (child is ASTNode && findRule(child, target)) return true;
  }
  return false;
}

String nestedQualifiedRuleSource(int n) => '${'.a{' * n}color:red;${'}' * n}';

String nestedAtRuleSource(int n) => '${'@media{' * n}a{color:red;}${'}' * n}';

String nestedCalcSource(int n) => 'a{width:${'calc(' * n}1${')' * n};}';

String nestedNotSource(int n) => '${':not(' * n}a${')' * n}{color:red;}';

String nestedSupportsParenSource(int n) =>
    '@supports ${'(' * n}a:1${')' * n}{a{color:red;}}';

void main() {
  group('css-parser', () {
    test('parse simple rule', () {
      // A basic CSS rule with a type selector and one declaration.
      final ast = parseCss('body { color: red; }');
      assertStylesheetRoot(ast);
    });

    test('parse multiple declarations', () {
      final ast = parseCss('p { color: red; margin: 0; }');
      assertStylesheetRoot(ast);
    });

    test('parse multiple rules', () {
      final ast = parseCss('h1 { color: blue; } p { color: red; }');
      assertStylesheetRoot(ast);
    });

    test('parse class selector', () {
      final ast = parseCss('.warning { color: orange; }');
      assertStylesheetRoot(ast);
    });

    test('parse id selector', () {
      final ast = parseCss('#main { color: black; }');
      assertStylesheetRoot(ast);
    });

    test('parse at rule', () {
      final ast = parseCss('@media screen { body { color: red; } }');
      assertStylesheetRoot(ast);
    });

    test('parse empty stylesheet', () {
      final ast = parseCss('');
      assertStylesheetRoot(ast);
    });

    test('create parser', () {
      final parser = createCssParser('body { color: red; }');
      final ast = parser.parse();
      assertStylesheetRoot(ast);
    });

    test('parse descendant selector', () {
      final ast = parseCss('div p { color: red; }');
      assertStylesheetRoot(ast);
    });

    test('parse with whitespace', () {
      final ast = parseCss('  body  {  color  :  red  ;  }  ');
      assertStylesheetRoot(ast);
    });
  });

  group('depth guard', () {
    test('deeply nested input returns error not overflow', () {
      // 5000 levels/links of every measured shape — far past the
      // cap — must fail with a clean error, never a
      // StackOverflowError: the guard, not the stack, stops the
      // recursion. (Dart has no thread-stack tuning, so this single
      // in-process case covers both reference worker tests.)
      for (final src in [
        nestedQualifiedRuleSource(5000),
        nestedAtRuleSource(5000),
        nestedCalcSource(5000),
        nestedNotSource(5000),
        nestedSupportsParenSource(5000),
      ]) {
        expect(soft(src), isNull,
            reason: 'deeply-nested input must fail with an error');
      }
    });

    test('reasonable nesting stays under the cap', () {
      expect(soft(nestedQualifiedRuleSource(10)), isNotNull);
      expect(soft(nestedAtRuleSource(10)), isNotNull);
      expect(soft(nestedCalcSource(10)), isNotNull);
      expect(soft(nestedNotSource(10)), isNotNull);
      expect(soft(nestedSupportsParenSource(10)), isNotNull);
    });

    test('measured boundaries hold exactly', () {
      // Every boundary the reference measured holds bit-for-bit here:
      // the cap counts rule frames identically in both engines.
      expect(soft(nestedQualifiedRuleSource(33)), isNotNull);
      expect(soft(nestedQualifiedRuleSource(34)), isNull);
      expect(soft(nestedAtRuleSource(39)), isNotNull);
      expect(soft(nestedAtRuleSource(40)), isNull);
      expect(soft(nestedSupportsParenSource(55)), isNotNull);
      expect(soft(nestedSupportsParenSource(56)), isNull);
      expect(soft(nestedCalcSource(79)), isNotNull);
      expect(soft(nestedCalcSource(80)), isNull);
      expect(soft(nestedNotSource(81)), isNotNull);
      expect(soft(nestedNotSource(82)), isNull);
    });
  });
}
