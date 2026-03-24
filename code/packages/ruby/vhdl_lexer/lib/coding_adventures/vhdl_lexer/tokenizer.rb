# frozen_string_literal: true

# ================================================================
# VHDL Lexer -- Tokenizes VHDL Source Code from Ruby
# ================================================================
#
# This module demonstrates the grammar-driven approach applied to
# VHDL (VHSIC Hardware Description Language). VHDL is fundamentally
# different from Verilog in philosophy:
#
# - It is verbose and Ada-like, with explicit type declarations
#   and strong typing. Where Verilog says "wire a;", VHDL says
#   "signal a : std_logic;".
# - It is CASE INSENSITIVE. "ENTITY", "Entity", and "entity" are
#   identical. We handle this via post-tokenization normalization.
# - Logical operations are keyword operators (and, or, xor, not)
#   rather than symbol operators (&, |, ^, ~).
# - It has no preprocessor. There are no `define, `ifdef, or
#   `include directives.
#
# The grammar-driven lexer handles all VHDL-specific patterns
# through the token definitions in vhdl.tokens. The only VHDL-
# specific code here is the post_tokenize hook that lowercases
# NAME and KEYWORD token values for case insensitivity.
#
# Usage:
#   tokens = CodingAdventures::VhdlLexer.tokenize("signal clk : std_logic;")
#
# Case insensitivity:
#   # These two calls produce identical token streams:
#   CodingAdventures::VhdlLexer.tokenize("ENTITY counter IS")
#   CodingAdventures::VhdlLexer.tokenize("entity counter is")
#   # Both yield: KEYWORD("entity"), NAME("counter"), KEYWORD("is")
# ================================================================

require "coding_adventures_grammar_tools"
require "coding_adventures_lexer"

module CodingAdventures
  module VhdlLexer
    # Path to the grammars directory, computed relative to this file.
    # We navigate up from lib/coding_adventures/vhdl_lexer/ to the
    # repository root's code/grammars/ directory.
    GRAMMAR_DIR = File.expand_path("../../../../../../grammars", __dir__)
    VHDL_TOKENS_PATH = File.join(GRAMMAR_DIR, "vhdl.tokens")

    # Tokenize a string of VHDL source code into an array of Token objects.
    #
    # Unlike the Verilog lexer, there is no `preprocess:` option because
    # VHDL has no preprocessor. The source is tokenized directly.
    #
    # After tokenization, a post-processing step normalizes all NAME and
    # KEYWORD token values to lowercase. This implements VHDL's case
    # insensitivity: "ENTITY", "Entity", and "entity" all become the
    # keyword "entity".
    #
    # @param source [String] VHDL source code to tokenize
    # @return [Array<CodingAdventures::Lexer::Token>] the token stream
    def self.tokenize(source)
      grammar = CodingAdventures::GrammarTools.parse_token_grammar(
        File.read(VHDL_TOKENS_PATH, encoding: "UTF-8")
      )
      lexer = CodingAdventures::Lexer::GrammarLexer.new(source, grammar)
      raw_tokens = lexer.tokenize

      # Build a set of keywords for re-classification after lowercasing.
      keyword_set = grammar.keywords.to_set

      # ----------------------------------------------------------
      # Post-tokenization case normalization
      # ----------------------------------------------------------
      #
      # VHDL is case-insensitive (IEEE 1076-2008, Section 15.4):
      #   "Basic identifiers differing only in the use of
      #    corresponding uppercase and lowercase letters are
      #    considered the same."
      #
      # We normalize NAME and KEYWORD values to lowercase so that:
      #   1. Keyword matching works regardless of source casing.
      #   2. Downstream tools (parsers, analyzers) don't need to
      #      handle case variations.
      #
      # Because the grammar lexer's keyword matching happens BEFORE
      # case normalization, an uppercase keyword like "ENTITY" is
      # initially classified as NAME (it doesn't match "entity" in
      # the keyword set). After lowercasing, we re-check: if the
      # lowercased value is in the keyword set, we reclassify the
      # token as KEYWORD.
      #
      # Extended identifiers (\like_this\) are NOT normalized --
      # they preserve case per the VHDL standard.
      #
      # Token objects are immutable (Data.define), so we create new
      # Token instances with the downcased value. Tokens of other
      # types (NUMBER, STRING, operators, etc.) pass through unchanged.
      # ----------------------------------------------------------
      raw_tokens.map do |token|
        if token.type == Lexer::TokenType::NAME || token.type == Lexer::TokenType::KEYWORD
          downcased = token.value.downcase
          # Re-classify: if the lowercased value is a keyword,
          # the token type becomes KEYWORD regardless of original case.
          new_type = keyword_set.include?(downcased) ? Lexer::TokenType::KEYWORD : Lexer::TokenType::NAME
          Lexer::Token.new(
            type: new_type,
            value: downcased,
            line: token.line,
            column: token.column
          )
        else
          token
        end
      end
    end
  end
end
