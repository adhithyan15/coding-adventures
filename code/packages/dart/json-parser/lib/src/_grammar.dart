import 'package:coding_adventures_grammar_tools/grammar_tools.dart';

final parserGrammar = ParserGrammar(
  version: 1,
  rules: [
    GrammarRule(
      name: 'value',
      body: Alternation(
        choices: [
          const RuleReference('object', isToken: false),
          const RuleReference('array', isToken: false),
          const RuleReference('STRING', isToken: true),
          const RuleReference('NUMBER', isToken: true),
          const RuleReference('TRUE', isToken: true),
          const RuleReference('FALSE', isToken: true),
          const RuleReference('NULL', isToken: true),
        ],
      ),
      lineNumber: 28,
    ),
    GrammarRule(
      name: 'object',
      body: Sequence(
        elements: [
          const RuleReference('LBRACE', isToken: true),
          Optional(
            element: Sequence(
              elements: [
                const RuleReference('pair', isToken: false),
                Repetition(
                  element: Sequence(
                    elements: [
                      const RuleReference('COMMA', isToken: true),
                      const RuleReference('pair', isToken: false),
                    ],
                  ),
                ),
              ],
            ),
          ),
          const RuleReference('RBRACE', isToken: true),
        ],
      ),
      lineNumber: 34,
    ),
    GrammarRule(
      name: 'pair',
      body: Sequence(
        elements: [
          const RuleReference('STRING', isToken: true),
          const RuleReference('COLON', isToken: true),
          const RuleReference('value', isToken: false),
        ],
      ),
      lineNumber: 38,
    ),
    GrammarRule(
      name: 'array',
      body: Sequence(
        elements: [
          const RuleReference('LBRACKET', isToken: true),
          Optional(
            element: Sequence(
              elements: [
                const RuleReference('value', isToken: false),
                Repetition(
                  element: Sequence(
                    elements: [
                      const RuleReference('COMMA', isToken: true),
                      const RuleReference('value', isToken: false),
                    ],
                  ),
                ),
              ],
            ),
          ),
          const RuleReference('RBRACKET', isToken: true),
        ],
      ),
      lineNumber: 42,
    ),
  ],
);
