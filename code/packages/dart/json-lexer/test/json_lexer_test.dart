import 'package:coding_adventures_lexer/lexer.dart';
import 'package:coding_adventures_json_lexer/json_lexer.dart';
import 'package:test/test.dart';

void main() {
  group('tokenizeJson', () {
    test('tokenizes numbers and literals', () {
      expect(
        tokenizeJson(
          '[42, -0.5, true, false, null]',
        ).map((token) => token.type).toList(),
        [
          'LBRACKET',
          'NUMBER',
          'COMMA',
          'NUMBER',
          'COMMA',
          'TRUE',
          'COMMA',
          'FALSE',
          'COMMA',
          'NULL',
          'RBRACKET',
          'EOF',
        ],
      );
    });

    test('strips quotes and preserves raw escapes for strings', () {
      final tokens = tokenizeJson(r'"line1\nline2"');
      expect(tokens.first.type, 'STRING');
      expect(tokens.first.value, r'line1\nline2');
    });

    test('skips insignificant whitespace', () {
      final compact = tokenizeJson('{"a":1}');
      final spaced = tokenizeJson('{\n  "a" : 1 \n}');
      expect(
        spaced.map((token) => token.type).toList(),
        compact.map((token) => token.type).toList(),
      );
      expect(
        spaced.map((token) => token.value).toList(),
        compact.map((token) => token.value).toList(),
      );
    });

    test('tracks line and column positions', () {
      final tokens = tokenizeJson('{\n  "key": 1\n}');
      expect(tokens[0].line, 1);
      expect(tokens[0].column, 1);
      expect(tokens[1].type, 'STRING');
      expect(tokens[1].line, 2);
      expect(tokens[1].column, 3);
    });

    test('throws on invalid JSON source', () {
      expect(() => tokenizeJson("undefined"), throwsA(isA<LexerError>()));
      expect(() => tokenizeJson("'hello'"), throwsA(isA<LexerError>()));
    });
  });
}
