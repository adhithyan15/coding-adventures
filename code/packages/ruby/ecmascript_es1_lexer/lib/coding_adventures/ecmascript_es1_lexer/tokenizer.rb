# frozen_string_literal: true

# ================================================================
# ECMAScript 1 (ES1) Lexer -- Tokenizes ES1 Source Code from Ruby
# ================================================================
#
# This module implements lexical analysis for the very first version of
# standardized JavaScript: ECMA-262, 1st Edition (June 1997).
#
# ES1 defined the foundational syntax that all subsequent ECMAScript
# editions build upon. It introduced:
#   - 23 keywords (break, case, continue, default, delete, do, else,
#     for, function, if, in, new, return, switch, this, typeof, var,
#     void, while, with, true, false, null)
#   - Basic operators: arithmetic (+, -, *, /, %), comparison (==, !=,
#     <, >, <=, >=), bitwise (&, |, ^, ~, <<, >>, >>>), logical (&&, ||, !)
#   - Compound assignment operators (+=, -=, *=, /=, etc.)
#   - Increment/decrement (++, --)
#   - The ternary operator (?:)
#   - String literals (single and double quoted)
#   - Numeric literals (decimal, float, hex with 0x prefix, scientific)
#   - The $ character in identifiers (unusual for 1997)
#
# Notably ABSENT from ES1:
#   - No === or !== (strict equality -- added in ES3)
#   - No try/catch/finally/throw (error handling -- added in ES3)
#   - No regex literals (formalized in ES3)
#   - No let/const/class/arrow functions (added in ES2015)
#
# Usage:
#   tokens = CodingAdventures::EcmascriptEs1Lexer.tokenize("var x = 1 + 2;")
#   tokens.each { |t| puts t }
# ================================================================

require "coding_adventures_grammar_tools"
require "coding_adventures_lexer"

module CodingAdventures
  module EcmascriptEs1Lexer
    # Path to the grammars directory, computed relative to this file.
    # We navigate up from lib/coding_adventures/ecmascript_es1_lexer/ to the
    # repository root's code/grammars/ directory.
    GRAMMAR_DIR = File.expand_path("../../../../../../grammars", __dir__)
    ES1_TOKENS_PATH = File.join(GRAMMAR_DIR, "ecmascript", "es1.tokens")

    # Tokenize a string of ES1 source code into an array of Token objects.
    #
    # The tokenizer reads the es1.tokens grammar file, which defines:
    #   - Token patterns (NAME, NUMBER, STRING, operators, delimiters)
    #   - Keywords (the 23 ES1 keywords)
    #   - Reserved words (class, const, enum, export, extends, import, super)
    #   - Skip patterns (whitespace, single-line comments, block comments)
    #   - Error recovery tokens (unterminated strings)
    #
    # @param source [String] ES1 JavaScript source code to tokenize
    # @return [Array<CodingAdventures::Lexer::Token>] the token stream
    def self.tokenize(source)
      grammar = CodingAdventures::GrammarTools.parse_token_grammar(
        File.read(ES1_TOKENS_PATH, encoding: "UTF-8")
      )
      lexer = CodingAdventures::Lexer::GrammarLexer.new(source, grammar)
      lexer.tokenize
    end
  end
end
