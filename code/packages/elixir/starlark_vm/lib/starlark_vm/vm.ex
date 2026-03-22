defmodule CodingAdventures.StarlarkVm.Vm do
  @moduledoc """
  Starlark VM Factory — Creates configured GenericVM instances for Starlark.

  ## Chapter 1: The Full Pipeline

  This module ties everything together. The `create_starlark_vm/0` factory
  creates a `GenericVM` that's fully configured for Starlark execution:

  1. All 46+ opcodes have registered handlers.
  2. All 23 built-in functions are registered.
  3. Starlark-specific restrictions are configured (recursion limits, etc.).

  The `execute_starlark/1` convenience function goes even further: it takes
  Starlark source code as a string, compiles it, and executes it in one call.

  ## Chapter 2: How to Use

  **Quick start — one call does everything:**

      result = StarlarkVm.execute_starlark("x = 1 + 2\\nprint(x)\\n")
      result.variables["x"]  #=> 3
      result.output           #=> ["3"]

  **Step by step — for more control:**

      code = StarlarkAstToBytecodeCompiler.compile_starlark("x = 1 + 2\\n")
      vm = StarlarkVm.create_starlark_vm()
      {traces, vm} = GenericVM.execute(vm, code)
      vm.variables["x"]  #=> 3
  """

  alias CodingAdventures.VirtualMachine.GenericVM
  alias CodingAdventures.StarlarkAstToBytecodeCompiler.Opcodes, as: Op
  alias CodingAdventures.StarlarkVm.Handlers
  alias CodingAdventures.StarlarkVm.Builtins

  # ===========================================================================
  # VM Factory
  # ===========================================================================

  @doc """
  Create a `GenericVM` fully configured for Starlark execution.

  This is the main factory function. It:
  1. Creates a fresh GenericVM.
  2. Registers all 46+ Starlark opcode handlers.
  3. Registers all 23 Starlark built-in functions.
  4. Configures Starlark-specific restrictions.

  ## Options

  - `:max_recursion_depth` — Maximum call stack depth. Default 200.
  - `:frozen` — Whether to start in frozen mode. Default false.

  ## Example

      vm = Vm.create_starlark_vm()
      code = compile_starlark("x = 42\\n")
      {traces, vm} = GenericVM.execute(vm, code)
      vm.variables["x"]  #=> 42
  """
  def create_starlark_vm(opts \\ []) do
    max_depth = Keyword.get(opts, :max_recursion_depth, 200)
    frozen = Keyword.get(opts, :frozen, false)

    vm = GenericVM.new()

    # -- Register all opcode handlers --

    # Stack operations
    vm = GenericVM.register_opcode(vm, Op.load_const(), &Handlers.handle_load_const/3)
    vm = GenericVM.register_opcode(vm, Op.pop(), &Handlers.handle_pop/3)
    vm = GenericVM.register_opcode(vm, Op.dup(), &Handlers.handle_dup/3)
    vm = GenericVM.register_opcode(vm, Op.load_none(), &Handlers.handle_load_none/3)
    vm = GenericVM.register_opcode(vm, Op.load_true(), &Handlers.handle_load_true/3)
    vm = GenericVM.register_opcode(vm, Op.load_false(), &Handlers.handle_load_false/3)

    # Variable operations
    vm = GenericVM.register_opcode(vm, Op.store_name(), &Handlers.handle_store_name/3)
    vm = GenericVM.register_opcode(vm, Op.load_name(), &Handlers.handle_load_name/3)
    vm = GenericVM.register_opcode(vm, Op.store_local(), &Handlers.handle_store_local/3)
    vm = GenericVM.register_opcode(vm, Op.load_local(), &Handlers.handle_load_local/3)
    vm = GenericVM.register_opcode(vm, Op.store_closure(), &Handlers.handle_store_closure/3)
    vm = GenericVM.register_opcode(vm, Op.load_closure(), &Handlers.handle_load_closure/3)

    # Arithmetic
    vm = GenericVM.register_opcode(vm, Op.add(), &Handlers.handle_add/3)
    vm = GenericVM.register_opcode(vm, Op.sub(), &Handlers.handle_sub/3)
    vm = GenericVM.register_opcode(vm, Op.mul(), &Handlers.handle_mul/3)
    vm = GenericVM.register_opcode(vm, Op.div_op(), &Handlers.handle_div/3)
    vm = GenericVM.register_opcode(vm, Op.floor_div(), &Handlers.handle_floor_div/3)
    vm = GenericVM.register_opcode(vm, Op.mod(), &Handlers.handle_mod/3)
    vm = GenericVM.register_opcode(vm, Op.power(), &Handlers.handle_power/3)
    vm = GenericVM.register_opcode(vm, Op.negate(), &Handlers.handle_negate/3)
    vm = GenericVM.register_opcode(vm, Op.bit_and(), &Handlers.handle_bit_and/3)
    vm = GenericVM.register_opcode(vm, Op.bit_or(), &Handlers.handle_bit_or/3)
    vm = GenericVM.register_opcode(vm, Op.bit_xor(), &Handlers.handle_bit_xor/3)
    vm = GenericVM.register_opcode(vm, Op.bit_not(), &Handlers.handle_bit_not/3)
    vm = GenericVM.register_opcode(vm, Op.lshift(), &Handlers.handle_lshift/3)
    vm = GenericVM.register_opcode(vm, Op.rshift(), &Handlers.handle_rshift/3)

    # Comparisons
    vm = GenericVM.register_opcode(vm, Op.cmp_eq(), &Handlers.handle_cmp_eq/3)
    vm = GenericVM.register_opcode(vm, Op.cmp_ne(), &Handlers.handle_cmp_ne/3)
    vm = GenericVM.register_opcode(vm, Op.cmp_lt(), &Handlers.handle_cmp_lt/3)
    vm = GenericVM.register_opcode(vm, Op.cmp_gt(), &Handlers.handle_cmp_gt/3)
    vm = GenericVM.register_opcode(vm, Op.cmp_le(), &Handlers.handle_cmp_le/3)
    vm = GenericVM.register_opcode(vm, Op.cmp_ge(), &Handlers.handle_cmp_ge/3)
    vm = GenericVM.register_opcode(vm, Op.cmp_in(), &Handlers.handle_cmp_in/3)
    vm = GenericVM.register_opcode(vm, Op.cmp_not_in(), &Handlers.handle_cmp_not_in/3)

    # Boolean
    vm = GenericVM.register_opcode(vm, Op.logical_not(), &Handlers.handle_not/3)

    # Control flow
    vm = GenericVM.register_opcode(vm, Op.jump(), &Handlers.handle_jump/3)
    vm = GenericVM.register_opcode(vm, Op.jump_if_false(), &Handlers.handle_jump_if_false/3)
    vm = GenericVM.register_opcode(vm, Op.jump_if_true(), &Handlers.handle_jump_if_true/3)
    vm = GenericVM.register_opcode(vm, Op.jump_if_false_or_pop(), &Handlers.handle_jump_if_false_or_pop/3)
    vm = GenericVM.register_opcode(vm, Op.jump_if_true_or_pop(), &Handlers.handle_jump_if_true_or_pop/3)

    # Functions
    vm = GenericVM.register_opcode(vm, Op.make_function(), &Handlers.handle_make_function/3)
    vm = GenericVM.register_opcode(vm, Op.call_function(), &Handlers.handle_call_function/3)
    vm = GenericVM.register_opcode(vm, Op.call_function_kw(), &Handlers.handle_call_function_kw/3)
    vm = GenericVM.register_opcode(vm, Op.return_op(), &Handlers.handle_return/3)

    # Collections
    vm = GenericVM.register_opcode(vm, Op.build_list(), &Handlers.handle_build_list/3)
    vm = GenericVM.register_opcode(vm, Op.build_dict(), &Handlers.handle_build_dict/3)
    vm = GenericVM.register_opcode(vm, Op.build_tuple(), &Handlers.handle_build_tuple/3)
    vm = GenericVM.register_opcode(vm, Op.list_append(), &Handlers.handle_list_append/3)
    vm = GenericVM.register_opcode(vm, Op.dict_set(), &Handlers.handle_dict_set/3)

    # Subscript & attribute
    vm = GenericVM.register_opcode(vm, Op.load_subscript(), &Handlers.handle_load_subscript/3)
    vm = GenericVM.register_opcode(vm, Op.store_subscript(), &Handlers.handle_store_subscript/3)
    vm = GenericVM.register_opcode(vm, Op.load_attr(), &Handlers.handle_load_attr/3)
    vm = GenericVM.register_opcode(vm, Op.store_attr(), &Handlers.handle_store_attr/3)
    vm = GenericVM.register_opcode(vm, Op.load_slice(), &Handlers.handle_load_slice/3)

    # Iteration
    vm = GenericVM.register_opcode(vm, Op.get_iter(), &Handlers.handle_get_iter/3)
    vm = GenericVM.register_opcode(vm, Op.for_iter(), &Handlers.handle_for_iter/3)
    vm = GenericVM.register_opcode(vm, Op.unpack_sequence(), &Handlers.handle_unpack_sequence/3)

    # Module
    vm = GenericVM.register_opcode(vm, Op.load_module(), &Handlers.handle_load_module/3)
    vm = GenericVM.register_opcode(vm, Op.import_from(), &Handlers.handle_import_from/3)

    # I/O
    vm = GenericVM.register_opcode(vm, Op.print_op(), &Handlers.handle_print/3)

    # VM control
    vm = GenericVM.register_opcode(vm, Op.halt(), &Handlers.handle_halt/3)

    # -- Register built-in functions --
    vm = Enum.reduce(Builtins.get_all_builtins(), vm, fn {name, impl}, acc_vm ->
      GenericVM.register_builtin(acc_vm, name, impl)
    end)

    # -- Configure restrictions --
    vm = GenericVM.set_max_recursion_depth(vm, max_depth)
    vm = if frozen, do: GenericVM.set_frozen(vm, true), else: vm

    vm
  end

  # ===========================================================================
  # Convenience Functions
  # ===========================================================================

  @doc """
  Compile and execute Starlark source code in one call.

  This is the highest-level API. Pass in Starlark source code,
  get back the execution result with variables, output, and traces.

  ## Parameters

  - `source` — Starlark source code. Should end with a newline.

  ## Returns

  A `%StarlarkResult{}` struct with variables, output, and traces.

  ## Example

      result = Vm.execute_starlark("x = 1 + 2\\nprint(x)\\n")
      result.variables["x"]  #=> 3
      result.output           #=> ["3"]
  """
  def execute_starlark(source) when is_binary(source) do
    alias CodingAdventures.StarlarkAstToBytecodeCompiler.Compiler

    code = Compiler.compile_starlark(source)
    vm = create_starlark_vm()
    {traces, vm} = GenericVM.execute(vm, code)

    %Handlers.StarlarkResult{
      variables: vm.variables,
      output: Enum.reverse(vm.output),
      traces: traces
    }
  end
end
