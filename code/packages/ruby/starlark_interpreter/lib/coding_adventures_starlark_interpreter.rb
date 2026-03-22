# frozen_string_literal: true

# ==========================================================================
# coding_adventures_starlark_interpreter -- Top-Level Require
# ==========================================================================
#
# This is the entry point for the gem. When someone writes:
#
#   require "coding_adventures_starlark_interpreter"
#
# Ruby loads this file, which in turn loads all dependencies and the
# interpreter module.
#
# Usage:
#   result = CodingAdventures::StarlarkInterpreter.interpret("x = 42\n")
#   result.variables["x"]  # => 42
# ==========================================================================

# Load all dependencies in the correct order.
require "coding_adventures_grammar_tools"
require "coding_adventures_lexer"
require "coding_adventures_parser"
require "coding_adventures_virtual_machine"
require "coding_adventures_bytecode_compiler"
require "coding_adventures_starlark_lexer"
require "coding_adventures_starlark_parser"
require "coding_adventures_starlark_ast_to_bytecode_compiler"
require "coding_adventures_starlark_vm"

require_relative "coding_adventures/starlark_interpreter/version"
require_relative "coding_adventures/starlark_interpreter/interpreter"

module CodingAdventures
  module StarlarkInterpreter
  end
end
