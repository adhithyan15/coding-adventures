import 'package:coding_adventures_grammar_tools/grammar_tools.dart';
import 'package:test/test.dart';

void main() {
  group('parseParserGrammar', () {
    test('parses sequences and alternations', () {
      final grammar = parseParserGrammar(
        'assignment = NAME EQUALS NUMBER | STRING ;',
      );
      final rule = grammar.rules.single;
      expect(rule.name, 'assignment');
      expect(rule.body, isA<Alternation>());
    });

    test('parses repetition, optional, grouping, and lookahead', () {
      final grammar = parseParserGrammar(
        'expression = term { ( PLUS | MINUS ) term } [ SEMI ] &EOF ;',
      );
      final rule = grammar.rules.single;
      final body = rule.body as Sequence;

      expect(body.elements[1], isA<Repetition>());
      expect(body.elements[2], isA<Optional>());
      expect(body.elements[3], isA<PositiveLookahead>());
    });

    test('parses separated repetition and one-or-more suffix', () {
      final grammar = parseParserGrammar('args = { expression // COMMA }+ ;');
      final rule = grammar.rules.single;
      expect(
        rule.body,
        const SeparatedRepetition(
          element: RuleReference('expression', isToken: false),
          separator: RuleReference('COMMA', isToken: true),
          atLeastOne: true,
        ),
      );
    });

    test('parses version magic comments', () {
      final grammar = parseParserGrammar('''
# @version 2
program = NUMBER ;
''');
      expect(grammar.version, 2);
    });

    test('throws on malformed grammar', () {
      expect(
        () => parseParserGrammar('program = NUMBER'),
        throwsA(isA<ParserGrammarError>()),
      );
    });
  });

  group('validateParserGrammar', () {
    test('detects undefined and unreachable rules', () {
      final grammar = parseParserGrammar('''
program = expression ;
expression = NUMBER ;
unused = STRING ;
''');

      final issues = validateParserGrammar(grammar, tokenNames: {'NUMBER'});

      expect(issues, contains("Undefined token reference: 'STRING'"));
      expect(
        issues.any(
          (issue) =>
              issue.contains("Rule 'unused' is defined but never referenced"),
        ),
        isTrue,
      );
    });
  });
}
