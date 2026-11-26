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

    test('escapes dollar signs in emitted strings', () {
      // JSON escaping (the emitter's string strategy) leaves `$` bare,
      // which Dart would read as interpolation. Every `$` must come out
      // as `\$` so generated files always parse.
      final grammar = parseTokenGrammar(r'''
DOLLAR_EQUALS = "$="
BAD_STRING = /"[^"]*$/
''');
      final source = compileTokenGrammar(grammar);
      expect(source, contains(r'pattern: "\$="'));
      expect(source, contains(r'pattern: "\"[^\"]*\$"'));
      expect(source, isNot(contains(r'"$="')));
    });

    test('compiles parser grammar to Dart source', () {
      final grammar = parseParserGrammar('program = NUMBER ;');
      final source = compileParserGrammar(grammar, sourceFile: 'json.grammar');

      expect(source, contains('ParserGrammar('));
      expect(source, contains('RuleReference("NUMBER"'));
      expect(source, contains('json.grammar'));
    });

    test('emits empty start mode and transitions as null and const', () {
      final grammar = parseTokenGrammar('NUMBER = /[0-9]+/');
      final source = compileTokenGrammar(grammar);

      expect(source, contains('startMode: null'));
      expect(source, contains('transitions: const []'));
    });

    test('emits groups, start mode, and transitions', () {
      final grammar = parseTokenGrammar('''
NAME = /[a-z]+/
group div:
  SLASH = "/"
start_mode: div
transitions:
  on NAME if space_before -> set-mode div
  on SLASH -> set-mode default, enable-skip
''');
      final source = compileTokenGrammar(grammar);

      expect(source, contains('startMode: "div"'));
      expect(
        source,
        contains(
          'ModeTransition(onTokens: ["NAME"], onValue: null, inMode: null, '
          'conditions: [TransitionCondition.spaceBefore], '
          'actions: [SetMode("div")], lineNumber: 6)',
        ),
      );
      expect(
        source,
        contains(
          'actions: [SetMode("default"), const EnableSkip()], lineNumber: 7',
        ),
      );
      expect(source, contains('"div": PatternGroup('));
    });

    test('emits every condition and action', () {
      final grammar = parseTokenGrammar('''
NAME = /[a-z]+/
BANG = "!"
group div:
  SLASH = "/"
transitions:
  on NAME if space_before && !space_before && space_after && !space_after && line_start && !line_start -> set-mode div, push div, pop, enable-skip, disable-skip
  on BANG -> refuse
''');
      final source = compileTokenGrammar(grammar);

      for (final condition in [
        'spaceBefore',
        'noSpaceBefore',
        'spaceAfter',
        'noSpaceAfter',
        'lineStart',
        'notLineStart',
      ]) {
        expect(source, contains('TransitionCondition.$condition'));
      }
      for (final action in [
        'SetMode("div")',
        'PushRegion("div")',
        'const PopRegion()',
        'const EnableSkip()',
        'const DisableSkip()',
        'const RefuseToken()',
      ]) {
        expect(source, contains(action));
      }
    });
  });
}
