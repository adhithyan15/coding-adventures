# frozen_string_literal: true

# ==========================================================================
# CodingAdventures::CompilerSourceMap — Source Map Chain for AOT Pipeline
# ==========================================================================
#
# This gem provides the source-mapping sidecar that flows through every stage
# of the AOT compiler pipeline. It records the transformations at each stage:
#
#   Source text → AST → IR → Optimised IR → Machine code
#
# and allows you to query in both directions:
#
#   source_to_mc(pos)  — "which machine code bytes came from this source pos?"
#   mc_to_source(off)  — "which source line produced this machine code byte?"
#
# Usage:
#
#   require "coding_adventures_compiler_source_map"
#
#   chain = CodingAdventures::CompilerSourceMap::SourceMapChain.new
#   pos   = CodingAdventures::CompilerSourceMap::SourcePosition.new(
#     file: "hello.bf", line: 1, column: 1, length: 1
#   )
#   chain.source_to_ast.add(pos, 0)
#   chain.ast_to_ir.add(0, [5, 6, 7, 8])
# ==========================================================================

require_relative "coding_adventures/compiler_source_map/version"
require_relative "coding_adventures/compiler_source_map/source_map"

module CodingAdventures
  module CompilerSourceMap
  end
end
