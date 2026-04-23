# frozen_string_literal: true
# AUTO-GENERATED FILE — DO NOT EDIT
# Source: sql.tokens
# Regenerate with: grammar-tools compile-tokens sql.tokens
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
        name: "NAME",
        pattern: "[a-zA-Z_][a-zA-Z0-9_]*",
        is_regex: true,
        line_number: 17,
        alias_name: nil,
      ),
      GT::TokenDefinition.new(
        name: "NUMBER",
        pattern: "[0-9]+\\.?[0-9]*",
        is_regex: true,
        line_number: 18,
        alias_name: nil,
      ),
      GT::TokenDefinition.new(
        name: "STRING_SQ",
        pattern: "'([^'\\\\]|\\\\.)*'",
        is_regex: true,
        line_number: 19,
        alias_name: "STRING",
      ),
      GT::TokenDefinition.new(
        name: "QUOTED_ID",
        pattern: "`[^`]+`",
        is_regex: true,
        line_number: 20,
        alias_name: "NAME",
      ),
      GT::TokenDefinition.new(
        name: "LESS_EQUALS",
        pattern: "<=",
        is_regex: false,
        line_number: 22,
        alias_name: nil,
      ),
      GT::TokenDefinition.new(
        name: "GREATER_EQUALS",
        pattern: ">=",
        is_regex: false,
        line_number: 23,
        alias_name: nil,
      ),
      GT::TokenDefinition.new(
        name: "NOT_EQUALS",
        pattern: "!=",
        is_regex: false,
        line_number: 24,
        alias_name: nil,
      ),
      GT::TokenDefinition.new(
        name: "NEQ_ANSI",
        pattern: "<>",
        is_regex: false,
        line_number: 25,
        alias_name: "NOT_EQUALS",
      ),
      GT::TokenDefinition.new(
        name: "EQUALS",
        pattern: "=",
        is_regex: false,
        line_number: 27,
        alias_name: nil,
      ),
      GT::TokenDefinition.new(
        name: "LESS_THAN",
        pattern: "<",
        is_regex: false,
        line_number: 28,
        alias_name: nil,
      ),
      GT::TokenDefinition.new(
        name: "GREATER_THAN",
        pattern: ">",
        is_regex: false,
        line_number: 29,
        alias_name: nil,
      ),
      GT::TokenDefinition.new(
        name: "PLUS",
        pattern: "+",
        is_regex: false,
        line_number: 30,
        alias_name: nil,
      ),
      GT::TokenDefinition.new(
        name: "MINUS",
        pattern: "-",
        is_regex: false,
        line_number: 31,
        alias_name: nil,
      ),
      GT::TokenDefinition.new(
        name: "STAR",
        pattern: "*",
        is_regex: false,
        line_number: 32,
        alias_name: nil,
      ),
      GT::TokenDefinition.new(
        name: "SLASH",
        pattern: "/",
        is_regex: false,
        line_number: 33,
        alias_name: nil,
      ),
      GT::TokenDefinition.new(
        name: "PERCENT",
        pattern: "%",
        is_regex: false,
        line_number: 34,
        alias_name: nil,
      ),
      GT::TokenDefinition.new(
        name: "LPAREN",
        pattern: "(",
        is_regex: false,
        line_number: 36,
        alias_name: nil,
      ),
      GT::TokenDefinition.new(
        name: "RPAREN",
        pattern: ")",
        is_regex: false,
        line_number: 37,
        alias_name: nil,
      ),
      GT::TokenDefinition.new(
        name: "COMMA",
        pattern: ",",
        is_regex: false,
        line_number: 38,
        alias_name: nil,
      ),
      GT::TokenDefinition.new(
        name: "SEMICOLON",
        pattern: ";",
        is_regex: false,
        line_number: 39,
        alias_name: nil,
      ),
      GT::TokenDefinition.new(
        name: "DOT",
        pattern: ".",
        is_regex: false,
        line_number: 40,
        alias_name: nil,
      ),
    ],
  keywords: ["SELECT", "FROM", "WHERE", "GROUP", "BY", "HAVING", "ORDER", "LIMIT", "OFFSET", "INSERT", "INTO", "VALUES", "UPDATE", "SET", "DELETE", "CREATE", "DROP", "TABLE", "IF", "EXISTS", "NOT", "AND", "OR", "NULL", "IS", "IN", "BETWEEN", "LIKE", "AS", "DISTINCT", "ALL", "UNION", "INTERSECT", "EXCEPT", "JOIN", "INNER", "LEFT", "RIGHT", "OUTER", "CROSS", "FULL", "ON", "ASC", "DESC", "TRUE", "FALSE", "CASE", "WHEN", "THEN", "ELSE", "END", "PRIMARY", "KEY", "UNIQUE", "DEFAULT"],
  mode: nil,
  escape_mode: nil,
  skip_definitions: [
      GT::TokenDefinition.new(
        name: "WHITESPACE",
        pattern: "[ \\t\\r\\n]+",
        is_regex: true,
        line_number: 100,
        alias_name: nil,
      ),
      GT::TokenDefinition.new(
        name: "LINE_COMMENT",
        pattern: "--[^\\n]*",
        is_regex: true,
        line_number: 101,
        alias_name: nil,
      ),
      GT::TokenDefinition.new(
        name: "BLOCK_COMMENT",
        pattern: "\\x2f\\*([^*]|\\*[^\\x2f])*\\*\\x2f",
        is_regex: true,
        line_number: 102,
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
