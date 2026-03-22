# frozen_string_literal: true

# ================================================================
# coding_adventures_starlark_ast_to_bytecode_compiler -- Top-Level Require
# ================================================================
#
# This is the entry point for the gem. When someone writes:
#
#   require "coding_adventures_starlark_ast_to_bytecode_compiler"
#
# Ruby loads this file, which in turn loads the opcodes, compiler,
# and all dependencies. The compiler is where the real work happens.
#
# Usage:
#   code = CodingAdventures::StarlarkAstToBytecodeCompiler::Compiler.compile_starlark("x = 42\n")
#   # => CodeObject with instructions=[LOAD_CONST 0, STORE_NAME 0, HALT],
#   #    constants=[42], names=["x"]
# ================================================================

# Dependencies must be loaded first -- the compiler uses GenericCompiler
# from bytecode_compiler, types from virtual_machine, and the parser pipeline.
require "coding_adventures_grammar_tools"
require "coding_adventures_lexer"
require "coding_adventures_parser"
require "coding_adventures_virtual_machine"
require "coding_adventures_bytecode_compiler"
require "coding_adventures_starlark_lexer"
require "coding_adventures_starlark_parser"

require_relative "coding_adventures/starlark_ast_to_bytecode_compiler/version"
require_relative "coding_adventures/starlark_ast_to_bytecode_compiler/opcodes"
require_relative "coding_adventures/starlark_ast_to_bytecode_compiler/compiler"

module CodingAdventures
  module StarlarkAstToBytecodeCompiler
  end
end
