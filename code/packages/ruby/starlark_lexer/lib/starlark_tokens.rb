# frozen_string_literal: true
# AUTO-GENERATED FILE - DO NOT EDIT
require "coding_adventures_grammar_tools"

module CodingAdventures
  module StarlarkTokens
    def self.grammar
      @grammar ||= CodingAdventures::GrammarTools::TokenGrammar.new(
        definitions: [
          CodingAdventures::GrammarTools::TokenDefinition.new(name: "STRING_RAW_TRIPLE_DQ", pattern: "[rR][bB]?\"\"\"([^\"\\\\]|\\\\.|\\n)*\"\"\"|[bB][rR]\"\"\"([^\"\\\\]|\\\\.|\\n)*\"\"\"", is_regex: true, line_number: 70, alias_name: "STRING"),
          CodingAdventures::GrammarTools::TokenDefinition.new(name: "STRING_RAW_TRIPLE_SQ", pattern: "[rR][bB]?'''([^'\\\\]|\\\\.|\\n)*'''|[bB][rR]'''([^'\\\\]|\\\\.|\\n)*'''", is_regex: true, line_number: 71, alias_name: "STRING"),
          CodingAdventures::GrammarTools::TokenDefinition.new(name: "STRING_TRIPLE_DQ", pattern: "[bB]?\"\"\"([^\"\\\\]|\\\\.|\\n)*\"\"\"", is_regex: true, line_number: 72, alias_name: "STRING"),
          CodingAdventures::GrammarTools::TokenDefinition.new(name: "STRING_TRIPLE_SQ", pattern: "[bB]?'''([^'\\\\]|\\\\.|\\n)*'''", is_regex: true, line_number: 73, alias_name: "STRING"),
          CodingAdventures::GrammarTools::TokenDefinition.new(name: "STRING_RAW_DQ", pattern: "[rR][bB]?\"([^\"\\\\]|\\\\.)*\"|[bB][rR]\"([^\"\\\\]|\\\\.)*\"", is_regex: true, line_number: 76, alias_name: "STRING"),
          CodingAdventures::GrammarTools::TokenDefinition.new(name: "STRING_RAW_SQ", pattern: "[rR][bB]?'([^'\\\\]|\\\\.)*'|[bB][rR]'([^'\\\\]|\\\\.)*'", is_regex: true, line_number: 77, alias_name: "STRING"),
          CodingAdventures::GrammarTools::TokenDefinition.new(name: "STRING_DQ", pattern: "[bB]?\"([^\"\\\\]|\\\\.)*\"", is_regex: true, line_number: 78, alias_name: "STRING"),
          CodingAdventures::GrammarTools::TokenDefinition.new(name: "STRING_SQ", pattern: "[bB]?'([^'\\\\]|\\\\.)*'", is_regex: true, line_number: 79, alias_name: "STRING"),
          CodingAdventures::GrammarTools::TokenDefinition.new(name: "FLOAT", pattern: "[0-9]+\\.[0-9]*([eE][+-]?[0-9]+)?|\\.[0-9]+([eE][+-]?[0-9]+)?|[0-9]+[eE][+-]?[0-9]+", is_regex: true, line_number: 92, alias_name: nil),
          CodingAdventures::GrammarTools::TokenDefinition.new(name: "INT_HEX", pattern: "0[xX][0-9a-fA-F]+", is_regex: true, line_number: 95, alias_name: "INT"),
          CodingAdventures::GrammarTools::TokenDefinition.new(name: "INT_OCT", pattern: "0[oO][0-7]+", is_regex: true, line_number: 96, alias_name: "INT"),
          CodingAdventures::GrammarTools::TokenDefinition.new(name: "INT", pattern: "[0-9]+", is_regex: true, line_number: 97, alias_name: nil),
          CodingAdventures::GrammarTools::TokenDefinition.new(name: "NAME", pattern: "[a-zA-Z_][a-zA-Z0-9_]*", is_regex: true, line_number: 107, alias_name: nil),
          CodingAdventures::GrammarTools::TokenDefinition.new(name: "DOUBLE_STAR_EQUALS", pattern: "**=", is_regex: false, line_number: 116, alias_name: nil),
          CodingAdventures::GrammarTools::TokenDefinition.new(name: "LEFT_SHIFT_EQUALS", pattern: "<<=", is_regex: false, line_number: 117, alias_name: nil),
          CodingAdventures::GrammarTools::TokenDefinition.new(name: "RIGHT_SHIFT_EQUALS", pattern: ">>=", is_regex: false, line_number: 118, alias_name: nil),
          CodingAdventures::GrammarTools::TokenDefinition.new(name: "FLOOR_DIV_EQUALS", pattern: "//=", is_regex: false, line_number: 119, alias_name: nil),
          CodingAdventures::GrammarTools::TokenDefinition.new(name: "DOUBLE_STAR", pattern: "**", is_regex: false, line_number: 128, alias_name: nil),
          CodingAdventures::GrammarTools::TokenDefinition.new(name: "FLOOR_DIV", pattern: "//", is_regex: false, line_number: 129, alias_name: nil),
          CodingAdventures::GrammarTools::TokenDefinition.new(name: "LEFT_SHIFT", pattern: "<<", is_regex: false, line_number: 130, alias_name: nil),
          CodingAdventures::GrammarTools::TokenDefinition.new(name: "RIGHT_SHIFT", pattern: ">>", is_regex: false, line_number: 131, alias_name: nil),
          CodingAdventures::GrammarTools::TokenDefinition.new(name: "EQUALS_EQUALS", pattern: "==", is_regex: false, line_number: 132, alias_name: nil),
          CodingAdventures::GrammarTools::TokenDefinition.new(name: "NOT_EQUALS", pattern: "!=", is_regex: false, line_number: 133, alias_name: nil),
          CodingAdventures::GrammarTools::TokenDefinition.new(name: "LESS_EQUALS", pattern: "<=", is_regex: false, line_number: 134, alias_name: nil),
          CodingAdventures::GrammarTools::TokenDefinition.new(name: "GREATER_EQUALS", pattern: ">=", is_regex: false, line_number: 135, alias_name: nil),
          CodingAdventures::GrammarTools::TokenDefinition.new(name: "PLUS_EQUALS", pattern: "+=", is_regex: false, line_number: 136, alias_name: nil),
          CodingAdventures::GrammarTools::TokenDefinition.new(name: "MINUS_EQUALS", pattern: "-=", is_regex: false, line_number: 137, alias_name: nil),
          CodingAdventures::GrammarTools::TokenDefinition.new(name: "STAR_EQUALS", pattern: "*=", is_regex: false, line_number: 138, alias_name: nil),
          CodingAdventures::GrammarTools::TokenDefinition.new(name: "SLASH_EQUALS", pattern: "/=", is_regex: false, line_number: 139, alias_name: nil),
          CodingAdventures::GrammarTools::TokenDefinition.new(name: "PERCENT_EQUALS", pattern: "%=", is_regex: false, line_number: 140, alias_name: nil),
          CodingAdventures::GrammarTools::TokenDefinition.new(name: "AMP_EQUALS", pattern: "&=", is_regex: false, line_number: 141, alias_name: nil),
          CodingAdventures::GrammarTools::TokenDefinition.new(name: "PIPE_EQUALS", pattern: "|=", is_regex: false, line_number: 142, alias_name: nil),
          CodingAdventures::GrammarTools::TokenDefinition.new(name: "CARET_EQUALS", pattern: "^=", is_regex: false, line_number: 143, alias_name: nil),
          CodingAdventures::GrammarTools::TokenDefinition.new(name: "PLUS", pattern: "+", is_regex: false, line_number: 149, alias_name: nil),
          CodingAdventures::GrammarTools::TokenDefinition.new(name: "MINUS", pattern: "-", is_regex: false, line_number: 150, alias_name: nil),
          CodingAdventures::GrammarTools::TokenDefinition.new(name: "STAR", pattern: "*", is_regex: false, line_number: 151, alias_name: nil),
          CodingAdventures::GrammarTools::TokenDefinition.new(name: "SLASH", pattern: "/", is_regex: false, line_number: 152, alias_name: nil),
          CodingAdventures::GrammarTools::TokenDefinition.new(name: "PERCENT", pattern: "%", is_regex: false, line_number: 153, alias_name: nil),
          CodingAdventures::GrammarTools::TokenDefinition.new(name: "EQUALS", pattern: "=", is_regex: false, line_number: 154, alias_name: nil),
          CodingAdventures::GrammarTools::TokenDefinition.new(name: "LESS_THAN", pattern: "<", is_regex: false, line_number: 155, alias_name: nil),
          CodingAdventures::GrammarTools::TokenDefinition.new(name: "GREATER_THAN", pattern: ">", is_regex: false, line_number: 156, alias_name: nil),
          CodingAdventures::GrammarTools::TokenDefinition.new(name: "AMP", pattern: "&", is_regex: false, line_number: 157, alias_name: nil),
          CodingAdventures::GrammarTools::TokenDefinition.new(name: "PIPE", pattern: "|", is_regex: false, line_number: 158, alias_name: nil),
          CodingAdventures::GrammarTools::TokenDefinition.new(name: "CARET", pattern: "^", is_regex: false, line_number: 159, alias_name: nil),
          CodingAdventures::GrammarTools::TokenDefinition.new(name: "TILDE", pattern: "~", is_regex: false, line_number: 160, alias_name: nil),
          CodingAdventures::GrammarTools::TokenDefinition.new(name: "LPAREN", pattern: "(", is_regex: false, line_number: 166, alias_name: nil),
          CodingAdventures::GrammarTools::TokenDefinition.new(name: "RPAREN", pattern: ")", is_regex: false, line_number: 167, alias_name: nil),
          CodingAdventures::GrammarTools::TokenDefinition.new(name: "LBRACKET", pattern: "[", is_regex: false, line_number: 168, alias_name: nil),
          CodingAdventures::GrammarTools::TokenDefinition.new(name: "RBRACKET", pattern: "]", is_regex: false, line_number: 169, alias_name: nil),
          CodingAdventures::GrammarTools::TokenDefinition.new(name: "LBRACE", pattern: "{", is_regex: false, line_number: 170, alias_name: nil),
          CodingAdventures::GrammarTools::TokenDefinition.new(name: "RBRACE", pattern: "}", is_regex: false, line_number: 171, alias_name: nil),
          CodingAdventures::GrammarTools::TokenDefinition.new(name: "COMMA", pattern: ",", is_regex: false, line_number: 172, alias_name: nil),
          CodingAdventures::GrammarTools::TokenDefinition.new(name: "COLON", pattern: ":", is_regex: false, line_number: 173, alias_name: nil),
          CodingAdventures::GrammarTools::TokenDefinition.new(name: "SEMICOLON", pattern: ";", is_regex: false, line_number: 174, alias_name: nil),
          CodingAdventures::GrammarTools::TokenDefinition.new(name: "DOT", pattern: ".", is_regex: false, line_number: 175, alias_name: nil),
        ],
        keywords: ["and", "break", "continue", "def", "elif", "else", "for", "if", "in", "lambda", "load", "not", "or", "pass", "return", "True", "False", "None"],
        mode: "indentation",
        skip_definitions: [
          CodingAdventures::GrammarTools::TokenDefinition.new(name: "COMMENT", pattern: "#[^\\n]*", is_regex: true, line_number: 51, alias_name: nil),
          CodingAdventures::GrammarTools::TokenDefinition.new(name: "WHITESPACE", pattern: "[ \\t]+", is_regex: true, line_number: 52, alias_name: nil),
        ],
        error_definitions: [
        ],
        reserved_keywords: ["as", "assert", "async", "await", "class", "del", "except", "finally", "from", "global", "import", "is", "nonlocal", "raise", "try", "while", "with", "yield"],
        escape_mode: nil,
        groups: {
        },
        case_sensitive: true,
        version: 1,
        case_insensitive: false
      )
    end
  end
end
