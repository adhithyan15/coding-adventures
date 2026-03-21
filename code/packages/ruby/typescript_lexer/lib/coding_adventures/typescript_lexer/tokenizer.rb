# frozen_string_literal: true

# ================================================================
# TypeScript Lexer -- Tokenizes TypeScript Source Code from Ruby
# ================================================================
#
# This module demonstrates the power of the grammar-driven approach
# to language tooling. Instead of writing a TypeScript-specific lexer
# from scratch, we reuse the general-purpose GrammarLexer engine
# from the coding_adventures_lexer gem, feeding it the TypeScript
# token definitions from typescript.tokens.
#
# TypeScript extends JavaScript with additional features:
# - `interface`, `type`, `enum`, `namespace`, `declare` keywords
# - Type annotation keywords: `number`, `string`, `boolean`, etc.
# - `readonly`, `abstract`, `implements` keywords
# - All JavaScript features carry over
#
# All of these are handled by the grammar file — no new code needed.
#
# Usage:
#   tokens = CodingAdventures::TypescriptLexer.tokenize("let x: number = 1 + 2;")
#   tokens.each { |t| puts t }
# ================================================================

require "coding_adventures_grammar_tools"
require "coding_adventures_lexer"

module CodingAdventures
  module TypescriptLexer
    # Path to the grammars directory, computed relative to this file.
    # We navigate up from lib/coding_adventures/typescript_lexer/ to the
    # repository root's code/grammars/ directory.
    GRAMMAR_DIR = File.expand_path("../../../../../../grammars", __dir__)
    TS_TOKENS_PATH = File.join(GRAMMAR_DIR, "typescript.tokens")

    # Tokenize a string of TypeScript source code into an array of Token objects.
    #
    # @param source [String] TypeScript source code to tokenize
    # @return [Array<CodingAdventures::Lexer::Token>] the token stream
    def self.tokenize(source)
      grammar = CodingAdventures::GrammarTools.parse_token_grammar(
        File.read(TS_TOKENS_PATH, encoding: "UTF-8")
      )
      lexer = CodingAdventures::Lexer::GrammarLexer.new(source, grammar)
      lexer.tokenize
    end
  end
end
