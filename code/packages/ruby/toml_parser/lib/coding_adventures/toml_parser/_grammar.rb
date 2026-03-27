# frozen_string_literal: true
# AUTO-GENERATED FILE — DO NOT EDIT
# Source: toml.grammar
# Regenerate with: grammar-tools compile-grammar toml.grammar
#
# This file embeds a ParserGrammar as native Ruby data structures.
# Downstream packages require this file directly instead of reading
# and parsing the .grammar file at runtime.

require "coding_adventures_grammar_tools"

GT = CodingAdventures::GrammarTools unless defined?(GT)

PARSER_GRAMMAR = GT::ParserGrammar.new(
  version: 1,
  rules: [
    GT::GrammarRule.new(
      name: "document",
      body: GT::Repetition.new(element: GT::Alternation.new(choices: [
          GT::RuleReference.new(name: "NEWLINE", is_token: true),
          GT::RuleReference.new(name: "expression", is_token: false),
        ])),
      line_number: 38,
    ),
    GT::GrammarRule.new(
      name: "expression",
      body: GT::Alternation.new(choices: [
        GT::RuleReference.new(name: "array_table_header", is_token: false),
        GT::RuleReference.new(name: "table_header", is_token: false),
        GT::RuleReference.new(name: "keyval", is_token: false),
      ]),
      line_number: 49,
    ),
    GT::GrammarRule.new(
      name: "keyval",
      body: GT::Sequence.new(elements: [
        GT::RuleReference.new(name: "key", is_token: false),
        GT::RuleReference.new(name: "EQUALS", is_token: true),
        GT::RuleReference.new(name: "value", is_token: false),
      ]),
      line_number: 57,
    ),
    GT::GrammarRule.new(
      name: "key",
      body: GT::Sequence.new(elements: [
        GT::RuleReference.new(name: "simple_key", is_token: false),
        GT::Repetition.new(element: GT::Sequence.new(elements: [
            GT::RuleReference.new(name: "DOT", is_token: true),
            GT::RuleReference.new(name: "simple_key", is_token: false),
          ])),
      ]),
      line_number: 65,
    ),
    GT::GrammarRule.new(
      name: "simple_key",
      body: GT::Alternation.new(choices: [
        GT::RuleReference.new(name: "BARE_KEY", is_token: true),
        GT::RuleReference.new(name: "BASIC_STRING", is_token: true),
        GT::RuleReference.new(name: "LITERAL_STRING", is_token: true),
        GT::RuleReference.new(name: "TRUE", is_token: true),
        GT::RuleReference.new(name: "FALSE", is_token: true),
        GT::RuleReference.new(name: "INTEGER", is_token: true),
        GT::RuleReference.new(name: "FLOAT", is_token: true),
        GT::RuleReference.new(name: "OFFSET_DATETIME", is_token: true),
        GT::RuleReference.new(name: "LOCAL_DATETIME", is_token: true),
        GT::RuleReference.new(name: "LOCAL_DATE", is_token: true),
        GT::RuleReference.new(name: "LOCAL_TIME", is_token: true),
      ]),
      line_number: 82,
    ),
    GT::GrammarRule.new(
      name: "table_header",
      body: GT::Sequence.new(elements: [
        GT::RuleReference.new(name: "LBRACKET", is_token: true),
        GT::RuleReference.new(name: "key", is_token: false),
        GT::RuleReference.new(name: "RBRACKET", is_token: true),
      ]),
      line_number: 92,
    ),
    GT::GrammarRule.new(
      name: "array_table_header",
      body: GT::Sequence.new(elements: [
        GT::RuleReference.new(name: "LBRACKET", is_token: true),
        GT::RuleReference.new(name: "LBRACKET", is_token: true),
        GT::RuleReference.new(name: "key", is_token: false),
        GT::RuleReference.new(name: "RBRACKET", is_token: true),
        GT::RuleReference.new(name: "RBRACKET", is_token: true),
      ]),
      line_number: 104,
    ),
    GT::GrammarRule.new(
      name: "value",
      body: GT::Alternation.new(choices: [
        GT::RuleReference.new(name: "BASIC_STRING", is_token: true),
        GT::RuleReference.new(name: "ML_BASIC_STRING", is_token: true),
        GT::RuleReference.new(name: "LITERAL_STRING", is_token: true),
        GT::RuleReference.new(name: "ML_LITERAL_STRING", is_token: true),
        GT::RuleReference.new(name: "INTEGER", is_token: true),
        GT::RuleReference.new(name: "FLOAT", is_token: true),
        GT::RuleReference.new(name: "TRUE", is_token: true),
        GT::RuleReference.new(name: "FALSE", is_token: true),
        GT::RuleReference.new(name: "OFFSET_DATETIME", is_token: true),
        GT::RuleReference.new(name: "LOCAL_DATETIME", is_token: true),
        GT::RuleReference.new(name: "LOCAL_DATE", is_token: true),
        GT::RuleReference.new(name: "LOCAL_TIME", is_token: true),
        GT::RuleReference.new(name: "array", is_token: false),
        GT::RuleReference.new(name: "inline_table", is_token: false),
      ]),
      line_number: 121,
    ),
    GT::GrammarRule.new(
      name: "array",
      body: GT::Sequence.new(elements: [
        GT::RuleReference.new(name: "LBRACKET", is_token: true),
        GT::RuleReference.new(name: "array_values", is_token: false),
        GT::RuleReference.new(name: "RBRACKET", is_token: true),
      ]),
      line_number: 140,
    ),
    GT::GrammarRule.new(
      name: "array_values",
      body: GT::Sequence.new(elements: [
        GT::Repetition.new(element: GT::RuleReference.new(name: "NEWLINE", is_token: true)),
        GT::OptionalElement.new(element: GT::Sequence.new(elements: [
            GT::RuleReference.new(name: "value", is_token: false),
            GT::Repetition.new(element: GT::RuleReference.new(name: "NEWLINE", is_token: true)),
            GT::Repetition.new(element: GT::Sequence.new(elements: [
                GT::RuleReference.new(name: "COMMA", is_token: true),
                GT::Repetition.new(element: GT::RuleReference.new(name: "NEWLINE", is_token: true)),
                GT::RuleReference.new(name: "value", is_token: false),
                GT::Repetition.new(element: GT::RuleReference.new(name: "NEWLINE", is_token: true)),
              ])),
            GT::OptionalElement.new(element: GT::RuleReference.new(name: "COMMA", is_token: true)),
            GT::Repetition.new(element: GT::RuleReference.new(name: "NEWLINE", is_token: true)),
          ])),
      ]),
      line_number: 142,
    ),
    GT::GrammarRule.new(
      name: "inline_table",
      body: GT::Sequence.new(elements: [
        GT::RuleReference.new(name: "LBRACE", is_token: true),
        GT::OptionalElement.new(element: GT::Sequence.new(elements: [
            GT::RuleReference.new(name: "keyval", is_token: false),
            GT::Repetition.new(element: GT::Sequence.new(elements: [
                GT::RuleReference.new(name: "COMMA", is_token: true),
                GT::RuleReference.new(name: "keyval", is_token: false),
              ])),
          ])),
        GT::RuleReference.new(name: "RBRACE", is_token: true),
      ]),
      line_number: 162,
    ),
  ],
)
