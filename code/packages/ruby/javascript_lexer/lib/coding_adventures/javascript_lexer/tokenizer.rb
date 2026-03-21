# frozen_string_literal: true

# ================================================================
# JavaScript Lexer -- Tokenizes JavaScript Source Code from Ruby
# ================================================================
#
# This module demonstrates the power of the grammar-driven approach
# to language tooling. Instead of writing a JavaScript-specific lexer
# from scratch, we reuse the general-purpose GrammarLexer engine
# from the coding_adventures_lexer gem, feeding it the JavaScript
# token definitions from javascript.tokens.
#
# JavaScript has features that Python and Ruby do not:
# - `let`, `const`, `var` for variable declarations
# - `===` and `!==` for strict equality
# - Semicolons terminate statements
# - Curly braces `{}` for blocks
# - `$` is valid in identifiers
# - `=>` for arrow functions
#
# All of these are handled by the grammar file — no new code needed.
#
# Usage:
#   tokens = CodingAdventures::JavascriptLexer.tokenize("let x = 1 + 2;")
#   tokens.each { |t| puts t }
# ================================================================

require "coding_adventures_grammar_tools"
require "coding_adventures_lexer"

module CodingAdventures
  module JavascriptLexer
    # Path to the grammars directory, computed relative to this file.
    # We navigate up from lib/coding_adventures/javascript_lexer/ to the
    # repository root's code/grammars/ directory.
    GRAMMAR_DIR = File.expand_path("../../../../../../grammars", __dir__)
    JS_TOKENS_PATH = File.join(GRAMMAR_DIR, "javascript.tokens")

    # Tokenize a string of JavaScript source code into an array of Token objects.
    #
    # @param source [String] JavaScript source code to tokenize
    # @return [Array<CodingAdventures::Lexer::Token>] the token stream
    def self.tokenize(source)
      grammar = CodingAdventures::GrammarTools.parse_token_grammar(
        File.read(JS_TOKENS_PATH, encoding: "UTF-8")
      )
      lexer = CodingAdventures::Lexer::GrammarLexer.new(source, grammar)
      lexer.tokenize
    end
  end
end
