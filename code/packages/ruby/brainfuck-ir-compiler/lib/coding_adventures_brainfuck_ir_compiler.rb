# frozen_string_literal: true

# ==========================================================================
# CodingAdventures::BrainfuckIrCompiler — Brainfuck AOT Frontend
# ==========================================================================
#
# This gem is the Brainfuck-specific frontend of the AOT compiler pipeline.
# It takes a Brainfuck AST (from coding_adventures_brainfuck) and emits:
#
#   1. An IrProgram (from coding_adventures_compiler_ir)
#   2. A SourceMapChain with segments 1+2 filled (from coding_adventures_compiler_source_map)
#
# Usage:
#
#   require "coding_adventures_brainfuck_ir_compiler"
#   require "coding_adventures_brainfuck"
#
#   ast    = CodingAdventures::Brainfuck::Parser.parse("++[>+<-].")
#   config = CodingAdventures::BrainfuckIrCompiler::BuildConfig.release_config
#   result = CodingAdventures::BrainfuckIrCompiler.compile(ast, "hello.bf", config)
#
#   result.program      # => IrProgram with instructions
#   result.source_map   # => SourceMapChain with SourceToAst + AstToIr
# ==========================================================================

require "coding_adventures_compiler_ir"
require "coding_adventures_compiler_source_map"
require "coding_adventures_lexer"
require "coding_adventures_parser"

require_relative "coding_adventures/brainfuck_ir_compiler/version"
require_relative "coding_adventures/brainfuck_ir_compiler/build_config"
require_relative "coding_adventures/brainfuck_ir_compiler/compiler"

module CodingAdventures
  module BrainfuckIrCompiler
  end
end
