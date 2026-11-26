/// Port of the reference `rust/nib-parser` unit suite (all 7
/// tests, with the 16/17 nesting boundary re-measured on this
/// engine) plus nullable-twin and error-shape tests.
library;

import 'package:coding_adventures_lexer/lexer.dart';
import 'package:coding_adventures_nib_lexer/nib_lexer.dart';
import 'package:coding_adventures_nib_parser/nib_parser.dart';
import 'package:coding_adventures_parser/parser.dart';
import 'package:test/test.dart';

String nestedParenSource(int n) =>
    'fn main() { let x: u4 = ${'(' * n}1${')' * n}; }';

bool hasRule(ASTNode node, String target) {
  if (node.ruleName == target) return true;
  return node.children.any((c) => c is ASTNode && hasRule(c, target));
}

void main() {
  group('depth guard (mirrors the reference suite)', () {
    test('deeply-nested input errors instead of overflowing', () {
      expect(
        () => parseNib(nestedParenSource(5000)),
        throwsA(isA<GrammarParseError>()),
      );
      expect(tryParseNib(nestedParenSource(5000)), isNull);
    });

    test('sixteen levels parse, seventeen trip the cap', () {
      // Boundary re-measured on this engine via scratch probe:
      // identical to the reference (16 parses, 17 trips at depth
      // cap 200 — same grammar cascade, same counting).
      final ast = parseNib(nestedParenSource(16));
      expect(ast.ruleName, equals('program'));
      expect(
        () => parseNib(nestedParenSource(17)),
        throwsA(isA<GrammarParseError>()),
      );
    });

    test('deep input fails cleanly on the normal isolate', () {
      // The reference proves the cap trips before native overflow
      // on a default-stack thread; the Dart equivalent is that
      // far-too-deep input surfaces a catchable engine error with
      // no stack exhaustion on the ordinary event loop.
      try {
        parseNib(nestedParenSource(5000));
        fail('must not parse');
      } on GrammarParseError catch (e) {
        expect(e.message, isNotEmpty);
      }
      // And the isolate is still healthy afterwards.
      expect(
        parseNib('fn main() { let x: u4 = 1; }').ruleName,
        equals('program'),
      );
    });
  });

  group('reference functional suite', () {
    test('parses function declaration', () {
      final ast = parseNib('fn main() { let x: u4 = 1; }');
      expect(ast.ruleName, equals('program'));
      expect(hasRule(ast, 'fn_decl'), isTrue);
      expect(hasRule(ast, 'let_stmt'), isTrue);
    });

    test('token input keeps nib grammar and source positions', () {
      const source = 'fn main() { let x: u4 = 1; }';
      expect(
        parseNibTokens(tokenizeNib(source)),
        equals(parseNib(source)),
      );
    });

    test('parses binary expression', () {
      final ast = parseNib('fn main() { let x: u4 = 1 +% 2; }');
      expect(hasRule(ast, 'expr'), isTrue);
    });

    test('parses shift expression', () {
      final ast = parseNib('fn main() { let x: u8 = 1 << 2 >> 1; }');
      expect(hasRule(ast, 'shift_expr'), isTrue);
    });
  });

  group('twins and errors', () {
    test('empty program parses', () {
      expect(parseNib('').ruleName, equals('program'));
    });

    test('nullable twins null-agree with the throwing API', () {
      const good = 'fn main() { let x: u4 = 1; }';
      expect(tryParseNib(good), equals(parseNib(good)));
      expect(
        tryParseNibTokens(tokenizeNib(good)),
        equals(parseNibTokens(tokenizeNib(good))),
      );
      const bad = 'fn main() { let x: u4 = 1 +; }';
      expect(tryParseNib(bad), isNull);
      expect(
        () => parseNibTokens(tokenizeNib(bad)),
        throwsA(isA<GrammarParseError>()),
      );
      expect(tryParseNibTokens(tokenizeNib(bad)), isNull);
    });

    test('empty token input fails closed, never StateError', () {
      // Regression: the engine indexes the token list directly,
      // so without the guard this escapes as `StateError: Bad
      // state: No element` instead of the documented vocabulary.
      expect(
        () => parseNibTokens([]),
        throwsA(
          isA<GrammarParseError>().having(
            (e) => e.message,
            'message',
            contains('empty'),
          ),
        ),
      );
      expect(tryParseNibTokens([]), isNull);
    });

    test('factories return wired parsers', () {
      expect(
        createNibParser('fn main() { }').parse().ruleName,
        equals('program'),
      );
      expect(
        createNibParserFromTokens(
          tokenizeNib('fn main() { }'),
        ).parse().ruleName,
        equals('program'),
      );
    });

    test('truncated and unbalanced input are syntax errors', () {
      for (final src in [
        'fn main() { let x: u4 = 1 +; }',
        'fn main() { let x = ; }',
        'fn (',
        'fn main() { let x: u4 = 1; ',
      ]) {
        expect(() => parseNib(src), throwsA(isA<GrammarParseError>()),
            reason: src);
        expect(tryParseNib(src), isNull, reason: src);
      }
    });

    test('out-of-alphabet bytes are lexer errors', () {
      for (final src in ['@', 'é']) {
        try {
          parseNib(src);
          fail('must not parse: $src');
        } on LexerError catch (e) {
          expect(e.errorId, equals('lexical-error'));
          expect(e.line, greaterThanOrEqualTo(1));
          expect(e.column, greaterThanOrEqualTo(1));
        }
        expect(tryParseNib(src), isNull);
      }
    });
  });
}
