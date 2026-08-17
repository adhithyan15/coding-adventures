# frozen_string_literal: true
# AUTO-GENERATED FILE — DO NOT EDIT
# Source: lisp.tokens
# Regenerate with: grammar-tools compile-tokens lisp.tokens
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
        name: "NUMBER",
        pattern: "-?[0-9]+",
        is_regex: true,
        line_number: 11,
        alias_name: nil,
      ),
      GT::TokenDefinition.new(
        name: "SYMBOL",
        pattern: "[a-zA-Z_+\\-*\\/=<>!?&][a-zA-Z0-9_+\\-*\\/=<>!?&]*",
        is_regex: true,
        line_number: 12,
        alias_name: nil,
      ),
      GT::TokenDefinition.new(
        name: "STRING",
        pattern: "\"([^\"\\\\]|\\\\.)*\"",
        is_regex: true,
        line_number: 13,
        alias_name: nil,
      ),
      GT::TokenDefinition.new(
        name: "LPAREN",
        pattern: "(",
        is_regex: false,
        line_number: 14,
        alias_name: nil,
      ),
      GT::TokenDefinition.new(
        name: "RPAREN",
        pattern: ")",
        is_regex: false,
        line_number: 15,
        alias_name: nil,
      ),
      GT::TokenDefinition.new(
        name: "QUOTE",
        pattern: "'",
        is_regex: false,
        line_number: 16,
        alias_name: nil,
      ),
      GT::TokenDefinition.new(
        name: "DOT",
        pattern: ".",
        is_regex: false,
        line_number: 17,
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
        line_number: 8,
        alias_name: nil,
      ),
      GT::TokenDefinition.new(
        name: "COMMENT",
        pattern: ";[^\\n]*",
        is_regex: true,
        line_number: 9,
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
