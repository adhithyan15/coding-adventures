# frozen_string_literal: true
# AUTO-GENERATED FILE - DO NOT EDIT
require "coding_adventures_grammar_tools"

module CodingAdventures
  module TomlGrammar
    def self.grammar
      @grammar ||= CodingAdventures::GrammarTools::ParserGrammar.new(
        version: 1,
        rules: [
          CodingAdventures::GrammarTools::GrammarRule.new(
            name: "document",
            line_number: 38,
            body: CodingAdventures::GrammarTools::Repetition.new(element: CodingAdventures::GrammarTools::Alternation.new(choices: [CodingAdventures::GrammarTools::RuleReference.new(name: "NEWLINE", is_token: true), CodingAdventures::GrammarTools::RuleReference.new(name: "expression", is_token: false)]))
          ),
          CodingAdventures::GrammarTools::GrammarRule.new(
            name: "expression",
            line_number: 49,
            body: CodingAdventures::GrammarTools::Alternation.new(choices: [CodingAdventures::GrammarTools::RuleReference.new(name: "array_table_header", is_token: false), CodingAdventures::GrammarTools::RuleReference.new(name: "table_header", is_token: false), CodingAdventures::GrammarTools::RuleReference.new(name: "keyval", is_token: false)])
          ),
          CodingAdventures::GrammarTools::GrammarRule.new(
            name: "keyval",
            line_number: 57,
            body: CodingAdventures::GrammarTools::Sequence.new(elements: [CodingAdventures::GrammarTools::RuleReference.new(name: "key", is_token: false), CodingAdventures::GrammarTools::RuleReference.new(name: "EQUALS", is_token: true), CodingAdventures::GrammarTools::RuleReference.new(name: "value", is_token: false)])
          ),
          CodingAdventures::GrammarTools::GrammarRule.new(
            name: "key",
            line_number: 65,
            body: CodingAdventures::GrammarTools::Sequence.new(elements: [CodingAdventures::GrammarTools::RuleReference.new(name: "simple_key", is_token: false), CodingAdventures::GrammarTools::Repetition.new(element: CodingAdventures::GrammarTools::Sequence.new(elements: [CodingAdventures::GrammarTools::RuleReference.new(name: "DOT", is_token: true), CodingAdventures::GrammarTools::RuleReference.new(name: "simple_key", is_token: false)]))])
          ),
          CodingAdventures::GrammarTools::GrammarRule.new(
            name: "simple_key",
            line_number: 82,
            body: CodingAdventures::GrammarTools::Alternation.new(choices: [CodingAdventures::GrammarTools::RuleReference.new(name: "BARE_KEY", is_token: true), CodingAdventures::GrammarTools::RuleReference.new(name: "BASIC_STRING", is_token: true), CodingAdventures::GrammarTools::RuleReference.new(name: "LITERAL_STRING", is_token: true), CodingAdventures::GrammarTools::RuleReference.new(name: "TRUE", is_token: true), CodingAdventures::GrammarTools::RuleReference.new(name: "FALSE", is_token: true), CodingAdventures::GrammarTools::RuleReference.new(name: "INTEGER", is_token: true), CodingAdventures::GrammarTools::RuleReference.new(name: "FLOAT", is_token: true), CodingAdventures::GrammarTools::RuleReference.new(name: "OFFSET_DATETIME", is_token: true), CodingAdventures::GrammarTools::RuleReference.new(name: "LOCAL_DATETIME", is_token: true), CodingAdventures::GrammarTools::RuleReference.new(name: "LOCAL_DATE", is_token: true), CodingAdventures::GrammarTools::RuleReference.new(name: "LOCAL_TIME", is_token: true)])
          ),
          CodingAdventures::GrammarTools::GrammarRule.new(
            name: "table_header",
            line_number: 92,
            body: CodingAdventures::GrammarTools::Sequence.new(elements: [CodingAdventures::GrammarTools::RuleReference.new(name: "LBRACKET", is_token: true), CodingAdventures::GrammarTools::RuleReference.new(name: "key", is_token: false), CodingAdventures::GrammarTools::RuleReference.new(name: "RBRACKET", is_token: true)])
          ),
          CodingAdventures::GrammarTools::GrammarRule.new(
            name: "array_table_header",
            line_number: 104,
            body: CodingAdventures::GrammarTools::Sequence.new(elements: [CodingAdventures::GrammarTools::RuleReference.new(name: "LBRACKET", is_token: true), CodingAdventures::GrammarTools::RuleReference.new(name: "LBRACKET", is_token: true), CodingAdventures::GrammarTools::RuleReference.new(name: "key", is_token: false), CodingAdventures::GrammarTools::RuleReference.new(name: "RBRACKET", is_token: true), CodingAdventures::GrammarTools::RuleReference.new(name: "RBRACKET", is_token: true)])
          ),
          CodingAdventures::GrammarTools::GrammarRule.new(
            name: "value",
            line_number: 121,
            body: CodingAdventures::GrammarTools::Alternation.new(choices: [CodingAdventures::GrammarTools::RuleReference.new(name: "BASIC_STRING", is_token: true), CodingAdventures::GrammarTools::RuleReference.new(name: "ML_BASIC_STRING", is_token: true), CodingAdventures::GrammarTools::RuleReference.new(name: "LITERAL_STRING", is_token: true), CodingAdventures::GrammarTools::RuleReference.new(name: "ML_LITERAL_STRING", is_token: true), CodingAdventures::GrammarTools::RuleReference.new(name: "INTEGER", is_token: true), CodingAdventures::GrammarTools::RuleReference.new(name: "FLOAT", is_token: true), CodingAdventures::GrammarTools::RuleReference.new(name: "TRUE", is_token: true), CodingAdventures::GrammarTools::RuleReference.new(name: "FALSE", is_token: true), CodingAdventures::GrammarTools::RuleReference.new(name: "OFFSET_DATETIME", is_token: true), CodingAdventures::GrammarTools::RuleReference.new(name: "LOCAL_DATETIME", is_token: true), CodingAdventures::GrammarTools::RuleReference.new(name: "LOCAL_DATE", is_token: true), CodingAdventures::GrammarTools::RuleReference.new(name: "LOCAL_TIME", is_token: true), CodingAdventures::GrammarTools::RuleReference.new(name: "array", is_token: false), CodingAdventures::GrammarTools::RuleReference.new(name: "inline_table", is_token: false)])
          ),
          CodingAdventures::GrammarTools::GrammarRule.new(
            name: "array",
            line_number: 140,
            body: CodingAdventures::GrammarTools::Sequence.new(elements: [CodingAdventures::GrammarTools::RuleReference.new(name: "LBRACKET", is_token: true), CodingAdventures::GrammarTools::RuleReference.new(name: "array_values", is_token: false), CodingAdventures::GrammarTools::RuleReference.new(name: "RBRACKET", is_token: true)])
          ),
          CodingAdventures::GrammarTools::GrammarRule.new(
            name: "array_values",
            line_number: 142,
            body: CodingAdventures::GrammarTools::Sequence.new(elements: [CodingAdventures::GrammarTools::Repetition.new(element: CodingAdventures::GrammarTools::RuleReference.new(name: "NEWLINE", is_token: true)), CodingAdventures::GrammarTools::OptionalElement.new(element: CodingAdventures::GrammarTools::Sequence.new(elements: [CodingAdventures::GrammarTools::RuleReference.new(name: "value", is_token: false), CodingAdventures::GrammarTools::Repetition.new(element: CodingAdventures::GrammarTools::RuleReference.new(name: "NEWLINE", is_token: true)), CodingAdventures::GrammarTools::Repetition.new(element: CodingAdventures::GrammarTools::Sequence.new(elements: [CodingAdventures::GrammarTools::RuleReference.new(name: "COMMA", is_token: true), CodingAdventures::GrammarTools::Repetition.new(element: CodingAdventures::GrammarTools::RuleReference.new(name: "NEWLINE", is_token: true)), CodingAdventures::GrammarTools::RuleReference.new(name: "value", is_token: false), CodingAdventures::GrammarTools::Repetition.new(element: CodingAdventures::GrammarTools::RuleReference.new(name: "NEWLINE", is_token: true))])), CodingAdventures::GrammarTools::OptionalElement.new(element: CodingAdventures::GrammarTools::RuleReference.new(name: "COMMA", is_token: true)), CodingAdventures::GrammarTools::Repetition.new(element: CodingAdventures::GrammarTools::RuleReference.new(name: "NEWLINE", is_token: true))]))])
          ),
          CodingAdventures::GrammarTools::GrammarRule.new(
            name: "inline_table",
            line_number: 162,
            body: CodingAdventures::GrammarTools::Sequence.new(elements: [CodingAdventures::GrammarTools::RuleReference.new(name: "LBRACE", is_token: true), CodingAdventures::GrammarTools::OptionalElement.new(element: CodingAdventures::GrammarTools::Sequence.new(elements: [CodingAdventures::GrammarTools::RuleReference.new(name: "keyval", is_token: false), CodingAdventures::GrammarTools::Repetition.new(element: CodingAdventures::GrammarTools::Sequence.new(elements: [CodingAdventures::GrammarTools::RuleReference.new(name: "COMMA", is_token: true), CodingAdventures::GrammarTools::RuleReference.new(name: "keyval", is_token: false)]))])), CodingAdventures::GrammarTools::RuleReference.new(name: "RBRACE", is_token: true)])
          ),
        ]
      )
    end
  end
end
