# frozen_string_literal: true

# ================================================================
# JSON Lexer -- Tokenizes JSON Text from Ruby
# ================================================================
#
# This module demonstrates the power of the grammar-driven approach
# to language tooling. Instead of writing a JSON-specific lexer
# from scratch, we reuse the general-purpose GrammarLexer engine
# from the coding_adventures_lexer gem, feeding it the JSON token
# definitions from json.tokens.
#
# JSON (JavaScript Object Notation, RFC 8259) is the simplest
# practical grammar for the grammar-driven infrastructure. Unlike
# programming languages, JSON has:
#
#   - No keywords (true/false/null are value literals, not keywords)
#   - No comments
#   - No identifiers or variable names
#   - No indentation significance
#   - No operators (colon and comma are structural delimiters)
#   - No reserved words
#
# The entire token set consists of just 9 token types:
#
#   Value tokens:   STRING, NUMBER, TRUE, FALSE, NULL
#   Structure:      LBRACE, RBRACE, LBRACKET, RBRACKET, COLON, COMMA
#   Skipped:        WHITESPACE (spaces, tabs, newlines)
#
# Because JSON has no keywords or indentation mode, the lexer
# produces a flat stream of tokens with no synthetic tokens like
# INDENT, DEDENT, or NEWLINE. This makes JSON the ideal "hello
# world" grammar for testing the lexer infrastructure.
#
# Usage:
#   tokens = CodingAdventures::JsonLexer.tokenize('{"key": 42}')
#   tokens.each { |t| puts t }
# ================================================================

require "coding_adventures_grammar_tools"
require "coding_adventures_lexer"

module CodingAdventures
  module JsonLexer
    # Path to the grammars directory, computed relative to this file.
    # We navigate up from lib/coding_adventures/json_lexer/ to the
    # repository root's code/grammars/ directory.
    #
    # The directory structure looks like this:
    #   code/
    #     grammars/
    #       json.tokens    <-- we need this file
    #     packages/
    #       ruby/
    #         json_lexer/
    #           lib/
    #             coding_adventures/
    #               json_lexer/
    #                 tokenizer.rb  <-- we are here (__dir__)
    #
    # So from __dir__ we go up 6 levels to reach code/, then into grammars/.
    GRAMMAR_DIR = File.expand_path("../../../../../../grammars", __dir__)
    JSON_TOKENS_PATH = File.join(GRAMMAR_DIR, "json.tokens")

    # Tokenize a string of JSON text into an array of Token objects.
    #
    # This is the main entry point. It:
    # 1. Reads the json.tokens grammar file
    # 2. Parses it into a TokenGrammar using grammar_tools
    # 3. Feeds the grammar and source into GrammarLexer
    # 4. Returns the resulting token array
    #
    # Unlike the Starlark or Python lexers, the JSON lexer does NOT
    # emit INDENT, DEDENT, or NEWLINE tokens because json.tokens does
    # not use "mode: indentation". Whitespace is simply skipped.
    #
    # @param source [String] JSON text to tokenize
    # @return [Array<CodingAdventures::Lexer::Token>] the token stream
    def self.tokenize(source)
      # Read the json.tokens file and parse it into a TokenGrammar.
      # The TokenGrammar contains the regex patterns for STRING, NUMBER,
      # TRUE, FALSE, NULL, and the structural delimiters. There are no
      # keywords or reserved words sections.
      grammar = CodingAdventures::GrammarTools.parse_token_grammar(
        File.read(JSON_TOKENS_PATH, encoding: "UTF-8")
      )

      # Create a GrammarLexer instance and run it. The lexer walks through
      # the source character by character, matching patterns from the grammar
      # in priority order (first match wins). Since JSON has no indentation
      # mode, the lexer runs in its simplest configuration.
      lexer = CodingAdventures::Lexer::GrammarLexer.new(source, grammar)
      lexer.tokenize
    end
  end
end
