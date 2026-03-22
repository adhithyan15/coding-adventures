# frozen_string_literal: true

# ==========================================================================
# Starlark VM Factory -- Create and Execute Starlark Virtual Machines
# ==========================================================================
#
# This module provides two main entry points:
#
# 1. create_starlark_vm -- Creates a configured GenericVM with all Starlark
#    opcode handlers and builtin functions registered. You can then compile
#    code separately and execute it on the VM.
#
# 2. execute_starlark -- One-shot execution: takes Starlark source code,
#    compiles it, creates a VM, executes it, and returns a StarlarkResult.
#
# == Architecture
#
# The Starlark VM is built on top of the GenericVM (from virtual_machine gem).
# The GenericVM provides the execution engine (stack, program counter, call
# stack, fetch-decode-execute loop). This module plugs in:
#
#   - 46 opcode handlers (from handlers.rb) via register_opcode()
#   - 23 builtin functions (from builtins.rb) via register_builtin()
#
# The result is a complete Starlark interpreter that can execute any valid
# Starlark program.
#
# == Usage
#
#   # Quick execution:
#   result = CodingAdventures::StarlarkVM.execute_starlark("x = 1 + 2\n")
#   result.variables["x"]  # => 3
#
#   # Manual setup (for more control):
#   vm = CodingAdventures::StarlarkVM.create_starlark_vm
#   code = CodingAdventures::StarlarkAstToBytecodeCompiler::Compiler.compile_starlark("x = 42\n")
#   traces = vm.execute(code)
#   vm.variables["x"]  # => 42
# ==========================================================================

module CodingAdventures
  module StarlarkVM
    # Create a new GenericVM configured for Starlark execution.
    #
    # Registers all 46 opcode handlers and 23 builtin functions.
    # The returned VM is ready to execute any compiled Starlark CodeObject.
    #
    # @param max_recursion_depth [Integer] maximum call stack depth (default: 200)
    # @return [VirtualMachine::GenericVM] a configured Starlark VM
    def self.create_starlark_vm(max_recursion_depth: 200)
      vm = CodingAdventures::VirtualMachine::GenericVM.new
      vm.set_max_recursion_depth(max_recursion_depth)
      Handlers.register_all(vm)
      Builtins.register_all(vm)
      vm
    end

    # Compile and execute Starlark source code in one step.
    #
    # This is the simplest way to run Starlark code. It handles the full
    # pipeline: parse -> compile -> execute.
    #
    # @param source [String] Starlark source code (must end with newline)
    # @return [StarlarkResult] execution result with variables, output, and traces
    #
    # @example
    #   result = CodingAdventures::StarlarkVM.execute_starlark("x = 1 + 2\n")
    #   result.variables["x"]  # => 3
    #   result.output           # => []
    #
    # @example with print
    #   result = CodingAdventures::StarlarkVM.execute_starlark("print(\"hello\")\n")
    #   result.output  # => ["hello"]
    def self.execute_starlark(source)
      code = CodingAdventures::StarlarkAstToBytecodeCompiler::Compiler.compile_starlark(source)
      vm = create_starlark_vm
      traces = vm.execute(code)
      StarlarkResult.new(
        variables: vm.variables.dup,
        output: vm.output.dup,
        traces: traces
      )
    end
  end
end
