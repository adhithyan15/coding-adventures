import 'package:coding_adventures_grammar_tools/grammar_tools.dart';
import 'package:test/test.dart';

void main() {
  group('parseTokenGrammar', () {
    test('parses regex and literal token definitions', () {
      final grammar = parseTokenGrammar('''NUMBER = /[0-9]+/
PLUS = "+"
''');

      expect(grammar.definitions.length, 2);
      expect(
        grammar.definitions.first,
        const TokenDefinition(
          name: 'NUMBER',
          pattern: '[0-9]+',
          isRegex: true,
          lineNumber: 1,
        ),
      );
      expect(grammar.definitions[1].isRegex, isFalse);
      expect(grammar.definitions[1].pattern, '+');
    });

    test('parses keywords, reserved, skip, errors, and aliases', () {
      final grammar = parseTokenGrammar('''
NAME = /[a-z]+/
STRING_DQ = /"[^"]*"/ -> STRING
mode: indentation
escapes: none
case_sensitive: false
keywords:
  if
reserved:
  class
skip:
  SPACE = /[ \t]+/
errors:
  BAD = /@+/
context_keywords:
  async
soft_keywords:
  match
''');

      expect(grammar.mode, 'indentation');
      expect(grammar.escapeMode, 'none');
      expect(grammar.caseSensitive, isFalse);
      expect(grammar.keywords, ['if']);
      expect(grammar.reservedKeywords, ['class']);
      expect(grammar.skipDefinitions.single.name, 'SPACE');
      expect(grammar.errorDefinitions.single.name, 'BAD');
      expect(grammar.contextKeywords, ['async']);
      expect(grammar.softKeywords, ['match']);
      expect(grammar.definitions[1].alias, 'STRING');
    });

    test('parses pattern groups', () {
      final grammar = parseTokenGrammar('''
TEXT = /[^<]+/
group tag:
  TAG_NAME = /[a-z]+/
  EQUALS = "="
''');

      expect(grammar.groups.keys, ['tag']);
      expect(grammar.groups['tag']!.definitions.length, 2);
    });

    test('reports malformed definitions with line numbers', () {
      expect(
        () => parseTokenGrammar('NUMBER /[0-9]+/'),
        throwsA(
          isA<TokenGrammarError>().having(
            (error) => error.lineNumber,
            'lineNumber',
            1,
          ),
        ),
      );
    });
  });

  group('validateTokenGrammar', () {
    test('detects duplicate names and invalid mode', () {
      final grammar = parseTokenGrammar('''
name = /[a-z]+/
name = /[a-z0-9]+/
mode: weird
''');

      final issues = validateTokenGrammar(grammar);
      expect(
        issues.any((issue) => issue.contains('Duplicate token name')),
        isTrue,
      );
      expect(
        issues.any((issue) => issue.contains("Unknown lexer mode 'weird'")),
        isTrue,
      );
      expect(
        issues.any(
          (issue) => issue.contains("Token name 'name' should be UPPER_CASE"),
        ),
        isTrue,
      );
    });

    test('includes aliases in token names', () {
      final grammar = parseTokenGrammar('STRING_DQ = /"[^"]*"/ -> STRING');
      expect(grammar.tokenNames(), containsAll(['STRING_DQ', 'STRING']));
      expect(grammar.effectiveTokenNames(), {'STRING'});
    });
  });
}
