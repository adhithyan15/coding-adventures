# frozen_string_literal: true

# ================================================================
# Mosaic Lexer -- Tokenizes .mosaic Source from Ruby
# ================================================================
#
# This module is the lexical analysis stage for the Mosaic Component
# Description Language (CDL). Mosaic is a declarative language for
# describing UI component structure with named typed slots. It compiles
# to native code per target platform (Web Components, React, SwiftUI,
# Compose, Rust/paint-vm).
#
# Like the JSON lexer, this module reuses the general-purpose
# GrammarLexer engine from coding_adventures_lexer, feeding it the
# Mosaic token definitions from mosaic.tokens.
#
# Mosaic's token vocabulary includes:
#
#   Literals:    STRING, NUMBER, DIMENSION, COLOR_HEX
#   Keywords:    component, slot, import, from, as, text, number,
#                bool, image, color, node, list, true, false, when, each
#   Identifier:  NAME (allows hyphens for CSS-like names)
#   Delimiters:  LBRACE, RBRACE, LANGLE, RANGLE, COLON, SEMICOLON,
#                COMMA, DOT, EQUALS, AT
#   Skipped:     LINE_COMMENT, BLOCK_COMMENT, WHITESPACE
#
# Usage:
#   tokens = CodingAdventures::MosaicLexer.tokenize(source)
#   tokens.each { |t| puts "#{t.type}: #{t.value}" }
# ================================================================

require "coding_adventures_grammar_tools"
require "coding_adventures_lexer"

module CodingAdventures
  module MosaicLexer
    # Path to the grammars directory, computed relative to this file.
    # Navigate up from lib/coding_adventures/mosaic_lexer/ to code/grammars/.
    #
    #   code/
    #     grammars/
    #       mosaic.tokens    <-- we need this file
    #     packages/
    #       ruby/
    #         mosaic_lexer/
    #           lib/
    #             coding_adventures/
    #               mosaic_lexer/
    #                 tokenizer.rb  <-- we are here (__dir__)
    GRAMMAR_DIR = File.expand_path("../../../../../../grammars", __dir__)
    MOSAIC_TOKENS_PATH = File.join(GRAMMAR_DIR, "mosaic.tokens")

    # Tokenize a string of Mosaic source into an array of Token objects.
    #
    # This is the main entry point. It:
    # 1. Reads the mosaic.tokens grammar file
    # 2. Parses it into a TokenGrammar using grammar_tools
    # 3. Feeds the grammar and source into GrammarLexer
    # 4. Returns the resulting token array
    #
    # Unlike JSON, Mosaic has keywords (component, slot, import, etc.) and
    # identifier tokens (NAME). Keywords take priority over NAME when the
    # text matches exactly. Comments and whitespace are skipped silently.
    #
    # @param source [String] Mosaic source text to tokenize
    # @return [Array<CodingAdventures::Lexer::Token>] the token stream
    def self.tokenize(source)
      grammar = CodingAdventures::GrammarTools.parse_token_grammar(
        File.read(MOSAIC_TOKENS_PATH, encoding: "UTF-8")
      )

      lexer = CodingAdventures::Lexer::GrammarLexer.new(source, grammar)
      lexer.tokenize
    end
  end
end
