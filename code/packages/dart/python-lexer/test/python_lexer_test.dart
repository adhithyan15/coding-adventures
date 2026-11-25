/// Port of the reference `rust/python-lexer` unit suite (9 tests),
/// plus lane extras pinning version resolution, the factory, and
/// token-shape invariants every legacy version must satisfy.
library;

import 'package:coding_adventures_lexer/lexer.dart';
import 'package:coding_adventures_python_lexer/python_lexer.dart';
import 'package:test/test.dart';

/// (type, value) for every non-EOF token.
List<(String, String)> pairs(List<Token> tokens) => [
      for (final t in tokens)
        if (t.type != 'EOF') (t.type, t.value),
    ];

List<Token> lexRelease(String release, String source) =>
    grammarTokenize(source, tokenGrammarForRelease(release)!);

void main() {
  group('python-lexer', () {
    test('tokenizes python with default version', () {
      final tokens = tokenizePython('x = 1\n', '');
      expect(tokens[0].type, equals('NAME'));
      expect(tokens[0].value, equals('x'));
    });

    test('supports indentation tokens', () {
      final tokens = tokenizePython('def f():\n    return 1\n', '3.12');
      expect(tokens.any((t) => t.type == 'INDENT'), isTrue);
      expect(tokens.any((t) => t.type == 'DEDENT'), isTrue);
    });

    test('soft keywords remain names', () {
      final tokens = tokenizePython('match = 42\n', '3.12');
      expect(tokens[0].type, equals('NAME'));
      expect(tokens[0].value, equals('match'));
    });

    test('all supported versions load', () {
      for (final version in supportedPythonVersions) {
        final tokens = tokenizePython('x = 1\n', version);
        expect(tokens.last.type, equals('EOF'), reason: 'version $version');
      }
    });

    test('unsupported version returns error', () {
      expect(
          () => tokenizePython('x = 1\n', '4.0'),
          throwsA(isA<ArgumentError>()
              .having((e) => e.message, 'message', contains('4.0'))));
    });

    test('release tables list the paired releases in inventory order', () {
      // Inventory (date) order, not version-sorted, on purpose.
      expect(
          supportedPythonReleases,
          equals([
            '2.2',
            '2.4',
            '3.0a1',
            '2.6',
            '3.6.0',
            '2.7.18',
            '3.5.10',
            '3.12.3',
            '3.13.16',
            '3.11.17',
            '3.16.0a1',
          ]));
      for (final release in supportedPythonReleases) {
        final tokens = lexRelease(release, 'print(1 // 2)\n');
        expect(tokens[0].value, equals('print'), reason: release);
        expect(tokens[3].type, equals('FLOOR_DIV'), reason: release);
      }
    });

    test('identical token files share one class', () {
      String cls(String r) => pythonGrammarClass(r)!;
      expect(cls('3.12.3'), equals(cls('3.13.16')));
      expect(cls('3.13.16'), equals(cls('3.16.0a1')));
      expect(cls('3.11.17'), isNot(equals(cls('3.13.16'))));
      expect(cls('3.0a1'), isNot(equals(cls('3.11.17'))));
      expect(cls('3.5.10'), isNot(equals(cls('3.0a1'))));
      expect(cls('3.6.0'), isNot(equals(cls('3.5.10'))));
      expect(cls('2.2'), isNot(equals(cls('2.4'))));
      expect(cls('2.4'), isNot(equals(cls('2.6'))));
      expect(cls('2.6'), equals(cls('2.7.18')));
      expect(cls('2.7.18'), isNot(equals(cls('3.0a1'))));
    });

    test('python 2 pairs lex print as keyword and keep old operators', () {
      for (final release in ['2.2', '2.4', '2.6', '2.7.18']) {
        final tokens = lexRelease(release, 'print 7 / 2 <> 3\n');
        expect(
          tokens.take(6).map((t) => t.type).toList(),
          equals(
              ['KEYWORD', 'INT', 'SLASH', 'INT', 'LEGACY_NOT_EQUALS', 'INT']),
          reason: release,
        );
        for (final source in [
          '010',
          '00',
          '0',
          '0x1F',
          '0XffL',
          '1L',
          '7l',
          '0777L'
        ]) {
          final ints = lexRelease(release, source);
          expect((ints[0].type, ints[0].value), equals(('INT', source)),
              reason: '$release $source');
        }
        for (final source in [
          '08\n',
          '0178\n',
          '0x\n',
          '1_0\n',
          '1LL\n',
          '0x1Fg\n',
          '1if\n'
        ]) {
          expect(() => lexRelease(release, source), throwsA(isA<LexerError>()),
              reason: '$release $source');
        }
      }
      bool lexes(String r, String source) {
        try {
          lexRelease(r, source);
          return true;
        } on LexerError {
          return false;
        }
      }

      for (final source in ['0o17\n', '0O17L\n', '0b101\n', '0B1l\n']) {
        expect(lexes('2.2', source), isFalse, reason: source);
        expect(lexes('2.4', source), isFalse, reason: source);
        expect(lexes('2.6', source), isTrue, reason: source);
        expect(lexes('2.7.18', source), isTrue, reason: source);
      }
      for (final source in ['0o8\n', '0b2\n', '0o\n', '0b\n']) {
        expect(lexes('2.6', source), isFalse, reason: source);
        expect(lexes('2.7.18', source), isFalse, reason: source);
      }
      expect(lexes('2.2', '@\n'), isFalse);
      expect(lexes('2.4', '@\n'), isTrue);
      expect(lexes('2.7.18', '@\n'), isTrue);
    });

    test('release ids and minor lines never cross', () {
      for (final version in supportedPythonVersions) {
        expect(tokenGrammarForRelease(version), isNull, reason: version);
      }
      for (final release in supportedPythonReleases) {
        expect(() => tokenizePython('x\n', release), throwsArgumentError,
            reason: release);
      }
      expect(tokenGrammarForRelease(''), isNull);
      expect(tokenGrammarForRelease('3.13'), isNull);
    });

    test('empty version selects the default version', () {
      expect(defaultPythonVersion, equals('3.12'));
      expect(resolvePythonVersion(''), equals('3.12'));
      expect(
        pairs(tokenizePython('x = 1\n', '')),
        equals(pairs(tokenizePython('x = 1\n', '3.12'))),
      );
    });

    test('version error message names every valid value', () {
      try {
        resolvePythonVersion('4.0');
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

    test('one-shot is deterministic', () {
      final first = tokenizePython('def f():\n    return 1\n', '3.12');
      final second = tokenizePython('def f():\n    return 1\n', '3.12');
      expect(pairs(second), equals(pairs(first)));
      expect(first.last.type, equals('EOF'));
    });

    test('unknown releases have no grammar and no class', () {
      expect(tokenGrammarForRelease('4.0'), isNull);
      expect(pythonGrammarClass('4.0'), isNull);
      expect(pythonGrammarClass(''), isNull);
      expect(pythonGrammarClass('3.12'), isNull);
    });

    test('every legacy version ends its stream with EOF', () {
      for (final version in supportedPythonVersions) {
        final tokens = tokenizePython('def f():\n    return 1\n', version);
        expect(tokens, isNotEmpty, reason: 'version $version');
        expect(tokens.last.type, equals('EOF'), reason: 'version $version');
      }
    });

    test('tokens keep sane positions in every legacy version', () {
      for (final version in supportedPythonVersions) {
        for (final token in tokenizePython('x = 1\n', version)) {
          expect(token.line, greaterThanOrEqualTo(1),
              reason: 'version $version');
          expect(token.column, greaterThanOrEqualTo(1),
              reason: 'version $version');
        }
      }
    });
  });
}
