# frozen_string_literal: true

require "coding_adventures_grammar_tools"
require "coding_adventures_parser"
require "coding_adventures_nib_lexer"

module CodingAdventures
  module NibParser
    GRAMMAR_DIR = File.expand_path("../../../../../../grammars", __dir__)
    NIB_GRAMMAR_PATH = File.join(GRAMMAR_DIR, "nib.grammar")

    def self.create_nib_parser(source)
      tokens = CodingAdventures::NibLexer.tokenize_nib(source)
      grammar = CodingAdventures::GrammarTools.parse_parser_grammar(
        File.read(NIB_GRAMMAR_PATH, encoding: "UTF-8")
      )
      CodingAdventures::Parser::GrammarDrivenParser.new(tokens, grammar)
    end

    def self.parse_nib(source)
      create_nib_parser(source).parse
    end
  end
end
