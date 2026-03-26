# frozen_string_literal: true
# AUTO-GENERATED FILE - DO NOT EDIT
require "coding_adventures_grammar_tools"

module CodingAdventures
  module TomlTokens
    def self.grammar
      @grammar ||= CodingAdventures::GrammarTools::TokenGrammar.new(
        definitions: [
          CodingAdventures::GrammarTools::TokenDefinition.new(name: "ML_BASIC_STRING", pattern: "\"\"\"([^\\\\]|\\\\(.|\\n)|\\n)*?\"\"\"", is_regex: true, line_number: 60, alias_name: nil),
          CodingAdventures::GrammarTools::TokenDefinition.new(name: "ML_LITERAL_STRING", pattern: "'''[\\s\\S]*?'''", is_regex: true, line_number: 61, alias_name: nil),
          CodingAdventures::GrammarTools::TokenDefinition.new(name: "BASIC_STRING", pattern: "\"([^\"\\\\\\n]|\\\\.)*\"", is_regex: true, line_number: 70, alias_name: nil),
          CodingAdventures::GrammarTools::TokenDefinition.new(name: "LITERAL_STRING", pattern: "'[^'\\n]*'", is_regex: true, line_number: 71, alias_name: nil),
          CodingAdventures::GrammarTools::TokenDefinition.new(name: "OFFSET_DATETIME", pattern: "\\d{4}-\\d{2}-\\d{2}[T ]\\d{2}:\\d{2}:\\d{2}(\\.\\d+)?(Z|[+-]\\d{2}:\\d{2})", is_regex: true, line_number: 91, alias_name: nil),
          CodingAdventures::GrammarTools::TokenDefinition.new(name: "LOCAL_DATETIME", pattern: "\\d{4}-\\d{2}-\\d{2}[T ]\\d{2}:\\d{2}:\\d{2}(\\.\\d+)?", is_regex: true, line_number: 92, alias_name: nil),
          CodingAdventures::GrammarTools::TokenDefinition.new(name: "LOCAL_DATE", pattern: "\\d{4}-\\d{2}-\\d{2}", is_regex: true, line_number: 93, alias_name: nil),
          CodingAdventures::GrammarTools::TokenDefinition.new(name: "LOCAL_TIME", pattern: "\\d{2}:\\d{2}:\\d{2}(\\.\\d+)?", is_regex: true, line_number: 94, alias_name: nil),
          CodingAdventures::GrammarTools::TokenDefinition.new(name: "FLOAT_SPECIAL", pattern: "[+-]?(inf|nan)", is_regex: true, line_number: 109, alias_name: "FLOAT"),
          CodingAdventures::GrammarTools::TokenDefinition.new(name: "FLOAT_EXP", pattern: "[+-]?([0-9](_?[0-9])*)(\\.[0-9](_?[0-9])*)?[eE][+-]?[0-9](_?[0-9])*", is_regex: true, line_number: 110, alias_name: "FLOAT"),
          CodingAdventures::GrammarTools::TokenDefinition.new(name: "FLOAT_DEC", pattern: "[+-]?([0-9](_?[0-9])*)\\.([0-9](_?[0-9])*)", is_regex: true, line_number: 111, alias_name: "FLOAT"),
          CodingAdventures::GrammarTools::TokenDefinition.new(name: "HEX_INTEGER", pattern: "0x[0-9a-fA-F](_?[0-9a-fA-F])*", is_regex: true, line_number: 123, alias_name: "INTEGER"),
          CodingAdventures::GrammarTools::TokenDefinition.new(name: "OCT_INTEGER", pattern: "0o[0-7](_?[0-7])*", is_regex: true, line_number: 124, alias_name: "INTEGER"),
          CodingAdventures::GrammarTools::TokenDefinition.new(name: "BIN_INTEGER", pattern: "0b[01](_?[01])*", is_regex: true, line_number: 125, alias_name: "INTEGER"),
          CodingAdventures::GrammarTools::TokenDefinition.new(name: "INTEGER", pattern: "[+-]?[0-9](_?[0-9])*", is_regex: true, line_number: 126, alias_name: nil),
          CodingAdventures::GrammarTools::TokenDefinition.new(name: "TRUE", pattern: "true", is_regex: false, line_number: 137, alias_name: nil),
          CodingAdventures::GrammarTools::TokenDefinition.new(name: "FALSE", pattern: "false", is_regex: false, line_number: 138, alias_name: nil),
          CodingAdventures::GrammarTools::TokenDefinition.new(name: "BARE_KEY", pattern: "[A-Za-z0-9_-]+", is_regex: true, line_number: 152, alias_name: nil),
          CodingAdventures::GrammarTools::TokenDefinition.new(name: "EQUALS", pattern: "=", is_regex: false, line_number: 162, alias_name: nil),
          CodingAdventures::GrammarTools::TokenDefinition.new(name: "DOT", pattern: ".", is_regex: false, line_number: 163, alias_name: nil),
          CodingAdventures::GrammarTools::TokenDefinition.new(name: "COMMA", pattern: ",", is_regex: false, line_number: 164, alias_name: nil),
          CodingAdventures::GrammarTools::TokenDefinition.new(name: "LBRACKET", pattern: "[", is_regex: false, line_number: 165, alias_name: nil),
          CodingAdventures::GrammarTools::TokenDefinition.new(name: "RBRACKET", pattern: "]", is_regex: false, line_number: 166, alias_name: nil),
          CodingAdventures::GrammarTools::TokenDefinition.new(name: "LBRACE", pattern: "{", is_regex: false, line_number: 167, alias_name: nil),
          CodingAdventures::GrammarTools::TokenDefinition.new(name: "RBRACE", pattern: "}", is_regex: false, line_number: 168, alias_name: nil),
        ],
        keywords: [],
        mode: nil,
        skip_definitions: [
          CodingAdventures::GrammarTools::TokenDefinition.new(name: "COMMENT", pattern: "#[^\\n]*", is_regex: true, line_number: 28, alias_name: nil),
          CodingAdventures::GrammarTools::TokenDefinition.new(name: "WHITESPACE", pattern: "[ \\t]+", is_regex: true, line_number: 29, alias_name: nil),
        ],
        error_definitions: [
        ],
        reserved_keywords: [],
        escape_mode: "none",
        groups: {
        },
        case_sensitive: true,
        version: 1,
        case_insensitive: false
      )
    end
  end
end
