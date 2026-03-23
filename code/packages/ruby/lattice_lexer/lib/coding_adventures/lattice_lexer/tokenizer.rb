# frozen_string_literal: true

# ================================================================
# Lattice Tokenizer -- Tokenizes Lattice CSS Superset Source
# ================================================================
#
# Lattice is a CSS superset language that adds:
#   - Variables: $color, $font-size
#   - Mixins: @mixin button($bg) { ... } / @include button(red)
#   - Control flow: @if $theme == dark { ... } @else { ... }
#   - Iteration: @for $i from 1 through 12 { ... }
#   - List iteration: @each $color in red, green, blue { ... }
#   - Functions: @function spacing($n) { @return $n * 8px; }
#   - Modules: @use "colors";
#
# This module is a thin wrapper around the generic GrammarLexer.
# It loads lattice.tokens (which inherits all CSS tokens plus 5
# new Lattice-specific tokens) and produces a token stream.
#
# New tokens beyond standard CSS:
#
#   VARIABLE       -- $color, $font-size
#   EQUALS_EQUALS  -- == (equality in @if expressions)
#   NOT_EQUALS     -- != (inequality)
#   GREATER_EQUALS -- >= (greater-or-equal)
#   LESS_EQUALS    -- <= (less-or-equal)
#
# // single-line comments are also supported (not in CSS).
#
# Grammar file location:
#
#   tokenizer.rb lives at:
#     code/packages/ruby/lattice_lexer/lib/coding_adventures/lattice_lexer/
#
#   lattice.tokens lives at:
#     code/grammars/lattice.tokens
#
#   Directory count from __dir__ up to code/:
#     lattice_lexer/  (1)
#     coding_adventures/ (2)
#     lib/           (3)
#     lattice_lexer/ (4)
#     ruby/          (5)
#     packages/      (6)
#     code/          -> grammars/lattice.tokens
#
# So: File.expand_path("../../../../../../grammars", __dir__)
# ================================================================

module CodingAdventures
  module LatticeLexer
    # Path to the grammars directory, relative to this file.
    # Navigate up 6 levels from lib/coding_adventures/lattice_lexer/
    # to reach code/, then into grammars/.
    GRAMMAR_DIR = File.expand_path("../../../../../../grammars", __dir__)
    LATTICE_TOKENS_PATH = File.join(GRAMMAR_DIR, "lattice.tokens")

    # Tokenize Lattice source text and return an array of Token objects.
    #
    # This is the primary entry point. It:
    # 1. Reads lattice.tokens and parses it into a TokenGrammar.
    # 2. Creates a GrammarLexer with the Lattice token definitions.
    # 3. Runs the lexer and returns the resulting token array.
    #
    # The returned array always ends with an EOF token. Skip patterns
    # (whitespace, line comments, block comments) are consumed without
    # producing tokens.
    #
    # @param source [String] Lattice source text to tokenize
    # @return [Array<CodingAdventures::Lexer::Token>] the token stream
    # @raise [CodingAdventures::Lexer::LexerError] on unrecognized input
    def self.tokenize(source)
      grammar = CodingAdventures::GrammarTools.parse_token_grammar(
        File.read(LATTICE_TOKENS_PATH, encoding: "UTF-8")
      )
      lexer = CodingAdventures::Lexer::GrammarLexer.new(source, grammar)
      lexer.tokenize
    end

    # Create a GrammarLexer configured for Lattice source text.
    #
    # Useful when you want to inspect or reuse the lexer object rather
    # than calling tokenize directly. Call .tokenize on the result to
    # get the token array.
    #
    # @param source [String] Lattice source text to tokenize
    # @return [CodingAdventures::Lexer::GrammarLexer]
    def self.create_lexer(source)
      grammar = CodingAdventures::GrammarTools.parse_token_grammar(
        File.read(LATTICE_TOKENS_PATH, encoding: "UTF-8")
      )
      CodingAdventures::Lexer::GrammarLexer.new(source, grammar)
    end
  end
end
