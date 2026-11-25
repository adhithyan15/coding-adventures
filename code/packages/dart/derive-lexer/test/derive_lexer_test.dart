/// Port of the reference `rust/derive-lexer` unit suite (13 tests).
///
/// Like the reference `lex` helper, [lex] drops both EOF and NEWLINE;
/// the hook's own test counts NEWLINEs on the full stream instead.
library;

import 'package:coding_adventures_derive_lexer/derive_lexer.dart';
import 'package:test/test.dart';

/// Non-EOF, non-NEWLINE (type, value) pairs of [source].
List<(String, String)> lex(String source) => [
      for (final t in tokenizeDerive(source))
        if (t.type != 'EOF' && t.type != 'NEWLINE') (t.type, t.value),
    ];

List<String> types(List<(String, String)> p) => [for (final t in p) t.$1];

void main() {
  group('derive-lexer', () {
    test('function application uses ordinary parens', () {
      final p = lex('DIF(u, x)\n');
      expect(p[0], equals(('NAME', 'DIF')));
      expect(p[1], equals(('LPAREN', '(')));
      expect(p[2], equals(('NAME', 'u')));
      expect(p[3], equals(('COMMA', ',')));
      expect(p[4], equals(('NAME', 'x')));
      expect(p[5], equals(('RPAREN', ')')));
    });

    test('assign token is shared by variable and function definition', () {
      expect(lex('x := 5\n')[1], equals(('ASSIGN', ':=')));
      expect(lex('F(x) := x^2 + 1\n')[4], equals(('ASSIGN', ':=')));
    });

    test('eq is equation distinct from assign', () {
      final p = lex('x = 4\n');
      expect(p[1], equals(('EQ', '=')));
      expect(types(p), isNot(contains('ASSIGN')));
    });

    test('vector literal lexes with brackets and commas', () {
      expect(
        types(lex('[a, b, c]\n')),
        equals([
          'LBRACKET',
          'NAME',
          'COMMA',
          'NAME',
          'COMMA',
          'NAME',
          'RBRACKET',
        ]),
      );
    });

    test('matrix literal row separator is semi', () {
      expect(
        types(lex('[a, b; c, d]\n')),
        equals([
          'LBRACKET',
          'NAME',
          'COMMA',
          'NAME',
          'SEMI',
          'NAME',
          'COMMA',
          'NAME',
          'RBRACKET',
        ]),
      );
    });

    test('boolean keywords promote to keyword token type', () {
      expect(
        lex('a AND b OR NOT c\n'),
        equals([
          ('NAME', 'a'),
          ('KEYWORD', 'AND'),
          ('NAME', 'b'),
          ('KEYWORD', 'OR'),
          ('KEYWORD', 'NOT'),
          ('NAME', 'c'),
        ]),
      );
    });

    test('lowercase boolean words are ordinary names not keywords', () {
      // Derive is case-sensitive: `and`/`or`/`not` in lowercase are NOT
      // the reserved boolean keywords — only exact-case AND/OR/NOT are.
      expect(
        types(lex('and or not\n')),
        equals(['NAME', 'NAME', 'NAME']),
      );
    });

    test('comparison and arithmetic operators', () {
      for (final (src, ty) in [
        ('a <= b\n', 'LE'),
        ('a >= b\n', 'GE'),
        ('a < b\n', 'LESS'),
        ('a > b\n', 'GREATER'),
        ('a ^ b\n', 'POWER'),
        ('a * b\n', 'TIMES'),
        ('a / b\n', 'SLASH'),
      ]) {
        expect(
          types(lex(src)),
          contains(ty),
          reason: 'expected $ty in $src',
        );
      }
    });

    test('assign wins over any shorter prefix', () {
      expect(types(lex('x:=5\n')), contains('ASSIGN'));
      expect(types(lex('x:=5\n')), isNot(contains('EQ')));
    });

    test('names are case sensitive', () {
      expect(lex('SIN\n')[0], equals(('NAME', 'SIN')));
      expect(lex('sin\n')[0], equals(('NAME', 'sin')));
    });

    test('newline inside parens or brackets is dropped top level kept', () {
      final inParen = tokenizeDerive('F(\n a,\n b\n)\n')
          .where((t) => t.type == 'NEWLINE')
          .length;
      expect(inParen, equals(1),
          reason: 'only the trailing top-level newline remains');

      final inBracket = tokenizeDerive('[\n1,\n2\n]\n')
          .where((t) => t.type == 'NEWLINE')
          .length;
      expect(inBracket, equals(1));

      final top =
          tokenizeDerive('a\nb\n').where((t) => t.type == 'NEWLINE').length;
      expect(top, equals(2),
          reason: 'two top-level statements keep their separating newline');
    });

    test('numbers and symbols', () {
      expect(lex('3.14\n')[0], equals(('NUMBER', '3.14')));
      expect(lex('42\n')[0], equals(('NUMBER', '42')));
      expect(lex('x\n')[0], equals(('NAME', 'x')));
    });

    test('unknown char is an error', () {
      expect(tryTokenizeDerive('x ~ y\n'), isNull);
    });
  });
}
