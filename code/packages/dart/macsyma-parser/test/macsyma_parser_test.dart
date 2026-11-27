/// Port of the reference `rust/macsyma-parser` unit suite (3
/// depth-guard tests, with the 14/15 nesting boundary re-measured on
/// this engine) plus valid-program smoke tests and error-shape tests.
library;

import 'package:coding_adventures_lexer/lexer.dart';
import 'package:coding_adventures_macsyma_parser/macsyma_parser.dart';
import 'package:coding_adventures_parser/parser.dart';
import 'package:test/test.dart';

/// Builds `n` nested parens around a `0`, terminated with `$`
/// (MACSYMA's suppress-display statement terminator).
String nestedParenSource(int n) => '${'(' * n}0${')' * n}\$\n';

bool hasRule(ASTNode node, String target) {
  if (node.ruleName == target) return true;
  return node.children.any((c) => c is ASTNode && hasRule(c, target));
}

void main() {
  group('depth guard (mirrors the reference suite)', () {
    test('deeply-nested input errors instead of overflowing', () {
      // 5000 levels — far past maxRuleDepth. The engine cap must
      // trip and surface a clean GrammarParseError, not a
      // StackOverflowError.
      expect(
        () => parseMacsyma(nestedParenSource(5000)),
        throwsA(isA<GrammarParseError>()),
      );
      expect(tryParseMacsyma(nestedParenSource(5000)), isNull);
    });

    test('fourteen levels parse, fifteen trip the cap', () {
      // Boundary re-measured on this engine via scratch probe:
      // identical to the reference (14 parses, 15 trips at depth
      // cap 200 — same grammar cascade, same counting).
      final ast = parseMacsyma(nestedParenSource(14));
      expect(ast.ruleName, equals('program'));
      expect(
        () => parseMacsyma(nestedParenSource(15)),
        throwsA(isA<GrammarParseError>()),
      );
    });

    test('deep input fails cleanly on the normal isolate', () {
      // The reference proves the cap trips before native overflow
      // on a default-stack thread; the Dart equivalent is that
      // far-too-deep input surfaces a catchable engine error with
      // no stack exhaustion on the ordinary event loop.
      try {
        parseMacsyma(nestedParenSource(5000));
        fail('must not parse');
      } on GrammarParseError catch (e) {
        expect(e.message, isNotEmpty);
      }
      // And the isolate is still healthy afterwards.
      expect(parseMacsyma('1+2;').ruleName, equals('program'));
    });
  });

  group('valid programs', () {
    test('empty program parses to an empty statement list', () {
      expect(parseMacsyma('').ruleName, equals('program'));
    });

    test('statements terminate with semicolon or dollar', () {
      expect(parseMacsyma('1+2;').ruleName, equals('program'));
      expect(parseMacsyma('x : 3\$').ruleName, equals('program'));
    });

    test('control forms parse', () {
      for (final src in [
        'if a then b else c;',
        'if a then b elseif c then d else e;',
        'for i in [1] do i;',
        'for i: 1 thru 10 do i;',
        'while a do b;',
        'block([x], x+1)\$',
        'return(f);',
      ]) {
        expect(parseMacsyma(src).ruleName, equals('program'), reason: src);
      }
    });

    test('operators and precedence forms parse', () {
      for (final src in [
        'a^b^c;',
        '-x^2;',
        '2 * -b;',
        'not a and b or c;',
        'a : b := c;',
        'f(x, y);',
        's : "hi";',
        'l : [1,2];',
      ]) {
        expect(parseMacsyma(src).ruleName, equals('program'), reason: src);
      }
    });

    test('precedence cascade rules appear in the tree', () {
      final ast = parseMacsyma('a+b*c;');
      expect(hasRule(ast, 'additive'), isTrue);
      expect(hasRule(ast, 'multiplicative'), isTrue);
    });

    test('power nests under unary for -x^2', () {
      expect(hasRule(parseMacsyma('-x^2;'), 'power'), isTrue);
      expect(hasRule(parseMacsyma('-x^2;'), 'unary'), isTrue);
    });
  });

  group('errors', () {
    test('truncated expression is a syntax error', () {
      expect(() => parseMacsyma('1+;'), throwsA(isA<GrammarParseError>()));
      expect(tryParseMacsyma('1+;'), isNull);
    });

    test('chained comparison is a syntax error', () {
      // Comparisons are non-chaining in MACSYMA.
      expect(
          () => parseMacsyma('a < b < c;'), throwsA(isA<GrammarParseError>()));
    });

    test('unbalanced input is a syntax error', () {
      expect(() => parseMacsyma('('), throwsA(isA<GrammarParseError>()));
      expect(() => parseMacsyma('f(;'), throwsA(isA<GrammarParseError>()));
    });

    test('out-of-alphabet bytes are lexer errors', () {
      for (final src in ['\\', '`', 'é;']) {
        try {
          parseMacsyma(src);
          fail('must not parse: $src');
        } on LexerError catch (e) {
          expect(e.errorId, equals('lexical-error'));
          expect(e.line, greaterThanOrEqualTo(1));
          expect(e.column, greaterThanOrEqualTo(1));
        }
        expect(tryParseMacsyma(src), isNull);
      }
    });

    test('factory returns a wired parser', () {
      final parser = createMacsymaParser('1+2;');
      expect(parser.parse().ruleName, equals('program'));
    });
  });
}
