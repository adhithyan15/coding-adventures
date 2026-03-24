# frozen_string_literal: true

# IMPORTANT: Require dependencies FIRST, before own modules.
# Ruby loads files in require order. If our modules reference
# constants from dependencies, those gems must be loaded first.
require "coding_adventures_lattice_parser"
require "coding_adventures_lattice_lexer"
require "coding_adventures_grammar_tools"
require "coding_adventures_parser"
require "coding_adventures_lexer"

require_relative "coding_adventures/lattice_ast_to_css/version"
require_relative "coding_adventures/lattice_ast_to_css/errors"
require_relative "coding_adventures/lattice_ast_to_css/scope"
require_relative "coding_adventures/lattice_ast_to_css/evaluator"
require_relative "coding_adventures/lattice_ast_to_css/transformer"
require_relative "coding_adventures/lattice_ast_to_css/emitter"

module CodingAdventures
  # Three-pass compiler: Lattice AST to clean CSS AST.
  #
  # Exports:
  #   LatticeAstToCss::LatticeTransformer -- three-pass transformer
  #   LatticeAstToCss::CSSEmitter         -- CSS text emitter
  #   LatticeAstToCss::LatticeError       -- base error class
  #   LatticeAstToCss::ScopeChain         -- lexical scope chain
  #   LatticeAstToCss::ExpressionEvaluator -- expression evaluator
  #   LatticeAstToCss::LatticeNumber, LatticeDimension, etc. -- value types
  module LatticeAstToCss
  end
end
