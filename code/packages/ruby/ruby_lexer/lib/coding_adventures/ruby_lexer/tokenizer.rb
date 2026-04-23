# frozen_string_literal: true

# ================================================================
# Ruby Lexer — Tokenizes Ruby Source Code from Ruby
# ================================================================
#
# The Ruby lexer is a thin wrapper around the grammar-driven lexer
# engine, loaded with ruby.tokens. This is a delightful example of
# meta-circularity: Ruby code tokenizing Ruby code.
#
# Ruby's token set is richer than Lisp's but shares the same
# grammar-driven philosophy. Key Ruby-specific tokens:
#
#   SYMBOL          — :name (a Symbol literal, distinct from identifiers)
#   ARROW           — => (hash rocket, used in hash literals and case/when)
#   RANGE           — .. (inclusive range)
#   RANGE_EXCL      — ... (exclusive range)
#   STRING_INTERP   — #{...} opening/closing (interpolation markers)
#   REGEX           — /pattern/ (regex literals, context-dependent)
#   METHOD_CALL     — . (method dispatch)
#
# Ruby uses significant indentation in some contexts (here-docs) but
# NOT for blocks (which use do/end or {/}). The grammar does NOT use
# indentation mode — newlines are not structurally significant.
#
# Usage:
#   tokens = CodingAdventures::RubyLexer.tokenize("x = 1 + 2")
#   tokens.map(&:value) # => ["x", "=", "1", "+", "2", ""]
# ================================================================

require "coding_adventures_grammar_tools"
require "coding_adventures_lexer"

module CodingAdventures
  module RubyLexer
    GRAMMAR_DIR       = File.expand_path("../../../../../../grammars", __dir__)
    RUBY_TOKENS_PATH  = File.join(GRAMMAR_DIR, "ruby.tokens")
    COMPILED_TOKENS_PATH = File.expand_path("_grammar.rb", __dir__)

    def self.token_grammar
      @token_grammar ||= CodingAdventures::GrammarTools.load_token_grammar(COMPILED_TOKENS_PATH)
    end

    # Create a GrammarLexer configured for Ruby.
    # @param source [String] Ruby source code
    # @return [CodingAdventures::Lexer::GrammarLexer]
    def self.create_ruby_lexer(source)
      CodingAdventures::Lexer::GrammarLexer.new(source, token_grammar)
    end

    # Tokenize Ruby source code.
    # @param source [String]
    # @return [Array<CodingAdventures::Lexer::Token>]
    def self.tokenize(source)
      create_ruby_lexer(source).tokenize
    end
  end
end
