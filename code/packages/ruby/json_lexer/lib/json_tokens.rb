# frozen_string_literal: true
# AUTO-GENERATED FILE - DO NOT EDIT
require "coding_adventures_grammar_tools"

module CodingAdventures
  module JsonTokens
    def self.grammar
      @grammar ||= CodingAdventures::GrammarTools::TokenGrammar.new(
        definitions: [
          CodingAdventures::GrammarTools::TokenDefinition.new(name: "STRING", pattern: "\"([^\"\\\\]|\\\\[\"\\\\\\x2fbfnrt]|\\\\u[0-9a-fA-F]{4})*\"", is_regex: true, line_number: 25, alias_name: nil),
          CodingAdventures::GrammarTools::TokenDefinition.new(name: "NUMBER", pattern: "-?(0|[1-9][0-9]*)(\\.[0-9]+)?([eE][+-]?[0-9]+)?", is_regex: true, line_number: 31, alias_name: nil),
          CodingAdventures::GrammarTools::TokenDefinition.new(name: "TRUE", pattern: "true", is_regex: false, line_number: 35, alias_name: nil),
          CodingAdventures::GrammarTools::TokenDefinition.new(name: "FALSE", pattern: "false", is_regex: false, line_number: 36, alias_name: nil),
          CodingAdventures::GrammarTools::TokenDefinition.new(name: "NULL", pattern: "null", is_regex: false, line_number: 37, alias_name: nil),
          CodingAdventures::GrammarTools::TokenDefinition.new(name: "LBRACE", pattern: "{", is_regex: false, line_number: 43, alias_name: nil),
          CodingAdventures::GrammarTools::TokenDefinition.new(name: "RBRACE", pattern: "}", is_regex: false, line_number: 44, alias_name: nil),
          CodingAdventures::GrammarTools::TokenDefinition.new(name: "LBRACKET", pattern: "[", is_regex: false, line_number: 45, alias_name: nil),
          CodingAdventures::GrammarTools::TokenDefinition.new(name: "RBRACKET", pattern: "]", is_regex: false, line_number: 46, alias_name: nil),
          CodingAdventures::GrammarTools::TokenDefinition.new(name: "COLON", pattern: ":", is_regex: false, line_number: 47, alias_name: nil),
          CodingAdventures::GrammarTools::TokenDefinition.new(name: "COMMA", pattern: ",", is_regex: false, line_number: 48, alias_name: nil),
        ],
        keywords: [],
        mode: nil,
        skip_definitions: [
          CodingAdventures::GrammarTools::TokenDefinition.new(name: "WHITESPACE", pattern: "[ \\t\\r\\n]+", is_regex: true, line_number: 59, alias_name: nil),
        ],
        error_definitions: [
        ],
        reserved_keywords: [],
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
