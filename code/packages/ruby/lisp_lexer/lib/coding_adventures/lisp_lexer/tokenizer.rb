# frozen_string_literal: true

# ================================================================
# Lisp Lexer — Tokenizes Lisp Source Code from Ruby
# ================================================================
#
# Lisp tokenization is elegantly simple. The entire language reduces
# to a handful of token types:
#
#   LPAREN / RPAREN  — ( and )   — the fundamental delimiters
#   NUMBER           — integers, possibly negative (-42)
#   SYMBOL           — names and operators: define, +, car, factorial?
#   STRING           — double-quoted text: "hello world"
#   QUOTE            — ' (syntactic sugar for (quote ...))
#   DOT              — . (for dotted pairs: (a . b))
#   EOF              — end of input
#
# The interesting challenge is NUMBER vs SYMBOL priority. In Lisp,
# "-" is a valid symbol (the subtraction function), but "-42" is
# a negative number. The grammar handles this by putting NUMBER
# (with its leading-minus variant) before SYMBOL in priority order.
#
# Comments start with ; and extend to end of line. Whitespace is
# not significant — the token grammar skips it automatically.
#
# Usage:
#   tokens = CodingAdventures::LispLexer.tokenize("(+ 1 2)")
#   # => [LPAREN, SYMBOL(+), NUMBER(1), NUMBER(2), RPAREN, EOF]
# ================================================================

require "coding_adventures_grammar_tools"
require "coding_adventures_lexer"

module CodingAdventures
  module LispLexer
    GRAMMAR_DIR       = File.expand_path("../../../../../../grammars", __dir__)
    LISP_TOKENS_PATH  = File.join(GRAMMAR_DIR, "lisp.tokens")

    # Create a GrammarLexer configured for Lisp.
    # @param source [String] Lisp source code
    # @return [CodingAdventures::Lexer::GrammarLexer]
    def self.create_lisp_lexer(source)
      grammar = CodingAdventures::GrammarTools.parse_token_grammar(
        File.read(LISP_TOKENS_PATH, encoding: "UTF-8")
      )
      CodingAdventures::Lexer::GrammarLexer.new(source, grammar)
    end

    # Tokenize Lisp source code.
    # @param source [String]
    # @return [Array<CodingAdventures::Lexer::Token>]
    def self.tokenize(source)
      create_lisp_lexer(source).tokenize
    end
  end
end
