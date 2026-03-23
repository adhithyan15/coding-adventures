# frozen_string_literal: true

# IMPORTANT: Require dependencies FIRST, before own modules.
# Ruby loads files in require order. If our modules reference
# constants from dependencies, those gems must be loaded first.
require "coding_adventures_lattice_lexer"
require "coding_adventures_grammar_tools"
require "coding_adventures_parser"

require_relative "coding_adventures/lattice_parser/version"
require_relative "coding_adventures/lattice_parser/parser"

module CodingAdventures
  # Parser producing an AST for Lattice source.
  #
  # Exposes:
  #   CodingAdventures::LatticeParser.parse(source) -> ASTNode
  #   CodingAdventures::LatticeParser.create_parser(source) -> GrammarDrivenParser
  module LatticeParser
  end
end
