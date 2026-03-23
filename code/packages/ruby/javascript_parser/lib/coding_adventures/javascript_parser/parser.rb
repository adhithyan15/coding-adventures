# frozen_string_literal: true

# ================================================================
# JavaScript Parser -- Parses JavaScript Source Code into ASTs from Ruby
# ================================================================
#
# This module mirrors the javascript_lexer pattern: instead of writing
# a JavaScript-specific parser from scratch, we reuse the general-purpose
# GrammarDrivenParser engine from the coding_adventures_parser gem,
# feeding it the JavaScript grammar from javascript.grammar.
#
# The pipeline is:
#
#   1. Read javascript.tokens -> build TokenGrammar -> GrammarLexer -> tokens
#   2. Read javascript.grammar -> build ParserGrammar -> GrammarDrivenParser -> AST
#
# Usage:
#   ast = CodingAdventures::JavascriptParser.parse("let x = 1 + 2;")
#   # => ASTNode(rule_name: "program", children: [...])
# ================================================================

require "coding_adventures_grammar_tools"
require "coding_adventures_lexer"
require "coding_adventures_parser"
require "coding_adventures_javascript_lexer"

module CodingAdventures
  module JavascriptParser
    # Paths to the grammar files, computed relative to this file.
    GRAMMAR_DIR = File.expand_path("../../../../../../grammars", __dir__)
    JS_GRAMMAR_PATH = File.join(GRAMMAR_DIR, "javascript.grammar")

    # Parse a string of JavaScript source code into a generic AST.
    #
    # @param source [String] JavaScript source code to parse
    # @return [CodingAdventures::Parser::ASTNode] the root AST node
    def self.parse(source)
      # Step 1: Tokenize using the JavaScript lexer
      tokens = CodingAdventures::JavascriptLexer.tokenize(source)

      # Step 2: Load and parse the JavaScript grammar
      grammar = CodingAdventures::GrammarTools.parse_parser_grammar(
        File.read(JS_GRAMMAR_PATH, encoding: "UTF-8")
      )

      # Step 3: Parse tokens using the grammar-driven parser
      parser = CodingAdventures::Parser::GrammarDrivenParser.new(tokens, grammar)
      parser.parse
    end
  end
end
