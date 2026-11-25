/// Unit tests for `coding_adventures_wolfram_lexer`: all 21
/// reference tests, plus the corners the reference leaves uncovered —
/// error positions, the nullable API shape, and empty input.
library;

import 'package:coding_adventures_lexer/lexer.dart';
import 'package:coding_adventures_wolfram_lexer/wolfram_lexer.dart';
import 'package:test/test.dart';

List<(String, String)> lex(String source) => [
      for (final t in tokenizeWolfram(source))
        if (t.type != 'EOF' && t.type != 'NEWLINE') (t.type, t.value)
    ];

List<String> types(List<(String, String)> pairs) =>
    [for (final (t, _) in pairs) t];

int newlineCount(String source) =>
    tokenizeWolfram(source).where((t) => t.type == 'NEWLINE').length;

void main() {
  group('application and lists', () {
    test('application uses square brackets', () {
      final p = lex('Sin[x]\n');
      expect(p[0], ('NAME', 'Sin'));
      expect(p[1], ('LBRACKET', '['));
      expect(p[2], ('NAME', 'x'));
      expect(p[3], ('RBRACKET', ']'));
    });

    test('list uses braces', () {
      final p = lex('{a, b, c}\n');
      expect(p[0], ('LBRACE', '{'));
      expect(p[1], ('NAME', 'a'));
      expect(p[2], ('COMMA', ','));
      expect(p[3], ('NAME', 'b'));
      expect(p.last.$1, 'RBRACE');
    });
  });

  group('blanks', () {
    test('named blank is name then blank', () {
      final p = lex('x_\n');
      expect(p[0], ('NAME', 'x'));
      expect(p[1], ('BLANK', '_'));
    });

    test('typed blank is blank then name', () {
      final p = lex('_h\n');
      expect(p[0], ('BLANK', '_'));
      expect(p[1], ('NAME', 'h'));
      final q = lex('x_Integer\n');
      expect(types(q), ['NAME', 'BLANK', 'NAME']);
    });
  });

  group('operators', () {
    test('replacement operators are single tokens', () {
      expect(lex('x /. r\n')[1], ('REPLACEALL', '/.'));
      expect(lex('a -> b\n')[1], ('RULE', '->'));
      expect(lex('a :> b\n')[1], ('RULEDELAYED', ':>'));
      expect(lex('x := y\n')[1], ('SETDELAYED', ':='));
      expect(types(lex('a/.b\n')), contains('REPLACEALL'));
      expect(types(lex('a/.b\n')), isNot(contains('SLASH')));
    });

    test('map and apply sugar are single tokens', () {
      expect(lex('f /@ x\n')[1], ('MAP', '/@'));
      expect(lex('f @@ x\n')[1], ('APPLY', '@@'));
      expect(types(lex('f/@x\n')), contains('MAP'));
      expect(types(lex('f/@x\n')), isNot(contains('SLASH')));
      expect(types(lex('f@@x\n')), contains('APPLY'));
    });

    test('double bracket opener wins, closer is two singles', () {
      final p = lex('x[[2]]\n');
      expect(p[0], ('NAME', 'x'));
      expect(p[1], ('LDBRACKET', '[['));
      expect(p[2], ('NUMBER', '2'));
      expect(p[3], ('RBRACKET', ']'));
      expect(p[4], ('RBRACKET', ']'));
      expect(types(lex('x[[2]]\n')), isNot(contains('LBRACKET')));
      expect(types(lex('f[x]\n')), contains('LBRACKET'));
      expect(types(lex('f[g[x]]\n')), [
        'NAME',
        'LBRACKET',
        'NAME',
        'LBRACKET',
        'NAME',
        'RBRACKET',
        'RBRACKET'
      ]);
    });

    test('slot hash and slotsequence are distinct', () {
      final p = lex('#\n');
      expect(p[0], ('HASH', '#'));
      final pp = lex('##\n');
      expect(types(pp), ['SLOTSEQ']);
      expect(pp[0], ('SLOTSEQ', '##'));
      expect(types(lex('##\n')), isNot(contains('HASH')));
    });

    test('numbered slot is hash then number', () {
      final p = lex('#2\n');
      expect(p[0], ('HASH', '#'));
      expect(p[1], ('NUMBER', '2'));
    });

    test('ampersand postfix distinct from and', () {
      expect(lex('#^2 &\n')[3], ('AMP', '&'));
      expect(types(lex('a && b\n')), contains('AND'));
      expect(types(lex('a && b\n')), isNot(contains('AMP')));
    });

    test('replacerepeated wins over replaceall and slash', () {
      final p = lex('e //. r\n');
      expect(types(p), ['NAME', 'REPLACEREPEATED', 'NAME']);
      expect(p[1], ('REPLACEREPEATED', '//.'));
      expect(types(lex('e /. r\n')), contains('REPLACEALL'));
      expect(types(lex('e /. r\n')), isNot(contains('REPLACEREPEATED')));
    });

    test('condition wins over slash then semi', () {
      final p = lex('p /; t\n');
      expect(types(p), ['NAME', 'CONDITION', 'NAME']);
      expect(p[1], ('CONDITION', '/;'));
      expect(types(lex('a / b\n')), contains('SLASH'));
      expect(types(lex('a; b\n')), contains('SEMI'));
    });

    test('alternatives is single pipe distinct from or', () {
      expect(lex('a | b\n')[1], ('ALTERNATIVES', '|'));
      expect(types(lex('a || b\n')), contains('OR'));
      expect(types(lex('a || b\n')), isNot(contains('ALTERNATIVES')));
    });

    test('patterntest question mark lexes', () {
      final p = lex('_?EvenQ\n');
      expect(types(p), ['BLANK', 'PATTERNTEST', 'NAME']);
      expect(p[1], ('PATTERNTEST', '?'));
    });

    test('pure function full form lexes', () {
      expect(types(lex('(#1 + #2) &\n')), [
        'LPAREN',
        'HASH',
        'NUMBER',
        'PLUS',
        'HASH',
        'NUMBER',
        'RPAREN',
        'AMP'
      ]);
    });

    test('comparison logic and arithmetic operators', () {
      for (final (src, ty) in [
        ('a == b\n', 'EQUAL'),
        ('a != b\n', 'UNEQUAL'),
        ('a <= b\n', 'LE'),
        ('a >= b\n', 'GE'),
        ('a && b\n', 'AND'),
        ('a || b\n', 'OR'),
        ('a ^ b\n', 'POWER'),
        ('a * b\n', 'TIMES'),
      ]) {
        expect(types(lex(src)), contains(ty));
      }
    });
  });

  group('comments and newlines', () {
    test('block comment is skipped', () {
      final p = lex('1 (* a comment *) + 2\n');
      expect(types(p), ['NUMBER', 'PLUS', 'NUMBER']);
    });

    test('newline inside double brackets is dropped', () {
      expect(newlineCount('x[[\n 1\n]]\n'), 1);
    });

    test('newline inside any bracket dropped, top level kept', () {
      expect(newlineCount('f[\n a\n]\n'), 1);
      expect(newlineCount('{\n1,\n2\n}\n'), 1);
      expect(newlineCount('a\nb\n'), 2);
    });
  });

  group('literals and errors', () {
    test('numbers strings and symbols', () {
      expect(lex('3.14\n')[0], ('NUMBER', '3.14'));
      expect(lex('"hi"\n')[0], ('STRING', 'hi'));
      expect(lex('Plus\n')[0], ('NAME', 'Plus'));
      expect(lex('plus\n')[0], ('NAME', 'plus'));
    });

    test('unknown char is an error', () {
      expect(tryTokenizeWolfram('x ~ y\n'), isNull);
    });

    test('errors carry positions', () {
      try {
        tokenizeWolfram('ab~cd');
        fail('expected LexerError');
      } on LexerError catch (e) {
        expect((e.line, e.column), (1, 3));
        expect(e.errorId, isNotEmpty);
      }
    });

    test('nullable API mirrors the throwing API', () {
      expect(tryTokenizeWolfram('Sin[x]\n'), isNotNull);
      expect(() => tokenizeWolfram('x ~ y\n'), throwsA(isA<LexerError>()));
    });

    test('empty input yields only EOF', () {
      final tokens = tokenizeWolfram('');
      expect(tokens, hasLength(1));
      expect(tokens.single.type, 'EOF');
    });

    test('factory tokenizes directly', () {
      final tokens = createWolframLexer('Sin[x]\n').tokenize();
      expect(tokens.last.type, 'EOF');
      expect(
          () => createWolframLexer('~').tokenize(), throwsA(isA<LexerError>()));
    });
  });
}
