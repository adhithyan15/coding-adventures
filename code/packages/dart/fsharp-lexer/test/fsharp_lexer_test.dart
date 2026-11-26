/// Port of the reference `rust/fsharp-lexer` unit suite (3 tests),
/// plus lane extras pinning version resolution, the factory, and
/// token-shape invariants every version must satisfy.
library;

import 'package:coding_adventures_fsharp_lexer/fsharp_lexer.dart';
import 'package:coding_adventures_lexer/lexer.dart';
import 'package:test/test.dart';

/// (type, value) for every non-EOF token.
List<(String, String)> pairs(List<Token> tokens) => [
      for (final t in tokens)
        if (t.type != 'EOF') (t.type, t.value),
    ];

void main() {
  group('fsharp-lexer', () {
    test('tokenizes let binding', () {
      final tokens = tokenizeFsharp('let value = 1', '');
      expect(tokens[0].type, equals('KEYWORD'));
      expect(tokens[0].value, equals('let'));
    });

    test('all supported versions load', () {
      for (final version in supportedFsharpVersions) {
        final tokens = tokenizeFsharp('let value = 1', version);
        expect(tokens.last.type, equals('EOF'), reason: 'version $version');
      }
    });

    test('unknown version returns error', () {
      expect(
        () => tokenizeFsharp('let value = 1', '11'),
        throwsA(
          isA<ArgumentError>().having(
            (e) => e.message,
            'message',
            contains('11'),
          ),
        ),
      );
    });

    test('empty version selects the default version', () {
      expect(defaultFsharpVersion, equals('10'));
      expect(resolveFsharpVersion(''), equals('10'));
      expect(
        pairs(tokenizeFsharp('let value = 1', '')),
        equals(pairs(tokenizeFsharp('let value = 1', '10'))),
      );
    });

    test('version error message names every valid value', () {
      try {
        resolveFsharpVersion('11');
        fail('expected ArgumentError');
      } on ArgumentError catch (e) {
        expect(
          e.message,
          equals(
            'Unknown F# version \'11\'. Valid values: "1.0", "2.0", '
            '"3.0", "3.1", "4.0", "4.1", "4.5", "4.6", "4.7", "5", '
            '"6", "7", "8", "9", "10"',
          ),
        );
      }
    });

    test('factory lexer tokenizes like the one-shot', () {
      final fromFactory = createFsharpLexer('let value = 1', '10').tokenize();
      expect(pairs(fromFactory),
          equals(pairs(tokenizeFsharp('let value = 1', '10'))));
      expect(fromFactory.last.type, equals('EOF'));
    });

    test('factory rejects unknown versions too', () {
      expect(
          () => createFsharpLexer('let value = 1', '99'), throwsArgumentError);
    });

    test('every version ends its stream with EOF', () {
      for (final version in supportedFsharpVersions) {
        final tokens = tokenizeFsharp('let f x = x + 1', version);
        expect(tokens, isNotEmpty, reason: 'version $version');
        expect(tokens.last.type, equals('EOF'), reason: 'version $version');
      }
    });

    test('tokens keep sane positions in every version', () {
      for (final version in supportedFsharpVersions) {
        for (final token in tokenizeFsharp('let f x = x + 1', version)) {
          expect(token.line, greaterThanOrEqualTo(1),
              reason: 'version $version');
          expect(token.column, greaterThanOrEqualTo(1),
              reason: 'version $version');
        }
      }
    });

    test('unrecognized input throws LexerError', () {
      // U+0000 is outside every version's alphabet.
      expect(() => tokenizeFsharp('let x = \x00', '10'),
          throwsA(isA<LexerError>()));
    });
  });
}
