# frozen_string_literal: true

# ==========================================================================
# coding_adventures_starlark_vm -- Top-Level Require
# ==========================================================================
#
# This is the entry point for the gem. When someone writes:
#
#   require "coding_adventures_starlark_vm"
#
# Ruby loads this file, which in turn loads all dependencies and the VM
# components: types, handlers, builtins, and the factory module.
#
# Usage:
#   result = CodingAdventures::StarlarkVM.execute_starlark("x = 42\n")
#   result.variables["x"]  # => 42
# ==========================================================================

# Dependencies must be loaded first -- the VM uses GenericVM from
# virtual_machine and opcodes from the compiler.
require "coding_adventures_grammar_tools"
require "coding_adventures_lexer"
require "coding_adventures_parser"
require "coding_adventures_virtual_machine"
require "coding_adventures_bytecode_compiler"
require "coding_adventures_starlark_lexer"
require "coding_adventures_starlark_parser"
require "coding_adventures_starlark_ast_to_bytecode_compiler"

require_relative "coding_adventures/starlark_vm/version"
require_relative "coding_adventures/starlark_vm/types"
require_relative "coding_adventures/starlark_vm/handlers"
require_relative "coding_adventures/starlark_vm/builtins"
require_relative "coding_adventures/starlark_vm/vm"

module CodingAdventures
  module StarlarkVM
  end
end
