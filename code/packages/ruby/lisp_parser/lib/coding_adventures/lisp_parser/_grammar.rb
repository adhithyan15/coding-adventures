# frozen_string_literal: true
# AUTO-GENERATED FILE — DO NOT EDIT
# Source: lisp.grammar
# Regenerate with: grammar-tools compile-grammar lisp.grammar
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
      name: "program",
      body: GT::Repetition.new(element: GT::RuleReference.new(name: "sexpr", is_token: false)),
      line_number: 2,
    ),
    GT::GrammarRule.new(
      name: "sexpr",
      body: GT::Alternation.new(choices: [
        GT::RuleReference.new(name: "atom", is_token: false),
        GT::RuleReference.new(name: "list", is_token: false),
        GT::RuleReference.new(name: "quoted", is_token: false),
      ]),
      line_number: 3,
    ),
    GT::GrammarRule.new(
      name: "atom",
      body: GT::Alternation.new(choices: [
        GT::RuleReference.new(name: "NUMBER", is_token: true),
        GT::RuleReference.new(name: "SYMBOL", is_token: true),
        GT::RuleReference.new(name: "STRING", is_token: true),
      ]),
      line_number: 4,
    ),
    GT::GrammarRule.new(
      name: "list",
      body: GT::Sequence.new(elements: [
        GT::RuleReference.new(name: "LPAREN", is_token: true),
        GT::RuleReference.new(name: "list_body", is_token: false),
        GT::RuleReference.new(name: "RPAREN", is_token: true),
      ]),
      line_number: 5,
    ),
    GT::GrammarRule.new(
      name: "list_body",
      body: GT::OptionalElement.new(element: GT::Sequence.new(elements: [
          GT::RuleReference.new(name: "sexpr", is_token: false),
          GT::Repetition.new(element: GT::RuleReference.new(name: "sexpr", is_token: false)),
          GT::OptionalElement.new(element: GT::Sequence.new(elements: [
              GT::RuleReference.new(name: "DOT", is_token: true),
              GT::RuleReference.new(name: "sexpr", is_token: false),
            ])),
        ])),
      line_number: 6,
    ),
    GT::GrammarRule.new(
      name: "quoted",
      body: GT::Sequence.new(elements: [
        GT::RuleReference.new(name: "QUOTE", is_token: true),
        GT::RuleReference.new(name: "sexpr", is_token: false),
      ]),
      line_number: 7,
    ),
  ],
)
