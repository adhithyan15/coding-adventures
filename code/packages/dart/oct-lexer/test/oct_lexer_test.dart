/// Unit tests for `coding_adventures_oct_lexer`: all 5 reference
/// tests, plus the corners the reference leaves uncovered — exact
/// streams, table-wide keyword promotion, error positions, empty
/// input, and factory-direct use.
library;

import 'package:coding_adventures_lexer/lexer.dart';
import 'package:coding_adventures_oct_lexer/oct_lexer.dart';
import 'package:test/test.dart';

void main() {
  group('reference', () {
    test('simple function tokenizes', () {
      final tokens = tokenizeOct('fn main() { let x: u8 = 5; }');
      expect(tokens.map((t) => t.value), contains('fn'));
      expect(tokens.map((t) => t.value), contains('main'));
      expect(tokens.map((t) => t.value), contains('5'));
      expect(tokens.map((t) => t.value), contains('u8'));
    });

    test('keyword types promote to literal values', () {
      final tokens = tokenizeOct('fn main() {}');
      final fnToken = tokens.firstWhere((t) => t.value == 'fn');
      expect(fnToken.type, 'fn');
    });

    test('intrinsics tokenize', () {
      final tokens =
          tokenizeOct('fn t() { out(1, 0); let c: bool = carry(); }');
      expect(tokens.map((t) => t.value), contains('out'));
      expect(tokens.map((t) => t.value), contains('carry'));
    });

    test('arithmetic and relops tokenize', () {
      final tokens =
          tokenizeOct('fn t() { let x: u8 = 1 + 2; if x == 3 { x = x - 1; } }');
      expect(tokens.map((t) => t.value), contains('+'));
      expect(tokens.map((t) => t.value), contains('=='));
      expect(tokens.map((t) => t.value), contains('-'));
    });

    test('loop and break tokenize', () {
      final tokens = tokenizeOct('fn t() { loop { break; } }');
      expect(tokens.map((t) => t.value), contains('loop'));
      expect(tokens.map((t) => t.value), contains('break'));
    });
  });

  group('extras', () {
    test('declaration stream is exact', () {
      expect([
        for (final t in tokenizeOct('fn main() { let x: u8 = 5; }'))
          (t.type, t.value)
      ], [
        ('fn', 'fn'),
        ('NAME', 'main'),
        ('LPAREN', '('),
        ('RPAREN', ')'),
        ('LBRACE', '{'),
        ('let', 'let'),
        ('NAME', 'x'),
        ('COLON', ':'),
        ('NAME', 'u8'),
        ('EQ', '='),
        ('INT_LIT', '5'),
        ('SEMICOLON', ';'),
        ('RBRACE', '}'),
        ('EOF', ''),
      ]);
    });

    test('every keyword promotes across the table', () {
      for (final kw in [
        'fn',
        'let',
        'static',
        'if',
        'else',
        'while',
        'loop',
        'break',
        'return',
        'true',
        'false',
        'in',
        'out',
        'carry',
      ]) {
        final tokens = tokenizeOct(kw);
        expect(tokens[0].type, kw);
        expect(tokens[0].value, kw);
      }
    });

    test('non-keyword names keep the NAME type', () {
      expect(tokenizeOct('main')[0].type, 'NAME');
    });

    test('errors carry positions', () {
      expect(() => tokenizeOct('@'), throwsA(isA<LexerError>()));
      try {
        tokenizeOct('ab@cd');
        fail('expected LexerError');
      } on LexerError catch (e) {
        expect((e.line, e.column), (1, 3));
        expect(e.errorId, isNotEmpty);
      }
    });

    test('empty input yields only EOF', () {
      final tokens = tokenizeOct('');
      expect(tokens, hasLength(1));
      expect(tokens.single.type, 'EOF');
    });

    test('factory tokenizes directly', () {
      final tokens = createOctLexer('fn main() {}').tokenize();
      expect(tokens.last.type, 'EOF');
      expect(() => createOctLexer('@').tokenize(), throwsA(isA<LexerError>()));
    });
  });
}
