# frozen_string_literal: true
# AUTO-GENERATED FILE — DO NOT EDIT
# Source: algol.tokens
# Regenerate with: grammar-tools compile-tokens algol.tokens
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
    # Value tokens — ordered so REAL_LIT precedes INTEGER_LIT.
    # "3.14" must match REAL_LIT, not INTEGER_LIT + then fail on ".14".
    GT::TokenDefinition.new(
      name: "REAL_LIT",
      pattern: "[0-9]+\\.[0-9]*([eE][+-]?[0-9]+)?|[0-9]+[eE][+-]?[0-9]+",
      is_regex: true,
      line_number: 37,
      alias_name: nil,
    ),
    GT::TokenDefinition.new(
      name: "INTEGER_LIT",
      pattern: "[0-9]+",
      is_regex: true,
      line_number: 40,
      alias_name: nil,
    ),
    GT::TokenDefinition.new(
      name: "STRING_LIT",
      pattern: "'[^']*'",
      is_regex: true,
      line_number: 44,
      alias_name: nil,
    ),
    GT::TokenDefinition.new(
      name: "NAME",
      pattern: "[a-zA-Z][a-zA-Z0-9]*",
      is_regex: true,
      line_number: 49,
      alias_name: nil,
    ),
    # Multi-character operators — must precede their single-char prefixes.
    GT::TokenDefinition.new(
      name: "ASSIGN",
      pattern: ":=",
      is_regex: false,
      line_number: 57,
      alias_name: nil,
    ),
    GT::TokenDefinition.new(
      name: "POWER",
      pattern: "**",
      is_regex: false,
      line_number: 62,
      alias_name: nil,
    ),
    GT::TokenDefinition.new(
      name: "LEQ",
      pattern: "<=",
      is_regex: false,
      line_number: 65,
      alias_name: nil,
    ),
    GT::TokenDefinition.new(
      name: "GEQ",
      pattern: ">=",
      is_regex: false,
      line_number: 66,
      alias_name: nil,
    ),
    GT::TokenDefinition.new(
      name: "NEQ",
      pattern: "!=",
      is_regex: false,
      line_number: 67,
      alias_name: nil,
    ),
    # Single-character operators.
    GT::TokenDefinition.new(
      name: "PLUS",
      pattern: "+",
      is_regex: false,
      line_number: 73,
      alias_name: nil,
    ),
    GT::TokenDefinition.new(
      name: "MINUS",
      pattern: "-",
      is_regex: false,
      line_number: 74,
      alias_name: nil,
    ),
    GT::TokenDefinition.new(
      name: "STAR",
      pattern: "*",
      is_regex: false,
      line_number: 75,
      alias_name: nil,
    ),
    GT::TokenDefinition.new(
      name: "SLASH",
      pattern: "/",
      is_regex: false,
      line_number: 76,
      alias_name: nil,
    ),
    GT::TokenDefinition.new(
      name: "CARET",
      pattern: "^",
      is_regex: false,
      line_number: 81,
      alias_name: nil,
    ),
    GT::TokenDefinition.new(
      name: "EQ",
      pattern: "=",
      is_regex: false,
      line_number: 84,
      alias_name: nil,
    ),
    GT::TokenDefinition.new(
      name: "LT",
      pattern: "<",
      is_regex: false,
      line_number: 85,
      alias_name: nil,
    ),
    GT::TokenDefinition.new(
      name: "GT",
      pattern: ">",
      is_regex: false,
      line_number: 86,
      alias_name: nil,
    ),
    # Delimiters.
    GT::TokenDefinition.new(
      name: "LPAREN",
      pattern: "(",
      is_regex: false,
      line_number: 92,
      alias_name: nil,
    ),
    GT::TokenDefinition.new(
      name: "RPAREN",
      pattern: ")",
      is_regex: false,
      line_number: 93,
      alias_name: nil,
    ),
    GT::TokenDefinition.new(
      name: "LBRACKET",
      pattern: "[",
      is_regex: false,
      line_number: 94,
      alias_name: nil,
    ),
    GT::TokenDefinition.new(
      name: "RBRACKET",
      pattern: "]",
      is_regex: false,
      line_number: 95,
      alias_name: nil,
    ),
    GT::TokenDefinition.new(
      name: "SEMICOLON",
      pattern: ";",
      is_regex: false,
      line_number: 96,
      alias_name: nil,
    ),
    GT::TokenDefinition.new(
      name: "COMMA",
      pattern: ",",
      is_regex: false,
      line_number: 97,
      alias_name: nil,
    ),
    GT::TokenDefinition.new(
      name: "COLON",
      pattern: ":",
      is_regex: false,
      line_number: 102,
      alias_name: nil,
    ),
  ],
  keywords: [
    "begin", "end", "if", "then", "else", "for", "do", "step",
    "until", "while", "goto", "switch", "procedure", "own", "array",
    "label", "value", "integer", "real", "boolean", "string",
    "true", "false", "not", "and", "or", "impl", "eqv", "div", "mod",
    "comment",
  ],
  mode: nil,
  escape_mode: nil,
  skip_definitions: [
    GT::TokenDefinition.new(
      name: "COMMENT",
      pattern: "comment[^;]*;",
      is_regex: true,
      line_number: 175,
      alias_name: nil,
    ),
    GT::TokenDefinition.new(
      name: "WHITESPACE",
      pattern: "[ \\t\\r\\n]+",
      is_regex: true,
      line_number: 169,
      alias_name: nil,
    ),
  ],
  reserved_keywords: [],
  error_definitions: [],
  groups: {},
)
