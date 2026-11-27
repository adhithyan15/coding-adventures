/// Port of the reference `rust/oct-parser` unit suite (all 14
/// tests, with the 20/21 nesting boundary re-measured on this
/// engine) plus nullable-twin and error-shape tests.
library;

import 'package:coding_adventures_lexer/lexer.dart';
import 'package:coding_adventures_oct_lexer/oct_lexer.dart';
import 'package:coding_adventures_oct_parser/oct_parser.dart';
import 'package:coding_adventures_parser/parser.dart';
import 'package:test/test.dart';

String nestedParenSource(int n) =>
    'fn main() { let x: u8 = ${'(' * n}1${')' * n}; }';

bool hasRule(ASTNode node, String target) {
  if (node.ruleName == target) return true;
  return node.children.any((c) => c is ASTNode && hasRule(c, target));
}

void main() {
  group('reference functional suite', () {
    test('parses minimal main', () {
      final ast = parseOct('fn main() { }');
      expect(ast.ruleName, equals('program'));
      expect(hasRule(ast, 'fn_decl'), isTrue);
    });

    test('parses let with type annotation', () {
      final ast = parseOct('fn main() { let x: u8 = 5; }');
      expect(hasRule(ast, 'let_stmt'), isTrue);
    });

    test('parses return statement', () {
      final ast = parseOct('fn add(a: u8, b: u8) -> u8 { return a + b; }');
      expect(hasRule(ast, 'return_stmt'), isTrue);
      expect(hasRule(ast, 'add_expr'), isTrue);
    });

    test('parses if else', () {
      final ast = parseOct('fn t() { if 1 == 1 { } else { } }');
      expect(hasRule(ast, 'if_stmt'), isTrue);
    });

    test('parses while loop', () {
      final ast = parseOct('fn t() { while 1 == 1 { } }');
      expect(hasRule(ast, 'while_stmt'), isTrue);
    });

    test('parses loop and break', () {
      final ast = parseOct('fn t() { loop { break; } }');
      expect(hasRule(ast, 'loop_stmt'), isTrue);
      expect(hasRule(ast, 'break_stmt'), isTrue);
    });

    test('parses intrinsic call', () {
      // `out(port, value)` is an intrinsic, not a regular call.
      final ast = parseOct('fn t() { out(1, 0); }');
      expect(hasRule(ast, 'intrinsic_call'), isTrue);
    });

    test('parses user function call', () {
      final ast = parseOct('fn forty_two() -> u8 { return 42; }\n'
          'fn main() { let r: u8 = forty_two(); }');
      expect(hasRule(ast, 'call_expr'), isTrue);
    });

    test('parses static decl', () {
      final ast = parseOct('static counter: u8 = 0;\nfn main() { }');
      expect(hasRule(ast, 'static_decl'), isTrue);
    });

    test('parses expression precedence', () {
      // Bitwise above additive, additive above relational.
      final ast = parseOct('fn t() { if 1 + 2 == 3 { } }');
      expect(hasRule(ast, 'add_expr'), isTrue);
      expect(hasRule(ast, 'eq_expr'), isTrue);
    });

    test('rejects syntax errors', () {
      // Missing closing brace. Only the failure itself is pinned —
      // exact message format is the grammar engine's concern.
      expect(() => parseOct('fn main() {'), throwsA(isA<GrammarParseError>()));
      expect(tryParseOct('fn main() {'), isNull);
    });
  });

  group('depth guard (mirrors the reference suite)', () {
    test('deeply-nested input errors instead of overflowing', () {
      expect(
        () => parseOct(nestedParenSource(5000)),
        throwsA(isA<GrammarParseError>()),
      );
      expect(tryParseOct(nestedParenSource(5000)), isNull);
    });

    test('twenty levels parse, twenty-one trip the cap', () {
      // Boundary re-measured on this engine via scratch probe:
      // identical to the reference (20 parses, 21 trips at depth
      // cap 200 — same grammar cascade, same counting).
      final ast = parseOct(nestedParenSource(20));
      expect(ast.ruleName, equals('program'));
      expect(
        () => parseOct(nestedParenSource(21)),
        throwsA(isA<GrammarParseError>()),
      );
    });

    test('deep input fails cleanly on the normal isolate', () {
      // The reference proves the cap trips before native overflow
      // on a default-stack thread; the Dart equivalent is that
      // far-too-deep input surfaces a catchable engine error with
      // no stack exhaustion on the ordinary event loop.
      try {
        parseOct(nestedParenSource(5000));
        fail('must not parse');
      } on GrammarParseError catch (e) {
        expect(e.message, isNotEmpty);
      }
      // And the isolate is still healthy afterwards.
      expect(
        parseOct('fn main() { let x: u8 = 5; }').ruleName,
        equals('program'),
      );
    });
  });

  group('twins and errors', () {
    test('empty program parses', () {
      expect(parseOct('').ruleName, equals('program'));
    });

    test('token factory parses dialect streams', () {
      const source = 'fn main() { let x: u8 = 5; }';
      expect(
        createOctParserFromTokens(tokenizeOct(source)).parse(),
        equals(parseOct(source)),
      );
    });

    test('factories return wired parsers', () {
      expect(
        createOctParser('fn main() { }').parse().ruleName,
        equals('program'),
      );
    });

    test('out-of-alphabet bytes are lexer errors', () {
      for (final src in ['@', 'é']) {
        try {
          parseOct(src);
          fail('must not parse: $src');
        } on LexerError catch (e) {
          expect(e.errorId, equals('lexical-error'));
          expect(e.line, greaterThanOrEqualTo(1));
          expect(e.column, greaterThanOrEqualTo(1));
        }
        expect(tryParseOct(src), isNull);
      }
    });
  });
}
