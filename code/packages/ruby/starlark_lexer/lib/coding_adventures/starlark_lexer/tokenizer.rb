# frozen_string_literal: true

# ================================================================
# Starlark Lexer -- Tokenizes Starlark Source Code from Ruby
# ================================================================
#
# This module demonstrates the power of the grammar-driven approach
# to language tooling. Instead of writing a Starlark-specific lexer
# from scratch, we reuse the general-purpose GrammarLexer engine
# from the coding_adventures_lexer gem, feeding it the Starlark
# token definitions from starlark.tokens.
#
# Starlark is a deterministic subset of Python designed for
# configuration files. It was created by Google for use in Bazel
# (their build system) and is now used in Buck2, Pants, and other
# tools. The key design choice: Starlark removes features that
# make Python non-deterministic or slow to evaluate:
#
#   - No while loops (only for-loops over finite collections)
#   - No recursion (functions cannot call themselves)
#   - No try/except (errors are always fatal)
#   - No classes (only functions and simple data types)
#   - No import statement (uses load() instead)
#   - No global mutable state (frozen after top-level evaluation)
#
# Because Starlark is a Python subset, many tokens are identical
# to Python's. The key differences in the token grammar are:
#
#   1. Keywords: Starlark adds 'load' and 'lambda', removes 'while',
#      'class', 'import', 'try', 'except', etc.
#   2. Reserved words: Python keywords not in Starlark (class, import,
#      while, etc.) are reserved -- using them is a syntax error.
#   3. Indentation: Like Python, Starlark uses significant whitespace.
#      The lexer emits INDENT/DEDENT/NEWLINE tokens.
#
# The insight is the same as with the Python lexer: the engine is
# fixed, and the grammar is the variable. By swapping python.tokens
# for starlark.tokens, we get a complete Starlark tokenizer.
#
# Usage:
#   tokens = CodingAdventures::StarlarkLexer.tokenize("x = 1 + 2")
#   tokens.each { |t| puts t }
# ================================================================

require "coding_adventures_grammar_tools"
require "coding_adventures_lexer"

module CodingAdventures
  module StarlarkLexer
    # Path to the grammars directory, computed relative to this file.
    # We navigate up from lib/coding_adventures/starlark_lexer/ to the
    # repository root's code/grammars/ directory.
    #
    # The directory structure looks like this:
    #   code/
    #     grammars/
    #       starlark.tokens    <-- we need this file
    #     packages/
    #       ruby/
    #         starlark_lexer/
    #           lib/
    #             coding_adventures/
    #               starlark_lexer/
    #                 tokenizer.rb  <-- we are here (__dir__)
    #
    # So from __dir__ we go up 6 levels to reach code/, then into grammars/.
    GRAMMAR_DIR = File.expand_path("../../../../../../grammars", __dir__)
    STARLARK_TOKENS_PATH = File.join(GRAMMAR_DIR, "starlark.tokens")

    # Tokenize a string of Starlark source code into an array of Token objects.
    #
    # This is the main entry point. It:
    # 1. Reads the starlark.tokens grammar file
    # 2. Parses it into a TokenGrammar using grammar_tools
    # 3. Feeds the grammar and source into GrammarLexer
    # 4. Returns the resulting token array
    #
    # The returned tokens include synthetic INDENT, DEDENT, and NEWLINE
    # tokens because starlark.tokens uses "mode: indentation". These
    # synthetic tokens are essential for the parser to understand block
    # structure (function bodies, if/for blocks, etc.).
    #
    # @param source [String] Starlark source code to tokenize
    # @return [Array<CodingAdventures::Lexer::Token>] the token stream
    def self.tokenize(source)
      # Read the starlark.tokens file and parse it into a TokenGrammar.
      # The TokenGrammar contains the regex patterns, keyword lists,
      # reserved word lists, and mode settings (indentation mode).
      grammar = CodingAdventures::GrammarTools.parse_token_grammar(
        File.read(STARLARK_TOKENS_PATH, encoding: "UTF-8")
      )

      # Create a GrammarLexer instance and run it. The lexer walks through
      # the source character by character, matching patterns from the grammar
      # in priority order (first match wins). When in indentation mode, it
      # also tracks leading whitespace and emits INDENT/DEDENT tokens.
      lexer = CodingAdventures::Lexer::GrammarLexer.new(source, grammar)
      lexer.tokenize
    end
  end
end
