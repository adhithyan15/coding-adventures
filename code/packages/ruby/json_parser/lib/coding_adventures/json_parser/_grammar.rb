# frozen_string_literal: true
# AUTO-GENERATED FILE — DO NOT EDIT
# Source: json.grammar
# Regenerate with: grammar-tools compile-grammar json.grammar
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
      name: "value",
      body: GT::Alternation.new(choices: [
        GT::RuleReference.new(name: "object", is_token: false),
        GT::RuleReference.new(name: "array", is_token: false),
        GT::RuleReference.new(name: "STRING", is_token: true),
        GT::RuleReference.new(name: "NUMBER", is_token: true),
        GT::RuleReference.new(name: "TRUE", is_token: true),
        GT::RuleReference.new(name: "FALSE", is_token: true),
        GT::RuleReference.new(name: "NULL", is_token: true),
      ]),
      line_number: 28,
    ),
    GT::GrammarRule.new(
      name: "object",
      body: GT::Sequence.new(elements: [
        GT::RuleReference.new(name: "LBRACE", is_token: true),
        GT::OptionalElement.new(element: GT::Sequence.new(elements: [
            GT::RuleReference.new(name: "pair", is_token: false),
            GT::Repetition.new(element: GT::Sequence.new(elements: [
                GT::RuleReference.new(name: "COMMA", is_token: true),
                GT::RuleReference.new(name: "pair", is_token: false),
              ])),
          ])),
        GT::RuleReference.new(name: "RBRACE", is_token: true),
      ]),
      line_number: 34,
    ),
    GT::GrammarRule.new(
      name: "pair",
      body: GT::Sequence.new(elements: [
        GT::RuleReference.new(name: "STRING", is_token: true),
        GT::RuleReference.new(name: "COLON", is_token: true),
        GT::RuleReference.new(name: "value", is_token: false),
      ]),
      line_number: 38,
    ),
    GT::GrammarRule.new(
      name: "array",
      body: GT::Sequence.new(elements: [
        GT::RuleReference.new(name: "LBRACKET", is_token: true),
        GT::OptionalElement.new(element: GT::Sequence.new(elements: [
            GT::RuleReference.new(name: "value", is_token: false),
            GT::Repetition.new(element: GT::Sequence.new(elements: [
                GT::RuleReference.new(name: "COMMA", is_token: true),
                GT::RuleReference.new(name: "value", is_token: false),
              ])),
          ])),
        GT::RuleReference.new(name: "RBRACKET", is_token: true),
      ]),
      line_number: 42,
    ),
  ],
)
