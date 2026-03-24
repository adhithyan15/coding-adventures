# frozen_string_literal: true

# IMPORTANT: Require dependencies FIRST, before own modules.
# Ruby loads files in require order. If our modules reference
# constants from dependencies, those gems must be loaded first.
require "coding_adventures_lattice_ast_to_css"
require "coding_adventures_lattice_parser"
require "coding_adventures_lattice_lexer"
require "coding_adventures_grammar_tools"
require "coding_adventures_parser"
require "coding_adventures_lexer"

require_relative "coding_adventures/lattice_transpiler/version"
require_relative "coding_adventures/lattice_transpiler/transpiler"

module CodingAdventures
  # End-to-end Lattice source to CSS text pipeline.
  #
  # Exposes:
  #   CodingAdventures::LatticeTranspiler.transpile(source, minified: false, indent: "  ")
  module LatticeTranspiler
  end
end
