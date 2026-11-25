/// Unit tests for `coding_adventures_maple_lexer`: all 28 reference
/// tests, plus the corners the reference leaves uncovered — error
/// positions, the nullable API shape, and empty input.
library;

import 'package:coding_adventures_lexer/lexer.dart';
import 'package:coding_adventures_maple_lexer/maple_lexer.dart';
import 'package:test/test.dart';

List<(String, String)> lex(String source) => [
      for (final t in tokenizeMaple(source))
        if (t.type != 'EOF') (t.type, t.value)
    ];

List<String> types(List<(String, String)> pairs) =>
    [for (final (t, _) in pairs) t];

void main() {
  group('calls and assignment', () {
    test('function application uses parens', () {
      expect(lex('f(x, y);'), [
        ('NAME', 'f'),
        ('LPAREN', '('),
        ('NAME', 'x'),
        ('COMMA', ','),
        ('NAME', 'y'),
        ('RPAREN', ')'),
        ('SEMI', ';'),
      ]);
    });

    test('assign and eq are distinct', () {
      expect(lex('x := 5;')[1], ('ASSIGN', ':='));
      expect(lex('x = 4;')[1], ('EQ', '='));
      expect(types(lex('x = 4;')), isNot(contains('ASSIGN')));
    });

    test('assign wins over colon and eq', () {
      final p = lex('x:=5;');
      expect(types(p), contains('ASSIGN'));
      expect(types(p), isNot(contains('COLON')));
      expect(types(p), isNot(contains('EQ')));
    });
  });

  group('arrows', () {
    test('arrow wins over minus and greater', () {
      final p = lex('f := (x, y) -> x + y;');
      expect(types(p), contains('ARROW'));
      expect(types(p), isNot(contains('GREATER')));
    });

    test('arrow definition snippet end to end', () {
      expect(types(lex('f := (x, y) -> x + y;')), [
        'NAME',
        'ASSIGN',
        'LPAREN',
        'NAME',
        'COMMA',
        'NAME',
        'RPAREN',
        'ARROW',
        'NAME',
        'PLUS',
        'NAME',
        'SEMI',
      ]);
    });
  });

  group('comparisons', () {
    test('not-equal wins over less and greater', () {
      final p = lex('a <> b;');
      expect(types(p), contains('NEQ'));
      expect(types(p), isNot(contains('LESS')));
      expect(types(p), isNot(contains('GREATER')));
    });

    test('le and ge win over bare brackets', () {
      final p1 = lex('a <= b;');
      expect(types(p1), contains('LE'));
      expect(types(p1), isNot(contains('LESS')));
      final p2 = lex('a >= b;');
      expect(types(p2), contains('GE'));
      expect(types(p2), isNot(contains('GREATER')));
    });

    test('bare less and greater lex alone', () {
      expect(types(lex('a < b;')), contains('LESS'));
      expect(types(lex('a > b;')), contains('GREATER'));
    });
  });

  group('brackets', () {
    test('list literal with square brackets', () {
      expect(types(lex('[a, b, c];')), [
        'LBRACKET',
        'NAME',
        'COMMA',
        'NAME',
        'COMMA',
        'NAME',
        'RBRACKET',
        'SEMI',
      ]);
    });

    test('set literal with curly braces', () {
      expect(types(lex('{a, b, c};')), [
        'LBRACE',
        'NAME',
        'COMMA',
        'NAME',
        'COMMA',
        'NAME',
        'RBRACE',
        'SEMI',
      ]);
    });

    test('list and set brackets stay distinct', () {
      expect(types(lex('[x];'))[0], 'LBRACKET');
      expect(types(lex('{x};'))[0], 'LBRACE');
    });
  });

  group('operators', () {
    test('arithmetic and power operators', () {
      for (final (src, ty) in [
        ('a + b;', 'PLUS'),
        ('a - b;', 'MINUS'),
        ('a * b;', 'TIMES'),
        ('a / b;', 'SLASH'),
        ('a ^ b;', 'CARET'),
      ]) {
        expect(types(lex(src)), contains(ty));
      }
    });

    test('double star is two times, never pow', () {
      final p = lex('a ** b;');
      expect(types(p).where((t) => t == 'TIMES'), hasLength(2));
      expect(types(p), isNot(contains('POW')));
      expect(types(p), isNot(contains('CARET')));
    });
  });

  group('terminators', () {
    test('semi and colon are distinct terminators', () {
      expect(lex('a := 1;').last.$1, 'SEMI');
      expect(lex('a := 1:').last.$1, 'COLON');
    });

    test('colon never collides with assign', () {
      expect(types(lex('a := 1: b := 2;')), [
        'NAME',
        'ASSIGN',
        'NUMBER',
        'COLON',
        'NAME',
        'ASSIGN',
        'NUMBER',
        'SEMI',
      ]);
    });
  });

  group('numbers and names', () {
    test('numbers and names', () {
      expect(lex('42;')[0], ('NUMBER', '42'));
      expect(lex('1.5;')[0], ('NUMBER', '1.5'));
      expect(lex('x;')[0], ('NAME', 'x'));
    });

    test('leading dot is not a number', () {
      expect(tryTokenizeMaple('.5;'), isNull);
    });

    test('names are case-sensitive', () {
      expect(lex('Sin;')[0], ('NAME', 'Sin'));
      expect(lex('sin;')[0], ('NAME', 'sin'));
    });
  });

  group('keywords', () {
    test('logical keywords promote', () {
      expect(lex('a and b or not c;'), [
        ('NAME', 'a'),
        ('KEYWORD', 'and'),
        ('NAME', 'b'),
        ('KEYWORD', 'or'),
        ('KEYWORD', 'not'),
        ('NAME', 'c'),
        ('SEMI', ';'),
      ]);
    });

    test('conditional keywords promote', () {
      expect(types(lex('if a then b elif c then d else e end;')), [
        'KEYWORD',
        'NAME',
        'KEYWORD',
        'NAME',
        'KEYWORD',
        'NAME',
        'KEYWORD',
        'NAME',
        'KEYWORD',
        'NAME',
        'KEYWORD',
        'SEMI',
      ]);
    });

    test('fi keyword promotes', () {
      final p = lex('if x > 0 then 1 else -1 fi');
      expect(types(p), contains('KEYWORD'));
      expect(p.last, ('KEYWORD', 'fi'));
    });

    test('end and if are two tokens', () {
      final p = lex('if true then 1 else 0 end if');
      expect(p[p.length - 2], ('KEYWORD', 'end'));
      expect(p[p.length - 1], ('KEYWORD', 'if'));
    });

    test('boolean literals promote', () {
      final p = lex('true; false;');
      expect(p[0], ('KEYWORD', 'true'));
      expect(p[2], ('KEYWORD', 'false'));
    });

    test('uppercase spellings stay names', () {
      expect(types(lex('AND OR NOT IF THEN ELIF ELSE END FI TRUE FALSE;')), [
        'NAME',
        'NAME',
        'NAME',
        'NAME',
        'NAME',
        'NAME',
        'NAME',
        'NAME',
        'NAME',
        'NAME',
        'NAME',
        'SEMI',
      ]);
    });
  });

  group('whitespace and programs', () {
    test('newlines are plain whitespace', () {
      final p = lex('a := 1;\nb := 2:\n');
      expect(types(p), isNot(contains('NEWLINE')));
    });

    test('if elif end-to-end snippet', () {
      expect(types(lex('if x > 0 then 1 elif x < 0 then -1 else 0 end if;')), [
        'KEYWORD',
        'NAME',
        'GREATER',
        'NUMBER',
        'KEYWORD',
        'NUMBER',
        'KEYWORD',
        'NAME',
        'LESS',
        'NUMBER',
        'KEYWORD',
        'MINUS',
        'NUMBER',
        'KEYWORD',
        'NUMBER',
        'KEYWORD',
        'KEYWORD',
        'SEMI',
      ]);
    });

    test('list and set call snippet', () {
      expect(types(lex('g([1, 2, 3], {1, 2, 2});')), [
        'NAME',
        'LPAREN',
        'LBRACKET',
        'NUMBER',
        'COMMA',
        'NUMBER',
        'COMMA',
        'NUMBER',
        'RBRACKET',
        'COMMA',
        'LBRACE',
        'NUMBER',
        'COMMA',
        'NUMBER',
        'COMMA',
        'NUMBER',
        'RBRACE',
        'RPAREN',
        'SEMI',
      ]);
    });
  });

  group('errors', () {
    test('unknown char is an error', () {
      expect(tryTokenizeMaple('x ~ y;'), isNull);
    });

    test('errors carry positions', () {
      try {
        tokenizeMaple('ab~cd');
        fail('expected LexerError');
      } on LexerError catch (e) {
        expect((e.line, e.column), (1, 3));
        expect(e.errorId, isNotEmpty);
      }
    });

    test('nullable API mirrors the throwing API', () {
      expect(tryTokenizeMaple('a := 1;'), isNotNull);
      expect(() => tokenizeMaple('x ~ y;'), throwsA(isA<LexerError>()));
    });

    test('empty input yields only EOF', () {
      final tokens = tokenizeMaple('');
      expect(tokens, hasLength(1));
      expect(tokens.single.type, 'EOF');
    });

    test('factory tokenizes directly', () {
      final tokens = createMapleLexer('a := 1;').tokenize();
      expect(tokens.last.type, 'EOF');
      expect(
          () => createMapleLexer('~').tokenize(), throwsA(isA<LexerError>()));
    });
  });
}
