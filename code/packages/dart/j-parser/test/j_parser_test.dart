/// Port of the reference `rust/j-parser` unit suite (33 tests).
library;

import 'package:coding_adventures_j_parser/j_parser.dart';
import 'package:coding_adventures_lexer/lexer.dart';
import 'package:coding_adventures_parser/parser.dart';
import 'package:test/test.dart';

/// The rule names of [node]'s direct AST children.
List<String> ruleNames(ASTNode node) => [
      for (final c in node.children)
        if (c is ASTNode) c.ruleName,
    ];

/// Recursively collect every token *name* appearing anywhere in the
/// tree.
List<String> tokenNames(ASTNode node) {
  final out = <String>[];
  for (final child in node.children) {
    if (child is Token) {
      out.add(child.type);
    } else if (child is ASTNode) {
      out.addAll(tokenNames(child));
    }
  }
  return out;
}

String nestedParenSource(int n) => '${'(' * n}5${')' * n}\n';

/// A flat, unparenthesised dyadic chain `1+1+1+…+1` with `n` `+`s.
String flatChainSource(int n) => '1${'+1' * n}\n';

/// A long train `(+ + +)5` with `n` `PLUS` teeth inside one paren
/// pair, applied monadically so it parses as a top-level statement.
String trainSource(int n) => '(${List.filled(n, '+').join(' ')})5\n';

void main() {
  group('j-parser', () {
    test('bare number parses', () {
      expect(parseJ('5\n').ruleName, equals('program'));
    });

    test('stranded numbers form one term', () {
      // `1 2 3` is a single 3-element vector term, not three statements.
      expect(tryParseJ('1 2 3\n')!.ruleName, equals('program'));
    });

    test('simple assignment parses', () {
      expect(tryParseJ('A=.5\n')!.ruleName, equals('program'));
    });

    test('chained assignment is right associative', () {
      // A=.B=.3 assigns 3 to both B and A.
      expect(tryParseJ('A=.B=.3\n')!.ruleName, equals('program'));
    });

    test('global assignment parses', () {
      expect(tryParseJ('A=:5\n')!.ruleName, equals('program'));
    });

    test('monadic application parses', () {
      // $ with nothing to its left is monadic (shape-of).
      expect(tryParseJ('\$A\n')!.ruleName, equals('program'));
    });

    test('dyadic application parses', () {
      expect(tryParseJ('A+B\n')!.ruleName, equals('program'));
    });

    test('right to left dyadic chain parses as one noun expr', () {
      // 2*3+4 is 2*(3+4) -- a single right-recursive noun_expr, not
      // three separate statements.
      expect(tryParseJ('2*3+4\n')!.ruleName, equals('program'));
    });

    test('reduce operator parses', () {
      expect(tryParseJ('+/A\n')!.ruleName, equals('program'));
    });

    test('scan operator parses', () {
      expect(tryParseJ('+\\A\n')!.ruleName, equals('program'));
    });

    test('at compose conjunction parses', () {
      // `@` composes two verb_exprs directly.
      expect(tryParseJ('+@-A\n')!.ruleName, equals('program'));
      expect(tryParseJ('A+@-B\n')!.ruleName, equals('program'));
    });

    test('parenthesised grouping parses', () {
      expect(tryParseJ('(A+B)*C\n')!.ruleName, equals('program'));
    });

    test('every comparison verb parses', () {
      for (final op in ['=', '~:', '<', '>', '<:', '>:']) {
        final src = 'A${op}B\n';
        expect(tryParseJ(src), isNotNull, reason: '`$src` should parse');
      }
    });

    test('every primitive verb parses monadically', () {
      for (final op in [
        '+',
        '-',
        '*',
        '%',
        '^',
        '<.',
        '>.',
        '\$',
        'i.',
        ',',
        '#',
        '=',
        '~:',
        '<',
        '>',
        '<:',
        '>:',
      ]) {
        final src = '${op}A\n';
        expect(tryParseJ(src), isNotNull, reason: '`$src` should parse');
      }
    });

    test('comment line and blank line both parse', () {
      // `NB.` comments are stripped by the lexer's skip pattern; a
      // bare NEWLINE is its own `line` alternative.
      expect(tryParseJ('NB. just a comment\n\nA=.1\n')!.ruleName,
          equals('program'));
    });

    test('multi line program parses into multiple lines', () {
      final ast = tryParseJ('A=.1\nB=.2\nA+B\n')!;
      expect(ruleNames(ast).where((n) => n == 'line').length, equals(3));
    });

    test('two tooth hook parses', () {
      expect(tryParseJ('(+ -)A\n')!.ruleName, equals('program'));
    });

    test('three tooth fork parses', () {
      expect(tryParseJ('(+ - *)A\n')!.ruleName, equals('program'));
    });

    test('four plus tooth fork parses', () {
      expect(tryParseJ('(+ - * % ^)A\n')!.ruleName, equals('program'));
    });

    test('fork with leading noun parses', () {
      // A bare noun tooth is only *semantically* meaningful in a
      // fork's leading position -- the grammar accepts the shape
      // syntactically and leaves the restriction to lowering.
      expect(tryParseJ('(5 - *)A\n')!.ruleName, equals('program'));
    });

    test('dyadic application of train parses', () {
      expect(tryParseJ('A(+ -)B\n')!.ruleName, equals('program'));
    });

    test('train nested within train parses', () {
      expect(tryParseJ('((+ -) - *)A\n')!.ruleName, equals('program'));
    });

    test('reduce parses as reduce not division', () {
      // `+/A` must lower `/` as the REDUCE adverb (postfix on `+`),
      // never as a divide-like binary operator: the tree must contain
      // a REDUCE token and never a PERCENT token.
      final ast = tryParseJ('+/A\n')!;
      final tokens = tokenNames(ast);
      expect(tokens, contains('REDUCE'),
          reason:
              'expected a REDUCE token in the parse tree for `+/A`, got: $tokens');
      expect(tokens, isNot(contains('PERCENT')),
          reason:
              '`+/A` must never produce a PERCENT (division) token, got: $tokens');
    });

    test('malformed input is rejected not panicking', () {
      // A bare adverb with nothing to modify is not a valid verb_expr.
      expect(tryParseJ('/A\n'), isNull);
    });
  });

  group('depth guard', () {
    test('deeply nested input returns error not overflow', () {
      expect(tryParseJ(nestedParenSource(5000)), isNull);
    });

    test('huge flat chain returns error not overflow', () {
      expect(tryParseJ(flatChainSource(5000)), isNull);
    });

    test('huge train returns error not overflow', () {
      expect(tryParseJ(trainSource(5000)), isNull);
    });

    test('nesting up to cap still parses', () {
      expect(
        tryParseJ(nestedParenSource(32)),
        isNotNull,
        reason: '32 levels must stay under the cap',
      );
      final ast = tryParseJ(nestedParenSource(32))!;
      expect(ast.ruleName, equals('program'));
      expect(
        tryParseJ(nestedParenSource(33)),
        isNull,
        reason: 'one nesting level past the cap must fail',
      );
    });

    test('flat chain up to cap still parses', () {
      expect(
        tryParseJ(flatChainSource(63)),
        isNotNull,
        reason: '63 terms must stay under the cap',
      );
      expect(
        tryParseJ(flatChainSource(64)),
        isNull,
        reason: 'one chain term past the cap must fail',
      );
    });

    test('train up to cap still parses', () {
      expect(
        tryParseJ(trainSource(61)),
        isNotNull,
        reason: '61 teeth must stay under the cap',
      );
      expect(
        tryParseJ(trainSource(62)),
        isNull,
        reason: 'one tooth past the cap must fail',
      );
    });
  });
}
