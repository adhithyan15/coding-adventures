# frozen_string_literal: true

# ================================================================
# ECMAScript 3 (ES3) Lexer -- Tokenizes ES3 Source Code from Ruby
# ================================================================
#
# This module implements lexical analysis for ECMAScript 3
# (ECMA-262, 3rd Edition, December 1999).
#
# ES3 was the version that made JavaScript a real, complete language.
# It landed two years after ES1 and added features that developers
# today consider fundamental.
#
# What ES3 adds over ES1:
#   - === and !== (strict equality -- no type coercion)
#   - try/catch/finally/throw (structured error handling)
#   - Regular expression literals (/pattern/flags)
#   - `instanceof` operator
#   - `catch` and `finally` keywords
#   - Expanded future-reserved word list
#
# The strict equality operators (=== and !==) are perhaps the most
# impactful addition. They compare without type coercion:
#   "" == 0     // true  (abstract equality coerces types)
#   "" === 0    // false (strict equality, different types)
#
# Usage:
#   tokens = CodingAdventures::EcmascriptEs3Lexer.tokenize("x === 1")
#   tokens.each { |t| puts t }
# ================================================================

require "coding_adventures_grammar_tools"
require "coding_adventures_lexer"

module CodingAdventures
  module EcmascriptEs3Lexer
    # Path to the grammars directory, computed relative to this file.
    GRAMMAR_DIR = File.expand_path("../../../../../../grammars", __dir__)
    ES3_TOKENS_PATH = File.join(GRAMMAR_DIR, "ecmascript", "es3.tokens")

    # Tokenize a string of ES3 source code into an array of Token objects.
    #
    # @param source [String] ES3 JavaScript source code to tokenize
    # @return [Array<CodingAdventures::Lexer::Token>] the token stream
    def self.tokenize(source)
      grammar = CodingAdventures::GrammarTools.parse_token_grammar(
        File.read(ES3_TOKENS_PATH, encoding: "UTF-8")
      )
      lexer = CodingAdventures::Lexer::GrammarLexer.new(source, grammar)
      lexer.tokenize
    end
  end
end
