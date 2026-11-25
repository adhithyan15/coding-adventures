/// Port of the reference `rust/mccarthy-lisp-lexer` unit suite (11
/// tests), plus lane extras pinning the factory and token-shape
/// invariants.
library;

import 'package:coding_adventures_lexer/lexer.dart';
import 'package:coding_adventures_mccarthy_lisp_lexer/mccarthy_lisp_lexer.dart';
import 'package:test/test.dart';

/// Token names (minus the trailing EOF) for a source string.
List<String> names(String src) => [
      for (final t in tokenizeMcCarthy(src))
        if (t.type != 'EOF') t.type,
    ];

void main() {
  group('mccarthy-lisp-lexer', () {
    test('tokenizes the canonical car example', () {
      expect(
        names("(CAR '(A B C))"),
        equals([
          'LPAREN',
          'SYMBOL',
          'QUOTE',
          'LPAREN',
          'SYMBOL',
          'SYMBOL',
          'SYMBOL',
          'RPAREN',
          'RPAREN',
        ]),
      );
    });

    test('values are preserved', () {
      final values = [
        for (final t in tokenizeMcCarthy('(CONS A NIL)'))
          if (t.type != 'EOF') t.value,
      ];
      expect(values, equals(['(', 'CONS', 'A', 'NIL', ')']));
    });

    test('negative integers are one token', () {
      expect(names('-42'), equals(['INTEGER']));
      expect(tokenizeMcCarthy('-42')[0].value, equals('-42'));
    });

    test('dotted pair separator', () {
      expect(
        names('(A . B)'),
        equals(['LPAREN', 'SYMBOL', 'DOT', 'SYMBOL', 'RPAREN']),
      );
    });

    test('hyphenated symbol is one token', () {
      expect(names('LIST-OF-3'), equals(['SYMBOL']));
      expect(tokenizeMcCarthy('LIST-OF-3')[0].value, equals('LIST-OF-3'));
    });

    test('comments and whitespace are skipped', () {
      expect(
          names('  CAR ; take the head\n  CDR'), equals(['SYMBOL', 'SYMBOL']));
    });

    test('bare minus is a lex error', () {
      expect(() => tokenizeMcCarthy('(- A B)'), throwsA(isA<LexerError>()));
    });

    test('lowercase is a lex error', () {
      expect(() => tokenizeMcCarthy('car'), throwsA(isA<LexerError>()));
    });

    test('position tracking is one based', () {
      final toks = tokenizeMcCarthy('CAR\n CDR');
      expect((toks[0].line, toks[0].column), equals((1, 1)));
      expect((toks[1].line, toks[1].column), equals((2, 2)));
    });

    test('empty source yields only eof', () {
      final toks = tokenizeMcCarthy('   ; just a comment\n');
      expect(toks.length, equals(1));
      expect(toks[0].type, equals('EOF'));
    });

    test('grammar spec is accessible', () {
      final definitionNames = [
        for (final d in mccarthyTokenGrammarSpec().definitions) d.name,
      ];
      expect(definitionNames, contains('SYMBOL'));
      expect(definitionNames, contains('INTEGER'));
    });

    test('factory lexer tokenizes like the one-shot', () {
      final fromFactory = createMcCarthyLexer("(CAR '(A B C))").tokenize();
      expect(
        [for (final t in fromFactory) (t.type, t.value)],
        equals([
          for (final t in tokenizeMcCarthy("(CAR '(A B C))")) (t.type, t.value),
        ]),
      );
    });

    test('integers and dotted pairs mix', () {
      expect(
        names('(A . -42)'),
        equals(['LPAREN', 'SYMBOL', 'DOT', 'INTEGER', 'RPAREN']),
      );
    });

    test('tokens keep sane positions', () {
      for (final token in tokenizeMcCarthy("(CONS A (CONS B NIL))\n")) {
        expect(token.line, greaterThanOrEqualTo(1));
        expect(token.column, greaterThanOrEqualTo(1));
      }
    });
  });
}
