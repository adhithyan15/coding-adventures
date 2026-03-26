# frozen_string_literal: true
# AUTO-GENERATED FILE - DO NOT EDIT
require "coding_adventures_grammar_tools"

module CodingAdventures
  module TypescriptGrammar
    def self.grammar
      @grammar ||= CodingAdventures::GrammarTools::ParserGrammar.new(
        version: 1,
        rules: [
          CodingAdventures::GrammarTools::GrammarRule.new(
            name: "program",
            line_number: 29,
            body: CodingAdventures::GrammarTools::Repetition.new(element: CodingAdventures::GrammarTools::RuleReference.new(name: "statement", is_token: false))
          ),
          CodingAdventures::GrammarTools::GrammarRule.new(
            name: "statement",
            line_number: 30,
            body: CodingAdventures::GrammarTools::Alternation.new(choices: [CodingAdventures::GrammarTools::RuleReference.new(name: "var_declaration", is_token: false), CodingAdventures::GrammarTools::RuleReference.new(name: "assignment", is_token: false), CodingAdventures::GrammarTools::RuleReference.new(name: "expression_stmt", is_token: false)])
          ),
          CodingAdventures::GrammarTools::GrammarRule.new(
            name: "var_declaration",
            line_number: 31,
            body: CodingAdventures::GrammarTools::Sequence.new(elements: [CodingAdventures::GrammarTools::RuleReference.new(name: "KEYWORD", is_token: true), CodingAdventures::GrammarTools::RuleReference.new(name: "NAME", is_token: true), CodingAdventures::GrammarTools::RuleReference.new(name: "EQUALS", is_token: true), CodingAdventures::GrammarTools::RuleReference.new(name: "expression", is_token: false), CodingAdventures::GrammarTools::RuleReference.new(name: "SEMICOLON", is_token: true)])
          ),
          CodingAdventures::GrammarTools::GrammarRule.new(
            name: "assignment",
            line_number: 32,
            body: CodingAdventures::GrammarTools::Sequence.new(elements: [CodingAdventures::GrammarTools::RuleReference.new(name: "NAME", is_token: true), CodingAdventures::GrammarTools::RuleReference.new(name: "EQUALS", is_token: true), CodingAdventures::GrammarTools::RuleReference.new(name: "expression", is_token: false), CodingAdventures::GrammarTools::RuleReference.new(name: "SEMICOLON", is_token: true)])
          ),
          CodingAdventures::GrammarTools::GrammarRule.new(
            name: "expression_stmt",
            line_number: 33,
            body: CodingAdventures::GrammarTools::Sequence.new(elements: [CodingAdventures::GrammarTools::RuleReference.new(name: "expression", is_token: false), CodingAdventures::GrammarTools::RuleReference.new(name: "SEMICOLON", is_token: true)])
          ),
          CodingAdventures::GrammarTools::GrammarRule.new(
            name: "expression",
            line_number: 34,
            body: CodingAdventures::GrammarTools::Sequence.new(elements: [CodingAdventures::GrammarTools::RuleReference.new(name: "term", is_token: false), CodingAdventures::GrammarTools::Repetition.new(element: CodingAdventures::GrammarTools::Sequence.new(elements: [CodingAdventures::GrammarTools::Group.new(element: CodingAdventures::GrammarTools::Alternation.new(choices: [CodingAdventures::GrammarTools::RuleReference.new(name: "PLUS", is_token: true), CodingAdventures::GrammarTools::RuleReference.new(name: "MINUS", is_token: true)])), CodingAdventures::GrammarTools::RuleReference.new(name: "term", is_token: false)]))])
          ),
          CodingAdventures::GrammarTools::GrammarRule.new(
            name: "term",
            line_number: 35,
            body: CodingAdventures::GrammarTools::Sequence.new(elements: [CodingAdventures::GrammarTools::RuleReference.new(name: "factor", is_token: false), CodingAdventures::GrammarTools::Repetition.new(element: CodingAdventures::GrammarTools::Sequence.new(elements: [CodingAdventures::GrammarTools::Group.new(element: CodingAdventures::GrammarTools::Alternation.new(choices: [CodingAdventures::GrammarTools::RuleReference.new(name: "STAR", is_token: true), CodingAdventures::GrammarTools::RuleReference.new(name: "SLASH", is_token: true)])), CodingAdventures::GrammarTools::RuleReference.new(name: "factor", is_token: false)]))])
          ),
          CodingAdventures::GrammarTools::GrammarRule.new(
            name: "factor",
            line_number: 36,
            body: CodingAdventures::GrammarTools::Alternation.new(choices: [CodingAdventures::GrammarTools::RuleReference.new(name: "NUMBER", is_token: true), CodingAdventures::GrammarTools::RuleReference.new(name: "STRING", is_token: true), CodingAdventures::GrammarTools::RuleReference.new(name: "NAME", is_token: true), CodingAdventures::GrammarTools::RuleReference.new(name: "KEYWORD", is_token: true), CodingAdventures::GrammarTools::Sequence.new(elements: [CodingAdventures::GrammarTools::RuleReference.new(name: "LPAREN", is_token: true), CodingAdventures::GrammarTools::RuleReference.new(name: "expression", is_token: false), CodingAdventures::GrammarTools::RuleReference.new(name: "RPAREN", is_token: true)])])
          ),
        ]
      )
    end
  end
end
