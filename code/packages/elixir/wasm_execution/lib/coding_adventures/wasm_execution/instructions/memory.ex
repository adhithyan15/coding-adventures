defmodule CodingAdventures.WasmExecution.Instructions.Memory do
  @moduledoc """
  Linear memory instruction handlers (27 instructions).
  Loads, stores, memory.size, and memory.grow.
  """

  import Bitwise
  alias CodingAdventures.VirtualMachine.GenericVM
  alias CodingAdventures.WasmExecution.{Values, LinearMemory, TrapError}

  @doc "Register all 27 memory instruction handlers."
  def register(vm) do
    vm
    |> register_loads()
    |> register_stores()
    |> register_management()
  end

  defp effective_addr(base, operand) do
    mem_offset =
      case operand do
        %{offset: o} -> o
        %{"memarg" => %{offset: o}} -> o
        _ -> 0
      end

    band(base, 0xFFFFFFFF) + mem_offset
  end

  defp require_memory!(ctx) do
    if ctx.memory == nil, do: raise(TrapError, "no linear memory")
    ctx.memory
  end

  defp register_loads(vm) do
    vm
    # 0x28: i32.load
    |> GenericVM.register_context_opcode(0x28, fn vm, instr, _code, ctx ->
      mem = require_memory!(ctx)
      {base_val, vm} = GenericVM.pop_typed(vm)
      addr = effective_addr(Values.as_i32(base_val), instr.operand)
      vm = GenericVM.push_typed(vm, Values.i32(LinearMemory.load_i32(mem, addr)))
      {nil, GenericVM.advance_pc(vm), ctx}
    end)
    # 0x29: i64.load
    |> GenericVM.register_context_opcode(0x29, fn vm, instr, _code, ctx ->
      mem = require_memory!(ctx)
      {base_val, vm} = GenericVM.pop_typed(vm)
      addr = effective_addr(Values.as_i32(base_val), instr.operand)
      vm = GenericVM.push_typed(vm, Values.i64(LinearMemory.load_i64(mem, addr)))
      {nil, GenericVM.advance_pc(vm), ctx}
    end)
    # 0x2A: f32.load
    |> GenericVM.register_context_opcode(0x2A, fn vm, instr, _code, ctx ->
      mem = require_memory!(ctx)
      {base_val, vm} = GenericVM.pop_typed(vm)
      addr = effective_addr(Values.as_i32(base_val), instr.operand)
      vm = GenericVM.push_typed(vm, Values.f32(LinearMemory.load_f32(mem, addr)))
      {nil, GenericVM.advance_pc(vm), ctx}
    end)
    # 0x2B: f64.load
    |> GenericVM.register_context_opcode(0x2B, fn vm, instr, _code, ctx ->
      mem = require_memory!(ctx)
      {base_val, vm} = GenericVM.pop_typed(vm)
      addr = effective_addr(Values.as_i32(base_val), instr.operand)
      vm = GenericVM.push_typed(vm, Values.f64(LinearMemory.load_f64(mem, addr)))
      {nil, GenericVM.advance_pc(vm), ctx}
    end)
    # Narrow loads for i32 (0x2C-0x2F)
    |> GenericVM.register_context_opcode(0x2C, fn vm, instr, _code, ctx ->
      mem = require_memory!(ctx)
      {base_val, vm} = GenericVM.pop_typed(vm)
      addr = effective_addr(Values.as_i32(base_val), instr.operand)
      vm = GenericVM.push_typed(vm, Values.i32(LinearMemory.load_i32_8s(mem, addr)))
      {nil, GenericVM.advance_pc(vm), ctx}
    end)
    |> GenericVM.register_context_opcode(0x2D, fn vm, instr, _code, ctx ->
      mem = require_memory!(ctx)
      {base_val, vm} = GenericVM.pop_typed(vm)
      addr = effective_addr(Values.as_i32(base_val), instr.operand)
      vm = GenericVM.push_typed(vm, Values.i32(LinearMemory.load_i32_8u(mem, addr)))
      {nil, GenericVM.advance_pc(vm), ctx}
    end)
    |> GenericVM.register_context_opcode(0x2E, fn vm, instr, _code, ctx ->
      mem = require_memory!(ctx)
      {base_val, vm} = GenericVM.pop_typed(vm)
      addr = effective_addr(Values.as_i32(base_val), instr.operand)
      vm = GenericVM.push_typed(vm, Values.i32(LinearMemory.load_i32_16s(mem, addr)))
      {nil, GenericVM.advance_pc(vm), ctx}
    end)
    |> GenericVM.register_context_opcode(0x2F, fn vm, instr, _code, ctx ->
      mem = require_memory!(ctx)
      {base_val, vm} = GenericVM.pop_typed(vm)
      addr = effective_addr(Values.as_i32(base_val), instr.operand)
      vm = GenericVM.push_typed(vm, Values.i32(LinearMemory.load_i32_16u(mem, addr)))
      {nil, GenericVM.advance_pc(vm), ctx}
    end)
    # Narrow loads for i64 (0x30-0x35)
    |> GenericVM.register_context_opcode(0x30, fn vm, instr, _code, ctx ->
      mem = require_memory!(ctx)
      {bv, vm} = GenericVM.pop_typed(vm)
      addr = effective_addr(Values.as_i32(bv), instr.operand)
      vm = GenericVM.push_typed(vm, Values.i64(LinearMemory.load_i64_8s(mem, addr)))
      {nil, GenericVM.advance_pc(vm), ctx}
    end)
    |> GenericVM.register_context_opcode(0x31, fn vm, instr, _code, ctx ->
      mem = require_memory!(ctx)
      {bv, vm} = GenericVM.pop_typed(vm)
      addr = effective_addr(Values.as_i32(bv), instr.operand)
      vm = GenericVM.push_typed(vm, Values.i64(LinearMemory.load_i64_8u(mem, addr)))
      {nil, GenericVM.advance_pc(vm), ctx}
    end)
    |> GenericVM.register_context_opcode(0x32, fn vm, instr, _code, ctx ->
      mem = require_memory!(ctx)
      {bv, vm} = GenericVM.pop_typed(vm)
      addr = effective_addr(Values.as_i32(bv), instr.operand)
      vm = GenericVM.push_typed(vm, Values.i64(LinearMemory.load_i64_16s(mem, addr)))
      {nil, GenericVM.advance_pc(vm), ctx}
    end)
    |> GenericVM.register_context_opcode(0x33, fn vm, instr, _code, ctx ->
      mem = require_memory!(ctx)
      {bv, vm} = GenericVM.pop_typed(vm)
      addr = effective_addr(Values.as_i32(bv), instr.operand)
      vm = GenericVM.push_typed(vm, Values.i64(LinearMemory.load_i64_16u(mem, addr)))
      {nil, GenericVM.advance_pc(vm), ctx}
    end)
    |> GenericVM.register_context_opcode(0x34, fn vm, instr, _code, ctx ->
      mem = require_memory!(ctx)
      {bv, vm} = GenericVM.pop_typed(vm)
      addr = effective_addr(Values.as_i32(bv), instr.operand)
      vm = GenericVM.push_typed(vm, Values.i64(LinearMemory.load_i64_32s(mem, addr)))
      {nil, GenericVM.advance_pc(vm), ctx}
    end)
    |> GenericVM.register_context_opcode(0x35, fn vm, instr, _code, ctx ->
      mem = require_memory!(ctx)
      {bv, vm} = GenericVM.pop_typed(vm)
      addr = effective_addr(Values.as_i32(bv), instr.operand)
      vm = GenericVM.push_typed(vm, Values.i64(LinearMemory.load_i64_32u(mem, addr)))
      {nil, GenericVM.advance_pc(vm), ctx}
    end)
  end

  defp register_stores(vm) do
    vm
    # 0x36: i32.store
    |> GenericVM.register_context_opcode(0x36, fn vm, instr, _code, ctx ->
      mem = require_memory!(ctx)
      {val, vm} = GenericVM.pop_typed(vm)
      {bv, vm} = GenericVM.pop_typed(vm)
      addr = effective_addr(Values.as_i32(bv), instr.operand)
      mem = LinearMemory.store_i32(mem, addr, Values.as_i32(val))
      {nil, GenericVM.advance_pc(vm), %{ctx | memory: mem}}
    end)
    # 0x37: i64.store
    |> GenericVM.register_context_opcode(0x37, fn vm, instr, _code, ctx ->
      mem = require_memory!(ctx)
      {val, vm} = GenericVM.pop_typed(vm)
      {bv, vm} = GenericVM.pop_typed(vm)
      addr = effective_addr(Values.as_i32(bv), instr.operand)
      mem = LinearMemory.store_i64(mem, addr, Values.as_i64(val))
      {nil, GenericVM.advance_pc(vm), %{ctx | memory: mem}}
    end)
    # 0x38: f32.store
    |> GenericVM.register_context_opcode(0x38, fn vm, instr, _code, ctx ->
      mem = require_memory!(ctx)
      {val, vm} = GenericVM.pop_typed(vm)
      {bv, vm} = GenericVM.pop_typed(vm)
      addr = effective_addr(Values.as_i32(bv), instr.operand)
      mem = LinearMemory.store_f32(mem, addr, Values.as_f32(val))
      {nil, GenericVM.advance_pc(vm), %{ctx | memory: mem}}
    end)
    # 0x39: f64.store
    |> GenericVM.register_context_opcode(0x39, fn vm, instr, _code, ctx ->
      mem = require_memory!(ctx)
      {val, vm} = GenericVM.pop_typed(vm)
      {bv, vm} = GenericVM.pop_typed(vm)
      addr = effective_addr(Values.as_i32(bv), instr.operand)
      mem = LinearMemory.store_f64(mem, addr, Values.as_f64(val))
      {nil, GenericVM.advance_pc(vm), %{ctx | memory: mem}}
    end)
    # Narrow stores (0x3A-0x3E)
    |> GenericVM.register_context_opcode(0x3A, fn vm, instr, _code, ctx ->
      mem = require_memory!(ctx)
      {val, vm} = GenericVM.pop_typed(vm)
      {bv, vm} = GenericVM.pop_typed(vm)
      addr = effective_addr(Values.as_i32(bv), instr.operand)
      mem = LinearMemory.store_i32_8(mem, addr, Values.as_i32(val))
      {nil, GenericVM.advance_pc(vm), %{ctx | memory: mem}}
    end)
    |> GenericVM.register_context_opcode(0x3B, fn vm, instr, _code, ctx ->
      mem = require_memory!(ctx)
      {val, vm} = GenericVM.pop_typed(vm)
      {bv, vm} = GenericVM.pop_typed(vm)
      addr = effective_addr(Values.as_i32(bv), instr.operand)
      mem = LinearMemory.store_i32_16(mem, addr, Values.as_i32(val))
      {nil, GenericVM.advance_pc(vm), %{ctx | memory: mem}}
    end)
    |> GenericVM.register_context_opcode(0x3C, fn vm, instr, _code, ctx ->
      mem = require_memory!(ctx)
      {val, vm} = GenericVM.pop_typed(vm)
      {bv, vm} = GenericVM.pop_typed(vm)
      addr = effective_addr(Values.as_i32(bv), instr.operand)
      mem = LinearMemory.store_i64_8(mem, addr, Values.as_i64(val))
      {nil, GenericVM.advance_pc(vm), %{ctx | memory: mem}}
    end)
    |> GenericVM.register_context_opcode(0x3D, fn vm, instr, _code, ctx ->
      mem = require_memory!(ctx)
      {val, vm} = GenericVM.pop_typed(vm)
      {bv, vm} = GenericVM.pop_typed(vm)
      addr = effective_addr(Values.as_i32(bv), instr.operand)
      mem = LinearMemory.store_i64_16(mem, addr, Values.as_i64(val))
      {nil, GenericVM.advance_pc(vm), %{ctx | memory: mem}}
    end)
    |> GenericVM.register_context_opcode(0x3E, fn vm, instr, _code, ctx ->
      mem = require_memory!(ctx)
      {val, vm} = GenericVM.pop_typed(vm)
      {bv, vm} = GenericVM.pop_typed(vm)
      addr = effective_addr(Values.as_i32(bv), instr.operand)
      mem = LinearMemory.store_i64_32(mem, addr, Values.as_i64(val))
      {nil, GenericVM.advance_pc(vm), %{ctx | memory: mem}}
    end)
  end

  defp register_management(vm) do
    vm
    # 0x3F: memory.size
    |> GenericVM.register_context_opcode(0x3F, fn vm, _instr, _code, ctx ->
      mem = require_memory!(ctx)
      vm = GenericVM.push_typed(vm, Values.i32(LinearMemory.size(mem)))
      {nil, GenericVM.advance_pc(vm), ctx}
    end)
    # 0x40: memory.grow
    |> GenericVM.register_context_opcode(0x40, fn vm, _instr, _code, ctx ->
      mem = require_memory!(ctx)
      {delta_val, vm} = GenericVM.pop_typed(vm)
      delta = Values.as_i32(delta_val)
      {result, new_mem} = LinearMemory.grow(mem, delta)
      vm = GenericVM.push_typed(vm, Values.i32(result))
      {nil, GenericVM.advance_pc(vm), %{ctx | memory: new_mem}}
    end)
  end
end
