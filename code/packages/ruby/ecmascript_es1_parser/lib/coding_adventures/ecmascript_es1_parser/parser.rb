# frozen_string_literal: true

# ================================================================
# ECMAScript 1 (ES1) Parser -- Parses ES1 Source Code into ASTs
# ================================================================
#
# This module mirrors the ecmascript_es1_lexer pattern: instead of
# writing an ES1-specific parser from scratch, we reuse the
# general-purpose GrammarDrivenParser engine from the
# coding_adventures_parser gem, feeding it the ES1 grammar.
#
# The pipeline is:
#
#   1. Read es1.tokens -> build TokenGrammar -> GrammarLexer -> tokens
#   2. Read es1.grammar -> build ParserGrammar -> GrammarDrivenParser -> AST
#
# ES1's grammar covers the complete language as specified in the first
# ECMAScript standard: variable declarations (var only), function
# declarations/expressions, all 14 statement types, and the full
# expression precedence chain from comma operator down to primary
# expressions.
#
# Notable grammar features:
#   - No try/catch/finally/throw (added in ES3)
#   - No debugger statement (added in ES5)
#   - Equality uses == and != only (no === or !==)
#   - The `with` statement is included (later deprecated)
#   - Operator precedence encoded via rule nesting (PEG semantics)
#
# Usage:
#   ast = CodingAdventures::EcmascriptEs1Parser.parse("var x = 1 + 2;")
#   # => ASTNode(rule_name: "program", children: [...])
# ================================================================

require "coding_adventures_grammar_tools"
require "coding_adventures_lexer"
require "coding_adventures_parser"
require "coding_adventures_ecmascript_es1_lexer"

module CodingAdventures
  module EcmascriptEs1Parser
    # Paths to the grammar files, computed relative to this file.
    GRAMMAR_DIR = File.expand_path("../../../../../../grammars", __dir__)
    ES1_GRAMMAR_PATH = File.join(GRAMMAR_DIR, "ecmascript", "es1.grammar")

    # Parse a string of ES1 source code into a generic AST.
    #
    # @param source [String] ES1 JavaScript source code to parse
    # @return [CodingAdventures::Parser::ASTNode] the root AST node
    def self.parse(source)
      # Step 1: Tokenize using the ES1 lexer
      tokens = CodingAdventures::EcmascriptEs1Lexer.tokenize(source)

      # Step 2: Load and parse the ES1 grammar
      grammar = CodingAdventures::GrammarTools.parse_parser_grammar(
        File.read(ES1_GRAMMAR_PATH, encoding: "UTF-8")
      )

      # Step 3: Parse tokens using the grammar-driven parser
      parser = CodingAdventures::Parser::GrammarDrivenParser.new(tokens, grammar)
      parser.parse
    end
  end
end
