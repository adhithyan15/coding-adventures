defmodule CodingAdventures.WasmExecution.Instructions.NumericF64 do
  @moduledoc """
  64-bit float instruction handlers for WASM.

  Registers handlers for f64 operations: const, comparisons,
  unary math, and binary arithmetic. Elixir floats are already
  64-bit doubles, so f64 is the native precision.

  ## Opcode Map (f64 instructions)

      +--------+------------------+
      | Opcode | Instruction      |
      +--------+------------------+
      | 0x44   | f64.const        |
      | 0x61   | f64.eq           |
      | 0x62   | f64.ne           |
      | 0x63   | f64.lt           |
      | 0x64   | f64.gt           |
      | 0x65   | f64.le           |
      | 0x66   | f64.ge           |
      | 0x99   | f64.abs          |
      | 0x9A   | f64.neg          |
      | 0x9B   | f64.ceil         |
      | 0x9C   | f64.floor        |
      | 0x9D   | f64.trunc        |
      | 0x9E   | f64.nearest      |
      | 0x9F   | f64.sqrt         |
      | 0xA0   | f64.add          |
      | 0xA1   | f64.sub          |
      | 0xA2   | f64.mul          |
      | 0xA3   | f64.div          |
      | 0xA4   | f64.min          |
      | 0xA5   | f64.max          |
      | 0xA6   | f64.copysign     |
      +--------+------------------+
  """

  alias CodingAdventures.WasmExecution.Values
  alias CodingAdventures.VirtualMachine.GenericVM

  @doc "Register all f64 numeric instruction handlers on the given GenericVM."
  def register(vm) do
    vm
    |> register_const()
    |> register_comparisons()
    |> register_unary()
    |> register_binary()
  end

  # -- f64.const (0x44) --
  defp register_const(vm) do
    GenericVM.register_context_opcode(vm, 0x44, fn vm, instr, _code, _ctx ->
      vm = GenericVM.push_typed(vm, Values.f64(instr.operand))
      vm = GenericVM.advance_pc(vm)
      {nil, vm}
    end)
  end

  # -- Comparisons (0x61-0x66) --
  defp register_comparisons(vm) do
    vm
    # f64.eq
    |> GenericVM.register_context_opcode(0x61, fn vm, _instr, _code, _ctx ->
      {b, vm} = GenericVM.pop_typed(vm)
      {a, vm} = GenericVM.pop_typed(vm)
      result = if Values.as_f64(a) == Values.as_f64(b), do: 1, else: 0
      vm = GenericVM.push_typed(vm, Values.i32(result))
      {nil, GenericVM.advance_pc(vm)}
    end)
    # f64.ne
    |> GenericVM.register_context_opcode(0x62, fn vm, _instr, _code, _ctx ->
      {b, vm} = GenericVM.pop_typed(vm)
      {a, vm} = GenericVM.pop_typed(vm)
      result = if Values.as_f64(a) != Values.as_f64(b), do: 1, else: 0
      vm = GenericVM.push_typed(vm, Values.i32(result))
      {nil, GenericVM.advance_pc(vm)}
    end)
    # f64.lt
    |> GenericVM.register_context_opcode(0x63, fn vm, _instr, _code, _ctx ->
      {b, vm} = GenericVM.pop_typed(vm)
      {a, vm} = GenericVM.pop_typed(vm)
      result = if Values.as_f64(a) < Values.as_f64(b), do: 1, else: 0
      vm = GenericVM.push_typed(vm, Values.i32(result))
      {nil, GenericVM.advance_pc(vm)}
    end)
    # f64.gt
    |> GenericVM.register_context_opcode(0x64, fn vm, _instr, _code, _ctx ->
      {b, vm} = GenericVM.pop_typed(vm)
      {a, vm} = GenericVM.pop_typed(vm)
      result = if Values.as_f64(a) > Values.as_f64(b), do: 1, else: 0
      vm = GenericVM.push_typed(vm, Values.i32(result))
      {nil, GenericVM.advance_pc(vm)}
    end)
    # f64.le
    |> GenericVM.register_context_opcode(0x65, fn vm, _instr, _code, _ctx ->
      {b, vm} = GenericVM.pop_typed(vm)
      {a, vm} = GenericVM.pop_typed(vm)
      result = if Values.as_f64(a) <= Values.as_f64(b), do: 1, else: 0
      vm = GenericVM.push_typed(vm, Values.i32(result))
      {nil, GenericVM.advance_pc(vm)}
    end)
    # f64.ge
    |> GenericVM.register_context_opcode(0x66, fn vm, _instr, _code, _ctx ->
      {b, vm} = GenericVM.pop_typed(vm)
      {a, vm} = GenericVM.pop_typed(vm)
      result = if Values.as_f64(a) >= Values.as_f64(b), do: 1, else: 0
      vm = GenericVM.push_typed(vm, Values.i32(result))
      {nil, GenericVM.advance_pc(vm)}
    end)
  end

  # -- Unary ops (0x99-0x9F) --
  defp register_unary(vm) do
    vm
    # f64.abs
    |> GenericVM.register_context_opcode(0x99, fn vm, _instr, _code, _ctx ->
      {a, vm} = GenericVM.pop_typed(vm)
      vm = GenericVM.push_typed(vm, Values.f64(abs(Values.as_f64(a))))
      {nil, GenericVM.advance_pc(vm)}
    end)
    # f64.neg
    |> GenericVM.register_context_opcode(0x9A, fn vm, _instr, _code, _ctx ->
      {a, vm} = GenericVM.pop_typed(vm)
      vm = GenericVM.push_typed(vm, Values.f64(-Values.as_f64(a)))
      {nil, GenericVM.advance_pc(vm)}
    end)
    # f64.ceil
    |> GenericVM.register_context_opcode(0x9B, fn vm, _instr, _code, _ctx ->
      {a, vm} = GenericVM.pop_typed(vm)
      vm = GenericVM.push_typed(vm, Values.f64(Float.ceil(Values.as_f64(a))))
      {nil, GenericVM.advance_pc(vm)}
    end)
    # f64.floor
    |> GenericVM.register_context_opcode(0x9C, fn vm, _instr, _code, _ctx ->
      {a, vm} = GenericVM.pop_typed(vm)
      vm = GenericVM.push_typed(vm, Values.f64(Float.floor(Values.as_f64(a))))
      {nil, GenericVM.advance_pc(vm)}
    end)
    # f64.trunc
    |> GenericVM.register_context_opcode(0x9D, fn vm, _instr, _code, _ctx ->
      {a, vm} = GenericVM.pop_typed(vm)
      v = Values.as_f64(a)
      truncated = if v >= 0.0, do: Float.floor(v), else: Float.ceil(v)
      vm = GenericVM.push_typed(vm, Values.f64(truncated))
      {nil, GenericVM.advance_pc(vm)}
    end)
    # f64.nearest
    |> GenericVM.register_context_opcode(0x9E, fn vm, _instr, _code, _ctx ->
      {a, vm} = GenericVM.pop_typed(vm)
      vm = GenericVM.push_typed(vm, Values.f64(Float.round(Values.as_f64(a))))
      {nil, GenericVM.advance_pc(vm)}
    end)
    # f64.sqrt
    |> GenericVM.register_context_opcode(0x9F, fn vm, _instr, _code, _ctx ->
      {a, vm} = GenericVM.pop_typed(vm)
      vm = GenericVM.push_typed(vm, Values.f64(:math.sqrt(Values.as_f64(a))))
      {nil, GenericVM.advance_pc(vm)}
    end)
  end

  # -- Binary ops (0xA0-0xA6) --
  defp register_binary(vm) do
    vm
    # f64.add
    |> GenericVM.register_context_opcode(0xA0, fn vm, _instr, _code, _ctx ->
      {b, vm} = GenericVM.pop_typed(vm)
      {a, vm} = GenericVM.pop_typed(vm)
      vm = GenericVM.push_typed(vm, Values.f64(Values.as_f64(a) + Values.as_f64(b)))
      {nil, GenericVM.advance_pc(vm)}
    end)
    # f64.sub
    |> GenericVM.register_context_opcode(0xA1, fn vm, _instr, _code, _ctx ->
      {b, vm} = GenericVM.pop_typed(vm)
      {a, vm} = GenericVM.pop_typed(vm)
      vm = GenericVM.push_typed(vm, Values.f64(Values.as_f64(a) - Values.as_f64(b)))
      {nil, GenericVM.advance_pc(vm)}
    end)
    # f64.mul
    |> GenericVM.register_context_opcode(0xA2, fn vm, _instr, _code, _ctx ->
      {b, vm} = GenericVM.pop_typed(vm)
      {a, vm} = GenericVM.pop_typed(vm)
      vm = GenericVM.push_typed(vm, Values.f64(Values.as_f64(a) * Values.as_f64(b)))
      {nil, GenericVM.advance_pc(vm)}
    end)
    # f64.div
    |> GenericVM.register_context_opcode(0xA3, fn vm, _instr, _code, _ctx ->
      {b, vm} = GenericVM.pop_typed(vm)
      {a, vm} = GenericVM.pop_typed(vm)
      vm = GenericVM.push_typed(vm, Values.f64(Values.as_f64(a) / Values.as_f64(b)))
      {nil, GenericVM.advance_pc(vm)}
    end)
    # f64.min
    |> GenericVM.register_context_opcode(0xA4, fn vm, _instr, _code, _ctx ->
      {b, vm} = GenericVM.pop_typed(vm)
      {a, vm} = GenericVM.pop_typed(vm)
      vm = GenericVM.push_typed(vm, Values.f64(min(Values.as_f64(a), Values.as_f64(b))))
      {nil, GenericVM.advance_pc(vm)}
    end)
    # f64.max
    |> GenericVM.register_context_opcode(0xA5, fn vm, _instr, _code, _ctx ->
      {b, vm} = GenericVM.pop_typed(vm)
      {a, vm} = GenericVM.pop_typed(vm)
      vm = GenericVM.push_typed(vm, Values.f64(max(Values.as_f64(a), Values.as_f64(b))))
      {nil, GenericVM.advance_pc(vm)}
    end)
    # f64.copysign
    |> GenericVM.register_context_opcode(0xA6, fn vm, _instr, _code, _ctx ->
      {b, vm} = GenericVM.pop_typed(vm)
      {a, vm} = GenericVM.pop_typed(vm)
      magnitude = abs(Values.as_f64(a))
      sign_val = Values.as_f64(b)
      result = if sign_val < 0.0, do: -magnitude, else: magnitude
      vm = GenericVM.push_typed(vm, Values.f64(result))
      {nil, GenericVM.advance_pc(vm)}
    end)
  end
end
