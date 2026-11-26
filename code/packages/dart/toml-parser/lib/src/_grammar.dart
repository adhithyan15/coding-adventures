// AUTO-GENERATED FILE - DO NOT EDIT
// Source: code/grammars/toml/toml.grammar
// Regenerate with: grammar-tools compile-grammar <source.grammar>
import 'package:coding_adventures_grammar_tools/grammar_tools.dart';

final parserGrammar = ParserGrammar(
  version: 1,
  rules: [
    GrammarRule(
        name: "document",
        body: Repetition(
            element: Alternation(choices: [
          RuleReference("NEWLINE", isToken: true),
          RuleReference("expression", isToken: false)
        ])),
        lineNumber: 38),
    GrammarRule(
        name: "expression",
        body: Alternation(choices: [
          RuleReference("array_table_header", isToken: false),
          RuleReference("table_header", isToken: false),
          RuleReference("keyval", isToken: false)
        ]),
        lineNumber: 49),
    GrammarRule(
        name: "keyval",
        body: Sequence(elements: [
          RuleReference("key", isToken: false),
          RuleReference("EQUALS", isToken: true),
          RuleReference("value", isToken: false)
        ]),
        lineNumber: 57),
    GrammarRule(
        name: "key",
        body: Sequence(elements: [
          RuleReference("simple_key", isToken: false),
          Repetition(
              element: Sequence(elements: [
            RuleReference("DOT", isToken: true),
            RuleReference("simple_key", isToken: false)
          ]))
        ]),
        lineNumber: 65),
    GrammarRule(
        name: "simple_key",
        body: Alternation(choices: [
          RuleReference("BARE_KEY", isToken: true),
          RuleReference("BASIC_STRING", isToken: true),
          RuleReference("LITERAL_STRING", isToken: true),
          RuleReference("TRUE", isToken: true),
          RuleReference("FALSE", isToken: true),
          RuleReference("INTEGER", isToken: true),
          RuleReference("FLOAT", isToken: true),
          RuleReference("OFFSET_DATETIME", isToken: true),
          RuleReference("LOCAL_DATETIME", isToken: true),
          RuleReference("LOCAL_DATE", isToken: true),
          RuleReference("LOCAL_TIME", isToken: true)
        ]),
        lineNumber: 82),
    GrammarRule(
        name: "table_header",
        body: Sequence(elements: [
          RuleReference("LBRACKET", isToken: true),
          RuleReference("key", isToken: false),
          RuleReference("RBRACKET", isToken: true)
        ]),
        lineNumber: 92),
    GrammarRule(
        name: "array_table_header",
        body: Sequence(elements: [
          RuleReference("LBRACKET", isToken: true),
          RuleReference("LBRACKET", isToken: true),
          RuleReference("key", isToken: false),
          RuleReference("RBRACKET", isToken: true),
          RuleReference("RBRACKET", isToken: true)
        ]),
        lineNumber: 104),
    GrammarRule(
        name: "value",
        body: Alternation(choices: [
          RuleReference("BASIC_STRING", isToken: true),
          RuleReference("ML_BASIC_STRING", isToken: true),
          RuleReference("LITERAL_STRING", isToken: true),
          RuleReference("ML_LITERAL_STRING", isToken: true),
          RuleReference("INTEGER", isToken: true),
          RuleReference("FLOAT", isToken: true),
          RuleReference("TRUE", isToken: true),
          RuleReference("FALSE", isToken: true),
          RuleReference("OFFSET_DATETIME", isToken: true),
          RuleReference("LOCAL_DATETIME", isToken: true),
          RuleReference("LOCAL_DATE", isToken: true),
          RuleReference("LOCAL_TIME", isToken: true),
          RuleReference("array", isToken: false),
          RuleReference("inline_table", isToken: false)
        ]),
        lineNumber: 121),
    GrammarRule(
        name: "array",
        body: Sequence(elements: [
          RuleReference("LBRACKET", isToken: true),
          RuleReference("array_values", isToken: false),
          RuleReference("RBRACKET", isToken: true)
        ]),
        lineNumber: 140),
    GrammarRule(
        name: "array_values",
        body: Sequence(elements: [
          Repetition(element: RuleReference("NEWLINE", isToken: true)),
          Optional(
              element: Sequence(elements: [
            RuleReference("value", isToken: false),
            Repetition(element: RuleReference("NEWLINE", isToken: true)),
            Repetition(
                element: Sequence(elements: [
              RuleReference("COMMA", isToken: true),
              Repetition(element: RuleReference("NEWLINE", isToken: true)),
              RuleReference("value", isToken: false),
              Repetition(element: RuleReference("NEWLINE", isToken: true))
            ])),
            Optional(element: RuleReference("COMMA", isToken: true)),
            Repetition(element: RuleReference("NEWLINE", isToken: true))
          ]))
        ]),
        lineNumber: 142),
    GrammarRule(
        name: "inline_table",
        body: Sequence(elements: [
          RuleReference("LBRACE", isToken: true),
          Optional(
              element: Sequence(elements: [
            RuleReference("keyval", isToken: false),
            Repetition(
                element: Sequence(elements: [
              RuleReference("COMMA", isToken: true),
              RuleReference("keyval", isToken: false)
            ]))
          ])),
          RuleReference("RBRACE", isToken: true)
        ]),
        lineNumber: 162),
  ],
);
