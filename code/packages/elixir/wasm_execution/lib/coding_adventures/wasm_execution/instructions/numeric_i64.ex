defmodule CodingAdventures.WasmExecution.Instructions.NumericI64 do
  @moduledoc """
  64-bit integer instruction handlers for WASM.

  Registers handlers for i64 operations: const, comparisons,
  unary bit ops, arithmetic, bitwise logic, shifts, and rotates.

  Pop order for binary ops: b first (top of stack), then a.
  The operation is a <op> b.

  i64 wrapping: all results are wrapped to signed 64-bit using
  `Values.wrap_i64/1` which does `band(val, 0xFFFFFFFFFFFFFFFF)` then sign extends.

  ## Opcode Map (i64 instructions)

      +--------+------------------+
      | Opcode | Instruction      |
      +--------+------------------+
      | 0x42   | i64.const        |
      | 0x50   | i64.eqz          |
      | 0x51   | i64.eq           |
      | 0x52   | i64.ne           |
      | 0x53   | i64.lt_s         |
      | 0x54   | i64.lt_u         |
      | 0x55   | i64.gt_s         |
      | 0x56   | i64.gt_u         |
      | 0x57   | i64.le_s         |
      | 0x58   | i64.le_u         |
      | 0x59   | i64.ge_s         |
      | 0x5A   | i64.ge_u         |
      | 0x79   | i64.clz          |
      | 0x7A   | i64.ctz          |
      | 0x7B   | i64.popcnt       |
      | 0x7C   | i64.add          |
      | 0x7D   | i64.sub          |
      | 0x7E   | i64.mul          |
      | 0x7F   | i64.div_s        |
      | 0x80   | i64.div_u        |
      | 0x81   | i64.rem_s        |
      | 0x82   | i64.rem_u        |
      | 0x83   | i64.and          |
      | 0x84   | i64.or           |
      | 0x85   | i64.xor          |
      | 0x86   | i64.shl          |
      | 0x87   | i64.shr_s        |
      | 0x88   | i64.shr_u        |
      | 0x89   | i64.rotl         |
      | 0x8A   | i64.rotr         |
      +--------+------------------+
  """

  import Bitwise
  alias CodingAdventures.WasmExecution.Values
  alias CodingAdventures.WasmExecution.TrapError
  alias CodingAdventures.VirtualMachine.GenericVM

  @int64_min -9_223_372_036_854_775_808

  @doc "Register all i64 numeric instruction handlers on the given GenericVM."
  def register(vm) do
    vm
    |> register_const()
    |> register_comparisons()
    |> register_unary()
    |> register_arithmetic()
    |> register_bitwise()
    |> register_shifts()
  end

  # -- i64.const (0x42) --
  defp register_const(vm) do
    GenericVM.register_context_opcode(vm, 0x42, fn vm, instr, _code, _ctx ->
      vm = GenericVM.push_typed(vm, Values.i64(instr.operand))
      vm = GenericVM.advance_pc(vm)
      {nil, vm}
    end)
  end

  # -- Comparisons (0x50-0x5A) --
  # i64 comparisons return i32 (0 or 1) per the WASM spec.
  defp register_comparisons(vm) do
    vm
    # i64.eqz
    |> GenericVM.register_context_opcode(0x50, fn vm, _instr, _code, _ctx ->
      {a, vm} = GenericVM.pop_typed(vm)
      result = if Values.as_i64(a) == 0, do: 1, else: 0
      vm = GenericVM.push_typed(vm, Values.i32(result))
      {nil, GenericVM.advance_pc(vm)}
    end)
    # i64.eq
    |> GenericVM.register_context_opcode(0x51, fn vm, _instr, _code, _ctx ->
      {b, vm} = GenericVM.pop_typed(vm)
      {a, vm} = GenericVM.pop_typed(vm)
      result = if Values.as_i64(a) == Values.as_i64(b), do: 1, else: 0
      vm = GenericVM.push_typed(vm, Values.i32(result))
      {nil, GenericVM.advance_pc(vm)}
    end)
    # i64.ne
    |> GenericVM.register_context_opcode(0x52, fn vm, _instr, _code, _ctx ->
      {b, vm} = GenericVM.pop_typed(vm)
      {a, vm} = GenericVM.pop_typed(vm)
      result = if Values.as_i64(a) != Values.as_i64(b), do: 1, else: 0
      vm = GenericVM.push_typed(vm, Values.i32(result))
      {nil, GenericVM.advance_pc(vm)}
    end)
    # i64.lt_s
    |> GenericVM.register_context_opcode(0x53, fn vm, _instr, _code, _ctx ->
      {b, vm} = GenericVM.pop_typed(vm)
      {a, vm} = GenericVM.pop_typed(vm)
      result = if Values.as_i64(a) < Values.as_i64(b), do: 1, else: 0
      vm = GenericVM.push_typed(vm, Values.i32(result))
      {nil, GenericVM.advance_pc(vm)}
    end)
    # i64.lt_u
    |> GenericVM.register_context_opcode(0x54, fn vm, _instr, _code, _ctx ->
      {b, vm} = GenericVM.pop_typed(vm)
      {a, vm} = GenericVM.pop_typed(vm)

      result =
        if Values.to_unsigned_64(Values.as_i64(a)) < Values.to_unsigned_64(Values.as_i64(b)),
          do: 1,
          else: 0

      vm = GenericVM.push_typed(vm, Values.i32(result))
      {nil, GenericVM.advance_pc(vm)}
    end)
    # i64.gt_s
    |> GenericVM.register_context_opcode(0x55, fn vm, _instr, _code, _ctx ->
      {b, vm} = GenericVM.pop_typed(vm)
      {a, vm} = GenericVM.pop_typed(vm)
      result = if Values.as_i64(a) > Values.as_i64(b), do: 1, else: 0
      vm = GenericVM.push_typed(vm, Values.i32(result))
      {nil, GenericVM.advance_pc(vm)}
    end)
    # i64.gt_u
    |> GenericVM.register_context_opcode(0x56, fn vm, _instr, _code, _ctx ->
      {b, vm} = GenericVM.pop_typed(vm)
      {a, vm} = GenericVM.pop_typed(vm)

      result =
        if Values.to_unsigned_64(Values.as_i64(a)) > Values.to_unsigned_64(Values.as_i64(b)),
          do: 1,
          else: 0

      vm = GenericVM.push_typed(vm, Values.i32(result))
      {nil, GenericVM.advance_pc(vm)}
    end)
    # i64.le_s
    |> GenericVM.register_context_opcode(0x57, fn vm, _instr, _code, _ctx ->
      {b, vm} = GenericVM.pop_typed(vm)
      {a, vm} = GenericVM.pop_typed(vm)
      result = if Values.as_i64(a) <= Values.as_i64(b), do: 1, else: 0
      vm = GenericVM.push_typed(vm, Values.i32(result))
      {nil, GenericVM.advance_pc(vm)}
    end)
    # i64.le_u
    |> GenericVM.register_context_opcode(0x58, fn vm, _instr, _code, _ctx ->
      {b, vm} = GenericVM.pop_typed(vm)
      {a, vm} = GenericVM.pop_typed(vm)

      result =
        if Values.to_unsigned_64(Values.as_i64(a)) <= Values.to_unsigned_64(Values.as_i64(b)),
          do: 1,
          else: 0

      vm = GenericVM.push_typed(vm, Values.i32(result))
      {nil, GenericVM.advance_pc(vm)}
    end)
    # i64.ge_s
    |> GenericVM.register_context_opcode(0x59, fn vm, _instr, _code, _ctx ->
      {b, vm} = GenericVM.pop_typed(vm)
      {a, vm} = GenericVM.pop_typed(vm)
      result = if Values.as_i64(a) >= Values.as_i64(b), do: 1, else: 0
      vm = GenericVM.push_typed(vm, Values.i32(result))
      {nil, GenericVM.advance_pc(vm)}
    end)
    # i64.ge_u
    |> GenericVM.register_context_opcode(0x5A, fn vm, _instr, _code, _ctx ->
      {b, vm} = GenericVM.pop_typed(vm)
      {a, vm} = GenericVM.pop_typed(vm)

      result =
        if Values.to_unsigned_64(Values.as_i64(a)) >= Values.to_unsigned_64(Values.as_i64(b)),
          do: 1,
          else: 0

      vm = GenericVM.push_typed(vm, Values.i32(result))
      {nil, GenericVM.advance_pc(vm)}
    end)
  end

  # -- Unary bit ops (0x79-0x7B) --
  defp register_unary(vm) do
    vm
    # i64.clz
    |> GenericVM.register_context_opcode(0x79, fn vm, _instr, _code, _ctx ->
      {a, vm} = GenericVM.pop_typed(vm)
      vm = GenericVM.push_typed(vm, Values.i64(clz64(Values.as_i64(a))))
      {nil, GenericVM.advance_pc(vm)}
    end)
    # i64.ctz
    |> GenericVM.register_context_opcode(0x7A, fn vm, _instr, _code, _ctx ->
      {a, vm} = GenericVM.pop_typed(vm)
      vm = GenericVM.push_typed(vm, Values.i64(ctz64(Values.as_i64(a))))
      {nil, GenericVM.advance_pc(vm)}
    end)
    # i64.popcnt
    |> GenericVM.register_context_opcode(0x7B, fn vm, _instr, _code, _ctx ->
      {a, vm} = GenericVM.pop_typed(vm)
      vm = GenericVM.push_typed(vm, Values.i64(popcnt64(Values.as_i64(a))))
      {nil, GenericVM.advance_pc(vm)}
    end)
  end

  # -- Arithmetic (0x7C-0x82) --
  defp register_arithmetic(vm) do
    vm
    # i64.add
    |> GenericVM.register_context_opcode(0x7C, fn vm, _instr, _code, _ctx ->
      {b, vm} = GenericVM.pop_typed(vm)
      {a, vm} = GenericVM.pop_typed(vm)
      vm = GenericVM.push_typed(vm, Values.i64(Values.as_i64(a) + Values.as_i64(b)))
      {nil, GenericVM.advance_pc(vm)}
    end)
    # i64.sub
    |> GenericVM.register_context_opcode(0x7D, fn vm, _instr, _code, _ctx ->
      {b, vm} = GenericVM.pop_typed(vm)
      {a, vm} = GenericVM.pop_typed(vm)
      vm = GenericVM.push_typed(vm, Values.i64(Values.as_i64(a) - Values.as_i64(b)))
      {nil, GenericVM.advance_pc(vm)}
    end)
    # i64.mul
    |> GenericVM.register_context_opcode(0x7E, fn vm, _instr, _code, _ctx ->
      {b, vm} = GenericVM.pop_typed(vm)
      {a, vm} = GenericVM.pop_typed(vm)
      vm = GenericVM.push_typed(vm, Values.i64(Values.as_i64(a) * Values.as_i64(b)))
      {nil, GenericVM.advance_pc(vm)}
    end)
    # i64.div_s
    |> GenericVM.register_context_opcode(0x7F, fn vm, _instr, _code, _ctx ->
      {b, vm} = GenericVM.pop_typed(vm)
      {a, vm} = GenericVM.pop_typed(vm)
      bv = Values.as_i64(b)
      av = Values.as_i64(a)
      if bv == 0, do: raise(TrapError, "integer divide by zero")
      if av == @int64_min and bv == -1, do: raise(TrapError, "integer overflow")
      vm = GenericVM.push_typed(vm, Values.i64(div(av, bv)))
      {nil, GenericVM.advance_pc(vm)}
    end)
    # i64.div_u
    |> GenericVM.register_context_opcode(0x80, fn vm, _instr, _code, _ctx ->
      {b, vm} = GenericVM.pop_typed(vm)
      {a, vm} = GenericVM.pop_typed(vm)
      bu = Values.to_unsigned_64(Values.as_i64(b))
      au = Values.to_unsigned_64(Values.as_i64(a))
      if bu == 0, do: raise(TrapError, "integer divide by zero")
      vm = GenericVM.push_typed(vm, Values.i64(div(au, bu)))
      {nil, GenericVM.advance_pc(vm)}
    end)
    # i64.rem_s
    |> GenericVM.register_context_opcode(0x81, fn vm, _instr, _code, _ctx ->
      {b, vm} = GenericVM.pop_typed(vm)
      {a, vm} = GenericVM.pop_typed(vm)
      bv = Values.as_i64(b)
      av = Values.as_i64(a)
      if bv == 0, do: raise(TrapError, "integer divide by zero")
      result = if av == @int64_min and bv == -1, do: 0, else: rem(av, bv)
      vm = GenericVM.push_typed(vm, Values.i64(result))
      {nil, GenericVM.advance_pc(vm)}
    end)
    # i64.rem_u
    |> GenericVM.register_context_opcode(0x82, fn vm, _instr, _code, _ctx ->
      {b, vm} = GenericVM.pop_typed(vm)
      {a, vm} = GenericVM.pop_typed(vm)
      bu = Values.to_unsigned_64(Values.as_i64(b))
      au = Values.to_unsigned_64(Values.as_i64(a))
      if bu == 0, do: raise(TrapError, "integer divide by zero")
      vm = GenericVM.push_typed(vm, Values.i64(rem(au, bu)))
      {nil, GenericVM.advance_pc(vm)}
    end)
  end

  # -- Bitwise (0x83-0x85) --
  defp register_bitwise(vm) do
    vm
    # i64.and
    |> GenericVM.register_context_opcode(0x83, fn vm, _instr, _code, _ctx ->
      {b, vm} = GenericVM.pop_typed(vm)
      {a, vm} = GenericVM.pop_typed(vm)
      vm = GenericVM.push_typed(vm, Values.i64(band(Values.as_i64(a), Values.as_i64(b))))
      {nil, GenericVM.advance_pc(vm)}
    end)
    # i64.or
    |> GenericVM.register_context_opcode(0x84, fn vm, _instr, _code, _ctx ->
      {b, vm} = GenericVM.pop_typed(vm)
      {a, vm} = GenericVM.pop_typed(vm)
      vm = GenericVM.push_typed(vm, Values.i64(bor(Values.as_i64(a), Values.as_i64(b))))
      {nil, GenericVM.advance_pc(vm)}
    end)
    # i64.xor
    |> GenericVM.register_context_opcode(0x85, fn vm, _instr, _code, _ctx ->
      {b, vm} = GenericVM.pop_typed(vm)
      {a, vm} = GenericVM.pop_typed(vm)
      vm = GenericVM.push_typed(vm, Values.i64(bxor(Values.as_i64(a), Values.as_i64(b))))
      {nil, GenericVM.advance_pc(vm)}
    end)
  end

  # -- Shifts and Rotates (0x86-0x8A) --
  defp register_shifts(vm) do
    vm
    # i64.shl
    |> GenericVM.register_context_opcode(0x86, fn vm, _instr, _code, _ctx ->
      {b, vm} = GenericVM.pop_typed(vm)
      {a, vm} = GenericVM.pop_typed(vm)
      n = band(Values.as_i64(b), 63)
      vm = GenericVM.push_typed(vm, Values.i64(bsl(Values.as_i64(a), n)))
      {nil, GenericVM.advance_pc(vm)}
    end)
    # i64.shr_s
    |> GenericVM.register_context_opcode(0x87, fn vm, _instr, _code, _ctx ->
      {b, vm} = GenericVM.pop_typed(vm)
      {a, vm} = GenericVM.pop_typed(vm)
      n = band(Values.as_i64(b), 63)
      vm = GenericVM.push_typed(vm, Values.i64(bsr(Values.as_i64(a), n)))
      {nil, GenericVM.advance_pc(vm)}
    end)
    # i64.shr_u
    |> GenericVM.register_context_opcode(0x88, fn vm, _instr, _code, _ctx ->
      {b, vm} = GenericVM.pop_typed(vm)
      {a, vm} = GenericVM.pop_typed(vm)
      n = band(Values.as_i64(b), 63)
      au = Values.to_unsigned_64(Values.as_i64(a))
      vm = GenericVM.push_typed(vm, Values.i64(bsr(au, n)))
      {nil, GenericVM.advance_pc(vm)}
    end)
    # i64.rotl
    |> GenericVM.register_context_opcode(0x89, fn vm, _instr, _code, _ctx ->
      {b, vm} = GenericVM.pop_typed(vm)
      {a, vm} = GenericVM.pop_typed(vm)
      n = band(Values.as_i64(b), 63)
      au = Values.to_unsigned_64(Values.as_i64(a))
      result = bor(bsl(au, n), bsr(au, 64 - n))
      vm = GenericVM.push_typed(vm, Values.i64(result))
      {nil, GenericVM.advance_pc(vm)}
    end)
    # i64.rotr
    |> GenericVM.register_context_opcode(0x8A, fn vm, _instr, _code, _ctx ->
      {b, vm} = GenericVM.pop_typed(vm)
      {a, vm} = GenericVM.pop_typed(vm)
      n = band(Values.as_i64(b), 63)
      au = Values.to_unsigned_64(Values.as_i64(a))
      result = bor(bsr(au, n), bsl(au, 64 - n))
      vm = GenericVM.push_typed(vm, Values.i64(result))
      {nil, GenericVM.advance_pc(vm)}
    end)
  end

  # -- Bit counting helpers for 64-bit --
  defp clz64(0), do: 64

  defp clz64(val) do
    v = band(val, 0xFFFFFFFFFFFFFFFF)
    do_clz(v, 63, 0)
  end

  defp do_clz(_v, -1, count), do: count

  defp do_clz(v, bit, count) do
    if band(v, bsl(1, bit)) != 0, do: count, else: do_clz(v, bit - 1, count + 1)
  end

  defp ctz64(0), do: 64

  defp ctz64(val) do
    v = band(val, 0xFFFFFFFFFFFFFFFF)
    do_ctz(v, 0, 0)
  end

  defp do_ctz(_v, 64, count), do: count

  defp do_ctz(v, bit, count) do
    if band(v, bsl(1, bit)) != 0, do: count, else: do_ctz(v, bit + 1, count + 1)
  end

  defp popcnt64(val) do
    v = band(val, 0xFFFFFFFFFFFFFFFF)
    do_popcnt(v, 0)
  end

  defp do_popcnt(0, count), do: count

  defp do_popcnt(v, count) do
    do_popcnt(band(v, v - 1), count + 1)
  end
end
