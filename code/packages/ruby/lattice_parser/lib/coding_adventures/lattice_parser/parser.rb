# frozen_string_literal: true

# ================================================================
# Lattice Parser -- Parses Lattice Source into an AST
# ================================================================
#
# This module is a thin wrapper around the generic GrammarDrivenParser.
# It combines two grammar files to go from Lattice source text to an
# Abstract Syntax Tree (AST):
#
#   1. lattice.tokens  -> token definitions (via LatticeLexer)
#   2. lattice.grammar -> syntactic rules  (this module)
#
# The pipeline:
#
#   Lattice source
#     |  (LatticeLexer.tokenize)
#     v
#   [Token, Token, ...]
#     |  (GrammarDrivenParser)
#     v
#   ASTNode(rule_name: "stylesheet", children: [...])
#
# The resulting AST contains BOTH CSS nodes and Lattice nodes:
#
#   CSS nodes: qualified_rule, at_rule, declaration, selector_list, ...
#   Lattice nodes: variable_declaration, mixin_definition, if_directive, ...
#
# The LatticeAstToCss package (next step in the pipeline) removes
# all Lattice nodes by expanding them into pure CSS.
#
# Grammar File Location:
#
#   parser.rb lives at:
#     code/packages/ruby/lattice_parser/lib/coding_adventures/lattice_parser/
#
#   lattice.grammar lives at:
#     code/grammars/lattice.grammar
#
#   Navigate up 6 levels from __dir__ to code/, then into grammars/.
# ================================================================

module CodingAdventures
  module LatticeParser
    # Path to the grammar file, relative to this file.
    GRAMMAR_DIR = File.expand_path("../../../../../../grammars", __dir__)
    LATTICE_GRAMMAR_PATH = File.join(GRAMMAR_DIR, "lattice.grammar")

    # Parse Lattice source text and return an AST.
    #
    # This is the primary entry point. It:
    # 1. Tokenizes the source using LatticeLexer (loads lattice.tokens).
    # 2. Reads and parses lattice.grammar into a ParserGrammar.
    # 3. Feeds the tokens and grammar into GrammarDrivenParser.
    # 4. Returns the root ASTNode (rule_name: "stylesheet").
    #
    # @param source [String] Lattice source text to parse
    # @return [CodingAdventures::Parser::ASTNode] the root AST node
    # @raise [CodingAdventures::Lexer::LexerError] on unrecognized input
    # @raise [CodingAdventures::Parser::GrammarParseError] on syntax errors
    def self.parse(source)
      tokens = CodingAdventures::LatticeLexer.tokenize(source)
      grammar = CodingAdventures::GrammarTools.parse_parser_grammar(
        File.read(LATTICE_GRAMMAR_PATH, encoding: "UTF-8")
      )
      parser = CodingAdventures::Parser::GrammarDrivenParser.new(tokens, grammar)
      parser.parse
    end

    # Create a GrammarDrivenParser configured for Lattice source text.
    #
    # Useful when you want to inspect the parser object before calling
    # .parse. The returned parser is ready to call .parse on.
    #
    # @param source [String] Lattice source text to parse
    # @return [CodingAdventures::Parser::GrammarDrivenParser]
    def self.create_parser(source)
      tokens = CodingAdventures::LatticeLexer.tokenize(source)
      grammar = CodingAdventures::GrammarTools.parse_parser_grammar(
        File.read(LATTICE_GRAMMAR_PATH, encoding: "UTF-8")
      )
      CodingAdventures::Parser::GrammarDrivenParser.new(tokens, grammar)
    end
  end
end
