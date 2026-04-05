# frozen_string_literal: true

# ================================================================
# ECMAScript 3 (ES3) Parser -- Parses ES3 Source Code into ASTs
# ================================================================
#
# This module parses ECMAScript 3 (ECMA-262, 3rd Edition, 1999)
# source code into abstract syntax trees.
#
# ES3 extends the ES1 grammar with:
#   - try/catch/finally/throw statements (structured error handling)
#   - Strict equality operators (===, !==) in expressions
#   - `instanceof` in relational expressions
#   - Regex literals as primary expressions
#
# The pipeline is:
#   1. Read es3.tokens -> GrammarLexer -> tokens
#   2. Read es3.grammar -> GrammarDrivenParser -> AST
#
# Usage:
#   ast = CodingAdventures::EcmascriptEs3Parser.parse("try { x; } catch (e) { }")
#   # => ASTNode(rule_name: "program", children: [...])
# ================================================================

require "coding_adventures_grammar_tools"
require "coding_adventures_lexer"
require "coding_adventures_parser"
require "coding_adventures_ecmascript_es3_lexer"

module CodingAdventures
  module EcmascriptEs3Parser
    # Paths to the grammar files, computed relative to this file.
    GRAMMAR_DIR = File.expand_path("../../../../../../grammars", __dir__)
    ES3_GRAMMAR_PATH = File.join(GRAMMAR_DIR, "ecmascript", "es3.grammar")

    # Parse a string of ES3 source code into a generic AST.
    #
    # @param source [String] ES3 JavaScript source code to parse
    # @return [CodingAdventures::Parser::ASTNode] the root AST node
    def self.parse(source)
      # Step 1: Tokenize using the ES3 lexer
      tokens = CodingAdventures::EcmascriptEs3Lexer.tokenize(source)

      # Step 2: Load and parse the ES3 grammar
      grammar = CodingAdventures::GrammarTools.parse_parser_grammar(
        File.read(ES3_GRAMMAR_PATH, encoding: "UTF-8")
      )

      # Step 3: Parse tokens using the grammar-driven parser
      parser = CodingAdventures::Parser::GrammarDrivenParser.new(tokens, grammar)
      parser.parse
    end
  end
end
