# AUTO-GENERATED FILE - DO NOT EDIT
require "coding_adventures_grammar_tools"

module CodingAdventures
  module PythonTokens
    def self.grammar
      @grammar ||= CodingAdventures::GrammarTools::TokenGrammar.new(
        definitions: [
          CodingAdventures::GrammarTools::TokenDefinition.new(name: "NAME", pattern: "[a-zA-Z_][a-zA-Z0-9_]*", is_regex: true, line_number: 13, alias_name: nil),
          CodingAdventures::GrammarTools::TokenDefinition.new(name: "NUMBER", pattern: "[0-9]+", is_regex: true, line_number: 14, alias_name: nil),
          CodingAdventures::GrammarTools::TokenDefinition.new(name: "STRING", pattern: "\"([^\"\\\\]|\\\\.)*\"", is_regex: true, line_number: 15, alias_name: nil),
          CodingAdventures::GrammarTools::TokenDefinition.new(name: "EQUALS_EQUALS", pattern: "==", is_regex: false, line_number: 18, alias_name: nil),
          CodingAdventures::GrammarTools::TokenDefinition.new(name: "EQUALS", pattern: "=", is_regex: false, line_number: 21, alias_name: nil),
          CodingAdventures::GrammarTools::TokenDefinition.new(name: "PLUS", pattern: "+", is_regex: false, line_number: 22, alias_name: nil),
          CodingAdventures::GrammarTools::TokenDefinition.new(name: "MINUS", pattern: "-", is_regex: false, line_number: 23, alias_name: nil),
          CodingAdventures::GrammarTools::TokenDefinition.new(name: "STAR", pattern: "*", is_regex: false, line_number: 24, alias_name: nil),
          CodingAdventures::GrammarTools::TokenDefinition.new(name: "SLASH", pattern: "/", is_regex: false, line_number: 25, alias_name: nil),
          CodingAdventures::GrammarTools::TokenDefinition.new(name: "LPAREN", pattern: "(", is_regex: false, line_number: 28, alias_name: nil),
          CodingAdventures::GrammarTools::TokenDefinition.new(name: "RPAREN", pattern: ")", is_regex: false, line_number: 29, alias_name: nil),
          CodingAdventures::GrammarTools::TokenDefinition.new(name: "COMMA", pattern: ",", is_regex: false, line_number: 30, alias_name: nil),
          CodingAdventures::GrammarTools::TokenDefinition.new(name: "COLON", pattern: ":", is_regex: false, line_number: 31, alias_name: nil),
        ],
        keywords: ["if", "else", "elif", "while", "for", "def", "return", "class", "import", "from", "as", "True", "False", "None"],
        mode: nil,
        skip_definitions: [
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
