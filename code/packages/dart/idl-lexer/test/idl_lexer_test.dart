/// Unit tests for `coding_adventures_idl_lexer`: all 5 in-module
/// reference tests and all 53 integration tests (two are exact
/// duplicates of in-module tests and share one mirror each), plus the
/// corners the reference leaves uncovered — error positions, the
/// nullable API shape, keyword boundaries, and empty input.
library;

import 'package:coding_adventures_idl_lexer/idl_lexer.dart';
import 'package:coding_adventures_lexer/lexer.dart';
import 'package:test/test.dart';

List<String> types(String source) => [
      for (final t in tokenizeIdl(source))
        if (t.type != 'EOF') t.type
    ];

List<String> values(String source) => [
      for (final t in tokenizeIdl(source))
        if (t.type != 'EOF') t.value
    ];

void main() {
  group('smoke', () {
    test('print statement with single-quoted string', () {
      // PRINT is an ordinary library-routine NAME, not a keyword.
      expect(types("PRINT, 'hello'"), ['NAME', 'COMMA', 'STRING']);
      expect(values("PRINT, 'hello'"), ['PRINT', ',', 'hello']);
    });

    test('keyword lookup folds case, names keep it', () {
      expect(types('if'), ['KEYWORD']);
      expect(values('if'), ['IF']);
      expect(values('MyVar'), ['MyVar']);
    });

    test('slash is always plain division', () {
      expect(types('a/YLOG'), ['NAME', 'SLASH', 'NAME']);
      expect(types('PLOT, x, /YLOG'),
          ['NAME', 'COMMA', 'NAME', 'COMMA', 'SLASH', 'NAME']);
    });

    test('tokenizeIdl throws on malformed source', () {
      expect(() => tokenizeIdl('@'), throwsA(isA<LexerError>()));
    });

    test('factory exposes the nullable API', () {
      expect(createIdlLexer('x = 1').tokenize(), isNotEmpty);
      expect(() => createIdlLexer('@').tokenize(), throwsA(isA<LexerError>()));
      expect(tryTokenizeIdl('x = 1'), isNotNull);
      expect(tryTokenizeIdl('@'), isNull);
    });
  });

  group('comments', () {
    test('semicolon opens a comment to end of line', () {
      expect(types('x = 1 ; a trailing comment'), ['NAME', 'EQUALS', 'NUMBER']);
    });

    test('comment does not swallow the newline', () {
      expect(types('x = 1 ; a comment\ny = 2'),
          ['NAME', 'EQUALS', 'NUMBER', 'NEWLINE', 'NAME', 'EQUALS', 'NUMBER']);
    });

    test('whole line comment between statements', () {
      expect(types('x = 1\n; a whole comment line\ny = 2'), [
        'NAME',
        'EQUALS',
        'NUMBER',
        'NEWLINE',
        'NEWLINE',
        'NAME',
        'EQUALS',
        'NUMBER'
      ]);
    });

    test('comment content is never re-lexed', () {
      expect(tryTokenizeIdl('x = 1 ; cost is @5 \\ off\ny = 2'), isNotNull);
    });
  });

  group('continuation and separator', () {
    test('dollar is its own continuation token', () {
      expect(types('x = 1 + \$\n    2'), [
        'NAME',
        'EQUALS',
        'NUMBER',
        'PLUS',
        'CONTINUATION',
        'NEWLINE',
        'NUMBER'
      ]);
    });

    test('ampersand separates statements', () {
      expect(types('x = 1 & y = 2'),
          ['NAME', 'EQUALS', 'NUMBER', 'STMT_SEP', 'NAME', 'EQUALS', 'NUMBER']);
    });
  });

  group('strings', () {
    test('single-quoted string', () {
      expect(types("'hello'"), ['STRING']);
      expect(values("'hello'"), ['hello']);
    });

    test('double-quoted string', () {
      expect(types('"hello"'), ['STRING']);
      expect(values('"hello"'), ['hello']);
    });

    test('both quote styles share one type', () {
      expect(types("'flux'"), types('"flux"'));
    });

    test('empty strings of both styles', () {
      expect(types("''"), ['STRING']);
      expect(values("''"), ['']);
      expect(types('""'), ['STRING']);
      expect(values('""'), ['']);
    });

    test('either quote nests inside the other style', () {
      expect(values("'say \"hi\"'"), ['say "hi"']);
      expect(values('"it\'s fine"'), ["it's fine"]);
    });
  });

  group('word operators', () {
    test('every comparison operator', () {
      expect(types('EQ NE LT LE GT GE'),
          ['KEYWORD', 'KEYWORD', 'KEYWORD', 'KEYWORD', 'KEYWORD', 'KEYWORD']);
      expect(values('EQ NE LT LE GT GE'), ['EQ', 'NE', 'LT', 'LE', 'GT', 'GE']);
    });

    test('every logical operator', () {
      expect(types('AND OR NOT XOR'),
          ['KEYWORD', 'KEYWORD', 'KEYWORD', 'KEYWORD']);
      expect(values('AND OR NOT XOR'), ['AND', 'OR', 'NOT', 'XOR']);
    });

    test('no symbolic alternative spellings', () {
      expect(tryTokenizeIdl('a < b'), isNull);
      expect(tryTokenizeIdl('a <= b'), isNull);
      // `==` is two EQUALS tokens, not a digraph.
      expect(types('a == b'), ['NAME', 'EQUALS', 'EQUALS', 'NAME']);
    });

    test('word operators fold case', () {
      expect(types('eq'), ['KEYWORD']);
      expect(values('eq'), ['EQ']);
      expect(values('Eq'), ['EQ']);
      expect(values('and'), ['AND']);
    });

    test('comparison expression', () {
      expect(types('IF x EQ 1 THEN y = 2'), [
        'KEYWORD',
        'NAME',
        'KEYWORD',
        'NUMBER',
        'KEYWORD',
        'NAME',
        'EQUALS',
        'NUMBER',
      ]);
    });
  });

  group('keywords', () {
    test('if family', () {
      expect(types('IF THEN ELSE ENDIF ENDELSE'),
          ['KEYWORD', 'KEYWORD', 'KEYWORD', 'KEYWORD', 'KEYWORD']);
      expect(values('IF THEN ELSE ENDIF ENDELSE'),
          ['IF', 'THEN', 'ELSE', 'ENDIF', 'ENDELSE']);
    });

    test('loop families share DO', () {
      expect(values('FOR DO ENDFOR'), ['FOR', 'DO', 'ENDFOR']);
      expect(values('WHILE DO ENDWHILE'), ['WHILE', 'DO', 'ENDWHILE']);
    });

    test('repeat family', () {
      expect(values('REPEAT UNTIL ENDREP'), ['REPEAT', 'UNTIL', 'ENDREP']);
    });

    test('break and continue', () {
      expect(types('BREAK'), ['KEYWORD']);
      expect(types('CONTINUE'), ['KEYWORD']);
    });

    test('begin and generic end', () {
      expect(values('BEGIN END'), ['BEGIN', 'END']);
    });

    test('pro, function, return', () {
      expect(values('PRO FUNCTION RETURN'), ['PRO', 'FUNCTION', 'RETURN']);
    });

    test('keywords fold, names keep exact case', () {
      expect(types('pro'), ['KEYWORD']);
      expect(values('pro'), ['PRO']);
      expect(types('Pro'), ['KEYWORD']);
      expect(values('Pro'), ['PRO']);
      expect(types('MyProc'), ['NAME']);
      expect(values('MyProc'), ['MyProc']);
    });

    test('keyword prefixes stay names', () {
      expect(types('IFFY'), ['NAME']);
      expect(types('ENDOR'), ['NAME']);
      expect(types('FORMAT'), ['NAME']);
    });
  });

  group('definitions and calls', () {
    test('pro definition header and end', () {
      final src = 'PRO GREET, name\n  PRINT, name\nEND';
      expect(types(src), [
        'KEYWORD',
        'NAME',
        'COMMA',
        'NAME',
        'NEWLINE',
        'NAME',
        'COMMA',
        'NAME',
        'NEWLINE',
        'KEYWORD',
      ]);
    });

    test('function definition with return', () {
      final src = 'FUNCTION DOUBLE, x\n  RETURN, x*2\nEND';
      expect(types(src), [
        'KEYWORD',
        'NAME',
        'COMMA',
        'NAME',
        'NEWLINE',
        'KEYWORD',
        'COMMA',
        'NAME',
        'STAR',
        'NUMBER',
        'NEWLINE',
        'KEYWORD',
      ]);
    });

    test('procedure call with keyword args and shorthand', () {
      final src = "PLOT, x, TITLE='flux', /YLOG";
      expect(types(src), [
        'NAME',
        'COMMA',
        'NAME',
        'COMMA',
        'NAME',
        'EQUALS',
        'STRING',
        'COMMA',
        'SLASH',
        'NAME',
      ]);
      expect(values(src)[0], 'PLOT');
      expect(values(src)[6], 'flux');
      expect(values(src)[9], 'YLOG');
    });

    test('slash shapes tokenize identically everywhere', () {
      expect(types('a/YLOG'), ['NAME', 'SLASH', 'NAME']);
      final callTail = types('PLOT, x, /YLOG').sublist(4);
      final divisionTail = types('y = a/YLOG').sublist(3);
      expect(callTail, ['SLASH', 'NAME']);
      expect(divisionTail, ['SLASH', 'NAME']);
      expect(callTail, divisionTail);
    });

    test('if then endif block', () {
      final src = 'IF x EQ 1 THEN BEGIN\n  y = 2\nENDIF';
      expect(types(src), [
        'KEYWORD',
        'NAME',
        'KEYWORD',
        'NUMBER',
        'KEYWORD',
        'KEYWORD',
        'NEWLINE',
        'NAME',
        'EQUALS',
        'NUMBER',
        'NEWLINE',
        'KEYWORD',
      ]);
    });
  });

  group('arrays', () {
    test('array literal', () {
      expect(types('[1, 2, 3]'), [
        'LBRACKET',
        'NUMBER',
        'COMMA',
        'NUMBER',
        'COMMA',
        'NUMBER',
        'RBRACKET'
      ]);
    });

    test('simple subscripting', () {
      expect(types('a[0]'), ['NAME', 'LBRACKET', 'NUMBER', 'RBRACKET']);
    });

    test('negative from-end subscripting', () {
      expect(
          types('a[-1]'), ['NAME', 'LBRACKET', 'MINUS', 'NUMBER', 'RBRACKET']);
    });

    test('ranged and strided subscripts', () {
      expect(types('a[s0:s1]'),
          ['NAME', 'LBRACKET', 'NAME', 'COLON', 'NAME', 'RBRACKET']);
      expect(types('a[s0:s1:n]'), [
        'NAME',
        'LBRACKET',
        'NAME',
        'COLON',
        'NAME',
        'COLON',
        'NAME',
        'RBRACKET'
      ]);
    });

    test('all-rest wildcard subscript', () {
      expect(types('a[*]'), ['NAME', 'LBRACKET', 'STAR', 'RBRACKET']);
      expect(types('a[s0:*]'),
          ['NAME', 'LBRACKET', 'NAME', 'COLON', 'STAR', 'RBRACKET']);
    });

    test('two-dimensional subscripting', () {
      expect(types('a[i, j]'),
          ['NAME', 'LBRACKET', 'NAME', 'COMMA', 'NAME', 'RBRACKET']);
    });
  });

  group('hashes', () {
    test('double hash wins over bare hash', () {
      expect(types('##'), ['HASH_HASH']);
      expect(types('#'), ['HASH']);
    });

    test('matrix products between operands', () {
      expect(types('a # b'), ['NAME', 'HASH', 'NAME']);
      expect(types('a ## b'), ['NAME', 'HASH_HASH', 'NAME']);
    });

    test('three hashes split as double then bare', () {
      expect(types('###'), ['HASH_HASH', 'HASH']);
    });

    test('four hashes split as two doubles', () {
      expect(types('####'), ['HASH_HASH', 'HASH_HASH']);
    });
  });

  group('arithmetic and numbers', () {
    test('ordinary arithmetic operators', () {
      expect(types('a + b - c * d / e ^ f'), [
        'NAME',
        'PLUS',
        'NAME',
        'MINUS',
        'NAME',
        'STAR',
        'NAME',
        'SLASH',
        'NAME',
        'CARET',
        'NAME',
      ]);
    });

    test('equals covers assignment and binding', () {
      expect(types('x = 1'), ['NAME', 'EQUALS', 'NUMBER']);
      expect(types("TITLE='flux'"), ['NAME', 'EQUALS', 'STRING']);
    });

    test('integers and decimals', () {
      expect(types('42'), ['NUMBER']);
      expect(values('3.14'), ['3.14']);
    });

    test('leading-dot floats', () {
      expect(types('.5'), ['NUMBER']);
      expect(values('.5'), ['.5']);
    });

    test('exponent numbers', () {
      expect(values('1e10'), ['1e10']);
      expect(values('1e-5'), ['1e-5']);
      expect(values('2.5E+3'), ['2.5E+3']);
    });

    test('typed suffixes split off the number', () {
      expect(types('5L'), ['NUMBER', 'NAME']);
      expect(values('5L'), ['5', 'L']);
    });
  });

  group('identifiers', () {
    test('names start with a letter', () {
      expect(types('N_ELEMENTS'), ['NAME']);
      expect(values('x9'), ['x9']);
    });

    test('underscore continues names', () {
      expect(types('PTR_NEW'), ['NAME']);
      expect(values('PTR_NEW'), ['PTR_NEW']);
    });

    test('leading underscore has no token', () {
      expect(tryTokenizeIdl('_EXTRA'), isNull);
    });
  });

  group('newlines and errors', () {
    test('newline is its own token', () {
      expect(types('x = 1\ny = 2'),
          ['NAME', 'EQUALS', 'NUMBER', 'NEWLINE', 'NAME', 'EQUALS', 'NUMBER']);
      final tokens = tokenizeIdl('x = 1\ny = 2');
      expect(tokens[3].value, r'\n');
    });

    test('ordinary whitespace vanishes', () {
      expect(types('x   =   1'), ['NAME', 'EQUALS', 'NUMBER']);
    });

    test('out-of-scope characters error', () {
      expect(tryTokenizeIdl('a@b'), isNull);
      expect(tryTokenizeIdl('a?b'), isNull);
      expect(tryTokenizeIdl('s.tag'), isNull);
      expect(tryTokenizeIdl('{1:2}'), isNull);
    });

    test('errors carry positions', () {
      try {
        tokenizeIdl('ab@cd');
        fail('expected LexerError');
      } on LexerError catch (e) {
        expect((e.line, e.column), (1, 3));
        expect(e.errorId, isNotEmpty);
      }
    });

    test('empty input yields only EOF', () {
      final tokens = tokenizeIdl('');
      expect(tokens, hasLength(1));
      expect(tokens.single.type, 'EOF');
    });
  });

  group('end to end', () {
    test('textbook session snippet', () {
      final src = 'PRO GREET, name        ; say hello\n'
          "  PRINT, 'Hello, ', name\n"
          'END';
      expect(types(src), [
        'KEYWORD',
        'NAME',
        'COMMA',
        'NAME',
        'NEWLINE',
        'NAME',
        'COMMA',
        'STRING',
        'COMMA',
        'NAME',
        'NEWLINE',
        'KEYWORD',
      ]);
      expect(values(src)[0], 'PRO');
      expect(values(src)[7], 'Hello, ');
    });
  });
}
