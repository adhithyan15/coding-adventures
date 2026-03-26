# frozen_string_literal: true
# AUTO-GENERATED FILE - DO NOT EDIT
require "coding_adventures_grammar_tools"

module CodingAdventures
  module JsonGrammar
    def self.grammar
      @grammar ||= CodingAdventures::GrammarTools::ParserGrammar.new(
        version: 1,
        rules: [
          CodingAdventures::GrammarTools::GrammarRule.new(
            name: "value",
            line_number: 28,
            body: CodingAdventures::GrammarTools::Alternation.new(choices: [CodingAdventures::GrammarTools::RuleReference.new(name: "object", is_token: false), CodingAdventures::GrammarTools::RuleReference.new(name: "array", is_token: false), CodingAdventures::GrammarTools::RuleReference.new(name: "STRING", is_token: true), CodingAdventures::GrammarTools::RuleReference.new(name: "NUMBER", is_token: true), CodingAdventures::GrammarTools::RuleReference.new(name: "TRUE", is_token: true), CodingAdventures::GrammarTools::RuleReference.new(name: "FALSE", is_token: true), CodingAdventures::GrammarTools::RuleReference.new(name: "NULL", is_token: true)])
          ),
          CodingAdventures::GrammarTools::GrammarRule.new(
            name: "object",
            line_number: 34,
            body: CodingAdventures::GrammarTools::Sequence.new(elements: [CodingAdventures::GrammarTools::RuleReference.new(name: "LBRACE", is_token: true), CodingAdventures::GrammarTools::OptionalElement.new(element: CodingAdventures::GrammarTools::Sequence.new(elements: [CodingAdventures::GrammarTools::RuleReference.new(name: "pair", is_token: false), CodingAdventures::GrammarTools::Repetition.new(element: CodingAdventures::GrammarTools::Sequence.new(elements: [CodingAdventures::GrammarTools::RuleReference.new(name: "COMMA", is_token: true), CodingAdventures::GrammarTools::RuleReference.new(name: "pair", is_token: false)]))])), CodingAdventures::GrammarTools::RuleReference.new(name: "RBRACE", is_token: true)])
          ),
          CodingAdventures::GrammarTools::GrammarRule.new(
            name: "pair",
            line_number: 38,
            body: CodingAdventures::GrammarTools::Sequence.new(elements: [CodingAdventures::GrammarTools::RuleReference.new(name: "STRING", is_token: true), CodingAdventures::GrammarTools::RuleReference.new(name: "COLON", is_token: true), CodingAdventures::GrammarTools::RuleReference.new(name: "value", is_token: false)])
          ),
          CodingAdventures::GrammarTools::GrammarRule.new(
            name: "array",
            line_number: 42,
            body: CodingAdventures::GrammarTools::Sequence.new(elements: [CodingAdventures::GrammarTools::RuleReference.new(name: "LBRACKET", is_token: true), CodingAdventures::GrammarTools::OptionalElement.new(element: CodingAdventures::GrammarTools::Sequence.new(elements: [CodingAdventures::GrammarTools::RuleReference.new(name: "value", is_token: false), CodingAdventures::GrammarTools::Repetition.new(element: CodingAdventures::GrammarTools::Sequence.new(elements: [CodingAdventures::GrammarTools::RuleReference.new(name: "COMMA", is_token: true), CodingAdventures::GrammarTools::RuleReference.new(name: "value", is_token: false)]))])), CodingAdventures::GrammarTools::RuleReference.new(name: "RBRACKET", is_token: true)])
          ),
        ]
      )
    end
  end
end
