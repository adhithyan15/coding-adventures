import 'package:coding_adventures_lexer/lexer.dart';
import 'package:coding_adventures_toml_lexer/toml_lexer.dart';
import 'package:test/test.dart';

void main() {
  List<String> meaningfulTypes(String source) => tokenizeToml(source)
      .where((token) => token.type != 'NEWLINE' && token.type != 'EOF')
      .map((token) => token.type)
      .toList();

  group('TOML strings', () {
    test('strips matching basic and literal string delimiters', () {
      expect(tokenizeToml('"hello"').first.value, 'hello');
      expect(tokenizeToml("'hello'").first.value, 'hello');
      expect(tokenizeToml(r'"hello\nworld"').first.value, r'hello\nworld');
      expect(
        tokenizeToml(r"'C:\Users\name'").first.value,
        r'C:\Users\name',
      );
    });

    test('strips triple delimiters and preserves multiline contents', () {
      final basic = tokenizeToml('"""hello\nworld"""').first;
      expect(basic.type, 'ML_BASIC_STRING');
      expect(basic.value, 'hello\nworld');

      final literal = tokenizeToml("'''hello\nworld'''").first;
      expect(literal.type, 'ML_LITERAL_STRING');
      expect(literal.value, 'hello\nworld');
    });
  });

  group('TOML values', () {
    test('tokenizes integer forms with aliases', () {
      for (final source in [
        '42',
        '+42',
        '-42',
        '1_000',
        '0xDEAD',
        '0o755',
        '0b1010'
      ]) {
        final token = tokenizeToml(source).first;
        expect(token.type, 'INTEGER', reason: source);
        expect(token.value, source, reason: source);
      }
    });

    test('tokenizes float forms with aliases', () {
      for (final source in [
        '3.14',
        '1e10',
        '6.626e-34',
        'inf',
        '-inf',
        'nan'
      ]) {
        final token = tokenizeToml(source).first;
        expect(token.type, 'FLOAT', reason: source);
        expect(token.value, source, reason: source);
      }
    });

    test('recognizes booleans before bare keys', () {
      expect(tokenizeToml('true').first.type, 'TRUE');
      expect(tokenizeToml('false').first.type, 'FALSE');
      expect(tokenizeToml('server-name').first.type, 'BARE_KEY');
    });

    test('recognizes date and time literals before numbers and keys', () {
      final cases = {
        '1979-05-27T07:32:00Z': 'OFFSET_DATETIME',
        '1979-05-27T07:32:00+09:00': 'OFFSET_DATETIME',
        '1979-05-27T07:32:00': 'LOCAL_DATETIME',
        '1979-05-27': 'LOCAL_DATE',
        '07:32:00': 'LOCAL_TIME',
        '07:32:00.999': 'LOCAL_TIME',
      };
      for (final entry in cases.entries) {
        expect(
          tokenizeToml(entry.key).first.type,
          entry.value,
          reason: entry.key,
        );
      }
    });
  });

  group('TOML documents', () {
    test('tokenizes dotted keys, tables, arrays, and inline tables', () {
      expect(
        meaningfulTypes('a.b = [1, 2]'),
        [
          'BARE_KEY',
          'DOT',
          'BARE_KEY',
          'EQUALS',
          'LBRACKET',
          'INTEGER',
          'COMMA',
          'INTEGER',
          'RBRACKET'
        ],
      );
      expect(
        meaningfulTypes('[[products]]'),
        ['LBRACKET', 'LBRACKET', 'BARE_KEY', 'RBRACKET', 'RBRACKET'],
      );
      expect(
        meaningfulTypes('{ x = 1, y = 2 }'),
        [
          'LBRACE',
          'BARE_KEY',
          'EQUALS',
          'INTEGER',
          'COMMA',
          'BARE_KEY',
          'EQUALS',
          'INTEGER',
          'RBRACE'
        ],
      );
    });

    test('keeps newlines significant and skips comments', () {
      final tokens = tokenizeToml('a = 1\n# comment\nb = 2');
      expect(tokens.where((token) => token.type == 'NEWLINE'), hasLength(2));
      expect(tokens.any((token) => token.type == 'COMMENT'), isFalse);
      expect(
        meaningfulTypes('a = 1\n# comment\nb = 2'),
        ['BARE_KEY', 'EQUALS', 'INTEGER', 'BARE_KEY', 'EQUALS', 'INTEGER'],
      );
    });

    test('tracks token positions across lines', () {
      final tokens = tokenizeToml('a = 1\nb = 2');
      final b = tokens.firstWhere(
        (token) => token.type == 'BARE_KEY' && token.value == 'b',
      );
      expect(b.line, 2);
      expect(b.column, 1);
    });

    test('returns only EOF for empty or ignored input', () {
      expect(tokenizeToml('').map((token) => token.type), ['EOF']);
      expect(
          tokenizeToml('  \t # comment').map((token) => token.type), ['EOF']);
    });

    test('rejects characters outside the TOML token grammar', () {
      expect(() => tokenizeToml('@'), throwsA(isA<LexerError>()));
    });
  });
}
