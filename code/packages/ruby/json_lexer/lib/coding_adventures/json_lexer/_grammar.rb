# frozen_string_literal: true
# AUTO-GENERATED FILE — DO NOT EDIT
# Source: json.tokens
# Regenerate with: grammar-tools compile-tokens json.tokens
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
        name: "STRING",
        pattern: "\"([^\"\\\\]|\\\\[\"\\\\\\x2fbfnrt]|\\\\u[0-9a-fA-F]{4})*\"",
        is_regex: true,
        line_number: 30,
        alias_name: nil,
      ),
      GT::TokenDefinition.new(
        name: "NUMBER",
        pattern: "-?[0-9]+\\.?[0-9]*[eE]?[-+]?[0-9]*",
        is_regex: true,
        line_number: 37,
        alias_name: nil,
      ),
      GT::TokenDefinition.new(
        name: "TRUE",
        pattern: "true",
        is_regex: false,
        line_number: 41,
        alias_name: nil,
      ),
      GT::TokenDefinition.new(
        name: "FALSE",
        pattern: "false",
        is_regex: false,
        line_number: 42,
        alias_name: nil,
      ),
      GT::TokenDefinition.new(
        name: "NULL",
        pattern: "null",
        is_regex: false,
        line_number: 43,
        alias_name: nil,
      ),
      GT::TokenDefinition.new(
        name: "LBRACE",
        pattern: "{",
        is_regex: false,
        line_number: 49,
        alias_name: nil,
      ),
      GT::TokenDefinition.new(
        name: "RBRACE",
        pattern: "}",
        is_regex: false,
        line_number: 50,
        alias_name: nil,
      ),
      GT::TokenDefinition.new(
        name: "LBRACKET",
        pattern: "[",
        is_regex: false,
        line_number: 51,
        alias_name: nil,
      ),
      GT::TokenDefinition.new(
        name: "RBRACKET",
        pattern: "]",
        is_regex: false,
        line_number: 52,
        alias_name: nil,
      ),
      GT::TokenDefinition.new(
        name: "COLON",
        pattern: ":",
        is_regex: false,
        line_number: 53,
        alias_name: nil,
      ),
      GT::TokenDefinition.new(
        name: "COMMA",
        pattern: ",",
        is_regex: false,
        line_number: 54,
        alias_name: nil,
      ),
    ],
  keywords: [],
  mode: nil,
  escape_mode: "none",
  skip_definitions: [
      GT::TokenDefinition.new(
        name: "WHITESPACE",
        pattern: "[ \\t\\r\\n]+",
        is_regex: true,
        line_number: 65,
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
