defmodule CodingAdventures.WasmExecution.Instructions.Variable do
  @moduledoc """
  Variable access instruction handlers (local.get/set/tee, global.get/set).

  Since the WASM execution context (locals, globals) is carried as an
  immutable map, we return `{output, vm, updated_ctx}` tuples.
  """

  alias CodingAdventures.VirtualMachine.GenericVM

  @doc "Register all 5 variable instruction handlers."
  def register(vm) do
    vm
    # 0x20: local.get
    |> GenericVM.register_context_opcode(0x20, fn vm, instr, _code, ctx ->
      index = instr.operand
      value = Enum.at(ctx.typed_locals, index)
      vm = GenericVM.push_typed(vm, value)
      vm = GenericVM.advance_pc(vm)
      {nil, vm, ctx}
    end)
    # 0x21: local.set
    |> GenericVM.register_context_opcode(0x21, fn vm, instr, _code, ctx ->
      index = instr.operand
      {value, vm} = GenericVM.pop_typed(vm)
      ctx = %{ctx | typed_locals: List.replace_at(ctx.typed_locals, index, value)}
      vm = GenericVM.advance_pc(vm)
      {nil, vm, ctx}
    end)
    # 0x22: local.tee
    |> GenericVM.register_context_opcode(0x22, fn vm, instr, _code, ctx ->
      index = instr.operand
      value = GenericVM.peek_typed(vm)
      ctx = %{ctx | typed_locals: List.replace_at(ctx.typed_locals, index, value)}
      vm = GenericVM.advance_pc(vm)
      {nil, vm, ctx}
    end)
    # 0x23: global.get
    |> GenericVM.register_context_opcode(0x23, fn vm, instr, _code, ctx ->
      index = instr.operand
      value = Enum.at(ctx.globals, index)
      vm = GenericVM.push_typed(vm, value)
      vm = GenericVM.advance_pc(vm)
      {nil, vm, ctx}
    end)
    # 0x24: global.set
    |> GenericVM.register_context_opcode(0x24, fn vm, instr, _code, ctx ->
      index = instr.operand
      {value, vm} = GenericVM.pop_typed(vm)
      ctx = %{ctx | globals: List.replace_at(ctx.globals, index, value)}
      vm = GenericVM.advance_pc(vm)
      {nil, vm, ctx}
    end)
  end
end
