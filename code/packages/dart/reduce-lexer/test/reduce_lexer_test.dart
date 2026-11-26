/// Unit tests for `coding_adventures_reduce_lexer`: all 17
/// reference tests, plus the corners the reference leaves uncovered —
/// error positions, the nullable API shape, and empty input.
library;

import 'package:coding_adventures_lexer/lexer.dart';
import 'package:coding_adventures_reduce_lexer/reduce_lexer.dart';
import 'package:test/test.dart';

List<(String, String)> lex(String source) => [
      for (final t in tokenizeReduce(source))
        if (t.type != 'EOF') (t.type, t.value)
    ];

List<String> types(List<(String, String)> pairs) =>
    [for (final (t, _) in pairs) t];

void main() {
  group('calls and assignment', () {
    test('function application uses parens', () {
      expect(lex('df(x, y)\$'), [
        ('NAME', 'df'),
        ('LPAREN', '('),
        ('NAME', 'x'),
        ('COMMA', ','),
        ('NAME', 'y'),
        ('RPAREN', ')'),
        ('DOLLAR', '\$'),
      ]);
    });

    test('assign and eq are distinct', () {
      expect(lex('x := 5')[1], ('ASSIGN', ':='));
      expect(lex('h(l, m) := l + m')[6], ('ASSIGN', ':='));
      expect(lex('x = 4')[1], ('EQ', '='));
      expect(types(lex('x = 4')), isNot(contains('ASSIGN')));
      // No DEFINE token exists here: `==` is two EQs.
      expect(types(lex('x == 4')), ['NAME', 'EQ', 'EQ', 'NUMBER']);
    });
  });

  group('lists and cons', () {
    test('list literal with braces and commas', () {
      expect(types(lex('{a, b, c}')),
          ['LBRACE', 'NAME', 'COMMA', 'NAME', 'COMMA', 'NAME', 'RBRACE']);
    });

    test('cons dot distinct from decimal', () {
      expect(types(lex('a . {b, c}')),
          ['NAME', 'DOT', 'LBRACE', 'NAME', 'COMMA', 'NAME', 'RBRACE']);
      expect(lex('3.14\$')[0], ('NUMBER', '3.14'));
    });
  });

  group('keywords', () {
    test('logical and relational keywords promote', () {
      expect(lex('a and b or not c neq d'), [
        ('NAME', 'a'),
        ('KEYWORD', 'and'),
        ('NAME', 'b'),
        ('KEYWORD', 'or'),
        ('KEYWORD', 'not'),
        ('NAME', 'c'),
        ('KEYWORD', 'neq'),
        ('NAME', 'd'),
      ]);
    });

    test('uppercase spellings stay names', () {
      expect(types(lex('AND OR NOT NEQ')), ['NAME', 'NAME', 'NAME', 'NAME']);
    });

    test('if then else promote', () {
      final p = lex('if a > b then a else b');
      expect(p[0], ('KEYWORD', 'if'));
      expect(p[4], ('KEYWORD', 'then'));
      expect(p[6], ('KEYWORD', 'else'));
    });
  });

  group('operators', () {
    test('comparison and arithmetic operators', () {
      for (final (src, ty) in [
        ('a <= b\$', 'LE'),
        ('a >= b\$', 'GE'),
        ('a < b\$', 'LESS'),
        ('a > b\$', 'GREATER'),
        ('a ^ b\$', 'CARET'),
        ('a ** b\$', 'POW'),
        ('a * b\$', 'TIMES'),
        ('a / b\$', 'SLASH'),
      ]) {
        expect(types(lex(src)), contains(ty));
      }
    });

    test('assign wins over shorter prefixes', () {
      expect(types(lex('x:=5\$')), contains('ASSIGN'));
      expect(types(lex('x:=5\$')), isNot(contains('EQ')));
    });

    test('double star wins over two singles', () {
      final p = lex('a**b\$');
      expect(types(p).where((t) => t == 'TIMES'), isEmpty);
      expect(types(p), contains('POW'));
    });

    test('double angle wins over comparisons', () {
      final p = lex('<<a>>\$');
      expect(types(p), contains('GROUP_OPEN'));
      expect(types(p), contains('GROUP_CLOSE'));
      expect(types(p), isNot(contains('LESS')));
      expect(types(p), isNot(contains('GREATER')));
      expect(types(p), isNot(contains('LE')));
      expect(types(p), isNot(contains('GE')));
    });
  });

  group('groups and terminators', () {
    test('group statement delimiters', () {
      final p = lex('<< a := 1; b := 2 >>\$');
      expect(p[0].$1, 'GROUP_OPEN');
      expect(p.last.$1, 'DOLLAR');
      expect(types(p), contains('GROUP_CLOSE'));
      expect(types(p), contains('SEMI'));
    });

    test('semi and dollar are distinct terminators', () {
      expect(lex('a := 1;').last.$1, 'SEMI');
      expect(lex('a := 1\$').last.$1, 'DOLLAR');
    });
  });

  group('whitespace and names', () {
    test('newlines are plain whitespace', () {
      final p = lex('a := 1;\nb := 2\$\n');
      expect(types(p), isNot(contains('NEWLINE')));
    });

    test('names are case-sensitive', () {
      expect(lex('Df\$')[0], ('NAME', 'Df'));
      expect(lex('df\$')[0], ('NAME', 'df'));
    });

    test('numbers and symbols', () {
      expect(lex('3.14\$')[0], ('NUMBER', '3.14'));
      expect(lex('42\$')[0], ('NUMBER', '42'));
      expect(lex('x\$')[0], ('NAME', 'x'));
    });
  });

  group('errors', () {
    test('unknown char is an error', () {
      expect(tryTokenizeReduce('x ~ y\$'), isNull);
    });

    test('errors carry positions', () {
      try {
        tokenizeReduce('ab~cd');
        fail('expected LexerError');
      } on LexerError catch (e) {
        expect((e.line, e.column), (1, 3));
        expect(e.errorId, isNotEmpty);
      }
    });

    test('nullable API mirrors the throwing API', () {
      expect(tryTokenizeReduce('a := 1\$'), isNotNull);
      expect(() => tokenizeReduce('x ~ y\$'), throwsA(isA<LexerError>()));
    });

    test('empty input yields only EOF', () {
      final tokens = tokenizeReduce('');
      expect(tokens, hasLength(1));
      expect(tokens.single.type, 'EOF');
    });

    test('factory tokenizes directly', () {
      final tokens = createReduceLexer('a := 1\$').tokenize();
      expect(tokens.last.type, 'EOF');
      expect(
          () => createReduceLexer('~').tokenize(), throwsA(isA<LexerError>()));
    });
  });
}
