# frozen_string_literal: true

# ================================================================
# Starlark Compiler — High-Level Facade for the Starlark Pipeline
# ================================================================
#
# This package provides a clean, unified API for the full Starlark
# compilation and execution pipeline. It wraps and re-exports the
# existing starlark_ast_to_bytecode_compiler with convenience methods:
#
#   compile(source)          → CodeObject
#   compile_and_run(source)  → { code:, vm:, variables:, output: }
#
# Why a separate package from starlark_ast_to_bytecode_compiler?
# The starlark_ast_to_bytecode_compiler package focuses on the
# compilation machinery: opcodes, handlers, and AST walking. This
# package focuses on usability: getting from source code to results
# in one call, matching the API that other languages (Rust, Python,
# TypeScript) expose through their own starlark_compiler packages.
#
# Usage:
#   result = CodingAdventures::StarlarkCompiler.compile_and_run("x = 1 + 2\n")
#   result[:variables]["x"]   # => 3
#
#   code = CodingAdventures::StarlarkCompiler.compile("x = 42\n")
#   # => CodeObject ready for a StarlarkVM
# ================================================================

require "coding_adventures_bytecode_compiler"
require "coding_adventures_starlark_parser"
require "coding_adventures_starlark_ast_to_bytecode_compiler"

module CodingAdventures
  module StarlarkCompiler
    # Compile Starlark source code to a CodeObject.
    #
    # This is a convenience wrapper around the full pipeline:
    #   StarlarkParser.parse(source) → AST
    #   StarlarkAstToBytecodeCompiler::Compiler.compile_starlark(source) → CodeObject
    #
    # @param source [String] Starlark source code
    # @return [CodingAdventures::VirtualMachine::CodeObject]
    def self.compile(source)
      CodingAdventures::StarlarkAstToBytecodeCompiler::Compiler.compile_starlark(source)
    end

    # Compile and execute Starlark source code.
    #
    # Returns a result hash with:
    #   :code      — the CodeObject produced by compilation
    #   :vm        — the GenericVM after execution
    #   :variables — the global variable bindings
    #   :output    — list of strings printed by print() calls
    #
    # @param source [String] Starlark source code
    # @return [Hash]
    def self.compile_and_run(source)
      require "coding_adventures_starlark_vm"
      code = compile(source)
      vm   = CodingAdventures::StarlarkVM.create_starlark_vm
      vm.execute(code)
      {
        code:      code,
        vm:        vm,
        variables: vm.variables,
        output:    vm.output
      }
    end

    # Compile Starlark and return just the variable bindings.
    # Convenience method for scripts that don't need the VM or code.
    #
    # @param source [String]
    # @return [Hash]
    def self.evaluate(source)
      compile_and_run(source)[:variables]
    end
  end
end
