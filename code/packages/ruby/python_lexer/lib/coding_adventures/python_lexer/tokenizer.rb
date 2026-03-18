# frozen_string_literal: true

# ================================================================
# Python Lexer -- Tokenizes Python Source Code from Ruby
# ================================================================
#
# This module demonstrates the power of the grammar-driven approach
# to language tooling. Instead of writing a Python-specific lexer
# from scratch, we reuse the general-purpose GrammarLexer engine
# from the coding_adventures_lexer gem, feeding it the Python token
# definitions from python.tokens.
#
# The insight is simple but profound: the same lexer code that
# tokenizes one language can tokenize any language, as long as you
# provide the right grammar file. This is exactly how tools like
# Tree-sitter and TextMate grammars work -- the engine is fixed,
# and the grammar is the variable.
#
# Donald Knuth called this "separation of concerns" -- the lexer
# engine knows *how* to tokenize (the algorithm), while the .tokens
# file knows *what* to tokenize (the language-specific patterns).
#
# Usage:
#   tokens = CodingAdventures::PythonLexer.tokenize("x = 1 + 2")
#   tokens.each { |t| puts t }
# ================================================================

require "coding_adventures_grammar_tools"
require "coding_adventures_lexer"

module CodingAdventures
  module PythonLexer
    # Path to the grammars directory, computed relative to this file.
    # We navigate up from lib/coding_adventures/python_lexer/ to the
    # repository root's code/grammars/ directory.
    GRAMMAR_DIR = File.expand_path("../../../../../../grammars", __dir__)
    PYTHON_TOKENS_PATH = File.join(GRAMMAR_DIR, "python.tokens")

    # Tokenize a string of Python source code into an array of Token objects.
    #
    # This is the main entry point. It:
    # 1. Reads the python.tokens grammar file
    # 2. Parses it into a TokenGrammar using grammar_tools
    # 3. Feeds the grammar and source into GrammarLexer
    # 4. Returns the resulting token array
    #
    # @param source [String] Python source code to tokenize
    # @return [Array<CodingAdventures::Lexer::Token>] the token stream
    def self.tokenize(source)
      grammar = CodingAdventures::GrammarTools.parse_token_grammar(
        File.read(PYTHON_TOKENS_PATH, encoding: "UTF-8")
      )
      lexer = CodingAdventures::Lexer::GrammarLexer.new(source, grammar)
      lexer.tokenize
    end
  end
end
