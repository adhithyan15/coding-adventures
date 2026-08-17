# frozen_string_literal: true
# AUTO-GENERATED FILE — DO NOT EDIT
# Source: dartmouth_basic.tokens
# Regenerate with: grammar-tools compile-tokens dartmouth_basic.tokens
#
# This file embeds a TokenGrammar as native Ruby data structures.
# Downstream packages require this file directly instead of reading
# and parsing the .tokens file at runtime.

require "coding_adventures_grammar_tools"

GT = CodingAdventures::GrammarTools unless defined?(GT)

TOKEN_GRAMMAR = GT::TokenGrammar.new(
  version: 1,
  case_insensitive: true,
  case_sensitive: false,
  definitions: [
      GT::TokenDefinition.new(
        name: "LE",
        pattern: "<=",
        is_regex: false,
        line_number: 50,
        alias_name: nil,
      ),
      GT::TokenDefinition.new(
        name: "GE",
        pattern: ">=",
        is_regex: false,
        line_number: 51,
        alias_name: nil,
      ),
      GT::TokenDefinition.new(
        name: "NE",
        pattern: "<>",
        is_regex: false,
        line_number: 52,
        alias_name: nil,
      ),
      GT::TokenDefinition.new(
        name: "NUMBER",
        pattern: "[0-9]*\\.?[0-9]+([Ee][+-]?[0-9]+)?",
        is_regex: true,
        line_number: 85,
        alias_name: nil,
      ),
      GT::TokenDefinition.new(
        name: "LINE_NUM",
        pattern: "[0-9]+",
        is_regex: true,
        line_number: 86,
        alias_name: nil,
      ),
      GT::TokenDefinition.new(
        name: "STRING_BODY",
        pattern: "\"[^\"]*\"",
        is_regex: true,
        line_number: 112,
        alias_name: "STRING",
      ),
      GT::TokenDefinition.new(
        name: "BUILTIN_FN",
        pattern: "(?:sin|cos|tan|atn|exp|log|abs|sqr|int|rnd|sgn)",
        is_regex: true,
        line_number: 168,
        alias_name: nil,
      ),
      GT::TokenDefinition.new(
        name: "USER_FN",
        pattern: "fn[a-z]",
        is_regex: true,
        line_number: 169,
        alias_name: nil,
      ),
      GT::TokenDefinition.new(
        name: "NAME",
        pattern: "[a-z][a-z0-9]*\\$?",
        is_regex: true,
        line_number: 204,
        alias_name: nil,
      ),
      GT::TokenDefinition.new(
        name: "PLUS",
        pattern: "+",
        is_regex: false,
        line_number: 244,
        alias_name: nil,
      ),
      GT::TokenDefinition.new(
        name: "MINUS",
        pattern: "-",
        is_regex: false,
        line_number: 245,
        alias_name: nil,
      ),
      GT::TokenDefinition.new(
        name: "STAR",
        pattern: "*",
        is_regex: false,
        line_number: 246,
        alias_name: nil,
      ),
      GT::TokenDefinition.new(
        name: "SLASH",
        pattern: "/",
        is_regex: false,
        line_number: 247,
        alias_name: nil,
      ),
      GT::TokenDefinition.new(
        name: "CARET",
        pattern: "^",
        is_regex: false,
        line_number: 248,
        alias_name: nil,
      ),
      GT::TokenDefinition.new(
        name: "EQ",
        pattern: "=",
        is_regex: false,
        line_number: 249,
        alias_name: nil,
      ),
      GT::TokenDefinition.new(
        name: "LT",
        pattern: "<",
        is_regex: false,
        line_number: 250,
        alias_name: nil,
      ),
      GT::TokenDefinition.new(
        name: "GT",
        pattern: ">",
        is_regex: false,
        line_number: 251,
        alias_name: nil,
      ),
      GT::TokenDefinition.new(
        name: "LPAREN",
        pattern: "(",
        is_regex: false,
        line_number: 252,
        alias_name: nil,
      ),
      GT::TokenDefinition.new(
        name: "RPAREN",
        pattern: ")",
        is_regex: false,
        line_number: 253,
        alias_name: nil,
      ),
      GT::TokenDefinition.new(
        name: "COMMA",
        pattern: ",",
        is_regex: false,
        line_number: 254,
        alias_name: nil,
      ),
      GT::TokenDefinition.new(
        name: "SEMICOLON",
        pattern: ";",
        is_regex: false,
        line_number: 255,
        alias_name: nil,
      ),
      GT::TokenDefinition.new(
        name: "NEWLINE",
        pattern: "\\r?\\n",
        is_regex: true,
        line_number: 276,
        alias_name: nil,
      ),
    ],
  keywords: ["LET", "PRINT", "INPUT", "IF", "THEN", "GOTO", "GOSUB", "RETURN", "FOR", "TO", "STEP", "NEXT", "END", "STOP", "REM", "READ", "DATA", "RESTORE", "DIM", "DEF"],
  mode: nil,
  escape_mode: nil,
  skip_definitions: [
      GT::TokenDefinition.new(
        name: "WHITESPACE",
        pattern: "[ \\t]+",
        is_regex: true,
        line_number: 288,
        alias_name: nil,
      ),
    ],
  reserved_keywords: [],
  error_definitions: [
      GT::TokenDefinition.new(
        name: "UNKNOWN",
        pattern: ".",
        is_regex: true,
        line_number: 304,
        alias_name: nil,
      ),
    ],
  groups: {},
  layout_keywords: [],
  context_keywords: [],
  soft_keywords: [],
)
