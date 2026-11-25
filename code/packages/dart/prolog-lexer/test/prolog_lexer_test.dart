/// Unit tests for `coding_adventures_prolog_lexer`: all 17
/// reference tests, plus the corners the reference leaves uncovered —
/// quoted/symbolic atom values, error positions, and factory-direct
/// use.
library;

import 'package:coding_adventures_lexer/lexer.dart';
import 'package:coding_adventures_prolog_lexer/prolog_lexer.dart';
import 'package:test/test.dart';

List<(String, String)> pairs(String source) => [
      for (final t in tokenizeIsoProlog(source))
        if (t.type != 'EOF') (t.type, t.value)
    ];

List<String> names(String source) => [for (final (n, _) in pairs(source)) n];

List<String> values(String source) => [for (final (_, v) in pairs(source)) v];

void main() {
  group('facts and rules', () {
    test('empty source yields only EOF', () {
      final toks = tokenizeIsoProlog('');
      expect(toks.last.type, 'EOF');
      expect(pairs(''), isEmpty);
    });

    test('whitespace and comments vanish', () {
      expect(pairs('   % a comment\n   '), isEmpty);
    });

    test('simple fact tokens in order', () {
      expect(names('father(homer, bart).'),
          ['ATOM', 'LPAREN', 'ATOM', 'COMMA', 'ATOM', 'RPAREN', 'DOT']);
      expect(values('father(homer, bart).'),
          ['father', '(', 'homer', ',', 'bart', ')', '.']);
    });

    test('rule arrow is its own token', () {
      expect(names('a :- b.'), ['ATOM', 'RULE', 'ATOM', 'DOT']);
    });

    test('query arrow is its own token', () {
      expect(names('?- foo(X).'),
          ['QUERY', 'ATOM', 'LPAREN', 'VARIABLE', 'RPAREN', 'DOT']);
    });

    test('dcg arrow spans three characters', () {
      expect(names('greet --> [hello].'), contains('DCG'));
    });

    test('complete small program tokenizes', () {
      final src = 'father(homer, bart).\n'
          'parent(X, Y) :- father(X, Y).\n'
          '?- parent(homer, Who).\n';
      final toks = tokenizeIsoProlog(src);
      int count(String label) => toks.where((t) => t.type == label).length;
      expect(count('RULE'), 1);
      expect(count('QUERY'), 1);
      expect(count('DOT'), 3);
      expect(toks.last.type, 'EOF');
    });
  });

  group('numbers', () {
    test('integer and float literals', () {
      expect(names('42 3.14 2.5e-3'), ['INTEGER', 'FLOAT', 'FLOAT']);
    });

    test('integer followed by dot is not a float', () {
      expect(names('42.'), ['INTEGER', 'DOT']);
    });
  });

  group('atoms and variables', () {
    test('lone underscore is anon var', () {
      expect(names('_'), ['ANON_VAR']);
    });

    test('underscore-led identifier is a variable', () {
      expect(names('_State'), ['VARIABLE']);
    });

    test('uppercase identifier is a variable', () {
      expect(names('X'), ['VARIABLE']);
    });

    test('lowercase identifier is an atom', () {
      expect(names('chest_pain'), ['ATOM']);
    });

    test('quoted atom aliases to atom, quotes kept', () {
      // Quote-stripping applies to names ending in STRING only (both
      // lanes, proven by differential probe) — ATOM keeps its quotes.
      expect(names("'Hello World'"), ['ATOM']);
      expect(values("'Hello World'"), ["'Hello World'"]);
    });

    test('double-quoted string strips quotes', () {
      expect(names('"x"'), ['STRING']);
      expect(values('"x"'), ['x']);
    });

    test('symbolic atom aliases to atom', () {
      expect(names('>='), ['ATOM']);
      expect(values('>='), ['>=']);
      // `$` is symbolic but letters are not: the run splits.
      expect(values(r'$x'), [r'$', 'x']);
      expect(names(r'$x'), ['ATOM', 'ATOM']);
    });

    test('naf and prob-sep win over symbolic', () {
      expect(names(r'\+'), ['NAF']);
      expect(names('::'), ['PROB_SEP']);
    });
  });

  group('structure', () {
    test('list brackets and pipe', () {
      expect(names('[H | T]'),
          ['LBRACKET', 'VARIABLE', 'BAR', 'VARIABLE', 'RBRACKET']);
    });

    test('cut is its own token', () {
      expect(names('!'), ['CUT']);
    });
  });

  group('errors', () {
    test('unrecognized input throws positioned errors', () {
      // No backtick token exists in this cut.
      expect(() => tokenizeIsoProlog('`'), throwsA(isA<LexerError>()));
      try {
        tokenizeIsoProlog('a`b');
        fail('expected LexerError');
      } on LexerError catch (e) {
        expect((e.line, e.column), (1, 2));
        expect(e.errorId, isNotEmpty);
      }
    });

    test('factory tokenizes directly', () {
      final tokens = createIsoPrologLexer('a.').tokenize();
      expect(tokens.last.type, 'EOF');
      expect(() => createIsoPrologLexer('`').tokenize(),
          throwsA(isA<LexerError>()));
    });
  });
}
