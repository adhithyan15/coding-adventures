# frozen_string_literal: true
# AUTO-GENERATED FILE — DO NOT EDIT
# Source: mosaic.tokens
# Regenerate with: grammar-tools compile-tokens mosaic.tokens
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
        pattern: "\"([^\"\\\\\\n]|\\\\.)*\"",
        is_regex: true,
        line_number: 23,
        alias_name: nil,
      ),
      GT::TokenDefinition.new(
        name: "DIMENSION",
        pattern: "-?[0-9]*\\.?[0-9]+[a-zA-Z%]+",
        is_regex: true,
        line_number: 31,
        alias_name: nil,
      ),
      GT::TokenDefinition.new(
        name: "NUMBER",
        pattern: "-?[0-9]*\\.?[0-9]+",
        is_regex: true,
        line_number: 32,
        alias_name: nil,
      ),
      GT::TokenDefinition.new(
        name: "COLOR_HEX",
        pattern: "#[0-9a-fA-F]{3,8}",
        is_regex: true,
        line_number: 39,
        alias_name: nil,
      ),
      GT::TokenDefinition.new(
        name: "NAME",
        pattern: "[a-zA-Z_][a-zA-Z0-9_-]*",
        is_regex: true,
        line_number: 70,
        alias_name: nil,
      ),
      GT::TokenDefinition.new(
        name: "LBRACE",
        pattern: "{",
        is_regex: false,
        line_number: 76,
        alias_name: nil,
      ),
      GT::TokenDefinition.new(
        name: "RBRACE",
        pattern: "}",
        is_regex: false,
        line_number: 77,
        alias_name: nil,
      ),
      GT::TokenDefinition.new(
        name: "LANGLE",
        pattern: "<",
        is_regex: false,
        line_number: 78,
        alias_name: nil,
      ),
      GT::TokenDefinition.new(
        name: "RANGLE",
        pattern: ">",
        is_regex: false,
        line_number: 79,
        alias_name: nil,
      ),
      GT::TokenDefinition.new(
        name: "COLON",
        pattern: ":",
        is_regex: false,
        line_number: 80,
        alias_name: nil,
      ),
      GT::TokenDefinition.new(
        name: "SEMICOLON",
        pattern: ";",
        is_regex: false,
        line_number: 81,
        alias_name: nil,
      ),
      GT::TokenDefinition.new(
        name: "COMMA",
        pattern: ",",
        is_regex: false,
        line_number: 82,
        alias_name: nil,
      ),
      GT::TokenDefinition.new(
        name: "DOT",
        pattern: ".",
        is_regex: false,
        line_number: 83,
        alias_name: nil,
      ),
      GT::TokenDefinition.new(
        name: "EQUALS",
        pattern: "=",
        is_regex: false,
        line_number: 84,
        alias_name: nil,
      ),
      GT::TokenDefinition.new(
        name: "AT",
        pattern: "@",
        is_regex: false,
        line_number: 85,
        alias_name: nil,
      ),
    ],
  keywords: ["component", "slot", "import", "from", "as", "text", "number", "bool", "image", "color", "node", "list", "true", "false", "when", "each"],
  mode: nil,
  escape_mode: "standard",
  skip_definitions: [
      GT::TokenDefinition.new(
        name: "LINE_COMMENT",
        pattern: "\\/\\/[^\\n]*",
        is_regex: true,
        line_number: 15,
        alias_name: nil,
      ),
      GT::TokenDefinition.new(
        name: "BLOCK_COMMENT",
        pattern: "\\/\\*[\\s\\S]*?\\*\\/",
        is_regex: true,
        line_number: 16,
        alias_name: nil,
      ),
      GT::TokenDefinition.new(
        name: "WHITESPACE",
        pattern: "[ \\t\\r\\n]+",
        is_regex: true,
        line_number: 17,
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
