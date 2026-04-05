# frozen_string_literal: true

# ================================================================
# ECMAScript 5 (ES5) Parser -- Parses ES5 Source Code into ASTs
# ================================================================
#
# This module parses ECMAScript 5 (ECMA-262, 5th Edition, 2009)
# source code into abstract syntax trees.
#
# ES5 extends the ES3 grammar with:
#   - The `debugger` statement
#   - Getter/setter property definitions in object literals
#     (using contextual `get` and `set` -- not keywords)
#
# The pipeline is:
#   1. Read es5.tokens -> GrammarLexer -> tokens
#   2. Read es5.grammar -> GrammarDrivenParser -> AST
#
# Usage:
#   ast = CodingAdventures::EcmascriptEs5Parser.parse("debugger;")
#   # => ASTNode(rule_name: "program", children: [...])
# ================================================================

require "coding_adventures_grammar_tools"
require "coding_adventures_lexer"
require "coding_adventures_parser"
require "coding_adventures_ecmascript_es5_lexer"

module CodingAdventures
  module EcmascriptEs5Parser
    # Paths to the grammar files, computed relative to this file.
    GRAMMAR_DIR = File.expand_path("../../../../../../grammars", __dir__)
    ES5_GRAMMAR_PATH = File.join(GRAMMAR_DIR, "ecmascript", "es5.grammar")

    # Parse a string of ES5 source code into a generic AST.
    #
    # @param source [String] ES5 JavaScript source code to parse
    # @return [CodingAdventures::Parser::ASTNode] the root AST node
    def self.parse(source)
      # Step 1: Tokenize using the ES5 lexer
      tokens = CodingAdventures::EcmascriptEs5Lexer.tokenize(source)

      # Step 2: Load and parse the ES5 grammar
      grammar = CodingAdventures::GrammarTools.parse_parser_grammar(
        File.read(ES5_GRAMMAR_PATH, encoding: "UTF-8")
      )

      # Step 3: Parse tokens using the grammar-driven parser
      parser = CodingAdventures::Parser::GrammarDrivenParser.new(tokens, grammar)
      parser.parse
    end
  end
end
