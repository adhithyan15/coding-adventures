import 'package:coding_adventures_grammar_tools/grammar_tools.dart';
import 'package:coding_adventures_lexer/lexer.dart';
import 'package:test/test.dart';

void main() {
  group('grammarTokenize', () {
    test('tokenizes a minimal grammar with skip patterns', () {
      final grammar = parseTokenGrammar('''
NUMBER = /[0-9]+/
PLUS = "+"
skip:
  SPACE = /[ \t]+/
''');

      final tokens = grammarTokenize('12 + 34', grammar);
      expect(tokens.map((token) => token.type).toList(), [
        'NUMBER',
        'PLUS',
        'NUMBER',
        'EOF',
      ]);
      expect(tokens.map((token) => token.value).toList(), [
        '12',
        '+',
        '34',
        '',
      ]);
    });

    test('respects aliases and string unescaping by default', () {
      final grammar = parseTokenGrammar(
        'STRING_DQ = /"([^"\\\\]|\\\\.)*"/ -> STRING',
      );

      final tokens = grammarTokenize(r'"line1\nline2"', grammar);
      expect(tokens.first.type, 'STRING');
      expect(tokens.first.value, 'line1\nline2');
    });

    test('keeps raw string escapes when escapes none is enabled', () {
      final grammar = parseTokenGrammar('''
escapes: none
STRING = /"([^"\\\\]|\\\\.)*"/
''');

      final tokens = grammarTokenize(r'"line1\nline2"', grammar);
      expect(tokens.first.value, r'line1\nline2');
    });

    test('emits NEWLINE when newline is not skipped', () {
      final grammar = parseTokenGrammar('NUMBER = /[0-9]+/');

      final tokens = grammarTokenize('1\n2', grammar);
      expect(tokens.map((token) => token.type).toList(), [
        'NUMBER',
        'NEWLINE',
        'NUMBER',
        'EOF',
      ]);
    });
  });
}
