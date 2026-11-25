/// Port of the reference `rust/perl-lexer` unit suite (5 tests),
/// plus lane extras pinning the source cap, the factory shape, and
/// token invariants. The reference's regeneration check reads
/// `releases.csv` and 118 `.tokens` files from disk; this lane pins
/// the same inventory and class relationships in-test instead (lane
/// tests never read outside the package), while the emitter asserts
/// compiled-fresh equality at generation time.
library;

import 'package:coding_adventures_lexer/lexer.dart';
import 'package:coding_adventures_perl_lexer/perl_lexer.dart';
import 'package:test/test.dart';

void main() {
  group('perl-lexer', () {
    test('tokenizes print and arithmetic', () {
      final tokens = tokenizePerl('print(7 - 2 * 3);\n');
      final values = tokens.map((t) => t.value).toList();
      expect(values.sublist(0, 9),
          equals(['print', '(', '7', '-', '2', '*', '3', ')', ';']));
    });

    test('source limit precedes tokenization', () {
      expect(
        () => tokenizePerl(' ' * (maxPerlSourceBytes + 1)),
        throwsA(isA<ArgumentError>()
            .having((e) => e.message, 'message', contains('source exceeds'))),
      );
    });

    test('adjacent minus is decrement token', () {
      final adjacent = tokenizePerl('print(1--2);');
      expect(adjacent.any((t) => t.type == 'DECREMENT'), isTrue);
      final spaced = tokenizePerl('print(1- -2);');
      expect(spaced.any((t) => t.type == 'DECREMENT'), isFalse);
    });

    test('compiled tables cover the inventoried pairs', () {
      // 118 releases have a pair, in inventory (date) order.
      expect(supportedPerlReleases.length, equals(118));
      expect(supportedPerlReleases.first, equals('1.000'));
      expect(supportedPerlReleases.last, equals('5.45.3'));
      for (final release in supportedPerlReleases) {
        expect(perlReleaseTokenGrammar(release), isNotNull, reason: release);
        expect(perlGrammarClass(release), startsWith('g_'), reason: release);
      }
      // Pending inventory rows have neither a table nor a class.
      for (final release in ['1.001', '1.002', '1.003', '1.004', '1.005']) {
        expect(perlReleaseTokenGrammar(release), isNull, reason: release);
        expect(perlGrammarClass(release), isNull, reason: release);
      }
    });

    test('identical pairs share one class', () {
      String cls(String r) => perlGrammarClass(r)!;
      // Same class.
      expect(cls('1.000'), equals(cls('2.000')));
      expect(cls('5.004_72'), equals(cls('5.38.0')));
      expect(cls('5.38.0'), equals(cls('5.38.2')));
      expect(cls('5.44.0'), equals(cls('5.45.3')));
      // Different classes.
      expect(cls('1.000'), isNot(equals(cls('5.004_72'))));
      expect(cls('5.44.0'), isNot(equals(cls('5.38.0'))));
      expect(cls('5.003_25'), isNot(equals(cls('5.003_26'))));
      // Exactly five distinct classes across all 118 releases.
      final classes = supportedPerlReleases.map(perlGrammarClass).toSet();
      expect(classes.length, equals(5));
    });

    test('unknown and empty release ids have no table', () {
      for (final release in [
        '',
        '5.38',
        'perl5.38.2',
        '5.38.2 ',
        '../perl',
        '5.999.0',
      ]) {
        expect(perlReleaseTokenGrammar(release), isNull, reason: '$release');
      }
    });

    test('source cap message is exact', () {
      try {
        tokenizePerl('x' * (maxPerlSourceBytes + 1));
        fail('expected ArgumentError');
      } on ArgumentError catch (e) {
        expect(
          e.message,
          equals('Perl source exceeds the 65536-byte native pilot limit'),
        );
      }
    });

    test('source cap counts bytes not code units', () {
      // Exactly at the cap lexes (whitespace-only input skips to EOF).
      expect(tokenizePerl(' ' * maxPerlSourceBytes).last.type, equals('EOF'));
      // One byte over fails even when the string is short in code
      // units: 32768 astral chars are 65536 UTF-16 units but 131072
      // UTF-8 bytes.
      expect(
        () => tokenizePerl('🎉' * 32768),
        throwsArgumentError,
      );
      // And a short byte count with astral text passes the cap
      // (failing later at tokenize time, not at the cap).
      expect(
        () => tokenizePerl('🎉'),
        throwsA(isA<LexerError>()),
      );
    });

    test('unversioned grammar lexes the pilot subset', () {
      final tokens =
          GrammarLexer('print(1);', unversionedPerlTokenGrammar()).tokenize();
      expect(tokens.map((t) => t.type).toList(),
          equals(['NAME', 'LPAREN', 'INT', 'RPAREN', 'SEMICOLON', 'EOF']));
    });

    test('tokens keep sane positions', () {
      for (final token in tokenizePerl('print(7 - 2 * 3);\n')) {
        expect(token.line, greaterThanOrEqualTo(1));
        expect(token.column, greaterThanOrEqualTo(1));
      }
    });
  });
}
