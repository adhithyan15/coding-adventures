/// Unit tests for `coding_adventures_css_lexer`: all 10 reference
/// tests, plus the corners the reference leaves uncovered — numeric
/// priority ordering, custom properties, unicode ranges, URL/function
/// discrimination, CDO/CDC, escapes-none string values, multi-char
/// operator priority, and the fail-closed error vocabulary.
library;

import 'package:coding_adventures_css_lexer/css_lexer.dart';
import 'package:coding_adventures_lexer/lexer.dart';
import 'package:test/test.dart';

/// (type, value) pairs, excluding the final EOF token.
List<(String, String)> tokenPairs(List<Token> tokens) => [
      for (final t in tokens)
        if (t.type != 'EOF') (t.type, t.value)
    ];

List<String> valuesOf(List<Token> tokens, String type) => [
      for (final (t, v) in tokenPairs(tokens))
        if (t == type) v
    ];

void main() {
  group('rules', () {
    test('simple rule', () {
      expect(tokenPairs(tokenizeCss('body { color: red; }')), [
        ('IDENT', 'body'),
        ('LBRACE', '{'),
        ('IDENT', 'color'),
        ('COLON', ':'),
        ('IDENT', 'red'),
        ('SEMICOLON', ';'),
        ('RBRACE', '}'),
      ]);
    });

    test('dimensions stay single tokens', () {
      final pairs = tokenPairs(tokenizeCss('h1 { font-size: 16px; }'));
      expect(pairs, contains(('DIMENSION', '16px')));
    });

    test('numeric priority: dimension, percentage, number', () {
      expect(tokenPairs(tokenizeCss('10px')), [('DIMENSION', '10px')]);
      expect(tokenPairs(tokenizeCss('1.5em')), [('DIMENSION', '1.5em')]);
      expect(tokenPairs(tokenizeCss('10PX')), [('DIMENSION', '10PX')]);
      expect(tokenPairs(tokenizeCss('50%')), [('PERCENTAGE', '50%')]);
      expect(tokenPairs(tokenizeCss('42')), [('NUMBER', '42')]);
      expect(tokenPairs(tokenizeCss('.5')), [('NUMBER', '.5')]);
      expect(tokenPairs(tokenizeCss('-5')), [('NUMBER', '-5')]);
      // Grammar quirk (proven identical in Rust via differential probe):
      // DIMENSION's unit class eats the exponent's `e`, so scientific
      // notation splits — contradicting both the grammar's own comment
      // and real CSS. Logged for the grammar owners.
      expect(tokenPairs(tokenizeCss('1e10')),
          [('DIMENSION', '1e'), ('NUMBER', '10')]);
      expect(tokenPairs(tokenizeCss('1.5e-3')),
          [('DIMENSION', '1.5e'), ('NUMBER', '-3')]);
    });

    test('hash tokens cover colors and ids', () {
      final pairs = tokenPairs(tokenizeCss('#main { color: #ff0000; }'));
      final hashes = [
        for (final (t, v) in pairs)
          if (t == 'HASH') v
      ];
      expect(hashes, ['#main', '#ff0000']);
    });

    test('strings strip quotes, keep escapes raw', () {
      final tokens = tokenizeCss('a::after { content: "hello"; }');
      expect(valuesOf(tokens, 'STRING'), ['hello']);
      expect(tokenPairs(tokenizeCss("'hi'")), [('STRING', 'hi')]);
      // escapes: none — CSS hex escapes survive verbatim.
      expect(tokenPairs(tokenizeCss(r'"a\26b"')), [('STRING', r'a\26b')]);
    });

    test('at-keyword', () {
      final pairs = tokenPairs(tokenizeCss('@media screen { }'));
      expect(pairs.first, ('AT_KEYWORD', '@media'));
    });

    test('whitespace never yields tokens', () {
      final compact = tokenPairs(tokenizeCss('a{b:c}'));
      final spaced = tokenPairs(tokenizeCss('a  {  b  :  c  }'));
      expect(spaced.length, compact.length);
      expect(tokenPairs(tokenizeCss('a\nb')), [('IDENT', 'a'), ('IDENT', 'b')]);
    });

    test('comments are skipped across lines', () {
      final pairs = tokenPairs(tokenizeCss('a /* comment */ { color: red; }'));
      expect(pairs.any((p) => p.$2.contains('comment')), isFalse);
      expect(tokenPairs(tokenizeCss('a /* x\ny */ b')),
          [('IDENT', 'a'), ('IDENT', 'b')]);
    });

    test('delimiters', () {
      final values = [
        for (final (_, v) in tokenPairs(tokenizeCss('a { b: c; }'))) v
      ];
      for (final d in ['{', '}', ':', ';']) {
        expect(values, contains(d));
      }
    });

    test('multiple selectors', () {
      final pairs = tokenPairs(tokenizeCss('h1, h2, h3 { margin: 0; }'));
      expect([
        for (final (_, v) in pairs)
          if (v == ',') v
      ], hasLength(2));
    });

    test('factory lexer tokenizes', () {
      final tokens = createCssLexer('a { }').tokenize();
      expect(tokens.length, greaterThanOrEqualTo(2));
      expect(tokens.last.type, 'EOF');
    });
  });

  group('tokens', () {
    test('custom property beats ident prefix', () {
      expect(tokenPairs(tokenizeCss('--main-color')),
          [('CUSTOM_PROPERTY', '--main-color')]);
    });

    test('unicode range stays single', () {
      expect(tokenPairs(tokenizeCss('U+0025-00FF')),
          [('UNICODE_RANGE', 'U+0025-00FF')]);
      expect(tokenPairs(tokenizeCss('U+4??')), [('UNICODE_RANGE', 'U+4??')]);
    });

    test('unquoted url is one token', () {
      expect(tokenPairs(tokenizeCss('url(path/to.png)')),
          [('URL_TOKEN', 'url(path/to.png)')]);
    });

    test('quoted url splits into function, string, rparen', () {
      expect(tokenPairs(tokenizeCss('url("path")')), [
        ('FUNCTION', 'url('),
        ('STRING', 'path'),
        ('RPAREN', ')'),
      ]);
    });

    test('function names keep their paren', () {
      expect(tokenPairs(tokenizeCss('rgb(')), [('FUNCTION', 'rgb(')]);
      expect(tokenPairs(tokenizeCss('URL(')), [('FUNCTION', 'URL(')]);
      expect(
          tokenPairs(tokenizeCss('-webkit-x(')), [('FUNCTION', '-webkit-x(')]);
    });

    test('legacy comment delimiters', () {
      expect(tokenPairs(tokenizeCss('<!--')), [('CDO', '<!--')]);
      expect(tokenPairs(tokenizeCss('-->')), [('CDC', '-->')]);
    });

    test('multi-char operators win over prefixes', () {
      expect(tokenPairs(tokenizeCss('::')), [('COLON_COLON', '::')]);
      expect(tokenPairs(tokenizeCss('*=')), [('STAR_EQUALS', '*=')]);
      expect(tokenPairs(tokenizeCss('~=')), [('TILDE_EQUALS', '~=')]);
      expect(tokenPairs(tokenizeCss('|=')), [('PIPE_EQUALS', '|=')]);
      expect(tokenPairs(tokenizeCss('^=')), [('CARET_EQUALS', '^=')]);
      expect(tokenPairs(tokenizeCss('\$=')), [('DOLLAR_EQUALS', '\$=')]);
    });

    test('combinators and important', () {
      expect(
          tokenPairs(tokenizeCss('a > b + c ~ d')), contains(('GREATER', '>')));
      expect(tokenPairs(tokenizeCss('!important')), contains(('BANG', '!')));
      expect(tokenPairs(tokenizeCss('&')), [('AMPERSAND', '&')]);
      expect(tokenPairs(tokenizeCss('/')), [('SLASH', '/')]);
    });

    test('vendor prefixes are idents', () {
      expect(tokenPairs(tokenizeCss('-webkit-transform')),
          [('IDENT', '-webkit-transform')]);
    });
  });

  group('errors', () {
    test('unclosed string fails closed', () {
      // BAD_STRING/BAD_URL exist in the grammar file but no engine
      // consults them (Rust and Dart alike) — malformed input throws.
      expect(() => tokenizeCss('"abc'), throwsA(isA<LexerError>()));
      expect(() => tokenizeCss('a { b: "x; }'), throwsA(isA<LexerError>()));
    });

    test('bare sigils fail closed with positions', () {
      for (final src in ['#', '@', 'a # b']) {
        expect(() => tokenizeCss(src), throwsA(isA<LexerError>()));
      }
      try {
        tokenizeCss('a # b');
        fail('expected LexerError');
      } on LexerError catch (e) {
        expect((e.line, e.column), (1, 3));
        expect(e.errorId, isNotEmpty);
      }
    });

    test('empty input yields only EOF', () {
      final tokens = tokenizeCss('');
      expect(tokens, hasLength(1));
      expect(tokens.single.type, 'EOF');
    });
  });
}
