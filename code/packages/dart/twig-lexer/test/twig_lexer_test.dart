/// Unit tests for `coding_adventures_twig_lexer`: all 22 reference
/// tests, plus the corners the reference leaves uncovered — the
/// grammar spec accessor, keyword values, error positions, and empty
/// input.
library;

import 'package:coding_adventures_lexer/lexer.dart';
import 'package:coding_adventures_twig_lexer/twig_lexer.dart';
import 'package:test/test.dart';

List<String> typeNames(String source) => [
      for (final t in tokenizeTwig(source))
        if (t.type != 'EOF') t.type
    ];

List<String> values(String source) => [
      for (final t in tokenizeTwig(source))
        if (t.type != 'EOF') t.value
    ];

void main() {
  group('atoms', () {
    test('empty source is only EOF', () {
      final toks = tokenizeTwig('');
      expect(toks.length, 1);
      expect(toks[0].type, 'EOF');
    });

    test('parens and quote', () {
      expect(typeNames("(')"), ['LPAREN', 'QUOTE', 'RPAREN']);
    });

    test('integer literal', () {
      expect(typeNames('42'), ['INTEGER']);
      expect(values('42'), ['42']);
    });

    test('negative integer literal', () {
      expect(typeNames('-7'), ['INTEGER']);
      expect(values('-7'), ['-7']);
    });

    test('booleans are distinct', () {
      expect(typeNames('#t'), ['BOOL_TRUE']);
      expect(typeNames('#f'), ['BOOL_FALSE']);
    });

    test('name token', () {
      expect(typeNames('foo'), ['NAME']);
      expect(values('foo'), ['foo']);
    });

    test('operator names lex as name', () {
      for (final op in ['+', '-', '*', '/', '=', '<', '>', 'null?', 'pair?']) {
        expect(typeNames(op), ['NAME']);
      }
    });
  });

  group('keywords', () {
    test('keywords promote with values intact', () {
      for (final kw in [
        'define',
        'lambda',
        'let',
        'if',
        'begin',
        'quote',
        'nil'
      ]) {
        final toks = tokenizeTwig(kw);
        expect(toks[0].type, 'KEYWORD');
        expect(toks[0].value, kw);
      }
    });

    test('keyword inside form keeps its kind', () {
      final toks = tokenizeTwig('(define x 42)');
      expect(toks[1].type, 'KEYWORD');
      expect(toks[1].value, 'define');
    });

    test('bare minus lexes as name', () {
      expect(typeNames('(- 3 1)'),
          ['LPAREN', 'NAME', 'INTEGER', 'INTEGER', 'RPAREN']);
      expect(tokenizeTwig('(- 3 1)')[1].value, '-');
    });
  });

  group('trivia and positions', () {
    test('comment to EOL skipped', () {
      expect(typeNames('; comment\n42'), ['INTEGER']);
    });

    test('whitespace skipped', () {
      expect(typeNames('   42   '), ['INTEGER']);
    });

    test('first token is at 1:1', () {
      final toks = tokenizeTwig('foo');
      expect((toks[0].line, toks[0].column), (1, 1));
    });

    test('newline resets column, advances line', () {
      final toks = tokenizeTwig('a\nb');
      expect((toks[0].line, toks[0].column), (1, 1));
      expect((toks[1].line, toks[1].column), (2, 1));
    });
  });

  group('programs', () {
    test('factorial tokenises cleanly', () {
      final kinds =
          typeNames('(define (fact n) (if (= n 0) 1 (* n (fact (- n 1)))))');
      expect(kinds, contains('KEYWORD'));
      expect(kinds, contains('NAME'));
      expect(kinds, contains('INTEGER'));
    });

    test('quoted symbol tokenises', () {
      expect(typeNames("(eq? 'foo)"),
          ['LPAREN', 'NAME', 'QUOTE', 'NAME', 'RPAREN']);
    });

    test('colon lexes as colon', () {
      expect(typeNames(':'), ['COLON']);
      expect(values(':'), [':']);
    });

    test('arrow lexes as arrow', () {
      expect(typeNames('->'), ['ARROW']);
      expect(values('->'), ['->']);
    });

    test('annotations appear in sequence', () {
      final kinds = typeNames('(x : (Int 0 128)) -> (Int 0 256)');
      expect(kinds, contains('COLON'));
      expect(kinds, contains('ARROW'));
    });

    test('bare minus still name after arrow added', () {
      expect(typeNames('-'), ['NAME']);
      expect(values('-'), ['-']);
    });
  });

  group('errors', () {
    test('stray at sign throws, never panics the process', () {
      expect(() => tokenizeTwig('@'), throwsA(isA<LexerError>()));
    });

    test('non-ASCII unicode throws', () {
      expect(() => tokenizeTwig('(+ 1 €)'), throwsA(isA<LexerError>()));
    });

    test('errors carry positions', () {
      try {
        tokenizeTwig('ab@cd');
        fail('expected LexerError');
      } on LexerError catch (e) {
        expect((e.line, e.column), (1, 3));
        expect(e.errorId, isNotEmpty);
      }
    });
  });

  group('grammar spec', () {
    test('spec exposes the compiled keyword table', () {
      final spec = twigTokenGrammarSpec();
      for (final kw in ['define', 'lambda', 'if', 'nil']) {
        expect(spec.keywords, contains(kw));
      }
      expect(spec.definitions, isNotEmpty);
    });
  });
}
