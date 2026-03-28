# frozen_string_literal: true

require "coding_adventures_grammar_tools"
require "coding_adventures_lexer"
require "coding_adventures_parser"
require "coding_adventures_directed_graph"
require "coding_adventures_state_machine"
require "coding_adventures_virtual_machine"
require "coding_adventures_bytecode_compiler"
require "coding_adventures_starlark_lexer"
require "coding_adventures_starlark_parser"
require "coding_adventures_starlark_ast_to_bytecode_compiler"

require_relative "coding_adventures/starlark_compiler/version"
require_relative "coding_adventures/starlark_compiler/compiler"

module CodingAdventures
  module StarlarkCompiler
  end
end
