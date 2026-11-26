/// Port of the reference `rust/apl-parser` unit suite (22 tests).
library;

import 'package:coding_adventures_apl_parser/apl_parser.dart';
import 'package:coding_adventures_parser/parser.dart';
import 'package:test/test.dart';

/// The rule names of [node]'s direct AST children.
List<String> ruleNames(ASTNode node) => [
      for (final c in node.children)
        if (c is ASTNode) c.ruleName,
    ];

String nestedParenSource(int n) => '${'(' * n}5${')' * n}\n';

/// A flat, unparenthesised dyadic chain `1+1+1+…+1` with `n` `+`s.
String flatChainSource(int n) => '1${'+1' * n}\n';

void main() {
  group('apl-parser', () {
    test('bare number parses as scalar value expr', () {
      expect(parseApl('5\n').ruleName, equals('program'));
    });

    test('stranded numbers form one term', () {
      // `1 2 3` is a single 3-element vector term, not three statements.
      expect(tryParseApl('1 2 3\n')!.ruleName, equals('program'));
    });

    test('simple assignment parses', () {
      expect(tryParseApl('A←5\n')!.ruleName, equals('program'));
    });

    test('chained assignment is right associative', () {
      // A←B←3 assigns 3 to both B and A.
      expect(tryParseApl('A←B←3\n')!.ruleName, equals('program'));
    });

    test('monadic application parses', () {
      // ⍴ with nothing to its left is monadic (shape-of).
      expect(tryParseApl('⍴A\n')!.ruleName, equals('program'));
    });

    test('dyadic application parses', () {
      expect(tryParseApl('A+B\n')!.ruleName, equals('program'));
    });

    test('right to left dyadic chain parses as one value expr', () {
      // 2×3+4 is 2×(3+4) -- a single right-recursive value_expr, not
      // three separate statements.
      expect(tryParseApl('2×3+4\n')!.ruleName, equals('program'));
    });

    test('reduce operator parses', () {
      expect(tryParseApl('+/A\n')!.ruleName, equals('program'));
    });

    test('scan operator parses', () {
      expect(tryParseApl('+\\A\n')!.ruleName, equals('program'));
    });

    test('outer product operator parses', () {
      expect(tryParseApl('A∘.×B\n')!.ruleName, equals('program'));
    });

    test('parenthesised grouping parses', () {
      expect(tryParseApl('(A+B)×C\n')!.ruleName, equals('program'));
    });

    test('comparison function atoms parse', () {
      for (final op in ['=', '≠', '<', '≤', '≥', '>']) {
        final src = 'A${op}B\n';
        expect(tryParseApl(src), isNotNull, reason: '`$src` should parse');
      }
    });

    test('every primitive function atom parses monadically', () {
      for (final op in ['+', '-', '×', '÷', '⌈', '⌊', '⍴', '⍳', ',']) {
        final src = '${op}A\n';
        expect(tryParseApl(src), isNotNull, reason: '`$src` should parse');
      }
    });

    test('comment line and blank line both parse', () {
      // `⍝` comments are stripped by the lexer's skip pattern; a bare
      // NEWLINE is its own `line` alternative.
      expect(tryParseApl('⍝ just a comment\n\nA←1\n')!.ruleName,
          equals('program'));
    });

    test('multi line program parses into multiple lines', () {
      final ast = tryParseApl('A←1\nB←2\nA+B\n')!;
      // Every non-blank line contributes one `line` child.
      expect(ruleNames(ast).where((n) => n == 'line').length, equals(3));
    });

    test('malformed input is rejected not panicking', () {
      // A bare operator with no operands is not a valid value_expr.
      expect(tryParseApl('←←←\n'), isNull);
    });
  });

  group('depth guard', () {
    test('deeply nested input returns error not overflow', () {
      // 5000 levels — far past the cap — must fail with a clean
      // error, never a StackOverflowError: the guard, not the stack,
      // stops the recursion. (Dart has no thread-stack tuning, so
      // this single in-process case covers the reference worker
      // tests.)
      expect(tryParseApl(nestedParenSource(5000)), isNull);
    });

    test('huge flat chain returns error not overflow', () {
      expect(tryParseApl(flatChainSource(5000)), isNull);
    });

    test('nesting up to cap still parses', () {
      expect(
        tryParseApl(nestedParenSource(47)),
        isNotNull,
        reason: '47 levels must stay under the cap',
      );
      final ast = tryParseApl(nestedParenSource(47))!;
      expect(ast.ruleName, equals('program'));
      expect(
        tryParseApl(nestedParenSource(48)),
        isNull,
        reason: 'one nesting level past the cap must fail',
      );
    });

    test('flat chain up to cap still parses', () {
      // 94 terms is the measured safe limit at cap 100 — the binding
      // constraint the value was corrected against.
      final ast = tryParseApl(flatChainSource(94))!;
      expect(ast.ruleName, equals('program'));
      expect(
        tryParseApl(flatChainSource(95)),
        isNull,
        reason: 'one chain term past the cap must fail',
      );
    });
  });
}
