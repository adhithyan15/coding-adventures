# frozen_string_literal: true

require "coding_adventures_grammar_tools"
require "coding_adventures_lexer"

module CodingAdventures
  module NibLexer
    GRAMMAR_DIR = File.expand_path("../../../../../../grammars", __dir__)
    NIB_TOKENS_PATH = File.join(GRAMMAR_DIR, "nib.tokens")

    def self.create_nib_lexer(source)
      grammar = CodingAdventures::GrammarTools.parse_token_grammar(
        File.read(NIB_TOKENS_PATH, encoding: "UTF-8")
      )
      CodingAdventures::Lexer::GrammarLexer.new(source, grammar)
    end

    def self.tokenize_nib(source)
      create_nib_lexer(source).tokenize
    end
  end
end
