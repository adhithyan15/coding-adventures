# frozen_string_literal: true
# AUTO-GENERATED FILE — DO NOT EDIT
# Source: ruby.tokens
# Regenerate with: grammar-tools compile-tokens ruby.tokens
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
        line_number: 23,
        alias_name: nil,
      ),
      GT::TokenDefinition.new(
        name: "NUMBER",
        pattern: "[0-9]+",
        is_regex: true,
        line_number: 24,
        alias_name: nil,
      ),
      GT::TokenDefinition.new(
        name: "STRING",
        pattern: "\"([^\"\\\\]|\\\\.)*\"",
        is_regex: true,
        line_number: 25,
        alias_name: nil,
      ),
      GT::TokenDefinition.new(
        name: "EQUALS_EQUALS",
        pattern: "==",
        is_regex: false,
        line_number: 28,
        alias_name: nil,
      ),
      GT::TokenDefinition.new(
        name: "DOT_DOT",
        pattern: "..",
        is_regex: false,
        line_number: 29,
        alias_name: nil,
      ),
      GT::TokenDefinition.new(
        name: "HASH_ROCKET",
        pattern: "=>",
        is_regex: false,
        line_number: 30,
        alias_name: nil,
      ),
      GT::TokenDefinition.new(
        name: "NOT_EQUALS",
        pattern: "!=",
        is_regex: false,
        line_number: 31,
        alias_name: nil,
      ),
      GT::TokenDefinition.new(
        name: "LESS_EQUALS",
        pattern: "<=",
        is_regex: false,
        line_number: 32,
        alias_name: nil,
      ),
      GT::TokenDefinition.new(
        name: "GREATER_EQUALS",
        pattern: ">=",
        is_regex: false,
        line_number: 33,
        alias_name: nil,
      ),
      GT::TokenDefinition.new(
        name: "EQUALS",
        pattern: "=",
        is_regex: false,
        line_number: 36,
        alias_name: nil,
      ),
      GT::TokenDefinition.new(
        name: "PLUS",
        pattern: "+",
        is_regex: false,
        line_number: 37,
        alias_name: nil,
      ),
      GT::TokenDefinition.new(
        name: "MINUS",
        pattern: "-",
        is_regex: false,
        line_number: 38,
        alias_name: nil,
      ),
      GT::TokenDefinition.new(
        name: "STAR",
        pattern: "*",
        is_regex: false,
        line_number: 39,
        alias_name: nil,
      ),
      GT::TokenDefinition.new(
        name: "SLASH",
        pattern: "/",
        is_regex: false,
        line_number: 40,
        alias_name: nil,
      ),
      GT::TokenDefinition.new(
        name: "LESS_THAN",
        pattern: "<",
        is_regex: false,
        line_number: 43,
        alias_name: nil,
      ),
      GT::TokenDefinition.new(
        name: "GREATER_THAN",
        pattern: ">",
        is_regex: false,
        line_number: 44,
        alias_name: nil,
      ),
      GT::TokenDefinition.new(
        name: "LPAREN",
        pattern: "(",
        is_regex: false,
        line_number: 47,
        alias_name: nil,
      ),
      GT::TokenDefinition.new(
        name: "RPAREN",
        pattern: ")",
        is_regex: false,
        line_number: 48,
        alias_name: nil,
      ),
      GT::TokenDefinition.new(
        name: "COMMA",
        pattern: ",",
        is_regex: false,
        line_number: 49,
        alias_name: nil,
      ),
      GT::TokenDefinition.new(
        name: "COLON",
        pattern: ":",
        is_regex: false,
        line_number: 50,
        alias_name: nil,
      ),
    ],
  keywords: ["if", "else", "elsif", "end", "while", "for", "do", "def", "return", "class", "module", "require", "puts", "true", "false", "nil", "and", "or", "not", "then", "unless", "until", "yield", "begin", "rescue", "ensure"],
  mode: nil,
  escape_mode: nil,
  skip_definitions: [],
  reserved_keywords: [],
  error_definitions: [],
  groups: {},
  layout_keywords: [],
  context_keywords: [],
  soft_keywords: [],
)
