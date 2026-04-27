import 'package:coding_adventures_grammar_tools/grammar_tools.dart';
import 'package:test/test.dart';

void main() {
  group('crossValidate', () {
    test('flags missing and unused tokens while respecting aliases', () {
      final tokenGrammar = parseTokenGrammar('''
STRING_DQ = /"[^"]*"/ -> STRING
UNUSED = "~"
''');
      final parserGrammar = parseParserGrammar(
        'program = STRING EOF MISSING ;',
      );

      final issues = crossValidate(tokenGrammar, parserGrammar);
      expect(
        issues,
        contains(
          "Error: Grammar references token 'MISSING' which is not defined in the tokens file",
        ),
      );
      expect(
        issues,
        contains(
          "Warning: Token 'UNUSED' (line 2) is defined but never used in the grammar",
        ),
      );
    });
  });

  group('compiler', () {
    test('compiles token grammar to Dart source', () {
      final grammar = parseTokenGrammar('NUMBER = /[0-9]+/');
      final source = compileTokenGrammar(grammar, sourceFile: 'json.tokens');

      expect(source, contains('AUTO-GENERATED FILE'));
      expect(source, contains('TokenGrammar('));
      expect(source, contains('json.tokens'));
    });

    test('compiles parser grammar to Dart source', () {
      final grammar = parseParserGrammar('program = NUMBER ;');
      final source = compileParserGrammar(grammar, sourceFile: 'json.grammar');

      expect(source, contains('ParserGrammar('));
      expect(source, contains('RuleReference("NUMBER"'));
      expect(source, contains('json.grammar'));
    });
  });
}
