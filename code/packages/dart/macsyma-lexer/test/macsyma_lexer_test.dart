/// Unit tests for `coding_adventures_macsyma_lexer`: all 5
/// reference tests, plus the corners the reference leaves uncovered —
/// error positions, empty input, and factory-direct use.
library;

import 'package:coding_adventures_lexer/lexer.dart';
import 'package:coding_adventures_macsyma_lexer/macsyma_lexer.dart';
import 'package:test/test.dart';

List<String> types(String source) => [
      for (final t in tokenizeMacsyma(source))
        if (t.type != 'EOF') t.type
    ];

List<String> values(String source) => [
      for (final t in tokenizeMacsyma(source))
        if (t.type != 'EOF') t.value
    ];

void main() {
  group('reference', () {
    test('numbers, names, percent constants, history refs', () {
      expect(types('42 3.14 1.5e10 x %pi % %i1 %o2'), [
        'NUMBER',
        'NUMBER',
        'NUMBER',
        'NAME',
        'NAME',
        'NAME',
        'NAME',
        'NAME'
      ]);
      expect(values('%pi % %i1 %o2'), ['%pi', '%', '%i1', '%o2']);
    });

    test('compiled longest-match operator order', () {
      expect(types('f(x) := x ** 2 <= y >= z -> q'), [
        'NAME',
        'LPAREN',
        'NAME',
        'RPAREN',
        'COLONEQ',
        'NAME',
        'STAREQ',
        'NUMBER',
        'LEQ',
        'NAME',
        'GEQ',
        'NAME',
        'ARROW',
        'NAME',
      ]);
    });

    test('keywords promote with values intact', () {
      expect([
        for (final t in tokenizeMacsyma('x and y or not false'))
          if (t.type != 'EOF') (t.type, t.value)
      ], [
        ('NAME', 'x'),
        ('KEYWORD', 'and'),
        ('NAME', 'y'),
        ('KEYWORD', 'or'),
        ('KEYWORD', 'not'),
        ('KEYWORD', 'false'),
      ]);
    });

    test('whitespace and comments vanish', () {
      expect(types('/* ignored */\n x + y'), ['NAME', 'PLUS', 'NAME']);
    });

    test('distinct statement terminators preserved', () {
      expect(types('x; y\$'), ['NAME', 'SEMI', 'NAME', 'DOLLAR']);
    });
  });

  group('extras', () {
    test('power and arrow singles', () {
      expect(types('**'), ['STAREQ']);
      expect(types('->'), ['ARROW']);
      expect(types(':='), ['COLONEQ']);
    });

    test('errors carry positions', () {
      expect(() => tokenizeMacsyma('@'), throwsA(isA<LexerError>()));
      try {
        tokenizeMacsyma('ab@cd');
        fail('expected LexerError');
      } on LexerError catch (e) {
        expect((e.line, e.column), (1, 3));
        expect(e.errorId, isNotEmpty);
      }
    });

    test('empty input yields only EOF', () {
      final tokens = tokenizeMacsyma('');
      expect(tokens, hasLength(1));
      expect(tokens.single.type, 'EOF');
    });

    test('factory tokenizes directly', () {
      final tokens = createMacsymaLexer('x;').tokenize();
      expect(tokens.last.type, 'EOF');
      expect(
          () => createMacsymaLexer('@').tokenize(), throwsA(isA<LexerError>()));
    });
  });
}
