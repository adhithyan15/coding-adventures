/// Port of the reference `rust/c-parser` unit suite (19 tests).
library;

import 'package:coding_adventures_c_lexer/c_lexer.dart';
import 'package:coding_adventures_c_parser/c_parser.dart';
import 'package:coding_adventures_lexer/lexer.dart';
import 'package:coding_adventures_parser/parser.dart';
import 'package:test/test.dart';

ASTNode root(String src) => parseC(src);

bool hasRule(ASTNode node, String target) {
  if (node.ruleName == target) return true;
  return node.children.any((c) => c is ASTNode && hasRule(c, target));
}

int countRule(ASTNode node, String target) {
  var here = node.ruleName == target ? 1 : 0;
  for (final c in node.children) {
    if (c is ASTNode) here += countRule(c, target);
  }
  return here;
}

String nestedParenSource(int n) =>
    'int main(void) { return ${'(' * n}1${')' * n}; }';

/// Copies [tokens] with token [index]'s value replaced by [value]
/// (Dart tokens are immutable; the reference mutates in place).
List<Token> withValue(List<Token> tokens, int index, String value) {
  final old = tokens[index];
  final copy = List<Token>.from(tokens);
  copy[index] = Token(
    type: old.type,
    value: value,
    line: old.line,
    column: old.column,
    flags: old.flags,
  );
  return copy;
}

void main() {
  group('token input', () {
    test('token input matches source input', () {
      const src = 'int main(void) { return 2 + 3; }';
      final tokens = tryTokenizeC(src)!;
      expect(tryParseCTokens(tokens), equals(tryParseC(src)));
    });

    test('token input without eof parses', () {
      // A preprocessor emits ordinary source tokens, not an EOF
      // sentinel. The parser accepts this representation directly.
      final tokens =
          List<Token>.from(tryTokenizeC('int main(void) { return 1; }')!)
            ..removeLast();
      expect(
        tryParseCTokens(tokens)!.ruleName,
        equals('translation_unit'),
      );
    });

    test('empty token input returns error', () {
      expect(
        checkTokenBudget([]),
        contains('empty'),
      );
      expect(tryParseCTokens([]), isNull);
    });

    test('token input rejects premature eof', () {
      final tokens =
          List<Token>.from(tryTokenizeC('int main(void) { return 1; }')!);
      final trailing =
          List<Token>.from(tryTokenizeC('int extra(void) { return 2; }')!)
            ..removeLast();
      tokens.addAll(trailing);
      expect(
        checkTokenBudget(tokens),
        contains('premature EOF'),
      );
      expect(tryParseCTokens(tokens), isNull);
    });

    test('source input retains legacy directive behavior', () {
      const src = '#include <stdint.h>\nint main(void) { return 1; }';
      expect(
        tryParseC(src)!.ruleName,
        equals('translation_unit'),
      );
      // The same tokens WITHOUT the directive strip are rejected:
      // the token-input API never strips.
      final tokens = tryTokenizeC(src)!;
      expect(tryParseCTokens(tokens), isNull);
    });

    test('oversized token field is rejected before parsing', () {
      final tokens = tryTokenizeC('int main(void) { return 1; }')!;
      final big = withValue(tokens, 0, 'x' * (maxTokenFieldBytes + 1));
      expect(
        checkTokenBudget(big),
        contains('field exceeds'),
      );
      expect(tryParseCTokens(big), isNull);
    });

    test('token budget counts metadata and aggregate text', () {
      final tokens = tryTokenizeC('int main(void) { return 1; }')!;
      expect(
        checkTokenBudget(tokens, maxTokens: tokens.length - 1),
        contains('tokens'),
      );
      expect(
        checkTokenBudget(tokens, maxFieldBytes: 5),
        contains('field'),
      );
      expect(
        checkTokenBudget(tokens, maxTextBytes: 10),
        contains('text'),
      );
    });
  });

  group('c-parser', () {
    test('empty main parses', () {
      final ast = root('int main(void) { }');
      expect(ast.ruleName, equals('translation_unit'));
      expect(hasRule(ast, 'function_def'), isTrue);
    });

    test('declaration with init', () {
      final ast = root('int main(void) { int32_t x = 5; return x; }');
      expect(hasRule(ast, 'declaration'), isTrue);
      expect(hasRule(ast, 'return_stmt'), isTrue);
    });

    test('arithmetic precedence is structural', () {
      // 2 + 3 * 4 must nest the multiply under the add (mult is
      // tighter).
      final ast = root('int main(void) { return 2 + 3 * 4; }');
      expect(hasRule(ast, 'additive'), isTrue);
      expect(hasRule(ast, 'multiplicative'), isTrue);
    });

    test('cast is recognized', () {
      // (uint8_t)x is a cast, not a parenthesised expression.
      final ast =
          root('int main(void) { uint8_t c = (uint8_t)300; return c; }');
      expect(hasRule(ast, 'cast'), isTrue, reason: 'cast not found:\n$ast');
    });

    test('control flow and calls', () {
      const src =
          'int f(int x) { if (x > 0) { return 1; } else { return 0; } }\n'
          'int main(void) { int i = 0; while (i < 10) { i = i + 1; } printf("%d\\n", f(i)); return 0; }';
      final ast = root(src);
      expect(countRule(ast, 'function_def'), equals(2));
      expect(hasRule(ast, 'if_stmt'), isTrue);
      expect(hasRule(ast, 'while_stmt'), isTrue);
      expect(hasRule(ast, 'call_suffix'), isTrue); // printf(...) and f(i)
    });

    test('for loop with declaration init', () {
      final ast = root(
          'int main(void) { for (int i = 0; i < 3; i = i + 1) { } return 0; }');
      expect(hasRule(ast, 'for_stmt'), isTrue);
      expect(hasRule(ast, 'init_declarator'), isTrue);
    });

    test('multiword types and params', () {
      final ast =
          root('unsigned long g(unsigned int a, long long b) { return a; }');
      expect(hasRule(ast, 'function_def'), isTrue);
      expect(hasRule(ast, 'param'), isTrue);
    });

    test('double declaration with float literal parses', () {
      // `double` is a type keyword and `3.14` a FLOAT_LIT primary.
      final ast =
          root('double area(double r) { double pi = 3.14; return pi; }');
      expect(hasRule(ast, 'function_def'), isTrue);
      expect(hasRule(ast, 'declaration'), isTrue);
    });

    test('array declaration with brace initializer parses', () {
      final ast = root('int main(void) { int a[3] = {1, 2, 3}; return a[0]; }');
      expect(hasRule(ast, 'init_declarator'), isTrue);
      expect(hasRule(ast, 'init_list'), isTrue, reason: 'no init_list:\n$ast');
      expect(hasRule(ast, 'index_suffix'), isTrue,
          reason: 'no index_suffix:\n$ast');
    });

    test('array sized from initializer and indexed assignment parse', () {
      // `int a[] = {…}` (size inferred) and `a[i] = v` (indexed write).
      final ast =
          root('int main(void) { int a[] = {5, 6}; a[1] = 9; return a[1]; }');
      expect(hasRule(ast, 'init_list'), isTrue);
      expect(
        countRule(ast, 'index_suffix'),
        equals(2),
        reason: 'a[1]=… and return a[1]',
      );
    });
  });

  group('extras', () {
    test('factory parses directly without budget check', () {
      // Like the reference factory: no budget gate here (use parseC
      // for untrusted input).
      final ast = createCParser('int main(void) { return 1; }').parse();
      expect(ast.ruleName, equals('translation_unit'));
    });

    test('budget refusal throws through parseC', () {
      // A single 70KB string literal trips the 64KB field cap: the
      // throwing twin raises GrammarParseError carrying the refusal,
      // the nullable twin returns null.
      final huge = 'x' * (maxTokenFieldBytes + 1);
      final src = 'int main(void) { char *s = "$huge"; return 0; }';
      try {
        parseC(src);
        fail('expected GrammarParseError');
      } on GrammarParseError catch (e) {
        expect(e.message, contains('field exceeds'));
      }
      expect(tryParseC(src), isNull);
    });
  });

  group('depth guard', () {
    test('deeply nested input does not overflow on default stack', () {
      // 5000 levels must fail with a clean error, never a
      // StackOverflowError: the guard, not the stack, stops the
      // recursion. (Dart has no thread-stack tuning, so this single
      // in-process case covers the reference worker test.)
      expect(tryParseC(nestedParenSource(5000)), isNull);
    });

    test('reasonable nesting stays under the cap', () {
      expect(tryParseC(nestedParenSource(10)), isNotNull);
    });
  });
}
