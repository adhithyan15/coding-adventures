# frozen_string_literal: true

require "coding_adventures_grammar_tools"
require "coding_adventures_parser"
require "coding_adventures_nib_lexer"

module CodingAdventures
  module NibParser
    GRAMMAR_DIR = File.expand_path("../../../../../../grammars", __dir__)
    NIB_GRAMMAR_PATH = File.join(GRAMMAR_DIR, "nib", "nib.grammar")
    COMPILED_GRAMMAR_PATH = File.expand_path("_grammar.rb", __dir__)

    def self.parser_grammar
      @parser_grammar ||= CodingAdventures::GrammarTools.load_parser_grammar(COMPILED_GRAMMAR_PATH)
    end

    def self.create_nib_parser(source)
      tokens = CodingAdventures::NibLexer.tokenize_nib(source)
      CodingAdventures::Parser::GrammarDrivenParser.new(tokens, parser_grammar)
    end

    def self.parse_nib(source)
      create_nib_parser(source).parse
    end
  end
end
