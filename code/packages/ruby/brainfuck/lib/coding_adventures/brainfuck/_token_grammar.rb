# frozen_string_literal: true
# AUTO-GENERATED FILE — DO NOT EDIT
# Source: brainfuck.tokens
# Regenerate with: grammar-tools compile-tokens brainfuck.tokens
#
# This file embeds a TokenGrammar as native Ruby data structures.
# Downstream packages require this file directly instead of reading
# and parsing the .tokens file at runtime.

require "coding_adventures_grammar_tools"

GT = CodingAdventures::GrammarTools unless defined?(GT)

TOKEN_GRAMMAR = GT::TokenGrammar.new(
  version: 1,
  case_insensitive: false,
  case_sensitive: true,
  definitions: [
      GT::TokenDefinition.new(
        name: "RIGHT",
        pattern: ">",
        is_regex: false,
        line_number: 23,
        alias_name: nil,
      ),
      GT::TokenDefinition.new(
        name: "LEFT",
        pattern: "<",
        is_regex: false,
        line_number: 24,
        alias_name: nil,
      ),
      GT::TokenDefinition.new(
        name: "INC",
        pattern: "+",
        is_regex: false,
        line_number: 29,
        alias_name: nil,
      ),
      GT::TokenDefinition.new(
        name: "DEC",
        pattern: "-",
        is_regex: false,
        line_number: 30,
        alias_name: nil,
      ),
      GT::TokenDefinition.new(
        name: "OUTPUT",
        pattern: ".",
        is_regex: false,
        line_number: 35,
        alias_name: nil,
      ),
      GT::TokenDefinition.new(
        name: "INPUT",
        pattern: ",",
        is_regex: false,
        line_number: 36,
        alias_name: nil,
      ),
      GT::TokenDefinition.new(
        name: "LOOP_START",
        pattern: "[",
        is_regex: false,
        line_number: 41,
        alias_name: nil,
      ),
      GT::TokenDefinition.new(
        name: "LOOP_END",
        pattern: "]",
        is_regex: false,
        line_number: 42,
        alias_name: nil,
      ),
    ],
  keywords: [],
  mode: nil,
  escape_mode: nil,
  skip_definitions: [
      GT::TokenDefinition.new(
        name: "WHITESPACE",
        pattern: "[ \\t\\r\\n]+",
        is_regex: true,
        line_number: 65,
        alias_name: nil,
      ),
      GT::TokenDefinition.new(
        name: "COMMENT",
        pattern: "[^><+\\-.,\\[\\] \\t\\r\\n]+",
        is_regex: true,
        line_number: 66,
        alias_name: nil,
      ),
    ],
  reserved_keywords: [],
  error_definitions: [],
  groups: {},
  layout_keywords: [],
  context_keywords: [],
  soft_keywords: [],
)
