defmodule CodingAdventures.WasmExecution.Instructions.NumericF32 do
  @moduledoc """
  32-bit float instruction handlers for WASM.

  Registers handlers for f32 operations: const, comparisons,
  unary math, and binary arithmetic.

  In Elixir, all floats are 64-bit doubles. We approximate f32 behavior
  by storing values as-is (exact f32 rounding would require a NIF).

  ## Opcode Map (f32 instructions)

      +--------+------------------+
      | Opcode | Instruction      |
      +--------+------------------+
      | 0x43   | f32.const        |
      | 0x5B   | f32.eq           |
      | 0x5C   | f32.ne           |
      | 0x5D   | f32.lt           |
      | 0x5E   | f32.gt           |
      | 0x5F   | f32.le           |
      | 0x60   | f32.ge           |
      | 0x8B   | f32.abs          |
      | 0x8C   | f32.neg          |
      | 0x8D   | f32.ceil         |
      | 0x8E   | f32.floor        |
      | 0x8F   | f32.trunc        |
      | 0x90   | f32.nearest      |
      | 0x91   | f32.sqrt         |
      | 0x92   | f32.add          |
      | 0x93   | f32.sub          |
      | 0x94   | f32.mul          |
      | 0x95   | f32.div          |
      | 0x96   | f32.min          |
      | 0x97   | f32.max          |
      | 0x98   | f32.copysign     |
      +--------+------------------+
  """

  alias CodingAdventures.WasmExecution.Values
  alias CodingAdventures.VirtualMachine.GenericVM

  @doc "Register all f32 numeric instruction handlers on the given GenericVM."
  def register(vm) do
    vm
    |> register_const()
    |> register_comparisons()
    |> register_unary()
    |> register_binary()
  end

  # -- f32.const (0x43) --
  defp register_const(vm) do
    GenericVM.register_context_opcode(vm, 0x43, fn vm, instr, _code, _ctx ->
      vm = GenericVM.push_typed(vm, Values.f32(instr.operand))
      vm = GenericVM.advance_pc(vm)
      {nil, vm}
    end)
  end

  # -- Comparisons (0x5B-0x60) --
  # f32 comparisons return i32 (0 or 1). NaN comparisons follow IEEE 754.
  defp register_comparisons(vm) do
    vm
    # f32.eq
    |> GenericVM.register_context_opcode(0x5B, fn vm, _instr, _code, _ctx ->
      {b, vm} = GenericVM.pop_typed(vm)
      {a, vm} = GenericVM.pop_typed(vm)
      result = if Values.as_f32(a) == Values.as_f32(b), do: 1, else: 0
      vm = GenericVM.push_typed(vm, Values.i32(result))
      {nil, GenericVM.advance_pc(vm)}
    end)
    # f32.ne
    |> GenericVM.register_context_opcode(0x5C, fn vm, _instr, _code, _ctx ->
      {b, vm} = GenericVM.pop_typed(vm)
      {a, vm} = GenericVM.pop_typed(vm)
      result = if Values.as_f32(a) != Values.as_f32(b), do: 1, else: 0
      vm = GenericVM.push_typed(vm, Values.i32(result))
      {nil, GenericVM.advance_pc(vm)}
    end)
    # f32.lt
    |> GenericVM.register_context_opcode(0x5D, fn vm, _instr, _code, _ctx ->
      {b, vm} = GenericVM.pop_typed(vm)
      {a, vm} = GenericVM.pop_typed(vm)
      result = if Values.as_f32(a) < Values.as_f32(b), do: 1, else: 0
      vm = GenericVM.push_typed(vm, Values.i32(result))
      {nil, GenericVM.advance_pc(vm)}
    end)
    # f32.gt
    |> GenericVM.register_context_opcode(0x5E, fn vm, _instr, _code, _ctx ->
      {b, vm} = GenericVM.pop_typed(vm)
      {a, vm} = GenericVM.pop_typed(vm)
      result = if Values.as_f32(a) > Values.as_f32(b), do: 1, else: 0
      vm = GenericVM.push_typed(vm, Values.i32(result))
      {nil, GenericVM.advance_pc(vm)}
    end)
    # f32.le
    |> GenericVM.register_context_opcode(0x5F, fn vm, _instr, _code, _ctx ->
      {b, vm} = GenericVM.pop_typed(vm)
      {a, vm} = GenericVM.pop_typed(vm)
      result = if Values.as_f32(a) <= Values.as_f32(b), do: 1, else: 0
      vm = GenericVM.push_typed(vm, Values.i32(result))
      {nil, GenericVM.advance_pc(vm)}
    end)
    # f32.ge
    |> GenericVM.register_context_opcode(0x60, fn vm, _instr, _code, _ctx ->
      {b, vm} = GenericVM.pop_typed(vm)
      {a, vm} = GenericVM.pop_typed(vm)
      result = if Values.as_f32(a) >= Values.as_f32(b), do: 1, else: 0
      vm = GenericVM.push_typed(vm, Values.i32(result))
      {nil, GenericVM.advance_pc(vm)}
    end)
  end

  # -- Unary ops (0x8B-0x91) --
  defp register_unary(vm) do
    vm
    # f32.abs
    |> GenericVM.register_context_opcode(0x8B, fn vm, _instr, _code, _ctx ->
      {a, vm} = GenericVM.pop_typed(vm)
      vm = GenericVM.push_typed(vm, Values.f32(abs(Values.as_f32(a))))
      {nil, GenericVM.advance_pc(vm)}
    end)
    # f32.neg
    |> GenericVM.register_context_opcode(0x8C, fn vm, _instr, _code, _ctx ->
      {a, vm} = GenericVM.pop_typed(vm)
      vm = GenericVM.push_typed(vm, Values.f32(-Values.as_f32(a)))
      {nil, GenericVM.advance_pc(vm)}
    end)
    # f32.ceil
    |> GenericVM.register_context_opcode(0x8D, fn vm, _instr, _code, _ctx ->
      {a, vm} = GenericVM.pop_typed(vm)
      vm = GenericVM.push_typed(vm, Values.f32(Float.ceil(Values.as_f32(a))))
      {nil, GenericVM.advance_pc(vm)}
    end)
    # f32.floor
    |> GenericVM.register_context_opcode(0x8E, fn vm, _instr, _code, _ctx ->
      {a, vm} = GenericVM.pop_typed(vm)
      vm = GenericVM.push_typed(vm, Values.f32(Float.floor(Values.as_f32(a))))
      {nil, GenericVM.advance_pc(vm)}
    end)
    # f32.trunc
    |> GenericVM.register_context_opcode(0x8F, fn vm, _instr, _code, _ctx ->
      {a, vm} = GenericVM.pop_typed(vm)
      v = Values.as_f32(a)
      truncated = if v >= 0.0, do: Float.floor(v), else: Float.ceil(v)
      vm = GenericVM.push_typed(vm, Values.f32(truncated))
      {nil, GenericVM.advance_pc(vm)}
    end)
    # f32.nearest (round to even)
    |> GenericVM.register_context_opcode(0x90, fn vm, _instr, _code, _ctx ->
      {a, vm} = GenericVM.pop_typed(vm)
      vm = GenericVM.push_typed(vm, Values.f32(Float.round(Values.as_f32(a))))
      {nil, GenericVM.advance_pc(vm)}
    end)
    # f32.sqrt
    |> GenericVM.register_context_opcode(0x91, fn vm, _instr, _code, _ctx ->
      {a, vm} = GenericVM.pop_typed(vm)
      vm = GenericVM.push_typed(vm, Values.f32(:math.sqrt(Values.as_f32(a))))
      {nil, GenericVM.advance_pc(vm)}
    end)
  end

  # -- Binary ops (0x92-0x98) --
  defp register_binary(vm) do
    vm
    # f32.add
    |> GenericVM.register_context_opcode(0x92, fn vm, _instr, _code, _ctx ->
      {b, vm} = GenericVM.pop_typed(vm)
      {a, vm} = GenericVM.pop_typed(vm)
      vm = GenericVM.push_typed(vm, Values.f32(Values.as_f32(a) + Values.as_f32(b)))
      {nil, GenericVM.advance_pc(vm)}
    end)
    # f32.sub
    |> GenericVM.register_context_opcode(0x93, fn vm, _instr, _code, _ctx ->
      {b, vm} = GenericVM.pop_typed(vm)
      {a, vm} = GenericVM.pop_typed(vm)
      vm = GenericVM.push_typed(vm, Values.f32(Values.as_f32(a) - Values.as_f32(b)))
      {nil, GenericVM.advance_pc(vm)}
    end)
    # f32.mul
    |> GenericVM.register_context_opcode(0x94, fn vm, _instr, _code, _ctx ->
      {b, vm} = GenericVM.pop_typed(vm)
      {a, vm} = GenericVM.pop_typed(vm)
      vm = GenericVM.push_typed(vm, Values.f32(Values.as_f32(a) * Values.as_f32(b)))
      {nil, GenericVM.advance_pc(vm)}
    end)
    # f32.div
    |> GenericVM.register_context_opcode(0x95, fn vm, _instr, _code, _ctx ->
      {b, vm} = GenericVM.pop_typed(vm)
      {a, vm} = GenericVM.pop_typed(vm)
      vm = GenericVM.push_typed(vm, Values.f32(Values.as_f32(a) / Values.as_f32(b)))
      {nil, GenericVM.advance_pc(vm)}
    end)
    # f32.min
    |> GenericVM.register_context_opcode(0x96, fn vm, _instr, _code, _ctx ->
      {b, vm} = GenericVM.pop_typed(vm)
      {a, vm} = GenericVM.pop_typed(vm)
      vm = GenericVM.push_typed(vm, Values.f32(min(Values.as_f32(a), Values.as_f32(b))))
      {nil, GenericVM.advance_pc(vm)}
    end)
    # f32.max
    |> GenericVM.register_context_opcode(0x97, fn vm, _instr, _code, _ctx ->
      {b, vm} = GenericVM.pop_typed(vm)
      {a, vm} = GenericVM.pop_typed(vm)
      vm = GenericVM.push_typed(vm, Values.f32(max(Values.as_f32(a), Values.as_f32(b))))
      {nil, GenericVM.advance_pc(vm)}
    end)
    # f32.copysign
    |> GenericVM.register_context_opcode(0x98, fn vm, _instr, _code, _ctx ->
      {b, vm} = GenericVM.pop_typed(vm)
      {a, vm} = GenericVM.pop_typed(vm)
      magnitude = abs(Values.as_f32(a))
      sign_val = Values.as_f32(b)
      result = if sign_val < 0.0, do: -magnitude, else: magnitude
      vm = GenericVM.push_typed(vm, Values.f32(result))
      {nil, GenericVM.advance_pc(vm)}
    end)
  end
end
