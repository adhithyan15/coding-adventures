# frozen_string_literal: true

require "coding_adventures_grammar_tools"
require "coding_adventures_lexer"

module CodingAdventures
  module NibLexer
    GRAMMAR_DIR = File.expand_path("../../../../../../grammars", __dir__)
    NIB_TOKENS_PATH = File.join(GRAMMAR_DIR, "nib", "nib.tokens")
    COMPILED_TOKENS_PATH = File.expand_path("_grammar.rb", __dir__)

    def self.token_grammar
      @token_grammar ||= CodingAdventures::GrammarTools.load_token_grammar(COMPILED_TOKENS_PATH)
    end

    def self.create_nib_lexer(source)
      CodingAdventures::Lexer::GrammarLexer.new(source, token_grammar)
    end

    def self.tokenize_nib(source)
      create_nib_lexer(source).tokenize
    end
  end
end
