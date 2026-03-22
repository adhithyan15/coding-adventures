# frozen_string_literal: true

# ================================================================
# TOML Lexer -- Tokenizes TOML Text from Ruby
# ================================================================
#
# This module is a thin wrapper around the grammar-driven GrammarLexer
# engine, feeding it the TOML token definitions from toml.tokens.
#
# TOML (Tom's Obvious Minimal Language, v1.0.0) has significantly more
# token types than JSON. Where JSON has 11 token types and no special
# modes, TOML has:
#
#   - 4 string types:  BASIC_STRING, ML_BASIC_STRING, LITERAL_STRING,
#                       ML_LITERAL_STRING
#   - 2 number types:  INTEGER, FLOAT (with hex/oct/bin/special aliases)
#   - 2 booleans:      TRUE, FALSE
#   - 4 date/times:    OFFSET_DATETIME, LOCAL_DATETIME, LOCAL_DATE,
#                       LOCAL_TIME
#   - 1 key type:      BARE_KEY
#   - 7 delimiters:    EQUALS, DOT, COMMA, LBRACKET, RBRACKET,
#                       LBRACE, RBRACE
#   - 2 structural:    NEWLINE, EOF
#
# Critical differences from JSON:
#
#   1. **Newline sensitivity** -- TOML's skip pattern only consumes
#      spaces and tabs. Newlines become NEWLINE tokens that the grammar
#      uses to delimit key-value pairs.
#
#   2. **escapes: none** -- The lexer strips quotes but does NOT process
#      escape sequences. TOML has 4 string types with different escape
#      rules; the semantic layer handles this.
#
#   3. **Token ordering** -- First-match-wins is critical. Multi-line
#      strings before single-line, dates before bare keys, floats before
#      integers, BARE_KEY dead last.
#
# Usage:
#   tokens = CodingAdventures::TomlLexer.tokenize('name = "TOML"')
#   tokens.each { |t| puts "#{t.type}: #{t.value}" }
# ================================================================

require "coding_adventures_grammar_tools"
require "coding_adventures_lexer"

module CodingAdventures
  module TomlLexer
    # Path to the grammars directory, computed relative to this file.
    # We navigate up from lib/coding_adventures/toml_lexer/ to the
    # repository root's code/grammars/ directory.
    #
    # The directory structure:
    #   code/
    #     grammars/
    #       toml.tokens    <-- we need this file
    #     packages/
    #       ruby/
    #         toml_lexer/
    #           lib/
    #             coding_adventures/
    #               toml_lexer/
    #                 tokenizer.rb  <-- we are here (__dir__)
    #
    # So from __dir__ we go up 6 levels to reach code/, then into grammars/.
    GRAMMAR_DIR = File.expand_path("../../../../../../grammars", __dir__)
    TOML_TOKENS_PATH = File.join(GRAMMAR_DIR, "toml.tokens")

    # Tokenize a string of TOML text into an array of Token objects.
    #
    # This is the main entry point. It:
    # 1. Reads the toml.tokens grammar file
    # 2. Parses it into a TokenGrammar using grammar_tools
    # 3. Feeds the grammar and source into GrammarLexer
    # 4. Returns the resulting token array
    #
    # Unlike JSON, the TOML lexer emits NEWLINE tokens because TOML is
    # newline-sensitive. The skip pattern only skips spaces and tabs.
    # Comments are skipped but their trailing newlines are preserved.
    #
    # The lexer uses escapes: none mode, meaning string tokens have their
    # surrounding quotes stripped but escape sequences are left as raw text.
    # The parser's semantic layer handles escape processing.
    #
    # @param source [String] TOML text to tokenize
    # @return [Array<CodingAdventures::Lexer::Token>] the token stream
    def self.tokenize(source)
      grammar = CodingAdventures::GrammarTools.parse_token_grammar(
        File.read(TOML_TOKENS_PATH, encoding: "UTF-8")
      )
      lexer = CodingAdventures::Lexer::GrammarLexer.new(source, grammar)
      lexer.tokenize
    end
  end
end
