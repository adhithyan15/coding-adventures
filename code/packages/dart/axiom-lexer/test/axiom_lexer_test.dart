/// Unit tests for `coding_adventures_axiom_lexer`: all 6 in-module
/// reference tests and all 56 integration tests, plus the corners the
/// reference leaves uncovered — error positions, the nullable API
/// shape, and empty input.
library;

import 'package:coding_adventures_axiom_lexer/axiom_lexer.dart';
import 'package:coding_adventures_lexer/lexer.dart';
import 'package:test/test.dart';

List<String> types(String source) => [
      for (final t in tokenizeAxiom(source))
        if (t.type != 'EOF') t.type
    ];

List<String> values(String source) => [
      for (final t in tokenizeAxiom(source))
        if (t.type != 'EOF') t.value
    ];

void main() {
  group('smoke', () {
    test('declaration and immediate assignment', () {
      expect(types('a : PositiveInteger\na := 3'),
          ['NAME', 'COLON', 'NAME', 'NAME', 'ASSIGN', 'NUMBER']);
    });

    test('coercion', () {
      expect(
          types('3 :: Fraction Integer'), ['NUMBER', 'COERCE', 'NAME', 'NAME']);
    });

    test('has query', () {
      expect(types('Polynomial(Integer) has Ring'),
          ['NAME', 'LPAREN', 'NAME', 'RPAREN', 'KEYWORD', 'NAME']);
      expect(values('Polynomial(Integer) has Ring')[4], 'has');
    });

    test('rational division is three ordinary tokens', () {
      expect(types('1/3'), ['NUMBER', 'SLASH', 'NUMBER']);
    });

    test('tokenizeAxiom throws on malformed source', () {
      expect(() => tokenizeAxiom('@'), throwsA(isA<LexerError>()));
    });

    test('factory exposes the nullable API', () {
      expect(createAxiomLexer('x := 1').tokenize(), isNotEmpty);
      expect(
          () => createAxiomLexer('@').tokenize(), throwsA(isA<LexerError>()));
      expect(tryTokenizeAxiom('x := 1'), isNotNull);
      expect(tryTokenizeAxiom('@'), isNull);
    });
  });

  group('numbers', () {
    test('integer literal', () {
      expect(types('123'), ['NUMBER']);
      expect(values('123'), ['123']);
    });

    test('float literal', () {
      expect(types('1.5'), ['NUMBER']);
      expect(values('1.5'), ['1.5']);
    });

    test('exponent numbers', () {
      expect(values('1e10'), ['1e10']);
      expect(values('2.5E+3'), ['2.5E+3']);
      expect(values('1e-5'), ['1e-5']);
    });

    test('leading digit required', () {
      expect(tryTokenizeAxiom('.5'), isNull);
    });

    test('rational spelling is three tokens', () {
      expect(types('1/3'), ['NUMBER', 'SLASH', 'NUMBER']);
      expect(values('1/3'), ['1', '/', '3']);
    });

    test('rational coercion', () {
      expect(
          types('3 :: Fraction Integer'), ['NUMBER', 'COERCE', 'NAME', 'NAME']);
    });
  });

  group('strings', () {
    test('string literal', () {
      expect(types('"hello"'), ['STRING']);
      expect(values('"hello"'), ['hello']);
    });

    test('empty string literal', () {
      expect(types('""'), ['STRING']);
      expect(values('""'), ['']);
    });

    test('string containing grammar characters', () {
      expect(values('"a := b :: c"'), ['a := b :: c']);
    });
  });

  group('names', () {
    test('plain identifiers', () {
      expect(types('x foo f'), ['NAME', 'NAME', 'NAME']);
      expect(values('x foo f'), ['x', 'foo', 'f']);
    });

    test('domain names are names, never keywords', () {
      for (final name in [
        'Integer',
        'Boolean',
        'Float',
        'String',
        'Fraction',
        'Polynomial',
        'List',
        'PositiveInteger',
        'NonNegativeInteger',
        'Ring',
        'OrderedSet',
      ]) {
        expect(types(name), ['NAME']);
      }
    });

    test('identifiers are case-sensitive', () {
      expect(types('IF'), ['NAME']);
      expect(types('Has'), ['NAME']);
    });
  });

  group('brackets', () {
    test('parens and call shape', () {
      expect(types('f(a, b)'),
          ['NAME', 'LPAREN', 'NAME', 'COMMA', 'NAME', 'RPAREN']);
    });

    test('list literal', () {
      expect(types('[a, b, c]'),
          ['LBRACKET', 'NAME', 'COMMA', 'NAME', 'COMMA', 'NAME', 'RBRACKET']);
    });

    test('paren-optional call is name number', () {
      expect(types('factorial 7'), ['NAME', 'NUMBER']);
    });
  });

  group('operators', () {
    test('ordinary arithmetic', () {
      expect(types('a + b - c * d / e'), [
        'NAME',
        'PLUS',
        'NAME',
        'MINUS',
        'NAME',
        'TIMES',
        'NAME',
        'SLASH',
        'NAME',
      ]);
    });

    test('both power spellings', () {
      expect(types('a ^ b'), ['NAME', 'CARET', 'NAME']);
      expect(types('a ** b'), ['NAME', 'POW', 'NAME']);
    });

    test('pow wins over bare times', () {
      expect(types('**'), ['POW']);
      expect(types('*'), ['TIMES']);
      expect(types('***'), ['POW', 'TIMES']);
      expect(types('****'), ['POW', 'POW']);
    });
  });

  group('comparisons', () {
    test('equality lowers to EQ', () {
      expect(types('a = b'), ['NAME', 'EQ', 'NAME']);
    });

    test('not-equal spells tilde-equals', () {
      expect(types('a ~= b'), ['NAME', 'NE', 'NAME']);
    });

    test('foreign not-equal spellings split or error', () {
      expect(types('a <> b'), ['NAME', 'LESS', 'GREATER', 'NAME']);
      expect(tryTokenizeAxiom('a != b'), isNull);
    });

    test('ordering comparisons', () {
      expect(types('a < b'), ['NAME', 'LESS', 'NAME']);
      expect(types('a <= b'), ['NAME', 'LE', 'NAME']);
      expect(types('a > b'), ['NAME', 'GREATER', 'NAME']);
      expect(types('a >= b'), ['NAME', 'GE', 'NAME']);
    });

    test('le and ge win over bare brackets', () {
      expect(types('<='), ['LE']);
      expect(types('>='), ['GE']);
      expect(types('<'), ['LESS']);
      expect(types('>'), ['GREATER']);
    });
  });

  group('colons', () {
    test('immediate assignment', () {
      expect(types('a := 3'), ['NAME', 'ASSIGN', 'NUMBER']);
    });

    test('held-body function definition', () {
      expect(types('f(x: T): T == x + 1'), [
        'NAME',
        'LPAREN',
        'NAME',
        'COLON',
        'NAME',
        'RPAREN',
        'COLON',
        'NAME',
        'DEFINE',
        'NAME',
        'PLUS',
        'NUMBER',
      ]);
    });

    test('undeclared function definition', () {
      expect(types('f x == x * x'),
          ['NAME', 'NAME', 'DEFINE', 'NAME', 'TIMES', 'NAME']);
    });

    test('plain declaration', () {
      expect(types('a : PositiveInteger'), ['NAME', 'COLON', 'NAME']);
    });

    test('tuple declaration', () {
      expect(types('(a, b, c) : T'), [
        'LPAREN',
        'NAME',
        'COMMA',
        'NAME',
        'COMMA',
        'NAME',
        'RPAREN',
        'COLON',
        'NAME',
      ]);
    });

    test('coercion expression', () {
      expect(types('e :: T'), ['NAME', 'COERCE', 'NAME']);
    });

    test('colon family longest-match regression', () {
      expect(types(':='), ['ASSIGN']);
      expect(types('::'), ['COERCE']);
      expect(types(':'), ['COLON']);
      expect(types(':::'), ['COERCE', 'COLON']);
      expect(types('::='), ['COERCE', 'EQ']);
      expect(types(':=:'), ['ASSIGN', 'COLON']);
    });

    test('define wins over bare eq', () {
      expect(types('=='), ['DEFINE']);
      expect(types('='), ['EQ']);
      expect(types('==='), ['DEFINE', 'EQ']);
      expect(types('===='), ['DEFINE', 'DEFINE']);
    });
  });

  group('keywords', () {
    test('true has query', () {
      expect(types('Polynomial(Integer) has Ring'),
          ['NAME', 'LPAREN', 'NAME', 'RPAREN', 'KEYWORD', 'NAME']);
      expect(values('Polynomial(Integer) has Ring')[4], 'has');
    });

    test('false has query', () {
      expect(types('List(Integer) has Ring'),
          ['NAME', 'LPAREN', 'NAME', 'RPAREN', 'KEYWORD', 'NAME']);
    });

    test('has is case-sensitive', () {
      expect(types('has'), ['KEYWORD']);
      expect(types('Has'), ['NAME']);
      expect(types('HAS'), ['NAME']);
    });

    test('if then else conditional', () {
      expect(types('if a > b then a else b'), [
        'KEYWORD',
        'NAME',
        'GREATER',
        'NAME',
        'KEYWORD',
        'NAME',
        'KEYWORD',
        'NAME'
      ]);
    });

    test('closed four-word keyword set', () {
      for (final kw in ['if', 'then', 'else', 'has']) {
        expect(types(kw), ['KEYWORD']);
        expect(values(kw), [kw]);
      }
    });
  });

  group('blocks and comments', () {
    test('semicolon-separated block', () {
      expect(types('(a := 1; b := 2; a + b)'), [
        'LPAREN',
        'NAME',
        'ASSIGN',
        'NUMBER',
        'SEMI',
        'NAME',
        'ASSIGN',
        'NUMBER',
        'SEMI',
        'NAME',
        'PLUS',
        'NAME',
        'RPAREN',
      ]);
    });

    test('double dash opens a comment', () {
      expect(
          types('a := 1 -- a trailing comment'), ['NAME', 'ASSIGN', 'NUMBER']);
    });

    test('comment does not eat the next line', () {
      expect(types('a := 1 -- comment\nb := 2'),
          ['NAME', 'ASSIGN', 'NUMBER', 'NAME', 'ASSIGN', 'NUMBER']);
    });

    test('whole-line comment between statements', () {
      expect(types('a := 1\n-- a whole comment line\nb := 2'),
          ['NAME', 'ASSIGN', 'NUMBER', 'NAME', 'ASSIGN', 'NUMBER']);
    });

    test('comment content is never re-lexed', () {
      expect(tryTokenizeAxiom('a := 1 -- cost is @5 \$ off \\ end\nb := 2'),
          isNotNull);
    });

    test('single minus is not a comment', () {
      expect(types('a - b'), ['NAME', 'MINUS', 'NAME']);
      expect(types('a-b'), ['NAME', 'MINUS', 'NAME']);
    });

    test('newlines are ordinary whitespace', () {
      expect(types('a := 1\nb := 2'),
          ['NAME', 'ASSIGN', 'NUMBER', 'NAME', 'ASSIGN', 'NUMBER']);
    });

    test('ordinary whitespace vanishes', () {
      expect(types('a   :=   1'), ['NAME', 'ASSIGN', 'NUMBER']);
    });
  });

  group('deferred surface', () {
    test('package and target markers error', () {
      expect(tryTokenizeAxiom('content(2)\$Polynomial(Integer)'), isNull);
      expect(tryTokenizeAxiom('(2 = 3)@Boolean'), isNull);
    });

    test('undeclared multi-char sequences decompose', () {
      expect(types('x +-> x * x'),
          ['NAME', 'PLUS', 'MINUS', 'GREATER', 'NAME', 'TIMES', 'NAME']);
      expect(types('=>'), ['EQ', 'GREATER']);
    });

    test('macro is an ordinary name', () {
      expect(types('macro'), ['NAME']);
    });

    test('record union any are ordinary names', () {
      expect(types('Record'), ['NAME']);
      expect(types('Union'), ['NAME']);
      expect(types('Any'), ['NAME']);
    });
  });

  group('errors', () {
    test('unrecognized characters error', () {
      expect(tryTokenizeAxiom('a @ b'), isNull);
      expect(tryTokenizeAxiom('a \$ b'), isNull);
      expect(tryTokenizeAxiom('a ! b'), isNull);
      expect(tryTokenizeAxiom('a ? b'), isNull);
    });

    test('errors carry positions', () {
      try {
        tokenizeAxiom('ab@cd');
        fail('expected LexerError');
      } on LexerError catch (e) {
        expect((e.line, e.column), (1, 3));
        expect(e.errorId, isNotEmpty);
      }
    });

    test('unterminated string completes without throwing', () {
      // try_ never throws by construction; the reference asserts the
      // same no-panic property by discarding the result.
      expect(() => tryTokenizeAxiom('a := "oops'), returnsNormally);
    });

    test('empty input yields only EOF', () {
      final tokens = tokenizeAxiom('');
      expect(tokens, hasLength(1));
      expect(tokens.single.type, 'EOF');
    });
  });

  group('scale', () {
    test('deep nesting never recurses', () {
      const depth = 50000;
      final src = '${'(' * depth}a${')' * depth}';
      final tokens = tokenizeAxiom(src);
      expect(tokens.length, depth * 2 + 2);
      expect(tokens[0].type, 'LPAREN');
      expect(tokens[depth].type, 'NAME');
      expect(tokens[depth + 1].type, 'RPAREN');
    });

    test('wide flat streams stay correct', () {
      const count = 100000;
      final src = '${'a + ' * count}a';
      final tokens = tokenizeAxiom(src);
      expect(tokens.length, count * 2 + 2);
    });

    test('very long comments stay linear', () {
      final body = 'x' * 500000;
      expect(types('a := 1 -- $body\nb := 2'),
          ['NAME', 'ASSIGN', 'NUMBER', 'NAME', 'ASSIGN', 'NUMBER']);
    });
  });

  group('end to end', () {
    test('textbook session snippet', () {
      final src = 'a : PositiveInteger -- declare a domain\n'
          'a := 3\n'
          'f(x: Integer): Integer == x * x\n'
          'if a > 0 then f(a) else 0\n'
          'Polynomial(Integer) has Ring';
      expect(types(src), [
        'NAME',
        'COLON',
        'NAME',
        'NAME',
        'ASSIGN',
        'NUMBER',
        'NAME',
        'LPAREN',
        'NAME',
        'COLON',
        'NAME',
        'RPAREN',
        'COLON',
        'NAME',
        'DEFINE',
        'NAME',
        'TIMES',
        'NAME',
        'KEYWORD',
        'NAME',
        'GREATER',
        'NUMBER',
        'KEYWORD',
        'NAME',
        'LPAREN',
        'NAME',
        'RPAREN',
        'KEYWORD',
        'NUMBER',
        'NAME',
        'LPAREN',
        'NAME',
        'RPAREN',
        'KEYWORD',
        'NAME',
      ]);
    });
  });
}
