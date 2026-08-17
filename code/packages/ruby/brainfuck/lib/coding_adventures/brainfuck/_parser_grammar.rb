# frozen_string_literal: true
# AUTO-GENERATED FILE — DO NOT EDIT
# Source: brainfuck.grammar
# Regenerate with: grammar-tools compile-grammar brainfuck.grammar
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
      body: GT::Repetition.new(element: GT::RuleReference.new(name: "instruction", is_token: false)),
      line_number: 15,
    ),
    GT::GrammarRule.new(
      name: "instruction",
      body: GT::Alternation.new(choices: [
        GT::RuleReference.new(name: "loop", is_token: false),
        GT::RuleReference.new(name: "command", is_token: false),
      ]),
      line_number: 21,
    ),
    GT::GrammarRule.new(
      name: "loop",
      body: GT::Sequence.new(elements: [
        GT::RuleReference.new(name: "LOOP_START", is_token: true),
        GT::Repetition.new(element: GT::RuleReference.new(name: "instruction", is_token: false)),
        GT::RuleReference.new(name: "LOOP_END", is_token: true),
      ]),
      line_number: 27,
    ),
    GT::GrammarRule.new(
      name: "command",
      body: GT::Alternation.new(choices: [
        GT::RuleReference.new(name: "RIGHT", is_token: true),
        GT::RuleReference.new(name: "LEFT", is_token: true),
        GT::RuleReference.new(name: "INC", is_token: true),
        GT::RuleReference.new(name: "DEC", is_token: true),
        GT::RuleReference.new(name: "OUTPUT", is_token: true),
        GT::RuleReference.new(name: "INPUT", is_token: true),
      ]),
      line_number: 32,
    ),
  ],
)
