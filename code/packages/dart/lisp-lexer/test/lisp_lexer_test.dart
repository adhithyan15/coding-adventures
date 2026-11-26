/// Unit tests for `coding_adventures_lisp_lexer`: the 35 reference
/// tests plus the extras this lane needs — exact error positions and
/// texts, the dot-digit quirk, the full symbol alphabet, display names,
/// and value semantics.
library;

import 'package:coding_adventures_lisp_lexer/lisp_lexer.dart';
import 'package:test/test.dart';

/// Tokenizes and returns just the types, excluding EOF.
List<TokenType> types(String source) => tokenize(source)
    .where((t) => t.tokenType != TokenType.eof)
    .map((t) => t.tokenType)
    .toList();

/// Tokenizes and returns just the values, excluding EOF.
List<String> values(String source) => tokenize(source)
    .where((t) => t.tokenType != TokenType.eof)
    .map((t) => t.value)
    .toList();

void main() {
  group('atoms', () {
    test('number', () {
      expect(types('42'), [TokenType.number]);
      expect(values('42'), ['42']);
    });

    test('negative number', () {
      expect(types('-7'), [TokenType.number]);
      expect(values('-7'), ['-7']);
    });

    test('zero', () {
      expect(types('0'), [TokenType.number]);
    });

    test('symbol', () {
      expect(types('define'), [TokenType.symbol]);
      expect(values('define'), ['define']);
    });

    test('string', () {
      final tokens = tokenize('"hello world"');
      final nonEof = tokens.where((t) => t.tokenType != TokenType.eof).toList();
      expect(nonEof.length, 1);
      expect(nonEof[0].tokenType, TokenType.string);
      expect(nonEof[0].value, '"hello world"');
    });

    test('string with escape', () {
      final tokens = tokenize(r'"hello \"world\""');
      final nonEof = tokens.where((t) => t.tokenType != TokenType.eof).toList();
      expect(nonEof.length, 1);
      expect(nonEof[0].tokenType, TokenType.string);
      expect(nonEof[0].value, r'"hello \"world\""');
    });

    test('empty string', () {
      expect(values('""'), ['""']);
      expect(types('""'), [TokenType.string]);
    });

    test('string ending in escaped backslash', () {
      expect(values(r'"a\\"'), [r'"a\\"']);
    });

    test('backslash at EOF is unterminated', () {
      expect(
          () => tokenize('"a\\'),
          throwsA(const LexerError(
              message: 'Unterminated string literal', position: 0)));
    });

    test('lone quote is unterminated at zero', () {
      expect(
          () => tokenize('"'),
          throwsA(const LexerError(
              message: 'Unterminated string literal', position: 0)));
    });
  });

  group('operator symbols', () {
    test('plus', () {
      expect(types('+'), [TokenType.symbol]);
      expect(values('+'), ['+']);
    });

    test('minus in expression', () {
      expect(types('(- 3 1)'), [
        TokenType.lParen,
        TokenType.symbol,
        TokenType.number,
        TokenType.number,
        TokenType.rParen,
      ]);
    });

    test('lone minus at end is a symbol', () {
      expect(types('-'), [TokenType.symbol]);
      expect(values('-'), ['-']);
    });

    test('star', () {
      expect(types('*'), [TokenType.symbol]);
    });

    test('slash', () {
      expect(types('/'), [TokenType.symbol]);
    });

    test('equals', () {
      expect(types('='), [TokenType.symbol]);
    });

    test('comparison', () {
      expect(types('< > <= >='), [
        TokenType.symbol,
        TokenType.symbol,
        TokenType.symbol,
        TokenType.symbol,
      ]);
    });

    test('multi char symbol', () {
      expect(types('set!'), [TokenType.symbol]);
      expect(values('set!'), ['set!']);
      expect(types('null?'), [TokenType.symbol]);
      expect(values('null?'), ['null?']);
    });

    test('full symbol alphabet', () {
      // Uppercase, underscore, ampersand, and digits-after-first: every
      // symbol character class in one token stream.
      expect(types('Define AND foo_bar2 a&b'), [
        TokenType.symbol,
        TokenType.symbol,
        TokenType.symbol,
        TokenType.symbol,
      ]);
      expect(values('Define AND foo_bar2 a&b'),
          ['Define', 'AND', 'foo_bar2', 'a&b']);
    });
  });

  group('delimiters', () {
    test('parentheses', () {
      expect(types('()'), [TokenType.lParen, TokenType.rParen]);
    });

    test('quote', () {
      expect(types("'x"), [TokenType.quote, TokenType.symbol]);
    });

    test('dot', () {
      expect(types('(a . b)'), [
        TokenType.lParen,
        TokenType.symbol,
        TokenType.dot,
        TokenType.symbol,
        TokenType.rParen,
      ]);
    });

    test('dot before digit is still dot', () {
      // The reference muses about `.5` but never checks: DOT always.
      expect(types('.5'), [TokenType.dot, TokenType.number]);
    });
  });

  group('whitespace and comments', () {
    test('whitespace skipped', () {
      expect(types('  42  '), [TokenType.number]);
      expect(types('a\tb'), [TokenType.symbol, TokenType.symbol]);
      expect(types('a\nb'), [TokenType.symbol, TokenType.symbol]);
    });

    test('carriage return and form feed skipped', () {
      expect(types('a\rb'), [TokenType.symbol, TokenType.symbol]);
      expect(types('a\fb'), [TokenType.symbol, TokenType.symbol]);
    });

    test('comment skipped', () {
      expect(types('; this is a comment\n42'), [TokenType.number]);
    });

    test('inline comment', () {
      expect(types('(+ 1 2) ; add them'), [
        TokenType.lParen,
        TokenType.symbol,
        TokenType.number,
        TokenType.number,
        TokenType.rParen,
      ]);
    });

    test('comment only, no trailing newline', () {
      expect(types('; nothing here'), isEmpty);
    });
  });

  group('expressions', () {
    test('simple call', () {
      expect(types('(+ 1 2)'), [
        TokenType.lParen,
        TokenType.symbol,
        TokenType.number,
        TokenType.number,
        TokenType.rParen,
      ]);
    });

    test('nested call', () {
      expect(types('(+ (* 2 3) 4)'), [
        TokenType.lParen,
        TokenType.symbol,
        TokenType.lParen,
        TokenType.symbol,
        TokenType.number,
        TokenType.number,
        TokenType.rParen,
        TokenType.number,
        TokenType.rParen,
      ]);
    });

    test('define', () {
      expect(types('(define x 42)'), [
        TokenType.lParen,
        TokenType.symbol,
        TokenType.symbol,
        TokenType.number,
        TokenType.rParen,
      ]);
    });

    test('lambda', () {
      expect(types('(lambda (x) (* x x))'), [
        TokenType.lParen,
        TokenType.symbol,
        TokenType.lParen,
        TokenType.symbol,
        TokenType.rParen,
        TokenType.lParen,
        TokenType.symbol,
        TokenType.symbol,
        TokenType.symbol,
        TokenType.rParen,
        TokenType.rParen,
      ]);
    });

    test('quoted symbol', () {
      expect(types("'foo"), [TokenType.quote, TokenType.symbol]);
    });

    test('quoted list', () {
      expect(types("'(1 2 3)"), [
        TokenType.quote,
        TokenType.lParen,
        TokenType.number,
        TokenType.number,
        TokenType.number,
        TokenType.rParen,
      ]);
    });

    test('dotted pair', () {
      expect(types('(1 . 2)'), [
        TokenType.lParen,
        TokenType.number,
        TokenType.dot,
        TokenType.number,
        TokenType.rParen,
      ]);
    });

    test('cond expression', () {
      expect(types('(cond ((eq x 0) 1) (t x))'), [
        TokenType.lParen,
        TokenType.symbol,
        TokenType.lParen,
        TokenType.lParen,
        TokenType.symbol,
        TokenType.symbol,
        TokenType.number,
        TokenType.rParen,
        TokenType.number,
        TokenType.rParen,
        TokenType.lParen,
        TokenType.symbol,
        TokenType.symbol,
        TokenType.rParen,
        TokenType.rParen,
      ]);
    });

    test('factorial', () {
      const source = '''
        (define factorial
          (lambda (n)
            (cond ((eq n 0) 1)
                  (t (* n (factorial (- n 1)))))))
        ''';
      final tokens =
          tokenize(source).where((t) => t.tokenType != TokenType.eof).toList();
      expect(tokens.length, greaterThan(20));
      expect(tokens[0].tokenType, TokenType.lParen);
      expect(tokens[1].value, 'define');
      expect(tokens[2].value, 'factorial');
    });

    test('empty input', () {
      final tokens = tokenize('');
      expect(tokens.length, 1);
      expect(tokens[0].tokenType, TokenType.eof);
    });

    test('only comments', () {
      final tokens = tokenize('; just a comment\n; another one');
      expect(tokens.length, 1);
      expect(tokens[0].tokenType, TokenType.eof);
    });

    test('eof always present', () {
      final tokens = tokenize('(+ 1 2)');
      expect(tokens.last.tokenType, TokenType.eof);
      expect(tokens.last.value, '');
    });
  });

  group('disambiguation', () {
    test('negative number in context', () {
      expect(types('-42'), [TokenType.number]);
      expect(values('-42'), ['-42']);
    });

    test('subtraction expression', () {
      expect(types('(- 3 1)'), [
        TokenType.lParen,
        TokenType.symbol,
        TokenType.number,
        TokenType.number,
        TokenType.rParen,
      ]);
      expect(values('(- 3 1)'), ['(', '-', '3', '1', ')']);
    });

    test('minus followed by symbol start is a symbol', () {
      expect(types('-a'), [TokenType.symbol]);
      expect(values('-a'), ['-a']);
    });
  });

  group('errors', () {
    test('unterminated string', () {
      expect(
          () => tokenize('"hello'),
          throwsA(const LexerError(
              message: 'Unterminated string literal', position: 0)));
    });

    test('unterminated string position is the quote', () {
      expect(
          () => tokenize('(a "oops'),
          throwsA(const LexerError(
              message: 'Unterminated string literal', position: 3)));
    });

    test('unexpected character', () {
      expect(
          () => tokenize('@'),
          throwsA(const LexerError(
              message: "Unexpected character: '@'", position: 0)));
    });

    test('unexpected character position and rendering', () {
      expect(
          () => tokenize('(a # b)'),
          throwsA(const LexerError(
              message: "Unexpected character: '#'", position: 3)));
      // Backslash outside strings has no meaning.
      expect(
          () => tokenize('\\'),
          throwsA(const LexerError(
              message: r"Unexpected character: '\\'", position: 0)));
    });

    test('non-ASCII errors use unicode escapes', () {
      expect(
          () => tokenize('é'),
          throwsA(const LexerError(
              message: r"Unexpected character: '\u{e9}'", position: 0)));
    });

    test('error after valid tokens', () {
      expect(
          () => tokenize('(+ 1 @)'),
          throwsA(const LexerError(
              message: "Unexpected character: '@'", position: 5)));
    });
  });

  group('display names', () {
    test('every type renders the reference name', () {
      expect(TokenType.number.displayName, 'NUMBER');
      expect(TokenType.symbol.displayName, 'SYMBOL');
      expect(TokenType.string.displayName, 'STRING');
      expect(TokenType.lParen.displayName, 'LPAREN');
      expect(TokenType.rParen.displayName, 'RPAREN');
      expect(TokenType.quote.displayName, 'QUOTE');
      expect(TokenType.dot.displayName, 'DOT');
      expect(TokenType.eof.displayName, 'EOF');
    });
  });

  group('value semantics', () {
    test('tokens compare by value', () {
      const a = Token(TokenType.symbol, 'define');
      expect(a, const Token(TokenType.symbol, 'define'));
      expect(a.hashCode, const Token(TokenType.symbol, 'define').hashCode);
      expect(a.toString(), 'Token(SYMBOL, "define")');
      expect(a, isNot(const Token(TokenType.symbol, 'defun')));
      expect(a, isNot(const Token(TokenType.number, 'define')));
      expect(a, isNot('not a token'));
    });

    test('errors compare by value', () {
      const a = LexerError(message: 'm', position: 3);
      expect(a, const LexerError(message: 'm', position: 3));
      expect(a.hashCode, const LexerError(message: 'm', position: 3).hashCode);
      expect(a.toString(), 'LexerError at position 3: m');
      expect(a, isA<Exception>());
      expect(a, isNot(const LexerError(message: 'm', position: 4)));
      expect(a, isNot(const LexerError(message: 'n', position: 3)));
      expect(a, isNot('not an error'));
    });
  });
}
