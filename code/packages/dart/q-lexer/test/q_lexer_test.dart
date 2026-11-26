/// Unit tests for `coding_adventures_q_lexer`: all 5 in-module
/// reference tests and all 47 integration tests, plus the corners the
/// reference leaves uncovered — error positions, the nullable API
/// shape, and empty input.
library;

import 'package:coding_adventures_lexer/lexer.dart';
import 'package:coding_adventures_q_lexer/q_lexer.dart';
import 'package:test/test.dart';

List<String> types(String source) => [
      for (final t in tokenizeQ(source))
        if (t.type != 'EOF') t.type
    ];

List<String> values(String source) => [
      for (final t in tokenizeQ(source))
        if (t.type != 'EOF') t.value
    ];

void main() {
  group('unit', () {
    test('space before minus only folds to negative literal', () {
      expect(types('2 -1'), ['NUMBER', 'NUMBER']);
      expect(values('2 -1'), ['2', '-1']);
    });

    test('space on both sides of minus stays subtraction', () {
      expect(types('2 - 1'), ['NUMBER', 'MINUS', 'NUMBER']);
    });

    test('no space at all stays subtraction', () {
      expect(types('2-1'), ['NUMBER', 'MINUS', 'NUMBER']);
      expect(values('2-1'), ['2', '-', '1']);
    });

    test('space after minus only is subtraction', () {
      expect(types('2- 1'), ['NUMBER', 'MINUS', 'NUMBER']);
    });

    test('negative literal after assignment colon', () {
      expect(types('x:-1'), ['NAME', 'COLON', 'NUMBER']);
      expect(values('x:-1'), ['x', ':', '-1']);
    });
  });

  group('verbs', () {
    test('every primitive verb glyph', () {
      expect(types('+ - * % ! , # _ & | ~'), [
        'PLUS',
        'MINUS',
        'STAR',
        'PERCENT',
        'BANG',
        'COMMA',
        'HASH',
        'UNDERSCORE',
        'AMP',
        'PIPE',
        'TILDE',
      ]);
    });

    test('all six comparison glyphs', () {
      expect(types('= < > <= >= <>'), ['EQ', 'LT', 'GT', 'LE', 'GE', 'NE']);
    });

    test('not-equal is angle brackets', () {
      expect(types('<>'), ['NE']);
      expect(types('~='), ['TILDE', 'EQ']);
      expect(types('#'), ['HASH']);
    });

    test('le ge digraphs win over bare lt gt', () {
      expect(types('<='), ['LE']);
      expect(types('<'), ['LT']);
      expect(types('>='), ['GE']);
      expect(types('>'), ['GT']);
    });

    test('the three adverbs', () {
      expect(types("'"), ['EACH']);
      expect(types('+/'), ['PLUS', 'REDUCE']);
      expect(types('+\\'), ['PLUS', 'SCAN']);
    });

    test('colon assignment', () {
      expect(types('x:1'), ['NAME', 'COLON', 'NUMBER']);
    });

    test('parenthesised grouping', () {
      expect(types('(1+2)'), ['LPAREN', 'NUMBER', 'PLUS', 'NUMBER', 'RPAREN']);
    });

    test('explicit list literal with semicolons', () {
      expect(types('(1;2;3)'), [
        'LPAREN',
        'NUMBER',
        'SEMICOLON',
        'NUMBER',
        'SEMICOLON',
        'NUMBER',
        'RPAREN',
      ]);
    });

    test('function literal with explicit params', () {
      expect(types('{[x;y] x+y}'), [
        'LBRACE',
        'LBRACKET',
        'NAME',
        'SEMICOLON',
        'NAME',
        'RBRACKET',
        'NAME',
        'PLUS',
        'NAME',
        'RBRACE',
      ]);
    });

    test('function literal with implicit params', () {
      expect(types('{x+y}'), ['LBRACE', 'NAME', 'PLUS', 'NAME', 'RBRACE']);
    });

    test('semicolon separates statements in bodies too', () {
      expect(types('{a:1;a+1}'), [
        'LBRACE',
        'NAME',
        'COLON',
        'NUMBER',
        'SEMICOLON',
        'NAME',
        'PLUS',
        'NUMBER',
        'RBRACE',
      ]);
    });
  });

  group('numbers', () {
    test('integers and decimals', () {
      expect(types('42'), ['NUMBER']);
      expect(values('3.14'), ['3.14']);
    });

    test('numbers with exponents', () {
      expect(values('1e10'), ['1e10']);
      expect(values('1e-5'), ['1e-5']);
      expect(values('2.5E+3'), ['2.5E+3']);
    });

    test('dense numeric arrays strand', () {
      expect(types('1 2 3'), ['NUMBER', 'NUMBER', 'NUMBER']);
      expect(values('1 2 3'), ['1', '2', '3']);
    });

    test('underscore is its own primitive, never in names', () {
      expect(types('a_b'), ['NAME', 'UNDERSCORE', 'NAME']);
      expect(values('a_b'), ['a', '_', 'b']);
    });
  });

  group('negative literals', () {
    test('space before minus none after folds', () {
      expect(types('2 -1'), ['NUMBER', 'NUMBER']);
      expect(values('2 -1'), ['2', '-1']);
    });

    test('space on both sides is subtraction', () {
      expect(types('2 - 1'), ['NUMBER', 'MINUS', 'NUMBER']);
    });

    test('no space at all is also subtraction', () {
      expect(types('2-1'), ['NUMBER', 'MINUS', 'NUMBER']);
      expect(values('2-1'), ['2', '-', '1']);
    });

    test('space after minus only is subtraction', () {
      expect(types('2- 1'), ['NUMBER', 'MINUS', 'NUMBER']);
    });

    test('negative literal at start of line', () {
      expect(types('-1'), ['NUMBER']);
      expect(values('-1'), ['-1']);
    });

    test('negative literal after assignment colon', () {
      expect(types('x:-1'), ['NAME', 'COLON', 'NUMBER']);
      expect(values('x:-1'), ['x', ':', '-1']);
    });

    test('negative literal after open paren and semicolon', () {
      expect(types('(-1;2)'),
          ['LPAREN', 'NUMBER', 'SEMICOLON', 'NUMBER', 'RPAREN']);
      expect(values('(-1;2)'), ['(', '-1', ';', '2', ')']);
    });

    test('subtraction glued to a closing paren', () {
      expect(types('(2+3)-1'),
          ['LPAREN', 'NUMBER', 'PLUS', 'NUMBER', 'RPAREN', 'MINUS', 'NUMBER']);
    });

    test('negative literal stranded after a spaced closing paren', () {
      expect(types('(2+3) -1'),
          ['LPAREN', 'NUMBER', 'PLUS', 'NUMBER', 'RPAREN', 'NUMBER']);
      expect(values('(2+3) -1'), ['(', '2', '+', '3', ')', '-1']);
    });

    test('subtraction glued to a name', () {
      expect(types('f-1'), ['NAME', 'MINUS', 'NUMBER']);
    });

    test('negative literal stranded after a spaced name', () {
      expect(types('f -1'), ['NAME', 'NUMBER']);
    });

    test('chained minus signs resolve independently', () {
      expect(types('1 - -1'), ['NUMBER', 'MINUS', 'NUMBER']);
      expect(values('1 - -1'), ['1', '-', '-1']);
    });

    test('three element negative strand', () {
      expect(types('2 -1 -2'), ['NUMBER', 'NUMBER', 'NUMBER']);
      expect(values('2 -1 -2'), ['2', '-1', '-2']);
    });

    test('negative literal after a function literal value', () {
      expect(types('{x+1} -1'),
          ['LBRACE', 'NAME', 'PLUS', 'NUMBER', 'RBRACE', 'NUMBER']);
      expect(types('{x+1}-1'),
          ['LBRACE', 'NAME', 'PLUS', 'NUMBER', 'RBRACE', 'MINUS', 'NUMBER']);
    });

    test('negative decimals and exponents fold too', () {
      expect(values('x: -3.5'), ['x', ':', '-3.5']);
      expect(values('2 -1e10'), ['2', '-1e10']);
    });
  });

  group('comments', () {
    test('reduce glued to a preceding verb', () {
      expect(types('+/x'), ['PLUS', 'REDUCE', 'NAME']);
    });

    test('reduce glued to a preceding noun', () {
      expect(types('x/'), ['NAME', 'REDUCE']);
    });

    test('comment at start of line', () {
      expect(types('/ comment to end of line'), isEmpty);
    });

    test('comment preceded by a single space', () {
      expect(types('x:1 / trailing comment'), ['NAME', 'COLON', 'NUMBER']);
    });

    test('comment preceded by many spaces behaves the same', () {
      expect(types('x:1     / trailing comment'), ['NAME', 'COLON', 'NUMBER']);
      expect(
          types('x:1 / trailing comment'), types('x:1     / trailing comment'));
    });

    test('reduce right after a closing paren', () {
      expect(types('(+/)y'), ['LPAREN', 'PLUS', 'REDUCE', 'RPAREN', 'NAME']);
    });

    test('comment after a closing paren with a space', () {
      expect(types('(1+2) / a comment'),
          ['LPAREN', 'NUMBER', 'PLUS', 'NUMBER', 'RPAREN']);
    });

    test('comment does not swallow the newline', () {
      expect(types('x:1 / a comment\ny:2'),
          ['NAME', 'COLON', 'NUMBER', 'NEWLINE', 'NAME', 'COLON', 'NUMBER']);
    });

    test('whole line comment between statements', () {
      expect(types('x:1\n/ a whole comment line\ny:2'), [
        'NAME',
        'COLON',
        'NUMBER',
        'NEWLINE',
        'NEWLINE',
        'NAME',
        'COLON',
        'NUMBER'
      ]);
    });

    test('comment content is never re-lexed', () {
      expect(tryTokenizeQ('x:1 / cost is \$5 @ 10% off\ny:2'), isNotNull);
      expect(types('x:1 / cost is \$5 @ 10% off\ny:2'),
          ['NAME', 'COLON', 'NUMBER', 'NEWLINE', 'NAME', 'COLON', 'NUMBER']);
    });

    test('scan and each ignore the comment rule', () {
      expect(types('+\\ / a comment'), ['PLUS', 'SCAN']);
      expect(types("+' / a comment"), ['PLUS', 'EACH']);
    });
  });

  group('newlines and errors', () {
    test('newline is its own significant token', () {
      expect(types('x:1\ny:2'),
          ['NAME', 'COLON', 'NUMBER', 'NEWLINE', 'NAME', 'COLON', 'NUMBER']);
    });

    test('skips ordinary whitespace', () {
      expect(types('x   :   1'), ['NAME', 'COLON', 'NUMBER']);
    });

    test('unrecognized character is an error', () {
      expect(tryTokenizeQ('a@b'), isNull);
      expect(tryTokenizeQ('a?b'), isNull);
      expect(tryTokenizeQ('a.b'), isNull);
      expect(tryTokenizeQ('a\$b'), isNull);
    });

    test('throwing API throws on malformed source', () {
      expect(() => tokenizeQ('@'), throwsA(isA<LexerError>()));
    });

    test('factory exposes the throwing API', () {
      expect(() => createQLexer('x:1').tokenize(), returnsNormally);
      expect(() => createQLexer('@').tokenize(), throwsA(isA<LexerError>()));
    });

    test('small end to end snippet', () {
      final src = 'x:2 -1\nsum:{[a] +/a}\nsum x';
      expect(types(src), [
        'NAME',
        'COLON',
        'NUMBER',
        'NUMBER',
        'NEWLINE',
        'NAME',
        'COLON',
        'LBRACE',
        'LBRACKET',
        'NAME',
        'RBRACKET',
        'PLUS',
        'REDUCE',
        'NAME',
        'RBRACE',
        'NEWLINE',
        'NAME',
        'NAME',
      ]);
      expect(values(src)[2], '2');
      expect(values(src)[3], '-1');
    });
  });

  group('extras', () {
    test('errors carry positions', () {
      try {
        tokenizeQ('ab@cd');
        fail('expected LexerError');
      } on LexerError catch (e) {
        expect((e.line, e.column), (1, 3));
        expect(e.errorId, isNotEmpty);
      }
    });

    test('nullable API mirrors the throwing API', () {
      expect(tryTokenizeQ('x:2 -1\n'), isNotNull);
      expect(() => tokenizeQ('a@b'), throwsA(isA<LexerError>()));
    });

    test('empty input yields only EOF', () {
      final tokens = tokenizeQ('');
      expect(tokens, hasLength(1));
      expect(tokens.single.type, 'EOF');
    });
  });
}
