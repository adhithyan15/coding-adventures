/// Unit tests for `coding_adventures_c_lexer`: all 9 reference
/// tests, plus the corners the reference leaves uncovered — literal
/// values with escapes, the keyword predicate's edges, error
/// positions, and empty input.
library;

import 'package:coding_adventures_c_lexer/c_lexer.dart';
import 'package:coding_adventures_c_lexer/src/_grammar.dart';
import 'package:coding_adventures_lexer/lexer.dart';
import 'package:test/test.dart';

List<String> kinds(String src) => [
      for (final t in tokenizeC(src))
        if (t.type != 'EOF') t.type
    ];

List<String> values(String src) => [
      for (final t in tokenizeC(src))
        if (t.type != 'EOF') t.value
    ];

void main() {
  group('declarations', () {
    test('declaration tokenizes', () {
      expect(values('int32_t x = 5;'), ['int32_t', 'x', '=', '5', ';']);
      expect(kinds('int32_t x = 5;'),
          ['KEYWORD', 'NAME', 'EQ', 'INT_LIT', 'SEMICOLON']);
    });

    test('multi-char operators win over single', () {
      expect(kinds('a == b << c && d'),
          ['NAME', 'EQ_EQ', 'NAME', 'SHL', 'NAME', 'AND_AND', 'NAME']);
      // No ARROW in this cut: `->` is MINUS + GT.
      expect(kinds('a->b'), ['NAME', 'MINUS', 'GT', 'NAME']);
    });

    test('directives exposed, comments skipped', () {
      final src =
          '#include <stdint.h>\n// a line comment\nint x; /* block */ int y;';
      expect(values(src), [
        '#',
        'include',
        '<',
        'stdint',
        '.',
        'h',
        '>',
        'int',
        'x',
        ';',
        'int',
        'y',
        ';'
      ]);
      expect(kinds('#define JOIN(a,b) a ## b'), [
        'HASH',
        'NAME',
        'NAME',
        'LPAREN',
        'NAME',
        'COMMA',
        'NAME',
        'RPAREN',
        'NAME',
        'HASH_HASH',
        'NAME'
      ]);
    });
  });

  group('literals', () {
    test('integers with hex and suffixes', () {
      expect(values('0xFF 100u 10LL 42'), ['0xFF', '100u', '10LL', '42']);
    });

    test('floats need a dot or exponent', () {
      expect(kinds('3.14 .5 10. 1e10 2.5e-3 1.0f 42'), [
        'FLOAT_LIT',
        'FLOAT_LIT',
        'FLOAT_LIT',
        'FLOAT_LIT',
        'FLOAT_LIT',
        'FLOAT_LIT',
        'INT_LIT'
      ]);
    });

    test('char and string literals', () {
      final ks = kinds('putchar(\'A\'); printf("%d\\n", x);');
      expect(ks, contains('CHAR_LIT'));
      expect(ks, contains('STR_LIT'));
    });

    test('char and string keep quotes verbatim', () {
      // Only names ending in STRING strip quotes (both lanes) — C's
      // CHAR_LIT and STR_LIT keep everything, escapes raw.
      expect(values("'A'"), ["'A'"]);
      expect(values(r'"%d\n"'), [r'"%d\n"']);
      expect(values(r'"tab\there"'), [r'"tab\there"']);
    });
  });

  group('keywords', () {
    test('float and double are keywords', () {
      for (final kw in ['float', 'double']) {
        expect(kinds(kw), ['KEYWORD']);
      }
    });

    test('stdint names are keywords', () {
      for (final kw in ['uint8_t', 'int64_t', 'size_t', 'unsigned']) {
        expect(kinds(kw), ['KEYWORD']);
      }
    });

    test('cached lookup agrees with the grammar', () {
      for (final keyword in tokenGrammar.keywords) {
        expect(isCKeyword(keyword), isTrue);
        expect(kinds(keyword), ['KEYWORD']);
      }
      for (final keyword in tokenGrammar.reservedKeywords) {
        expect(isCKeyword(keyword), isTrue);
      }
      for (final name in ['preNAME', 'int32_tail', '_name']) {
        expect(isCKeyword(name), isFalse);
        expect(kinds(name), ['NAME']);
      }
    });

    test('keyword lookup is exact and case-sensitive', () {
      expect(isCKeyword('Int'), isFalse);
      expect(isCKeyword('INT'), isFalse);
      expect(isCKeyword('int '), isFalse);
      expect(isCKeyword(''), isFalse);
    });
  });

  group('errors', () {
    test('nullable API mirrors the throwing API', () {
      expect(tryTokenizeC('int x;'), isNotNull);
      expect(tryTokenizeC('int x = @;'), isNull);
      expect(() => tokenizeC('int x = @;'), throwsA(isA<LexerError>()));
    });

    test('errors carry positions', () {
      try {
        tokenizeC('ab@cd');
        fail('expected LexerError');
      } on LexerError catch (e) {
        expect((e.line, e.column), (1, 3));
        expect(e.errorId, isNotEmpty);
      }
    });

    test('empty input yields only EOF', () {
      final tokens = tokenizeC('');
      expect(tokens, hasLength(1));
      expect(tokens.single.type, 'EOF');
    });
  });
}
