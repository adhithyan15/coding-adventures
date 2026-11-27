/// Port of the reference `rust/fsharp-parser` unit suite (5 tests),
/// plus lane extras pinning version resolution, the factory, and
/// per-version root shapes.
library;

import 'package:coding_adventures_fsharp_parser/fsharp_parser.dart';
import 'package:coding_adventures_lexer/lexer.dart';
import 'package:coding_adventures_parser/parser.dart';
import 'package:test/test.dart';

String nestedParenSource(int n) => 'let x = ${'(' * n}1${')' * n}';

void main() {
  group('fsharp-parser', () {
    test('parses let binding', () {
      final ast = parseFsharp('let value = 1', '');
      expect(ast.ruleName, equals('compilation_unit'));
    });

    test('all supported versions load', () {
      for (final version in supportedFsharpParserVersions) {
        final ast = parseFsharp('let value = 1', version);
        expect(ast.ruleName, equals('compilation_unit'),
            reason: 'version $version');
      }
    });

    test('unknown version returns error', () {
      expect(
        () => parseFsharp('let value = 1', '11'),
        throwsA(
          isA<ArgumentError>().having(
            (e) => e.message,
            'message',
            contains('11'),
          ),
        ),
      );
      expect(tryParseFsharp('let value = 1', '11'), isNull);
    });

    test('deeply nested input does not overflow on default stack', () {
      // No thread-stack tuning exists in Dart, so the depth test runs
      // in-process: the guard must fail the parse with a clean error
      // — never a StackOverflowError — which proves it trips first.
      expect(tryParseFsharp(nestedParenSource(5000)), isNull,
          reason: 'deeply-nested input must fail, not parse or crash');
      expect(() => parseFsharp(nestedParenSource(5000)),
          throwsA(isA<GrammarParseError>()));
    });

    test('reasonable nesting stays under the cap', () {
      expect(tryParseFsharp(nestedParenSource(10)), isNotNull);
    });

    test('empty version selects the default version', () {
      expect(resolveFsharpParserVersion(''), equals('10'));
      expect(
        parseFsharp('let value = 1', ''),
        equals(parseFsharp('let value = 1', '10')),
      );
    });

    test('version error message names every valid value', () {
      try {
        resolveFsharpParserVersion('11');
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

    test('factory parser parses like the one-shot', () {
      final fromFactory = createFsharpParser('let value = 1', '10').parse();
      expect(fromFactory, equals(parseFsharp('let value = 1', '10')));
    });

    test('factory rejects unknown versions and bad input', () {
      expect(
          () => createFsharpParser('let value = 1', '99'), throwsArgumentError);
      expect(() => createFsharpParser('let x = \x00', '10'),
          throwsA(isA<LexerError>()));
    });

    test('every version roots at compilation_unit', () {
      for (final version in supportedFsharpParserVersions) {
        final ast = parseFsharp('let f x = x + 1', version);
        expect(ast.ruleName, equals('compilation_unit'),
            reason: 'version $version');
      }
    });

    test('syntax failures throw GrammarParseError', () {
      expect(() => parseFsharp('let = 1', '10'),
          throwsA(isA<GrammarParseError>()));
      expect(tryParseFsharp('let = 1', '10'), isNull);
    });
  });
}
