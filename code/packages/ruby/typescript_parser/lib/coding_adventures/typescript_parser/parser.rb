# frozen_string_literal: true

# ================================================================
# TypeScript Parser -- Parses TypeScript Source Code into ASTs from Ruby
# ================================================================
#
# This module mirrors the typescript_lexer pattern: instead of writing
# a TypeScript-specific parser from scratch, we reuse the general-purpose
# GrammarDrivenParser engine from the coding_adventures_parser gem,
# feeding it the TypeScript grammar from typescript.grammar.
#
# The pipeline is:
#
#   1. Read typescript.tokens -> build TokenGrammar -> GrammarLexer -> tokens
#   2. Read typescript.grammar -> build ParserGrammar -> GrammarDrivenParser -> AST
#
# Usage:
#   ast = CodingAdventures::TypescriptParser.parse("let x = 1 + 2;")
#   # => ASTNode(rule_name: "program", children: [...])
# ================================================================

require "coding_adventures_grammar_tools"
require "coding_adventures_lexer"
require "coding_adventures_parser"
require "coding_adventures_typescript_lexer"

module CodingAdventures
  module TypescriptParser
    # Paths to the grammar files, computed relative to this file.
    GRAMMAR_DIR = File.expand_path("../../../../../../grammars", __dir__)
    TS_GRAMMAR_PATH = File.join(GRAMMAR_DIR, "typescript.grammar")

    # Parse a string of TypeScript source code into a generic AST.
    #
    # @param source [String] TypeScript source code to parse
    # @return [CodingAdventures::Parser::ASTNode] the root AST node
    def self.parse(source)
      # Step 1: Tokenize using the TypeScript lexer
      tokens = CodingAdventures::TypescriptLexer.tokenize(source)

      # Step 2: Load and parse the TypeScript grammar
      grammar = CodingAdventures::GrammarTools.parse_parser_grammar(
        File.read(TS_GRAMMAR_PATH, encoding: "UTF-8")
      )

      # Step 3: Parse tokens using the grammar-driven parser
      parser = CodingAdventures::Parser::GrammarDrivenParser.new(tokens, grammar)
      parser.parse
    end
  end
end
