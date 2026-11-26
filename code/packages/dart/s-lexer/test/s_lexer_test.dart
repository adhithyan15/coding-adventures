/// Unit tests for `coding_adventures_s_lexer`: all 24 reference
/// tests, plus the corners the reference leaves uncovered — error
/// positions, the nullable API shape, and empty input.
library;

import 'package:coding_adventures_lexer/lexer.dart';
import 'package:coding_adventures_s_lexer/s_lexer.dart';
import 'package:test/test.dart';

List<(String, String)> lex(String source) => [
      for (final t in tokenizeS(source))
        if (t.type != 'EOF') (t.type, t.value)
    ];

List<(String, String)> lexNl(String source) => [
      for (final t in tokenizeS(source))
        if (t.type != 'EOF' && t.type != 'NEWLINE') (t.type, t.value)
    ];

int newlineCount(String source) =>
    tokenizeS(source).where((t) => t.type == 'NEWLINE').length;

bool hasKeyword(String source, String value) =>
    tokenizeS(source).any((t) => t.type == 'KEYWORD' && t.value == value);

void main() {
  group('assignment', () {
    test('arrow assignment', () {
      final p = lexNl('x <- 1\n');
      expect(p[0], ('NAME', 'x'));
      expect(p[1], ('ASSIGN', '<-'));
      expect(p[2], ('NUMBER', '1'));
    });

    test('historical underscore assignment', () {
      final p = lexNl('x _ 1\n');
      expect(p[0], ('NAME', 'x'));
      expect(p[1], ('UNDERSCORE', '_'));
      expect(p[2], ('NUMBER', '1'));
    });

    test('right assignment arrow', () {
      final p = lexNl('1 -> x\n');
      expect(p[1], ('RIGHT_ASSIGN', '->'));
    });

    test('super assignment', () {
      final p = lexNl('x <<- 1\n');
      expect(p[1], ('SUPER_ASSIGN', '<<-'));
    });
  });

  group('numbers and strings', () {
    test('number forms', () {
      for (final n in ['42', '3.14', '.5', '1e3', '1.5e-3']) {
        final p = lexNl('x <- $n\n');
        expect(p[2], ('NUMBER', n));
      }
    });

    test('both string quote styles', () {
      expect(lexNl('"hello"\n')[0].$1, 'STRING');
      expect(lexNl("'world'\n")[0].$1, 'STRING');
    });
  });

  group('identifiers', () {
    test('dotted names are one token', () {
      final p = lexNl('data.frame\n');
      expect(p[0], ('NAME', 'data.frame'));
    });

    test('underscore splits a name', () {
      final p = lexNl('a_b\n');
      expect(p[0], ('NAME', 'a'));
      expect(p[1], ('UNDERSCORE', '_'));
      expect(p[2], ('NAME', 'b'));
    });
  });

  group('keywords', () {
    test('control keywords promote', () {
      for (final kw in [
        'if',
        'else',
        'for',
        'while',
        'repeat',
        'function',
        'break',
        'next',
        'in',
      ]) {
        expect(hasKeyword('$kw\n', kw), isTrue);
      }
    });

    test('constant keywords promote', () {
      for (final kw in [
        'TRUE',
        'FALSE',
        'T',
        'F',
        'NULL',
        'NA',
        'Inf',
        'NaN'
      ]) {
        expect(hasKeyword('$kw\n', kw), isTrue);
      }
    });

    test('case-sensitive true versus name', () {
      final p = lexNl('True\n');
      expect(p[0], ('NAME', 'True'));
    });
  });

  group('operators', () {
    test('arithmetic and sequence operators', () {
      final p = lexNl('a + b - c * d / e ^ f : g\n');
      final names = [for (final (t, _) in p) t];
      for (final op in ['PLUS', 'MINUS', 'STAR', 'SLASH', 'CARET', 'COLON']) {
        expect(names, contains(op));
      }
    });

    test('multichar comparisons not split', () {
      final p = lexNl('a <= b >= c == d != e\n');
      expect(p[1], ('LE', '<='));
      expect(p[3], ('GE', '>='));
      expect(p[5], ('EQEQ', '=='));
      expect(p[7], ('NE', '!='));
    });

    test('single char relops and equals', () {
      final p = lexNl('a < b > c\n');
      expect(p[1], ('LT', '<'));
      expect(p[3], ('GT', '>'));
      final q = lexNl('f(x = 1)\n');
      expect(q.any((t) => t.$1 == 'EQ' && t.$2 == '='), isTrue);
    });

    test('brackets braces parens and separators', () {
      final p = lexNl('f(x)[1]{}\n');
      final names = [for (final (t, _) in p) t];
      for (final b in [
        'LPAREN',
        'RPAREN',
        'LBRACKET',
        'RBRACKET',
        'LBRACE',
        'RBRACE',
      ]) {
        expect(names, contains(b));
      }
    });
  });

  group('comments and whitespace', () {
    test('line comments are stripped', () {
      final p = lex('x <- 1 # assign one\n');
      expect(p.any((t) => t.$2.contains('assign')), isFalse);
      expect(p[0], ('NAME', 'x'));
    });

    test('whitespace is insignificant', () {
      expect(lex('x<-c(1,2)\n'), lex('x  <-  c( 1 , 2 )\n'));
    });
  });

  group('newlines', () {
    test('top level newlines are kept', () {
      expect(newlineCount('x <- 1\ny <- 2\n'), 2);
    });

    test('newlines inside parens are dropped', () {
      expect(newlineCount('sum(1,\n2,\n3)\n'), 1);
      final p = lexNl('sum(1,\n2,\n3)\n');
      expect(p.last.$1, 'RPAREN');
    });

    test('newlines inside braces are kept', () {
      expect(newlineCount('{\nx <- 1\ny <- 2\n}\n'), greaterThanOrEqualTo(3));
    });

    test('newlines inside brackets are dropped', () {
      expect(newlineCount('x[1,\n2]\n'), 1);
    });
  });

  group('end to end and errors', () {
    test('canonical session line', () {
      final p = lexNl('x <- c(1, 2, 3)\n');
      expect(p, [
        ('NAME', 'x'),
        ('ASSIGN', '<-'),
        ('NAME', 'c'),
        ('LPAREN', '('),
        ('NUMBER', '1'),
        ('COMMA', ','),
        ('NUMBER', '2'),
        ('COMMA', ','),
        ('NUMBER', '3'),
        ('RPAREN', ')'),
      ]);
    });

    test('always ends with EOF', () {
      expect(tokenizeS('1\n').last.type, 'EOF');
    });

    test('unknown character is an error', () {
      expect(tryTokenizeS('x <- @\n'), isNull);
    });

    test('errors carry positions', () {
      try {
        tokenizeS('ab@cd');
        fail('expected LexerError');
      } on LexerError catch (e) {
        expect((e.line, e.column), (1, 3));
        expect(e.errorId, isNotEmpty);
      }
    });

    test('nullable API mirrors the throwing API', () {
      expect(tryTokenizeS('x <- 1\n'), isNotNull);
      expect(() => tokenizeS('x <- @\n'), throwsA(isA<LexerError>()));
    });

    test('empty input yields only EOF', () {
      final tokens = tokenizeS('');
      expect(tokens, hasLength(1));
      expect(tokens.single.type, 'EOF');
    });

    test('factory tokenizes directly', () {
      final tokens = createSLexer('x <- 1\n').tokenize();
      expect(tokens.last.type, 'EOF');
      expect(() => createSLexer('@').tokenize(), throwsA(isA<LexerError>()));
    });
  });
}
