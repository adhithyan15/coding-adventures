/// Port of the reference `rust/python-parser` lib suite (6 tests),
/// plus lane extras pinning version resolution, the factory, and
/// per-version root shapes.
library;

import 'package:coding_adventures_lexer/lexer.dart';
import 'package:coding_adventures_parser/parser.dart';
import 'package:coding_adventures_python_parser/python_parser.dart';
import 'package:test/test.dart';

String nestedParenSource(int n) => 'x = ${'(' * n}1${')' * n}\n';

void main() {
  group('python-parser legacy', () {
    test('parses with default version', () {
      final ast = parsePython('x = 1\n', '');
      expect(ast.ruleName, equals('file'));
    });

    test('parses indented block', () {
      final ast = parsePython('def f():\n    return 1\n', '3.12');
      expect(ast.ruleName, equals('file'));
    });

    test('all supported versions load', () {
      for (final version in supportedPythonParserVersions) {
        final ast = parsePython('', version);
        expect(ast.ruleName, isNotEmpty,
            reason: 'version $version should produce a root rule');
      }
    });

    test('unsupported version returns error', () {
      expect(
          () => parsePython('x = 1\n', '4.0'),
          throwsA(isA<ArgumentError>().having(
            (e) => e.message,
            'message',
            contains('4.0'),
          )));
      expect(tryParsePython('x = 1\n', '4.0'), isNull);
    });

    test('deeply nested input does not overflow on default stack', () {
      // No thread-stack tuning exists in Dart, so the depth test runs
      // in-process: the guard must fail the parse with a clean error
      // — never a StackOverflowError — which proves it trips first.
      expect(tryParsePython(nestedParenSource(5000), '3.12'), isNull,
          reason: 'deeply-nested input must fail, not parse or crash');
      expect(() => parsePython(nestedParenSource(5000), '3.12'),
          throwsA(isA<GrammarParseError>()));
    });

    test('reasonable nesting stays under the cap', () {
      expect(tryParsePython(nestedParenSource(9), '3.12'), isNotNull);
    });

    test('empty version selects the default version', () {
      expect(resolvePythonParserVersion(''), equals('3.12'));
      expect(
        parsePython('x = 1\n', ''),
        equals(parsePython('x = 1\n', '3.12')),
      );
    });

    test('version error message names every valid value', () {
      try {
        resolvePythonParserVersion('4.0');
        fail('expected ArgumentError');
      } on ArgumentError catch (e) {
        expect(
          e.message,
          equals(
            'Unsupported Python version \'4.0\'. Supported versions: '
            '"2.7", "3.0", "3.6", "3.8", "3.10", "3.12"',
          ),
        );
      }
    });

    test('hostile version echo cannot forge log lines', () {
      try {
        resolvePythonParserVersion('3.1\n2\x1b[31m');
        fail('expected ArgumentError');
      } on ArgumentError catch (e) {
        final message = e.message as String;
        expect(message, isNot(contains('\n')));
        expect(message, isNot(contains('\x1b')));
        expect(
          message,
          startsWith(
            r"Unsupported Python version '3.1\n2\u{1b}[31m'. ",
          ),
        );
      }
    });

    test('over-long version echo is truncated', () {
      final version = '9.' + '9' * 40;
      try {
        resolvePythonParserVersion(version);
        fail('expected ArgumentError');
      } on ArgumentError catch (e) {
        expect(
          e.message,
          startsWith(
            "Unsupported Python version '9.${'9' * 30}...'. ",
          ),
        );
      }
    });

    test('factory parser parses like the one-shot', () {
      final fromFactory = createPythonParser('x = 1\n', '3.12').parse();
      expect(fromFactory, equals(parsePython('x = 1\n', '3.12')));
    });

    test('factory rejects unknown versions and bad input', () {
      expect(() => createPythonParser('x = 1\n', '4.0'), throwsArgumentError);
      expect(() => createPythonParser('x = \x00\n', '3.12'),
          throwsA(isA<LexerError>()));
    });

    test('every version roots at its start rule', () {
      // Per the grammars: 3.8 starts at `file_input`, the rest at
      // `file` (the reference only asserts non-emptiness).
      const roots = {
        '2.7': 'file',
        '3.0': 'file',
        '3.6': 'file',
        '3.8': 'file_input',
        '3.10': 'file',
        '3.12': 'file',
      };
      for (final version in supportedPythonParserVersions) {
        final ast = parsePython('def f():\n    return 1\n', version);
        expect(ast.ruleName, equals(roots[version]),
            reason: 'version $version');
      }
    });

    test('syntax failures throw GrammarParseError', () {
      expect(() => parsePython('x = \n', '3.12'),
          throwsA(isA<GrammarParseError>()));
      expect(tryParsePython('x = \n', '3.12'), isNull);
    });

    test('release ids are not legacy versions', () {
      for (final release in supportedPythonParserReleases) {
        expect(() => parsePython('x = 1\n', release), throwsArgumentError,
            reason: release);
      }
    });
  });
}
