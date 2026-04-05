defmodule CodingAdventures.WasmExecution.Instructions.Parametric do
  @moduledoc """
  Parametric instruction handlers: drop (0x1A) and select (0x1B).
  These work with values of any type (type-polymorphic).
  """

  alias CodingAdventures.VirtualMachine.GenericVM

  @doc "Register the 2 parametric instruction handlers."
  def register(vm) do
    vm
    # 0x1A: drop -- discard the top stack value
    |> GenericVM.register_context_opcode(0x1A, fn vm, _instr, _code, _ctx ->
      {_value, vm} = GenericVM.pop_typed(vm)
      vm = GenericVM.advance_pc(vm)
      {nil, vm}
    end)
    # 0x1B: select -- ternary conditional
    |> GenericVM.register_context_opcode(0x1B, fn vm, _instr, _code, _ctx ->
      {condition, vm} = GenericVM.pop_typed(vm)
      {val2, vm} = GenericVM.pop_typed(vm)
      {val1, vm} = GenericVM.pop_typed(vm)
      result = if condition.value != 0, do: val1, else: val2
      vm = GenericVM.push_typed(vm, result)
      vm = GenericVM.advance_pc(vm)
      {nil, vm}
    end)
  end
end
