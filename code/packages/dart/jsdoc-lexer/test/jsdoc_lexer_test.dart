/// Unit tests for `coding_adventures_jsdoc_lexer`: all 9
/// reference tests, plus the corners the reference leaves uncovered
/// — error positions and the factory's explicit error path.
library;

import 'package:coding_adventures_jsdoc_lexer/jsdoc_lexer.dart';
import 'package:coding_adventures_lexer/lexer.dart';
import 'package:test/test.dart';

/// Owned (type, value) pairs minus the trailing EOF. Dart tokens
/// carry their grammar names directly, so no built-in mapping is
/// needed (the reference's `pairs` helper maps `TokenType` variants
/// to the same SCREAMING_SNAKE_CASE spellings).
List<(String, String)> pairs(List<Token> tokens) => [
      for (final t in tokens)
        if (t.type != 'EOF') (t.type, t.value),
    ];

List<String> names(List<Token> tokens) =>
    pairs(tokens).map((p) => p.$1).toList();

void main() {
  group('reference', () {
    test('tokenizes type tag', () {
      // `@type {number}` is the simplest JSDoc tag — one tag, one
      // type expression with one primitive.
      final found = names(tokenizeJsdoc('@type {number}\n'));
      expect(found, contains('AT_TAG'));
      expect(found, contains('LBRACE'));
      expect(found, contains('NAME'));
      expect(found, contains('RBRACE'));
      expect(found, contains('NEWLINE'));
    });

    test('at tag value includes at sign', () {
      final tokens = tokenizeJsdoc('@param {string} name\n');
      final at = tokens.where((t) => t.type == 'AT_TAG').toList();
      expect(at, isNotEmpty);
      expect(at.first.value, equals('@param'));
    });

    test('nominal type with dots', () {
      // `Foo.Bar` should produce NAME DOT NAME — three tokens.
      final nameDotName = pairs(tokenizeJsdoc('@type {Foo.Bar}\n'))
          .where((p) => p.$1 == 'NAME' || p.$1 == 'DOT')
          .map((p) => p.$1)
          .toList();
      expect(nameDotName, equals(['NAME', 'DOT', 'NAME']));
    });

    test('nullable wrapper', () {
      // `?Foo` → QUESTION NAME.
      final p = pairs(tokenizeJsdoc('@type {?Foo}\n'));
      expect(p.any((q) => q.$1 == 'QUESTION'), isTrue);
      expect(p.any((q) => q.$1 == 'NAME' && q.$2 == 'Foo'), isTrue);
    });

    test('variadic wrapper', () {
      final p = pairs(tokenizeJsdoc('@param {...string} args\n'));
      expect(p.any((q) => q.$1 == 'ELLIPSIS'), isTrue);
    });

    test('array suffix', () {
      // `string[]` → NAME LBRACKET RBRACKET.
      final p = pairs(tokenizeJsdoc('@type {string[]}\n'));
      expect(p.any((q) => q.$1 == 'NAME' && q.$2 == 'string'), isTrue);
      expect(p.any((q) => q.$1 == 'LBRACKET'), isTrue);
      expect(p.any((q) => q.$1 == 'RBRACKET'), isTrue);
    });

    test('whitespace skipped between tokens', () {
      final compact = pairs(tokenizeJsdoc('@type{number}\n'));
      final spaced = pairs(tokenizeJsdoc('@type   {  number  }\n'));
      // Same logical sequence, different whitespace.
      expect(
        spaced.map((q) => q.$1).toList(),
        equals(compact.map((q) => q.$1).toList()),
      );
    });

    test('factory returns working lexer', () {
      // Factory path produces a lexer that can drive tokenize() itself.
      final tokens = createJsdocLexer('@type {number}\n').tokenize();
      expect(tokens.last.type, equals('EOF'));
    });

    test('empty input is eof only', () {
      final tokens = tokenizeJsdoc('');
      expect(tokens.length, equals(1));
      expect(tokens[0].type, equals('EOF'));
    });
  });

  group('extras', () {
    test('errors carry positions', () {
      // Backtick has no token in this grammar.
      expect(() => tokenizeJsdoc('`'), throwsA(isA<LexerError>()));
      try {
        tokenizeJsdoc('ab`cd');
        fail('expected LexerError');
      } on LexerError catch (e) {
        expect((e.line, e.column), equals((1, 3)));
        expect(e.errorId, isNotEmpty);
      }
    });

    test('factory surfaces errors explicitly', () {
      // The reference's documented error path: factory + tokenize()
      // for callers that handle errors explicitly (no Result twin).
      expect(
        () => createJsdocLexer('`').tokenize(),
        throwsA(isA<LexerError>()),
      );
      final tokens = createJsdocLexer('@type {number}\n').tokenize();
      expect(tokens, equals(tokenizeJsdoc('@type {number}\n')));
    });
  });
}
