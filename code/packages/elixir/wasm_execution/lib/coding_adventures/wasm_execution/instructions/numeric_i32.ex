defmodule CodingAdventures.WasmExecution.Instructions.NumericI32 do
  @moduledoc """
  32-bit integer instruction handlers for WASM.

  Registers 33 handlers for i32 operations: const, comparisons,
  unary bit ops, arithmetic, bitwise logic, shifts, and rotates.

  Pop order for binary ops: b first (top of stack), then a.
  The operation is a <op> b.

  i32 wrapping: all results are wrapped to signed 32-bit using
  `Values.wrap_i32/1` which does `band(val, 0xFFFFFFFF)` then sign extends.
  """

  import Bitwise
  alias CodingAdventures.WasmExecution.Values
  alias CodingAdventures.WasmExecution.TrapError
  alias CodingAdventures.VirtualMachine.GenericVM

  @int32_min -2_147_483_648

  @doc "Register all 33 i32 numeric instruction handlers on the given GenericVM."
  def register(vm) do
    vm
    |> register_const()
    |> register_comparisons()
    |> register_unary()
    |> register_arithmetic()
    |> register_bitwise()
    |> register_shifts()
  end

  # -- i32.const (0x41) --
  defp register_const(vm) do
    GenericVM.register_context_opcode(vm, 0x41, fn vm, instr, _code, _ctx ->
      vm = GenericVM.push_typed(vm, Values.i32(instr.operand))
      vm = GenericVM.advance_pc(vm)
      {nil, vm}
    end)
  end

  # -- Comparisons (0x45-0x4F) --
  defp register_comparisons(vm) do
    vm
    # i32.eqz
    |> GenericVM.register_context_opcode(0x45, fn vm, _instr, _code, _ctx ->
      {a, vm} = GenericVM.pop_typed(vm)
      vm = GenericVM.push_typed(vm, Values.i32(if(Values.as_i32(a) == 0, do: 1, else: 0)))
      vm = GenericVM.advance_pc(vm)
      {nil, vm}
    end)
    # i32.eq
    |> GenericVM.register_context_opcode(0x46, fn vm, _instr, _code, _ctx ->
      {b, vm} = GenericVM.pop_typed(vm)
      {a, vm} = GenericVM.pop_typed(vm)
      result = if Values.as_i32(a) == Values.as_i32(b), do: 1, else: 0
      vm = GenericVM.push_typed(vm, Values.i32(result))
      {nil, GenericVM.advance_pc(vm)}
    end)
    # i32.ne
    |> GenericVM.register_context_opcode(0x47, fn vm, _instr, _code, _ctx ->
      {b, vm} = GenericVM.pop_typed(vm)
      {a, vm} = GenericVM.pop_typed(vm)
      result = if Values.as_i32(a) != Values.as_i32(b), do: 1, else: 0
      vm = GenericVM.push_typed(vm, Values.i32(result))
      {nil, GenericVM.advance_pc(vm)}
    end)
    # i32.lt_s
    |> GenericVM.register_context_opcode(0x48, fn vm, _instr, _code, _ctx ->
      {b, vm} = GenericVM.pop_typed(vm)
      {a, vm} = GenericVM.pop_typed(vm)
      result = if Values.as_i32(a) < Values.as_i32(b), do: 1, else: 0
      vm = GenericVM.push_typed(vm, Values.i32(result))
      {nil, GenericVM.advance_pc(vm)}
    end)
    # i32.lt_u
    |> GenericVM.register_context_opcode(0x49, fn vm, _instr, _code, _ctx ->
      {b, vm} = GenericVM.pop_typed(vm)
      {a, vm} = GenericVM.pop_typed(vm)

      result =
        if Values.to_unsigned_32(Values.as_i32(a)) < Values.to_unsigned_32(Values.as_i32(b)),
          do: 1,
          else: 0

      vm = GenericVM.push_typed(vm, Values.i32(result))
      {nil, GenericVM.advance_pc(vm)}
    end)
    # i32.gt_s
    |> GenericVM.register_context_opcode(0x4A, fn vm, _instr, _code, _ctx ->
      {b, vm} = GenericVM.pop_typed(vm)
      {a, vm} = GenericVM.pop_typed(vm)
      result = if Values.as_i32(a) > Values.as_i32(b), do: 1, else: 0
      vm = GenericVM.push_typed(vm, Values.i32(result))
      {nil, GenericVM.advance_pc(vm)}
    end)
    # i32.gt_u
    |> GenericVM.register_context_opcode(0x4B, fn vm, _instr, _code, _ctx ->
      {b, vm} = GenericVM.pop_typed(vm)
      {a, vm} = GenericVM.pop_typed(vm)

      result =
        if Values.to_unsigned_32(Values.as_i32(a)) > Values.to_unsigned_32(Values.as_i32(b)),
          do: 1,
          else: 0

      vm = GenericVM.push_typed(vm, Values.i32(result))
      {nil, GenericVM.advance_pc(vm)}
    end)
    # i32.le_s
    |> GenericVM.register_context_opcode(0x4C, fn vm, _instr, _code, _ctx ->
      {b, vm} = GenericVM.pop_typed(vm)
      {a, vm} = GenericVM.pop_typed(vm)
      result = if Values.as_i32(a) <= Values.as_i32(b), do: 1, else: 0
      vm = GenericVM.push_typed(vm, Values.i32(result))
      {nil, GenericVM.advance_pc(vm)}
    end)
    # i32.le_u
    |> GenericVM.register_context_opcode(0x4D, fn vm, _instr, _code, _ctx ->
      {b, vm} = GenericVM.pop_typed(vm)
      {a, vm} = GenericVM.pop_typed(vm)

      result =
        if Values.to_unsigned_32(Values.as_i32(a)) <= Values.to_unsigned_32(Values.as_i32(b)),
          do: 1,
          else: 0

      vm = GenericVM.push_typed(vm, Values.i32(result))
      {nil, GenericVM.advance_pc(vm)}
    end)
    # i32.ge_s
    |> GenericVM.register_context_opcode(0x4E, fn vm, _instr, _code, _ctx ->
      {b, vm} = GenericVM.pop_typed(vm)
      {a, vm} = GenericVM.pop_typed(vm)
      result = if Values.as_i32(a) >= Values.as_i32(b), do: 1, else: 0
      vm = GenericVM.push_typed(vm, Values.i32(result))
      {nil, GenericVM.advance_pc(vm)}
    end)
    # i32.ge_u
    |> GenericVM.register_context_opcode(0x4F, fn vm, _instr, _code, _ctx ->
      {b, vm} = GenericVM.pop_typed(vm)
      {a, vm} = GenericVM.pop_typed(vm)

      result =
        if Values.to_unsigned_32(Values.as_i32(a)) >= Values.to_unsigned_32(Values.as_i32(b)),
          do: 1,
          else: 0

      vm = GenericVM.push_typed(vm, Values.i32(result))
      {nil, GenericVM.advance_pc(vm)}
    end)
  end

  # -- Unary bit ops (0x67-0x69) --
  defp register_unary(vm) do
    vm
    # i32.clz
    |> GenericVM.register_context_opcode(0x67, fn vm, _instr, _code, _ctx ->
      {a, vm} = GenericVM.pop_typed(vm)
      vm = GenericVM.push_typed(vm, Values.i32(clz32(Values.as_i32(a))))
      {nil, GenericVM.advance_pc(vm)}
    end)
    # i32.ctz
    |> GenericVM.register_context_opcode(0x68, fn vm, _instr, _code, _ctx ->
      {a, vm} = GenericVM.pop_typed(vm)
      vm = GenericVM.push_typed(vm, Values.i32(ctz32(Values.as_i32(a))))
      {nil, GenericVM.advance_pc(vm)}
    end)
    # i32.popcnt
    |> GenericVM.register_context_opcode(0x69, fn vm, _instr, _code, _ctx ->
      {a, vm} = GenericVM.pop_typed(vm)
      vm = GenericVM.push_typed(vm, Values.i32(popcnt32(Values.as_i32(a))))
      {nil, GenericVM.advance_pc(vm)}
    end)
  end

  # -- Arithmetic (0x6A-0x70) --
  defp register_arithmetic(vm) do
    vm
    # i32.add
    |> GenericVM.register_context_opcode(0x6A, fn vm, _instr, _code, _ctx ->
      {b, vm} = GenericVM.pop_typed(vm)
      {a, vm} = GenericVM.pop_typed(vm)
      vm = GenericVM.push_typed(vm, Values.i32(Values.as_i32(a) + Values.as_i32(b)))
      {nil, GenericVM.advance_pc(vm)}
    end)
    # i32.sub
    |> GenericVM.register_context_opcode(0x6B, fn vm, _instr, _code, _ctx ->
      {b, vm} = GenericVM.pop_typed(vm)
      {a, vm} = GenericVM.pop_typed(vm)
      vm = GenericVM.push_typed(vm, Values.i32(Values.as_i32(a) - Values.as_i32(b)))
      {nil, GenericVM.advance_pc(vm)}
    end)
    # i32.mul
    |> GenericVM.register_context_opcode(0x6C, fn vm, _instr, _code, _ctx ->
      {b, vm} = GenericVM.pop_typed(vm)
      {a, vm} = GenericVM.pop_typed(vm)
      vm = GenericVM.push_typed(vm, Values.i32(Values.as_i32(a) * Values.as_i32(b)))
      {nil, GenericVM.advance_pc(vm)}
    end)
    # i32.div_s
    |> GenericVM.register_context_opcode(0x6D, fn vm, _instr, _code, _ctx ->
      {b, vm} = GenericVM.pop_typed(vm)
      {a, vm} = GenericVM.pop_typed(vm)
      bv = Values.as_i32(b)
      av = Values.as_i32(a)
      if bv == 0, do: raise(TrapError, "integer divide by zero")
      if av == @int32_min and bv == -1, do: raise(TrapError, "integer overflow")
      vm = GenericVM.push_typed(vm, Values.i32(div(av, bv)))
      {nil, GenericVM.advance_pc(vm)}
    end)
    # i32.div_u
    |> GenericVM.register_context_opcode(0x6E, fn vm, _instr, _code, _ctx ->
      {b, vm} = GenericVM.pop_typed(vm)
      {a, vm} = GenericVM.pop_typed(vm)
      bu = Values.to_unsigned_32(Values.as_i32(b))
      au = Values.to_unsigned_32(Values.as_i32(a))
      if bu == 0, do: raise(TrapError, "integer divide by zero")
      vm = GenericVM.push_typed(vm, Values.i32(div(au, bu)))
      {nil, GenericVM.advance_pc(vm)}
    end)
    # i32.rem_s
    |> GenericVM.register_context_opcode(0x6F, fn vm, _instr, _code, _ctx ->
      {b, vm} = GenericVM.pop_typed(vm)
      {a, vm} = GenericVM.pop_typed(vm)
      bv = Values.as_i32(b)
      av = Values.as_i32(a)
      if bv == 0, do: raise(TrapError, "integer divide by zero")
      result = if av == @int32_min and bv == -1, do: 0, else: rem(av, bv)
      vm = GenericVM.push_typed(vm, Values.i32(result))
      {nil, GenericVM.advance_pc(vm)}
    end)
    # i32.rem_u
    |> GenericVM.register_context_opcode(0x70, fn vm, _instr, _code, _ctx ->
      {b, vm} = GenericVM.pop_typed(vm)
      {a, vm} = GenericVM.pop_typed(vm)
      bu = Values.to_unsigned_32(Values.as_i32(b))
      au = Values.to_unsigned_32(Values.as_i32(a))
      if bu == 0, do: raise(TrapError, "integer divide by zero")
      vm = GenericVM.push_typed(vm, Values.i32(rem(au, bu)))
      {nil, GenericVM.advance_pc(vm)}
    end)
  end

  # -- Bitwise (0x71-0x73) --
  defp register_bitwise(vm) do
    vm
    # i32.and
    |> GenericVM.register_context_opcode(0x71, fn vm, _instr, _code, _ctx ->
      {b, vm} = GenericVM.pop_typed(vm)
      {a, vm} = GenericVM.pop_typed(vm)
      vm = GenericVM.push_typed(vm, Values.i32(band(Values.as_i32(a), Values.as_i32(b))))
      {nil, GenericVM.advance_pc(vm)}
    end)
    # i32.or
    |> GenericVM.register_context_opcode(0x72, fn vm, _instr, _code, _ctx ->
      {b, vm} = GenericVM.pop_typed(vm)
      {a, vm} = GenericVM.pop_typed(vm)
      vm = GenericVM.push_typed(vm, Values.i32(bor(Values.as_i32(a), Values.as_i32(b))))
      {nil, GenericVM.advance_pc(vm)}
    end)
    # i32.xor
    |> GenericVM.register_context_opcode(0x73, fn vm, _instr, _code, _ctx ->
      {b, vm} = GenericVM.pop_typed(vm)
      {a, vm} = GenericVM.pop_typed(vm)
      vm = GenericVM.push_typed(vm, Values.i32(bxor(Values.as_i32(a), Values.as_i32(b))))
      {nil, GenericVM.advance_pc(vm)}
    end)
  end

  # -- Shifts and Rotates (0x74-0x78) --
  defp register_shifts(vm) do
    vm
    # i32.shl
    |> GenericVM.register_context_opcode(0x74, fn vm, _instr, _code, _ctx ->
      {b, vm} = GenericVM.pop_typed(vm)
      {a, vm} = GenericVM.pop_typed(vm)
      n = band(Values.as_i32(b), 31)
      vm = GenericVM.push_typed(vm, Values.i32(bsl(Values.as_i32(a), n)))
      {nil, GenericVM.advance_pc(vm)}
    end)
    # i32.shr_s
    |> GenericVM.register_context_opcode(0x75, fn vm, _instr, _code, _ctx ->
      {b, vm} = GenericVM.pop_typed(vm)
      {a, vm} = GenericVM.pop_typed(vm)
      n = band(Values.as_i32(b), 31)
      # Arithmetic shift right (sign preserving)
      vm = GenericVM.push_typed(vm, Values.i32(bsr(Values.as_i32(a), n)))
      {nil, GenericVM.advance_pc(vm)}
    end)
    # i32.shr_u
    |> GenericVM.register_context_opcode(0x76, fn vm, _instr, _code, _ctx ->
      {b, vm} = GenericVM.pop_typed(vm)
      {a, vm} = GenericVM.pop_typed(vm)
      n = band(Values.as_i32(b), 31)
      au = Values.to_unsigned_32(Values.as_i32(a))
      vm = GenericVM.push_typed(vm, Values.i32(bsr(au, n)))
      {nil, GenericVM.advance_pc(vm)}
    end)
    # i32.rotl
    |> GenericVM.register_context_opcode(0x77, fn vm, _instr, _code, _ctx ->
      {b, vm} = GenericVM.pop_typed(vm)
      {a, vm} = GenericVM.pop_typed(vm)
      n = band(Values.as_i32(b), 31)
      au = Values.to_unsigned_32(Values.as_i32(a))
      result = bor(bsl(au, n), bsr(au, 32 - n))
      vm = GenericVM.push_typed(vm, Values.i32(result))
      {nil, GenericVM.advance_pc(vm)}
    end)
    # i32.rotr
    |> GenericVM.register_context_opcode(0x78, fn vm, _instr, _code, _ctx ->
      {b, vm} = GenericVM.pop_typed(vm)
      {a, vm} = GenericVM.pop_typed(vm)
      n = band(Values.as_i32(b), 31)
      au = Values.to_unsigned_32(Values.as_i32(a))
      result = bor(bsr(au, n), bsl(au, 32 - n))
      vm = GenericVM.push_typed(vm, Values.i32(result))
      {nil, GenericVM.advance_pc(vm)}
    end)
  end

  # -- Bit counting helpers --
  defp clz32(0), do: 32

  defp clz32(value) do
    v = band(value, 0xFFFFFFFF)
    do_clz(v, 31, 0)
  end

  defp do_clz(_v, -1, count), do: count

  defp do_clz(v, bit, count) do
    if band(v, bsl(1, bit)) != 0, do: count, else: do_clz(v, bit - 1, count + 1)
  end

  defp ctz32(0), do: 32

  defp ctz32(value) do
    v = band(value, 0xFFFFFFFF)
    do_ctz(v, 0, 0)
  end

  defp do_ctz(_v, 32, count), do: count

  defp do_ctz(v, bit, count) do
    if band(v, bsl(1, bit)) != 0, do: count, else: do_ctz(v, bit + 1, count + 1)
  end

  defp popcnt32(value) do
    v = band(value, 0xFFFFFFFF)
    do_popcnt(v, 0)
  end

  defp do_popcnt(0, count), do: count

  defp do_popcnt(v, count) do
    do_popcnt(band(v, v - 1), count + 1)
  end
end
