# frozen_string_literal: true

# ==========================================================================
# Parser -- Building Abstract Syntax Trees from Token Streams
# ==========================================================================
#
# This gem is the Ruby port of the Python parser package. It provides two
# complementary approaches:
#
# 1. A hand-written recursive descent Parser that produces typed AST nodes
#    (NumberLiteral, BinaryOp, etc.).
#
# 2. A grammar-driven GrammarParser that reads .grammar files and produces
#    generic ASTNode objects.
#
# The hand-written parser is the reference implementation -- clear and easy
# to debug. The grammar-driven parser is flexible and language-agnostic.
# ==========================================================================

require_relative "coding_adventures/parser/version"
require_relative "coding_adventures/parser/ast_nodes"
require_relative "coding_adventures/parser/parser"
require_relative "coding_adventures/parser/grammar_parser"

module CodingAdventures
  module Parser
  end
end
