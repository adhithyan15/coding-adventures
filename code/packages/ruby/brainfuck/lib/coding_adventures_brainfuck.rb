# frozen_string_literal: true

# ==========================================================================
# Brainfuck Interpreter — Built on the Pluggable GenericVM
# ==========================================================================
#
# This gem is a Brainfuck interpreter that proves the GenericVM architecture
# works for radically different languages. Starlark has 50+ opcodes,
# variables, functions, and collections. Brainfuck has 8 opcodes and a tape.
# Both run on the same GenericVM chassis — different engines, same car.
#
# Usage:
#
#   require "coding_adventures_brainfuck"
#
#   result = CodingAdventures::Brainfuck.execute_brainfuck("++>+++++[<+>-]")
#   result.tape[0]  #=> 7
# ==========================================================================

require "coding_adventures_virtual_machine"
require "coding_adventures_grammar_tools"
require "coding_adventures_lexer"
require "coding_adventures_parser"
require "coding_adventures_interpreter_ir"
require "coding_adventures_vm_core"
require "coding_adventures_jit_core"

require_relative "coding_adventures/brainfuck/version"
require_relative "coding_adventures/brainfuck/opcodes"
require_relative "coding_adventures/brainfuck/translator"
require_relative "coding_adventures/brainfuck/handlers"
require_relative "coding_adventures/brainfuck/vm"
require_relative "coding_adventures/brainfuck/lexer"
require_relative "coding_adventures/brainfuck/parser"
require_relative "coding_adventures/brainfuck/lang_vm"

module CodingAdventures
  module Brainfuck
  end
end
