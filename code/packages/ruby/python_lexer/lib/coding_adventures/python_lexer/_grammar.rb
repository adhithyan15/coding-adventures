# frozen_string_literal: true
# AUTO-GENERATED FILE — DO NOT EDIT
# Source: python.tokens
# Regenerate with: grammar-tools compile-tokens python.tokens
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
        name: "NAME",
        pattern: "[a-zA-Z_][a-zA-Z0-9_]*",
        is_regex: true,
        line_number: 13,
        alias_name: nil,
      ),
      GT::TokenDefinition.new(
        name: "NUMBER",
        pattern: "[0-9]+",
        is_regex: true,
        line_number: 14,
        alias_name: nil,
      ),
      GT::TokenDefinition.new(
        name: "STRING",
        pattern: "\"([^\"\\\\]|\\\\.)*\"",
        is_regex: true,
        line_number: 15,
        alias_name: nil,
      ),
      GT::TokenDefinition.new(
        name: "EQUALS_EQUALS",
        pattern: "==",
        is_regex: false,
        line_number: 18,
        alias_name: nil,
      ),
      GT::TokenDefinition.new(
        name: "EQUALS",
        pattern: "=",
        is_regex: false,
        line_number: 21,
        alias_name: nil,
      ),
      GT::TokenDefinition.new(
        name: "PLUS",
        pattern: "+",
        is_regex: false,
        line_number: 22,
        alias_name: nil,
      ),
      GT::TokenDefinition.new(
        name: "MINUS",
        pattern: "-",
        is_regex: false,
        line_number: 23,
        alias_name: nil,
      ),
      GT::TokenDefinition.new(
        name: "STAR",
        pattern: "*",
        is_regex: false,
        line_number: 24,
        alias_name: nil,
      ),
      GT::TokenDefinition.new(
        name: "SLASH",
        pattern: "/",
        is_regex: false,
        line_number: 25,
        alias_name: nil,
      ),
      GT::TokenDefinition.new(
        name: "LPAREN",
        pattern: "(",
        is_regex: false,
        line_number: 28,
        alias_name: nil,
      ),
      GT::TokenDefinition.new(
        name: "RPAREN",
        pattern: ")",
        is_regex: false,
        line_number: 29,
        alias_name: nil,
      ),
      GT::TokenDefinition.new(
        name: "COMMA",
        pattern: ",",
        is_regex: false,
        line_number: 30,
        alias_name: nil,
      ),
      GT::TokenDefinition.new(
        name: "COLON",
        pattern: ":",
        is_regex: false,
        line_number: 31,
        alias_name: nil,
      ),
    ],
  keywords: ["if", "else", "elif", "while", "for", "def", "return", "class", "import", "from", "as", "True", "False", "None"],
  mode: nil,
  escape_mode: nil,
  skip_definitions: [],
  reserved_keywords: [],
  error_definitions: [],
  groups: {},
  context_keywords: [],
  soft_keywords: [],
)
