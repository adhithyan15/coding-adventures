# frozen_string_literal: true

# ================================================================
# ECMAScript 5 (ES5) Lexer -- Tokenizes ES5 Source Code from Ruby
# ================================================================
#
# This module implements lexical analysis for ECMAScript 5
# (ECMA-262, 5th Edition, December 2009).
#
# ES5 landed a full decade after ES3 (ES4 was abandoned after years
# of debate). The lexical changes are modest -- the real innovations
# were strict mode semantics, native JSON support, and property
# descriptors. At the token level, the main change is:
#
#   - `debugger` keyword (promoted from future-reserved in ES3)
#   - String line continuation (backslash before newline)
#   - Reduced future-reserved word list vs ES3
#
# ES5 introduced "use strict" directive prologue, but that is a
# SEMANTIC restriction, not a lexical one. The grammar is identical
# at the token level.
#
# What ES5 does NOT have:
#   - No let/const (added in ES2015)
#   - No class syntax (added in ES2015)
#   - No arrow functions (added in ES2015)
#   - No template literals (added in ES2015)
#   - No modules (added in ES2015)
#
# Usage:
#   tokens = CodingAdventures::EcmascriptEs5Lexer.tokenize("debugger;")
#   tokens.each { |t| puts t }
# ================================================================

require "coding_adventures_grammar_tools"
require "coding_adventures_lexer"

module CodingAdventures
  module EcmascriptEs5Lexer
    # Path to the grammars directory, computed relative to this file.
    GRAMMAR_DIR = File.expand_path("../../../../../../grammars", __dir__)
    ES5_TOKENS_PATH = File.join(GRAMMAR_DIR, "ecmascript", "es5.tokens")

    # Tokenize a string of ES5 source code into an array of Token objects.
    #
    # @param source [String] ES5 JavaScript source code to tokenize
    # @return [Array<CodingAdventures::Lexer::Token>] the token stream
    def self.tokenize(source)
      grammar = CodingAdventures::GrammarTools.parse_token_grammar(
        File.read(ES5_TOKENS_PATH, encoding: "UTF-8")
      )
      lexer = CodingAdventures::Lexer::GrammarLexer.new(source, grammar)
      lexer.tokenize
    end
  end
end
