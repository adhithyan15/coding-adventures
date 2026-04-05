defmodule CodingAdventures.WasmExecution.Instructions.Conversion do
  @moduledoc """
  Type conversion instruction handlers for WASM.

  These instructions convert between the four value types (i32, i64, f32, f64).
  Conversions fall into several categories:

  1. **Wrap** -- truncate a wider type to narrower (i64 -> i32)
  2. **Extend** -- widen a narrower type (i32 -> i64), signed or unsigned
  3. **Trunc** -- float to integer truncation (rounds toward zero), traps on overflow
  4. **Convert** -- integer to float
  5. **Demote/Promote** -- float precision changes (f64 -> f32, f32 -> f64)
  6. **Reinterpret** -- bit-level reinterpretation between int and float

  ## Opcode Map

      +--------+------------------------------+
      | Opcode | Instruction                  |
      +--------+------------------------------+
      | 0xA7   | i32.wrap_i64                 |
      | 0xA8   | i32.trunc_f32_s              |
      | 0xA9   | i32.trunc_f32_u              |
      | 0xAA   | i32.trunc_f64_s              |
      | 0xAB   | i32.trunc_f64_u              |
      | 0xAC   | i64.extend_i32_s             |
      | 0xAD   | i64.extend_i32_u             |
      | 0xAE   | i64.trunc_f32_s              |
      | 0xAF   | i64.trunc_f32_u              |
      | 0xB0   | i64.trunc_f64_s              |
      | 0xB1   | i64.trunc_f64_u              |
      | 0xB2   | f32.convert_i32_s            |
      | 0xB3   | f32.convert_i32_u            |
      | 0xB4   | f32.convert_i64_s            |
      | 0xB5   | f32.convert_i64_u            |
      | 0xB6   | f32.demote_f64               |
      | 0xB7   | f64.convert_i32_s            |
      | 0xB8   | f64.convert_i32_u            |
      | 0xB9   | f64.convert_i64_s            |
      | 0xBA   | f64.convert_i64_u            |
      | 0xBB   | f64.promote_f32              |
      | 0xBC   | i32.reinterpret_f32          |
      | 0xBD   | i64.reinterpret_f64          |
      | 0xBE   | f32.reinterpret_i32          |
      | 0xBF   | f64.reinterpret_i64          |
      +--------+------------------------------+
  """

  import Bitwise
  alias CodingAdventures.WasmExecution.Values
  alias CodingAdventures.WasmExecution.TrapError
  alias CodingAdventures.VirtualMachine.GenericVM

  @doc "Register all conversion instruction handlers on the given GenericVM."
  def register(vm) do
    vm
    |> register_wrap_extend()
    |> register_trunc()
    |> register_convert()
    |> register_demote_promote()
    |> register_reinterpret()
  end

  # -- Wrap and Extend --
  defp register_wrap_extend(vm) do
    vm
    # 0xA7: i32.wrap_i64 -- take low 32 bits of an i64
    |> GenericVM.register_context_opcode(0xA7, fn vm, _instr, _code, _ctx ->
      {a, vm} = GenericVM.pop_typed(vm)
      vm = GenericVM.push_typed(vm, Values.i32(Values.as_i64(a)))
      {nil, GenericVM.advance_pc(vm)}
    end)
    # 0xAC: i64.extend_i32_s -- sign-extend i32 to i64
    |> GenericVM.register_context_opcode(0xAC, fn vm, _instr, _code, _ctx ->
      {a, vm} = GenericVM.pop_typed(vm)
      vm = GenericVM.push_typed(vm, Values.i64(Values.as_i32(a)))
      {nil, GenericVM.advance_pc(vm)}
    end)
    # 0xAD: i64.extend_i32_u -- zero-extend i32 to i64
    |> GenericVM.register_context_opcode(0xAD, fn vm, _instr, _code, _ctx ->
      {a, vm} = GenericVM.pop_typed(vm)
      vm = GenericVM.push_typed(vm, Values.i64(Values.to_unsigned_32(Values.as_i32(a))))
      {nil, GenericVM.advance_pc(vm)}
    end)
  end

  # -- Float-to-Int Truncations --
  defp register_trunc(vm) do
    vm
    # 0xA8: i32.trunc_f32_s
    |> GenericVM.register_context_opcode(0xA8, fn vm, _instr, _code, _ctx ->
      {a, vm} = GenericVM.pop_typed(vm)
      v = Values.as_f32(a)
      check_trunc_i32_s!(v)
      vm = GenericVM.push_typed(vm, Values.i32(trunc(v)))
      {nil, GenericVM.advance_pc(vm)}
    end)
    # 0xA9: i32.trunc_f32_u
    |> GenericVM.register_context_opcode(0xA9, fn vm, _instr, _code, _ctx ->
      {a, vm} = GenericVM.pop_typed(vm)
      v = Values.as_f32(a)
      check_trunc_i32_u!(v)
      vm = GenericVM.push_typed(vm, Values.i32(trunc(v)))
      {nil, GenericVM.advance_pc(vm)}
    end)
    # 0xAA: i32.trunc_f64_s
    |> GenericVM.register_context_opcode(0xAA, fn vm, _instr, _code, _ctx ->
      {a, vm} = GenericVM.pop_typed(vm)
      v = Values.as_f64(a)
      check_trunc_i32_s!(v)
      vm = GenericVM.push_typed(vm, Values.i32(trunc(v)))
      {nil, GenericVM.advance_pc(vm)}
    end)
    # 0xAB: i32.trunc_f64_u
    |> GenericVM.register_context_opcode(0xAB, fn vm, _instr, _code, _ctx ->
      {a, vm} = GenericVM.pop_typed(vm)
      v = Values.as_f64(a)
      check_trunc_i32_u!(v)
      vm = GenericVM.push_typed(vm, Values.i32(trunc(v)))
      {nil, GenericVM.advance_pc(vm)}
    end)
    # 0xAE: i64.trunc_f32_s
    |> GenericVM.register_context_opcode(0xAE, fn vm, _instr, _code, _ctx ->
      {a, vm} = GenericVM.pop_typed(vm)
      v = Values.as_f32(a)
      check_trunc_i64_s!(v)
      vm = GenericVM.push_typed(vm, Values.i64(trunc(v)))
      {nil, GenericVM.advance_pc(vm)}
    end)
    # 0xAF: i64.trunc_f32_u
    |> GenericVM.register_context_opcode(0xAF, fn vm, _instr, _code, _ctx ->
      {a, vm} = GenericVM.pop_typed(vm)
      v = Values.as_f32(a)
      check_trunc_i64_u!(v)
      vm = GenericVM.push_typed(vm, Values.i64(trunc(v)))
      {nil, GenericVM.advance_pc(vm)}
    end)
    # 0xB0: i64.trunc_f64_s
    |> GenericVM.register_context_opcode(0xB0, fn vm, _instr, _code, _ctx ->
      {a, vm} = GenericVM.pop_typed(vm)
      v = Values.as_f64(a)
      check_trunc_i64_s!(v)
      vm = GenericVM.push_typed(vm, Values.i64(trunc(v)))
      {nil, GenericVM.advance_pc(vm)}
    end)
    # 0xB1: i64.trunc_f64_u
    |> GenericVM.register_context_opcode(0xB1, fn vm, _instr, _code, _ctx ->
      {a, vm} = GenericVM.pop_typed(vm)
      v = Values.as_f64(a)
      check_trunc_i64_u!(v)
      vm = GenericVM.push_typed(vm, Values.i64(trunc(v)))
      {nil, GenericVM.advance_pc(vm)}
    end)
  end

  # -- Int-to-Float Conversions --
  defp register_convert(vm) do
    vm
    # 0xB2: f32.convert_i32_s
    |> GenericVM.register_context_opcode(0xB2, fn vm, _instr, _code, _ctx ->
      {a, vm} = GenericVM.pop_typed(vm)
      vm = GenericVM.push_typed(vm, Values.f32(Values.as_i32(a)))
      {nil, GenericVM.advance_pc(vm)}
    end)
    # 0xB3: f32.convert_i32_u
    |> GenericVM.register_context_opcode(0xB3, fn vm, _instr, _code, _ctx ->
      {a, vm} = GenericVM.pop_typed(vm)
      vm = GenericVM.push_typed(vm, Values.f32(Values.to_unsigned_32(Values.as_i32(a))))
      {nil, GenericVM.advance_pc(vm)}
    end)
    # 0xB4: f32.convert_i64_s
    |> GenericVM.register_context_opcode(0xB4, fn vm, _instr, _code, _ctx ->
      {a, vm} = GenericVM.pop_typed(vm)
      vm = GenericVM.push_typed(vm, Values.f32(Values.as_i64(a)))
      {nil, GenericVM.advance_pc(vm)}
    end)
    # 0xB5: f32.convert_i64_u
    |> GenericVM.register_context_opcode(0xB5, fn vm, _instr, _code, _ctx ->
      {a, vm} = GenericVM.pop_typed(vm)
      vm = GenericVM.push_typed(vm, Values.f32(Values.to_unsigned_64(Values.as_i64(a))))
      {nil, GenericVM.advance_pc(vm)}
    end)
    # 0xB7: f64.convert_i32_s
    |> GenericVM.register_context_opcode(0xB7, fn vm, _instr, _code, _ctx ->
      {a, vm} = GenericVM.pop_typed(vm)
      vm = GenericVM.push_typed(vm, Values.f64(Values.as_i32(a)))
      {nil, GenericVM.advance_pc(vm)}
    end)
    # 0xB8: f64.convert_i32_u
    |> GenericVM.register_context_opcode(0xB8, fn vm, _instr, _code, _ctx ->
      {a, vm} = GenericVM.pop_typed(vm)
      vm = GenericVM.push_typed(vm, Values.f64(Values.to_unsigned_32(Values.as_i32(a))))
      {nil, GenericVM.advance_pc(vm)}
    end)
    # 0xB9: f64.convert_i64_s
    |> GenericVM.register_context_opcode(0xB9, fn vm, _instr, _code, _ctx ->
      {a, vm} = GenericVM.pop_typed(vm)
      vm = GenericVM.push_typed(vm, Values.f64(Values.as_i64(a)))
      {nil, GenericVM.advance_pc(vm)}
    end)
    # 0xBA: f64.convert_i64_u
    |> GenericVM.register_context_opcode(0xBA, fn vm, _instr, _code, _ctx ->
      {a, vm} = GenericVM.pop_typed(vm)
      vm = GenericVM.push_typed(vm, Values.f64(Values.to_unsigned_64(Values.as_i64(a))))
      {nil, GenericVM.advance_pc(vm)}
    end)
  end

  # -- Demote / Promote --
  defp register_demote_promote(vm) do
    vm
    # 0xB6: f32.demote_f64
    |> GenericVM.register_context_opcode(0xB6, fn vm, _instr, _code, _ctx ->
      {a, vm} = GenericVM.pop_typed(vm)
      vm = GenericVM.push_typed(vm, Values.f32(Values.as_f64(a)))
      {nil, GenericVM.advance_pc(vm)}
    end)
    # 0xBB: f64.promote_f32
    |> GenericVM.register_context_opcode(0xBB, fn vm, _instr, _code, _ctx ->
      {a, vm} = GenericVM.pop_typed(vm)
      vm = GenericVM.push_typed(vm, Values.f64(Values.as_f32(a)))
      {nil, GenericVM.advance_pc(vm)}
    end)
  end

  # -- Reinterpret (bit-level casts) --
  defp register_reinterpret(vm) do
    vm
    # 0xBC: i32.reinterpret_f32
    |> GenericVM.register_context_opcode(0xBC, fn vm, _instr, _code, _ctx ->
      {a, vm} = GenericVM.pop_typed(vm)
      fv = Values.as_f32(a)
      <<bits::signed-integer-size(32)>> = <<fv::little-float-size(32)>>
      vm = GenericVM.push_typed(vm, Values.i32(bits))
      {nil, GenericVM.advance_pc(vm)}
    end)
    # 0xBD: i64.reinterpret_f64
    |> GenericVM.register_context_opcode(0xBD, fn vm, _instr, _code, _ctx ->
      {a, vm} = GenericVM.pop_typed(vm)
      fv = Values.as_f64(a)
      <<bits::signed-integer-size(64)>> = <<fv::little-float-size(64)>>
      vm = GenericVM.push_typed(vm, Values.i64(bits))
      {nil, GenericVM.advance_pc(vm)}
    end)
    # 0xBE: f32.reinterpret_i32
    |> GenericVM.register_context_opcode(0xBE, fn vm, _instr, _code, _ctx ->
      {a, vm} = GenericVM.pop_typed(vm)
      iv = Values.as_i32(a)
      <<fv::little-float-size(32)>> = <<band(iv, 0xFFFFFFFF)::little-integer-size(32)>>
      vm = GenericVM.push_typed(vm, Values.f32(fv))
      {nil, GenericVM.advance_pc(vm)}
    end)
    # 0xBF: f64.reinterpret_i64
    |> GenericVM.register_context_opcode(0xBF, fn vm, _instr, _code, _ctx ->
      {a, vm} = GenericVM.pop_typed(vm)
      iv = Values.as_i64(a)
      <<fv::little-float-size(64)>> = <<band(iv, 0xFFFFFFFFFFFFFFFF)::little-integer-size(64)>>
      vm = GenericVM.push_typed(vm, Values.f64(fv))
      {nil, GenericVM.advance_pc(vm)}
    end)
  end

  # -- Truncation bounds checking --

  defp check_trunc_i32_s!(v) do
    if v < -2_147_483_648.0 or v >= 2_147_483_648.0 do
      raise TrapError, "integer overflow in trunc"
    end
  end

  defp check_trunc_i32_u!(v) do
    if v < 0.0 or v >= 4_294_967_296.0 do
      raise TrapError, "integer overflow in trunc"
    end
  end

  defp check_trunc_i64_s!(v) do
    if v < -9_223_372_036_854_775_808.0 or v >= 9_223_372_036_854_775_808.0 do
      raise TrapError, "integer overflow in trunc"
    end
  end

  defp check_trunc_i64_u!(v) do
    if v < 0.0 or v >= 18_446_744_073_709_551_616.0 do
      raise TrapError, "integer overflow in trunc"
    end
  end
end
